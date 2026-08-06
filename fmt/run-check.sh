#!/usr/bin/env sh
set -eu

export UV_CACHE_DIR="${UV_CACHE_DIR:-$PWD/.uv-cache}"

UV_SYNC=false
if [ ! -f fmt/.venv/bin/python ]; then
    echo "→ creating fmt venv …"
    uv venv fmt/.venv --seed 2>/dev/null
    UV_SYNC=true
fi

if $UV_SYNC || [ ! -f fmt/.venv/.sync-stamp ]; then
    echo "→ installing fmt dependencies …"
    uv pip sync fmt/requirements.txt --python fmt/.venv/bin/python
    date +%s > fmt/.venv/.sync-stamp
fi

FAILED=false

echo "━━━ rust-style-lint: shared checkers ━━━"
RUST_STYLE_LINT_DIR="${RUST_STYLE_LINT_DIR:-../rust-style-lint}"

if [ -d "$RUST_STYLE_LINT_DIR/rust_style_lint" ]; then
    if PYTHONPATH="$RUST_STYLE_LINT_DIR" \
        uv run --python fmt/.venv/bin/python python3 -m rust_style_lint --root .; then
        echo "✓ rust-style-lint passed"
    else
        echo "✗ rust-style-lint failed"
        FAILED=true
    fi
else
    echo "✗ rust-style-lint not found at $RUST_STYLE_LINT_DIR" >&2
    FAILED=true
fi
echo

for f in fmt/*/check.py; do
    [ -f "$f" ] || continue
    name=$(basename "$(dirname "$f")")
    echo "━━━ fmt: $name ━━━"
    passed=true

    case $name in
        direct-struct-import)
            for layer in model data; do
                uv run --python fmt/.venv/bin/python python3 "$f" --layer "$layer" || passed=false
            done
            ;;
        forbidden-identifiers)
            uv run --python fmt/.venv/bin/python python3 "$f" \
                --ignore-list fmt/forbidden-identifiers/ignore-files.txt \
                || passed=false
            ;;
        *)
            uv run --python fmt/.venv/bin/python python3 "$f" || passed=false
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
