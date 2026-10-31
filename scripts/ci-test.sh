#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

# Execute the PostgreSQL repository and Prom regressions behind feature gates.
# These tests own their disposable PostgreSQL containers and require Docker.
cargo test --workspace --all-features
