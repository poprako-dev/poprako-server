#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce canonical module-scope Rust `use` declarations."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from dataclasses import dataclass, replace
from pathlib import Path

import tree_sitter
import tree_sitter_rust


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
CATEGORIES = ("super", "std", "third_party", "local_crate", "crate")
KNOWN_TRAITS = {
    "anyhow::Context",
    "futures::FutureExt",
    "futures::StreamExt",
    "itertools::Itertools",
    "serde::Deserialize",
    "serde::Serialize",
    "std::convert::TryFrom",
    "std::convert::TryInto",
    "std::io::BufRead",
    "std::io::Read",
    "std::io::Write",
    "std::iter::Iterator",
    "tokio_stream::StreamExt",
    "tracing::Instrument",
}


@dataclass(frozen=True)
class Leaf:
    prefix: tuple[str, ...]
    leaf: str
    kind: str
    alias: str | None

    @property
    def full_path(self) -> str:
        return "::".join((*self.prefix, self.leaf))

    @property
    def identity(self) -> tuple[tuple[str, ...], str, str, str | None]:
        return self.prefix, self.leaf, self.kind, self.alias


@dataclass
class UseStmt:
    start: int
    end: int
    attr_start: int
    condition: tuple[str, ...]
    attr_signature: tuple[str, ...]
    indent: str
    scope_id: int
    in_tests_mod: bool
    requires_super_glob: bool
    leaves: list[Leaf]
    diagnostics: list[tuple[int, str, str]]


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    pending = [node]

    while pending:
        current = pending.pop()

        if current.type == kind:
            found.append(current)

        pending.extend(reversed(current.named_children))

    return found


def module_scope(node: tree_sitter.Node) -> bool:
    return node.parent is not None and node.parent.type in {"source_file", "declaration_list"}


def is_public_use(node: tree_sitter.Node, source: bytes) -> bool:
    return text(source, node).lstrip().startswith("pub use ")


def leading_attributes(node: tree_sitter.Node) -> tuple[tree_sitter.Node, ...]:
    parent = node.parent

    if parent is None:
        return ()

    siblings = parent.named_children
    index = next(index for index, sibling in enumerate(siblings) if sibling.id == node.id)
    attrs: list[tree_sitter.Node] = []

    for sibling in reversed(siblings[:index]):
        if sibling.type != "attribute_item":
            break

        attrs.append(sibling)

    return tuple(reversed(attrs))


def normalized_cfg(source: bytes, attr: tree_sitter.Node) -> str | None:
    attribute = attr.named_child(0)

    if attribute is None or text(source, attribute.child_by_field_name("name") or attribute)[:3] != "cfg":
        return None

    value = text(source, attribute)

    if not value.startswith("cfg("):
        return None

    return re.sub(r"\s+", "", value)


def inside_tests_mod(node: tree_sitter.Node, source: bytes) -> bool:
    current = node.parent

    while current is not None:
        if current.type == "mod_item":
            name = current.child_by_field_name("name")

            if name is not None and text(source, name) == "tests":
                return True

        current = current.parent

    return False


def path_segments(source: bytes, node: tree_sitter.Node) -> tuple[str, ...]:
    return tuple(segment.strip() for segment in text(source, node).split("::"))


def direct_brace_item(node: tree_sitter.Node, source: bytes) -> bool:
    if node.type in {"identifier", "crate", "self", "super"}:
        return True

    if node.type == "use_wildcard":
        return text(source, node).strip() == "*"

    if node.type != "use_as_clause":
        return False

    target = node.named_children[0] if node.named_children else None
    return target is not None and target.type in {"identifier", "self"}


def leaf_from_path(segments: tuple[str, ...], alias: str | None = None) -> Leaf:
    if not segments:
        raise ValueError("empty use path")

    if segments[-1] == "*":
        return Leaf(segments[:-1], "*", "glob", alias)

    if segments[-1] == "self":
        return Leaf(segments[:-1], "self", "self", alias)

    return Leaf(segments[:-1], segments[-1], "name", alias)


