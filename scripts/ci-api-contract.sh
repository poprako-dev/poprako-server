#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
web_root="${WEB_ROOT:-$project_root/web}"

if [ ! -f "$web_root/script/check-api-contract.mjs" ]; then
    echo "frontend contract client is missing: $web_root" >&2
    exit 1
fi

cd "$web_root"
pnpm install --frozen-lockfile --ignore-scripts
node script/check-api-contract.mjs --swagger "$project_root/docs/swagger.json"
