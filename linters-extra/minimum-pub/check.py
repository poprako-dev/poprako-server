#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require every `pub` item to be referenced outside its defining module."""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
import os
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest

import tree_sitter


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).parents[1]))
sys.path.insert(0, str(ROOT / "linters"))

from analysis_session import parse_rust
from production_source import is_production_path, production_source
from rust_style_lint.base import IGNORED_SOURCE_PARENTS
from rust_style_lint.checkers.module_dependency import leading_attributes
ITEM_KINDS = {
    "const_item",
    "enum_item",
    "function_item",
    "static_item",
    "struct_item",
    "trait_item",
    "type_item",
    "union_item",
}
PATHS = {"scoped_identifier", "scoped_type_identifier"}
IDENTIFIERS = {"identifier", "type_identifier", "crate", "self", "super"}
IGNORED = {"line_comment", "block_comment"}
PROC_MACRO = re.compile(
    r"#\s*\[\s*(?:unsafe\s*\(\s*)?proc_macro(?:_derive|_attribute)?\b",
)
TUPLE_INDEX = re.compile(r"^[0-9]+$")


@dataclass
class CrateInfo:
    name: str
    root: Path
    src: Path
    has_lib: bool


@dataclass
class Decl:
    crate: str
    module: tuple[str, ...]
    name: str
    kind: str
    file: Path
    node: tree_sitter.Node
    source: bytes
    visibility: tree_sitter.Node
    owner: str | None = None
    field_index: int | None = None
    used: bool = False
    lint: bool = True


@dataclass
class Module:
    crate: str
    path: tuple[str, ...]
    parent: Module | None = None
    external: bool = False
    module_names: set[str] = field(default_factory=set)
    item_names: set[str] = field(default_factory=set)
    aliases: dict[str, tuple[str, tuple[str, ...]] | None] = field(default_factory=dict)
    globs: set[tuple[str, tuple[str, ...]]] = field(default_factory=set)
    pending_imports: list[tuple] = field(default_factory=list)
    macro_generated: bool = False


@dataclass
class Reference:
    scope: Module
    parts: tuple[str, ...]
    node: tree_sitter.Node
    file: Path
    external: bool = False


@dataclass
class NameReference:
    scope: Module
    name: str
    kind: str
    external: bool = False


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def parts(source: bytes, node: tree_sitter.Node) -> tuple[str, ...]:
    if node.type in IDENTIFIERS:
        return (text(source, node).removeprefix("r#"),)

    if node.type in PATHS:
        base = node.child_by_field_name("path")
        name = node.child_by_field_name("name")

        if name is not None:
            return (
                (parts(source, base) if base is not None else ())
                + (text(source, name).removeprefix("r#"),)
            )

    if node.type in {"generic_type", "generic_function"}:
        base = node.child_by_field_name("type") or node.child_by_field_name("function")

        if base is not None:
            return parts(source, base)

    return ()


def use_leaves(source: bytes, node: tree_sitter.Node, prefix=()):
    if node.type in IGNORED:
        return

    if node.type == "scoped_use_list":
        base = node.child_by_field_name("path")
        group = node.child_by_field_name("list")

        if base is not None and group is not None:
            yield from use_leaves(source, group, prefix + parts(source, base))

        return

    if node.type == "use_list":
        for child in node.named_children:
            yield from use_leaves(source, child, prefix)

        return

    if node.type == "use_as_clause":
        base = node.child_by_field_name("path")
        alias = node.child_by_field_name("alias")

        if base is not None and alias is not None:
            yield prefix + parts(source, base), text(source, alias).removeprefix("r#")

        return

    if node.type == "use_wildcard":
        base = next(
            (child for child in node.named_children if child.type not in IGNORED),
            None,
        )

        if base is not None:
            yield prefix + parts(source, base), None

        return

    path = prefix + parts(source, node)

    if path:
        alias = path[-2] if path[-1] == "self" and len(path) > 1 else path[-1]
        yield path, alias


def visibility(node: tree_sitter.Node, source: bytes) -> tree_sitter.Node | None:
    for child in node.children:
        if child.type == "visibility_modifier" and text(source, child) == "pub":
            return child

    return None


