#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if ! command -v uv >/dev/null 2>&1; then
    ci_tool_root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/poprako-ci-tools"
    python3 -m venv "$ci_tool_root"
    "$ci_tool_root/bin/python" -m pip install \
        --disable-pip-version-check \
        "uv==0.11.8"
    PATH="$ci_tool_root/bin:$PATH"
    export PATH
fi

cargo fmt --all --check
sh scripts/check-rust-lines.sh
sh scripts/test-deployment-scripts.sh
cargo check --workspace --all-targets --all-features
sh linters-extra/run-check.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
