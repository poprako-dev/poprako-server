#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Require qualified module aliases for public use-case operations."""

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

    if relative == Path("usecase.rs"):
        return ()

    parts = list(relative.parts[1:])
    parts[-1] = Path(parts[-1]).stem

    return tuple(parts)


def rust_files(root: Path) -> list[RustFile]:
    usecase_dir = root / "src" / "usecase"
    paths = production_files(root, "src/usecase") if usecase_dir.is_dir() else []
    root_module = root / "src" / "usecase.rs"

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


def check_root(root: Path) -> list[str]:
    files = rust_files(root)
    exported_modules = public_modules(files)
    functions = {("usecase",) + module + (name,)
                 for (module, name) in public_free_functions(files)
                 if module in exported_modules and "view" not in module}
    behavior_modules = {path[:-1] for path in functions}
    diagnostics = []
    alias_pattern = r"[a-z][a-z0-9]*(?:_[a-z0-9]+)*_usecase"
    for path in production_files(root):
        source = production_source(path, root)
        file = RustFile(path, source_module(path, root), source, PARSER.parse(source))
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
                    diagnostics.append(diagnostic(root, file, leaf, "UCA001",
                        "import the usecase behavior module as `*_usecase`, not its free functions"))
                elif target in behavior_modules and re.fullmatch(alias_pattern, alias) is None:
                    diagnostics.append(diagnostic(root, file, leaf, "UCA002",
                        "usecase behavior modules require a snake_case `*_usecase` alias"))
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
            diagnostics.append(diagnostic(root, file, node, "UCA003",
                "reference usecase free functions through an imported `*_usecase` module alias"))
    return sorted(diagnostics)


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        usecase_dir = root / "src" / "usecase"
        usecase_dir.mkdir(parents=True)
        (root / "src" / "usecase.rs").write_text(
            "pub mod user; mod internal; mod stage;\n",
        )
        (usecase_dir / "user.rs").write_text(
            "pub fn run<T>() { helper(); run::<()>(); }\n"
            "fn helper() {}\n"
            "pub mod delete { pub async fn apply<T>() {} }\n"
            "pub mod view { pub fn render() {} }\n"
            "mod code { pub fn check() {} }\n"
            "pub struct UserData { value: u32 }\n"
            "impl UserData { pub fn new() -> Self { todo!() } }\n",
        )
        (usecase_dir / "internal.rs").write_text("pub fn helper() {}\n")
        (usecase_dir / "stage.rs").write_text("pub fn check() {}\n")
        fixture = root / "src" / "lib.rs"
        valid = (
            "use crate::usecase::{user as user_usecase, user::{delete as user_delete_usecase, UserData}};\n"
            "use crate::usecase::user::view::render;\n"
            "use crate::usecase::user::code::check;\n"
            "use crate::usecase::internal::helper;\n"
            "use crate::usecase::stage;\n"
            "fn run() { user_usecase::run::<()>(); user_delete_usecase::apply::<()>(); helper(); }\n"
            "fn reference() { let _ = user_usecase::run::<()>; stage::check(); }\n"
            "#[cfg(test)] mod tests { use crate::usecase::user::run; }\n"
            "mod local { use crate::usecase::user::delete as action_usecase; fn f() { action_usecase::apply::<()>(); } }\n"
        )
        fixture.write_text(valid)
        (usecase_dir / "tests.rs").write_text("use crate::usecase::user::run;\n")
        diagnostics = check_root(root)
        if diagnostics:
            print("self-test: valid module aliases rejected", *diagnostics, sep="\n", file=sys.stderr)
            return 1
        cases = [
            ("use crate::usecase::user::run;", "UCA001"),
            ("use crate::usecase::{user::{run as action}};", "UCA001"),
            ("use crate::usecase::user::*;", "UCA001"),
            ("use crate::usecase::{user::{*}};", "UCA001"),
            ("use crate::usecase::user;", "UCA002"),
            ("use crate::usecase::user as UserUsecase;", "UCA002"),
            ("fn f() { crate::usecase::user::run::<()>(); }", "UCA003"),
            ("use crate::usecase; fn f() { usecase::user::run::<()>(); }", "UCA003"),
            ("use crate::usecase as u; fn f() { u::user::run::<()>(); }", "UCA003"),
            ("use crate::usecase::user as user_usecase; fn f() { user_usecase::delete::apply::<()>(); }", "UCA003"),
            ("use crate::usecase; fn f() { let _ = usecase::user::run::<()>; }", "UCA003"),
            ("mod nested { use super::usecase::user::run; }", "UCA001"),
        ]
        for sample, code in cases:
            fixture.write_text(sample)
            diagnostics = check_root(root)
            if len(diagnostics) != 1 or code not in diagnostics[0]:
                print(f"self-test: expected {code} for {sample!r}", *diagnostics, sep="\n", file=sys.stderr)
                return 1

        # Nested imports must not overwrite another lexical scope's binding.
        fixture.write_text(
            "use crate::usecase::user as action_usecase;\n"
            "fn outer() { action_usecase::run::<()>(); }\n"
            "mod nested { use crate::usecase as action_usecase;\n"
            "fn inner() { action_usecase::user::run::<()>(); } }\n",
        )
        diagnostics = check_root(root)
        if len(diagnostics) != 1 or "UCA003" not in diagnostics[0]:
            print("self-test: lexical module aliases resolved incorrectly", *diagnostics, sep="\n", file=sys.stderr)
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
