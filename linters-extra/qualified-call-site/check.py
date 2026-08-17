#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce qualified-path policy at Rust call sites."""

from __future__ import annotations

import argparse
import sys
import tempfile
import tomllib
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
PATH_NODES = {"scoped_identifier", "scoped_type_identifier"}


def descendants(node: tree_sitter.Node) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    pending = [node]

    while pending:
        current = pending.pop()
        found.append(current)
        pending.extend(reversed(current.named_children))

    return found


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def path_parts(source: bytes, node: tree_sitter.Node) -> tuple[str, ...]:
    return tuple(text(source, node).removesuffix("::").split("::"))


def imported_paths(
    node: tree_sitter.Node,
    source: bytes,
    prefix: tuple[str, ...] = (),
) -> list[tuple[tuple[str, ...], str, tree_sitter.Node]]:
    if node.type == "use_declaration":
        return [
            leaf
            for child in node.named_children
            if child.type != "visibility_modifier"
            for leaf in imported_paths(child, source)
        ]

    if node.type == "scoped_use_list":
        base, use_list = node.named_children
        return imported_paths(use_list, source, prefix + path_parts(source, base))

    if node.type == "use_list":
        return [
            leaf
            for child in node.named_children
            for leaf in imported_paths(child, source, prefix)
        ]

    if node.type == "use_as_clause":
        path_node, alias_node = node.named_children
        path = prefix + path_parts(source, path_node)
        return [(path, text(source, alias_node), path_node)]

    if node.type in {"scoped_identifier", "scoped_type_identifier"}:
        path = prefix + path_parts(source, node)
        return [(path, path[-1], node)]

    if node.type == "identifier":
        path = prefix + (text(source, node),)
        return [(path, path[-1], node)]

    return []


def inside(node: tree_sitter.Node, kind: str) -> bool:
    current = node.parent

    while current is not None:
        if current.type == kind:
            return True

        current = current.parent

    return False


def nested_path(node: tree_sitter.Node) -> bool:
    current = node.parent

    while current is not None:
        if current.type in PATH_NODES:
            return True

        current = current.parent

    return False


def inside_macro(node: tree_sitter.Node) -> bool:
    current = node.parent

    while current is not None:
        if current.type in {"attribute_item", "macro_invocation"}:
            return True

        current = current.parent

    return False


def path_root(node: tree_sitter.Node, source: bytes) -> str | None:
    path = text(source, node).removeprefix("::")
    root, separator, _ = path.partition("::")

    if not separator:
        return None

    return root


def diagnostic(path: Path, root: Path, node: tree_sitter.Node, code: str, message: str) -> str:
    return f"{path.relative_to(root)}:{node.start_point.row + 1}:{node.start_point.column + 1}: {code}: {message}"


def third_party_imports(tree: tree_sitter.Tree, source: bytes) -> dict[str, str]:
    imports: dict[str, str] = {}

    for declaration in descendants(tree.root_node):
        if declaration.type != "use_declaration":
            continue

        for path, local_name, path_node in imported_paths(declaration, source):
            root_name = path[0]

            if root_name in {"crate", "self", "super", "std"} or root_name.startswith("poprako_"):
                continue

            if local_name != "_":
                imports[local_name] = "::".join(path)

    return imports