def parse_tree(
    source: bytes,
    node: tree_sitter.Node,
    prefix: tuple[str, ...] = (),
) -> tuple[list[Leaf], list[tuple[int, str, str]]]:
    diagnostics: list[tuple[int, str, str]] = []

    if node.type == "scoped_use_list":
        path = node.child_by_field_name("path")
        use_list = node.child_by_field_name("list")

        if path is None or use_list is None:
            return [], [(node.start_byte, "USE_PARSE_ERROR", "invalid scoped use list")]

        return parse_tree(source, use_list, (*prefix, *path_segments(source, path)))

    if node.type == "use_list":
        leaves: list[Leaf] = []

        for child in node.named_children:
            if not direct_brace_item(child, source):
                diagnostics.append(
                    (
                        child.start_byte,
                        "USE_BRACE_NON_LEAF",
                        "brace items must be direct leaves, not nested paths",
                    ),
                )

            child_leaves, child_diagnostics = parse_tree(source, child, prefix)
            leaves.extend(child_leaves)
            diagnostics.extend(child_diagnostics)

        return leaves, diagnostics

    if node.type == "use_as_clause":
        children = node.named_children

        if len(children) != 2:
            return [], [(node.start_byte, "USE_PARSE_ERROR", "invalid use alias")]

        leaves, diagnostics = parse_tree(source, children[0], prefix)
        alias = text(source, children[1])
        return [replace(leaf, alias=alias) for leaf in leaves], diagnostics

    if node.type == "use_wildcard":
        value = text(source, node).strip()
        segments = prefix if value == "*" else (*prefix, *path_segments(source, node)[:-1])
        return [Leaf(segments, "*", "glob", None)], diagnostics

    if node.type in {"identifier", "crate", "self", "super", "scoped_identifier"}:
        return [leaf_from_path((*prefix, *path_segments(source, node)))], diagnostics

    return [], [(node.start_byte, "USE_PARSE_ERROR", f"unsupported use node {node.type}")]


def collect_uses(path: Path, source: bytes) -> list[UseStmt]:
    tree = PARSER.parse(source)
    statements: list[UseStmt] = []

    for node in descendants(tree.root_node, "use_declaration"):
        if not module_scope(node) or is_public_use(node, source):
            continue

        argument = node.child_by_field_name("argument")

        if argument is None:
            continue

        attrs = leading_attributes(node)
        condition = tuple(filter(None, (normalized_cfg(source, attr) for attr in attrs)))
        leaves, diagnostics = parse_tree(source, argument)
        lexical_tests_mod = inside_tests_mod(node, source)
        test_file = path.name == "tests.rs" or "tests" in path.parts
        line_start = source.rfind(b"\n", 0, node.start_byte) + 1
        indent = source[line_start:node.start_byte].decode()
        attr_start = attrs[0].start_byte if attrs else node.start_byte
        statements.append(
            UseStmt(
                start=node.start_byte,
                end=node.end_byte,
                attr_start=attr_start,
                condition=condition,
                attr_signature=tuple(text(source, attr) for attr in attrs),
                indent=indent,
                scope_id=node.parent.id,
                in_tests_mod=lexical_tests_mod or test_file,
                requires_super_glob=lexical_tests_mod,
                leaves=leaves,
                diagnostics=diagnostics,
            ),
        )

    return sorted(statements, key=lambda statement: statement.start)


def workspace_crates(root: Path) -> set[str]:
    crates = {root.name.replace("-", "_")}

    for cargo_toml in (root / "Cargo.toml", *root.glob("*/Cargo.toml")):
        if not cargo_toml.exists():
            continue

        for match in re.finditer(r'^name\s*=\s*"([^"]+)"', cargo_toml.read_text(), re.MULTILINE):
            crates.add(match.group(1).replace("-", "_"))

    return crates


def category(leaf: Leaf, local_crates: set[str]) -> str:
    root = (leaf.prefix or (leaf.leaf,))[0]

    if root == "super":
        return "super"

    if root == "std":
        return "std"

    if root in {"crate", "self"}:
        return "crate"

    if root in local_crates or root.endswith("_util"):
        return "local_crate"

    return "third_party"


def canonicalize(leaves: list[Leaf]) -> list[Leaf]:
    prefixes = {leaf.prefix for leaf in leaves}
    normalized = []
    identities = set()

    for leaf in leaves:
        full = (*leaf.prefix, leaf.leaf)

        if leaf.kind == "name" and leaf.alias is None and full in prefixes:
            leaf = Leaf(full, "self", "self", None)

        if leaf.identity not in identities:
            normalized.append(leaf)
            identities.add(leaf.identity)

    return normalized


