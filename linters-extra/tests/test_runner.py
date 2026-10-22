"""Verify checker dispatch and read-only CLI behavior with small fixtures."""

import os
from contextlib import redirect_stderr, redirect_stdout
import io
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]


sys.path.insert(0, str(ROOT / "linters"))
sys.path.insert(0, str(ROOT / "linters-extra"))
import runner
from analysis_session import analysis_session
from production_source import source_session


class RunnerTests(unittest.TestCase):
    def test_shared_analysis_preserves_every_rule_diagnostic(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname = "poprako-server"\n')
            files = {
                "src/lib.rs": "mod model; mod complex; mod data;",
                "src/model/team.rs": "pub struct Info;",
                "src/complex/team.rs": "use crate::data::instr::team::Input;",
                "src/data/instr/team.rs": "pub struct Input;",
            }
            for name, source in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)
            modules = [(path.parent.name, runner.load_checker(path)) for path in runner.checker_paths()]

            def collect():
                results = []
                for name, module in modules:
                    variants = [["--layer", layer, "--no-fix"] for layer in ("model", "data")] if name == "direct-struct-import" else [[]]
                    for extra in variants:
                        out, err = io.StringIO(), io.StringIO()
                        with redirect_stdout(out), redirect_stderr(err):
                            status = runner.invoke(module, ["--root", str(root), *extra])
                        results.append((name, status, out.getvalue(), err.getvalue()))
                return results

            original = collect()
            self.assertTrue(any(status != 0 for _, status, _, _ in original))
            with analysis_session(), source_session():
                self.assertEqual(collect(), original)

    def test_each_layer_runs_once_per_crate_without_fixing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for crate in (root, root / "member with spaces", root / "target/generated", root / "nested/node_modules/pkg"):
                (crate / "src").mkdir(parents=True)
                (crate / "Cargo.toml").touch()
            calls = []
            module = SimpleNamespace(__file__="check.py", main=lambda: calls.append(sys.argv[1:]) or 0)
            with patch.object(runner, "load_checker", return_value=module):
                status = runner.run_extra(root, [Path("direct-struct-import/check.py")])
            self.assertEqual(status, 0)
            self.assertEqual(len(calls), 4)
            self.assertEqual({call[1] for call in calls}, {str(root), str(root / "member with spaces")})
            self.assertTrue(all("--no-fix" in call for call in calls))

    def test_failures_do_not_skip_later_rules(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").touch()
            modules = [
                SimpleNamespace(__file__="first", main=lambda: 1),
                SimpleNamespace(__file__="second", main=lambda: 0),
            ]
            with patch.object(runner, "load_checker", side_effect=modules) as load:
                self.assertEqual(runner.run_extra(root, [Path("first/check.py"), Path("second/check.py")]), 1)
            self.assertEqual(load.call_count, 2)

    def test_compatibility_aliases_are_not_dispatched_twice(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("direct-struct-import", "model-struct-qualification", "data-struct-qualification"):
                (root / name).mkdir()
                (root / name / "check.py").touch()
            with patch.object(runner, "CHECKER_DIR", root):
                self.assertEqual([path.parent.name for path in runner.checker_paths()], ["direct-struct-import"])

    def test_cli_arguments_are_restored_after_failure(self):
        previous = sys.argv
        module = SimpleNamespace(__file__="broken", main=lambda: (_ for _ in ()).throw(RuntimeError("fixture")))
        with self.assertRaises(RuntimeError):
            runner.invoke(module, ["--root", "fixture"])
        self.assertIs(sys.argv, previous)

    def test_shell_launches_one_dispatcher(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            extra = root / "linters-extra"
            python = extra / ".venv/bin/python"
            python.parent.mkdir(parents=True)
            (extra / ".venv/.sync-stamp").touch()
            shutil.copy(ROOT / "linters-extra/run-check.sh", extra / "run-check.sh")
            python.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > calls\n')
            python.chmod(0o755)
            result = subprocess.run(["sh", "linters-extra/run-check.sh"], cwd=root, capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((root / "calls").read_text().splitlines(), ["linters-extra/runner.py"])


class DirectImportCliTests(unittest.TestCase):
    def test_checks_are_read_only_and_fix_is_explicit(self):
        for checker, layer in (
            ("direct-struct-import", "model"),
            ("model-struct-qualification", "model"),
            ("data-struct-qualification", "data"),
        ):
            with self.subTest(checker=checker), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                domain = root / ("src/model/team.rs" if layer == "model" else "src/data/instr/team.rs")
                domain.parent.mkdir(parents=True)
                domain.write_text("pub struct Team;\n")
                source = root / "src/lib.rs"
                original = f"use crate::{layer}::team_{layer};\n"
                source.write_text(original)
                args = [sys.executable, str(ROOT / "linters-extra" / checker / "check.py"), "--root", str(root)]
                if checker == "direct-struct-import":
                    args.extend(("--layer", layer))
                env = {**os.environ, "PYTHONPATH": str(ROOT / "linters")}
                result = subprocess.run(args, env=env, capture_output=True, text=True, timeout=5)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertEqual(source.read_text(), original)
                result = subprocess.run([*args, "--fix"], env=env, capture_output=True, text=True, timeout=5)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertNotIn("team_", source.read_text())


if __name__ == "__main__":
    unittest.main()
