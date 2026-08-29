#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Forbid free-function exports from the complex module interface."""

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
from production_source import production_files, production_source


DEFAULT_ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
COMPLEX_TYPE = re.compile(r"(?:^|::)([A-Za-z_][A-Za-z0-9_]*)\s*(?:<.*>)?$")


@dataclass(frozen=True)
class RustFile:
    path: Path
    module: tuple[str, ...]
    source: bytes
    tree: tree_sitter.Tree


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def is_public(node: tree_sitter.Node) -> bool:
    return any(child.type == "visibility_modifier" for child in node.children)


def file_module(path: Path, root: Path) -> tuple[str, ...]:
    relative = path.relative_to(root / "src")

    if relative == Path("complex.rs"):
        return ()

    parts = list(relative.parts[1:])
    parts[-1] = Path(parts[-1]).stem

    return tuple(parts)


def rust_files(root: Path) -> list[RustFile]:
    complex_dir = root / "src" / "complex"
    paths = production_files(root, "src/complex") if complex_dir.is_dir() else []
    root_module = root / "src" / "complex.rs"

    if root_module.is_file():
        paths.insert(0, root_module)

    return [
        RustFile(
            path=path,
            module=file_module(path, root),
            source=(source := production_source(path, root)),
            tree=PARSER.parse(source),
        )
        for path in paths
    ]


def inline_module_path(
    base: tuple[str, ...],
    node: tree_sitter.Node,
    source: bytes,
) -> tuple[str, ...]:
    names: list[str] = []
    current = node.parent

    while current is not None:
        if current.type == "mod_item" and current.child_by_field_name("body") is not None:
            name = current.child_by_field_name("name")

            if name is not None:
                names.append(node_text(source, name))

        current = current.parent

    return base + tuple(reversed(names))


def public_modules(files: list[RustFile]) -> set[tuple[str, ...]]:
    declarations: list[tuple[tuple[str, ...], tuple[str, ...], bool]] = []

    for rust_file in files:
        pending = [rust_file.tree.root_node]

        while pending:
            node = pending.pop()

            if node.type == "mod_item":
                name = node.child_by_field_name("name")

                if name is not None:
                    parent = inline_module_path(
                        rust_file.module, node, rust_file.source,
                    )
                    child = parent + (node_text(rust_file.source, name),)
                    declarations.append((parent, child, is_public(node)))

            pending.extend(reversed(node.named_children))

    exported = {()}
    changed = True

    while changed:
        changed = False

        for parent, child, visible in declarations:
            if visible and parent in exported and child not in exported:
                exported.add(child)
                changed = True

    return exported


def enclosing_impl(node: tree_sitter.Node) -> tree_sitter.Node | None:
    current = node.parent

    while current is not None:
        if current.type == "impl_item":
            return current

        if current.type in {"function_item", "trait_item"}:
            return None

        current = current.parent

    return None


def impl_type_name(node: tree_sitter.Node, source: bytes) -> str | None:
    target = node.child_by_field_name("type")

    if target is None:
        return None

    match = COMPLEX_TYPE.search(node_text(source, target))

    return match.group(1) if match is not None else None


def public_free_functions(
    files: list[RustFile],
) -> dict[tuple[tuple[str, ...], str], tuple[RustFile, tree_sitter.Node]]:
    functions = {}

    for rust_file in files:
        pending = [rust_file.tree.root_node]

        while pending:
            node = pending.pop()

            if node.type == "function_item" and is_public(node) and enclosing_impl(node) is None:
                name = node.child_by_field_name("name")

                if name is not None:
                    module = inline_module_path(
                        rust_file.module, node, rust_file.source,
                    )
                    functions[(module, node_text(rust_file.source, name))] = (
                        rust_file,
                        node,
                    )

            pending.extend(reversed(node.named_children))

    return functions


def normalize_path(
    current: tuple[str, ...],
    raw_segments: list[str],
) -> tuple[str, ...]:
    segments = list(raw_segments)

    if segments and segments[0] == "crate":
        segments.pop(0)

        if segments and segments[0] == "complex":
            segments.pop(0)

        return tuple(segments)

    base = list(current)

    if segments and segments[0] == "self":
        segments.pop(0)

    while segments and segments[0] == "super":
        segments.pop(0)

        if base:
            base.pop()

    return tuple(base + segments)


