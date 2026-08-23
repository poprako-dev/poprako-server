#!/usr/bin/env sh
set -eu

if [ "$#" -ne 9 ]; then
    echo "expected source image, local image, tag, container, root, port, bind host, network, and PostgreSQL container" >&2
    exit 1
fi

source_image_ref=$1
image_name=$2
image_tag=$3
container_name=$4
deploy_root=$5
public_port=$6
bind_host=$7
docker_network=$8
postgres_container=$9

image_ref="${image_name}:${image_tag}"
release_sha=${image_tag#sha-}
release_dir="${deploy_root}/releases/${release_sha}"
legacy_runtime_env_file="${deploy_root}/shared/runtime.env"
legacy_previous_env_file="${deploy_root}/shared/runtime.env.previous"
migration_script="${release_dir}/ga-apply-migrations.sh"
migration_root="${release_dir}/migrations"
previous_name="${container_name}-previous"
rollback_required=0
registry_authenticated=0
registry_config_dir=
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

validate_source_image() {
    case "$source_image_ref" in
        "${source_image_repository}@sha256:"*)
            source_digest=${source_image_ref#"${source_image_repository}@sha256:"}
            ;;
        *)
            echo "source image must identify the PopRaKo GHCR image by digest" >&2
            exit 1
            ;;
    esac

    [ "${#source_digest}" -eq 64 ] || {
        echo "source image digest must contain exactly 64 characters" >&2
        exit 1
    }

    case "$source_digest" in
        *[!0-9a-f]*)
            echo "source image digest must be lowercase hexadecimal" >&2
            exit 1
            ;;
    esac
}

validate_deploy_root() {
    case "$deploy_root" in
        / | /opt | /var | /srv)
            echo "deployment root must identify a dedicated application directory" >&2
            exit 1
            ;;
        /*) ;;
        *)
            echo "deployment root must be absolute" >&2
            exit 1
            ;;
    esac

    case "$deploy_root" in
        *[!A-Za-z0-9_./-]*)
            echo "deployment root contains unsupported characters" >&2
            exit 1
            ;;
    esac
}

validate_source_image
validate_simple_value "$image_name" image
validate_simple_value "$container_name" container
validate_simple_value "$bind_host" "bind host"
validate_simple_value "$docker_network" network
validate_simple_value "$postgres_container" "PostgreSQL container"
validate_port "$public_port" port
validate_deploy_root

case "$image_tag" in
    sha-*) ;;
    *)
        echo "image tag must start with sha-" >&2
        exit 1
        ;;
esac

[ "${#release_sha}" -eq 40 ] || {
    echo "image tag commit must contain exactly 40 characters" >&2
    exit 1
}

case "$release_sha" in
    *[!0-9a-f]*)
        echo "image tag commit must be lowercase hexadecimal" >&2
        exit 1
        ;;
esac

read_runtime_value() {
    label=$1

    if ! IFS= read -r runtime_value; then
        echo "missing runtime value for $label" >&2
        exit 1
    fi

    if [ -z "$runtime_value" ]; then
        echo "empty runtime value for $label" >&2
        exit 1
    fi

    case "$runtime_value" in
        *[[:space:]]*)
            echo "$label must not contain whitespace" >&2
            exit 1
            ;;
    esac
}

read_runtime_value GHCR_USERNAME
ghcr_username=$runtime_value
read_runtime_value GHCR_TOKEN
ghcr_token=$runtime_value
read_runtime_value DATABASE_URL
database_url=$runtime_value
read_runtime_value JWT_SECRET
jwt_secret=$runtime_value
read_runtime_value JWT_EXPIRATION_HOURS
jwt_expiration_hours=$runtime_value
read_runtime_value R2_ACCOUNT_ID
r2_account_id=$runtime_value
read_runtime_value R2_ACCESS_KEY_ID
r2_access_key_id=$runtime_value
read_runtime_value R2_SECRET_ACCESS_KEY
r2_secret_access_key=$runtime_value
read_runtime_value R2_BUCKET_NAME
r2_bucket_name=$runtime_value
read_runtime_value R2_REGION
r2_region=$runtime_value
read_runtime_value R2_CUSTOM_DOMAIN
r2_custom_domain=$runtime_value
read_runtime_value POPRAKO_SNOWFLAKE_NODE_ID
snowflake_node_id=$runtime_value

export DATABASE_URL=$database_url
export JWT_SECRET=$jwt_secret
export JWT_EXPIRATION_HOURS=$jwt_expiration_hours
export R2_ACCOUNT_ID=$r2_account_id
export R2_ACCESS_KEY_ID=$r2_access_key_id
export R2_SECRET_ACCESS_KEY=$r2_secret_access_key
export R2_BUCKET_NAME=$r2_bucket_name
export R2_REGION=$r2_region
export R2_CUSTOM_DOMAIN=$r2_custom_domain
export POPRAKO_SNOWFLAKE_NODE_ID=$snowflake_node_id

container_exists() {
    docker container inspect "$1" >/dev/null 2>&1
}

is_commit_sha() {
    candidate_sha=$1

    [ "${#candidate_sha}" -eq 40 ] || return 1

    case "$candidate_sha" in
        *[!0-9a-f]*) return 1 ;;
    esac
}

cleanup_release_directories() {
    retained_release_sha=$1

    for cleanup_dir in "${deploy_root}/releases/"*; do
        [ -d "$cleanup_dir" ] || continue

        cleanup_sha=${cleanup_dir##*/}
        is_commit_sha "$cleanup_sha" || continue

        case "$cleanup_sha" in
            "$release_sha" | "$retained_release_sha") continue ;;
        esac

        rm -rf "$cleanup_dir"
    done
}

