#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid generic Rust module names in PopRaKo source."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
FORBIDDEN_NAMES = {
    "helper": (
        "MODNAME001",
        "module name 'helper' is forbidden — name the business responsibility",
    ),
    "helpers": (
        "MODNAME001",
        "module name 'helpers' is forbidden — name the business responsibility",
    ),
    "operation": (
        "MODNAME002",
        "module name 'operation' is forbidden — name the business responsibility",
    ),
    "operations": (
        "MODNAME002",
        "module name 'operations' is forbidden — name the business responsibility",
    ),
}
EXCLUDED_FILENAMES = {"schema.rs"}


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def diagnostic(
    path: Path,
    root: Path,
    node: tree_sitter.Node,
    code: str,
    message: str,
) -> str:
    return (
        f"{path.relative_to(root)}:{node.start_point.row + 1}:"
        f"{node.start_point.column + 1}: {code}: {message}"
    )


def check_file(path: Path, root: Path) -> list[str]:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    pending = [tree.root_node]
    diagnostics: list[str] = []

    while pending:
        node = pending.pop()

        if node.type == "mod_item":
            name_node = node.child_by_field_name("name")

            if name_node is not None:
                name = node_text(source, name_node)
                rule = FORBIDDEN_NAMES.get(name)

                if rule is not None:
                    code, message = rule
                    diagnostics.append(
                        diagnostic(path, root, name_node, code, message),
                    )

        pending.extend(reversed(node.named_children))

    return diagnostics


def check_root(root: Path) -> list[str]:
    return [
        diagnostic
        for path in sorted((root / "src").rglob("*.rs"))
        if path.name not in EXCLUDED_FILENAMES
        for diagnostic in check_file(path, root)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "lib.rs"
        fixture.write_text(
            "mod helper;\n"
            "mod profile;\n"
            "pub mod operations;\n"
            "#[cfg(test)]\n"
            "mod helpers;\n",
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 3:
            print(
                "self-test: forbidden module names were not fully detected",
                file=sys.stderr,
            )
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        (source_dir / "schema.rs").write_text("mod operation;\n")

        if len(check_root(root)) != 3:
            print("self-test: generated schema was not excluded", file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="forbidden-module-names")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    diagnostics = check_root(args.root.resolve())

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
