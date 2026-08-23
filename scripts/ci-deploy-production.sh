#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}

deploy_host=${DEPLOY_HOST:?DEPLOY_HOST is required}
deploy_user=${DEPLOY_USER:?DEPLOY_USER is required}
deploy_private_key=${DEPLOY_SSH_PRIVATE_KEY:?DEPLOY_SSH_PRIVATE_KEY is required}
deploy_known_hosts=${DEPLOY_KNOWN_HOSTS:?DEPLOY_KNOWN_HOSTS is required}
deploy_sha=${DEPLOY_SHA:?DEPLOY_SHA is required}
deploy_source_image=${DEPLOY_SOURCE_IMAGE:?DEPLOY_SOURCE_IMAGE is required}
deploy_port=${DEPLOY_PORT:?DEPLOY_PORT is required}
deploy_root=${DEPLOY_ROOT:?DEPLOY_ROOT is required}
public_port=${DEPLOY_PUBLIC_PORT:?DEPLOY_PUBLIC_PORT is required}
bind_host=${DEPLOY_BIND_HOST:?DEPLOY_BIND_HOST is required}
docker_network=${DEPLOY_DOCKER_NETWORK:?DEPLOY_DOCKER_NETWORK is required}
postgres_container=${DEPLOY_POSTGRES_CONTAINER:?DEPLOY_POSTGRES_CONTAINER is required}
ghcr_username=${GHCR_USERNAME:?GHCR_USERNAME is required}
ghcr_token=${GHCR_TOKEN:?GHCR_TOKEN is required}
database_url=${DATABASE_URL:?DATABASE_URL is required}
jwt_expiration_hours=${JWT_EXPIRATION_HOURS:?JWT_EXPIRATION_HOURS is required}
jwt_secret=${JWT_SECRET:?JWT_SECRET is required}
snowflake_node_id=${POPRAKO_SNOWFLAKE_NODE_ID:?POPRAKO_SNOWFLAKE_NODE_ID is required}
r2_access_key_id=${R2_ACCESS_KEY_ID:?R2_ACCESS_KEY_ID is required}
r2_account_id=${R2_ACCOUNT_ID:?R2_ACCOUNT_ID is required}
r2_bucket_name=${R2_BUCKET_NAME:?R2_BUCKET_NAME is required}
r2_custom_domain=${R2_CUSTOM_DOMAIN:?R2_CUSTOM_DOMAIN is required}
r2_region=${R2_REGION:?R2_REGION is required}
r2_secret_access_key=${R2_SECRET_ACCESS_KEY:?R2_SECRET_ACCESS_KEY is required}
container_name=poprako-server-prod
image_name=poprako-server-prod
source_image_repository=ghcr.io/poprako-dev/poprako-server

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

validate_runtime_value() {
    value=$1
    label=$2

    case "$value" in
        "" | *[[:space:]]*)
            echo "$label must not be empty or contain whitespace" >&2
            exit 1
            ;;
    esac
}

validate_source_image() {
    case "$deploy_source_image" in
        "${source_image_repository}@sha256:"*)
            source_digest=${deploy_source_image#"${source_image_repository}@sha256:"}
            ;;
        *)
            echo "DEPLOY_SOURCE_IMAGE must identify the PopRaKo GHCR image by digest" >&2
            exit 1
            ;;
    esac

    [ "${#source_digest}" -eq 64 ] || {
        echo "DEPLOY_SOURCE_IMAGE digest must contain exactly 64 characters" >&2
        exit 1
    }

    case "$source_digest" in
        *[!0-9a-f]*)
            echo "DEPLOY_SOURCE_IMAGE digest must be lowercase hexadecimal" >&2
            exit 1
            ;;
    esac
}

stream_runtime_values() {
    printf '%s\n' \
        "$ghcr_username" \
        "$ghcr_token" \
        "$database_url" \
        "$jwt_secret" \
        "$jwt_expiration_hours" \
        "$r2_account_id" \
        "$r2_access_key_id" \
        "$r2_secret_access_key" \
        "$r2_bucket_name" \
        "$r2_region" \
        "$r2_custom_domain" \
        "$snowflake_node_id"
}

