#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce domain-qualified public model type names and forbid `Form`."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_files, production_source


DEFAULT_ROOT = Path(__file__).parents[2]
LAYER = "model"
DECLARATION_KINDS = ("struct_item", "enum_item", "type_item")
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def rust_files(root: Path) -> list[Path]:
    return production_files(root, f"src/{LAYER}")


def pascal_name(module: str) -> str:
    module = module.removesuffix("_port")

    return "".join(part[:1].upper() + part[1:] for part in module.split("_"))


def descendants(node: tree_sitter.Node, kinds: tuple[str, ...]) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type in kinds:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def is_public(declaration: tree_sitter.Node, name: tree_sitter.Node, source: bytes) -> bool:
    prefix = source[declaration.start_byte : name.start_byte].lstrip()

    return prefix.startswith(b"pub")


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    domain = pascal_name(path.stem)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node, DECLARATION_KINDS):
        name = declaration.child_by_field_name("name")

        if name is None or not is_public(declaration, name, source):
            continue

        type_name = source[name.start_byte : name.end_byte].decode()
        location = f"{path.relative_to(root)}:{name.start_point.row + 1}"

        archive_child = path.stem.endswith("_archive") and type_name.startswith("Archived")

        if domain not in type_name and not archive_child:
            diagnostics.append(
                f"{location}: public model type {type_name} must contain domain name {domain}",
            )

        if "Form" in type_name:
            diagnostics.append(
                f"{location}: public model type {type_name} must use Entry or a precise role, not Form",
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
        model = root / "src" / "model"
        model.mkdir(parents=True)
        (model / "team.rs").write_text(
            "pub struct TeamInfo;\n"
            "pub enum TeamEntry { Empty }\n"
            "struct InternalHelper;\n",
        )

        if check_root(root):
            print("self-test: valid model fixture was rejected", file=sys.stderr)
            return 1

        (model / "team.rs").write_text(
            "pub struct Info;\n"
            "pub struct TeamForm;\n",
        )
        diagnostics = check_root(root)

        if len(diagnostics) != 2:
            print("self-test: invalid model fixture was not fully rejected", file=sys.stderr)
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
