#!/usr/bin/env sh
set -eu

# Runs both lint layers:
#   1. the shared `linters/` submodule suite (rust-style-lint), selected
#      and configured by the root rust-style-lint.toml, then
#   2. the PopRaKo-specific checkers versioned in this directory.

export UV_CACHE_DIR="${UV_CACHE_DIR:-$PWD/.uv-cache}"
export PYTHONPATH="$PWD/linters${PYTHONPATH:+:$PYTHONPATH}"

UV_SYNC=false
if [ ! -f linters-extra/.venv/bin/python ]; then
    echo "→ creating linters-extra venv …"
    uv venv linters-extra/.venv --seed 2>/dev/null
    UV_SYNC=true
fi

if $UV_SYNC || [ ! -f linters-extra/.venv/.sync-stamp ]; then
    echo "→ installing linters-extra dependencies …"
    uv pip sync linters-extra/requirements.txt --python linters-extra/.venv/bin/python
    date +%s > linters-extra/.venv/.sync-stamp
fi

# All checkers share one read-only analysis session. Standalone checker CLIs
# remain available for individual rules and explicit fixes.
exec linters-extra/.venv/bin/python linters-extra/runner.py "$@"
