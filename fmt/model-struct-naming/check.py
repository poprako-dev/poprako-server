#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid model struct names from repeating their domain module name."""

from __future__ import annotations

import sys
from pathlib import Path

import tree_sitter
import tree_sitter_rust


ROOT = Path(__file__).parents[2]
LAYER = "model"
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def rust_files() -> list[Path]:
    return sorted((ROOT / "src" / LAYER).rglob("*.rs"))


def domain_name(path: Path) -> str:
    relative = path.relative_to(ROOT / "src" / LAYER)
    domain = relative.stem if len(relative.parts) == 1 else relative.parts[0]

    return "".join(part[:1].upper() + part[1:] for part in domain.split("_"))


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type == kind:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def check_file(path: Path) -> list[str]:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    domain = domain_name(path)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node, "struct_item"):
        name = declaration.child_by_field_name("name")

        if name is None:
            continue

        struct_name = source[name.start_byte : name.end_byte].decode()

        if domain not in struct_name:
            continue

        diagnostics.append(
            f"{path.relative_to(ROOT)}:{name.start_point.row + 1}: "
            f"struct {struct_name} must not contain domain name {domain}",
        )

    return diagnostics


def main() -> int:
    diagnostics = [diagnostic for path in rust_files() for diagnostic in check_file(path)]

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