def explicit_trait_use(masked: bytes, name: str) -> bool:
    escaped = re.escape(name.encode())
    patterns = (
        rb"\bimpl\b[^;{}]*\b" + escaped + rb"\b[^;{}]*\bfor\b",
        rb"\bdyn\s+" + escaped + rb"\b",
        rb"<[^>]*\bas\s+" + escaped + rb"\b",
        rb"\bwhere\b[^{};]*:\s*[^{};]*\b" + escaped + rb"\b",
        rb"[:+<,]\s*" + escaped + rb"\b",
        rb"derive\s*\([^)]*\b" + escaped + rb"\b",
    )
    return any(re.search(pattern, masked, re.MULTILINE) for pattern in patterns)


def apply_trait_aliases(leaves: list[Leaf], masked: bytes) -> tuple[list[Leaf], list[Leaf]]:
    changed: list[Leaf] = []
    fixed: list[Leaf] = []

    for leaf in leaves:
        if leaf.kind == "name" and leaf.full_path in KNOWN_TRAITS and leaf.alias != "_":
            local_name = leaf.alias or leaf.leaf

            if not explicit_trait_use(masked, local_name):
                changed.append(leaf)
                leaf = replace(leaf, alias="_")

        fixed.append(leaf)

    return fixed, changed


def render_leaf(leaf: Leaf) -> str:
    base = "::".join((*leaf.prefix, "*" if leaf.kind == "glob" else leaf.leaf))

    if leaf.kind == "self":
        base = "::".join(leaf.prefix)

    return f"use {base}{f' as {leaf.alias}' if leaf.alias else ''};"


def render_bucket(leaves: list[Leaf]) -> str:
    rank = {"self": 0, "name": 1, "glob": 2}
    leaves = sorted(leaves, key=lambda leaf: (rank[leaf.kind], leaf.leaf, leaf.alias or ""))

    if len(leaves) == 1:
        return render_leaf(leaves[0])

    prefix = "::".join(leaves[0].prefix)
    items = []

    for leaf in leaves:
        name = "self" if leaf.kind == "self" else "*" if leaf.kind == "glob" else leaf.leaf
        items.append(f"{name}{f' as {leaf.alias}' if leaf.alias else ''}")

    return f"use {prefix}::{{{', '.join(items)}}};"


def render_block(leaves: list[Leaf], local_crates: set[str], indent: str) -> str:
    by_category: dict[str, list[Leaf]] = {category_name: [] for category_name in CATEGORIES}

    for leaf in canonicalize(leaves):
        by_category[category(leaf, local_crates)].append(leaf)

    lines: list[str] = []

    for category_name in CATEGORIES:
        buckets: dict[tuple[str, ...], list[Leaf]] = {}

        for leaf in by_category[category_name]:
            buckets.setdefault(leaf.prefix, []).append(leaf)

        group = [render_bucket(bucket) for bucket in buckets.values()]

        if group and lines:
            lines.append("")

        lines.extend(group)

    return ("\n" + indent).join(lines)


def contiguous_blocks(source: bytes, statements: list[UseStmt]) -> list[list[UseStmt]]:
    blocks: list[list[UseStmt]] = []
    current: list[UseStmt] = []

    for statement in statements:
        if current:
            previous = current[-1]
            gap = source[previous.end : statement.attr_start]
            separated = previous.scope_id != statement.scope_id or gap.strip()

            if separated:
                blocks.append(current)
                current = []

        current.append(statement)

    if current:
        blocks.append(current)

    return blocks


def condition_segments(block: list[UseStmt]) -> list[list[UseStmt]]:
    segments: list[list[UseStmt]] = []
    current: list[UseStmt] = []

    for statement in block:
        if current and (
            statement.condition != current[-1].condition
            or statement.attr_signature != current[-1].attr_signature
        ):
            segments.append(current)
            current = []

        current.append(statement)

    if current:
        segments.append(current)

    return segments


def line(path: Path, root: Path, source: bytes, index: int, code: str, message: str) -> str:
    return f"{path.relative_to(root)}:{source.count(b'\n', 0, index) + 1}: {code}: {message}"


