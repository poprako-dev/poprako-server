#!/usr/bin/env python3
"""Enforce the rustfmt column limit on macro-related Rust source lines."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import os
from pathlib import Path
import sys
import tomllib
import unicodedata
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from analysis_session import parse_rust


ROOT = Path(__file__).resolve().parents[2]
IGNORED_DIRS = {
    ".git", ".hg", ".svn", ".idea", ".vscode", ".venv", ".uv-cache",
    "__pycache__", "target", "node_modules", "dist", "linters",
    "linters-extra", "references", "worktrees",
}
BUILTIN_ATTRIBUTES = {
    "allow", "expect", "warn", "deny", "forbid", "cfg", "derive", "doc",
    "deprecated", "must_use", "repr", "inline", "cold", "track_caller",
    "test", "ignore", "should_panic", "no_mangle", "export_name", "link",
    "link_name", "link_section", "path", "macro_export", "macro_use",
    "recursion_limit", "type_length_limit", "feature", "no_std", "no_main",
    "automatically_derived", "rustfmt::skip", "unsafe", "non_exhaustive",
    "used", "crate_name", "crate_type", "windows_subsystem", "panic_handler",
    "global_allocator", "no_implicit_prelude", "link_ordinal",
}


def is_macro_attribute(tokens, source: bytes) -> bool:
    """Recognize procedural/helper attributes, including cfg_attr wrappers."""
    children = [node for node in tokens if node.is_named and node.type not in {"line_comment", "block_comment"}]
    if not children:
        return False
    name = source[children[0].start_byte:children[0].end_byte].decode()
    if name != "cfg_attr":
        return name not in BUILTIN_ATTRIBUTES
    arguments = next((child for child in children if child.type == "token_tree"), None)
    if arguments is None:
        return False
    tokens = arguments.children
    for index, token in enumerate(tokens):
        if token.type != ",":
            continue
        return is_macro_attribute(tokens[index + 1:], source)
    return False


def macro_rows(source: bytes) -> set[int]:
    """Select macros and custom attributes; omit built-in attributes/comments."""
    tree = parse_rust(source)
    included = set()
    excluded = set()
    comments = []
    pending = [tree.root_node]
    while pending:
        node = pending.pop()
        pending.extend(reversed(node.children))
        rows = range(node.start_point.row, node.end_point.row + 1)
        if node.type in {"macro_invocation", "macro_definition"}:
            included.update(rows)
        if node.type in {"attribute_item", "inner_attribute_item"}:
            attribute = next((child for child in node.named_children if child.type == "attribute"), None)
            if attribute is not None and is_macro_attribute(attribute.children, source):
                included.update(rows)
            else:
                excluded.update(rows)
        # Attributes inside opaque macro token trees are not attribute_item nodes.
        if node.type == "token_tree" and source[node.start_byte:node.start_byte + 1] == b"[":
            previous = node.prev_sibling
            if previous is not None and previous.type == "!":
                previous = previous.prev_sibling
            if previous is not None and previous.type == "#" and not is_macro_attribute(node.children, source):
                excluded.update(rows)
        if node.type in {"line_comment", "block_comment"}:
            comments.append((node.start_byte, node.end_byte))
    masked = bytearray(source)
    for start, end in comments:
        for index in range(start, end):
            if masked[index] not in {10, 13}:
                masked[index] = 32
    code_lines = bytes(masked).splitlines()
    return {row for row in included - excluded if row < len(code_lines) and code_lines[row].strip()}


@dataclass(frozen=True)
class WidthConfig:
    max_width: int
    tab_spaces: int


def read_config(root: Path) -> WidthConfig:
    for name in ("rustfmt.toml", ".rustfmt.toml"):
        path = root / name
        if not path.is_file():
            continue
        with path.open("rb") as source:
            config = tomllib.load(source)
        values = {"max_width": config.get("max_width"), "tab_spaces": config.get("tab_spaces", 4)}
        for key, value in values.items():
            if type(value) is not int or value <= 0:
                raise ValueError(f"{path}: {key} must be a positive integer")
        return WidthConfig(**values)
    raise ValueError(f"{root}: rustfmt.toml or .rustfmt.toml with max_width is required")


def source_files(root: Path) -> list[Path]:
    paths = []
    for directory, names, files in os.walk(root):
        names[:] = sorted(name for name in names if name not in IGNORED_DIRS)
        paths.extend(Path(directory) / name for name in files if name.endswith(".rs"))
    return sorted(paths)


def line_width(line: str, tab_spaces: int) -> int:
    width = 0
    for char in line:
        if char == "\t":
            width += tab_spaces - width % tab_spaces
            continue
        if unicodedata.category(char) in {"Mn", "Me", "Cf"}:
            continue
        width += 2 if unicodedata.east_asian_width(char) in {"W", "F"} else 1
    return width


def check_root(root: Path) -> int:
    config = read_config(root)
    violations = 0
    affected = 0
    for path in source_files(root):
        file_violations = 0
        source = path.read_bytes()
        selected = macro_rows(source)
        for number, line in enumerate(source.decode("utf-8").splitlines(), 1):
            if number - 1 not in selected:
                continue
            width = line_width(line.rstrip("\r\n"), config.tab_spaces)
            if width <= config.max_width:
                continue
            print(
                f"{path.relative_to(root)}:{number}:{config.max_width + 1}: "
                f"WID001: line is {width} columns; maximum is {config.max_width} "
                f"(exceeds by {width - config.max_width})",
                file=sys.stderr,
            )
            file_violations += 1
        violations += file_violations
        affected += int(file_violations > 0)
    if violations:
        print(
            f"line-width: {violations} overlong macro lines in {affected} files "
            f"(max_width = {config.max_width})",
            file=sys.stderr,
        )
    return int(violations > 0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        tests = unittest.defaultTestLoader.discover(str(Path(__file__).parent), pattern="test_check.py")
        return int(not unittest.TextTestRunner().run(tests).wasSuccessful())
    try:
        return check_root(args.root.resolve())
    except (OSError, ValueError) as err:
        print(f"line-width: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
