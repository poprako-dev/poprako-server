#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require complex free functions and qualified module-alias references."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

import tree_sitter

sys.path.insert(0, str(Path(__file__).parents[1]))
from analysis_session import parse_rust
from production_source import production_files, production_source


DEFAULT_ROOT = Path(__file__).parents[2]

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
            tree=parse_rust(source),
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


def descendants(node):
    yield node
    for child in node.named_children:
        yield from descendants(child)


def imported_paths(node, source, prefix=()):
    """Expand nested Rust use trees while retaining each binding's alias."""
    if node.type == "use_declaration":
        argument = node.child_by_field_name("argument")
        return imported_paths(argument, source) if argument is not None else []
    if node.type == "scoped_use_list":
        base, items = node.named_children
        return imported_paths(items, source, prefix + tuple(node_text(source, base).split("::")))
    if node.type == "use_list":
        return [item for child in node.named_children for item in imported_paths(child, source, prefix)]
    if node.type == "use_as_clause":
        target, alias = node.named_children
        return [(prefix + tuple(node_text(source, target).split("::")), node_text(source, alias), node)]
    if node.type == "use_wildcard":
        parts = tuple(node_text(source, node).removesuffix("::*").split("::"))
        return [(prefix + (() if parts == ("*",) else parts) + ("*",), "*", node)]
    parts = prefix + tuple(node_text(source, node).split("::"))
    if parts[-1] == "self":
        parts = parts[:-1]
    return [(parts, parts[-1], node)]


def source_module(path, root):
    parts = list(path.relative_to(root / "src").with_suffix("").parts)
    if parts[-1] in {"lib", "main", "mod"}:
        parts.pop()
    return tuple(parts)


def resolve_path(parts, current, bindings):
    if not parts:
        return ()
    if parts[0] == "crate":
        return parts[1:]
    if parts[0] in bindings:
        return bindings[parts[0]] + parts[1:]
    base = list(current)
    segments = list(parts)
    if segments[0] == "self":
        segments.pop(0)
    while segments and segments[0] == "super":
        segments.pop(0)
        if base:
            base.pop()
    return tuple(base + segments)


def diagnostic(root, file, node, code, message):
    return f"{file.path.relative_to(root)}:{node.start_point.row + 1}:{node.start_point.column + 1}: {code}: {message}"


def namespace_diagnostics(root, files):
    diagnostics = []
    for file in files:
        nodes = list(descendants(file.tree.root_node))
        for node in nodes:
            if node.type != "struct_item":
                continue
            name = node_text(file.source, node.child_by_field_name("name"))
            body = node.child_by_field_name("body")
            if not name.endswith("Complex") or (body is not None and body.named_children):
                continue
            module = inline_module_path(file.module, node, file.source)
            impls = [item for item in nodes if item.type == "impl_item"
                     and impl_type_name(item, file.source) == name
                     and inline_module_path(file.module, item, file.source) == module]
            meaningful = any(item.child_by_field_name("trait") is not None
                             or any(child.type == "self_parameter" for child in descendants(item))
                             for item in impls)
            if not meaningful:
                diagnostics.append(diagnostic(root, file, node, "CPX001",
                    f"empty namespace `{name}` must be replaced with module free functions"))
    return diagnostics


