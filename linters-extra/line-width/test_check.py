"""Regression coverage for macro scope, hard column limits, and CLI errors."""

from contextlib import redirect_stderr
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import check


class LineWidthTests(unittest.TestCase):
    def fixture(self, root, name, source):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")
        return path

    def test_all_violations_are_reported_and_sources_stay_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, ".rustfmt.toml", "max_width = 20\ntab_spaces = 4\n")
            sources = {
                "src/lib.rs": 'macro! { "' + "x" * 21 + '" }\n' + "x" * 20 + "\n",
                "member/src/tests.rs": '#[instrument(skip_all, fields(id = 123))]\nfn f() {}\n',
                "tests/regression.rs": 'm!("' + "x" * 21 + '");\n',
                "examples/demo.rs": 'm!("' + "x" * 21 + '");\n',
                "benches/probe.rs": 'm!("' + "x" * 21 + '");\n',
                "build.rs": 'm!("' + "x" * 21 + '");\n',
                "src/schema.rs": 'm!("' + "x" * 21 + '");\n',
            }
            for name, source in sources.items():
                self.fixture(root, name, source)
            out = io.StringIO()
            with redirect_stderr(out):
                self.assertEqual(check.check_root(root), 1)
            for name, source in sources.items():
                self.assertIn(f"{name}:1:21: WID001:", out.getvalue())
                self.assertEqual((root / name).read_text(), source)
            self.assertEqual(out.getvalue().count("WID001"), 7)
            self.assertIn("7 overlong macro lines in 7 files", out.getvalue())

    def test_ordinary_code_comments_strings_and_builtin_attributes_are_ignored(self):
        source = '''use crate::a_very_long_module_path::AnExcessivelyLongTypeName;
// instrument!(a comment that is deliberately wider than the limit)
const TEXT: &str = "instrument!(this is only a long ordinary string)";
#[allow(clippy::print_stdout, reason = "This long reason must remain untouched")]
#[expect(clippy::print_stdout, reason = "This long reason must remain untouched")]
#[cfg_attr(all(unix, not(windows)), derive(Debug, Clone))]
#[cfg_attr(unix, cfg_attr(unix, allow(clippy::print_stdout, reason = "A long reason")))]
struct Ordinary { a_very_long_field_name: AnExcessivelyLongTypeName }
'''
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, '.rustfmt.toml', 'max_width = 20\n')
            self.fixture(root, 'src/lib.rs', source)
            self.assertEqual(check.check_root(root), 0)

    def test_macro_definitions_nested_calls_and_wrapped_attributes_are_checked(self):
        source = '''#[cfg_attr(all(unix, not(windows)), cfg_attr(unix, tracing::instrument(skip_all, fields(id = 123))))]
fn f() { outer!(inner!("a deliberately long macro argument")); }
macro_rules! m {
    // This standalone comment is longer than the configured column limit.
    () => { nested!("a deliberately long macro argument"); };
}
quote! {
    #[allow(clippy::print_stdout, reason = "This long built-in attribute is ignored")]
    fn f() { inner!("a deliberately long macro argument"); }
}
'''
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, '.rustfmt.toml', 'max_width = 40\n')
            self.fixture(root, 'src/lib.rs', source)
            out = io.StringIO()
            with redirect_stderr(out):
                self.assertEqual(check.check_root(root), 1)
            self.assertEqual(out.getvalue().count('WID001'), 4)
            self.assertNotIn('src/lib.rs:4:', out.getvalue())
            self.assertNotIn('src/lib.rs:8:', out.getvalue())

    def test_limit_is_read_again_after_config_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = 'm!("' + "x" * 21 + '");\n'
            self.fixture(root, "src/lib.rs", source)
            width = len(source.rstrip())
            config = self.fixture(root, ".rustfmt.toml", f"max_width = {width}\n")
            self.assertEqual(check.check_root(root), 0)
            config.write_text(f"max_width = {width - 1}\n")
            with redirect_stderr(io.StringIO()):
                self.assertEqual(check.check_root(root), 1)

    def test_width_counts_columns_and_excludes_line_endings(self):
        self.assertEqual(check.line_width("界界", 4), 4)
        self.assertEqual(check.line_width("e\u0301", 4), 1)
        self.assertEqual(check.line_width("a\tX", 4), 5)
        self.assertEqual(check.line_width("界\tX", 4), 5)
        self.assertEqual(check.line_width("\tX", 8), 9)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, "rustfmt.toml", "max_width = 4\n")
            self.fixture(root, "src/lib.rs", "m!(\n界界\r\ne\u0301abc\nabc \nabc  \n);")
            out = io.StringIO()
            with redirect_stderr(out):
                self.assertEqual(check.check_root(root), 1)
            self.assertIn("src/lib.rs:5:5: WID001: line is 5 columns", out.getvalue())
            self.assertIn("1 overlong macro lines in 1 files", out.getvalue())

    def test_tooling_dependencies_and_build_output_are_excluded(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, ".rustfmt.toml", "max_width = 1\n")
            self.fixture(root, "src/lib.rs", "x\n")
            for name in check.IGNORED_DIRS:
                self.fixture(root, f"{name}/src/lib.rs", 'm!("xxx");\n')
                self.fixture(root, f"member/{name}/src/lib.rs", 'm!("xxx");\n')
            self.assertEqual(check.check_root(root), 0)

    def test_missing_and_invalid_config_fail_cli(self):
        configs = (None, "not toml", "tab_spaces = 4", "max_width = 0", "max_width = -1",
                   'max_width = "80"', "max_width = true", "max_width = 20\ntab_spaces = 0")
        for config in configs:
            with self.subTest(config=config), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                if config is not None:
                    self.fixture(root, ".rustfmt.toml", config)
                process = subprocess.run(
                    [sys.executable, check.__file__, "--root", str(root)],
                    capture_output=True, text=True, timeout=5,
                )
                self.assertEqual(process.returncode, 2, process.stderr)
                self.assertIn("line-width:", process.stderr)

    def test_cli_reports_every_line_and_fails_on_invalid_utf8(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, ".rustfmt.toml", "max_width = 2\n")
            source = self.fixture(root, "src/lib.rs", 'm!("xxx");\nm!("xxx");\n')
            args = [sys.executable, check.__file__, "--root", str(root)]
            process = subprocess.run(args, capture_output=True, text=True, timeout=5)
            self.assertEqual(process.returncode, 1, process.stderr)
            self.assertEqual(process.stderr.count("WID001"), 2)
            source.write_bytes(b"\xff\n")
            process = subprocess.run(args, capture_output=True, text=True, timeout=5)
            self.assertEqual(process.returncode, 2, process.stderr)

    def test_rustfmt_filename_precedence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root, ".rustfmt.toml", "max_width = 80\n")
            self.fixture(root, "rustfmt.toml", "max_width = 40\ntab_spaces = 8\n")
            self.assertEqual(check.read_config(root), check.WidthConfig(40, 8))


if __name__ == "__main__":
    unittest.main()
