#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["tree-sitter==0.25.0", "tree-sitter-rust==0.23.3"]
# ///
"""Forbid mixing model, data, and entity type suffixes across layers."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter

sys.path.insert(0, str(Path(__file__).parents[1]))
from analysis_session import parse_rust
from production_source import production_files, production_source


DEFAULT_ROOT = Path(__file__).parents[2]
# Each layer owns its role suffixes; no other layer may reuse them.
LAYER_SUFFIXES = {
    "src/model/": ("Info", "Entry", "Spec"),
    "src/data/": ("Instr", "Val", "View"),
    "src/part_impl/repo/rdb_impl/entity/": ("Record", "Item", "Pack"),
}
DECLARATION_KINDS = ("struct_item", "enum_item", "union_item", "type_item")


def rust_files(root: Path) -> list[Path]:
    return production_files(root, "src")


def descendants(node: tree_sitter.Node, kinds: tuple[str, ...]) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type in kinds:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def suffix_owner(type_name: str) -> tuple[str, str] | None:
    for layer, suffixes in LAYER_SUFFIXES.items():
        for suffix in suffixes:
            if type_name.endswith(suffix):
                return layer, suffix

    return None


def check_file(path: Path, root: Path) -> list[str]:
    relative = path.relative_to(root).as_posix()
    layer = next(
        (prefix for prefix in LAYER_SUFFIXES if relative.startswith(prefix)),
        None,
    )

    if layer is None:
        return []

    source = production_source(path, root)
    tree = parse_rust(source)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node, DECLARATION_KINDS):
        name = declaration.child_by_field_name("name")

        if name is None:
            continue

        type_name = source[name.start_byte : name.end_byte].decode()
        owner = suffix_owner(type_name)

        if owner is None:
            continue

        owned_layer, suffix = owner

        if owned_layer != layer:
            location = f"{relative}:{name.start_point.row + 1}"
            diagnostics.append(
                f"{location}: TSL001: type {type_name} uses suffix {suffix} owned by {owned_layer}",
            )

    return diagnostics


def check_root(root: Path) -> list[str]:
    manifest = root / "Cargo.toml"

    if not manifest.is_file() or 'name = "poprako-server"' not in manifest.read_text():
        return []

    return [
        diagnostic
        for path in rust_files(root)
        for diagnostic in check_file(path, root)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "Cargo.toml").write_text(
            '[package]\nname = "poprako-server"\nversion = "0.0.0"\n',
        )
        model = root / "src" / "model" / "read"
        model.mkdir(parents=True)
        (model / "team.rs").write_text(
            "pub struct TeamInfo;\n"
            "pub struct TeamEntry;\n",
        )

        if check_root(root):
            print("self-test: valid model fixture was rejected", file=sys.stderr)
            return 1

        (model / "team.rs").write_text("pub struct TeamRecord;\n")

        if len(check_root(root)) != 1:
            print("self-test: misplaced model suffix was not rejected", file=sys.stderr)
            return 1

        entity = root / "src" / "part_impl" / "repo" / "rdb_impl" / "entity"
        entity.mkdir(parents=True)
        (entity / "team.rs").write_text("pub struct TeamEntry;\n")

        if len(check_root(root)) != 2:
            print("self-test: misplaced entity suffix was not rejected", file=sys.stderr)
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
