#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

image_name="${IMAGE_NAME:-poprako-sr-prod:latest}"
container_name="${CONTAINER_NAME:-poprako-sr-prod}"
env_file="${ENV_FILE:-$project_root/.env}"
http_port="${HTTP_PORT:-18888}"

IMAGE_NAME="$image_name" "$project_root/scripts/docker-build-prod.sh"

IMAGE_NAME="$image_name" \
CONTAINER_NAME="$container_name" \
ENV_FILE="$env_file" \
HTTP_PORT="$http_port" \
"$project_root/scripts/docker-run-prod.sh"

health_ok=0
health_attempt=1

while [ "$health_attempt" -le 30 ]; do
    if docker exec "$container_name" \
        wget -q -O /dev/null http://127.0.0.1:8888/api/health; then
        health_ok=1
        break
    fi

    health_attempt=$((health_attempt + 1))
    sleep 1
done

if [ "$health_ok" != "1" ]; then
    echo "container health check did not become ready" >&2
    exit 1
fi

route_ok=0
route_attempt=1

while [ "$route_attempt" -le 30 ]; do
    status=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$http_port/api/v1/users/me")

    if [ "$status" = "401" ]; then
        route_ok=1
        break
    fi

    route_attempt=$((route_attempt + 1))
    sleep 1
done

if [ "$route_ok" != "1" ]; then
    echo "unexpected container smoke status: $status" >&2
    exit 1
fi

docker ps --filter "name=$container_name" --format '{{.Names}} {{.Status}} {{.Ports}}'
