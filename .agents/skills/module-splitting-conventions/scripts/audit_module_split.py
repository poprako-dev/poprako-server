#!/usr/bin/env python3
"""Report Rust parent/direct-child line counts and enforce the file limit."""

from __future__ import annotations

import re
import sys
from pathlib import Path


LIMIT = 600
MOD_PATTERN = re.compile(r"^\s*(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;")


def line_count(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


def direct_children(parent: Path) -> list[Path]:
    child_dir = parent.with_suffix("")
    children: list[Path] = []

    for line in parent.read_text(encoding="utf-8").splitlines():
        match = MOD_PATTERN.match(line)

        if match is None:
            continue

        child = child_dir / f"{match.group(1)}.rs"

        if child.is_file():
            children.append(child)

    return children


def audit(parent: Path) -> bool:
    children = direct_children(parent)
    parent_lines = line_count(parent)
    merged_lines = parent_lines + sum(line_count(child) for child in children)
    failed = parent_lines >= LIMIT

    print(f"parent: {parent} ({parent_lines} lines)")

    for child in children:
        child_lines = line_count(child)
        failed = failed or child_lines >= LIMIT
        print(f"  child: {child} ({child_lines} lines)")

    print(f"  merged: {merged_lines} lines")

    return failed


def main() -> int:
    parents = [Path(argument) for argument in sys.argv[1:]]

    if not parents:
        print("usage: audit_module_split.py PARENT.rs [...]", file=sys.stderr)
        return 2

    failed = False

    for parent in parents:
        if not parent.is_file() or parent.suffix != ".rs":
            print(f"error: not a Rust parent file: {parent}", file=sys.stderr)
            failed = True
            continue

        failed = audit(parent) or failed

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
