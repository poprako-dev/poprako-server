#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["tree-sitter==0.25.0", "tree-sitter-rust==0.23.3"]
# ///
"""Check owned model storage and explicit borrowed domain shapes."""
from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "linters-extra"))
sys.path.insert(0, str(ROOT / "linters"))

spec = importlib.util.spec_from_file_location(
    "shape_dependency_scan", ROOT / "linters-extra/layer-dependency/check.py"
)
dependency = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = dependency
spec.loader.exec_module(dependency)

KINDS = {"struct_item", "enum_item", "type_item", "union_item"}
ROLES = {"read", "write", "shared"}


def walk(node):
    yield node
    for child in node.named_children:
        yield from walk(child)


def storage_types(node):
    if node.type == "type_item":
        value = node.child_by_field_name("type")
        return [value] if value is not None else []
    body = node.child_by_field_name("body")
    if body is None:
        return []
    result = []
    for child in walk(body):
        if child.type == "field_declaration":
            result.append(child.child_by_field_name("type"))
        if child.type == "ordered_field_declaration_list":
            result.extend(
                item for item in child.named_children
                if item.type not in {"attribute_item", "visibility_modifier", "line_comment", "block_comment"}
            )
    return [item for item in result if item is not None]


class Scanner(dependency.Scanner):
    def __init__(self, root):
        super().__init__(root, "poprako_server")
        self.declarations = {}

    def walk(self, node, source, file, scope):
        if dependency.has_direct_test_cfg(node, source):
            return
        if node.type in KINDS:
            name = node.child_by_field_name("name")
            if name is not None:
                name = dependency.text(source, name)
                scope.children.add(name)
                self.declarations[scope.module + (name,)] = (node, source, file, scope)
        super().walk(node, source, file, scope)

    def resolve(self, scope, path, expand=True):
        target = super().resolve(scope, path, expand)
        if target is None and path[:1] in {("std",), ("core",), ("alloc",)}:
            return path
        return target

    def borrowed(self, node, source, scope, seen=frozenset()):
        # Function signatures describe invocation, not borrowed field storage.
        if node.type == "function_type":
            return False
        if node.type in {"reference_type", "lifetime"}:
            return True
        path = dependency.parts(source, node)
        if path:
            target = self.resolve(scope, path)
            if path[-1] == "Cow" or target == ("std", "borrow", "Cow"):
                return True
            if target and target[0] == "shape":
                return True
            if target in self.declarations and target not in seen:
                declaration, other_source, _, other_scope = self.declarations[target]
                if any(self.borrowed(value, other_source, other_scope, seen | {target})
                       for value in storage_types(declaration)):
                    return True
        return any(self.borrowed(child, source, scope, seen) for child in node.named_children)

    def stored_lifetimes(self, node, source, scope):
        if node.type == "function_type":
            return set()
        if node.type == "generic_type":
            base = node.child_by_field_name("type")
            path = dependency.parts(source, base) if base is not None else ()
            target = self.resolve(scope, path)
            if path[-1:] == ("PhantomData",) or target in {
                ("std", "marker", "PhantomData"), ("core", "marker", "PhantomData"),
            }:
                return set()
        if node.type == "lifetime":
            return {dependency.text(source, node)}
        names = set()
        for child in node.named_children:
            names.update(self.stored_lifetimes(child, source, scope))
        return names

    def ownership_errors(self):
        # Reuse the existing import/alias resolver; dependency diagnostics have
        # their own runner entry and are not duplicated here.
        super().check()
        diagnostics = [message for message in self.errors if "LYR002" in message]
        for path, (node, source, file, scope) in self.declarations.items():
            if len(path) < 2:
                continue
            layer, *_, name = path
            if layer not in {"model", "shape"}:
                continue
            location = f"{file.relative_to(self.root)}:{node.start_point.row + 1}"
            parameters = node.child_by_field_name("type_parameters")
            lifetimes = {
                dependency.text(source, child)
                for child in walk(parameters) if child.type == "lifetime"
            } if parameters is not None else set()
            values = storage_types(node)
            if layer == "model":
                if name.endswith("Shape") or lifetimes or any(
                    self.borrowed(value, source, scope, frozenset({path})) for value in values
                ):
                    diagnostics.append(f"{location}: OWN001: model {name} must own its stored data")
                continue
            if len(path) < 4 or path[1] not in ROLES:
                diagnostics.append(f"{location}: SHP001: shapes belong under shape::read/write/shared::<domain>")
            domain = "".join(part.title() for part in path[2].split("_")) if len(path) >= 4 else ""
            if not name.endswith("Shape") or not name.startswith(domain):
                diagnostics.append(f"{location}: SHP002: {name} must be domain-qualified and end in Shape")
            stored = set().union(*(self.stored_lifetimes(value, source, scope) for value in values))
            if not lifetimes or not lifetimes.issubset(stored):
                diagnostics.append(f"{location}: SHP003: {name} must use its named lifetimes in borrowed storage")
        return sorted(diagnostics)


