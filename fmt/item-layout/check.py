#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Enforce declaration and helper ordering in hand-written Rust source."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust


DEFAULT_ROOT = Path(__file__).parents[2]
GENERATED_SCHEMA = Path("src/part_impl/repo/rdb_impl/schema.rs")
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))


def node_text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def declaration_name(node: tree_sitter.Node, source: bytes) -> str | None:
    name = node.child_by_field_name("name")

    return node_text(source, name) if name is not None else None


def impl_type_name(node: tree_sitter.Node, source: bytes) -> str | None:
    type_node = node.child_by_field_name("type")

    if type_node is None:
        return None

    identifiers: list[tree_sitter.Node] = []
    pending = [type_node]

    while pending:
        current = pending.pop()

        if current.type == "type_identifier":
            identifiers.append(current)

        pending.extend(reversed(current.named_children))

    return node_text(source, identifiers[-1]) if identifiers else None


def is_inherent_impl(node: tree_sitter.Node) -> bool:
    return node.child_by_field_name("trait") is None


def is_public(function: tree_sitter.Node, source: bytes) -> bool:
    name = function.child_by_field_name("name")

    if name is None:
        return False

    return source[function.start_byte : name.start_byte].lstrip().startswith(b"pub")


def is_test_only(node: tree_sitter.Node, source: bytes) -> bool:
    current: tree_sitter.Node | None = node

    while current is not None:
        prefix = source[current.start_byte : current.end_byte]

        if current.type == "attribute_item" and b"cfg(test)" in prefix.replace(b" ", b""):
            return True

        previous = current.prev_named_sibling

        while previous is not None and previous.type == "attribute_item":
            attribute = node_text(source, previous).replace(" ", "")

            if attribute.startswith("#[cfg(test)]"):
                return True

            previous = previous.prev_named_sibling

        current = current.parent

    return False


def diagnostics_for_struct_order(
    container: tree_sitter.Node,
    path: Path,
    root: Path,
    source: bytes,
) -> list[str]:
    declarations = [
        child for child in container_items(container, source) if not is_test_only(child, source)
    ]
    diagnostics: list[str] = []

    for index, declaration in enumerate(declarations):
        if declaration.type != "struct_item":
            continue

        struct_name = declaration_name(declaration, source)

        if struct_name is None:
            continue

        matching = [
            candidate
            for candidate in declarations[index + 1 :]
            if candidate.type == "impl_item" and impl_type_name(candidate, source) == struct_name
        ]

        if not matching:
            continue

        adjacent = declarations[index + 1 : index + 1 + len(matching)]

        if adjacent != matching:
            offending = next(
                candidate
                for candidate in matching
                if candidate not in adjacent or adjacent.index(candidate) != matching.index(candidate)
            )
            diagnostics.append(
                f"{path.relative_to(root)}:{offending.start_point.row + 1}: LAYOUT001: "
                f"impl for {struct_name} must immediately follow its struct declaration",
            )

        trait_seen = False

        for implementation in matching:
            if is_inherent_impl(implementation):
                if trait_seen:
                    diagnostics.append(
                        f"{path.relative_to(root)}:{implementation.start_point.row + 1}: LAYOUT002: "
                        f"inherent impl for {struct_name} must precede its trait impls",
                    )
            else:
                trait_seen = True

    return diagnostics


def called_name(call: tree_sitter.Node, source: bytes) -> str | None:
    function = call.child_by_field_name("function")

    if function is None:
        return None

    text = node_text(source, function)
    match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*$", text)

    return match.group(1) if match is not None else None


def calls_in(function: tree_sitter.Node, source: bytes) -> list[tuple[str, int]]:
    calls: list[tuple[str, int]] = []
    pending = list(reversed(function.named_children))

    while pending:
        current = pending.pop()

        if current.type == "function_item":
            continue

        if current.type == "call_expression":
            name = called_name(current, source)

            if name is not None:
                calls.append((name, current.start_byte))

        pending.extend(reversed(current.named_children))

    return calls