def masked_source(source: bytes, statements: list[UseStmt]) -> bytes:
    masked = bytearray(source)

    for statement in statements:
        for index in range(statement.start, statement.end):
            if masked[index] not in (10, 13):
                masked[index] = 32

    return bytes(masked)


def check_test_super_imports(
    path: Path,
    root: Path,
    source: bytes,
    statements: list[UseStmt],
) -> list[str]:
    scopes: dict[int, list[UseStmt]] = {}

    for statement in statements:
        if statement.requires_super_glob:
            scopes.setdefault(statement.scope_id, []).append(statement)

    diagnostics: list[str] = []

    for scope_statements in scopes.values():
        super_statements = [
            statement
            for statement in scope_statements
            if any(category(leaf, set()) == "super" for leaf in statement.leaves)
        ]

        if not super_statements:
            diagnostics.append(line(path, root, source, scope_statements[0].start, "USE_SUPER_IN_TESTS_MISSING", "mod tests must contain exactly one `use super::*;`"))
            continue

        for statement in super_statements:
            pure_glob = len(statement.leaves) == 1 and statement.leaves[0] == Leaf(("super",), "*", "glob", None)

            if not pure_glob:
                diagnostics.append(line(path, root, source, statement.start, "USE_SUPER_IN_TESTS_NOT_GLOB", "super imports in mod tests must be `use super::*;`"))

        for statement in super_statements[1:]:
            diagnostics.append(line(path, root, source, statement.start, "USE_SUPER_IN_TESTS_TOO_MANY", "only one `use super::*;` is allowed in mod tests"))

    return diagnostics


def check_file(path: Path, root: Path, local_crates: set[str]) -> tuple[list[str], list[tuple[int, int, bytes]]]:
    source = path.read_bytes()
    statements = collect_uses(path, source)
    masked = masked_source(source, statements)
    diagnostics: list[str] = []
    edits: list[tuple[int, int, bytes]] = []

    for statement in statements:
        for index, code, message in statement.diagnostics:
            diagnostics.append(line(path, root, source, index, code, message))

        for leaf in statement.leaves:
            if category(leaf, local_crates) == "super" and not statement.in_tests_mod:
                diagnostics.append(line(path, root, source, statement.start, "USE_SUPER_OUTSIDE_TESTS", "`super` imports are only allowed inside mod tests"))

    diagnostics.extend(check_test_super_imports(path, root, source, statements))

    for block in contiguous_blocks(source, statements):
        for segment in condition_segments(block):
            leaves = [leaf for statement in segment for leaf in statement.leaves]
            aliased_leaves, missing_aliases = apply_trait_aliases(leaves, masked)

            for leaf in missing_aliases:
                diagnostics.append(line(path, root, source, segment[0].start, "USE_TRAIT_ALIAS_MISSING", f"trait import `{leaf.full_path}` should use `as _`"))

            categories = [
                {category(leaf, local_crates) for leaf in statement.leaves}
                for statement in segment
            ]
            last_category = -1

            for statement, statement_categories in zip(segment, categories, strict=True):
                if len(statement_categories) > 1:
                    diagnostics.append(line(path, root, source, statement.start, "USE_MIXED_GROUP", "one use tree must not mix import groups"))

                if statement_categories:
                    category_index = max(CATEGORIES.index(value) for value in statement_categories)

                    if category_index < last_category:
                        diagnostics.append(line(path, root, source, statement.start, "USE_GROUP_ORDER", "use group appears after a later group"))

                    last_category = max(last_category, category_index)

            for previous, current in zip(segment, segment[1:]):
                previous_categories = {category(leaf, local_crates) for leaf in previous.leaves}
                current_categories = {category(leaf, local_crates) for leaf in current.leaves}

                if previous_categories and current_categories and previous_categories != current_categories:
                    gap = source[previous.end : current.attr_start]

                    if gap.count(b"\n") != 2:
                        diagnostics.append(line(path, root, source, current.start, "USE_GROUP_BLANK_LINE", "different use groups need exactly one blank line"))

            canonical = canonicalize(aliased_leaves)
            raw_identities = [leaf.identity for leaf in aliased_leaves]

            if len(set(raw_identities)) != len(raw_identities):
                diagnostics.append(line(path, root, source, segment[0].start, "USE_DUPLICATE_IMPORT", "duplicate imports must be merged"))

            buckets: dict[tuple[str, ...], list[Leaf]] = {}

            for leaf in canonical:
                buckets.setdefault(leaf.prefix, []).append(leaf)

            for bucket in buckets.values():
                if len(bucket) < 2:
                    continue

                expected = {leaf.identity for leaf in bucket}

                if not any({leaf.identity for leaf in canonicalize(statement.leaves)} == expected for statement in segment):
                    diagnostics.append(line(path, root, source, segment[0].start, "USE_MISSING_MERGE", f"imports under `{'::'.join(bucket[0].prefix)}` must share one use tree"))

            if any(
                diagnostic[1] == "USE_PARSE_ERROR"
                for statement in segment
                for diagnostic in statement.diagnostics
            ):
                continue

            fixed_leaves, _ = apply_trait_aliases(leaves, masked)
            rendered = render_block(fixed_leaves, local_crates, segment[0].indent)
            first = segment[0]
            prefix = source[first.attr_start : first.start]
            replacement = prefix + rendered.encode()
            edits.append((first.attr_start, segment[-1].end, replacement))

    return diagnostics, edits


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


