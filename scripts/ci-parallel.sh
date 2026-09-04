#!/usr/bin/env sh
set -u

# Run independent CI commands concurrently, then report every failure after all
# commands have finished. Arguments are pairs of a human-readable name and the
# shell command that performs that check.

if [ "$#" -eq 0 ] || [ $(( $# % 2 )) -ne 0 ]; then
    echo "usage: $0 CHECK_NAME CHECK_COMMAND [CHECK_NAME CHECK_COMMAND ...]" >&2
    exit 2
fi

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

job_root=$(mktemp -d "${TMPDIR:-/tmp}/poprako-ci-parallel.XXXXXX") || {
    echo "ci-parallel: could not create a temporary log directory" >&2
    exit 1
}
job_manifest="$job_root/jobs"

cleanup() {
    exit_status=$?

    trap - EXIT HUP INT TERM

    if [ -f "$job_manifest" ]; then
        while IFS='|' read -r job_pid _job_name _job_log; do
            kill "$job_pid" >/dev/null 2>&1 || true
        done <"$job_manifest"

        while IFS='|' read -r job_pid _job_name _job_log; do
            wait "$job_pid" >/dev/null 2>&1 || true
        done <"$job_manifest"
    fi

    rm -rf "$job_root"

    exit "$exit_status"
}

trap cleanup EXIT
trap 'exit 130' HUP INT TERM

job_index=0

while [ "$#" -gt 0 ]; do
    job_name=$1
    job_command=$2
    shift 2

    case "$job_name" in
        '' | *'|'*)
            echo "ci-parallel: check names must be non-empty and cannot contain |" >&2
            exit 2
            ;;
    esac

    if [ -z "$job_command" ]; then
        echo "ci-parallel: $job_name has an empty command" >&2
        exit 2
    fi

    job_index=$((job_index + 1))
    job_log="$job_root/$job_index.log"

    printf '━━━ ci: started %s ━━━\n' "$job_name"

    (
        exec sh -e -c "$job_command"
    ) >"$job_log" 2>&1 &

    job_pid=$!
    printf '%s|%s|%s\n' "$job_pid" "$job_name" "$job_log" >>"$job_manifest"
done

failed_jobs=0

while IFS='|' read -r job_pid job_name job_log; do
    if wait "$job_pid"; then
        printf '✓ ci: %s passed\n' "$job_name"
        continue
    fi

    failed_jobs=$((failed_jobs + 1))
    printf '✗ ci: %s failed\n' "$job_name" >&2

    if [ -s "$job_log" ]; then
        cat "$job_log" >&2
    fi
done <"$job_manifest"

if [ "$failed_jobs" -gt 0 ]; then
    printf '✗ ci: %s check(s) failed\n' "$failed_jobs" >&2
    exit 1
fi

printf '✓ ci: all %s check(s) passed\n' "$job_index"
