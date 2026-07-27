#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require Defer and DeferBatch operations to be constructed in step arguments."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
WRAPPERS = {"parenthesized_expression", "reference_expression"}


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


def is_inline(node: tree_sitter.Node) -> bool:
    parent = node.parent

    while parent is not None and parent.type in WRAPPERS:
        parent = parent.parent

    return parent is not None and parent.type == "arguments"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    root = args.root.resolve()
    errors: list[str] = []

    for path in sorted((root / "src").rglob("*.rs")):
        source = production_source(path, root)
        tree = PARSER.parse(source)

        for call in descendants(tree.root_node, "call_expression"):
            if is_defer_constructor(call, source) and not is_inline(call):
                errors.append(
                    f"{path.relative_to(root)}:{call.start_point.row + 1}: "
                    "DEF001: construct Defer or DeferBatch directly in the consuming call argument",
                )

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