def check_root(root: Path) -> list[str]:
    files = rust_files(root)
    exported_modules = public_modules(files)
    functions = {("complex",) + module + (name,)
                 for (module, name) in public_free_functions(files)
                 if module in exported_modules}
    behavior_modules = {path[:-1] for path in functions}
    diagnostics = namespace_diagnostics(root, files)
    alias_pattern = r"[a-z][a-z0-9]*(?:_[a-z0-9]+)*_complex"
    for path in production_files(root):
        source = production_source(path, root)
        file = RustFile(path, source_module(path, root), source, parse_rust(source))
        nodes = list(descendants(file.tree.root_node))
        scopes = {}

        def bindings_for(node):
            chain = []
            current = node
            while current is not None:
                chain.append(current)
                current = current.parent
            bindings = {}
            for scope in reversed(chain):
                bindings.update(scopes.get(scope.id, {}))
            return bindings

        for node in nodes:
            if node.type != "use_declaration":
                continue
            current = inline_module_path(file.module, node, source)
            bindings = bindings_for(node)
            for parts, alias, leaf in imported_paths(node, source):
                target = resolve_path(parts, current, bindings)
                scopes.setdefault(node.parent.id, {})[alias] = target
                if target in functions or (target[-1:] == ("*",) and target[:-1] in behavior_modules):
                    diagnostics.append(diagnostic(root, file, leaf, "CPX002",
                        "import the complex behavior module as `*_complex`, not its free functions"))
                elif target in behavior_modules and re.fullmatch(alias_pattern, alias) is None:
                    diagnostics.append(diagnostic(root, file, leaf, "CPX003",
                        "complex behavior modules require a snake_case `*_complex` alias"))
        for node in nodes:
            if node.type != "scoped_identifier" or (node.parent is not None and node.parent.type == "scoped_identifier"):
                continue
            ancestors = []
            current_node = node.parent
            while current_node is not None:
                ancestors.append(current_node.type)
                current_node = current_node.parent
            if "use_declaration" in ancestors:
                continue
            parts = tuple(node_text(source, node).split("::"))
            current = inline_module_path(file.module, node, source)
            target = resolve_path(parts, current, bindings_for(node))
            if target not in functions:
                continue
            if len(parts) == 2 and re.fullmatch(alias_pattern, parts[0]):
                continue
            diagnostics.append(diagnostic(root, file, node, "CPX004",
                "reference complex free functions through an imported `*_complex` module alias"))
    return sorted(diagnostics)


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        complex_dir = root / "src" / "complex"
        complex_dir.mkdir(parents=True)
        (root / "src" / "complex.rs").write_text("pub mod team; mod private;\n")
        (complex_dir / "team.rs").write_text(
            "pub fn run() { helper(); run(); }\n"
            "fn helper() {}\n"
            "pub mod perm { pub fn check() {} }\n"
            "pub struct DataComplex { value: u32 }\n"
            "impl DataComplex { pub fn new() -> Self { todo!() } }\n"
            "pub struct MarkerComplex;\n"
            "impl SomeTrait for MarkerComplex {}\n"
            "pub struct ReceiverComplex;\n"
            "impl ReceiverComplex { pub fn run(&self) {} }\n"
            "#[cfg(test)] mod tests { pub struct IgnoredComplex; }\n",
        )
        (complex_dir / "private.rs").write_text("pub fn helper() {}\n")
        fixture = root / "src" / "lib.rs"
        valid = (
            "use crate::complex::{team as team_complex, team::{perm as team_perm_complex, DataComplex}};\n"
            "use crate::complex::private::helper;\n"
            "fn run() { team_complex::run(); team_perm_complex::check(); helper(); }\n"
            "fn reference() { let _ = team_complex::run; }\n"
            "#[cfg(test)] mod tests { use crate::complex::team::run; }\n"
        )
        fixture.write_text(valid)
        diagnostics = check_root(root)
        if diagnostics:
            print("self-test: valid module interface rejected", *diagnostics, sep="\n", file=sys.stderr)
            return 1
        cases = [
            ("use crate::complex::team::run;", "CPX002"),
            ("use crate::complex::{team::{run as action}};", "CPX002"),
            ("use crate::complex::team::*;", "CPX002"),
            ("use crate::complex::{team::{*}};", "CPX002"),
            ("use crate::complex::team;", "CPX003"),
            ("use crate::complex::team as TeamComplex;", "CPX003"),
            ("fn f() { crate::complex::team::run(); }", "CPX004"),
            ("use crate::complex as c; fn f() { c::team::run(); }", "CPX004"),
            ("use crate::complex::team as team_complex; fn f() { team_complex::perm::check(); }", "CPX004"),
            ("mod nested { use super::complex::team::run; }", "CPX002"),
        ]
        for sample, code in cases:
            fixture.write_text(sample)
            diagnostics = check_root(root)
            if len(diagnostics) != 1 or code not in diagnostics[0]:
                print(f"self-test: expected {code} for {sample!r}", *diagnostics, sep="\n", file=sys.stderr)
                return 1
        fixture.write_text(valid)
        with (complex_dir / "team.rs").open("a") as file:
            file.write("pub struct TeamComplex; impl TeamComplex { pub fn run() {} }\n"
                       "pub struct EmptyComplex {}\n"
                       "pub struct TupleComplex();\n")
        diagnostics = check_root(root)
        if len(diagnostics) != 3 or any("CPX001" not in item for item in diagnostics):
            print("self-test: namespace structs were not rejected", *diagnostics, sep="\n", file=sys.stderr)
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
