#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

uv run --no-project python scripts/check-lint-policy.py
uv run --no-project python scripts/test-lint-policy.py