cleanup_application_images() {
    retained_image_ref=$1

    docker image ls \
        --format '{{.Repository}}:{{.Tag}}' \
        "$image_name" |
        while IFS= read -r cleanup_image_ref; do
            case "$cleanup_image_ref" in
                "$image_ref" | "$retained_image_ref") continue ;;
                "${image_name}:sha-"*) ;;
                *) continue ;;
            esac

            cleanup_image_sha=${cleanup_image_ref#"${image_name}:sha-"}
            is_commit_sha "$cleanup_image_sha" || continue

            docker image rm "$cleanup_image_ref" >/dev/null
        done
}

verify_post_deployment() {
    metrics_text=$(docker exec "$container_name" \
        wget -q -O - \
        http://127.0.0.1:8888/api/health/detailed-metrics)

    printf '%s\n' "$metrics_text" | grep -q '^http_requests_total'
    printf '%s\n' "$metrics_text" | grep -q '^http_responses_total'

    startup_logs=$(docker logs --since 5m "$container_name" 2>&1)

    if printf '%s\n' "$startup_logs" | grep -E -q \
        '(^|[[:space:]])ERROR([[:space:]]|$)|panicked at|fatal runtime error'; then
        echo "new release emitted an error during startup" >&2
        printf '%s\n' "$startup_logs" >&2
        return 1
    fi
}

wait_for_health() {
    target_container=$1
    health_attempt=1

    while [ "$health_attempt" -le 60 ]; do
        health_status=$(docker container inspect \
            --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' \
            "$target_container")

        case "$health_status" in
            healthy)
                return 0
                ;;
            unhealthy)
                return 1
                ;;
            missing)
                if docker exec "$target_container" \
                    wget -q -O /dev/null \
                    http://127.0.0.1:8888/api/health; then
                    return 0
                fi
                ;;
        esac

        health_attempt=$((health_attempt + 1))
        sleep 2
    done

    return 1
}

restore_previous() {
    rollback_required=0

    if container_exists "$previous_name"; then
        docker rm -f "$container_name" >/dev/null 2>&1 || true
        docker rename "$previous_name" "$container_name"
        docker start "$container_name" >/dev/null 2>&1 || true

        if wait_for_health "$container_name"; then
            echo "previous release restored" >&2
            return 0
        fi

        echo "previous release was restarted but did not become healthy" >&2
        return 1
    fi

    if container_exists "$container_name"; then
        current_image=$(docker container inspect \
            --format '{{.Config.Image}}' \
            "$container_name")

        if [ "$current_image" != "$image_ref" ]; then
            docker start "$container_name" >/dev/null 2>&1 || true

            if wait_for_health "$container_name"; then
                echo "previous release restored" >&2
                return 0
            fi
        else
            docker rm -f "$container_name" >/dev/null 2>&1 || true
        fi
    fi

    echo "deployment failed and no healthy previous container exists" >&2
    return 1
}

cleanup_registry_auth() {
    if [ -z "$registry_config_dir" ]; then
        return
    fi

    if [ "$registry_authenticated" = "1" ]; then
        docker logout ghcr.io >/dev/null 2>&1 || true
    fi

    rm -rf "$registry_config_dir"
    registry_authenticated=0
    registry_config_dir=
    unset DOCKER_CONFIG
}

