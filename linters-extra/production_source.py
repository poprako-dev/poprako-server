"""Re-export the shared production-source masker from the linters submodule."""

from __future__ import annotations

import re
from contextlib import contextmanager
from contextvars import ContextVar
from pathlib import Path

from rust_style_lint.production_source import _test_module_files
from rust_style_lint.production_source import production_source as _production_source


_SOURCES: ContextVar[dict | None] = ContextVar("lint_sources", default=None)


@contextmanager
def source_session():
    """Reuse masked sources for one read-only run, never across edits or fixes."""
    _test_module_files.cache_clear()
    token = _SOURCES.set({})
    try:
        yield
    finally:
        _SOURCES.reset(token)
        _test_module_files.cache_clear()


def production_source(path: Path, root: Path) -> bytes:
    sources = _SOURCES.get()
    if sources is None:
        return _production_source(path, root)
    key = (path, root)
    if key not in sources:
        sources[key] = _production_source(path, root)
    return sources[key]


_NON_PRODUCTION_PARTS = {"test", "tests", "mock", "mocks", "mock_impl"}
_NON_PRODUCTION_FILENAME = re.compile(
    r"(?:^|[_-])(test|tests|mock|mocks)(?:[_-]|$)",
)


def is_production_path(path: Path) -> bool:
    """Return whether a Rust path belongs to production code."""
    if path.name == "tests.rs":
        return False

    if any(part in _NON_PRODUCTION_PARTS for part in path.parts):
        return False

    return _NON_PRODUCTION_FILENAME.search(path.stem) is None


def production_files(root: Path, subdir: str = "src") -> list[Path]:
    """Return Rust files outside test and mock paths."""
    return sorted(
        path
        for path in (root / subdir).rglob("*.rs")
        if is_production_path(path.relative_to(root))
    )
