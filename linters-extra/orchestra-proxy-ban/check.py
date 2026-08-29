#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Ban Orchestra operation proxies from Rust source."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_files, production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
PROXY_NAMES = {"Proxy", "OperProxy", "PromProxy"}
PROXY_MACROS = {"proxy", "run_proxy", "step_proxy"}


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


def is_proxy_name(name: str) -> bool:
    return name in PROXY_NAMES or name.endswith("RepoProxy")


def is_drive_proxy_attribute(node: tree_sitter.Node, source: bytes) -> bool:
    if node.type != "attribute":
        return False

    named_children = node.named_children

    if not named_children or text(source, named_children[0]) != "drive":
        return False

    arguments = next(
        (child for child in named_children[1:] if child.type == "token_tree"),
        None,
    )

    if arguments is None:
        return False

    children = arguments.children

    for index, child in enumerate(children):
        if child.type != "identifier" or text(source, child) != "proxy":
            continue

        following = index + 1

        while following < len(children) and children[following].type in {
            "line_comment",
            "block_comment",
        }:
            following += 1

        if following < len(children) and children[following].type == "=":
            return True

    return False


def diagnostic(path: Path, root: Path, node: tree_sitter.Node) -> str:
    return (
        f"{path.relative_to(root)}:{node.start_point.row + 1}:"
        f"{node.start_point.column + 1}: PRX001: "
        "Orchestra operation proxies are forbidden"
    )


def check_file(path: Path, root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    diagnostics: list[str] = []

    for node in descendants(tree.root_node):
        if node.type in {"identifier", "type_identifier", "field_identifier"}:
            if is_proxy_name(text(source, node)):
                diagnostics.append(diagnostic(path, root, node))
            continue

        if node.type == "macro_invocation":
            macro = node.child_by_field_name("macro")

            if macro is not None and text(source, macro) in PROXY_MACROS:
                diagnostics.append(diagnostic(path, root, macro))
            continue

        if node.type == "call_expression":
            function = node.child_by_field_name("function")

            if function is None or function.type != "field_expression":
                continue

            field = function.child_by_field_name("field")

            if field is not None and text(source, field) == "proxy_on":
                diagnostics.append(diagnostic(path, root, node))
            continue

        if node.type == "attribute" and is_drive_proxy_attribute(node, source):
            diagnostics.append(diagnostic(path, root, node))

    return diagnostics


def check_root(root: Path) -> list[str]:
    return [
        diagnostic
        for path in production_files(root)
        for diagnostic in check_file(path, root)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"
        fixture.write_text(
            "// Proxy proxy! and .proxy_on( are documented here.\n"
            "const MESSAGE: &str = \"Proxy proxy! .proxy_on( #[drive(proxy = x)]\";\n"
            "#[drive(note = \"proxy = Proxy\")]\n"
            "struct NuclProxy;\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use poprako_orchestra::{OperProxy, Proxy};\n"
            "    fn ignored() { proxy! {}; value.proxy_on(); }\n"
            "}\n"
        )

        if check_root(root):
            print("self-test: comments, strings, or test-only code was rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "use poprako_orchestra::{OperProxy, Proxy};\n"
            "type P = ComicRepoProxy;\n"
            "fn invalid() { proxy! {}; op.proxy_on(todo!()); }\n"
            "#[drive(run(Op), proxy = PromProxy)] trait Repo {}\n"
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 7:
            print("self-test: proxy forms were not fully rejected", file=sys.stderr)
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
