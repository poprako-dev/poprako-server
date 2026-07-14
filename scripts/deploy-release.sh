#!/usr/bin/env sh
set -eu

image_name=${IMAGE_NAME:-poprako-server-prod}
image_tag=${IMAGE_TAG:-$(git rev-parse --short=12 HEAD)}
container_name=${CONTAINER_NAME:-poprako-server-prod}
deploy_root=${DEPLOY_ROOT:-/opt/poprako-server-prod}
source_env_file=${SOURCE_ENV_FILE:-.env}
target_platform=${TARGET_PLATFORM:-linux/amd64}
public_port=${PUBLIC_PORT:-8080}
docker_network=${DOCKER_NETWORK:-docker_default}
server_user=${SERVER_USER:?SERVER_USER is required}
server_host=${SERVER_HOST:?SERVER_HOST is required}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_root=$(CDPATH= cd -- "${script_dir}/.." && pwd)
release_dir="${deploy_root}/releases/${image_tag}"
dist_dir="${project_root}/dist"
image_archive="${dist_dir}/${image_name}-${image_tag}.tar.gz"
remote_script="${release_dir}/remote-deploy-release.sh"

case "$source_env_file" in
    /*) source_env_path="$source_env_file" ;;
    *) source_env_path="${project_root}/${source_env_file}" ;;
esac

[ -f "$source_env_path" ] || {
    echo "Missing source env file: $source_env_path" >&2
    exit 1
}

mkdir -p "$dist_dir"

IMAGE_NAME="${image_name}:${image_tag}" \
PLATFORM="$target_platform" \
PUSH=0 \
"${project_root}/scripts/docker-build-prod.sh"

docker save "${image_name}:${image_tag}" | gzip > "$image_archive"

ssh "${server_user}@${server_host}" "mkdir -p '${release_dir}' '${deploy_root}/shared'"
scp "$image_archive" "${server_user}@${server_host}:${release_dir}/"
scp "$source_env_path" "${server_user}@${server_host}:${release_dir}/runtime.env"
scp "${script_dir}/remote-deploy-release.sh" "${server_user}@${server_host}:${remote_script}"

ssh "${server_user}@${server_host}" \
    "chmod 755 '${remote_script}' && IMAGE_NAME='${image_name}' IMAGE_TAG='${image_tag}' CONTAINER_NAME='${container_name}' DEPLOY_ROOT='${deploy_root}' PUBLIC_PORT='${public_port}' DOCKER_NETWORK='${docker_network}' LEGACY_ENV_FILE='/opt/poprako-s/shared/.env' sh '${remote_script}'"
