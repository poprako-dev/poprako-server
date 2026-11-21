#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Keep serde and utoipa wire usage inside the HTTP object module."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys
import tempfile
import unittest

import tree_sitter

sys.path.insert(0, str(Path(__file__).parents[1]))
from analysis_session import parse_rust
from production_source import production_files, production_source


ROOT = Path(__file__).parents[2]
BASELINE = Path(__file__).with_name("baseline.txt")

RESTRICTED = ("data", "model", "shape", "complex", "usecase")
SERDE_ALLOWED_PATHS = (
    Path("src/value"),
    Path("src/util.rs"),
    Path("src/util/patch_input.rs"),
    Path("src/api/http/object"),
)
SERDE_IGNORED_PATHS = (
    Path("src/config.rs"),
    Path("src/config"),
    Path("src/part.rs"),
    Path("src/part"),
    Path("src/part_impl.rs"),
    Path("src/part_impl"),
)
HTTP_INFRASTRUCTURE = (
    Path("src/api/http/result.rs"),
    Path("src/api/http/shared/metric.rs"),
)
API_ONLY_PATHS = (Path("src/value"), Path("src/model"))
SERDE_ROOTS = ("serde", "serde_json", "utoipa")
SERDE_NAMES = (
    "Serialize",
    "Deserialize",
    "Serializer",
    "SerializeStruct",
    "DeserializeOwned",
    "ToSchema",
    "PartialSchema",
)
SERDE_NAME = re.compile(r"\b(?:" + "|".join(SERDE_NAMES) + r")\b")
SERDE_PATH = re.compile(
    r"\buse\s+(?:crate::)?(?:::\s*)?(?:"
    + "|".join(SERDE_ROOTS)
    + r")\b"
)
ATTRIBUTE = re.compile(r"\b(?:" + "|".join(SERDE_ROOTS + SERDE_NAMES) + r")\b")
OBJECT_REFERENCE = re.compile(r"\bapi::http::object\b")


def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def diagnostic(
    path: Path,
    root: Path,
    node: tree_sitter.Node,
    code: str,
    message: str,
) -> str:
    return (
        f"{path.relative_to(root)}:{node.start_point.row + 1}:"
        f"{node.start_point.column + 1}: {code}: {message}"
    )


def under(path: Path, root: Path, parents: tuple[Path, ...]) -> bool:
    return any(path.relative_to(root).is_relative_to(parent) for parent in parents)


def parse(path: Path, root: Path) -> tuple[bytes, tree_sitter.Tree]:
    source = production_source(path, root)

    return source, parse_rust(source)


def walk(tree: tree_sitter.Tree, skip=lambda node: False):
    pending = [tree.root_node]

    while pending:
        node = pending.pop()

        yield node

        if skip(node):
            continue

        pending.extend(reversed(node.named_children))


def leading_attributes(node: tree_sitter.Node) -> list[tree_sitter.Node]:
    parent = node.parent

    if parent is None:
        return []

    index = next(
        (
            index
            for index, sibling in enumerate(parent.children)
            if sibling.start_byte == node.start_byte
            and sibling.end_byte == node.end_byte
        ),
        None,
    )

    if index is None:
        return []

    attributes: list[tree_sitter.Node] = []

    for sibling in reversed(parent.children[:index]):
        if sibling.type == "attribute_item":
            attributes.append(sibling)
            continue

        if not sibling.is_named:
            continue

        break

    attributes.reverse()

    return attributes


def has_wire_attribute(node: tree_sitter.Node, source: bytes) -> bool:
    return any(
        ATTRIBUTE.search(text(source, attribute))
        for attribute in leading_attributes(node)
    )


def serde_definition(
    node: tree_sitter.Node,
    source: bytes,
) -> tree_sitter.Node | None:
    if node.type in {"struct_item", "enum_item", "union_item"}:
        if has_wire_attribute(node, source):
            return node

        return None

    if node.type != "impl_item":
        return None

    trait = node.child_by_field_name("trait")
    body = node.child_by_field_name("body")

    if trait is None or body is None:
        return None

    if not SERDE_NAME.search(text(source, trait)):
        return None

    return node


def definition_name(node: tree_sitter.Node) -> tree_sitter.Node:
    name = node.child_by_field_name("name")

    return name if name is not None else node


