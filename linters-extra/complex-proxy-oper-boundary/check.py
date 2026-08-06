#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Restrict complex port access to Proxy operations."""

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


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    pending = [node]

    while pending:
        current = pending.pop()

        if current.type == kind:
            found.append(current)

        pending.extend(reversed(current.named_children))

    return found


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def belongs_to(node: tree_sitter.Node, kind: str) -> bool:
    current = node.parent

    while current is not None:
        if current.type == kind:
            return True

        current = current.parent

    return False


def diagnostic(path: Path, root: Path, node: tree_sitter.Node, code: str, message: str) -> str:
    return f"{path.relative_to(root)}:{node.start_point.row + 1}:{node.start_point.column + 1}: {code}: {message}"


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node, "use_declaration"):
        imported = "".join(text(source, declaration).split())

        if "crate::part::repo::oper" in imported or "crate::part::prom::payload" in imported:
            continue

        if "crate::part::repo" in imported or "crate::part::prom" in imported:
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    declaration,
                    "CPO001",
                    "complex may import only operation descriptors and deferred-operation payloads; access ports through Proxy<Oper>",
                )
            )

    for identifier in descendants(tree.root_node, "type_identifier"):
        if belongs_to(identifier, "use_declaration"):
            continue

        name = text(source, identifier)

        if name == "Prom" or name.endswith("Repo"):
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    identifier,
                    "CPO002",
                    "complex must not name direct port traits; use Proxy<Oper>",
                )
            )

    for call in descendants(tree.root_node, "call_expression"):
        function = call.child_by_field_name("function")

        if function is None or function.type != "field_expression":
            continue

        field = function.child_by_field_name("field")

        if field is None or text(source, field) not in {"run", "step"}:
            continue

        diagnostics.append(
            diagnostic(
                path,
                root,
                call,
                "CPO003",
                "complex must dispatch operations with Proxy::exec, not direct run/step",
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
            "use poprako_orchestra::Proxy;\n"
            "use crate::part::repo::oper::comic::GetComicInfo;\n"
            "fn valid<P: Proxy<GetComicInfo<'_>>>(proxy: &mut P) {\n"
            "    let _ = proxy.exec(&GetComicInfo { id: \"id\", incls: &[] });\n"
            "}\n"
        )

        if check_root(root):
            print("self-test: valid Proxy<Oper> fixture was rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "use crate::part::prom::Prom;\n"
            "use crate::part::repo::comic::ComicRepo;\n"
            "fn invalid<R: ComicRepo<()>, P: Prom<()>>(repo: &R, prom: &P) {\n"
            "    let _ = repo.run(todo!());\n"
            "    let _ = prom.step(todo!(), todo!());\n"
            "}\n"
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 6:
            print("self-test: direct port access was not fully rejected", file=sys.stderr)
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
