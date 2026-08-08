#!/usr/bin/env sh
set -eu

# Local full CI: runs every CI check script exactly like GitHub Actions.
# The migration database is prepared automatically, mirroring the
# migrations job's `services` block:
#
#   1. A disposable postgres:18-alpine container is started when Docker
#      can pull the image (identical to GA).
#   2. Otherwise a local PostgreSQL instance is discovered on 5432/3306
#      and the CI database is created on demand.
#
# Nothing needs manual bootstrap, and the container is removed afterwards.

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

DB_NAME="db_poprako_ci"
CONTAINER="poprako-ci-pg"
CONTAINER_USER="poprako"
CONTAINER_PASSWORD="poprako"
LOCAL_USER="${CI_LOCAL_DB_USER:-postgres}"
LOCAL_PASSWORD="${CI_LOCAL_DB_PASSWORD:-devpwd}"

DATABASE_URL=""
CLEANUP=""

cleanup() {
    if [ -n "$CLEANUP" ]; then
        docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT INT TERM

prepare_container_db() {
    echo "━━━ ci-local: starting $CONTAINER (postgres:18-alpine) ━━━"
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true

    if ! docker run -d \
        --name "$CONTAINER" \
        -e "POSTGRES_DB=$DB_NAME" \
        -e "POSTGRES_PASSWORD=$CONTAINER_PASSWORD" \
        -e "POSTGRES_USER=$CONTAINER_USER" \
        -p 5432:5432 \
        postgres:18-alpine >/dev/null 2>&1; then
        echo "ci-local: docker image unavailable; falling back to local PostgreSQL" >&2
        return 1
    fi

    CLEANUP=1

    for _ in $(seq 1 30); do
        if docker exec "$CONTAINER" pg_isready -U "$CONTAINER_USER" -d "$DB_NAME" >/dev/null 2>&1; then
            DATABASE_URL="postgres://$CONTAINER_USER:$CONTAINER_PASSWORD@127.0.0.1:5432/$DB_NAME"
            return 0
        fi

        sleep 1
    done

    return 1
}

prepare_local_db() {
    DB_PORT=""

    for port in 5432 3306; do
        if PGPASSWORD="$LOCAL_PASSWORD" pg_isready -h 127.0.0.1 -p "$port" -U "$LOCAL_USER" -q 2>/dev/null; then
            DB_PORT="$port"
            break
        fi
    done

    if [ -z "$DB_PORT" ]; then
        echo "✗ ci-local: no local PostgreSQL on 5432/3306 and no docker image" >&2
        return 1
    fi

    if ! PGPASSWORD="$LOCAL_PASSWORD" psql -h 127.0.0.1 -p "$DB_PORT" -U "$LOCAL_USER" -d postgres \
        -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'" | grep -q 1; then
        PGPASSWORD="$LOCAL_PASSWORD" psql -h 127.0.0.1 -p "$DB_PORT" -U "$LOCAL_USER" -d postgres \
            -c "CREATE DATABASE $DB_NAME" >/dev/null
    fi

    DATABASE_URL="postgres://$LOCAL_USER:$LOCAL_PASSWORD@127.0.0.1:$DB_PORT/$DB_NAME"
    return 0
}

prepare_container_db || prepare_local_db || exit 1

export CI_MIGRATION_DATABASE=1
export DATABASE_URL

for script in \
    scripts/ci-check.sh \
    scripts/ci-openapi-check.sh \
    scripts/ci-test.sh \
    scripts/ci-typecheck.sh \
    scripts/ci-migration-check.sh \
    scripts/ci-audit.sh
do
    echo "━━━ ci-local: running $script ━━━"

    if ! sh "$script"; then
        echo "✗ ci-local: $script failed" >&2
        exit 1
    fi
done

echo "✓ ci-local: full CI chain passed"
