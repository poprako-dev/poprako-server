#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid type-annotation hints on let bindings when turbofish is idiomatic.

Prefer turbofish over type hints::

    let x = expr.collect::<Vec<_>>();       // GOOD
    let x: Vec<_> = expr.collect();         // BAD — type hint instead of turbofish
    let x = expr.collect();                 // BEST — inference when possible

Only ``collect`` and ``parse`` are checked for now.
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

# Methods where turbofish is the canonical way to supply the type argument.
TURBOFISH_METHODS = frozenset({"collect", "parse", "from_str", "from_reader", "from_slice"})


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


def method_name(node: tree_sitter.Node, source: bytes) -> str | None:
    """Return the method name if `node` is a field-expression (`.method`)."""
    if node.type == "field_expression":
        field = node.child_by_field_name("field")

        if field is not None:
            return text(source, field)

    return None


def has_turbofish(node: tree_sitter.Node, source: bytes) -> bool:
    """Check whether a call_expression node already carries turbofish."""
    # tree-sitter-rust represents turbofish as a `type_arguments` child of the
    # call_expression (or sometimes the field_expression).
    for child in node.named_children:
        if child.type == "type_arguments":
            return True

    func = node.child_by_field_name("function")

    if func is not None:
        for child in func.named_children:
            if child.type == "type_arguments":
                return True

    return False


def type_annotation_text(node: tree_sitter.Node, source: bytes) -> str | None:
    """Extract the type annotation string from a let_declaration."""
    type_node = node.child_by_field_name("type")

    if type_node is not None:
        return text(source, type_node)

    return None


def check_file(path: Path, source: bytes) -> list[Violation]:
    tree = PARSER.parse(source)
    violations: list[Violation] = []

    for let_node in descendants(tree.root_node, "let_declaration"):
        type_ann = type_annotation_text(let_node, source)

        if type_ann is None:
            continue

        value = let_node.child_by_field_name("value")

        if value is None:
            continue

        # Drill into block, return, or parenthesized expressions.
        inner = value

        while inner.type in {"block", "return_expression", "parenthesized_expression"}:
            inner = inner.child_by_field_name("body") or inner.named_children[0] if inner.named_children else None

            if inner is None:
                break

        if inner is None or inner.type != "call_expression":
            continue

        if has_turbofish(inner, source):
            continue

        func = inner.child_by_field_name("function")

        if func is None:
            continue

        name = method_name(func, source)

        if name is None or name not in TURBOFISH_METHODS:
            continue

        line = line_number(source, let_node.start_byte)
        violations.append(
            Violation(
                path=path,
                line=line,
                code="NO_TYPE_HINT",
                message=f"type hint `{type_ann}` should be turbofish on `.{name}()`; "
                f"write `{name}::<{type_ann}>()` instead of annotating the let binding",
            ),
        )

    return violations


def fix_file(path: Path, source: bytes) -> tuple[bytes, bool]:
    tree = PARSER.parse(source)
    edits: list[tuple[int, int, bytes]] = []

    for let_node in descendants(tree.root_node, "let_declaration"):
        type_ann = type_annotation_text(let_node, source)

        if type_ann is None:
            continue

        value = let_node.child_by_field_name("value")

        if value is None:
            continue

        inner = value

        while inner.type in {"block", "return_expression", "parenthesized_expression"}:
            inner = inner.child_by_field_name("body") or inner.named_children[0] if inner.named_children else None

            if inner is None:
                break

        if inner is None or inner.type != "call_expression":
            continue

        if has_turbofish(inner, source):
            continue

        func = inner.child_by_field_name("function")

        if func is None:
            continue

        name = method_name(func, source)

        if name is None or name not in TURBOFISH_METHODS:
            continue

        type_node = let_node.child_by_field_name("type")

        if type_node is None:
            continue

        # Remove the type annotation: cut from ':' to end of type.
        # Walk back to find the ':'
        colon_end = type_node.start_byte

        while colon_end > 0 and source[colon_end - 1 : colon_end] != b":":
            colon_end -= 1

        colon_start = colon_end - 1  # position of ':'

        # Remove `: Type` — include trailing whitespace before `=`
        remove_end = type_node.end_byte

        while remove_end < len(source) and source[remove_end:remove_end + 1] in (b" ", b"\t"):
            remove_end += 1

        edits.append((colon_start, remove_end, b""))

    if not edits:
        return source, False

    result = bytearray(source)

    for start, end, replacement in sorted(edits, reverse=True):
        result[start:end] = replacement

    return bytes(result), True


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
    parser = argparse.ArgumentParser(description="Forbid type hints where turbofish is idiomatic")
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--fix", action="store_true", help="remove type annotations (add turbofish manually if needed)")
    args = parser.parse_args()

    root = args.root.resolve()
    paths = rust_files(root, args.paths)
    all_violations: list[Violation] = []

    for path in paths:
        source = path.read_bytes()
        visible_source = production_source(path, root)

        if args.fix and source == visible_source:
            fixed, changed = fix_file(path, source)

            if changed:
                path.write_bytes(fixed)
                # Re-read and check after fix
                source = path.read_bytes()

            visible_source = source

        violations = check_file(path, visible_source)
        all_violations.extend(violations)

    if all_violations:
        for v in sorted(all_violations, key=lambda v: (str(v.path), v.line)):
            print(f"{v.path.relative_to(root)}:{v.line}: {v.code}: {v.message}", file=sys.stderr)

        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
