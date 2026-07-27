#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require run_proxy! and step_proxy! invocations to be call arguments."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


DEFAULT_ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
PROXY_MACRO = re.compile(rb"(?:^|::)(run_proxy|step_proxy)\s*!")
INLINE_WRAPPERS = {"parenthesized_expression", "reference_expression"}


def rust_files(root: Path) -> list[Path]:
    return sorted((root / "src").rglob("*.rs"))


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type == kind:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def proxy_macro_name(node: tree_sitter.Node, source: bytes) -> str | None:
    invocation = source[node.start_byte : node.end_byte]
    match = PROXY_MACRO.search(invocation)

    if match is None:
        return None

    return match.group(1).decode()


def is_inline_argument(node: tree_sitter.Node) -> bool:
    parent = node.parent

    while parent is not None and parent.type in INLINE_WRAPPERS:
        parent = parent.parent

    return parent is not None and parent.type == "arguments"


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    for invocation in descendants(tree.root_node, "macro_invocation"):
        macro_name = proxy_macro_name(invocation, source)

        if macro_name is None or is_inline_argument(invocation):
            continue

        line = invocation.start_point.row + 1
        column = invocation.start_point.column + 1
        diagnostics.append(
            f"{path.relative_to(root)}:{line}:{column}: PRX001: "
            f"inline {macro_name}! directly in the consuming call argument",
        )

    return diagnostics


def check_root(root: Path) -> list[str]:
    return [
        diagnostic
        for path in rust_files(root)
        for diagnostic in check_file(path, root)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"
        fixture.write_text(
            "fn valid() {\n"
            "    consume(&mut run_proxy! { repo => Oper; });\n"
            "    consume((&mut step_proxy! { context; repo => Oper; }));\n"
            "}\n",
        )

        if check_root(root):
            print("self-test: valid inline proxy fixture was rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "fn invalid() -> impl Sized {\n"
            "    let mut run = run_proxy! { repo => Oper; };\n"
            "    let step = (step_proxy! { context; repo => Oper; });\n"
            "    saved = run_proxy! { repo => Oper; };\n"
            "    return step_proxy! { context; repo => Oper; };\n"
            "}\n",
        )
        diagnostics = check_root(root)

        if len(diagnostics) != 4:
            print("self-test: non-inline proxy macros were not fully rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--self-test", action="store_true")

    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.self_test:
        return self_test()

    diagnostics = check_root(args.root.resolve())

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
