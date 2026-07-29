#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

image_name="${IMAGE_NAME:-poprako-server-prod:latest}"
platform="${PLATFORM:-linux/amd64}"

if [ "${PUSH:-0}" = "1" ]; then
    output_flag="--push"
else
    output_flag="--load"
fi

docker buildx build \
    --ulimit nofile=65536 \
    --platform "$platform" \
    --tag "$image_name" \
    "$output_flag" \
    "$project_root"
