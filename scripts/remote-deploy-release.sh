#!/usr/bin/env sh
set -eu

image_name=${IMAGE_NAME:-poprako-server-prod}
image_tag=${IMAGE_TAG:?IMAGE_TAG is required}
container_name=${CONTAINER_NAME:-poprako-server-prod}
deploy_root=${DEPLOY_ROOT:-/opt/poprako-server-prod}
public_port=${PUBLIC_PORT:-8080}
docker_network=${DOCKER_NETWORK:-docker_default}
legacy_env_file=${LEGACY_ENV_FILE:-/opt/poprako-s/shared/.env}

release_dir="${deploy_root}/releases/${image_tag}"
shared_dir="${deploy_root}/shared"
source_env_file="${release_dir}/runtime.env"
runtime_env_file="${shared_dir}/.env"
image_archive="${release_dir}/${image_name}-${image_tag}.tar.gz"

read_env() {
    env_file=$1
    key=$2
    value=$(sed -n "s/^${key}=//p" "$env_file" | tail -n 1)

    [ -n "$value" ] || {
        echo "Missing required key '${key}' in ${env_file}" >&2
        exit 1
    }

    printf '%s' "$value"
}

read_optional_env() {
    env_file=$1
    key=$2

    sed -n "s/^${key}=//p" "$env_file" | tail -n 1
}

preserve_production_env() {
    key=$1
    production_value=$(read_optional_env "$legacy_env_file" "$key")

    [ -n "$production_value" ] || return 0

    next_env_file=$(mktemp)
    grep -v "^${key}=" "$tmp_env_file" >"$next_env_file" || true
    printf '%s=%s\n' "$key" "$production_value" >>"$next_env_file"
    mv "$next_env_file" "$tmp_env_file"
}

[ -f "$source_env_file" ] || {
    echo "Missing uploaded runtime env: $source_env_file" >&2
    exit 1
}

[ -f "$legacy_env_file" ] || {
    echo "Missing legacy production env: $legacy_env_file" >&2
    exit 1
}

[ -f "$image_archive" ] || {
    echo "Missing image archive: $image_archive" >&2
    exit 1
}

database_user=$(read_env "$legacy_env_file" DATABASE_USER)
database_password=$(read_env "$legacy_env_file" DATABASE_PASSWORD)

mkdir -p "$shared_dir"

tmp_env_file=$(mktemp)
trap 'rm -f "$tmp_env_file"' EXIT INT TERM HUP

grep -v '^DATABASE_URL=' "$source_env_file" >"$tmp_env_file" || true

for env_key in \
    JWT_SECRET \
    JWT_EXPIRATION_HOURS \
    OSS_PLATFORM \
    R2_ACCOUNT_ID \
    R2_ACCESS_KEY_ID \
    R2_SECRET_ACCESS_KEY \
    R2_BUCKET_NAME \
    R2_REGION \
    R2_CUSTOM_DOMAIN \
    ALIYUN_OSS_ACCESS_KEY_ID \
    ALIYUN_OSS_ACCESS_KEY_SECRET \
    ALIYUN_OSS_REGION \
    ALIYUN_OSS_BUCKET_NAME \
    ALIYUN_OSS_ENDPOINT \
    ALIYUN_OSS_CUSTOM_DOMAIN; do
    preserve_production_env "$env_key"
done

printf 'DATABASE_URL=postgres://%s:%s@prod-postgres:5432/db_poprako_server_prod\n' \
    "$database_user" "$database_password" >>"$tmp_env_file"

mv "$tmp_env_file" "$runtime_env_file"
trap - EXIT INT TERM HUP
chmod 600 "$runtime_env_file"

printf '%s\n' "Generated production runtime dotenv with preserved production secrets"

docker network inspect "$docker_network" >/dev/null
docker load -i "$image_archive"
docker rm -f "$container_name" >/dev/null 2>&1 || true

docker run \
    --detach \
    --restart unless-stopped \
    --name "$container_name" \
    --network "$docker_network" \
    --env-file "$runtime_env_file" \
    --mount "type=bind,source=$runtime_env_file,target=/app/.env,readonly" \
    --publish "${public_port}:8888" \
    "${image_name}:${image_tag}"

health_attempt=1

while [ "$health_attempt" -le 30 ]; do
    if docker exec "$container_name" \
        wget -q -O /dev/null http://127.0.0.1:8888/api/health; then
        docker ps --filter "name=^/${container_name}$" --format '{{.Names}} {{.Status}} {{.Ports}}'
        exit 0
    fi

    health_attempt=$((health_attempt + 1))
    sleep 1
done

docker logs --tail 120 "$container_name" >&2 || true
exit 1
