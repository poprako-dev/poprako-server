#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

if [ "${CI_MIGRATION_DATABASE:-}" != "1" ]; then
    echo "CI_MIGRATION_DATABASE=1 is required for destructive migration checks" >&2
    exit 1
fi

case "${DATABASE_URL:-}" in
    postgres://*/db_poprako_ci | postgresql://*/db_poprako_ci)
        ;;
    *)
        echo "DATABASE_URL must target the dedicated db_poprako_ci database" >&2
        exit 1
        ;;
esac

if ! command -v diesel >/dev/null 2>&1; then
    cargo install \
        --locked \
        --no-default-features \
        --features postgres \
        --version 2.3.7 \
        diesel_cli
fi

diesel migration run --config-file /dev/null
diesel migration revert --all --config-file /dev/null
diesel migration run --config-file /dev/null
