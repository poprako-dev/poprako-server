#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce public `*_data` module qualification outside data and model."""

from __future__ import annotations

import re
import sys
from pathlib import Path

import tree_sitter
import tree_sitter_rust


ROOT = Path(__file__).parents[2]
LAYER = "data"
SUFFIX = "_data"
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def rust_files() -> list[Path]:
    return sorted((ROOT / "src").rglob("*.rs"))


def layer_modules() -> set[str]:
    return {path.stem for path in (ROOT / "src" / LAYER).glob("*.rs")}


def is_layer_internal(path: Path) -> bool:
    relative = path.relative_to(ROOT)
    return relative in (Path("src/data.rs"), Path("src/model.rs")) or relative.parts[:2] in (("src", "data"), ("src", "model"))


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type == kind:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def check_file(path: Path, modules: set[str]) -> list[str]:
    if is_layer_internal(path):
        return []

    source = path.read_bytes()
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node, "use_declaration"):
        text = source[declaration.start_byte : declaration.end_byte].decode()
        direct_paths = re.finditer(rf"crate::{LAYER}::([a-z_]+)(?=::|;|\s*as)", text)

        for match in direct_paths:
            domain = match.group(1)

            if domain not in modules or domain.endswith(SUFFIX):
                continue

            diagnostics.append(f"{path.relative_to(ROOT)}:{declaration.start_point.row + 1}: use {domain}{SUFFIX}")

        if re.search(rf"crate::{LAYER}::[a-z_]+{SUFFIX}::", text):
            diagnostics.append(
                f"{path.relative_to(ROOT)}:{declaration.start_point.row + 1}: import the {SUFFIX} module, not an individual {LAYER} type",
            )

    return diagnostics


def main() -> int:
    modules = layer_modules()
    diagnostics = [diagnostic for path in rust_files() for diagnostic in check_file(path, modules)]

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