def rule_one(path: Path, root: Path) -> list[str]:
    if not under(path, root, tuple(Path("src") / name for name in RESTRICTED)):
        return []

    source, tree = parse(path, root)
    diagnostics: list[str] = []

    def matches(node: tree_sitter.Node) -> bool:
        if node.type == "use_declaration":
            return bool(SERDE_PATH.search(text(source, node)))

        if node.type == "attribute_item":
            return bool(ATTRIBUTE.search(text(source, node)))

        if node.type not in {"identifier", "type_identifier", "scoped_identifier"}:
            return False

        value = text(source, node)

        return value.split("::", 1)[0].strip() in SERDE_ROOTS or value in SERDE_NAMES

    for node in walk(tree, matches):
        if not matches(node):
            continue

        if node.type == "use_declaration":
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    node,
                    "SSC001",
                    "serde, serde_json, and utoipa imports are forbidden "
                    "outside the HTTP wire object module",
                )
            )
        elif node.type == "attribute_item":
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    node,
                    "SSC001",
                    "serde and utoipa derives or attributes are forbidden "
                    "outside the HTTP wire object module",
                )
            )
        elif matches(node):
            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    node,
                    "SSC001",
                    "serde and utoipa paths are forbidden outside the HTTP "
                    "wire object module",
                )
            )

    return diagnostics


def rule_two(path: Path, root: Path) -> list[str]:
    if under(path, root, SERDE_ALLOWED_PATHS):
        return []

    if under(path, root, tuple(Path("src") / name for name in RESTRICTED)):
        return []

    if under(path, root, SERDE_IGNORED_PATHS):
        return []

    if path.relative_to(root) in HTTP_INFRASTRUCTURE:
        return []

    source, tree = parse(path, root)

    return [
        diagnostic(
            path,
            root,
            definition_name(node),
            "SSC002",
            "serde type definitions and manual impls are allowed only in "
            "src/value, src/util.rs, src/util/patch_input.rs, and "
            "src/api/http/object",
        )
        for node in walk(tree)
        if serde_definition(node, source) is not None
    ]


def rule_three(path: Path, root: Path) -> list[str]:
    relative_path = path.relative_to(root)

    if not relative_path.is_relative_to(Path("src/api/http")):
        return []

    if relative_path.is_relative_to(Path("src/api/http/object")):
        return []

    if relative_path in HTTP_INFRASTRUCTURE:
        return []

    source, tree = parse(path, root)

    return [
        diagnostic(
            path,
            root,
            definition_name(node),
            "SSC003",
            "serde wire types under src/api/http must live in "
            "src/api/http/object",
        )
        for node in walk(tree)
        if serde_definition(node, source) is not None
    ]


def declared_types(path: Path, root: Path) -> list[tuple[str, tree_sitter.Node]]:
    source, tree = parse(path, root)
    declarations: list[tuple[str, tree_sitter.Node]] = []

    for node in walk(tree):
        if node.type not in {"struct_item", "enum_item", "type_item"}:
            continue

        if not any(
            child.type == "visibility_modifier" for child in node.children
        ):
            continue

        name = node.child_by_field_name("name")

        if name is not None:
            declarations.append((text(source, name), name))

    return declarations


def referenced_names(
    root: Path,
    files: list[Path],
) -> dict[str, set[Path]]:
    references: dict[str, set[Path]] = {}

    for path in files:
        source, tree = parse(path, root)

        for node in walk(tree):
            if node.type not in {
                "identifier",
                "type_identifier",
                "scoped_identifier",
            }:
                continue

            value = text(source, node)
            name = value.rsplit("::", 1)[-1]

            if name:
                references.setdefault(name, set()).add(path)

    return references


def rule_four(root: Path, files: list[Path]) -> list[str]:
    references = referenced_names(root, files)
    diagnostics: list[str] = []

    for path in files:
        if not under(path, root, API_ONLY_PATHS):
            continue

        for name, node in declared_types(path, root):
            users = references.get(name, set()) - {path}

            if not users:
                continue

            if all(under(user, root, (Path("src/api"),)) for user in users):
                diagnostics.append(
                    diagnostic(
                        path,
                        root,
                        node,
                        "SSC004",
                        f"public type `{name}` is referenced only from "
                        "src/api; move it to src/api/http/object",
                    )
                )

    return diagnostics


