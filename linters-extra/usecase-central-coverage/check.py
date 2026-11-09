#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["tree-sitter==0.25.0", "tree-sitter-rust==0.23.3"]
# ///
"""Require every part/model definition to be reachable from usecase or complex."""

from __future__ import annotations

import argparse
import bisect
from collections import deque
from dataclasses import dataclass, field
from pathlib import Path
import sys
import tomllib
import unittest

import tree_sitter

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "linters"))
sys.path.insert(0, str(ROOT / "linters-extra"))

from analysis_session import parse_rust
from production_source import production_files, production_source
from rust_style_lint.checkers.module_dependency import (
    CfgParser, cfg_expression, file_module, leading_attributes,
)


CONSTRAINED = frozenset({"part", "model"})
ROOTS = frozenset({"usecase", "complex"})

DEFINITIONS = {
    "struct_item": "struct",
    "enum_item": "enum",
    "union_item": "union",
    "trait_item": "trait",
    "type_item": "type",
    "const_item": "const",
    "static_item": "static",
    "function_item": "fn",
}

IGNORED = {"line_comment", "block_comment", "string_literal", "raw_string_literal", "char_literal", "byte_literal"}
PATHS = {"scoped_identifier", "scoped_type_identifier"}
USE_NODES = {"use_declaration", "use_list", "use_as_clause", "scoped_use_list", "use_wildcard"}
IDENTIFIERS = {"identifier", "type_identifier", "crate", "self", "super"}
MISSING = object()


@dataclass
class Scope:
    module: tuple[str, ...]
    parent: Scope | None = None
    imports: list = field(default_factory=list)
    aliases: dict[str, tuple[str, ...] | None] = field(default_factory=dict)
    globs: list[tuple[str, ...]] = field(default_factory=list)
    children: set[str] = field(default_factory=set)
    definitions: dict[str, "Definition"] = field(default_factory=dict)


@dataclass
class Definition:
    module: tuple[str, ...]
    name: str
    kind: str
    file: Path
    node: tree_sitter.Node
    scope: Scope
    attributes: list[tuple[Path, int, int]] = field(default_factory=list)


@dataclass
class Reference:
    scope: Scope
    parts: tuple[str, ...]
    node: tree_sitter.Node
    file: Path


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte:node.end_byte].decode().removeprefix("r#")


def has_direct_test_cfg(node: tree_sitter.Node, source: bytes) -> bool:
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
            yield prefix + parts(source, base), text(source, alias), False, base
        return
    if node.type == "use_wildcard":
        base = next((child for child in node.named_children if child.type not in IGNORED), None)
        yield prefix + (parts(source, base) if base is not None else ()), None, True, node
        return
    path = prefix + parts(source, node)
    if path:
        alias = path[-2] if path[-1] == "self" and len(path) > 1 else path[-1]
        yield path, alias, False, node


def is_associated_item(node: tree_sitter.Node) -> bool:
    current = node.parent
    while current is not None:
        if current.type in {"trait_item", "impl_item"}:
            return True
        current = current.parent
    return False


def is_reference_identifier(node: tree_sitter.Node) -> bool:
    parent = node.parent
    if parent is None:
        return True
    if parent.type in PATHS or parent.type in USE_NODES:
        return False
    if parent.type in DEFINITIONS and parent.child_by_field_name("name") == node:
        return False
    for field_name in ("name", "field", "field_name"):
        if parent.child_by_field_name(field_name) == node and parent.type in {
            "field_declaration", "field_expression", "field_initializer", "enum_variant",
        }:
            return False
    return not (parent.type == "impl_item" and parent.child_by_field_name("type") == node)


