#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
generated_openapi=$(mktemp)

cleanup() {
    rm -f "$generated_openapi"
}

trap cleanup EXIT INT TERM

cd "$project_root"

cargo run -p poprako-swagger >"$generated_openapi"
diff -u docs/swagger.json "$generated_openapi"
