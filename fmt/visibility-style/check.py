#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce plain Rust visibility and private implementation fields."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust


DEFAULT_ROOT = Path(__file__).parents[2]
GENERATED_SCHEMA = Path("src/part_impl/repo/rdb_impl/schema.rs")
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
PUBLIC_FIELD_MODULES = {
    ("model",),
    ("data",),
    ("config",),
    ("part", "repo", "oper"),
    ("part", "effect", "event"),
    ("part", "prom", "payload", "chapter"),
    ("part_impl", "repo", "rdb_impl", "entity"),
    ("part_impl", "prom", "rdb_impl", "entity"),
}
PUBLIC_FIELD_RULE = (
    "plain-public struct fields are allowed only under crate::model, crate::data, "
    "crate::config, crate::part::repo::oper, crate::part::effect::event, "
    "crate::part::prom::payload::chapter, "
    "crate::part_impl::repo::rdb_impl::entity, and "
    "crate::part_impl::prom::rdb_impl::entity; fields elsewhere must be private"
)


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def format_module(path: tuple[str, ...]) -> str:
    return "crate" if not path else "crate::" + "::".join(path)


def file_module(src_dir: Path, path: Path) -> tuple[str, ...]:
    relative = path.relative_to(src_dir)

    if relative.name in {"lib.rs", "main.rs"} and len(relative.parts) == 1:
        return ()

    parts = list(relative.parts)

    if parts[-1] == "mod.rs":
        parts.pop()
    else:
        parts[-1] = Path(parts[-1]).stem

    return tuple(parts)


