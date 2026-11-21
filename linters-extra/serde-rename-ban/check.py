#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid manual `serde(rename = \"...\")` attributes."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest

import tree_sitter

sys.path.insert(0, str(Path(__file__).parents[1]))
sys.path.insert(0, str(Path(__file__).parents[2] / "linters"))
from analysis_session import parse_rust
from production_source import production_files, production_source


ROOT = Path(__file__).parents[2]
PICA_PREFIX = Path("src/part_impl/relay/pica")
RENAME = re.compile(r"(?<![A-Za-z0-9_])rename\s*(?:=|\()")
STRING = re.compile(r'"((?:\\.|[^"\\])*)"')


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def diagnostic(path: Path, root: Path, node: tree_sitter.Node) -> str:
    return (
        f"{path.relative_to(root)}:{node.start_point.row + 1}:"
        f"{node.start_point.column + 1}: SRN001: manual serde rename is "
        "forbidden; use the Rust field or variant name, `rename_all`, or a "
        "dedicated wire mapping module"
    )


def is_pica_wire(path: Path, root: Path) -> bool:
    return path.relative_to(root).is_relative_to(PICA_PREFIX)


def forbidden_words() -> set[str]:
    config = tomllib.loads((ROOT / "rust-style-lint.toml").read_text())
    identifiers = config.get("forbidden-identifiers", {})
    values = {rule["word"].lower() for rule in identifiers.get("words", [])}
    values.update(rule["prefix"].lower() for rule in identifiers.get("prefixes", []))
    return values


def contains_forbidden_word(attribute: str) -> bool:
    words = forbidden_words()

    for value in STRING.findall(attribute):
        parts = re.split(r"[^A-Za-z0-9]+", value.lower())

        if any(part in words for part in parts):
            return True

    return False


def rename_attributes(path: Path, root: Path) -> list[tuple[tree_sitter.Node, str]]:
    source = production_source(path, root)
    tree = parse_rust(source)
    found: list[tuple[tree_sitter.Node, str]] = []
    pending = [tree.root_node]

    while pending:
        node = pending.pop()

        if node.type == "attribute_item":
            value = text(source, node)

            if "serde" in value and RENAME.search(value):
                found.append((node, value))

        pending.extend(reversed(node.named_children))

    return found


def file_allows_special_wire(path: Path, root: Path) -> bool:
    for _, attribute in rename_attributes(path, root):
        for value in STRING.findall(attribute):
            if "." in value or "-" in value:
                return True

    return False


def check_file(path: Path, root: Path) -> list[str]:
    if is_pica_wire(path, root) or file_allows_special_wire(path, root):
        return []

    return [
        diagnostic(path, root, node)
        for node, attribute in rename_attributes(path, root)
        if not contains_forbidden_word(attribute)
    ]


def check_root(root: Path) -> list[str]:
    root = root.resolve()

    return [
        item
        for path in production_files(root)
        for item in check_file(path, root)
    ]


def self_test() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(SerdeRenameTests)

    return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1


class SerdeRenameTests(unittest.TestCase):
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

    def test_manual_rename_is_rejected(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Value {\n"
                    "    #[serde(rename = \"renamed\")]\n"
                    "    pub field: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)
        self.assertIn("SRN001", diagnostics[0])

    def test_split_rename_is_rejected(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Value {\n"
                    "    #[serde(rename(serialize = \"one\", deserialize = \"two\"))]\n"
                    "    pub field: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)

    def test_rename_all_is_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "#[serde(rename_all = \"camelCase\")]\n"
                    "pub struct Value { pub field_name: String }\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_dotted_and_hyphenated_wire_values_are_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub enum Include {\n"
                    "    #[serde(rename = \"comic\")]\n"
                    "    Comic,\n"
                    "    #[serde(rename = \"comic.workset\")]\n"
                    "    ComicWorkset,\n"
                    "}\n"
                    "pub enum Format {\n"
                    "    #[serde(rename = \"label-plus\")]\n"
                    "    LabelPlus,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_pica_wire_module_is_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "pub mod wire;\n",
                "part_impl/relay/pica/wire.rs": (
                    "pub struct PicaComic {\n"
                    "    #[serde(rename = \"chineseTeam\")]\n"
                    "    pub chinese_team: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_non_pica_camel_case_is_rejected(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Value {\n"
                    "    #[serde(rename = \"camelCase\")]\n"
                    "    pub field: String,\n"
                    "}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)

    def test_forbidden_rust_identifier_wire_name_is_allowed(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": (
                    "pub struct Value {\n"
                    "    #[serde(rename = \"image_version\")]\n"
                    "    pub image_ver: u32,\n"
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
