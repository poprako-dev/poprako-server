#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid type-annotation hints on let bindings.

Prefer inference, and turbofish when a type must be pinned explicitly::

    let x = expr.collect::<Vec<_>>();       // GOOD
    let y = resolver.parse::<u32>()?;       // GOOD — turbofish on the call
    let z: u32 = expr.parse()?;             // BAD — type hint on the let binding

The rule is uniform: no `let x: T = value` is allowed. When the value's type
cannot be inferred, supply it as turbofish on the value's generic call instead
of annotating the binding.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))

@dataclass(frozen=True)
class Violation:
    path: Path
    line: int
    code: str
    message: str


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def line_number(source: bytes, offset: int) -> int:
    return source.count(b"\n", 0, offset) + 1


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    pending = [node]

    while pending:
        current = pending.pop()

        if current.type == kind:
            found.append(current)

        pending.extend(reversed(current.named_children))

    return found


def type_annotation_text(node: tree_sitter.Node, source: bytes) -> str | None:
    """Extract the type annotation string from a let_declaration."""
    type_node = node.child_by_field_name("type")

    if type_node is not None:
        return text(source, type_node)

    return None


def violation_for(path: Path, let_node: tree_sitter.Node, source: bytes) -> Violation:
    type_ann = type_annotation_text(let_node, source)
    type_text = type_ann if type_ann is not None else "<type>"

    message = (
        f"type hint `{type_text}` on let binding; remove the annotation and rely on inference, "
        "or pin the type with turbofish on the value's generic call"
    )

    line = line_number(source, let_node.start_byte)
    return Violation(path=path, line=line, code="NO_TYPE_HINT", message=message)


def check_file(path: Path, source: bytes) -> list[Violation]:
    tree = PARSER.parse(source)
    violations: list[Violation] = []

    for let_node in descendants(tree.root_node, "let_declaration"):
        if type_annotation_text(let_node, source) is None:
            continue

        violations.append(violation_for(path, let_node, source))

    return violations


def fix_file(path: Path, source: bytes) -> tuple[bytes, bool]:
    """Conservatively leave the source untouched.

    Removing an annotation can silently change the inferred type (literals,
    `as` casts, generic constructors) or break compilation entirely (Diesel
    `.first()?` / `.load()`), and turbofish is not valid on every method.
    Auto-editing is therefore a no-op: the check reports, and the developer
    applies the recommended fix by hand.
    """
    return source, False


def rust_files(root: Path, paths: list[Path]) -> list[Path]:
    if paths:
        files: list[Path] = []

        for path in paths:
            resolved = path.resolve()

            if resolved.is_file() and resolved.suffix == ".rs":
                files.append(resolved)
            elif resolved.is_dir():
                files.extend(resolved.rglob("*.rs"))

        return sorted(set(files))

    return sorted((root / "src").rglob("*.rs"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Forbid type hints on let bindings")
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--fix", action="store_true", help="accepted for compatibility; auto-fix is not implemented")
    args = parser.parse_args()

    root = args.root.resolve()
    paths = rust_files(root, args.paths)
    all_violations: list[Violation] = []

    for path in paths:
        source = path.read_bytes()
        visible_source = production_source(path, root)

        if args.fix and source == visible_source:
            _, changed = fix_file(path, source)

            if changed:
                path.write_bytes(visible_source)

            visible_source = source

        all_violations.extend(check_file(path, visible_source))

    if args.fix:
        print(
            "note: --fix is a no-op; remove or turbofish the annotation by hand "
            "(auto-editing can change the inferred type or break compilation)",
            file=sys.stderr,
        )

    if all_violations:
        for v in sorted(all_violations, key=lambda v: (str(v.path), v.line)):
            print(f"{v.path.relative_to(root)}:{v.line}: {v.code}: {v.message}", file=sys.stderr)

        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