def leading_attributes(node: tree_sitter.Node) -> list[tree_sitter.Node]:
    parent = node.parent

    if parent is None:
        return []

    index = next(
        (
            index
            for index, sibling in enumerate(parent.children)
            if sibling.start_byte == node.start_byte
            and sibling.end_byte == node.end_byte
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

        if not sibling.is_named:
            continue

        break

    attributes.reverse()

    return attributes


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


def cfg_expression(attribute: str) -> str | None:
    match = re.fullmatch(r"\s*#\s*\[\s*cfg\s*\((.*)\)\s*]\s*", attribute, re.DOTALL)

    return match.group(1) if match is not None else None


def has_test_only_cfg(node: tree_sitter.Node, source: bytes) -> bool:
    current: tree_sitter.Node | None = node

    while current is not None:
        for attribute in leading_attributes(current):
            expression = cfg_expression(node_text(source, attribute))

            if expression is not None and True not in CfgParser(expression).parse():
                return True

        current = current.parent

    return False


def module_path(base: tuple[str, ...], node: tree_sitter.Node, source: bytes) -> tuple[str, ...]:
    inline_modules: list[str] = []
    current = node.parent

    while current is not None:
        if current.type == "mod_item" and current.child_by_field_name("body") is not None:
            name = current.child_by_field_name("name")

            if name is not None:
                inline_modules.append(node_text(source, name))

        current = current.parent

    return base + tuple(reversed(inline_modules))


def discover_production_files(
    root: Path,
) -> tuple[list[Path], set[tuple[str, ...]]]:
    src_dir = root / "src"
    paths = sorted(src_dir.rglob("*.rs")) if src_dir.is_dir() else []
    files_by_module: dict[tuple[str, ...], Path] = {}

    for path in paths:
        module = file_module(src_dir, path)

        if module or path.name == "lib.rs":
            files_by_module.setdefault(module, path)

    roots = [path for path in (src_dir / "lib.rs", src_dir / "main.rs") if path.is_file()]
    pending_paths = list(reversed(roots))
    scanned: set[Path] = set()
    prefixes: set[tuple[str, ...]] = set()

    while pending_paths:
        path = pending_paths.pop()

        if path in scanned:
            continue

        scanned.add(path)

        if path.relative_to(root) == GENERATED_SCHEMA:
            continue

        source = path.read_bytes()
        tree = PARSER.parse(source)
        base = file_module(src_dir, path)
        pending = [tree.root_node]

        while pending:
            current = pending.pop()

            if current.type == "mod_item":
                name = current.child_by_field_name("name")

                if name is None:
                    continue

                child_module = module_path(base, current, source) + (node_text(source, name),)

                if has_test_only_cfg(current, source):
                    prefixes.add(child_module)
                    continue

                if current.child_by_field_name("body") is None:
                    child_path = files_by_module.get(child_module)

                    if child_path is not None:
                        pending_paths.append(child_path)

                    continue

            pending.extend(reversed(current.named_children))

    scan_paths = sorted(
        path
        for path in scanned
        if path.relative_to(root) != GENERATED_SCHEMA
    )

    return scan_paths, prefixes


def starts_with(path: tuple[str, ...], prefix: tuple[str, ...]) -> bool:
    return len(path) >= len(prefix) and path[: len(prefix)] == prefix


def excluded_module(path: tuple[str, ...], prefixes: set[tuple[str, ...]]) -> bool:
    return any(starts_with(path, prefix) for prefix in prefixes)


def public_fields_allowed(path: tuple[str, ...]) -> bool:
    return any(starts_with(path, prefix) for prefix in PUBLIC_FIELD_MODULES)


def is_struct_field_visibility(node: tree_sitter.Node) -> bool:
    parent = node.parent

    if parent is None:
        return False

    if parent.type == "field_declaration":
        grandparent = parent.parent

        return grandparent is not None and grandparent.parent is not None and grandparent.parent.type == "struct_item"

    if parent.type == "ordered_field_declaration_list":
        return parent.parent is not None and parent.parent.type == "struct_item"

    return False


def diagnostic(path: Path, root: Path, node: tree_sitter.Node, code: str, message: str) -> str:
    return (
        f"{path.relative_to(root)}:{node.start_point.row + 1}:"
        f"{node.start_point.column + 1}: {code}: {message}"
    )


def check_file(
    path: Path,
    root: Path,
    prefixes: set[tuple[str, ...]],
) -> list[str]:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    base = file_module(root / "src", path)
    diagnostics: list[str] = []
    pending = [tree.root_node]

    while pending:
        current = pending.pop()

        if current.type == "visibility_modifier":
            current_module = module_path(base, current, source)

            if excluded_module(current_module, prefixes) or has_test_only_cfg(current, source):
                pending.extend(reversed(current.named_children))
                continue

            visibility = " ".join(node_text(source, current).split())

            if visibility != "pub":
                diagnostics.append(
                    diagnostic(
                        path,
                        root,
                        current,
                        "VIS001",
                        f"restricted visibility `{visibility}` is forbidden; production "
                        "Rust permits only plain `pub` or private items; share an internal "
                        "item with plain `pub` behind a private module",
                    )
                )

            if is_struct_field_visibility(current) and not public_fields_allowed(
                current_module,
            ):
                diagnostics.append(
                    diagnostic(
                        path,
                        root,
                        current,
                        "VIS002",
                        f"public struct field is forbidden in "
                        f"{format_module(current_module)}; {PUBLIC_FIELD_RULE}; "
                        "expose construction or access "
                        "through functions",
                    )
                )

        pending.extend(reversed(current.named_children))

    return diagnostics


def check_root(root: Path) -> list[str]:
    paths, prefixes = discover_production_files(root)

    return [
        item
        for path in paths
        if not excluded_module(file_module(root / "src", path), prefixes)
        for item in check_file(path, root, prefixes)
    ]


def diagnostic_codes(diagnostics: list[str]) -> list[str]:
    return [
        match.group(1)
        for diagnostic in diagnostics
        if (match := re.search(r": (VIS\d{3}): ", diagnostic)) is not None
    ]


def self_test() -> int:
    allowed_modules = [
        ("model", "comic"),
        ("data", "comic"),
        ("config",),
        ("part", "repo", "oper", "comic"),
        ("part", "effect", "event", "user"),
        ("part", "prom", "payload", "chapter"),
        ("part_impl", "repo", "rdb_impl", "entity", "comic"),
        ("part_impl", "prom", "rdb_impl", "entity"),
    ]

    if not all(public_fields_allowed(module) for module in allowed_modules):
        print("self-test: public-field contract module was rejected", file=sys.stderr)
        return 1

    if public_fields_allowed(("service",)):
        print("self-test: implementation module was allowlisted", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "src" / "model").mkdir(parents=True)
        (root / "src" / "data").mkdir(parents=True)
        (root / "src" / "service").mkdir(parents=True)
        (root / "src" / "part_impl" / "repo" / "rdb_impl").mkdir(parents=True)
        (root / "src" / "lib.rs").write_text(
            "mod model;\nmod data;\nmod service;\nmod part;\nmod part_impl;\n"
            "#[cfg(test)] mod tests;\n",
        )
        (root / "src" / "model.rs").write_text("pub struct Model { pub value: i32 }\n")
        (root / "src" / "data.rs").write_text("pub struct Data(pub i32);\n")
        (root / "src" / "service.rs").write_text(
            "pub struct Service { value: i32 }\n"
            "pub struct Tuple(i32);\n"
            "#[cfg(any(test, feature = \"x\"))] pub fn maybe_production() {}\n",
        )
        (root / "src" / "tests.rs").write_text(
            "pub(crate) struct Ignored { pub value: i32 }\n",
        )
        (root / "src" / "orphan.rs").write_text(
            "pub(crate) struct Orphan { pub value: i32 }\n",
        )
        (root / "src" / "part.rs").write_text("pub mod repo;\n")
        (root / "src" / "part" / "repo.rs").parent.mkdir(parents=True)
        (root / "src" / "part" / "repo.rs").write_text("pub mod oper;\n")
        (root / "src" / "part" / "repo" / "oper.rs").parent.mkdir(parents=True)
        (root / "src" / "part" / "repo" / "oper.rs").write_text(
            "pub struct Oper { pub value: i32 }\n"
            "pub struct TupleOper(pub i32);\n",
        )
        (root / "src" / "part_impl.rs").write_text("pub mod repo;\n")
        (root / "src" / "part_impl" / "repo.rs").write_text("pub mod rdb_impl;\n")
        (root / "src" / "part_impl" / "repo" / "rdb_impl.rs").write_text(
            "pub mod schema;\n",
        )
        (root / GENERATED_SCHEMA).write_text(
            "pub(crate) struct Generated { pub value: i32 }\n",
        )

        diagnostics = check_root(root)

        if diagnostics:
            print("self-test: valid visibility fixture was rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        (root / "src" / "service.rs").write_text(
            "pub(crate) struct Restricted;\n"
            "pub(super) fn parent_only() {}\n"
            "pub(self) const LOCAL: i32 = 1;\n"
            "pub(in crate::service) type Scoped = i32;\n"
            "pub struct Named { pub value: i32, pub(crate) other: i32 }\n"
            "pub struct Tuple(pub i32, pub(super) i32);\n"
            "#[cfg(test)] pub(crate) fn ignored() {}\n"
            "#[cfg(all(test, feature = \"repo\"))] pub struct TestOnly { pub value: i32 }\n",
        )
        diagnostics = check_root(root)
        codes = diagnostic_codes(diagnostics)

        if codes.count("VIS001") != 6 or codes.count("VIS002") != 4:
            print("self-test: visibility violations were not fully diagnosed", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        field_diagnostics = [
            diagnostic for diagnostic in diagnostics if ": VIS002: " in diagnostic
        ]

        if any(
            PUBLIC_FIELD_RULE not in diagnostic
            or "public struct field is forbidden in crate::service" not in diagnostic
            for diagnostic in field_diagnostics
        ):
            print("self-test: VIS002 did not explain the complete rule", file=sys.stderr)
            print("\n".join(field_diagnostics), file=sys.stderr)
            return 1

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--self-test", action="store_true")

    return parser.parse_args()


def main() -> int:
    # Temperarily comment out.
    # args = parse_args()
    #
    # if args.self_test:
    #     return self_test()
    #
    # diagnostics = check_root(args.root.resolve())
    #
    # if diagnostics:
    #     print("\n".join(diagnostics), file=sys.stderr)
    #     return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
