#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "expected migration directory and PostgreSQL container name" >&2
    exit 1
fi

migration_root=$1
postgres_container=$2

case "$postgres_container" in
    "" | *[!A-Za-z0-9._-]*)
        echo "PostgreSQL container name contains unsupported characters" >&2
        exit 1
        ;;
esac

[ -d "$migration_root" ] || {
    echo "migration directory does not exist: $migration_root" >&2
    exit 1
}

postgres_running=$(docker container inspect \
    --format '{{.State.Running}}' \
    "$postgres_container")

[ "$postgres_running" = "true" ] || {
    echo "PostgreSQL container is not running" >&2
    exit 1
}

migration_batch=$(mktemp)

cleanup() {
    rm -f "$migration_batch"
}

trap cleanup EXIT
trap 'exit 1' INT TERM

LC_ALL=C
export LC_ALL

migration_count=0

{
    printf 'BEGIN;\n'

    for migration_dir in "$migration_root"/*; do
        [ -d "$migration_dir" ] || continue

        migration_file="${migration_dir}/up.sql"
        [ -f "$migration_file" ] || {
            echo "missing migration file: $migration_file" >&2
            exit 1
        }

        cat "$migration_file"
        printf '\n'
        migration_count=$((migration_count + 1))
    done

    printf 'COMMIT;\n'
} >"$migration_batch"

[ "$migration_count" -gt 0 ] || {
    echo "no migrations found" >&2
    exit 1
}

docker exec -i "$postgres_container" sh -eu -c \
    'database_name=${POSTGRES_DB:-${POSTGRES_USER:?}}; exec psql --no-psqlrc --set ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$database_name"' \
    <"$migration_batch"
