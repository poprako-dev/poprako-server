#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if ! cargo audit --version >/dev/null 2>&1; then
    cargo install --locked --version 0.22.2 cargo-audit
fi

# Accepted exceptions for zip 3.0.0, a build dependency of utoipa-swagger-ui 9.0.2.
# Remove these exceptions when upstream supports a patched zip release.
run_audit() {
    cargo audit \
        --ignore RUSTSEC-2026-0367 \
        --ignore RUSTSEC-2026-0368 \
        --ignore RUSTSEC-2026-0369 \
        "$@"
}

if run_audit; then
    exit 0
fi

printf '%s\n' \
    'warning: advisory database refresh failed; retrying with the cached database' >&2
run_audit --no-fetch
