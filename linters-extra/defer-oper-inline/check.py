#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require Defer and DeferBatch to be constructed as step_on receivers."""

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


def is_defer_constructor(node: tree_sitter.Node, source: bytes) -> bool:
    function = node.child_by_field_name("function")

    if function is None:
        return False

    return text(source, function).split("::")[-2:] in (["Defer", "new"], ["DeferBatch", "new"])


def is_step_on_receiver(node: tree_sitter.Node, source: bytes) -> bool:
    parent = node.parent

    while parent is not None and parent.type in WRAPPERS:
        parent = parent.parent

    if parent is None or parent.type != "field_expression":
        return False

    value = parent.child_by_field_name("value")
    field = parent.child_by_field_name("field")

    if value is None or field is None or text(source, field) != "step_on":
        return False

    call = parent.parent

    return (
        call is not None
        and call.type == "call_expression"
        and call.child_by_field_name("function") == parent
    )


def check_root(root: Path) -> list[str]:
    errors: list[str] = []

    for path in sorted((root / "src").rglob("*.rs")):
        source = production_source(path, root)
        tree = PARSER.parse(source)

        for call in descendants(tree.root_node, "call_expression"):
            if is_defer_constructor(call, source) and not is_step_on_receiver(call, source):
                errors.append(
                    f"{path.relative_to(root)}:{call.start_point.row + 1}: "
                    "DEF001: construct Defer or DeferBatch directly as the step_on receiver",
                )

    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"
        fixture.write_text(
            "fn valid() {\n"
            "    Defer::new(task).step_on(prom, context);\n"
            "    (DeferBatch::new(&tasks)).step_on(prom, context);\n"
            "}\n",
        )

        if check_root(root):
            print("self-test: step_on receivers were rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "fn invalid() {\n"
            "    let defer = Defer::new(task);\n"
            "    defer.step_on(prom, context);\n"
            "    prom.step(context, &Defer::new(task));\n"
            "    consume(DeferBatch::new(&tasks));\n"
            "    Defer::new(task).run_on(prom);\n"
            "}\n",
        )
        diagnostics = check_root(root)

        if len(diagnostics) != 4:
            print("self-test: invalid Defer uses were not fully rejected", file=sys.stderr)
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

    errors = check_root(root)

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
