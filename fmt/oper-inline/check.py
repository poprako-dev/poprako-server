#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require every poprako_orchestra Oper construction to be inline."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import tree_sitter
import tree_sitter_rust


ROOT = Path(__file__).parents[2]
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


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def oper_names(paths: list[Path]) -> set[str]:
    names: set[str] = set()

    for path in paths:
        source = path.read_bytes()
        tree = PARSER.parse(source)

        for item in descendants(tree.root_node, "impl_item"):
            item_text = text(source, item)
            marker = "Oper for "

            if marker not in item_text:
                continue

            suffix = item_text.split(marker, 1)[1].lstrip()
            name = suffix.split("<", 1)[0].split(" ", 1)[0].split("{", 1)[0]

            if name[:1].isupper():
                names.add(name)

    return names


def constructor_name(source: bytes, value: tree_sitter.Node) -> str | None:
    if value.type == "struct_expression":
        name = value.child_by_field_name("name")

        if name is not None:
            return text(source, name).split("::")[-1]

    if value.type == "call_expression":
        function = value.child_by_field_name("function")

        if function is not None:
            segments = text(source, function).split("::")

            if len(segments) >= 2 and segments[-1] == "new":
                return segments[-2]

    return None


def binding_name(source: bytes, declaration: tree_sitter.Node) -> str | None:
    pattern = declaration.child_by_field_name("pattern")

    if pattern is None:
        return None

    identifiers = descendants(pattern, "identifier")

    if len(identifiers) != 1:
        return None

    return text(source, identifiers[0])


def next_statement(declaration: tree_sitter.Node) -> tree_sitter.Node | None:
    parent = declaration.parent

    if parent is None:
        return None

    siblings = parent.named_children

    for index, sibling in enumerate(siblings):
        if sibling.id == declaration.id and index + 1 < len(siblings):
            return siblings[index + 1]

    return None


def safe_inline_edits(path: Path, names: set[str]) -> list[tuple[int, int, bytes]]:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    edits: list[tuple[int, int, bytes]] = []

    for declaration in descendants(tree.root_node, "let_declaration"):
        value = declaration.child_by_field_name("value")

        if value is None or constructor_name(source, value) not in names:
            continue

        bound_name = binding_name(source, declaration)
        statement = next_statement(declaration)

        if bound_name is None or statement is None:
            continue

        references = [
            identifier
            for identifier in descendants(statement, "identifier")
            if text(source, identifier) == bound_name
        ]

        if len(references) != 1:
            continue

        reference = references[0]
        replacement = reference
        parent = reference.parent

        if parent is not None and parent.type == "reference_expression":
            replacement = parent
            parent = parent.parent

        while parent is not None and parent.type == "parenthesized_expression":
            parent = parent.parent

        if parent is None or parent.type != "arguments":
            continue

        edits.append((declaration.start_byte, declaration.end_byte, b""))
        edits.append((replacement.start_byte, replacement.end_byte, source[value.start_byte : value.end_byte]))

    return edits


def apply_safe_fixes(paths: list[Path], names: set[str]) -> None:
    for path in paths:
        edits = safe_inline_edits(path, names)

        if not edits:
            continue

        source = path.read_bytes()

        for start, end, replacement in sorted(edits, reverse=True):
            source = source[:start] + replacement + source[end:]

        path.write_bytes(source)


def apply_missing_borrows(paths: list[Path], names: set[str]) -> None:
    for path in paths:
        source = path.read_bytes()
        tree = PARSER.parse(source)
        edits: list[tuple[int, bytes]] = []

        for expression in descendants(tree.root_node, "struct_expression"):
            name = expression.child_by_field_name("name")

            if name is None or text(source, name).split("::")[-1] not in names:
                continue

            parent = expression.parent

            while parent is not None and parent.type == "parenthesized_expression":
                parent = parent.parent

            if parent is None or parent.type != "arguments":
                continue

            if expression.parent is not None and expression.parent.type == "reference_expression":
                continue

            edits.append((expression.start_byte, b"&"))

        for start, replacement in reversed(edits):
            source = source[:start] + replacement + source[start:]

        if edits:
            path.write_bytes(source)


def check_file(path: Path, names: set[str], root: Path) -> list[str]:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    errors: list[str] = []

    for declaration in descendants(tree.root_node, "let_declaration"):
        value = declaration.child_by_field_name("value")

        if value is None:
            continue

        name = constructor_name(source, value)

        if name not in names:
            continue

        errors.append(
            f"{path.relative_to(root)}:{declaration.start_point.row + 1}: "
            f"OPR001: construct {name} directly in its consuming run or step argument",
        )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--fix-safe", action="store_true")
    parser.add_argument("--fix-borrows", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    paths = sorted((root / "src").rglob("*.rs"))
    names = oper_names(paths)

    if args.fix_safe:
        apply_safe_fixes(paths, names)

    if args.fix_borrows:
        apply_missing_borrows(paths, names)

    errors = [error for path in paths for error in check_file(path, names, root)]

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
