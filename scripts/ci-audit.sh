#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if ! cargo audit --version >/dev/null 2>&1; then
    cargo install --locked --version 0.22.2 cargo-audit
fi

if cargo audit; then
    exit 0
fi

printf '%s\n' \
    'warning: advisory database refresh failed; retrying with the cached database' >&2
cargo audit --no-fetch