on_exit() {
    exit_status=$?

    trap - EXIT HUP INT TERM

    cleanup_registry_auth

    if [ "$rollback_required" = "1" ]; then
        restore_previous || true
    fi

    exit "$exit_status"
}

trap on_exit EXIT
trap 'exit 1' HUP INT TERM

[ -f "$migration_script" ] || {
    echo "missing migration script: $migration_script" >&2
    exit 1
}

[ -d "$migration_root" ] || {
    echo "missing migration directory: $migration_root" >&2
    exit 1
}

docker info >/dev/null
docker network inspect "$docker_network" >/dev/null
umask 077
registry_config_dir=$(mktemp -d)
export DOCKER_CONFIG=$registry_config_dir

printf '%s\n' "$ghcr_token" | docker login ghcr.io \
    --username "$ghcr_username" \
    --password-stdin
registry_authenticated=1

docker pull "$source_image_ref"

pulled_repo_digests=$(docker image inspect \
    --format '{{range .RepoDigests}}{{println .}}{{end}}' \
    "$source_image_ref")

if ! printf '%s\n' "$pulled_repo_digests" | grep -F -x -q "$source_image_ref"; then
    echo "pulled image does not contain the requested digest" >&2
    exit 1
fi

docker tag "$source_image_ref" "$image_ref"
docker image inspect "$image_ref" >/dev/null
docker image rm "$source_image_ref" >/dev/null
docker image inspect "$image_ref" >/dev/null

docker logout ghcr.io >/dev/null 2>&1 || true
registry_authenticated=0
rm -rf "$registry_config_dir"
registry_config_dir=
unset ghcr_token
unset DOCKER_CONFIG

sh "$migration_script" "$migration_root" "$postgres_container"

if container_exists "$previous_name"; then
    docker rm -f "$previous_name" >/dev/null
fi

if container_exists "$container_name"; then
    rollback_required=1
    docker stop --time 30 "$container_name" >/dev/null
    docker rename "$container_name" "$previous_name"
fi

rollback_required=1

if ! docker run \
    --detach \
    --restart unless-stopped \
    --name "$container_name" \
    --network "$docker_network" \
    --env DATABASE_URL \
    --env JWT_SECRET \
    --env JWT_EXPIRATION_HOURS \
    --env R2_ACCOUNT_ID \
    --env R2_ACCESS_KEY_ID \
    --env R2_SECRET_ACCESS_KEY \
    --env R2_BUCKET_NAME \
    --env R2_REGION \
    --env R2_CUSTOM_DOMAIN \
    --env POPRAKO_SNOWFLAKE_NODE_ID \
    --log-opt max-size=10m \
    --log-opt max-file=5 \
    --publish "${bind_host}:${public_port}:8888" \
    "$image_ref" >/dev/null; then
    exit 1
fi

if ! wait_for_health "$container_name"; then
    docker logs --tail 120 "$container_name" >&2 || true
    exit 1
fi

if ! verify_post_deployment; then
    echo "post-deployment log or metric verification failed" >&2
    exit 1
fi

rollback_required=0

rm -f "$legacy_runtime_env_file" "$legacy_previous_env_file"

previous_image_ref=
previous_release_sha=

if container_exists "$previous_name"; then
    previous_image_ref=$(docker container inspect \
        --format '{{.Config.Image}}' \
        "$previous_name")

    case "$previous_image_ref" in
        "${image_name}:sha-"*)
            previous_release_sha=${previous_image_ref#"${image_name}:sha-"}

            if ! is_commit_sha "$previous_release_sha"; then
                previous_image_ref=
                previous_release_sha=
            fi
            ;;
        *)
            previous_image_ref=
            ;;
    esac
fi

cleanup_release_directories "$previous_release_sha"
cleanup_application_images "$previous_image_ref"

image_id=$(docker image inspect --format '{{.Id}}' "$image_ref")

printf 'deployed_commit=%s\n' "$release_sha"
printf 'deployed_source_image=%s\n' "$source_image_ref"
printf 'deployed_image=%s\n' "$image_ref"
printf 'deployed_image_id=%s\n' "$image_id"
docker ps \
    --filter "name=^/${container_name}$" \
    --format '{{.Names}} {{.Status}} {{.Ports}}'
