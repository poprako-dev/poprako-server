#!/usr/bin/env bash
set -euo pipefail

DEPS="tree-sitter==0.25.0, tree-sitter-rust==0.23.3"
FAILED=false

for f in fmt/*/check.py; do
    [ -f "$f" ] || continue
    name=$(basename "$(dirname "$f")")
    echo "━━━ fmt: $name ━━━"
    passed=true

    case $name in
        direct-struct-import)
            for layer in model data; do
                uv run --with "$DEPS" python3 "$f" --layer "$layer" || passed=false
            done
            ;;
        *)
            uv run --with "$DEPS" python3 "$f" || passed=false
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

$FAILED && exit 1
echo "all checkers passed"
