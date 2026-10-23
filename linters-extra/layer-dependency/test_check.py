"""Exercise forbidden edges and legal growth using disposable Rust fixtures."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from check import FORBIDDEN, Scanner, check_root


class LayerDependencyTests(unittest.TestCase):
    def test_anonymous_imports_do_not_prevent_resolution_convergence(self):
        original = Scanner.resolve
        calls = []

        def resolve(scanner, *args, **kwargs):
            calls.append(args)
            return original(scanner, *args, **kwargs)

        with patch.object(Scanner, "resolve", resolve):
            diagnostics = self.check({"src/complex/sample.rs": (
                "use crate::model::{First as _, Second as _, Third as _, Fourth as _};"
            )})

        self.assertEqual(diagnostics, [])
        self.assertLessEqual(len(calls), 16)

    def test_anonymous_imports_still_report_forbidden_dependencies(self):
        self.assert_edge("use crate::data::{First as _, Second as _};")

    def check(self, files, package="poprako-server"):
        with tempfile.TemporaryDirectory(prefix="poprako-layers-") as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(f'[package]\nname = "{package}"\n')
            files = {
                "src/lib.rs": "mod value; mod model; mod shape; mod complex; mod data; mod part; mod usecase; mod util; mod future;",
                **files,
            }
            for name, source in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)
            return check_root(root)

    def assert_edge(self, source, origin="complex", target="data", files=None):
        diagnostics = self.check({f"src/{origin}/sample.rs": source, **(files or {})})
        self.assertTrue(any(f"`{origin} -> {target}`" in item for item in diagnostics), diagnostics)
        self.assertFalse(any("LYR002" in item for item in diagnostics), diagnostics)

    def test_every_explicitly_forbidden_edge(self):
        for origin, targets in FORBIDDEN.items():
            for target in targets:
                with self.subTest(origin=origin, target=target):
                    self.assert_edge(f"use crate::{target}::Thing;", origin, target)

    def test_architecture_regressions_independent_of_the_rule_table(self):
        for origin, target in [("value", "model"), ("value", "shape"), ("model", "shape"), ("shape", "complex"), ("shape", "data"), ("shape", "part_impl"), ("model", "complex"), ("data", "complex"), ("complex", "data"), ("part", "usecase"), ("usecase", "part_impl")]:
            with self.subTest(origin=origin, target=target):
                self.assert_edge(f"use crate::{target}::Thing;", origin, target)

    def test_core_legal_edges_and_new_dependencies(self):
        cases = {
            "value": ["result", "util", "future", "value::new_type"],
            "model": ["value", "result", "util", "future", "model::new_model"],
            "shape": ["model", "value", "shape::write", "util", "result"],
            "complex": ["shape", "model", "value", "result", "util", "config", "future"],
            "data": ["model", "value", "result", "util", "future", "data::new_dto"],
            "part": ["shape", "model", "value", "result", "future"],
            "usecase": ["shape", "model", "value", "complex", "data", "part", "result", "future"],
            "future": ["data", "complex", "part_impl"],
        }
        for origin, targets in cases.items():
            for target in targets:
                with self.subTest(origin=origin, target=target):
                    self.assertEqual([], self.check({f"src/{origin}/new.rs": f"use crate::{target}::NewThing;"}))

    def test_import_path_forms(self):
        cases = [
            "use crate::data::instr::Input as Renamed;",
            "pub use crate::{model::Info, data::{instr::Input as Renamed, view::*}};",
            "use crate::data::{self as dto};",
            "use crate::data::*;",
            "use crate /* comment */ :: r#data :: Thing;",
            "use super::super::data::Thing;",
            "use poprako_server::data::Thing;",
            "use ::poprako_server::data::Thing;",
            "use crate as app; use app::data::Thing;",
            "use app::data::Thing; use crate as app;",
            "use crate::{self as app}; use app::data::*;",
            "use app::{data::Thing}; use crate as app;",
            "extern crate self as app; use app::data::Thing;",
            "extern crate poprako_server as app; use app::data::Thing;",
        ]
        for source in cases:
            with self.subTest(source=source):
                self.assert_edge(source)

    def test_qualified_paths_and_generic_arguments(self):
        cases = [
            "fn run() { crate::data::run(); }",
            "fn run(_: Option<crate::data::Thing>) {}",
            "type Thing = crate::data::Thing;",
            "impl crate::data::Trait for Local {}",
            "fn run() { crate::data::Thing::<u8>::run(); }",
            "fn run() { call::<crate::data::Thing>(); }",
            "use crate as app; fn run() { app::data::run(); }",
            "use crate::*; fn run(_: data::Thing) {}",
        ]
        for source in cases:
            with self.subTest(source=source):
                self.assert_edge(source)

    def test_macro_and_attribute_tokens(self):
        for source in [
            "fn run() { wrap!(crate::data::Thing); }",
            "fn run() { wrap!({crate /* comment */ :: r#data :: Thing}); }",
            "#[derive(crate::data::Trait)] struct Thing;",
            "#[custom(type = crate::data::Thing)] struct Thing;",
            "macro_rules! build { () => { crate::data::run() }; }",
            "use crate as app; fn run() { wrap!(app::data::Thing); }",
        ]:
            with self.subTest(source=source):
                self.assert_edge(source)

    def test_nested_module_and_parent_alias(self):
        self.assert_edge("mod nested { use super::super::super::data::Thing; }")
        self.assert_edge("use crate as root; mod nested { fn f(_: super::root::data::Thing) {} }")
        self.assert_edge("fn f() { use crate as root; { let _: root::data::Thing; } }")

    def test_inline_layer_in_crate_root(self):
        diagnostics = self.check({"src/lib.rs": "mod data {} mod complex { use crate::data::Thing; }"})
        self.assertTrue(any("complex -> data" in item for item in diagnostics), diagnostics)

    def test_layer_root_files(self):
        diagnostics = self.check({"src/complex.rs": "use super::data::Thing;"})
        self.assertTrue(any("complex -> data" in item for item in diagnostics), diagnostics)

    def test_reexported_root_and_type(self):
        self.assert_edge(
            "use crate::util::app; fn f(_: app::data::Thing) {}",
            files={"src/util.rs": "pub use crate as app;"},
        )
        self.assert_edge(
            "use crate::util::Exposed;",
            files={"src/util.rs": "pub use crate::data::Thing as Exposed;"},
        )

    def test_comments_strings_external_names_and_local_modules(self):
        source = '''
// use crate::data::Thing;
/* crate::data::run(); */
/// See crate::data::Thing.
const TEXT: &str = r#"crate::data::Thing"#;
#[doc = "crate::data::Thing"] struct Local;
use external::data::Thing;
use external as data;
fn f() { log!("crate::data::Thing"); data::run(); }
mod nested { mod data {} fn f() { data::run(); } }
'''
        self.assertEqual([], self.check({"src/complex/sample.rs": source}))

    def test_forbidden_module_cannot_hide_behind_legal_reexport(self):
        self.assert_edge("use crate::data::Thing;", files={"src/data.rs": "pub use crate::model::Thing;"})

    def test_block_alias_shadowing(self):
        self.assertEqual([], self.check({"src/complex/sample.rs": '''
use crate as app;
fn f() { use external as app; app::data::run(); }
'''}))

    def test_cfg_test_exclusion_and_feature_inclusion(self):
        self.assertEqual([], self.check({"src/complex/sample.rs": '''
#[cfg(test)] mod tests { use crate::data::Thing; }
#[cfg(all(test, feature = "fixture"))] fn f(_: crate::data::Thing) {}
#[cfg(test)] use crate::data::Thing;
'''}))
        for condition in ['feature = "optional"', 'any(test, feature = "optional")', "not(test)"]:
            with self.subTest(condition=condition):
                self.assert_edge(f"#[cfg({condition})] use crate::data::Thing;")

    def test_external_test_and_mock_files(self):
        self.assertEqual([], self.check({
            "src/complex/tests.rs": "use crate::data::Thing;",
            "src/complex/tests/case.rs": "use crate::data::Thing;",
            "src/complex/mock_impl.rs": "use crate::data::Thing;",
            "src/complex.rs": "#[cfg(test)] mod fixture;",
            "src/complex/fixture.rs": "use crate::data::Thing;",
        }))

    def test_infrastructure_crate_is_not_restricted(self):
        self.assertEqual([], self.check({"src/model.rs": "use crate::part_impl::Thing;"}, "poprako-rdb-core"))

    def test_invalid_rust_fails_closed_without_an_allowlist(self):
        diagnostics = self.check({"src/complex/sample.rs": "use crate::data::{;"})
        self.assertTrue(any("LYR002" in item for item in diagnostics), diagnostics)

    def test_diagnostic_location_and_cli_exit(self):
        diagnostics = self.check({"src/complex/sample.rs": "// 中文\nuse crate::data::Thing;"})
        self.assertTrue(any(item.startswith("src/complex/sample.rs:2:") for item in diagnostics), diagnostics)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname = "poprako-server"\n')
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("mod complex; mod data;")
            file = root / "src/complex.rs"
            for source, exit_code in [("use crate::data::Thing;", 1), ("use crate::future::Thing;", 0)]:
                file.write_text(source)
                process = subprocess.run([sys.executable, str(Path(__file__).with_name("check.py")), "--root", str(root)], capture_output=True, text=True)
                self.assertEqual(exit_code, process.returncode, process.stderr)


if __name__ == "__main__":
    unittest.main()
