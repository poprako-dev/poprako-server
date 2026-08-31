#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

for rust_file in $(find . \
    -path './.git' -prune -o \
    -path './target' -prune -o \
    -path './linters-extra/.venv' -prune -o \
    -path '*/src/*' -type f -name '*.rs' -print \
    | LC_ALL=C sort); do
    max_lines=600

    line_count=$(wc -l < "$rust_file")

    if [ "$line_count" -gt "$max_lines" ]; then
        echo "$rust_file has $line_count lines; maximum is $max_lines" >&2
        exit 1
    fi
done
