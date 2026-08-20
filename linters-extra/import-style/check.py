#!/usr/bin/env python3
"""Require explicit production imports rooted at `crate` for local paths."""

from __future__ import annotations

import argparse
import sys
import tempfile
import tomllib
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))

from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
PATH_NODE_TYPES = {"crate", "identifier", "scoped_identifier", "self", "super"}


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def normalize_identifier(value: str) -> str:
    return value.removeprefix("r#")


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    pending = [node]

    while pending:
        current = pending.pop()

        if current.type == kind:
            found.append(current)

        pending.extend(reversed(current.named_children))

    return found


def parse_errors(node: tree_sitter.Node) -> list[tree_sitter.Node]:
    errors: list[tree_sitter.Node] = []
    pending = [node]

    while pending:
        current = pending.pop()

        if current.type == "ERROR" or current.is_missing:
            errors.append(current)

        pending.extend(reversed(current.named_children))

    return errors


def path_parts(source: bytes, node: tree_sitter.Node) -> tuple[str, ...]:
    if node.type in {"crate", "identifier", "self", "super"}:
        return (normalize_identifier(node_text(source, node)),)

    if node.type == "scoped_identifier":
        path = node.child_by_field_name("path")
        name = node.child_by_field_name("name")

        if path is not None and name is not None:
            return path_parts(source, path) + path_parts(source, name)

    return tuple(
        normalize_identifier(part.strip())
        for part in node_text(source, node).split("::")
        if part.strip()
    )


def imported_paths(
    source: bytes,
    node: tree_sitter.Node,
    prefix: tuple[str, ...] = (),
) -> list[tuple[tuple[str, ...], tree_sitter.Node]]:
    if node.type == "scoped_use_list":
        path = node.child_by_field_name("path")
        use_list = node.child_by_field_name("list")

        if path is None or use_list is None:
            return []

        return imported_paths(source, use_list, prefix + path_parts(source, path))

    if node.type == "use_list":
        return [
            imported
            for child in node.named_children
            for imported in imported_paths(source, child, prefix)
        ]

    if node.type == "use_as_clause":
        path = node.child_by_field_name("path")

        return [] if path is None else [(prefix + path_parts(source, path), path)]

    if node.type == "use_wildcard":
        path = next(
            (child for child in node.named_children if child.type in PATH_NODE_TYPES),
            None,
        )
        parts = () if path is None else path_parts(source, path)

        return [(prefix + parts + ("*",), node)]

    if node.type in PATH_NODE_TYPES:
        return [(prefix + path_parts(source, node), node)]

    return []


def direct_module_names(path: Path) -> set[str]:
    if not path.is_file():
        return set()

    source = path.read_bytes()
    tree = PARSER.parse(source)
    names: set[str] = set()

    for node in tree.root_node.named_children:
        if node.type != "mod_item":
            continue

        name = node.child_by_field_name("name")

        if name is not None:
            names.add(normalize_identifier(node_text(source, name)))

    return names


def parent_module_files(path: Path, root: Path) -> list[Path]:
    src = root / "src"
    relative = path.relative_to(src)
    files = [src / "lib.rs", src / "main.rs"]
    parts = list(relative.parts[:-1])

    if path.name not in {"lib.rs", "main.rs"}:
        files.append(path)

    for depth in range(1, len(parts) + 1):
        files.append(src.joinpath(*parts[:depth]).with_suffix(".rs"))

    return files


def lib_crate_name(root: Path) -> str:
    with (root / "Cargo.toml").open("rb") as cargo_file:
        manifest = tomllib.load(cargo_file)

    explicit_name = manifest.get("lib", {}).get("name")

    if explicit_name is not None:
        return normalize_identifier(explicit_name)

    return manifest["package"]["name"].replace("-", "_")


