#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
release_tag=${RELEASE_TAG:?RELEASE_TAG is required}
release_sha=${RELEASE_SHA:?RELEASE_SHA is required}
release_root=${RELEASE_DIR:?RELEASE_DIR is required}

case "$release_tag" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *)
        echo "RELEASE_TAG must be a vMAJOR.MINOR.PATCH tag" >&2
        exit 1
        ;;
esac

case "$release_tag" in
    *[!A-Za-z0-9._-]*)
        echo "RELEASE_TAG contains unsupported characters" >&2
        exit 1
        ;;
esac

case "$release_sha" in
    *[!0-9a-f]*)
        echo "RELEASE_SHA must be a lowercase hexadecimal commit SHA" >&2
        exit 1
        ;;
esac

[ "${#release_sha}" -eq 40 ] || {
    echo "RELEASE_SHA must contain exactly 40 characters" >&2
    exit 1
}

case "$release_root" in
    / | "")
        echo "RELEASE_DIR must identify a dedicated directory" >&2
        exit 1
        ;;
esac

artifact_name="poprako-server-${release_tag}-linux-amd64"
binary_file="${release_root}/poprako-server"
archive_file="${release_root}/${artifact_name}.tar.gz"
sbom_file="${release_root}/${artifact_name}.cargo-metadata.json"
provenance_file="${release_root}/${artifact_name}.provenance.json"

mkdir -p "$release_root"
cd "$project_root"

cargo build --locked --release --bin poprako-server
cp target/release/poprako-server "$binary_file"
tar -czf "$archive_file" -C "$release_root" poprako-server
rm -f "$binary_file"

cargo metadata --locked --format-version 1 >"$sbom_file"

rust_version=$(rustc --version)

printf '{\n  "source_commit": "%s",\n  "release_tag": "%s",\n  "rust_toolchain": "%s",\n  "builder": "github-actions"\n}\n' \
    "$release_sha" \
    "$release_tag" \
    "$rust_version" \
    >"$provenance_file"

cd "$release_root"
sha256sum \
    "${artifact_name}.tar.gz" \
    "${artifact_name}.cargo-metadata.json" \
    "${artifact_name}.provenance.json" \
    >SHA256SUMS
