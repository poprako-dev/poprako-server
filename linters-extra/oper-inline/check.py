#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require every poprako_orchestra Oper construction to be inline."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_files, production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def descendants(node: tree_sitter.Node, kind: str) -> list[tree_sitter.Node]:
    nodes = [node]
    found: list[tree_sitter.Node] = []

    while nodes:
        current = nodes.pop()

        if current.type == kind:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def oper_names(paths: list[Path], root: Path) -> set[str]:
    names: set[str] = set()

    for path in paths:
        source = production_source(path, root)
        tree = PARSER.parse(source)

        for item in descendants(tree.root_node, "impl_item"):
            item_text = text(source, item)
            marker = "Oper for "

            if marker not in item_text:
                continue

            suffix = item_text.split(marker, 1)[1].lstrip()
            name = suffix.split("<", 1)[0].split(" ", 1)[0].split("{", 1)[0]

            if name[:1].isupper():
                names.add(name)

        for item_kind in {"struct_item", "enum_item"}:
            for item in descendants(tree.root_node, item_kind):
                sibling = item.prev_named_sibling
                derives_oper = False

                while sibling is not None and sibling.type in {
                    "attribute_item",
                    "block_comment",
                    "line_comment",
                }:
                    if sibling.type == "attribute_item" and re.search(
                        r"#\s*\[\s*derive\s*\([^\]]*\bOper\b",
                        text(source, sibling),
                    ):
                        derives_oper = True

                    sibling = sibling.prev_named_sibling

                if not derives_oper:
                    continue

                name = item.child_by_field_name("name")

                if name is not None:
                    names.add(text(source, name))

    return names


def constructor_name(source: bytes, value: tree_sitter.Node) -> str | None:
    if value.type == "struct_expression":
        name = value.child_by_field_name("name")

        if name is not None:
            segments = text(source, name).split("::")

            if len(segments) >= 2:
                return segments[-2]

            return segments[-1]

    if value.type in {"identifier", "scoped_identifier"}:
        return text(source, value).split("::")[-1]

    if value.type == "call_expression":
        function = value.child_by_field_name("function")

        if function is not None:
            segments = text(source, function).split("::")

            if len(segments) >= 2 and segments[-1] == "new":
                return segments[-2]

    return None


def check_file(path: Path, names: set[str], root: Path) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    errors: list[str] = []

    for declaration in descendants(tree.root_node, "let_declaration"):
        value = declaration.child_by_field_name("value")

        if value is None:
            continue

        name = constructor_name(source, value)

        if name not in names:
            continue

        errors.append(
            f"{path.relative_to(root)}:{declaration.start_point.row + 1}: "
            f"OPR001: construct {name} directly as a run_on or step_on receiver",
        )

    imported_traits: dict[str, set[str]] = {
        "Run": set(),
        "Step": set(),
    }
    available_traits: set[str] = set()

    for declaration in descendants(tree.root_node, "use_declaration"):
        declaration_text = text(source, declaration)

        if "poprako_orchestra" not in declaration_text:
            continue

        imports_wildcard = re.search(r"(?:\{|::)\s*\*", declaration_text)

        for trait_name in imported_traits:
            if not imports_wildcard and not re.search(
                rf"\b{trait_name}\b",
                declaration_text,
            ):
                continue

            available_traits.add(trait_name)

            aliases = re.findall(
                rf"\b{trait_name}\s+as\s+([A-Za-z_][A-Za-z0-9_]*)",
                declaration_text,
            )
            imported_traits[trait_name].update(
                alias for alias in aliases if alias != "_"
            )

            if re.search(
                rf"\b{trait_name}\b(?!\s+as\b)",
                declaration_text,
            ):
                imported_traits[trait_name].add(trait_name)

    direct_methods = {
        "run": ("Run", 1, "run_on"),
        "step": ("Step", 2, "step_on"),
    }

    for call in descendants(tree.root_node, "call_expression"):
        function = call.child_by_field_name("function")
        arguments = call.child_by_field_name("arguments")

        if function is None or arguments is None:
            continue

        if function.type == "field_expression":
            field = function.child_by_field_name("field")

            if field is None:
                continue

            method_name = text(source, field)
            direct_method = direct_methods.get(method_name)

            if direct_method is None:
                continue

            trait_name, argument_count, replacement = direct_method

            if trait_name not in available_traits:
                continue

            if len(arguments.named_children) != argument_count:
                continue

            errors.append(
                f"{path.relative_to(root)}:{call.start_point.row + 1}: "
                f"OPR002: call the operation's {replacement} method instead of "
                f"{trait_name}::{method_name}",
            )

            continue

        function_text = text(source, function)

        for method_name, (trait_name, _, replacement) in direct_methods.items():
            local_trait_names = imported_traits[trait_name]
            qualified = f"poprako_orchestra::{trait_name}" in function_text
            trait_paths = local_trait_names | {
                f"poprako_orchestra::{trait_name}",
            }
            ufcs = any(
                re.search(
                    rf"(?:^|\bas\s+|::){re.escape(trait_path)}"
                    rf"(?:::)?(?:<.*>)?>?::{method_name}$",
                    function_text,
                )
                for trait_path in trait_paths
            )

            if (trait_name in available_traits or qualified) and ufcs:
                errors.append(
                    f"{path.relative_to(root)}:{call.start_point.row + 1}: "
                    f"OPR002: call the operation's {replacement} method instead of "
                    f"{trait_name}::{method_name}",
                )

    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source_dir = root / "src"
        source_dir.mkdir()
        fixture = source_dir / "fixture.rs"
        fixture.write_text(
            "use poprako_orchestra::{OperRun as _, OperStep as _};\n"
            "trait Oper {}\n"
            "struct Create;\n"
            "#[derive(Oper)]\n"
            "struct Derived;\n"
            "enum Get { Id { id: String } }\n"
            "impl Oper for Get {}\n"
            "fn valid() {\n"
            "    Create.run_on(repo);\n"
            "    Derived.run_on(repo);\n"
            "    Get::Id { id: String::new() }.step_on(repo, context);\n"
            "}\n",
        )

        if check_file(fixture, oper_names([fixture], root), root):
            print("self-test: inline operations were rejected", file=sys.stderr)
            return 1

        fixture.write_text(
            "use poprako_orchestra::{Run as _, Run as OrchestraRun, Step as _};\n"
            "trait Oper {}\n"
            "struct Create;\n"
            "impl Oper for Create {}\n"
            "#[derive(Oper)]\n"
            "struct Derived;\n"
            "enum Get { Id { id: String } }\n"
            "impl Oper for Get {}\n"
            "fn invalid() {\n"
            "    let create = Create;\n"
            "    create.run_on(repo);\n"
            "    let derived = Derived;\n"
            "    derived.run_on(repo);\n"
            "    let get = Get::Id { id: String::new() };\n"
            "    get.step_on(repo, context);\n"
            "    repo.run(&Create);\n"
            "    repo.step(context, &Create);\n"
            "    OrchestraRun::run(repo, &Create);\n"
            "    <Repo as OrchestraRun<Create>>::run(repo, &Create);\n"
            "    poprako_orchestra::Step::step(repo, context, &Create);\n"
            "}\n",
        )
        diagnostics = check_file(fixture, oper_names([fixture], root), root)

        if len(diagnostics) != 8:
            print("self-test: invalid operation calls were not fully rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    paths = production_files(root)
    names = oper_names(paths, root)

    if args.self_test:
        return self_test()

    errors = [error for path in paths for error in check_file(path, names, root)]

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