def diagnostics_for_functions(
    container: tree_sitter.Node,
    path: Path,
    root: Path,
    source: bytes,
) -> list[str]:
    functions = [
        child
        for child in container.named_children
        if child.type == "function_item" and not is_test_only(child, source)
    ]
    private_functions = [function for function in functions if not is_public(function, source)]
    public_functions = [function for function in functions if is_public(function, source)]
    diagnostics: list[str] = []

    if public_functions and private_functions:
        last_public = max(function.start_byte for function in public_functions)
        first_private = min(function.start_byte for function in private_functions)

        if last_public > first_private:
            first_private_function = next(
                function
                for function in private_functions
                if function.start_byte == first_private
            )
            diagnostics.append(
                f"{path.relative_to(root)}:{first_private_function.start_point.row + 1}: "
                "LAYOUT003: private functions must follow all public functions",
            )

    private_names = {
        declaration_name(function, source): function
        for function in private_functions
        if declaration_name(function, source) is not None
    }
    first_calls: dict[str, int] = {}

    for function in functions:
        for name, offset in calls_in(function, source):
            if name in private_names:
                first_calls[name] = min(first_calls.get(name, offset), offset)

    expected = sorted(
        private_functions,
        key=lambda function: (
            first_calls.get(declaration_name(function, source) or "", sys.maxsize),
            function.start_byte,
        ),
    )

    if private_functions != expected:
        first_wrong = next(
            current
            for current, expected_function in zip(private_functions, expected)
            if current != expected_function
        )
        expected_name = declaration_name(expected[private_functions.index(first_wrong)], source)
        diagnostics.append(
            f"{path.relative_to(root)}:{first_wrong.start_point.row + 1}: LAYOUT004: "
            f"private function {declaration_name(first_wrong, source)} must follow "
            f"first-call order; {expected_name} is called earlier",
        )

    return diagnostics


def check_container(
    container: tree_sitter.Node,
    path: Path,
    root: Path,
    source: bytes,
) -> list[str]:
    diagnostics = diagnostics_for_struct_order(container, path, root, source)
    diagnostics.extend(diagnostics_for_functions(container, path, root, source))

    for child in container.named_children:
        if child.type == "impl_item":
            body = child.child_by_field_name("body")

            if body is not None:
                diagnostics.extend(check_container(body, path, root, source))
        elif child.type == "mod_item":
            body = child.child_by_field_name("body")

            if body is not None:
                diagnostics.extend(check_container(body, path, root, source))

    return diagnostics


def item_start(node: tree_sitter.Node, source: bytes) -> int:
    """Return an item's start together with directly attached docs/attributes."""

    start = node.start_byte
    previous = node.prev_named_sibling

    while previous is not None and previous.type == "attribute_item":
        gap = source[previous.end_byte:start]

        if gap.count(b"\n") > 1:
            break

        start = previous.start_byte
        previous = previous.prev_named_sibling

    line_start = source.rfind(b"\n", 0, start) + 1

    while line_start > 0:
        previous_end = line_start - 1
        previous_start = source.rfind(b"\n", 0, previous_end) + 1
        previous_line = source[previous_start:previous_end].lstrip()

        if not previous_line.startswith((b"//", b"/*", b"*", b"*/")):
            break

        line_start = previous_start

    return line_start


def container_items(
    container: tree_sitter.Node,
    source: bytes,
) -> list[tree_sitter.Node]:
    return [
        child
        for child in container.named_children
        if child.type not in {"attribute_item", "line_comment", "block_comment"}
    ]


def reorder_items(
    source: bytes,
    items: list[tree_sitter.Node],
    ordered: list[tree_sitter.Node],
) -> bytes:
    starts = [item_start(item, source) for item in items]
    chunks = [
        source[start : starts[index + 1] if index + 1 < len(starts) else item.end_byte]
        for index, (item, start) in enumerate(zip(items, starts))
    ]
    chunks_by_item = dict(zip(items, chunks))
    replacement = b"".join(chunks_by_item[item] for item in ordered)

    return source[: starts[0]] + replacement + source[items[-1].end_byte :]


def structural_order(
    container: tree_sitter.Node,
    source: bytes,
) -> tuple[list[tree_sitter.Node], list[tree_sitter.Node]] | None:
    items = container_items(container, source)
    ordered = list(items)

    for struct in items:
        if struct.type != "struct_item" or is_test_only(struct, source):
            continue

        struct_name = declaration_name(struct, source)

        if struct_name is None:
            continue

        implementations = [
            item
            for item in ordered
            if item.type == "impl_item" and impl_type_name(item, source) == struct_name
        ]

        if not implementations:
            continue

        implementations.sort(key=lambda item: (not is_inherent_impl(item), item.start_byte))
        ordered = [item for item in ordered if item not in implementations]
        struct_index = ordered.index(struct)
        ordered[struct_index + 1 : struct_index + 1] = implementations

    return (items, ordered) if items != ordered else None


