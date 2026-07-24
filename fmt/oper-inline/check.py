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
import tempfile
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
            segments = text(source, name).split("::")

            if len(segments) >= 2:
                return segments[-2]

            return segments[-1]

    if value.type in {"identifier", "scoped_identifier"}:
        return text(source, value).split("::")[-1]

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

        expressions = [
            expression
            for kind in {"identifier", "scoped_identifier", "struct_expression"}
            for expression in descendants(tree.root_node, kind)
        ]

        for expression in expressions:
            if constructor_name(source, expression) not in names:
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
            f"OPR001: construct {name} directly in its consuming call argument",
        )

    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"
        fixture.write_text(
            "trait Oper {}\n"
            "struct Create;\n"
            "impl Oper for Create {}\n"
            "enum Get { Id { id: String } }\n"
            "impl Oper for Get {}\n"
            "fn valid() {\n"
            "    consume(&Create);\n"
            "    consume(&Get::Id { id: String::new() });\n"
            "}\n",
        )

        if check_file(fixture, oper_names([fixture]), root):
            print("self-test: inline operations were rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "trait Oper {}\n"
            "struct Create;\n"
            "impl Oper for Create {}\n"
            "enum Get { Id { id: String } }\n"
            "impl Oper for Get {}\n"
            "fn invalid() {\n"
            "    let create = Create;\n"
            "    consume(&create);\n"
            "    let get = Get::Id { id: String::new() };\n"
            "    consume(&get);\n"
            "}\n",
        )
        diagnostics = check_file(fixture, oper_names([fixture]), root)

        if len(diagnostics) != 2:
            print("self-test: bound operations were not fully rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        names = oper_names([fixture])
        apply_safe_fixes([fixture], names)
        apply_missing_borrows([fixture], names)

        if check_file(fixture, names, root):
            print("self-test: safe fixes did not inline operations", file=sys.stderr)
            return 1

        fixed = fixture.read_text()

        if "consume(&Get::Id" not in fixed:
            print("self-test: safe fixes did not retain the operation borrow", file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--fix-safe", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--fix-borrows", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    paths = sorted((root / "src").rglob("*.rs"))
    names = oper_names(paths)

    if args.self_test:
        return self_test()

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
