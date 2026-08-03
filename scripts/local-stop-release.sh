#!/usr/bin/env sh
set -eu

container_name="${CONTAINER_NAME:-poprako-server-prod}"

docker rm -f "$container_name" >/dev/null 2>&1 || true