def file_module(src_dir: Path, path: Path) -> tuple[str, ...]:
    relative = path.relative_to(src_dir)

    if relative.name in {"lib.rs", "main.rs"} and len(relative.parts) == 1:
        return ()

    parts_list = list(relative.parts)

    if parts_list[-1] == "mod.rs":
        parts_list.pop()
    else:
        parts_list[-1] = Path(parts_list[-1]).stem

    return tuple(parts_list)


def crate_dirs(root: Path) -> list[Path]:
    roots: list[Path] = []

    for directory, names, files in os.walk(root):
        names[:] = sorted(
            name
            for name in names
            if name not in IGNORED_SOURCE_PARENTS
            and name not in {".uv-cache", ".pnpm-store", "dist", "__pycache__"}
        )
        path = Path(directory)

        if "Cargo.toml" in files and (path / "src").is_dir():
            roots.append(path)

    return sorted(roots)


class Workspace:
    def __init__(self, root: Path) -> None:
        self.root = root.resolve()
        self.crates: dict[str, CrateInfo] = {}
        self.modules: dict[tuple[str, tuple[str, ...]], Module] = {}
        self.declarations: list[Decl] = []
        self.module_decls: dict[tuple[str, tuple[str, ...]], list[Decl]] = {}
        self.decls_by_key: dict[tuple[str, tuple[str, ...], str], list[Decl]] = {}
        self.references: list[Reference] = []
        self.name_references: list[NameReference] = []
        self.pending_imports: list[tuple] = []
        self.lint_current = True

    def add_crate(self, crate: CrateInfo) -> None:
        self.crates[crate.name] = crate

    def module(self, crate: str, path: tuple[str, ...], external: bool = False) -> Module:
        key = (crate, path)
        existing = self.modules.get(key)

        if existing is not None:
            existing.external = existing.external or external

            return existing

        parent = self.module(crate, path[:-1]) if path else None
        module = Module(crate, path, parent, external)
        self.modules[key] = module

        if parent is not None and path:
            parent.module_names.add(path[-1])

        return module

    def ensure_module(self, crate: str, path: tuple[str, ...], external: bool = False) -> Module:
        current = self.module(crate, ())

        for index in range(1, len(path) + 1):
            current = self.module(crate, path[:index])

        current.external = current.external or external

        return current

    def add_decl(self, decl: Decl) -> None:
        decl.lint = self.lint_current
        self.declarations.append(decl)

        if decl.kind == "module":
            key = (decl.crate, decl.module + (decl.name,))
            self.module_decls.setdefault(key, []).append(decl)
        else:
            key = (decl.crate, decl.module, decl.name)
            self.decls_by_key.setdefault(key, []).append(decl)

        if decl.kind == "re-export":
            scope = self.ensure_module(decl.crate, decl.module)
            scope.aliases.setdefault(decl.name, None)

    def discover(self) -> None:
        for crate_root in crate_dirs(self.root):
            manifest = crate_root / "Cargo.toml"

            try:
                config = tomllib.loads(manifest.read_text())
            except (OSError, tomllib.TOMLDecodeError):
                continue

            package = config.get("package", {})
            package_name = package.get("name")

            if not isinstance(package_name, str) or not package_name:
                continue

            lib = config.get("lib", {})
            name = lib.get("name") or package_name.replace("-", "_")
            self.add_crate(CrateInfo(name, crate_root, crate_root / "src", (crate_root / "src/lib.rs").is_file()))

    def collect(self) -> None:
        self.discover()

        for crate in self.crates.values():
            self.collect_crate_declarations(crate)

        for crate in self.crates.values():
            self.collect_crate_references(crate)

        self.resolve_imports()
        self.mark_references()

    def collect_crate_declarations(self, crate: CrateInfo) -> None:
        for path in sorted(crate.src.rglob("*.rs")):
            relative = path.relative_to(crate.root)

            if path.name == "main.rs" and crate.has_lib:
                continue

            if path.parts[len(crate.src.parts) :][:1] == ("bin",) and crate.has_lib:
                continue

            base = file_module(crate.src, path)
            module = self.ensure_module(crate.name, base)
            production = is_production_path(relative)
            source = production_source(path, crate.root) if production else path.read_bytes()
            self.lint_current = production
            self.collect_declarations(module, parse_rust(source).root_node, source, path)

    def collect_declarations(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
    ) -> None:
        for child in node.named_children:
            if child.type in IGNORED or child.type in {"attribute_item", "inner_attribute_item"}:
                continue

            if child.type == "macro_invocation":
                module.macro_generated = True
                continue

            if child.type == "mod_item":
                self.collect_mod_declaration(module, child, source, file)
                continue

            if child.type == "use_declaration":
                self.collect_use_declaration(module, child, source, file)
                continue

            if child.type == "impl_item":
                self.collect_associated_items(module, child, source, file)
                continue

            if child.type not in ITEM_KINDS:
                continue

            name_node = child.child_by_field_name("name")

            if name_node is None:
                continue

            name = text(source, name_node).removeprefix("r#")
            module.item_names.add(name)
            pub = visibility(child, source)

            if pub is not None:
                self.add_decl(
                    Decl(
                        crate=module.crate,
                        module=module.path,
                        name=name,
                        kind=child.type,
                        file=file,
                        node=child,
                        source=source,
                        visibility=pub,
                    )
                )

            if child.type in {"struct_item", "union_item"}:
                self.collect_fields(module, child, name, source, file)

    def collect_mod_declaration(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
    ) -> None:
        name_node = node.child_by_field_name("name")

        if name_node is None:
            return

        name = text(source, name_node).removeprefix("r#")
        module.module_names.add(name)
        child = self.ensure_module(module.crate, module.path + (name,))
        pub = visibility(node, source)

        if pub is not None:
            self.add_decl(
                Decl(
                    crate=module.crate,
                    module=module.path,
                    name=name,
                    kind="module",
                    file=file,
                    node=node,
                    source=source,
                    visibility=pub,
                )
            )

        body = node.child_by_field_name("body")

        if body is not None:
            self.collect_declarations(child, body, source, file)

    def collect_use_declaration(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
    ) -> None:
        pub = visibility(node, source)
        argument = node.child_by_field_name("argument")

        if pub is None or argument is None:
            return

        seen: set[str] = set()

        for _, alias in use_leaves(source, argument):
            if alias is None or alias == "_" or alias in seen:
                continue

            seen.add(alias)
            module.item_names.add(alias)
            self.add_decl(
                Decl(
                    crate=module.crate,
                    module=module.path,
                    name=alias,
                    kind="re-export",
                    file=file,
                    node=node,
                    source=source,
                    visibility=pub,
                )
            )

    def collect_associated_items(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
    ) -> None:
        body = node.child_by_field_name("body")

        if body is None:
            return

        owner_node = node.child_by_field_name("type")
        owner = text(source, owner_node).split("<", 1)[0].split("::")[-1] if owner_node is not None else None

        for child in body.named_children:
            if child.type not in {"function_item", "const_item", "type_item"}:
                continue

            name_node = child.child_by_field_name("name")
            pub = visibility(child, source)

            if name_node is None or pub is None:
                continue

            self.add_decl(
                Decl(
                    crate=module.crate,
                    module=module.path,
                    name=text(source, name_node).removeprefix("r#"),
                    kind="associated",
                    file=file,
                    node=child,
                    source=source,
                    visibility=pub,
                    owner=owner,
                )
            )

    def collect_fields(
        self,
        module: Module,
        node: tree_sitter.Node,
        owner: str,
        source: bytes,
        file: Path,
    ) -> None:
        body = node.child_by_field_name("body")

        if body is None:
            return

        if body.type == "field_declaration_list":
            for child in body.named_children:
                if child.type != "field_declaration":
                    continue

                name_node = child.child_by_field_name("name")
                pub = visibility(child, source)

                if name_node is None or pub is None:
                    continue

                self.add_decl(
                    Decl(
                        crate=module.crate,
                        module=module.path,
                        name=text(source, name_node).removeprefix("r#"),
                        kind="field",
                        file=file,
                        node=child,
                        source=source,
                        visibility=pub,
                        owner=owner,
                    )
                )

            return

        if body.type != "ordered_field_declaration_list":
            return

        index = 0

        for child in body.children:
            if child.type in IGNORED or child.type == ",":
                continue

            if child.type == "visibility_modifier":
                if text(source, child) == "pub":
                    self.add_decl(
                        Decl(
                            crate=module.crate,
                            module=module.path,
                            name=f"@{index}",
                            kind="field",
                            file=file,
                            node=body,
                            source=source,
                            visibility=child,
                            owner=owner,
                            field_index=index,
                        )
                    )

                continue

            if child.is_named:
                index += 1

    def collect_crate_references(self, crate: CrateInfo) -> None:
        for path in sorted((crate.root / "src").rglob("*.rs")):
            if path.name == "main.rs" and crate.has_lib:
                self.collect_external_file(crate, path, ("__main__",))
                continue

            base = file_module(crate.src, path)
            module = self.ensure_module(crate.name, base)
            self.collect_file_references(module, path, parse_rust(path.read_bytes()).root_node)

        for directory in ("tests", "benches", "examples"):
            for path in sorted((crate.root / directory).rglob("*.rs")):
                relative = path.relative_to(crate.root / directory)
                stem = tuple(part.removesuffix(".rs") for part in relative.parts)
                self.collect_external_file(crate, path, ("__external__", directory, *stem))

    def collect_external_file(
        self,
        crate: CrateInfo,
        path: Path,
        module_path: tuple[str, ...],
    ) -> None:
        module = self.ensure_module(crate.name, module_path, external=True)
        module.external = True
        self.collect_file_references(module, path, parse_rust(path.read_bytes()).root_node, external=True)

    def collect_file_references(
        self,
        module: Module,
        path: Path,
        node: tree_sitter.Node,
        external: bool = False,
    ) -> None:
        self.walk_references(module, node, path.read_bytes(), path, external)

    def walk_references(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
        external: bool,
        macro_body: bool = False,
    ) -> None:
        for child in node.named_children:
            if child.type in IGNORED:
                continue

            if child.type == "mod_item":
                name_node = child.child_by_field_name("name")
                body = child.child_by_field_name("body")

                if name_node is not None and body is not None:
                    name = text(source, name_node).removeprefix("r#")
                    child_module = self.ensure_module(module.crate, module.path + (name,), external)
                    child_module.external = child_module.external or external
                    self.walk_references(
                        child_module,
                        body,
                        source,
                        file,
                        external,
                        macro_body,
                    )

                continue

            if child.type == "macro_definition":
                # Macro bodies expand elsewhere; any referenced item is public
                # surface for the expansion site, even when declared locally.
                self.walk_references(module, child, source, file, True, True)
                continue

            if child.type == "use_declaration":
                self.collect_import(module, child, source, file, external)
                continue

            if child.type in PATHS:
                path = parts(source, child)

                if path:
                    if not (
                        external
                        and not macro_body
                        and path[0] in {"crate", "self", "super"}
                    ):
                        self.references.append(
                            Reference(module, path, child, file, external)
                        )
                        self.name_references.append(
                            NameReference(module, path[-1], "path", external)
                        )

            if child.type == "token_tree":
                self.collect_token_paths(
                    module,
                    child,
                    source,
                    file,
                    external,
                    macro_body,
                )

            if child.type == "field_expression":
                self.collect_field_expression(module, child, source, external)

            if child.type == "field_initializer":
                self.collect_named_child(module, child, "field", source, external)

            if child.type in {"field_pattern", "shorthand_field_identifier"}:
                self.collect_named_child(module, child, "field", source, external)

            if child.type in {"attribute_item", "inner_attribute_item"}:
                for content in re.findall(r'"([^"]+)"', text(source, child)):
                    if re.fullmatch(
                        r"(?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*",
                        content,
                    ):
                        self.name_references.append(
                            NameReference(
                                module,
                                content.rsplit("::", 1)[-1],
                                "attribute",
                                external,
                            )
                        )

            self.walk_references(
                module,
                child,
                source,
                file,
                external,
                macro_body,
            )

    def collect_import(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
        external: bool,
    ) -> None:
        argument = node.child_by_field_name("argument")

        if argument is None:
            return

        for path, alias in use_leaves(source, argument):
            if not path:
                continue

            if external and path[0] in {"crate", "self", "super"}:
                continue

            if alias == "_":
                alias = None

            self.references.append(Reference(module, path, node, file))
            self.pending_imports.append((module, path, alias, node, file))

    def collect_token_paths(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        file: Path,
        external: bool,
        macro_body: bool,
    ) -> None:
        tokens = [child for child in node.children if child.type not in IGNORED]
        index = 0

        while index < len(tokens):
            first = tokens[index]
            end = index

            if first.type == "metavariable" and text(source, first) == "$crate":
                path = ["crate"]
            elif first.type in IDENTIFIERS:
                path = [text(source, first)]
            else:
                path = []

            while (
                path
                and end + 2 < len(tokens)
                and tokens[end + 1].type == "::"
                and tokens[end + 2].type in IDENTIFIERS
            ):
                path.append(text(source, tokens[end + 2]))
                end += 2

            if len(path) > 1 and not (
                external
                and not macro_body
                and path[0] in {"crate", "self", "super"}
            ):
                self.references.append(
                    Reference(module, tuple(path), first, file, external)
                )
                self.name_references.append(
                    NameReference(module, path[-1], "path", external)
                )

            if (
                first.type in {"identifier", "field_identifier"}
                and index + 1 < len(tokens)
                and tokens[index + 1].type == ":"
            ):
                self.name_references.append(
                    NameReference(
                        module,
                        text(source, first).removeprefix("r#"),
                        "field",
                        external,
                    )
                )

            if (
                first.type == "."
                and index + 1 < len(tokens)
                and tokens[index + 1].type
                in {"identifier", "field_identifier", "integer_literal"}
            ):
                name = text(source, tokens[index + 1]).removeprefix("r#")
                self.name_references.append(
                    NameReference(module, name, "field", external)
                )
                self.name_references.append(
                    NameReference(module, name, "method", external)
                )

                if TUPLE_INDEX.fullmatch(name):
                    self.name_references.append(
                        NameReference(module, f"@{name}", "tuple", external)
                    )

            index = end + 1

    def collect_field_expression(
        self,
        module: Module,
        node: tree_sitter.Node,
        source: bytes,
        external: bool,
    ) -> None:
        name_node = node.child_by_field_name("field")

        if name_node is None:
            return

        name = text(source, name_node).removeprefix("r#")
        self.name_references.append(NameReference(module, name, "field", external))
        self.name_references.append(NameReference(module, name, "method", external))

        if TUPLE_INDEX.fullmatch(name):
            self.name_references.append(
                NameReference(module, f"@{name}", "tuple", external)
            )

    def collect_named_child(
        self,
        module: Module,
        node: tree_sitter.Node,
        kind: str,
        source: bytes,
        external: bool,
    ) -> None:
        name_node = node.child_by_field_name("name")

        if name_node is None:
            for child in node.named_children:
                if child.type in {"field_identifier", "identifier", "shorthand_field_identifier"}:
                    name_node = child
                    break

        if name_node is not None:
            self.name_references.append(
                NameReference(
                    module,
                    text(source, name_node).removeprefix("r#"),
                    kind,
                    external,
                )
            )

    def resolve_imports(self) -> None:
        limit = min(32, len(self.pending_imports) + len(self.modules) + 1)

        for _ in range(limit):
            changed = False

            for scope, path, alias, _, _ in self.pending_imports:
                result = self.resolve(scope, path)

                if result is None:
                    continue

                crate, target, _ = result

                if alias is None:
                    key = (crate, target)
                    module = self.modules.get(key)

                    if module is None:
                        continue

                    if key not in scope.globs:
                        scope.globs.add(key)
                        changed = True

                    continue

                current = scope.aliases.get(alias)
                target_value = (crate, target)

                if current != target_value:
                    scope.aliases[alias] = target_value
                    changed = True

            if not changed:
                break

    def resolve(
        self,
        scope: Module,
        path: tuple[str, ...],
    ) -> tuple[str, tuple[str, ...], list[Decl]] | None:
        if not path:
            return None

        head, *tail = path

        if head == "crate":
            return self.resolve_absolute(scope.crate, (), tail, [])

        if head == scope.crate and not scope.external:
            return self.resolve_absolute(scope.crate, (), tail, [])

        if head == scope.crate and scope.external:
            return self.resolve_absolute(scope.crate, (), tail, [])

        if head == "self":
            return self.resolve_absolute(scope.crate, scope.path, tail, [])

        if head == "super":
            current = scope.path
            index = 0

            while index < len(path) and path[index] == "super":
                if not current:
                    return None

                current = current[:-1]
                index += 1

            return self.resolve_absolute(scope.crate, current, list(path[index:]), [])

        located = self.resolve_local(scope, head)

        if located is not None:
            crate, target, decls = located

            if (crate, target) in self.modules:
                return self.resolve_absolute(crate, target, tail, decls)

            return crate, target + tuple(tail), decls

        if head in self.crates:
            return self.resolve_absolute(head, (), tail, [])

        return None

    def resolve_local(
        self,
        scope: Module,
        head: str,
    ) -> tuple[str, tuple[str, ...], list[Decl]] | None:
        current: Module | None = scope

        while current is not None:
            alias = current.aliases.get(head)

            if alias is not None:
                crate, target = alias
                decls = self.decls_by_key.get((current.crate, current.path, head), [])

                return crate, target, list(decls)

            for glob_crate, glob_path in current.globs:
                module = self.modules.get((glob_crate, glob_path))

                if module is None:
                    continue

                if head in module.module_names or head in module.item_names:
                    target = glob_path + (head,)

                    if (glob_crate, target) in self.modules:
                        return self.resolve_absolute(glob_crate, target, (), [])

                    return glob_crate, target, []

            target = current.path + (head,)

            if (current.crate, target) in self.modules:
                return current.crate, target, []

            if head in current.item_names:
                return current.crate, target, list(
                    self.decls_by_key.get((current.crate, current.path, head), [])
                )

            current = current.parent

        return None

    def resolve_absolute(
        self,
        crate: str,
        module_path: tuple[str, ...],
        segments: list[str],
        alias_decls: list[Decl],
    ) -> tuple[str, tuple[str, ...], list[Decl]] | None:
        if (crate, module_path) not in self.modules:
            return crate, module_path + tuple(segments), alias_decls

        current = self.modules[(crate, module_path)]

        for index, segment in enumerate(segments):
            if segment == "self":
                continue

            if segment == "super":
                if current.parent is None:
                    return None

                current = current.parent
                continue

            alias = current.aliases.get(segment)

            if alias is not None:
                target_crate, target = alias
                alias_decls.extend(
                    self.decls_by_key.get((crate, current.path, segment), [])
                )

                return self.resolve_absolute(
                    target_crate,
                    target,
                    segments[index + 1 :],
                    alias_decls,
                )

            target = current.path + (segment,)

            if (crate, target) in self.modules:
                current = self.modules[(crate, target)]
                continue

            if segment in current.item_names:
                return crate, target + tuple(segments[index + 1 :]), alias_decls

            return None

        return crate, current.path, alias_decls

    def mark_references(self) -> None:
        for reference in self.references:
            result = self.resolve(reference.scope, reference.parts)

            if result is None:
                continue

            crate, target, alias_decls = result

            for decl in alias_decls:
                self.mark_decl(decl, reference.scope, reference.external)

            self.mark_target(
                crate,
                target,
                reference.scope,
                reference.external,
            )

    def mark_target(
        self,
        crate: str,
        target: tuple[str, ...],
        scope: Module,
        external: bool,
    ) -> None:
        for index in range(1, len(target) + 1):
            prefix = target[:index]

            for decl in self.module_decls.get((crate, prefix), []):
                self.mark_decl(decl, scope, external)

        for index in range(len(target), 0, -1):
            key = (crate, target[: index - 1], target[index - 1])
            decls = self.decls_by_key.get(key)

            if decls:
                for decl in decls:
                    self.mark_decl(decl, scope, external)

                break

    def mark_decl(self, decl: Decl, scope: Module, external: bool) -> None:
        if external or scope.external or scope.crate != decl.crate:
            decl.used = True
            return

        if not is_within(scope.path, decl.module):
            decl.used = True

    def mark_attribute_references(self) -> None:
        for decl in self.declarations:
            if not decl.lint or decl.used:
                continue

            for reference in self.name_references:
                if reference.kind != "attribute" or reference.name != decl.name:
                    continue

                if (
                    reference.external
                    or reference.scope.external
                    or reference.scope.crate != decl.crate
                ):
                    decl.used = True
                    break

                if not is_within(reference.scope.path, decl.module):
                    decl.used = True
                    break

    def mark_named_references(self) -> None:
        for decl in self.declarations:
            if decl.used or decl.kind not in {"associated", "field"}:
                continue

            names = {decl.name}

            for reference in self.name_references:
                if reference.name not in names:
                    continue

                if decl.kind == "field":
                    named = reference.kind == "field"
                    indexed = (
                        reference.kind == "tuple" and decl.field_index is not None
                    )

                    if not named and not indexed:
                        continue
                elif decl.kind == "associated":
                    if reference.kind not in {"method", "path", "tuple"}:
                        continue
                else:
                    continue

                if (
                    reference.external
                    or reference.scope.external
                    or reference.scope.crate != decl.crate
                ):
                    decl.used = True
                    break

                if not is_within(reference.scope.path, decl.module):
                    decl.used = True
                    break

    def mark_public_signature_types(self) -> None:
        for outer in self.declarations:
            if not outer.used or outer.kind == "module":
                continue

            names = signature_names(outer.node, outer.source)
            names.discard(outer.name)

            if not names:
                continue

            for decl in self.declarations:
                if (
                    decl is not outer
                    and decl.crate == outer.crate
                    and decl.kind not in {"field", "module"}
                    and decl.name in names
                ):
                    decl.used = True

    def diagnostics(self) -> list[str]:
        self.mark_named_references()
        self.mark_attribute_references()
        self.mark_public_signature_types()
        messages = set()

        for decl in self.declarations:
            if not decl.lint or decl.used or excused(decl):
                continue

            if decl.kind == "module":
                target = self.modules.get(
                    (decl.crate, decl.module + (decl.name,))
                )

                if target is not None and target.macro_generated:
                    continue

            code, label = diagnostic_parts(decl)
            location = decl.visibility.start_point
            boundary = (
                f"module `crate::{('::'.join(decl.module))}`"
                if decl.module
                else "the crate root"
            )
            messages.add(
                f"{relative(decl.file, self.root)}:{location.row + 1}:"
                f"{location.column + 1}: {code}: `pub {decl.name}` is unnecessary; "
                f"{label} is not referenced outside {boundary}"
            )

        return sorted(messages)


