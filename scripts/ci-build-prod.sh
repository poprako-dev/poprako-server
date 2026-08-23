#!/usr/bin/env sh
set -eu

image_name=${IMAGE_NAME:?IMAGE_NAME is required}
cache_from=${CACHE_FROM:?CACHE_FROM is required}
cache_to=${CACHE_TO:?CACHE_TO is required}
metadata_file=${BUILD_METADATA_FILE:?BUILD_METADATA_FILE is required}
ghcr_username=${GHCR_USERNAME:?GHCR_USERNAME is required}
ghcr_token=${GHCR_TOKEN:?GHCR_TOKEN is required}
github_output=${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}
runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
image_repository=ghcr.io/poprako-dev/poprako-server
cache_ref="${image_repository}:buildcache"

case "$image_name" in
    "${image_repository}:sha-"*)
        image_sha=${image_name#"${image_repository}:sha-"}
        ;;
    *)
        echo "IMAGE_NAME must identify the PopRaKo GHCR image with a sha- tag" >&2
        exit 1
        ;;
esac

[ "${#image_sha}" -eq 40 ] || {
    echo "IMAGE_NAME sha tag must contain exactly 40 characters" >&2
    exit 1
}

case "$image_sha" in
    *[!0-9a-f]*)
        echo "IMAGE_NAME sha tag must be lowercase hexadecimal" >&2
        exit 1
        ;;
esac

[ "$cache_from" = "type=registry,ref=${cache_ref}" ] || {
    echo "CACHE_FROM must identify the PopRaKo GHCR build cache" >&2
    exit 1
}

case "$cache_to" in
    "type=registry,ref=${cache_ref},"*) ;;
    *)
        echo "CACHE_TO must identify the PopRaKo GHCR build cache" >&2
        exit 1
        ;;
esac

case "$ghcr_username" in
    "" | *[[:space:]]*)
        echo "GHCR_USERNAME must not be empty or contain whitespace" >&2
        exit 1
        ;;
esac

command -v docker >/dev/null 2>&1 || {
    echo "docker is required" >&2
    exit 1
}

command -v jq >/dev/null 2>&1 || {
    echo "jq is required" >&2
    exit 1
}

umask 077
docker_config=$(mktemp -d "${runner_temp}/poprako-ghcr.XXXXXX")
logged_in=0

cleanup() {
    exit_status=$?

    trap - EXIT INT TERM

    if [ "$logged_in" = "1" ]; then
        docker logout ghcr.io >/dev/null 2>&1 || true
    fi

    rm -rf "$docker_config"

    exit "$exit_status"
}

trap cleanup EXIT
trap 'exit 1' INT TERM

export DOCKER_CONFIG=$docker_config

printf '%s\n' "$ghcr_token" | docker login ghcr.io \
    --username "$ghcr_username" \
    --password-stdin
logged_in=1

export PLATFORM="${PLATFORM:-linux/amd64}"
export PUSH="${PUSH:-1}"

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

"$project_root/scripts/docker-build-prod.sh"

image_digest=$(jq -er '."containerimage.digest"' "$metadata_file")

case "$image_digest" in
    sha256:*)
        digest_hex=${image_digest#sha256:}
        ;;
    *)
        echo "Buildx metadata did not contain a sha256 image digest" >&2
        exit 1
        ;;
esac

[ "${#digest_hex}" -eq 64 ] || {
    echo "Buildx image digest must contain exactly 64 hexadecimal characters" >&2
    exit 1
}

case "$digest_hex" in
    *[!0-9a-f]*)
        echo "Buildx image digest must be lowercase hexadecimal" >&2
        exit 1
        ;;
esac

printf 'image_digest=%s\n' "$image_digest" >>"$github_output"
printf 'image_ref=%s@%s\n' "$image_repository" "$image_digest" >>"$github_output"