class Analyzer:
    def __init__(self, root: Path, crate_name: str):
        self.root = root
        self.crate_name = crate_name
        self.modules: dict[tuple[str, ...], Scope] = {}
        self.scopes: list[Scope] = []
        self.definitions: dict[tuple[str, str], Definition] = {}
        self.references: list[Reference] = []
        self.references_by_file: dict[Path, list[tuple[int, Reference]]] = {}
        self.impl_ranges: dict[tuple[str, str], list[tuple[Path, int, int]]] = {}
        self.errors: list[str] = []

    def module_scope(self, module, parent=None):
        existing = self.modules.get(module)
        if existing is not None:
            return existing
        scope = Scope(module, parent)
        self.modules[module] = scope
        self.scopes.append(scope)
        return scope

    def block_scope(self, parent: Scope):
        scope = Scope(parent.module, parent)
        self.scopes.append(scope)
        return scope

    def index_file(self, file: Path):
        source = production_source(file, self.root)
        tree = parse_rust(source)
        node = tree.root_node
        module = file_module(self.root / "src", file)
        scope = self.module_scope(module)
        if node.has_error and module and module[0] in CONSTRAINED:
            self.errors.append(f"{file.relative_to(self.root)}:1:1: UCC002: cannot reliably inspect malformed Rust")
        self.walk(node, source, file, scope)

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
                    child = self.module_scope(scope.module + (name,), scope)
                    for item in body.named_children:
                        self.walk(item, source, file, child)
            return
        if node.type == "block":
            scope = self.block_scope(scope)
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
                    if alias == "_":
                        alias = None
                    scope.imports.append((path, alias, glob))
                    if alias is not None:
                        scope.aliases[alias] = None
                    self.add_reference(Reference(scope, path, leaf, file))
            return
        if node.type in DEFINITIONS and not is_associated_item(node):
            name = node.child_by_field_name("name")
            if name is not None and scope.module and scope.module[0] in CONSTRAINED:
                definition = Definition(scope.module, text(source, name), DEFINITIONS[node.type], file, node, scope)
                definition.attributes = [
                    (file, attribute.start_byte, attribute.end_byte)
                    for attribute in leading_attributes(node)
                ]
                self.definitions[(scope.module, definition.name)] = definition
                scope.definitions[definition.name] = definition
        if node.type == "impl_item":
            self.index_impl(node, source, file, scope)
        if node.type in PATHS:
            path = parts(source, node)
            if path:
                self.add_reference(Reference(scope, path, node, file))
        if node.type in IDENTIFIERS and is_reference_identifier(node):
            self.add_reference(Reference(scope, (text(source, node),), node, file))
        # Rust macro and attribute arguments are token trees, not path nodes.
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
                    self.add_reference(Reference(scope, tuple(path), first, file))
                index = end + 1
        for child in node.named_children:
            self.walk(child, source, file, scope)

    def add_reference(self, reference: Reference):
        self.references.append(reference)
        self.references_by_file.setdefault(reference.file, []).append((reference.node.start_byte, reference))

    def index_impl(self, node, source, file, scope):
        if not scope.module or scope.module[0] not in CONSTRAINED:
            return
        type_node = node.child_by_field_name("type")
        if type_node is None:
            return
        target = self.resolve_definition(scope, parts(source, type_node))
        if target is None:
            return
        key = (target.module, target.name)
        self.impl_ranges.setdefault(key, []).append((file, node.start_byte, node.end_byte))

    def expand(self, path, seen=frozenset()):
        for index, name in enumerate(path):
            module = self.modules.get(path[:index])
            key = path[:index + 1]
            if module is None or name not in module.aliases or key in seen:
                continue
            target = module.aliases[name]
            if target is not None:
                return self.expand(target + path[index + 1:], seen | {key})
        return path

    def resolve_path(self, scope, path, expand=True):
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
                target = current.aliases.get(head, MISSING)
                if target is not MISSING:
                    if target is None:
                        return None
                    combined = target + tuple(tail)
                    return self.expand(combined) if expand else combined
                if head in current.definitions or head in current.children:
                    combined = current.module + (head,) + tuple(tail)
                    return self.expand(combined) if expand else combined
                for glob in current.globs:
                    module = self.modules.get(glob)
                    if module is not None and (head in module.definitions or head in module.children or head in module.aliases):
                        combined = glob + (head,) + tuple(tail)
                        return self.expand(combined) if expand else combined
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

    def lookup_definition(self, path):
        if not path:
            return None
        module_path, name = path[:-1], path[-1]
        found = self.definitions.get((module_path, name))
        if found is not None:
            return found
        module = self.modules.get(module_path)
        if module is None or name not in module.aliases:
            return None
        target = module.aliases[name]
        return None if target is None else self.lookup_definition(self.expand(target))

    def resolve_definition(self, scope, path):
        resolved = self.resolve_path(scope, path)
        return None if resolved is None else self.lookup_definition(resolved)

    def settle_aliases(self):
        # Imports can refer to aliases declared later or in a different file.
        for _ in range(sum(len(scope.imports) for scope in self.scopes) + 1):
            changed = False
            for scope in self.scopes:
                for path, alias, glob in scope.imports:
                    target = self.resolve_path(scope, path)
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

    def body_ranges(self, definition: Definition):
        yield definition.file, definition.node.start_byte, definition.node.end_byte
        # Contract attributes such as `#[oper(output = ...)]` belong to the item.
        yield from definition.attributes
        yield from self.impl_ranges.get((definition.module, definition.name), [])

    def body_references(self, definition: Definition):
        for file, start, end in self.body_ranges(definition):
            entries = self.references_by_file.get(file)
            if not entries:
                continue
            index = bisect.bisect_left([entry[0] for entry in entries], start)
            while index < len(entries) and entries[index][0] < end:
                yield entries[index][1]
                index += 1

    def coverage(self):
        reached: set[tuple[tuple[str, ...], str]] = set()
        pending = deque(
            reference for reference in self.references
            if reference.scope.module and reference.scope.module[0] in ROOTS
        )
        while pending:
            reference = pending.popleft()
            definition = self.resolve_definition(reference.scope, reference.parts)
            if definition is None or definition.module[0] not in CONSTRAINED:
                continue
            key = (definition.module, definition.name)
            if key in reached:
                continue
            reached.add(key)
            pending.extend(self.body_references(definition))
        return reached

    def check(self):
        for file in production_files(self.root):
            if file.name == "main.rs" or file.name == "schema.rs":
                continue
            self.index_file(file)
        self.settle_aliases()
        reached = self.coverage()
        diagnostics = set(self.errors)
        for key, definition in sorted(self.definitions.items()):
            if key in reached:
                continue
            node = definition.node
            diagnostics.add(
                f"{definition.file.relative_to(self.root)}:{node.start_point.row + 1}:{node.start_point.column + 1}: "
                f"UCC001: `{definition.kind} {definition.name}` in `{definition.module[0]}` is not reachable "
                f"from usecase or complex"
            )
        return sorted(diagnostics)


def check_root(root: Path) -> list[str]:
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        return [f"{manifest}:1:1: UCC002: Cargo.toml is missing"]
    try:
        config = tomllib.loads(manifest.read_text())
    except tomllib.TOMLDecodeError as error:
        return [f"{manifest}:1:1: UCC002: cannot inspect manifest: {error}"]
    if config.get("package", {}).get("name") != "poprako-server":
        return []
    if not (root / "src/lib.rs").is_file():
        return [f"{root}:1:1: UCC002: server src/lib.rs is missing"]
    crate_name = config.get("lib", {}).get("name", "poprako_server")
    return Analyzer(root, crate_name).check()


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
