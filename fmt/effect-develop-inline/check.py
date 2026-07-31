#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require domain events to be constructed inline for `develop_on`."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
WRAPPERS = {"parenthesized_expression"}


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


def event_names(paths: list[Path], root: Path) -> set[str]:
    names: set[str] = set()

    for path in paths:
        source = production_source(path, root)
        tree = PARSER.parse(source)

        for item in descendants(tree.root_node, "impl_item"):
            item_text = text(source, item)
            marker = "EffectEvent for "

            if marker not in item_text:
                continue

            suffix = item_text.split(marker, 1)[1].lstrip()
            name = suffix.split("<", 1)[0].split(" ", 1)[0].split("{", 1)[0]

            if name.endswith("Event") and name != "Event":
                names.add(name)

    return names


def struct_name(source: bytes, expression: tree_sitter.Node) -> str | None:
    name = expression.child_by_field_name("name")

    if name is None:
        return None

    return text(source, name).split("::")[-1]


def method_name(source: bytes, expression: tree_sitter.Node) -> str | None:
    field = expression.child_by_field_name("field")

    if field is None:
        return None

    return text(source, field)


def is_inline_develop_on(expression: tree_sitter.Node, source: bytes) -> bool:
    parent = expression.parent

    while parent is not None and parent.type in WRAPPERS:
        parent = parent.parent

    if parent is None or parent.type != "field_expression":
        return False

    if method_name(source, parent) != "develop_on":
        return False

    call = parent.parent

    return (
        call is not None
        and call.type == "call_expression"
        and call.child_by_field_name("function") is not None
        and call.child_by_field_name("function").id == parent.id
    )


def has_call_arguments(call: tree_sitter.Node) -> bool:
    arguments = next(
        (child for child in call.children if child.type == "arguments"), None
    )

    return arguments is not None and bool(arguments.named_children)


def is_internal_event_constructor(
    source: bytes,
    function: tree_sitter.Node,
) -> bool:
    segments = text(source, function).split("::")

    return len(segments) >= 2 and segments[-2] == "Event"


def check_file(path: Path, names: set[str], root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    errors: list[str] = []

    for expression in descendants(tree.root_node, "struct_expression"):
        name = struct_name(source, expression)

        if name not in names or is_inline_develop_on(expression, source):
            continue

        errors.append(
            f"{path.relative_to(root)}:{expression.start_point.row + 1}: "
            f"EVD001: construct {name} directly as {name} {{ ... }}.develop_on(develop)",
        )

    if path.relative_to(root) == Path("src/part/effect.rs"):
        return errors

    for call in descendants(tree.root_node, "call_expression"):
        function = call.child_by_field_name("function")

        if (
            function is None
            or function.type != "field_expression"
            or method_name(source, function) != "develop"
            or not has_call_arguments(call)
        ):
            continue

        errors.append(
            f"{path.relative_to(root)}:{call.start_point.row + 1}: "
            "EVD002: dispatch events with XxxEvent { ... }.develop_on(develop)",
        )

    for call in descendants(tree.root_node, "call_expression"):
        function = call.child_by_field_name("function")

        if function is None or not is_internal_event_constructor(source, function):
            continue

        errors.append(
            f"{path.relative_to(root)}:{call.start_point.row + 1}: "
            "EVD003: construct the concrete XxxEvent instead of Event::Variant(...) at a caller",
        )

    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"
        fixture.write_text(
            "struct UserActiveEvent { user_id: String }\n"
            "fn valid(develop: &Develop) {\n"
            "    UserActiveEvent { user_id: String::new() }.develop_on(develop);\n"
            "}\n",
        )

        if check_file(fixture, {"UserActiveEvent"}, root):
            print("self-test: inline event development was rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "struct UserActiveEvent { user_id: String }\n"
            "fn invalid(develop: &Develop) {\n"
            "    let event = UserActiveEvent { user_id: String::new() };\n"
            "    develop.develop(event);\n"
            "    Event::UserActive(UserActiveEvent { user_id: String::new() });\n"
            "}\n",
        )
        diagnostics = check_file(fixture, {"UserActiveEvent"}, root)

        if len(diagnostics) != 4:
            print("self-test: invalid event development was not rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()

    if args.self_test:
        return self_test()

    paths = sorted((root / "src").rglob("*.rs"))
    names = event_names(paths, root)
    errors = [error for path in paths for error in check_file(path, names, root)]

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
