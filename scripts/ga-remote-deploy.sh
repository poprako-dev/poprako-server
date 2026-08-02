#!/usr/bin/env sh
set -eu

if [ "$#" -ne 7 ]; then
    echo "expected image, tag, container, root, port, bind host, and network" >&2
    exit 1
fi

image_name=$1
image_tag=$2
container_name=$3
deploy_root=$4
public_port=$5
bind_host=$6
docker_network=$7

image_ref="${image_name}:${image_tag}"
release_sha=${image_tag#sha-}
release_dir="${deploy_root}/releases/${release_sha}"
runtime_env_file="${deploy_root}/shared/runtime.env"
previous_env_file="${deploy_root}/shared/runtime.env.previous"
uploaded_env_file="${release_dir}/poprako-runtime.env"
image_archive="${release_dir}/${image_name}-${image_tag}.tar.gz"
previous_name="${container_name}-previous"
rollback_required=0

container_exists() {
    docker container inspect "$1" >/dev/null 2>&1
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

    if [ -f "$previous_env_file" ]; then
        cp "$previous_env_file" "$runtime_env_file"
        chmod 600 "$runtime_env_file"
    else
        rm -f "$runtime_env_file"
    fi

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

on_exit() {
    exit_status=$?

    trap - EXIT HUP INT TERM

    if [ "$rollback_required" = "1" ]; then
        restore_previous || true
    fi

    exit "$exit_status"
}

trap on_exit EXIT
trap 'exit 1' HUP INT TERM

[ -f "$uploaded_env_file" ] || {
    echo "missing uploaded runtime environment: $uploaded_env_file" >&2
    exit 1
}

[ -f "$image_archive" ] || {
    echo "missing uploaded image archive: $image_archive" >&2
    exit 1
}

docker info >/dev/null
docker network inspect "$docker_network" >/dev/null
docker load --input "$image_archive"
rm -f "$image_archive"

if [ -f "$runtime_env_file" ]; then
    cp "$runtime_env_file" "$previous_env_file"
    chmod 600 "$previous_env_file"
fi

mv "$uploaded_env_file" "$runtime_env_file"
chmod 600 "$runtime_env_file"

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
    --env-file "$runtime_env_file" \
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

rollback_required=0

image_id=$(docker image inspect --format '{{.Id}}' "$image_ref")

printf 'deployed_commit=%s\n' "$release_sha"
printf 'deployed_image=%s\n' "$image_ref"
printf 'deployed_image_id=%s\n' "$image_id"
docker ps \
    --filter "name=^/${container_name}$" \
    --format '{{.Names}} {{.Status}} {{.Ports}}'
