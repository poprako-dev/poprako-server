#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

# Supply only credentials for a dedicated integration bucket. Never inherit
# the developer's .env or production credentials in this CI entry point.
: "${INTEGRATION_DATABASE_URL:?A disposable integration database is required}"
: "${R2_ACCOUNT_ID:?Integration R2 account is required}"
: "${R2_ACCESS_KEY_ID:?Integration R2 access key is required}"
: "${R2_SECRET_ACCESS_KEY:?Integration R2 secret key is required}"
: "${R2_BUCKET_NAME:?A dedicated integration R2 bucket is required}"
: "${R2_CUSTOM_DOMAIN:?Integration asset domain is required}"

if ! command -v diesel >/dev/null 2>&1; then
    cargo install --locked --no-default-features --features postgres --version 2.3.7 diesel_cli
fi

export ENV_FILE=/dev/null
export JWT_SECRET=poprako-disposable-integration-jwt-secret
export JWT_EXPIRATION_HOURS=72
export POPRAKO_SNOWFLAKE_NODE_ID=0
export START_API_SERVER=1
export RUN_MIGRATIONS=1

sh scripts/ci-typecheck.sh
sh scripts/api-integration-test.sh
