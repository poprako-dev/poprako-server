#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["tree-sitter==0.25.0", "tree-sitter-rust==0.23.3"]
# ///
"""Reject explicitly forbidden PopRaKo server layer dependencies."""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
from pathlib import Path
import sys
import tomllib
import unittest

import tree_sitter
import tree_sitter_rust

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "linters"))
sys.path.insert(0, str(ROOT / "linters-extra"))

from production_source import production_files, production_source
from rust_style_lint.checkers.module_dependency import (
    CfgParser, cfg_expression, file_module, leading_attributes,
)


# This is a denylist, never an exhaustive list of permitted dependencies.
# Unknown source/target modules and new submodules remain unrestricted.
FORBIDDEN = {
    "value": frozenset({"model", "complex", "data", "part", "usecase", "api", "extra", "part_impl", "harn", "shared"}),
    "model": frozenset({"complex", "data", "part", "usecase", "api", "extra", "part_impl", "harn", "shared"}),
    "complex": frozenset({"data", "part", "usecase", "api", "extra", "part_impl", "harn", "shared"}),
    "data": frozenset({"complex", "part", "usecase", "api", "extra", "part_impl", "harn", "shared"}),
    "part": frozenset({"complex", "data", "usecase", "api", "extra", "part_impl", "harn", "shared"}),
    "usecase": frozenset({"api", "extra", "part_impl", "harn", "shared"}),
}
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
IGNORED = {"line_comment", "block_comment", "string_literal", "raw_string_literal", "char_literal", "byte_literal"}
PATHS = {"scoped_identifier", "scoped_type_identifier"}
IDENTIFIERS = {"identifier", "type_identifier", "crate", "self", "super"}


@dataclass
class Scope:
    module: tuple[str, ...]
    parent: Scope | None = None
    imports: list = field(default_factory=list)
    aliases: dict[str, tuple[str, ...] | None] = field(default_factory=dict)
    globs: list[tuple[str, ...]] = field(default_factory=list)
    children: set[str] = field(default_factory=set)


@dataclass
class Reference:
    scope: Scope
    parts: tuple[str, ...]
    node: tree_sitter.Node
    file: Path


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte:node.end_byte].decode().removeprefix("r#")


def has_direct_test_cfg(node: tree_sitter.Node, source: bytes) -> bool:
    # The walker already prunes excluded ancestors before visiting children.
    for attribute in leading_attributes(node):
        expression = cfg_expression(text(source, attribute))
        if expression is not None and True not in CfgParser(expression).parse():
            return True
    return False


def parts(source: bytes, node: tree_sitter.Node) -> tuple[str, ...]:
    if node.type in IDENTIFIERS:
        return (text(source, node),)
    if node.type in PATHS:
        base = node.child_by_field_name("path")
        name = node.child_by_field_name("name")
        if name is not None:
            return (parts(source, base) if base is not None else ()) + (text(source, name),)
    if node.type in {"generic_type", "generic_function"}:
        base = node.child_by_field_name("type") or node.child_by_field_name("function")
        if base is not None:
            return parts(source, base)
    return ()


def use_leaves(source: bytes, node: tree_sitter.Node, prefix=()):
    """Expand nested use trees without interpreting comments as paths."""
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
            path = prefix + parts(source, base)
            yield path, text(source, alias), False, base
        return
    if node.type == "use_wildcard":
        base = next((child for child in node.named_children if child.type not in IGNORED), None)
        yield prefix + (parts(source, base) if base is not None else ()), None, True, node
        return
    path = prefix + parts(source, node)
    if path:
        alias = path[-2] if path[-1] == "self" and len(path) > 1 else path[-1]
        yield path, alias, False, node


