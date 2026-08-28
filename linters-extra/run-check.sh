#!/usr/bin/env sh
set -eu

# Runs both lint layers:
#   1. the shared `linters/` submodule suite (rust-style-lint), selected
#      and configured by the root rust-style-lint.toml, then
#   2. the PopRaKo-specific checkers versioned in this directory.

export UV_CACHE_DIR="${UV_CACHE_DIR:-$PWD/.uv-cache}"

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

FAILED=false

echo "━━━ rust-style-lint: shared checkers (linters/ submodule) ━━━"

if [ -d linters/rust_style_lint ]; then
    if PYTHONPATH="$PWD/linters" \
        uv run --python linters-extra/.venv/bin/python python3 -m rust_style_lint --root .; then
        echo "✓ rust-style-lint passed"
    else
        echo "✗ rust-style-lint failed"
        FAILED=true
    fi
else
    echo "✗ linters/ submodule is not initialized; run git submodule update --init" >&2
    FAILED=true
fi
echo

for f in linters-extra/*/check.py; do
    [ -f "$f" ] || continue
    name=$(basename "$(dirname "$f")")
    echo "━━━ linters-extra: $name ━━━"
    passed=true

    case $name in
        direct-struct-import)
            for layer in model data; do
                uv run --python linters-extra/.venv/bin/python python3 "$f" --layer "$layer" || passed=false
            done
            ;;
        defer-oper-inline|oper-inline)
            uv run --python linters-extra/.venv/bin/python python3 "$f" --self-test || passed=false
            uv run --python linters-extra/.venv/bin/python python3 "$f" || passed=false
            ;;
        *)
            uv run --python linters-extra/.venv/bin/python python3 "$f" || passed=false
            ;;
    esac

    if $passed; then
        echo "✓ $name passed"
    else
        echo "✗ $name failed"
        FAILED=true
    fi
    echo
done

[ "$FAILED" = false ] || exit 1
echo "all checkers passed"
