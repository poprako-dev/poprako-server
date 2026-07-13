#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

from pathlib import Path
import runpy
import sys


checker = Path(__file__).parents[1] / "direct-struct-import" / "check.py"
namespace = runpy.run_path(checker)
sys.argv.extend(("--layer", "model"))
raise SystemExit(namespace["main"]())
