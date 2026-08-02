#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

cargo fmt --all --check
cargo check --workspace --all-targets --all-features
sh fmt/run-check.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