validate_simple_value "$deploy_host" DEPLOY_HOST
validate_simple_value "$deploy_user" DEPLOY_USER
validate_simple_value "$bind_host" DEPLOY_BIND_HOST
validate_simple_value "$docker_network" DEPLOY_DOCKER_NETWORK
validate_simple_value "$postgres_container" DEPLOY_POSTGRES_CONTAINER
validate_port "$deploy_port" DEPLOY_PORT
validate_port "$public_port" DEPLOY_PUBLIC_PORT
validate_runtime_value "$ghcr_username" GHCR_USERNAME
validate_runtime_value "$ghcr_token" GHCR_TOKEN
validate_runtime_value "$database_url" DATABASE_URL
validate_runtime_value "$jwt_expiration_hours" JWT_EXPIRATION_HOURS
validate_runtime_value "$jwt_secret" JWT_SECRET
validate_runtime_value "$snowflake_node_id" POPRAKO_SNOWFLAKE_NODE_ID
validate_runtime_value "$r2_access_key_id" R2_ACCESS_KEY_ID
validate_runtime_value "$r2_account_id" R2_ACCOUNT_ID
validate_runtime_value "$r2_bucket_name" R2_BUCKET_NAME
validate_runtime_value "$r2_custom_domain" R2_CUSTOM_DOMAIN
validate_runtime_value "$r2_region" R2_REGION
validate_runtime_value "$r2_secret_access_key" R2_SECRET_ACCESS_KEY
validate_port "$jwt_expiration_hours" JWT_EXPIRATION_HOURS
validate_port "$snowflake_node_id" POPRAKO_SNOWFLAKE_NODE_ID
validate_source_image

database_name=${database_url##*/}
database_name=${database_name%%\?*}

case "$database_name" in
    db_poprako_server_prod) ;;
    *)
        echo "DATABASE_URL must target db_poprako_server_prod" >&2
        exit 1
        ;;
esac

case "$r2_account_id" in
    *[!0-9A-Fa-f]*)
        echo "R2_ACCOUNT_ID must contain only hexadecimal characters" >&2
        exit 1
        ;;
esac

[ "${#r2_account_id}" -eq 32 ] || {
    echo "R2_ACCOUNT_ID must contain exactly 32 characters" >&2
    exit 1
}

case "$r2_custom_domain" in
    https://*) ;;
    *)
        echo "R2_CUSTOM_DOMAIN must use https" >&2
        exit 1
        ;;
esac

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

[ "${#deploy_sha}" -eq 40 ] || {
    echo "DEPLOY_SHA must contain exactly 40 characters" >&2
    exit 1
}

image_tag="sha-${deploy_sha}"
release_dir="${deploy_root}/releases/${deploy_sha}"
umask 077
ssh_root=$(mktemp -d "${runner_temp}/poprako-deploy-ssh.XXXXXX")
private_key_file="${ssh_root}/id_ed25519"
known_hosts_file="${ssh_root}/known_hosts"
ssh_target="${deploy_user}@${deploy_host}"

cleanup() {
    exit_status=$?

    trap - EXIT INT TERM

    rm -f "$private_key_file" "$known_hosts_file"
    rmdir "$ssh_root" >/dev/null 2>&1 || true

    exit "$exit_status"
}

trap cleanup EXIT
trap 'exit 1' INT TERM

printf '%s\n' "$deploy_private_key" >"$private_key_file"
printf '%s\n' "$deploy_known_hosts" >"$known_hosts_file"

cd "$project_root"

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
    "migrations" \
    "scripts/ga-apply-migrations.sh" \
    "scripts/ga-remote-deploy.sh" \
    "${ssh_target}:${release_dir}/"

if ! stream_runtime_values | ssh \
    -i "$private_key_file" \
    -p "$deploy_port" \
    -o BatchMode=yes \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=yes \
    -o "UserKnownHostsFile=$known_hosts_file" \
    "$ssh_target" \
    "sh '$release_dir/ga-remote-deploy.sh' '$deploy_source_image' '$image_name' '$image_tag' '$container_name' '$deploy_root' '$public_port' '$bind_host' '$docker_network' '$postgres_container'"; then
    echo "::error title=Production deployment failed::Remote deployment or post-deployment verification failed for ${deploy_sha}"
    exit 1
fi
