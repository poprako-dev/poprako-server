#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid inline #[cfg(test)] mod tests { ... } — must be a separate file."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust


DEFAULT_ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def rust_files(root: Path) -> list[Path]:
    return sorted((root / "src").rglob("*.rs"))


def leading_attributes(node: tree_sitter.Node) -> list[tree_sitter.Node]:
    parent = node.parent

    if parent is None:
        return []

    index = next(
        (
            index
            for index, sibling in enumerate(parent.children)
            if sibling.start_byte == node.start_byte and sibling.end_byte == node.end_byte
        ),
        None,
    )

    if index is None:
        return []

    attributes: list[tree_sitter.Node] = []

    for sibling in reversed(parent.children[:index]):
        if sibling.type == "attribute_item":
            attributes.append(sibling)
            continue

        if sibling.type in {"line_comment", "block_comment"}:
            continue

        if not sibling.is_named:
            continue

        break

    attributes.reverse()

    return attributes


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def cfg_expression(attribute: str) -> str | None:
    match = re.fullmatch(r"\s*#\s*\[\s*cfg\s*\((.*)\)\s*]\s*", attribute, re.DOTALL)

    return match.group(1) if match is not None else None


class CfgParser:
    """Evaluate possible cfg truth values while forcing `test = false`."""

    TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z0-9_]*|[(),=]')

    def __init__(self, expression: str) -> None:
        self.tokens = self.TOKEN.findall(expression)
        self.index = 0

    def parse(self) -> set[bool]:
        values = self._expression()

        if self.index != len(self.tokens):
            return {False, True}

        return values

    def _expression(self) -> set[bool]:
        if self.index >= len(self.tokens):
            return {False, True}

        name = self.tokens[self.index]
        self.index += 1

        if self._take("="):
            if self.index < len(self.tokens):
                self.index += 1

            return {False, True}

        if not self._take("("):
            return {False} if name == "test" else {False, True}

        arguments: list[set[bool]] = []

        while self.index < len(self.tokens) and self.tokens[self.index] != ")":
            arguments.append(self._expression())

            if not self._take(","):
                break

        if not self._take(")"):
            return {False, True}

        if name == "all":
            return {
                all(values)
                for values in _products(arguments)
            }

        if name == "any":
            return {
                any(values)
                for values in _products(arguments)
            }

        if name == "not" and len(arguments) == 1:
            return {not value for value in arguments[0]}

        return {False, True}

    def _take(self, expected: str) -> bool:
        if self.index >= len(self.tokens) or self.tokens[self.index] != expected:
            return False

        self.index += 1

        return True


def _products(sets: list[set[bool]]) -> list[tuple[bool, ...]]:
    products: list[tuple[bool, ...]] = [()]

    for values in sets:
        products = [prefix + (value,) for prefix in products for value in values]

    return products


def has_test_only_cfg(node: tree_sitter.Node, source: bytes) -> bool:
    current: tree_sitter.Node | None = node

    while current is not None:
        for attribute in leading_attributes(current):
            expression = cfg_expression(node_text(source, attribute))

            if expression is not None and True not in CfgParser(expression).parse():
                return True

        current = current.parent

    return False


def is_inline_test_mod(node: tree_sitter.Node, source: bytes) -> bool:
    if node.type != "mod_item":
        return False

    name = node.child_by_field_name("name")

    if name is None or node_text(source, name) != "tests":
        return False

    body = node.child_by_field_name("body")

    if body is None:
        return False

    if not has_test_only_cfg(node, source):
        return False

    return True


def check_file(path: Path, root: Path) -> list[str]:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    pending = [tree.root_node]

    while pending:
        node = pending.pop()

        if is_inline_test_mod(node, source):
            line = node.start_point.row + 1
            column = node.start_point.column + 1
            diagnostics.append(
                f"{path.relative_to(root)}:{line}:{column}: TST001: "
                f"inline #[cfg(test)] mod tests {{ ... }} is forbidden; "
                f"extract tests into a separate tests.rs file",
            )

        pending.extend(reversed(node.named_children))

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

        (source_dir / "lib.rs").write_text(
            "pub mod things;\n"
            "#[cfg(test)] mod tests;\n",
        )
        (source_dir / "things.rs").write_text(
            "pub fn do_thing() {}\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use super::*;\n"
            "    #[test]\n"
            "    fn it_works() {}\n"
            "}\n",
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 1:
            print("self-test: inline test module was not rejected", file=sys.stderr)
            print(f"got {len(diagnostics)} diagnostics", file=sys.stderr)

            for diagnostic in diagnostics:
                print(f"  {diagnostic}", file=sys.stderr)

            return 1

        if "TST001" not in diagnostics[0]:
            print("self-test: diagnostic code is wrong", file=sys.stderr)
            print(f"  {diagnostics[0]}", file=sys.stderr)
            return 1

        if "things.rs" not in diagnostics[0]:
            print("self-test: wrong file reported", file=sys.stderr)
            print(f"  {diagnostics[0]}", file=sys.stderr)
            return 1

        (source_dir / "things.rs").write_text(
            "pub fn do_thing() {}\n"
            "#[cfg(test)]\n"
            "// Tests remain in a sibling file.\n"
            "mod tests;\n",
        )

        diagnostics = check_root(root)

        if diagnostics:
            print("self-test: valid separate-file test mod was rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        (source_dir / "things.rs").write_text(
            "pub fn do_thing() {}\n"
            "#[cfg(test)]\n"
            "// This comment belongs to the test module.\n"
            "mod tests {\n"
            "    use super::*;\n"
            "}\n",
        )

        diagnostics = check_root(root)

        if not diagnostics:
            print("self-test: commented inline test module was not rejected", file=sys.stderr)
            return 1

        (source_dir / "lib.rs").write_text(
            "pub mod things;\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use super::*;\n"
            "    #[test]\n"
            "    fn it_works() {}\n"
            "}\n",
        )

        diagnostics = check_root(root)

        if not diagnostics:
            print("self-test: inline test mod in lib.rs was not rejected", file=sys.stderr)
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