def function_order(
    container: tree_sitter.Node,
    source: bytes,
) -> tuple[list[tree_sitter.Node], list[tree_sitter.Node]] | None:
    items = container_items(container, source)
    functions = [
        item
        for item in items
        if item.type == "function_item" and not is_test_only(item, source)
    ]

    if len(functions) < 2:
        return None

    private_functions = [function for function in functions if not is_public(function, source)]
    private_names = {
        declaration_name(function, source)
        for function in private_functions
        if declaration_name(function, source) is not None
    }
    first_calls: dict[str, int] = {}

    for function in functions:
        for name, offset in calls_in(function, source):
            if name in private_names:
                first_calls[name] = min(first_calls.get(name, offset), offset)

    ordered_functions = sorted(
        functions,
        key=lambda function: (
            not is_public(function, source),
            first_calls.get(declaration_name(function, source) or "", sys.maxsize),
            function.start_byte,
        ),
    )

    if functions == ordered_functions:
        return None

    first_index = items.index(functions[0])
    ordered = [item for item in items if item not in functions]
    ordered[first_index:first_index] = ordered_functions

    return items, ordered


def nested_containers(container: tree_sitter.Node) -> list[tree_sitter.Node]:
    containers: list[tree_sitter.Node] = []

    for child in container.named_children:
        if child.type not in {"impl_item", "mod_item"}:
            continue

        body = child.child_by_field_name("body")

        if body is not None:
            containers.append(body)
            containers.extend(nested_containers(body))

    return containers


def fix_file(path: Path) -> bool:
    source = path.read_bytes()

    for _ in range(1000):
        tree = PARSER.parse(source)
        containers = [*reversed(nested_containers(tree.root_node)), tree.root_node]
        replacement: bytes | None = None

        for container in containers:
            layout = structural_order(container, source)

            if layout is None:
                layout = function_order(container, source)

            if layout is not None:
                replacement = reorder_items(source, *layout)
                break

        if replacement is None:
            break

        source = replacement
    else:
        raise RuntimeError(f"item-layout fixer did not converge for {path}")

    original = path.read_bytes()

    if source == original:
        return False

    path.write_bytes(source)

    return True


def fix_root(root: Path) -> int:
    changed_paths = [
        path
        for path in sorted((root / "src").rglob("*.rs"))
        if path.relative_to(root) != GENERATED_SCHEMA and fix_file(path)
    ]

    for path in changed_paths:
        print(f"fixed {path.relative_to(root)}")

    return len(changed_paths)


def check_root(root: Path) -> list[str]:
    diagnostics: list[str] = []

    for path in sorted((root / "src").rglob("*.rs")):
        if path.relative_to(root) == GENERATED_SCHEMA:
            continue

        source = path.read_bytes()
        diagnostics.extend(
            check_container(PARSER.parse(source).root_node, path, root, source),
        )

    return diagnostics


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        source = root / "src" / "layout.rs"
        source.parent.mkdir(parents=True)
        source.write_text(
            "pub struct Good;\n"
            "impl Good { pub fn create() {} fn prepare() {} }\n"
            "impl Default for Good { fn default() -> Self { Self } }\n"
            "pub fn run() { prepare(); finish(); }\n"
            "fn prepare() {}\n"
            "fn finish() {}\n",
        )

        if check_root(root):
            print("self-test: valid layout fixture was rejected", file=sys.stderr)
            return 1

        source.write_text(
            "pub struct Wrong;\n"
            "const SEPARATES_WRONG_IMPL: () = ();\n"
            "impl Default for Wrong { fn default() -> Self { Self } }\n"
            "impl Wrong { fn create() {} }\n"
            "/// Runs the private helpers.\n"
            "fn second() {}\n"
            "/// Calls helpers in their required order.\n"
            "pub fn run() { first(); second(); }\n"
            "/// Runs before the second helper.\n"
            "fn first() {}\n",
        )
        diagnostics = check_root(root)
        codes = {match.group(1) for item in diagnostics if (match := re.search(r"(LAYOUT\d{3})", item))}

        if codes != {"LAYOUT001", "LAYOUT002", "LAYOUT003", "LAYOUT004"}:
            print("self-test: layout violations were not fully diagnosed", file=sys.stderr)
            print("\n".join(diagnostics), file=sys.stderr)
            return 1

        fix_root(root)

        if check_root(root):
            print("self-test: fixer did not repair the layout fixture", file=sys.stderr)
            print("\n".join(check_root(root)), file=sys.stderr)
            return 1

        fixed = source.read_text()

        if "/// Calls helpers in their required order.\npub fn run" not in fixed or (
            "/// Runs before the second helper.\nfn first" not in fixed
        ):
            print("self-test: fixer separated a doc comment from its item", file=sys.stderr)
            print(fixed, file=sys.stderr)
            return 1

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--fix", action="store_true")
    parser.add_argument("--self-test", action="store_true")

    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.self_test:
        return self_test()

    if args.fix:
        fix_root(args.root.resolve())

    diagnostics = check_root(args.root.resolve())

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