def check_root(root):
    manifest = root / "Cargo.toml"
    if not manifest.exists():
        return [f"{manifest}: OWN002: Cargo.toml is missing"]
    if tomllib.loads(manifest.read_text()).get("package", {}).get("name") != "poprako-server":
        return []
    return Scanner(root).ownership_errors()


class OwnershipTests(unittest.TestCase):
    def scan(self, model, shape="", support=""):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src/model/write").mkdir(parents=True)
            (root / "src/shape/write").mkdir(parents=True)
            (root / "Cargo.toml").write_text('[package]\nname = "poprako-server"\n')
            (root / "src/lib.rs").write_text("mod model; mod shape; mod value;\n")
            (root / "src/model.rs").write_text("pub mod write;\n")
            (root / "src/model/write.rs").write_text("pub mod unit;\n")
            (root / "src/shape.rs").write_text("pub mod write;\n")
            (root / "src/shape/write.rs").write_text("pub mod unit;\n")
            (root / "src/model/write/unit.rs").write_text(model)
            (root / "src/shape/write/unit.rs").write_text(shape)
            (root / "src/value.rs").write_text(support)
            return check_root(root)

    def test_owned_methods_and_mixed_shape(self):
        self.assertEqual([], self.scan(
            'pub struct UnitEdit { text: String }\nimpl UnitEdit { pub fn text(&self) -> &str { &self.text } }',
            "use crate::model::write::unit::UnitEdit; pub struct UnitPlanShape<'a> { edits: Vec<&'a UnitEdit>, id: String }",
        ))

    def test_nested_static_alias_and_cow(self):
        for definition in [
            "pub struct UnitEdit<'a> { text: Vec<Option<&'a str>> }",
            "pub enum UnitEdit { Text(&'static str) }",
            "pub struct UnitEdit { text: crate::value::Wrapper }",
            "use std::borrow::Cow as Text; pub struct UnitEdit { text: Text<'static, str> }",
            "use crate::value::Text; pub struct UnitEdit { text: Text }",
        ]:
            with self.subTest(definition=definition):
                self.assertTrue(any("OWN001" in err for err in self.scan(definition, support="pub type Text = &'static str; pub struct Wrapper { text: Text }")))

    def test_malformed_model_fails(self):
        self.assertTrue(any("LYR002" in err for err in self.scan("struct UnitEdit {")))

    def test_shape_nested_helpers_keep_domain_name(self):
        self.assertEqual([], self.scan("", "pub mod detail { pub struct UnitTextShape<'a> { text: &'a str } }"))

    def test_alias_of_fixed_lifetime_shape(self):
        self.assertTrue(any("OWN001" in err for err in self.scan(
            "use crate::shape::write::unit::UnitTextShape as Text; type Alias = Text<'static>; pub struct UnitEdit { text: Alias }",
            "pub struct UnitTextShape<'a> { text: &'a str }",
        )))

    def test_shape_name_and_real_lifetime(self):
        for declaration, code in [
            ("pub struct UnitText<'a> { text: &'a str }", "SHP002"),
            ("pub struct UnitTextShape { text: String }", "SHP003"),
            ("pub struct UnitTextShape<'a> { marker: std::marker::PhantomData<&'a str> }", "SHP003"),
            ("use std::marker::PhantomData as Marker; pub struct UnitTextShape<'a> { marker: Marker<&'a str> }", "SHP003"),
            ("pub struct WrongTextShape<'a> { text: &'a str }", "SHP002"),
        ]:
            with self.subTest(declaration=declaration):
                self.assertTrue(any(code in err for err in self.scan("", declaration)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(OwnershipTests)
        return int(not unittest.TextTestRunner().run(suite).wasSuccessful())
    diagnostics = check_root(args.root.resolve())
    for diagnostic in diagnostics:
        print(diagnostic, file=sys.stderr)
    return int(bool(diagnostics))


if __name__ == "__main__":
    raise SystemExit(main())