def rule_five(root: Path, files: list[Path]) -> list[str]:
    diagnostics: list[str] = []
    http_path = root / "src" / "api" / "http.rs"

    if http_path.is_file():
        source, tree = parse(http_path, root)
        declared = [
            node
            for node in walk(tree)
            if node.type == "mod_item"
            and (name := node.child_by_field_name("name")) is not None
            and text(source, name) == "object"
        ]

        if not declared:
            diagnostics.append(
                f"src/api/http.rs:1:1: SSC005: src/api/http.rs must declare "
                "`mod object;`"
            )

    for path in files:
        if path.relative_to(root).is_relative_to(Path("src/api/http")):
            continue

        source, tree = parse(path, root)

        def matched(node: tree_sitter.Node) -> bool:
            if node.type == "use_declaration":
                return bool(OBJECT_REFERENCE.search(text(source, node)))

            return node.type == "scoped_identifier" and bool(
                OBJECT_REFERENCE.search(text(source, node))
            )

        for node in walk(tree, matched):
            if not matched(node):
                continue

            diagnostics.append(
                diagnostic(
                    path,
                    root,
                    node,
                    "SSC005",
                    "src/api/http/object may only be referenced from "
                    "src/api/http",
                )
            )

    return diagnostics


def check_root(root: Path) -> list[str]:
    root = root.resolve()
    files = production_files(root)
    diagnostics: list[str] = []

    for path in files:
        diagnostics.extend(rule_one(path, root))
        diagnostics.extend(rule_two(path, root))
        diagnostics.extend(rule_three(path, root))

    diagnostics.extend(rule_four(root, files))
    diagnostics.extend(rule_five(root, files))

    return diagnostics


def baseline_entries() -> set[str]:
    if not BASELINE.is_file():
        return set()

    return {
        line.strip()
        for line in BASELINE.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    }


def filter_baseline(diagnostics: list[str]) -> list[str]:
    baseline = baseline_entries()
    kept: list[str] = []

    for item in diagnostics:
        parts = item.split(":", 4)

        if len(parts) >= 4 and f"{parts[0]}:{parts[3].strip()}" in baseline:
            continue

        kept.append(item)

    return kept


def write_baseline(diagnostics: list[str]) -> None:
    entries = sorted(
        {
            f"{item.split(':', 1)[0]}:{item.split(':', 4)[3].strip()}"
            for item in diagnostics
        }
    )
    lines = [
        "# Existing serde/utoipa scope violations accepted during migration.",
        "# Regenerate after each migration phase; never add new entries.",
    ]

    BASELINE.write_text("\n".join([*lines, *entries]) + "\n")


def self_test() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(SerdeScopeTests)

    return (
        0
        if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful()
        else 1
    )


