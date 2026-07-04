#!/usr/bin/env sh
set -eu

if [ -z "${IMAGE_NAME:-}" ]; then
    echo "IMAGE_NAME must be set, for example ghcr.io/owner/repo:sha-\$GITHUB_SHA" >&2
    exit 1
fi

export PLATFORM="${PLATFORM:-linux/amd64}"
export PUSH="${PUSH:-1}"

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

"$project_root/scripts/docker-build-prod.sh"