def signature_names(node: tree_sitter.Node, source: bytes) -> set[str]:
    names: set[str] = set()
    pending = [node]

    while pending:
        current = pending.pop()

        if current.type in {"block", "declaration_list"} and current.parent is node:
            continue

        if current.type in {"identifier", "type_identifier"}:
            names.add(text(source, current).removeprefix("r#"))

        pending.extend(reversed(current.named_children))

    return names


def is_within(path: tuple[str, ...], module: tuple[str, ...]) -> bool:
    return len(path) >= len(module) and path[: len(module)] == module


def relative(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def excused(decl: Decl) -> bool:
    if decl.kind in {"module", "field"}:
        return False

    if PROC_MACRO.search(
        " ".join(text(decl.source, item) for item in leading_attributes(decl.node))
    ):
        return True

    return False


def diagnostic_parts(decl: Decl) -> tuple[str, str]:
    if decl.kind == "field":
        return "PUB002", f"public field `{decl.owner}.{decl.name}`"

    if decl.kind == "associated":
        return "PUB003", f"public associated item `{decl.owner}::{decl.name}`"

    if decl.kind == "module":
        return "PUB001", f"public module `{decl.name}`"

    return "PUB001", f"public {decl.kind.removesuffix('_item').replace('_', ' ')} `{decl.name}`"


def check_workspace(root: Path) -> list[str]:
    workspace = Workspace(root)
    workspace.collect()

    return workspace.diagnostics()


def self_test() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(MinimumPubTests)

    return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1


class MinimumPubTests(unittest.TestCase):
    def diagnostics(self, files: dict[str, str], crate: str = "sample") -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                f'[package]\nname = "{crate}"\nversion = "0.0.0"\n'
            )

            for name, source in files.items():
                path = root / "src" / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)

            return check_workspace(root)

    def test_unused_public_items_are_rejected(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod local;\n",
                "local.rs": (
                    "pub struct Unused;\n"
                    "pub fn unused() {}\n"
                    "pub const UNUSED: i32 = 1;\n"
                    "pub struct Fields { pub value: i32 }\n"
                    "pub struct Holder;\n"
                    "impl Holder { pub fn helper(&self) {} }\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 7, "\n".join(diagnostics))
        self.assertTrue(all("PUB00" in item for item in diagnostics))

    def test_cross_module_uses_require_public(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod local;\nmod user;\n",
                "local.rs": "pub struct Shared;\npub fn shared() {}\n",
                "user.rs": (
                    "use crate::local::{Shared, shared};\n"
                    "pub fn consume(_value: Shared) { shared(); }\n"
                ),
            }
        )

        self.assertFalse(
            any("Shared" in item or "shared" in item for item in diagnostics),
            diagnostics,
        )

    def test_descendant_use_does_not_require_public(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod local;\n",
                "local.rs": (
                    "pub struct Parent;\n"
                    "mod child;\n"
                    "mod nested { use super::Parent; pub fn own(_: Parent) {} }\n"
                ),
                "local/child.rs": "use super::Parent;\npub fn own(_: Parent) {}\n",
            }
        )

        self.assertEqual(len(diagnostics), 3, "\n".join(diagnostics))
        self.assertTrue(any("Parent" in item for item in diagnostics), diagnostics)
        self.assertTrue(all("PUB001" in item for item in diagnostics))

    def test_public_fields_and_associated_items_need_outside_use(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod fields;\nmod user;\n",
                "fields.rs": (
                    "pub struct Data { pub value: i32 }\n"
                    "pub struct Helper;\n"
                    "impl Helper { pub fn run(&self) {} }\n"
                ),
                "user.rs": (
                    "use crate::fields::{Data, Helper};\n"
                    "pub fn read(data: &Data, helper: &Helper) { let _ = data.value; helper.run(); }\n"
                ),
            }
        )

        self.assertFalse(
            any("PUB002" in item or "PUB003" in item for item in diagnostics),
            diagnostics,
        )

    def test_public_reexport_needs_outside_use(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod item;\npub use crate::item::Thing;\npub use crate::item::Unused;\n",
                "item.rs": "pub struct Thing;\npub struct Unused;\n",
                "main.rs": (
                    "use sample::Thing;\n"
                    "fn main() { let _ = core::mem::size_of::<Thing>(); }\n"
                ),
            }
        )

        self.assertEqual(sum("Thing" in item for item in diagnostics), 0)
        self.assertGreaterEqual(sum("Unused" in item for item in diagnostics), 1)

    def test_other_crates_count_as_outside_use(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for crate, source in (
                ("base", "pub struct Shared;\n"),
                (
                    "consumer",
                    "use base::Shared;\npub fn take(_: Shared) {}\n",
                ),
            ):
                directory_path = root / crate
                (directory_path / "src").mkdir(parents=True)
                (directory_path / "Cargo.toml").write_text(
                    f'[package]\nname = "{crate}"\nversion = "0.0.0"\n'
                )
                (directory_path / "src/lib.rs").write_text(source)

            diagnostics = check_workspace(root)
            self.assertFalse(
                any("base/src" in item and "Shared" in item for item in diagnostics),
                diagnostics,
            )

    def test_macro_body_references_keep_public(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod generated;\nmod macros;\n",
                "generated.rs": (
                    "#[macro_export]\n"
                    "macro_rules! use_value {\n"
                    "    () => { $crate::macros::VALUE };\n"
                    "}\n"
                ),
                "macros.rs": "pub const VALUE: i32 = 1;\n",
            }
        )

        self.assertEqual(diagnostics, [])

    def test_macro_generated_module_is_not_reported(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "pub mod generated;\n",
                "generated.rs": "define_children! { }\n",
            }
        )

        self.assertEqual(diagnostics, [])

    def test_attribute_string_references_keep_public(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod config;\nmod duration;\n",
                "duration.rs": (
                    "pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>\n"
                    "where D: serde::Deserializer<'de> { unimplemented!() }\n"
                ),
                "config.rs": (
                    "#[derive(serde::Deserialize)]\n"
                    "struct Config { #[serde(deserialize_with = \"duration::deserialize\")] value: u64 }\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_test_module_references_keep_public(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "mod cleanup;\nmod test_shared;\n",
                "cleanup.rs": "pub fn reset() {}\n",
                "test_shared.rs": "pub use crate::cleanup::reset;\n",
            }
        )

        self.assertEqual(diagnostics, [])

    def test_proc_macro_entry_points_are_exempt(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "#[proc_macro]\n"
                    "pub fn generated(input: proc_macro::TokenStream) -> proc_macro::TokenStream { input }\n"
                ),
            },
            crate="sample_macro",
        )

        self.assertEqual(diagnostics, [])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    diagnostics = check_workspace(args.root.resolve())

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
