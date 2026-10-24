"""Run shared and project checkers in one process with per-run analysis reuse."""

from __future__ import annotations

import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import time

from analysis_session import analysis_session
from production_source import source_session
from rust_style_lint import runner as shared_runner
from rust_style_lint.base import IGNORED_SOURCE_PARENTS


CHECKER_DIR = Path(__file__).resolve().parent
ALIASES = {"model-struct-qualification", "data-struct-qualification"}
SELF_TESTS = {"defer-oper-inline", "oper-inline", "diesel-type-safety", "layer-dependency", "model-shape-boundaries", "line-width", "configuration"}
WORKSPACE_CHECKERS = {"line-width", "configuration"}


def crate_roots(root: Path) -> list[Path]:
    roots = []
    for directory, names, files in os.walk(root):
        names[:] = [name for name in names if name not in IGNORED_SOURCE_PARENTS | {".uv-cache"}]
        path = Path(directory)
        if "Cargo.toml" in files and (path / "src").is_dir():
            roots.append(path)
    return sorted(roots)


def checker_paths() -> list[Path]:
    return [path for path in sorted(CHECKER_DIR.glob("*/check.py")) if path.parent.name not in ALIASES]


def load_checker(path: Path):
    name = "poprako_lint_" + path.parent.name.replace("-", "_")
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Cannot load checker: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def invoke(module, args: list[str]) -> int:
    previous = sys.argv
    sys.argv = [module.__file__, *args]
    try:
        return module.main()
    except SystemExit as exit_status:
        return exit_status.code if isinstance(exit_status.code, int) else int(exit_status.code is not None)
    finally:
        sys.argv = previous


def run_extra(root: Path, paths: list[Path]) -> int:
    roots = crate_roots(root)
    failed = False
    for path in paths:
        name = path.parent.name
        started = time.perf_counter()
        print(f"━━━ linters-extra: {name} ━━━", flush=True)
        passed = True
        try:
            module = load_checker(path)
            for crate in [root] if name in WORKSPACE_CHECKERS else roots:
                variants = [["--layer", layer, "--no-fix"] for layer in ("model", "data", "shape")] if name == "direct-struct-import" else [[]]
                for extra in variants:
                    if invoke(module, ["--root", str(crate), *extra]) != 0:
                        passed = False
        except Exception as err:
            print(f"{name}: {err}", file=sys.stderr)
            passed = False
        elapsed = time.perf_counter() - started
        print(f"{'✓' if passed else '✗'} {name} {'passed' if passed else 'failed'} ({elapsed:.2f}s)", flush=True)
        failed = failed or not passed
    return int(failed)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    root = args.root.resolve()
    paths = checker_paths()
    failed = False

    # Keep fixture-writing self-tests outside the read-only analysis session.
    # Isolated processes also avoid collisions between test modules named check.
    for path in paths:
        if path.parent.name in SELF_TESTS:
            result = subprocess.run([sys.executable, str(path), "--self-test"], cwd=root)
            failed = failed or result.returncode != 0

    with analysis_session(), source_session():
        print("━━━ rust-style-lint: shared checkers ━━━", flush=True)
        started = time.perf_counter()
        shared_status = invoke(shared_runner, ["--root", str(root)])
        elapsed = time.perf_counter() - started
        print(f"{'✓' if shared_status == 0 else '✗'} shared checkers ({elapsed:.2f}s)", flush=True)
        failed = failed or shared_status != 0
        extra_status = run_extra(root, paths)
        failed = failed or extra_status != 0

    if not failed:
        print("all checkers passed")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
