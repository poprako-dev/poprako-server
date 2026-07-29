#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require method-resolution-only trait imports to use ``as _``."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))

# These dependency sources are not part of every checkout, but these are the
# external trait paths already governed by the project's Rust import style.
KNOWN_EXTERNAL_TRAITS = {
    "anyhow::Context",
    "futures::FutureExt",
    "futures::StreamExt",
    "itertools::Itertools",
    "poprako_util::time::ToUnixMilli",
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
MACRO_EXPANSION_TRAITS = {
    "crate::part_impl::repo::rdb_impl::incl::framework::BatchByIds",
    "crate::part_impl::repo::rdb_impl::incl::framework::Incl",
}


@dataclass(frozen=True)
class TraitImport:
    path: str
    local_name: str
    alias: str | None
    start_byte: int
    end_byte: int
    line: int


def rust_files(root: Path) -> list[Path]:
    excluded = {".git", ".venv", "target", "node_modules"}

    return sorted(
        path
        for path in root.rglob("*.rs")
        if not any(part in excluded for part in path.parts)
    )


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type == kind:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def path_parts(node: tree_sitter.Node, source: bytes) -> list[str]:
    return node_text(source, node).removesuffix("::").split("::")


def use_leaves(
    node: tree_sitter.Node,
    source: bytes,
    prefix: list[str] | None = None,
) -> list[tuple[str, str | None, tree_sitter.Node]]:
    """Flatten one use tree into (path, alias, leaf-node) tuples."""

    prefix = prefix or []

    if node.type == "use_declaration":
        return [
            leaf
            for child in node.named_children
            if child.type != "visibility_modifier"
            for leaf in use_leaves(child, source)
        ]

    if node.type == "scoped_use_list":
        base, use_list = node.named_children

        return use_leaves(use_list, source, prefix + path_parts(base, source))

    if node.type == "use_list":
        return [
            leaf
            for child in node.named_children
            for leaf in use_leaves(child, source, prefix)
        ]

    if node.type == "use_as_clause":
        path_node, alias_node = node.named_children
        path = prefix + path_parts(path_node, source)

        return [("::".join(path), node_text(source, alias_node), path_node)]

    if node.type == "scoped_identifier":
        path = prefix + path_parts(node, source)

        return [("::".join(path), None, node)]

    if node.type == "identifier":
        path = prefix + [node_text(source, node)]

        return [("::".join(path), None, node)]

    return []


def trait_names(root: Path) -> set[str]:
    names: set[str] = set()

    for path in rust_files(root):
        source = path.read_bytes()
        tree = PARSER.parse(source)

        for declaration in descendants(tree.root_node, "trait_item"):
            name = declaration.child_by_field_name("name")

            if name is not None:
                names.add(node_text(source, name))

    return names


def is_trait_path(path: str, names: set[str]) -> bool:
    return path in KNOWN_EXTERNAL_TRAITS or path.rsplit("::", 1)[-1] in names


def is_inside_use(node: tree_sitter.Node) -> bool:
    current = node.parent

    while current is not None:
        if current.type == "use_declaration":
            return True

        current = current.parent

    return False


def explicitly_used(
    tree: tree_sitter.Tree,
    source: bytes,
    imported: TraitImport,
) -> bool:
    if imported.path in MACRO_EXPANSION_TRAITS and b"preloadable!" in source:
        return True

    for macro in descendants(tree.root_node, "macro_invocation"):
        if macro.start_byte <= imported.end_byte:
            continue

        if re.search(
            rf"\b{re.escape(imported.local_name)}\b",
            node_text(source, macro),
        ):
            return True

    for node in descendants(tree.root_node, "identifier") + descendants(
        tree.root_node,
        "type_identifier",
    ):
        if node.start_byte <= imported.end_byte:
            continue

        if node_text(source, node) != imported.local_name:
            continue

        if is_inside_use(node):
            continue

        return True

    return False


def imports_in_file(path: Path, root: Path, names: set[str]) -> list[TraitImport]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    imports: list[TraitImport] = []

    for declaration in descendants(tree.root_node, "use_declaration"):
        if declaration.child_by_field_name("visibility") is not None:
            continue

        for path_name, alias, path_node in use_leaves(declaration, source):
            if path_name.rsplit("::", 1)[-1] == "self":
                continue

            local_name = alias or path_name.rsplit("::", 1)[-1]

            if alias == "_" or not is_trait_path(path_name, names):
                continue

            imports.append(
                TraitImport(
                    path=path_name,
                    local_name=local_name,
                    alias=alias,
                    start_byte=path_node.start_byte,
                    end_byte=declaration.end_byte,
                    line=path_node.start_point.row + 1,
                )
            )

    return imports


def check_file(path: Path, root: Path, names: set[str]) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    for imported in imports_in_file(path, root, names):
        if explicitly_used(tree, source, imported):
            continue

        diagnostics.append(
            f"{path.relative_to(root)}:{imported.line}: TRAIT001: "
            f"trait import `{imported.path}` is only used for method resolution; "
            "import it as `_`",
        )

    return diagnostics


def check_root(root: Path) -> list[str]:
    names = trait_names(root)

    return [
        diagnostic
        for path in rust_files(root)
        for diagnostic in check_file(path, root, names)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        src = root / "src"
        src.mkdir(parents=True)
        (src / "traits.rs").write_text(
            "pub trait MethodTrait { fn ping(&self); }\n"
            "pub trait NamedTrait { fn named(&self); }\n",
        )
        (src / "lib.rs").write_text(
            "mod traits;\n"
            "use crate::traits::MethodTrait;\n"
            "use crate::traits::NamedTrait;\n"
            "use poprako_util::time::ToUnixMilli;\n"
            "struct Value;\n"
            "impl NamedTrait for Value { fn named(&self) {} }\n"
            "fn call(value: &Value) { value.ping(); }\n"
            "fn millis(value: &Value) { value.to_unix_milli(); }\n"
            "fn bound<T: NamedTrait>() {}\n",
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 2 or not any(
            "MethodTrait" in diagnostic for diagnostic in diagnostics
        ):
            print("self-test: method-only trait import was not detected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        (src / "lib.rs").write_text(
            "mod traits;\n"
            "use crate::traits::MethodTrait as _;\n"
            "use crate::traits::NamedTrait;\n"
            "use poprako_util::time::ToUnixMilli as _;\n"
            "struct Value;\n"
            "impl NamedTrait for Value { fn named(&self) {} }\n"
            "fn call(value: &Value) { value.ping(); }\n"
            "fn millis(value: &Value) { value.to_unix_milli(); }\n"
            "fn bound<T: NamedTrait>() {}\n",
        )

        if check_root(root):
            print("self-test: valid trait imports were rejected", file=sys.stderr)
            return 1

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
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
