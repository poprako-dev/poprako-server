#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require direct model/data type imports and bare type uses."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_files, production_source


ROOT = Path(__file__).parents[2]
DATA_ROLES = ("instr", "val", "view")
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    nodes = [node]
    found: list[tree_sitter.Node] = []

    while nodes:
        current = nodes.pop()

        if current.type == kind:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def belongs_to(node: tree_sitter.Node, kind: str) -> bool:
    current = node.parent

    while current is not None:
        if current.type == kind:
            return True

        current = current.parent

    return False


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def layer_domains(root: Path, layer: str) -> set[str]:
    if layer == "data":
        return {
            path.stem
            for role in DATA_ROLES
            for path in production_files(root, f"src/{layer}/{role}")
        }

    return {path.stem for path in production_files(root, f"src/{layer}")}


def legacy_aliases(layer: str, domains: set[str]) -> set[str]:
    return {f"{domain}_{layer}" for domain in domains}


def remove_legacy_imports(path: Path, root: Path, layer: str, domains: set[str]) -> bool:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    aliases = legacy_aliases(layer, domains)
    edits: list[tuple[int, int]] = []

    for declaration in descendants(tree.root_node, "use_declaration"):
        names = {
            node_text(source, identifier)
            for identifier in descendants(declaration, "identifier")
        }
        imported = names - {"crate", layer}

        if imported and imported.issubset(aliases):
            edits.append((declaration.start_byte, declaration.end_byte))

    if not edits:
        return False

    updated = path.read_bytes()

    for start, end in reversed(edits):
        updated = updated[:start] + updated[end:]

    path.write_bytes(updated)
    return True


def check_file(path: Path, root: Path, layer: str, domains: set[str]) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    errors: list[str] = []
    imported_modules: set[str] = set()

    for declaration in descendants(tree.root_node, "use_declaration"):
        for identifier in descendants(declaration, "identifier"):
            name = node_text(source, identifier)

            for domain in domains:
                alias = f"{domain}_{layer}"

                if name == alias:
                    errors.append(
                        f"{path.relative_to(root)}:{identifier.start_point.row + 1}: "
                        f"DIR001: {alias} is forbidden; import the concrete type from "
                        f"crate::{layer}::{domain}",
                    )

        for path_node in descendants(declaration, "scoped_identifier"):
            if path_node.parent is not None and path_node.parent.type == "scoped_identifier":
                continue

            if source[path_node.end_byte : path_node.end_byte + 3] == b"::{":
                continue

            segments = node_text(source, path_node).split("::")

            for domain in domains:
                alias = f"{domain}_{layer}"

                if alias in segments:
                    errors.append(
                        f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                        f"DIR001: {alias} is forbidden; import the concrete type from "
                        f"crate::{layer}::{domain}",
                    )

            if (
                segments[:2] == ["crate", layer]
                and len(segments) == 3
                and segments[2] in domains
            ):
                imported_modules.add(segments[2])
                errors.append(
                    f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                    f"DIR002: import a concrete type from crate::{layer}::{segments[2]}",
                )

            if (
                segments[:2] == ["crate", layer]
                and len(segments) == 3
                and segments[2][:1].isupper()
            ):
                errors.append(
                    f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                    f"DIR003: root {layer} re-exports are forbidden; import its domain type directly",
                )

            if (
                layer == "data"
                and segments[:2] == ["crate", layer]
                and len(segments) == 4
                and segments[2] in DATA_ROLES
                and segments[3] in domains
                and source[path_node.end_byte : path_node.end_byte + 3] != b"::{"
            ):
                imported_modules.add(segments[3])
                errors.append(
                    f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                    f"DIR002: import a concrete type from crate::{layer}::{segments[2]}::{segments[3]}",
                )

    for path_node in descendants(tree.root_node, "scoped_identifier"):
        if belongs_to(path_node, "use_declaration"):
            continue

        segments = node_text(source, path_node).split("::")

        if (
            len(segments) >= 4
            and segments[:2] == ["crate", layer]
            and segments[2] in domains
            and segments[3][:1].isupper()
        ):
            errors.append(
                f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                f"DIR005: import and use {segments[3]} bare",
            )

        if (
            layer == "data"
            and len(segments) >= 5
            and segments[:2] == ["crate", layer]
            and segments[2] in DATA_ROLES
            and segments[3] in domains
            and segments[4][:1].isupper()
        ):
            errors.append(
                f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                f"DIR005: import and use {segments[4]} bare",
            )

        if len(segments) == 2 and segments[0] in imported_modules and segments[1][:1].isupper():
            errors.append(
                f"{path.relative_to(root)}:{path_node.start_point.row + 1}: "
                f"DIR004: import and use {segments[1]} bare",
            )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--layer", choices=("model", "data"), required=True)
    parser.add_argument("--fix", action=argparse.BooleanOptionalAction, default=True)
    args = parser.parse_args()
    root = args.root.resolve()
    domains = layer_domains(root, args.layer)

    if args.fix:
        for path in production_files(root):
            remove_legacy_imports(path, root, args.layer, domains)

    errors = [
        error
        for path in production_files(root)
        for error in check_file(path, root, args.layer, domains)
    ]

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