def exported_use_paths(
    node: tree_sitter.Node,
    source: bytes,
    current: tuple[str, ...],
) -> list[tuple[str, ...]]:
    argument = node.child_by_field_name("argument")

    if argument is None:
        return []

    text = node_text(source, argument).strip()
    text = re.sub(r"\s+as\s+[A-Za-z_][A-Za-z0-9_]*$", "", text)

    if "{" not in text:
        return [normalize_path(current, text.split("::"))]

    prefix, remainder = text.split("{", 1)
    prefix_segments = [segment for segment in prefix.rstrip(":").split("::") if segment]
    base = normalize_path(current, prefix_segments)
    names = remainder.rsplit("}", 1)[0]

    return [
        base + (re.sub(r"\s+as\s+.*$", "", name.strip()),)
        for name in names.split(",")
        if name.strip()
    ]


def check_root(root: Path) -> list[str]:
    files = rust_files(root)
    exported_modules = public_modules(files)
    free_functions = public_free_functions(files)
    violations: dict[tuple[Path, int, str], str] = {}

    for (module, function_name), (rust_file, node) in free_functions.items():
        if module in exported_modules:
            key = (rust_file.path, node.start_byte, function_name)
            violations[key] = diagnostic_for_root(
                root, rust_file, node, function_name,
            )

    for rust_file in files:
        pending = [rust_file.tree.root_node]

        while pending:
            node = pending.pop()
            module = inline_module_path(
                rust_file.module, node, rust_file.source,
            )

            if node.type == "use_declaration" and is_public(node) and module in exported_modules:
                for path in exported_use_paths(node, rust_file.source, module):
                    if len(path) < 1:
                        continue

                    target = free_functions.get((path[:-1], path[-1]))

                    if target is not None:
                        key = (rust_file.path, node.start_byte, path[-1])
                        violations[key] = diagnostic_for_root(
                            root, rust_file, node, path[-1],
                        )

            if node.type == "function_item" and is_public(node) and module in exported_modules:
                impl = enclosing_impl(node)

                if impl is not None:
                    type_name = impl_type_name(impl, rust_file.source)

                    if type_name is None or not type_name.endswith("Complex"):
                        name = node.child_by_field_name("name")
                        function_name = node_text(rust_file.source, name) if name is not None else "<unknown>"
                        key = (
                            rust_file.path,
                            node.start_byte,
                            function_name,
                        )
                        violations[key] = diagnostic_for_root(
                            root, rust_file, node, function_name,
                        )

            pending.extend(reversed(node.named_children))

    return [violations[key] for key in sorted(violations)]


def diagnostic_for_root(
    root: Path,
    rust_file: RustFile,
    node: tree_sitter.Node,
    function_name: str,
) -> str:
    line = node.start_point.row + 1
    column = node.start_point.column + 1

    return (
        f"{rust_file.path.relative_to(root)}:{line}:{column}: CPX001: "
        f"complex module exports function `{function_name}` outside a `*Complex` impl"
    )


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        complex_dir = root / "src" / "complex"
        sample_dir = complex_dir / "sample"
        sample_dir.mkdir(parents=True)

        (root / "src" / "complex.rs").write_text("pub mod sample;\nmod util;\n")
        (complex_dir / "sample.rs").write_text(
            "mod private;\n"
            "pub mod public;\n"
            "pub use private::reexported_free;\n"
            "pub struct SampleComplex;\n"
            "impl SampleComplex { pub fn allowed() {} }\n",
        )
        (sample_dir / "private.rs").write_text(
            "pub fn internal_free() {}\n"
            "pub fn reexported_free() {}\n",
        )
        (sample_dir / "public.rs").write_text("pub fn exported_free() {}\n")
        (complex_dir / "util.rs").write_text("pub fn internal_helper() {}\n")

        diagnostics = check_root(root)

        if len(diagnostics) != 2:
            print("self-test: expected two exported-function diagnostics", file=sys.stderr)
            print(f"got {len(diagnostics)} diagnostics", file=sys.stderr)

            for item in diagnostics:
                print(f"  {item}", file=sys.stderr)

            return 1

    print("self-test passed")

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    diagnostics = check_root(args.root.resolve())

    for item in diagnostics:
        print(item)

    return 1 if diagnostics else 0


if __name__ == "__main__":
    raise SystemExit(main())