def check_file(
    path: Path,
    root: Path,
    enforced_std_paths: set[str],
    exempt_poprako_paths: set[str],
    enforced_third_party_paths: set[str],
) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    diagnostics: list[str] = []
    imported_third_party = third_party_imports(tree, source)
    reported: set[tuple[int, int, str]] = set()

    for node in descendants(tree.root_node):
        if (
            node.type not in PATH_NODES
            or nested_path(node)
            or inside(node, "use_declaration")
            or inside_macro(node)
        ):
            continue

        root_name = path_root(node, source)

        qualified_path = text(source, node)

        if qualified_path in enforced_std_paths:
            key = (node.start_byte, node.end_byte, "QCS001")

            if key in reported:
                continue

            reported.add(key)
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    node,
                    "QCS001",
                    f"standard-library path `{qualified_path}` must be imported before use",
                ),
            )
            continue

        if (
            root_name is not None
            and root_name.startswith("poprako_")
            and qualified_path not in exempt_poprako_paths
        ):
            key = (node.start_byte, node.end_byte, "QCS002")

            if key in reported:
                continue

            reported.add(key)
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    node,
                    "QCS002",
                    f"PopRaKo crate path `{qualified_path}` must be imported before use",
                ),
            )

    for node in descendants(tree.root_node):
        if node.type not in {"identifier", "type_identifier"}:
            continue

        local_name = text(source, node)

        if (
            local_name not in imported_third_party
            or inside(node, "use_declaration")
            or nested_path(node)
            or inside_macro(node)
        ):
            continue

        if imported_third_party[local_name] not in enforced_third_party_paths:
            continue

        key = (node.start_byte, node.end_byte, "QCS003")

        if key in reported:
            continue

        reported.add(key)
        diagnostics.append(
            diagnostic(
                path,
                root,
                node,
                "QCS003",
                f"third-party import `{imported_third_party[local_name]}` must remain qualified at call sites",
            ),
        )

    return diagnostics


def configured_paths(root: Path) -> set[str]:
    config_path = root / "rust-style-lint.toml"

    if not config_path.is_file():
        return set()

    with config_path.open("rb") as config_file:
        config = tomllib.load(config_file)

    section = config.get("qualified-call-site", {})
    return {str(import_path) for import_path in section.get("enforced_third_party_paths", [])}


def configured_set(root: Path, key: str) -> set[str]:
    config_path = root / "rust-style-lint.toml"

    if not config_path.is_file():
        return set()

    with config_path.open("rb") as config_file:
        config = tomllib.load(config_file)

    section = config.get("qualified-call-site", {})
    return {str(value) for value in section.get(key, [])}


def check_root(root: Path) -> list[str]:
    enforced_std_paths = configured_set(root, "enforced_std_paths")
    exempt_poprako_paths = configured_set(root, "exempt_poprako_paths")
    enforced_third_party_paths = configured_paths(root)

    return [
        diagnostic
        for path in sorted((root / "src").rglob("*.rs"))
        for diagnostic in check_file(
            path,
            root,
            enforced_std_paths,
            exempt_poprako_paths,
            enforced_third_party_paths,
        )
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir(parents=True)
        (root / "rust-style-lint.toml").write_text(
            "[qualified-call-site]\n"
            "enforced_std_paths = [\"std::env::var\", \"std::result::Result\"]\n"
            "exempt_poprako_paths = [\"poprako_server::serve\", \"poprako_server::init_log\"]\n"
            "enforced_third_party_paths = [\"jsonwebtoken::encode\", \"jsonwebtoken::decode\", \"serde_json::Value\"]\n",
        )
        fixture = source_dir / "lib.rs"

        fixture.write_text(
            "use std::mem::take;\n"
            "use poprako_orchestra::Context;\n"
            "use serde_json::Value;\n"
            "use serde::Deserialize;\n"
            "use axum::Json;\n"
            "use time::OffsetDateTime;\n"
            "#[derive(serde::Deserialize)]\n"
            "#[tracing::instrument]\n"
            "struct Item;\n"
            "fn valid(_: Context, value: serde_json::Value) {\n"
            "    let _ = take;\n"
            "    let _ = serde_json::json!({\"value\": value});\n"
            "    let _: Json<serde_json::Value> = todo!();\n"
            "    let _: OffsetDateTime = todo!();\n"
            "}\n",
        )

        if check_root(root):
            print("self-test: valid qualified-path policy was rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "use serde_json::Value;\n"
            "fn invalid<C: poprako_orchestra::Context>(value: std::result::Result<(), ()>) {\n"
            "    let _ = std::env::var(\"VALUE\");\n"
            "    let _ = value;\n"
            "    let _: Value = value;\n"
            "}\n",
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 4 or not any("QCS001" in item for item in diagnostics) or not any("QCS002" in item for item in diagnostics) or not any("QCS003" in item for item in diagnostics):
            print("self-test: forbidden qualified call-site paths were not fully rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="qualified-call-site")
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
