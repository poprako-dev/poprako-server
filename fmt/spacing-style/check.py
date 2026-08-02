#!/usr/bin/env python3

"""Run the shared Rust block-spacing checker from the format suite."""

from __future__ import annotations

import runpy
import sys
from pathlib import Path


ROOT = Path(__file__).parents[2]
CHECKER = ROOT / "fmt/spacing-style/check_impl.py"


if __name__ == "__main__":
    sys.argv[0] = str(CHECKER)
    runpy.run_path(CHECKER, run_name="__main__")
