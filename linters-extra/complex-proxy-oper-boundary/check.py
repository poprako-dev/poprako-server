#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Keep complex modules free from ports and Orchestra dispatch."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
PROM_FORBIDDEN_PATHS = {"oper", "task", "Prom", "*"}


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
    if node.type in {"crate", "self", "super", "identifier"}:
        return (text(source, node),)

    if node.type in {"scoped_identifier", "scoped_type_identifier"}:
        path = node.child_by_field_name("path")
        name = node.child_by_field_name("name")

        if path is not None and name is not None:
            return path_parts(source, path) + (text(source, name),)

    return tuple(text(source, node).split("::"))


def imported_paths(
    node: tree_sitter.Node,
    source: bytes,
    prefix: tuple[str, ...] = (),
) -> list[tuple[tuple[str, ...], tree_sitter.Node]]:
    if node.type == "use_declaration":
        return [
            imported_path
            for child in node.named_children
            if child.type != "visibility_modifier"
            for imported_path in imported_paths(child, source)
        ]

    if node.type == "scoped_use_list":
        base = node.child_by_field_name("path")
        use_list = node.child_by_field_name("list")

        if base is None or use_list is None:
            return []

        base_path = prefix + path_parts(source, base)
        return [(base_path, base)] + imported_paths(use_list, source, base_path)

    if node.type == "use_list":
        return [
            imported_path
            for child in node.named_children
            for imported_path in imported_paths(child, source, prefix)
        ]

    if node.type == "use_as_clause":
        path_node = node.child_by_field_name("path")

        if path_node is not None:
            return [(prefix + path_parts(source, path_node), path_node)]

        return []

    if node.type in {"scoped_identifier", "scoped_type_identifier", "identifier"}:
        return [(prefix + path_parts(source, node), node)]

    if node.type == "use_wildcard":
        return [(prefix + ("*",), node)]

    return []


def diagnostic(
    path: Path,
    root: Path,
    node: tree_sitter.Node,
    code: str,
    message: str,
) -> str:
    return f"{path.relative_to(root)}:{node.start_point.row + 1}:{node.start_point.column + 1}: {code}: {message}"


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node):
        if declaration.type != "use_declaration":
            continue

        imported = imported_paths(declaration, source)
        orchestra_path = next(
            (node for path_parts_value, node in imported if path_parts_value[0] == "poprako_orchestra"),
            None,
        )
        repository_path = next(
            (
                node
                for path_parts_value, node in imported
                if path_parts_value[:3] == ("crate", "part", "repo")
            ),
            None,
        )
        prom_path = next(
            (
                node
                for path_parts_value, node in imported
                if path_parts_value[:3] == ("crate", "part", "prom")
                and (
                    len(path_parts_value) == 3
                    or path_parts_value[3] in PROM_FORBIDDEN_PATHS
                )
            ),
            None,
        )

        if orchestra_path is not None:
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    orchestra_path,
                    "CPO001",
                    "complex must not import Orchestra",
                )
            )

        if repository_path is not None:
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    repository_path,
                    "CPO002",
                    "complex must not import repository ports or operations",
                )
            )

        if prom_path is not None:
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    prom_path,
                    "CPO003",
                    "complex must not import Prom ports, operations, or tasks",
                )
            )

    for call in descendants(tree.root_node):
        if call.type != "call_expression":
            continue

        function = call.child_by_field_name("function")

        if function is None or function.type != "field_expression":
            continue

        field = function.child_by_field_name("field")

        if field is None or text(source, field) not in {"run_on", "step_on", "proxy_on"}:
            continue

        diagnostics.append(
            diagnostic(
                path,
                root,
                call,
                "CPO004",
                "complex must not dispatch operations",
            )
        )

    return diagnostics


def check_root(root: Path) -> list[str]:
    return [
        diagnostic
        for path in sorted((root / "src" / "complex").rglob("*.rs"))
        for diagnostic in check_file(path, root)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        complex_dir = root / "src" / "complex"
        complex_dir.mkdir(parents=True)
        fixture = complex_dir / "fixture.rs"
        fixture.write_text(
            "//! `.run_on(` and crate::part::repo are documented here.\n"
            "// poprako_orchestra and crate::part::prom::oper are comments.\n"
            "const MESSAGE: &str = \".step_on( crate::part::repo poprako_orchestra\";\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use poprako_orchestra::Proxy;\n"
            "    fn ignored() { Op.run_on(repo); }\n"
            "}\n"
            "fn valid() {}\n"
        )

        if check_root(root):
            print("self-test: comments, strings, or test-only code was rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "use poprako_orchestra::OperRun as _;\n"
            "use crate::part::repo::comic::ComicRepo;\n"
            "use crate::part::prom::oper::Defer;\n"
            "async fn invalid(repo: &impl ComicRepo<()>) { let _ = Op.run_on(repo).await; }\n"
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 4:
            print("self-test: complex boundary violations were not fully rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
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
