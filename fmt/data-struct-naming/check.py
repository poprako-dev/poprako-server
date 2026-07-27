#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce domain-qualified Params, Payload, and serde Val data type names."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


DEFAULT_ROOT = Path(__file__).parents[2]
LAYER = "data"
ROLE_SUFFIXES = ("Params", "Payload", "Val")
ACTION_PREFIXES = (
    "Archive",
    "Create",
    "Export",
    "Import",
    "Join",
    "List",
    "Login",
    "Mark",
    "Register",
    "Reserve",
    "Save",
    "Update",
)
DECLARATION_KINDS = ("struct_item", "enum_item", "type_item")
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def rust_files(root: Path) -> list[Path]:
    return sorted((root / "src" / LAYER).rglob("*.rs"))


def pascal_name(module: str) -> str:
    module = module.removesuffix("_port")

    return "".join(part[:1].upper() + part[1:] for part in module.split("_"))


def domain_names(root: Path) -> set[str]:
    modules = {
        path.stem
        for layer in ("model", "data")
        for path in (root / "src" / layer).glob("*.rs")
    }

    return {pascal_name(module) for module in modules}


def descendants(node: tree_sitter.Node, kinds: tuple[str, ...]) -> list[tree_sitter.Node]:
    found: list[tree_sitter.Node] = []
    nodes = [node]

    while nodes:
        current = nodes.pop()

        if current.type in kinds:
            found.append(current)

        nodes.extend(reversed(current.named_children))

    return found


def is_public(declaration: tree_sitter.Node, name: tree_sitter.Node, source: bytes) -> bool:
    prefix = source[declaration.start_byte : name.start_byte].lstrip()

    return prefix.startswith(b"pub")


def check_file(path: Path, root: Path, domains: set[str]) -> list[str]:
    source = production_source(path, root)
    tree = PARSER.parse(source)
    source_domain = pascal_name(path.stem)
    diagnostics: list[str] = []

    for declaration in descendants(tree.root_node, DECLARATION_KINDS):
        name = declaration.child_by_field_name("name")

        if name is None or not is_public(declaration, name, source):
            continue

        type_name = source[name.start_byte : name.end_byte].decode()
        location = f"{path.relative_to(root)}:{name.start_point.row + 1}"

        targets_other_domain = type_name.startswith(ACTION_PREFIXES) and any(
            domain in type_name for domain in domains
        )

        if source_domain not in type_name and not targets_other_domain:
            diagnostics.append(
                f"{location}: public data type {type_name} must contain its domain or an explicit action target",
            )

        if not type_name.endswith(ROLE_SUFFIXES):
            diagnostics.append(
                f"{location}: public data type {type_name} must end with Params, Payload, or Val",
            )

        if "Form" in type_name:
            diagnostics.append(
                f"{location}: public data type {type_name} must not use Form",
            )

    return diagnostics


def check_root(root: Path) -> list[str]:
    domains = domain_names(root)

    return [
        diagnostic
        for path in rust_files(root)
        for diagnostic in check_file(path, root, domains)
    ]


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        data = root / "src" / "data"
        data.mkdir(parents=True)
        (data / "team.rs").write_text(
            "pub struct CreateTeamParams;\n"
            "pub struct CreateTeamPayload;\n"
            "pub struct TeamInfoVal;\n"
            "struct InternalHelper;\n",
        )

        if check_root(root):
            print("self-test: valid data fixture was rejected", file=sys.stderr)
            return 1

        (data / "team.rs").write_text(
            "pub struct CreateParams;\n"
            "pub struct TeamCreateData;\n"
            "pub struct TeamFormParams;\n",
        )
        diagnostics = check_root(root)

        if len(diagnostics) != 3:
            print("self-test: invalid data fixture was not fully rejected", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
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
