"""Re-export the shared production-source masker from the linters submodule.

The canonical implementation lives in the `linters/` submodule
(`rust_style_lint/production_source.py`) and is pinned by poprako-server's
.gitmodules. This shim loads it under a distinct module name so that
`from production_source import production_source` inside the local checkers
keeps resolving to this file without a circular import.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


_IMPLEMENTATION = (
    Path(__file__).parent.parent / "linters" / "rust_style_lint" / "production_source.py"
)
_MODULE_NAME = "_linters_submodule_production_source"

spec = importlib.util.spec_from_file_location(_MODULE_NAME, _IMPLEMENTATION)

assert spec is not None
assert spec.loader is not None

module = importlib.util.module_from_spec(spec)
sys.modules[_MODULE_NAME] = module
spec.loader.exec_module(module)

production_source = module.production_source
