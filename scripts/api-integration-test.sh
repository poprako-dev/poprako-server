#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
env_file="${ENV_FILE:-$project_root/.env}"
api_base_url="${API_BASE_URL:-http://127.0.0.1:8888}"
start_api_server="${START_API_SERVER:-1}"
run_migrations="${RUN_MIGRATIONS:-1}"
server_pid=""

if [ -f "$env_file" ]; then
    set -a
    # shellcheck disable=SC1090
    . "$env_file"
    set +a
fi

DATABASE_URL="${DATABASE_URL:-}"

if [ -z "$DATABASE_URL" ]; then
    echo "DATABASE_URL must be set" >&2
    exit 1
fi

export API_BASE_URL="$api_base_url"
export DATABASE_URL

cleanup() {
    if [ -n "$server_pid" ]; then
        kill "$server_pid" >/dev/null 2>&1 || true
        wait "$server_pid" >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT INT TERM

cd "$project_root"

if [ "$run_migrations" = "1" ]; then
    just mgr-run
fi

if [ "$start_api_server" = "1" ]; then
    existing_status=$(curl -s -o /dev/null -w '%{http_code}' "$api_base_url/api/health" || true)

    if [ "$existing_status" = "204" ]; then
        echo "API server already responds at $api_base_url; stop it or set START_API_SERVER=0" >&2
        exit 1
    fi

    cargo run -p poprako-r --bin poprako-r &
    server_pid="$!"

    health_ok=0
    health_attempt=1

    while [ "$health_attempt" -le 60 ]; do
        if ! kill -0 "$server_pid" >/dev/null 2>&1; then
            wait "$server_pid" || true
            echo "API server process exited before becoming healthy" >&2
            exit 1
        fi

        status=$(curl -s -o /dev/null -w '%{http_code}' "$api_base_url/api/health" || true)

        if [ "$status" = "204" ]; then
            health_ok=1
            break
        fi

        health_attempt=$((health_attempt + 1))
        sleep 1
    done

    if [ "$health_ok" != "1" ]; then
        echo "API server did not become healthy at $api_base_url" >&2
        exit 1
    fi
fi

cd "$project_root/tests"

if [ ! -d node_modules ]; then
    pnpm install
fi

pnpm api
