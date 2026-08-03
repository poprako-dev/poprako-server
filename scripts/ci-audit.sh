#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if ! cargo audit --version >/dev/null 2>&1; then
    cargo install --locked --version 0.22.2 cargo-audit
fi

cargo audit
