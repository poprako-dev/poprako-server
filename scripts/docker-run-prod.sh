#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

image_name="${IMAGE_NAME:-poprako-sr-prod:latest}"
container_name="${CONTAINER_NAME:-poprako-sr-prod}"
env_file="${ENV_FILE:-$project_root/.env}"
http_port="${HTTP_PORT:-18888}"

if [ ! -f "$env_file" ]; then
    echo "missing env file: $env_file" >&2
    exit 1
fi

database_url="${DATABASE_URL:-}"

if [ -z "$database_url" ]; then
    database_url=$(awk -F= '/^DATABASE_URL=/{print substr($0, 14); exit}' "$env_file")
fi

if [ -z "$database_url" ]; then
    echo "DATABASE_URL is not set in environment or $env_file" >&2
    exit 1
fi

container_database_url=$(printf '%s' "$database_url" | sed \
    -e 's/@localhost:/@host.docker.internal:/' \
    -e 's/@127\.0\.0\.1:/@host.docker.internal:/')

docker rm -f "$container_name" >/dev/null 2>&1 || true

docker run \
    --detach \
    --platform linux/amd64 \
    --name "$container_name" \
    --log-opt max-size=10m \
    --log-opt max-file=5 \
    --env-file "$env_file" \
    --env "DATABASE_URL=$container_database_url" \
    --mount "type=bind,source=$env_file,target=/app/.env,readonly" \
    --publish "$http_port:8888" \
    "$image_name"