def self_aliases(source: bytes, tree: tree_sitter.Tree) -> set[str]:
    aliases: set[str] = set()

    for declaration in descendants(tree.root_node, "extern_crate_declaration"):
        crate = next(
            (child for child in declaration.named_children if child.type == "crate"),
            None,
        )
        alias = declaration.child_by_field_name("alias")

        if crate is not None and alias is not None:
            aliases.add(normalize_identifier(node_text(source, alias)))

    return aliases


def local_roots(path: Path, root: Path, source: bytes, tree: tree_sitter.Tree) -> set[str]:
    roots = self_aliases(source, tree)

    if path.name != "main.rs":
        roots.add(lib_crate_name(root))

    for module_file in parent_module_files(path, root):
        roots.update(direct_module_names(module_file))

    return roots


def diagnostic(path: Path, root: Path, node: tree_sitter.Node, code: str, message: str) -> str:
    return (
        f"{path.relative_to(root)}:{node.start_point.row + 1}:"
        f"{node.start_point.column + 1}: {code}: {message}"
    )


def check(root: Path) -> list[str]:
    root = root.resolve()
    violations: list[str] = []

    for path in sorted((root / "src").rglob("*.rs")):
        source = production_source(path, root)
        tree = PARSER.parse(source)

        for error in parse_errors(tree.root_node):
            violations.append(
                diagnostic(path, root, error, "IMPORT003", "Rust source contains a parse error"),
            )

        roots = local_roots(path, root, source, tree)

        for declaration in descendants(tree.root_node, "use_declaration"):
            argument = declaration.child_by_field_name("argument")

            if argument is None:
                continue

            imported = imported_paths(source, argument)

            if any(parts and parts[-1] == "*" for parts, _ in imported):
                violations.append(
                    diagnostic(
                        path,
                        root,
                        declaration,
                        "IMPORT001",
                        "wildcard imports are forbidden in production source",
                    ),
                )

            invalid_roots = {
                parts[0]
                for parts, _ in imported
                if parts and (parts[0] in {"self", "super"} or parts[0] in roots)
            }

            if invalid_roots:
                violations.append(
                    diagnostic(
                        path,
                        root,
                        declaration,
                        "IMPORT002",
                        "current-crate imports must start with `crate::`",
                    ),
                )

    return violations


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        src = root / "src"
        src.mkdir()
        (root / "Cargo.toml").write_text(
            '[package]\nname = "renamed-package"\nversion = "0.1.0"\n'
            '[lib]\nname = "custom_lib"\n',
        )
        (src / "lib.rs").write_text(
            "mod domain;\n"
            "extern crate self as local;\n"
            "use crate::domain::Item;\n"
            "use {domain::Other, std::fmt};\n"
            "use custom_lib::Thing;\n"
            "use local::AliasThing;\n"
            "use r#domain::RawThing;\n"
            "use std::collections::*;\n"
            "#[cfg(test)]\nmod tests;\n"
            "mod outer { mod nested {} }\n"
            "use nested::ExternalThing;\n",
        )
        (src / "domain.rs").write_text("pub struct Item;\n")
        (src / "main.rs").write_text("use custom_lib::Thing;\n")
        (src / "schema.rs").write_text("use std::prelude::*;\n")
        (src / "tests.rs").write_text("use super::*;\n")
        violations = check(root)
        codes = [violation.split(": ", 1)[1].split(":", 1)[0] for violation in violations]

        if codes != ["IMPORT002", "IMPORT002", "IMPORT002", "IMPORT002", "IMPORT001", "IMPORT001"]:
            print("self-test: import boundary cases produced unexpected diagnostics", file=sys.stderr)
            print("\n".join(violations), file=sys.stderr)
            return 1

        (src / "broken.rs").write_text("fn {\n")
        violations = check(root)

        if not any("IMPORT003" in violation for violation in violations):
            print("self-test: expected a parse-error diagnostic", file=sys.stderr)
            print("\n".join(violations), file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="import-style")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    violations = check(args.root)

    for violation in violations:
        print(violation, file=sys.stderr)

    return 1 if violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