class SerdeScopeTests(unittest.TestCase):
    def diagnostics(self, files: dict[str, str]) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                '[package]\nname = "sample"\nversion = "0.0.0"\n'
            )

            for name, source in files.items():
                path = root / "src" / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)

            return check_root(root)

    def test_restricted_layers_reject_serde_paths(self):
        diagnostics = self.diagnostics(
            {
                "data/foo.rs": "use serde::Serialize;\n",
                "model/foo.rs": (
                    "#[derive(Serialize)]\npub struct Foo { pub id: u8 }\n"
                ),
                "complex/foo.rs": (
                    "#[cfg_attr(feature = \"swagger\", derive(ToSchema))]\n"
                    "pub struct Foo;\n"
                ),
                "usecase/foo.rs": "fn f() -> u8 { serde_json::json!(1).as_u64().unwrap() as u8 }\n",
            }
        )

        self.assertEqual(len(diagnostics), 4, diagnostics)
        self.assertEqual(
            {item.split(": ")[1] for item in diagnostics},
            {"SSC001"},
        )

    def test_allowed_value_module_accepts_serde(self):
        diagnostics = self.diagnostics(
            {
                "value/foo.rs": (
                    "use serde::Serialize;\n"
                    "#[derive(Serialize)]\n"
                    "pub struct Foo { pub id: u8 }\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_test_sources_are_ignored(self):
        diagnostics = self.diagnostics(
            {
                "lib.rs": "pub mod data;\n",
                "data/foo.rs": "pub struct Foo { pub id: u8 }\n",
                "data/tests.rs": (
                    "use serde::Serialize;\n"
                    "#[derive(Serialize)]\n"
                    "struct Fixture { id: u8 }\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_serde_definitions_outside_allowed_paths_are_rejected(self):
        diagnostics = self.diagnostics(
            {
                "extra/foo.rs": (
                    "use serde::Serialize;\n"
                    "#[derive(Serialize)]\n"
                    "pub struct Foo { pub id: u8 }\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)
        self.assertIn("SSC002", diagnostics[0])

    def test_serde_definition_locations_are_allowed_or_ignored(self):
        diagnostics = self.diagnostics(
            {
                "util.rs": (
                    "use serde::Serialize;\n"
                    "#[derive(Serialize)]\n"
                    "pub struct Patch { pub id: u8 }\n"
                ),
                "config/foo.rs": (
                    "#[derive(Serialize)]\n"
                    "pub struct Config { pub id: u8 }\n"
                ),
                "part/foo.rs": (
                    "#[derive(Serialize)]\n"
                    "pub struct Port { pub id: u8 }\n"
                ),
                "part_impl/foo.rs": (
                    "#[derive(Serialize)]\n"
                    "pub struct Adapter { pub id: u8 }\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_http_wire_types_must_live_in_object(self):
        diagnostics = self.diagnostics(
            {
                "api/http/handler/foo.rs": (
                    "use serde::Deserialize;\n"
                    "#[derive(Deserialize)]\n"
                    "pub struct FooBody { pub id: u8 }\n"
                ),
                "api/http/object/foo.rs": (
                    "use serde::Deserialize;\n"
                    "#[derive(Deserialize)]\n"
                    "pub struct FooBody { pub id: u8 }\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 2, diagnostics)
        self.assertTrue(any("SSC003" in item for item in diagnostics))
        self.assertIn("handler/foo.rs", "\n".join(diagnostics))

    def test_http_infrastructure_is_exempt_from_object_placement(self):
        diagnostics = self.diagnostics(
            {
                "api/http/result.rs": (
                    "use serde::Serialize;\n"
                    "#[derive(Serialize)]\n"
                    "pub struct HttpError { pub code: u8 }\n"
                ),
                "api/http/shared/metric.rs": (
                    "use serde::Serialize;\n"
                    "#[derive(Serialize)]\n"
                    "pub struct MetricTotal { pub total: u64 }\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])

    def test_api_only_public_type_is_rejected(self):
        diagnostics = self.diagnostics(
            {
                "value/foo.rs": (
                    "pub struct OnlyApi { pub id: u8 }\n"
                    "pub struct Shared { pub id: u8 }\n"
                ),
                "api/http/handler/foo.rs": (
                    "use crate::value::foo::OnlyApi;\n"
                    "pub fn f(_: OnlyApi) {}\n"
                ),
                "usecase/foo.rs": (
                    "use crate::value::foo::Shared;\n"
                    "pub fn f(_: Shared) {}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)
        self.assertIn("SSC004", diagnostics[0])
        self.assertIn("OnlyApi", diagnostics[0])

    def test_object_reference_outside_http_is_rejected(self):
        diagnostics = self.diagnostics(
            {
                "api/http.rs": "pub mod object;\n",
                "api/http/object.rs": "pub mod foo;\n",
                "api/http/object/foo.rs": "pub struct Foo;\n",
                "usecase/foo.rs": (
                    "use crate::api::http::object::foo::Foo;\n"
                    "pub fn f(_: Foo) {}\n"
                ),
            }
        )

        self.assertEqual(len(diagnostics), 1, diagnostics)
        self.assertIn("SSC005", diagnostics[0])

    def test_private_object_module_and_internal_reference_are_allowed(self):
        diagnostics = self.diagnostics(
            {
                "api/http.rs": "mod object;\n",
                "api/http/object.rs": "pub mod foo;\n",
                "api/http/object/foo.rs": "pub struct Foo;\n",
                "api/http/handler/foo.rs": (
                    "use crate::api::http::object::foo::Foo;\n"
                    "pub fn f(_: Foo) {}\n"
                ),
            }
        )

        self.assertEqual(diagnostics, [])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--write-baseline", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    diagnostics = check_root(args.root.resolve())

    if args.write_baseline:
        write_baseline(diagnostics)

    diagnostics = filter_baseline(diagnostics)

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