class Scanner:
    def __init__(self, root: Path, crate_name: str):
        self.root = root
        self.crate_name = crate_name
        self.scopes: list[Scope] = []
        self.modules: dict[tuple[str, ...], Scope] = {}
        self.references: list[Reference] = []
        self.errors: list[str] = []

    def scope(self, module, parent=None, module_scope=False):
        scope = Scope(module, parent)
        self.scopes.append(scope)
        if module_scope:
            self.modules[module] = scope
        return scope

    def walk(self, node, source, file, scope):
        if node.type in IGNORED or has_direct_test_cfg(node, source):
            return
        if node.type == "mod_item":
            name = node.child_by_field_name("name")
            body = node.child_by_field_name("body")
            if name is not None:
                name = text(source, name)
                scope.children.add(name)
                if body is not None:
                    child_scope = self.scope(scope.module + (name,), module_scope=True)
                    for child in body.named_children:
                        self.walk(child, source, file, child_scope)
            return
        if node.type == "block":
            scope = self.scope(scope.module, scope)
        if node.type == "extern_crate_declaration":
            name = node.child_by_field_name("name")
            alias = node.child_by_field_name("alias") or name
            if name is not None and alias is not None:
                name, alias = text(source, name), text(source, alias)
                path = ("crate",) if name in {"self", self.crate_name} else (name,)
                scope.aliases[alias] = None
                scope.imports.append((path, alias, False))
            return
        if node.type == "use_declaration":
            argument = node.child_by_field_name("argument")
            if argument is not None:
                for path, alias, glob, leaf in use_leaves(source, argument):
                    # Anonymous trait imports remain references, not bindings.
                    if alias == "_":
                        alias = None
                    scope.imports.append((path, alias, glob))
                    if alias is not None:
                        scope.aliases[alias] = None
                    self.references.append(Reference(scope, path, leaf, file))
            return
        if node.type in PATHS:
            path = parts(source, node)
            if path:
                self.references.append(Reference(scope, path, node, file))
        # Rust macro and attribute arguments are token trees, not path nodes.
        # Inspect actual identifier/:: tokens; literals and comments stay opaque.
        if node.type == "token_tree":
            tokens = [child for child in node.children if child.type not in {"line_comment", "block_comment"}]
            index = 0
            while index < len(tokens):
                first = tokens[index]
                end = index
                path = [text(source, first)] if first.type in IDENTIFIERS else []
                while path and end + 2 < len(tokens) and tokens[end + 1].type == "::" and tokens[end + 2].type in IDENTIFIERS:
                    path.append(text(source, tokens[end + 2]))
                    end += 2
                if len(path) > 1:
                    self.references.append(Reference(scope, tuple(path), first, file))
                index = end + 1
        for child in node.named_children:
            self.walk(child, source, file, scope)

    def expand(self, path, seen=frozenset()):
        """Follow module import aliases, including re-exports and root aliases."""
        for index, name in enumerate(path):
            module = self.modules.get(path[:index])
            key = path[:index + 1]
            if module is None or name not in module.aliases or key in seen:
                continue
            target = module.aliases[name]
            if target is not None:
                return self.expand(target + path[index + 1:], seen | {key})
        return path

    def resolve(self, scope, path, expand=True):
        if not path:
            return ()
        head, *tail = path
        if head in {"crate", self.crate_name}:
            base = ()
        elif head == "self":
            base = scope.module
        elif head == "super":
            base = scope.module
            tail.insert(0, head)
        else:
            current = scope
            while current is not None:
                if head in current.aliases:
                    target = current.aliases[head]
                    if target is None:
                        return None
                    return self.expand(target + tuple(tail)) if expand else target + tuple(tail)
                if head in current.children:
                    return self.expand(current.module + path) if expand else current.module + path
                for glob in current.globs:
                    module = self.modules.get(glob)
                    if module is not None and (head in module.children or head in module.aliases):
                        return self.expand(glob + path) if expand else glob + path
                current = current.parent
            return None
        result = list(base)
        for part in tail:
            if part == "self":
                continue
            if part == "super":
                if not result:
                    return None
                result.pop()
                continue
            result.append(part)
        return self.expand(tuple(result)) if expand else tuple(result)

    def check(self):
        for file in production_files(self.root):
            if file.name == "main.rs" or file.name == "schema.rs":
                continue
            source = production_source(file, self.root)
            node = PARSER.parse(source).root_node
            module = file_module(self.root / "src", file)
            scope = self.scope(module, module_scope=True)
            if node.has_error and module and module[0] in FORBIDDEN:
                self.errors.append(f"{file.relative_to(self.root)}:1:1: LYR002: cannot reliably inspect malformed Rust")
            self.walk(node, source, file, scope)
        # Imports can refer to aliases declared later or in a different file.
        for _ in range(sum(len(scope.imports) for scope in self.scopes) + 1):
            changed = False
            for scope in self.scopes:
                for path, alias, glob in scope.imports:
                    target = self.resolve(scope, path)
                    if target is None:
                        continue
                    if glob and target not in scope.globs:
                        scope.globs.append(target)
                        changed = True
                    if alias is not None and scope.aliases[alias] != target:
                        scope.aliases[alias] = target
                        changed = True
            if not changed:
                break
        diagnostics = set(self.errors)
        for reference in self.references:
            module = reference.scope.module
            if not module or module[0] not in FORBIDDEN:
                continue
            targets = {self.resolve(reference.scope, reference.parts, expand) for expand in (False, True)}
            for target in targets:
                if target and target[0] in FORBIDDEN[module[0]]:
                    node = reference.node
                    diagnostics.add(
                        f"{reference.file.relative_to(self.root)}:{node.start_point.row + 1}:{node.start_point.column + 1}: "
                        f"LYR001: forbidden layer dependency `{module[0]} -> {target[0]}` "
                        f"via `{'::'.join(reference.parts)}`"
                    )
        return sorted(diagnostics)


def check_root(root: Path) -> list[str]:
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        return [f"{manifest}:1:1: LYR002: Cargo.toml is missing"]
    try:
        config = tomllib.loads(manifest.read_text())
    except tomllib.TOMLDecodeError as error:
        return [f"{manifest}:1:1: LYR002: cannot inspect manifest: {error}"]
    # The runner also visits infrastructure crates; server policy does not apply there.
    if config.get("package", {}).get("name") != "poprako-server":
        return []
    if not (root / "src/lib.rs").is_file():
        return [f"{root}:1:1: LYR002: server src/lib.rs is missing"]
    crate_name = config.get("lib", {}).get("name", "poprako_server")
    return Scanner(root, crate_name).check()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.discover(str(Path(__file__).parent), pattern="test_check.py")
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    diagnostics = check_root(args.root.resolve())
    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
