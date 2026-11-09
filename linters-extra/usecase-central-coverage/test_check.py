"""Exercise usecase/complex reachability using disposable Rust fixtures."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from check import check_root


class UsecaseCentralCoverageTests(unittest.TestCase):
    def check(self, files, package="poprako-server"):
        with tempfile.TemporaryDirectory(prefix="poprako-coverage-") as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(f'[package]\nname = "{package}"\n')
            files = {
                "src/lib.rs": "mod model; mod part; mod part_impl; mod complex; mod usecase;",
                **files,
            }
            for name, source in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)
            return check_root(root)

    def assert_unreachable(self, name, diagnostics):
        self.assertTrue(any(f"`{name}`" in item for item in diagnostics), diagnostics)

    def assert_clean(self, diagnostics):
        self.assertFalse(any("UCC001" in item for item in diagnostics), diagnostics)
        self.assertFalse(any("UCC002" in item for item in diagnostics), diagnostics)

    def test_reachable_definition_passes_and_unreachable_definition_fails(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Used; pub struct Dead;",
            "src/usecase/sample.rs": "use crate::model::sample::Used; fn f(_: Used) {}",
        })
        self.assertFalse(any("Used" in item and "UCC001" in item for item in diagnostics), diagnostics)
        self.assert_unreachable("struct Dead", diagnostics)

    def test_complex_is_also_a_root(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Used;",
            "src/complex/sample.rs": "fn f(_: crate::model::sample::Used) {}",
        })
        self.assert_clean(diagnostics)

    def test_transitive_steps_through_impl_output(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Payload;",
            "src/part/repo/oper/sample.rs": (
                "use crate::model::sample::Payload;\n"
                "pub struct Get<'a> { pub id: &'a str }\n"
                "impl<'a> Step for Get<'a> { type Output = Payload; }\n"
            ),
            "src/usecase/sample.rs": "use crate::part::repo::oper::sample::Get; fn f(_: Get<'_>) {}",
        })
        self.assert_clean(diagnostics)

    def test_oper_contract_attribute_links_output_model(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Payload;",
            "src/part/repo/oper/sample.rs": (
                "use crate::model::sample::Payload;\n"
                "#[oper(output = Payload)]\n"
                "pub struct Get<'a> { pub id: &'a str }\n"
            ),
            "src/usecase/sample.rs": "use crate::part::repo::oper::sample::Get; fn f(_: Get<'_>) {}",
        })
        self.assert_clean(diagnostics)

    def test_root_trait_bound_and_reexport_alias(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Thing;",
            "src/part/sample.rs": "pub trait Repo {} pub use crate::model::sample::Thing as Alias;",
            "src/usecase/sample.rs": "use crate::part::sample::{Repo, Alias}; fn f<R: Repo>(_: Alias) {}",
        })
        self.assert_clean(diagnostics)

    def test_glob_and_same_module_resolution(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Thing; pub struct Helper { pub inner: Thing }",
            "src/usecase/sample.rs": "use crate::model::sample::*; fn f(_: Helper) {}",
        })
        self.assert_clean(diagnostics)

    def test_adapter_and_test_only_consumers_do_not_count(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Thing;",
            "src/part/sample.rs": "pub trait Unused {}",
            "src/part_impl/sample.rs": "use crate::model::sample::Thing; use crate::part::sample::Unused; impl Unused for u8 {}",
            "src/usecase/sample.rs": "fn f(_: crate::model::sample::Thing) {}",
        })
        self.assert_unreachable("trait Unused", diagnostics)

    def test_cfg_test_usage_is_ignored(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct OnlyTest;",
            "src/usecase/sample.rs": "#[cfg(test)] mod tests { use crate::model::sample::OnlyTest; }",
        })
        self.assert_unreachable("struct OnlyTest", diagnostics)

    def test_comments_and_strings_are_not_references(self):
        diagnostics = self.check({
            "src/model/sample.rs": "pub struct Ghost;",
            "src/usecase/sample.rs": '// Ghost\nconst TEXT: &str = "Ghost";',
        })
        self.assert_unreachable("struct Ghost", diagnostics)

    def test_malformed_constrained_source_fails_closed(self):
        diagnostics = self.check({"src/model/sample.rs": "pub struct {"})
        self.assertTrue(any("UCC002" in item for item in diagnostics), diagnostics)

    def test_infrastructure_crate_is_not_restricted(self):
        diagnostics = self.check({"src/model/sample.rs": "pub struct Dead;"}, "poprako-rdb-core")
        self.assertEqual([], diagnostics)

    def test_cli_exit_code(self):
        with tempfile.TemporaryDirectory(prefix="poprako-coverage-") as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname = "poprako-server"\n')
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("mod model; mod usecase;")
            (root / "src/usecase.rs").write_text("")
            model = root / "src/model.rs"
            for source, exit_code in [
                ("pub struct Dead;", 1),
                ("pub struct Live;", 0),
            ]:
                model.write_text(source)
                usecase = root / "src/usecase.rs"
                usecase.write_text("fn f(_: crate::model::Live) {}")
                process = subprocess.run(
                    [sys.executable, str(Path(__file__).with_name("check.py")), "--root", str(root)],
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(exit_code, process.returncode, process.stderr)


if __name__ == "__main__":
    unittest.main()
