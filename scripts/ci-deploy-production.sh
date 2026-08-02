#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}

deploy_host=${DEPLOY_HOST:?DEPLOY_HOST is required}
deploy_user=${DEPLOY_USER:?DEPLOY_USER is required}
deploy_private_key=${DEPLOY_SSH_PRIVATE_KEY:?DEPLOY_SSH_PRIVATE_KEY is required}
deploy_known_hosts=${DEPLOY_KNOWN_HOSTS:?DEPLOY_KNOWN_HOSTS is required}
deploy_runtime_env=${DEPLOY_RUNTIME_ENV:?DEPLOY_RUNTIME_ENV is required}
deploy_sha=${DEPLOY_SHA:?DEPLOY_SHA is required}
deploy_port=${DEPLOY_PORT:?DEPLOY_PORT is required}
deploy_root=${DEPLOY_ROOT:?DEPLOY_ROOT is required}
public_port=${DEPLOY_PUBLIC_PORT:?DEPLOY_PUBLIC_PORT is required}
bind_host=${DEPLOY_BIND_HOST:?DEPLOY_BIND_HOST is required}
docker_network=${DEPLOY_DOCKER_NETWORK:?DEPLOY_DOCKER_NETWORK is required}
postgres_container=${DEPLOY_POSTGRES_CONTAINER:?DEPLOY_POSTGRES_CONTAINER is required}
container_name=poprako-server-prod
image_name=poprako-server-prod

validate_simple_value() {
    value=$1
    label=$2

    case "$value" in
        "" | *[!A-Za-z0-9._-]*)
            echo "$label contains unsupported characters" >&2
            exit 1
            ;;
    esac
}

validate_port() {
    value=$1
    label=$2

    case "$value" in
        "" | *[!0-9]*)
            echo "$label must be numeric" >&2
            exit 1
            ;;
    esac
}

validate_simple_value "$deploy_host" DEPLOY_HOST
validate_simple_value "$deploy_user" DEPLOY_USER
validate_simple_value "$bind_host" DEPLOY_BIND_HOST
validate_simple_value "$docker_network" DEPLOY_DOCKER_NETWORK
validate_simple_value "$postgres_container" DEPLOY_POSTGRES_CONTAINER
validate_port "$deploy_port" DEPLOY_PORT
validate_port "$public_port" DEPLOY_PUBLIC_PORT

case "$deploy_root" in
    / | /opt | /var | /srv)
        echo "DEPLOY_ROOT must identify a dedicated application directory" >&2
        exit 1
        ;;
    /*) ;;
    *)
        echo "DEPLOY_ROOT must be an absolute path" >&2
        exit 1
        ;;
esac

case "$deploy_root" in
    *[!A-Za-z0-9_./-]*)
        echo "DEPLOY_ROOT contains unsupported characters" >&2
        exit 1
        ;;
esac

case "$deploy_sha" in
    *[!0-9a-f]*)
        echo "DEPLOY_SHA must be a lowercase hexadecimal commit SHA" >&2
        exit 1
        ;;
esac

if [ "${#deploy_sha}" -ne 40 ]; then
    echo "DEPLOY_SHA must contain exactly 40 characters" >&2
    exit 1
fi

image_tag="sha-${deploy_sha}"
image_ref="${image_name}:${image_tag}"
release_dir="${deploy_root}/releases/${deploy_sha}"
image_tar="${runner_temp}/${image_name}-${image_tag}.tar"
image_archive="${image_tar}.gz"
ssh_root="${runner_temp}/poprako-deploy-ssh"
private_key_file="${ssh_root}/id_ed25519"
known_hosts_file="${ssh_root}/known_hosts"
runtime_env_file="${runner_temp}/poprako-runtime.env"
ssh_target="${deploy_user}@${deploy_host}"

cleanup() {
    rm -f \
        "$private_key_file" \
        "$known_hosts_file" \
        "$runtime_env_file" \
        "$image_tar" \
        "$image_archive"
    rmdir "$ssh_root" >/dev/null 2>&1 || true
}

trap cleanup EXIT
trap 'exit 1' INT TERM

umask 077
mkdir -p "$ssh_root"
printf '%s\n' "$deploy_private_key" >"$private_key_file"
printf '%s\n' "$deploy_known_hosts" >"$known_hosts_file"
printf '%s\n' "$deploy_runtime_env" >"$runtime_env_file"

cd "$project_root"

IMAGE_NAME="$image_ref" \
PLATFORM=linux/amd64 \
PUSH=0 \
sh scripts/docker-build-prod.sh

docker save --output "$image_tar" "$image_ref"
gzip -f "$image_tar"

ssh \
    -i "$private_key_file" \
    -p "$deploy_port" \
    -o BatchMode=yes \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=yes \
    -o "UserKnownHostsFile=$known_hosts_file" \
    "$ssh_target" \
    "mkdir -p '$release_dir' '$deploy_root/shared'"

scp \
    -r \
    -i "$private_key_file" \
    -P "$deploy_port" \
    -o BatchMode=yes \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=yes \
    -o "UserKnownHostsFile=$known_hosts_file" \
    "$image_archive" \
    "$runtime_env_file" \
    "migrations" \
    "scripts/ga-apply-migrations.sh" \
    "scripts/ga-remote-deploy.sh" \
    "${ssh_target}:${release_dir}/"

if ! ssh \
    -i "$private_key_file" \
    -p "$deploy_port" \
    -o BatchMode=yes \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=yes \
    -o "UserKnownHostsFile=$known_hosts_file" \
    "$ssh_target" \
    "sh '$release_dir/ga-remote-deploy.sh' '$image_name' '$image_tag' '$container_name' '$deploy_root' '$public_port' '$bind_host' '$docker_network' '$postgres_container'"; then
    echo "::error title=Production deployment failed::Remote deployment or post-deployment verification failed for ${deploy_sha}"
    exit 1
fi
