#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

command -v uv >/dev/null 2>&1 || {
    echo "uv is required for CI checks" >&2
    exit 1
}

# Each check is independent. Run them all so one failure never hides another.
sh scripts/ci-parallel.sh \
    "Rust formatting" \
    "cargo fmt --all --check" \
    "Rust file length" \
    "sh scripts/check-rust-lines.sh" \
    "Deployment scripts" \
    "sh scripts/test-deployment-scripts.sh" \
    "Rust compilation" \
    "cargo check --workspace --all-targets --all-features" \
    "Rust lint" \
    "cargo clippy --workspace --all-targets --all-features -- -D warnings" \
    "Repository lint" \
    "sh linters-extra/run-check.sh"
