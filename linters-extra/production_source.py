"""Re-export the shared production-source masker from the linters submodule."""

from __future__ import annotations

import re
from pathlib import Path

from rust_style_lint.production_source import production_source


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
