#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require uniform field visibility within a struct."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
import tempfile
import unittest

import tree_sitter

sys.path.insert(0, str(Path(__file__).parents[1]))
from analysis_session import parse_rust
from production_source import production_files, production_source


ROOT = Path(__file__).parents[2]
IGNORED = {"attribute_item", "line_comment", "block_comment"}


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def diagnostic(path: Path, root: Path, name: tree_sitter.Node, struct: str) -> str:
    return (
        f"{path.relative_to(root)}:{name.start_point.row + 1}:"
        f"{name.start_point.column + 1}: SFV001: struct `{struct}` mixes "
        "public and private fields; a data-oriented struct exposes every "
        "field, a behaviour-oriented struct exposes none"
    )


def declaration_visible(node: tree_sitter.Node, source: bytes) -> bool:
    return any(
        child.type == "visibility_modifier" for child in node.children
    )


def field_visibilities(
    declaration: tree_sitter.Node,
    source: bytes,
) -> list[bool]:
    body = declaration.child_by_field_name("body")

    if body is None:
        return []

    if body.type == "field_declaration_list":
        return [
            declaration_visible(child, source)
            for child in body.named_children
            if child.type == "field_declaration"
        ]

    if body.type != "ordered_field_declaration_list":
        return []

    visibilities: list[bool] = []
    exposed = False

    for child in body.children:
        if child.type == "visibility_modifier":
            exposed = True
            continue

        if child.is_named and child.type not in IGNORED:
            visibilities.append(exposed)
            exposed = False

    return visibilities


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = parse_rust(source)
    diagnostics: list[str] = []
    pending = [tree.root_node]

    while pending:
        node = pending.pop()

        if node.type == "struct_item":
            name = node.child_by_field_name("name")
            visibilities = field_visibilities(node, source)

            if name is not None and any(visibilities) and not all(visibilities):
                diagnostics.append(
                    diagnostic(path, root, name, text(source, name))
                )

        pending.extend(reversed(node.named_children))

    return diagnostics


def check_root(root: Path) -> list[str]:
    root = root.resolve()

    return [
        item
        for path in production_files(root)
        for item in check_file(path, root)
    ]


def self_test() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(
        StructFieldVisibilityTests
    )

    return (
        0
        if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful()
        else 1
    )


class StructFieldVisibilityTests(unittest.TestCase):
    def diagnostics(
        self,
        files: dict[str, str],
        crate: str = "sample",
    ) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                f'[package]\nname = "{crate}"\nversion = "0.0.0"\n'
            )

            for name, source in files.items():
                path = root / "src" / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)

            return check_root(root)

    def test_all_public_struct_is_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Data {\n"
                    "    pub first: u32,\n"
                    "    pub second: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_all_private_struct_is_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Helper {\n"
                    "    first: u32,\n"
                    "    second: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_mixed_struct_is_rejected(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Mixed {\n"
                    "    pub first: u32,\n"
                    "    second: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)
        self.assertIn("SFV001", diagnostics[0])
        self.assertIn("Mixed", diagnostics[0])

    def test_restricted_visibility_counts_as_exposed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Mixed {\n"
                    "    pub(crate) first: u32,\n"
                    "    second: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)

    def test_tuple_struct_is_checked(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Mixed(pub u32, String);\n"
                    "pub struct Data(pub u32, pub String);\n"
                    "pub struct Helper(u32, String);\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)
        self.assertIn("Mixed", diagnostics[0])

    def test_unit_and_empty_structs_are_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": ("pub struct Unit;\npub struct Empty {}\n"),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_test_sources_are_ignored(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "pub mod value;\n",
                "value/tests.rs": (
                    "struct Fixture {\n"
                    "    pub first: u32,\n"
                    "    second: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
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
