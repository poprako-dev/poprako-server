#!/usr/bin/env sh
set -eu

value=${1:?R2 custom domain is required}

case "$value" in
    '<'*'>')
        value=${value#'<'}
        value=${value%'>'}
        ;;
esac

case "$value" in
    *[\<\>]*)
        echo "R2_CUSTOM_DOMAIN contains invalid characters" >&2
        exit 1
        ;;
esac

case "$value" in
    https://*) ;;
    http://*)
        value="https://${value#http://}"
        ;;
    *://*)
        echo "R2_CUSTOM_DOMAIN must use https" >&2
        exit 1
        ;;
    *)
        value="https://${value}"
        ;;
esac

[ "$value" != "https://" ] || {
    echo "R2_CUSTOM_DOMAIN must identify a host" >&2
    exit 1
}

printf '%s\n' "$value"