def apply_fixes(path: Path, edits: list[tuple[int, int, bytes]]) -> None:
    source = path.read_bytes()

    for start, end, replacement in sorted(edits, reverse=True):
        source = source[:start] + replacement + source[end:]

    path.write_bytes(source)


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        fixture = root / "src" / "lib.rs"
        fixture.parent.mkdir(parents=True)
        fixture.write_text(
            "#[cfg(feature = \"a\")]\n"
            "use crate::a::A;\n"
            "#[cfg(not(feature = \"a\"))]\n"
            "use crate::a::B;\n"
            "#[cfg(unix)]\n"
            "use std::io::Write;\n"
            "fn local() { use std::mem::take; }\n"
            "pub use crate::reexport::Thing;\n",
        )
        diagnostics, _ = check_file(fixture, root, {root.name})

        if any("USE_GROUP_ORDER" in item or "USE_MISSING_MERGE" in item for item in diagnostics):
            print("self-test: cfg conditions were merged into one import group", file=sys.stderr)
            return 1

        if not any("USE_TRAIT_ALIAS_MISSING" in item for item in diagnostics):
            print("self-test: known trait import was not checked", file=sys.stderr)
            return 1

        fixture.write_text(
            "#[cfg(feature = \"a\")]\n"
            "use std::{mem::take, time};\n"
            "#[cfg(feature = \"a\")]\n"
            "use std::mem::take;\n"
            "#[cfg(not(feature = \"a\"))]\n"
            "use crate::{b::B, a::A};\n",
        )
        _, edits = check_file(fixture, root, {root.name})
        apply_fixes(fixture, edits)
        fixed = fixture.read_text()

        if "use std::{mem::take, time};" in fixed or fixed.count("#[cfg(feature = \"a\")]") != 1:
            print("self-test: cfg-safe use fix was not applied", file=sys.stderr)
            return 1

        diagnostics, _ = check_file(fixture, root, {root.name})

        if diagnostics:
            print("self-test: fixed source still has diagnostics", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        fixture.write_text("mod tests {\n    use super::*;\n}\n")
        diagnostics, _ = check_file(fixture, root, {root.name})

        if diagnostics:
            print("self-test: valid test-module super import was rejected", file=sys.stderr)
            return 1

        fixture.write_text("mod tests {\n    use super::helper;\n}\n")
        diagnostics, _ = check_file(fixture, root, {root.name})

        if not any("USE_SUPER_IN_TESTS_NOT_GLOB" in item for item in diagnostics):
            print("self-test: invalid test-module super import was accepted", file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--fix", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    root = args.root.resolve()
    local_crates = workspace_crates(root)
    diagnostics: list[str] = []

    for path in rust_files(root, args.paths):
        errors, edits = check_file(path, root, local_crates)

        if args.fix:
            apply_fixes(path, edits)
            errors, _ = check_file(path, root, local_crates)

        diagnostics.extend(errors)

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
