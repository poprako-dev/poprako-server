#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

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
    'if ! command -v uv >/dev/null 2>&1; then
        ci_tool_root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/poprako-ci-tools"
        python3 -m venv "$ci_tool_root"
        "$ci_tool_root/bin/python" -m pip install \
            --disable-pip-version-check \
            "uv==0.11.8"
        PATH="$ci_tool_root/bin:$PATH"
        export PATH
    fi
    sh linters-extra/run-check.sh'
