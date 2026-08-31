#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require domain events to be constructed inline on `.develop_on(develop)`.

Events are dispatched through the [`Event`] enum: each variant carries a
concrete payload struct, and dispatch happens by constructing
``Event::Variant(Payload { ... }).develop_on(develop)`` inline at the call
site. The checker enforces two rules:

Rules
-----

* **EVD001** — an ``Event::Variant(...)`` construction must be the inline
  receiver of ``.develop_on(develop)``. Binding an event to a variable before
  dispatching it (or constructing it without an immediate dispatch) is
  forbidden.
* **EVD002** — callers must never invoke ``Develop::develop`` directly;
  dispatch through ``Event::Variant(...).develop_on(develop)`` instead.

[`Event`]: crate::part::effect::event::Event
"""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_files, production_source


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


def method_name(source: bytes, expression: tree_sitter.Node) -> str | None:
    field = expression.child_by_field_name("field")

    if field is None:
        return None

    return text(source, field)


def has_call_arguments(call: tree_sitter.Node) -> bool:
    arguments = next(
        (child for child in call.children if child.type == "arguments"), None
    )

    return arguments is not None and bool(arguments.named_children)


def is_event_constructor(source: bytes, function: tree_sitter.Node) -> bool:
    """Return True when `function` names an `Event::Variant` constructor.

    The constructor is the field expression ``Event::Variant`` whose receiver
    is the literal ``Event`` enum path. A dispatch like ``Event::Variant(...)``
    ``.develop_on(...)`` surfaces the whole receiver as the outer call's
    ``function``; requiring the receiver to be exactly ``Event`` keeps the
    outer ``.develop_on`` call from being mistaken for a construction.
    """
    if function.type != "scoped_identifier":
        return False

    return text(source, function).split("::", 1)[0] == "Event"


def is_inline_develop_on(receiver: tree_sitter.Node, source: bytes) -> bool:
    """Return True when `receiver` is the inline receiver of `.develop_on(...)`."""
    parent = receiver.parent

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


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    errors: list[str] = []

    skip_dispatch = path.relative_to(root) == Path("src/part/effect.rs")

    # EVD001 — every Event::Variant(...) construction must be dispatched
    # inline on .develop_on(develop); do not bind an event first.
    for call in descendants(tree.root_node, "call_expression"):
        function = call.child_by_field_name("function")

        if function is None or not is_event_constructor(source, function):
            continue

        if is_inline_develop_on(call, source):
            continue

        errors.append(
            f"{path.relative_to(root)}:{call.start_point.row + 1}: "
            "EVD001: construct Event::Variant(...) inline on .develop_on(develop); "
            "do not bind an event before dispatching it",
        )

    # EVD002 — never call Develop::develop(...) at a caller.
    if not skip_dispatch:
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
                "EVD002: dispatch events with Event::Variant(...).develop_on(develop)",
            )

    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"

        # ── valid: inline Event::Variant(...).develop_on(develop) ─────────
        fixture.write_text(
            "enum Event { UserActive(UserActiveEvent) }\n"
            "struct UserActiveEvent { user_id: String }\n"
            "fn valid(develop: &Develop) {\n"
            "    Event::UserActive(UserActiveEvent { user_id: String::new() })\n"
            "        .develop_on(develop);\n"
            "}\n",
        )

        if check_file(fixture, root):
            print("self-test: inline event dispatch was rejected", file=sys.stderr)
            return 1

        # ── invalid: bound before dispatch, and direct develop call ───────
        fixture.write_text(
            "enum Event { UserActive(UserActiveEvent) }\n"
            "struct UserActiveEvent { user_id: String }\n"
            "fn invalid(develop: &Develop) {\n"
            "    let event = Event::UserActive(UserActiveEvent { user_id: String::new() });\n"
            "    develop.develop(event);\n"
            "}\n",
        )
        diagnostics = check_file(fixture, root)

        if len(diagnostics) != 2:
            print("self-test: non-inline event dispatch was not rejected", file=sys.stderr)
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

    paths = production_files(root)
    errors = [error for path in paths for error in check_file(path, root)]

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
