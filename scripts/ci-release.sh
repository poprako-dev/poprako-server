#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
[ "${GITHUB_ACTIONS:-}" = true ] || {
    echo "Release artifacts must be built by GitHub Actions" >&2
    exit 1
}

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
sbom_file="${release_root}/${artifact_name}.cdx.json"
provenance_file="${release_root}/${artifact_name}.provenance.json"

mkdir -p "$release_root"
cd "$project_root"

sh scripts/ci-release-check.sh --deployed --output "$release_root/deployment.json"

source_manifest=$(mktemp)
trap 'rm -f "$source_manifest"' EXIT
trap 'exit 1' INT TERM
git ls-files --recurse-submodules -z >"$source_manifest"
tar -czf "$release_root/poprako-server-${release_tag}-source.tar.gz" --null --files-from "$source_manifest"

cargo build --locked --release --bin poprako-server
cp target/release/poprako-server "$binary_file"
(cd "$release_root" && sha256sum poprako-server >"${artifact_name}.binary.sha256")
tar -czf "$archive_file" -C "$release_root" poprako-server
rm -f "$binary_file"

if ! cargo cyclonedx --version >/dev/null 2>&1; then
    cargo install --locked --version 0.5.9 cargo-cyclonedx
fi

[ "$(cargo cyclonedx --version)" = "cargo-cyclonedx 0.5.9" ] || {
    echo "cargo-cyclonedx 0.5.9 is required" >&2
    exit 1
}

cargo cyclonedx --format json --spec-version 1.5 \
    --target x86_64-unknown-linux-gnu --override-filename release-sbom
cp release-sbom.json "$sbom_file"
jq -e --arg version "${release_tag#v}" \
    '.bomFormat == "CycloneDX" and .specVersion == "1.5"
      and .metadata.component.name == "poprako-server"
      and .metadata.component.version == $version
      and (.components | length > 0)' "$sbom_file" >/dev/null
git diff --exit-code -- Cargo.lock

rust_version=$(rustc --version)

jq -n \
    --arg source_commit "$release_sha" \
    --arg release_tag "$release_tag" \
    --arg rust_toolchain "$rust_version" \
    --arg workflow_url "${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}" \
    --slurpfile deployment "$release_root/deployment.json" \
    '{source_commit: $source_commit, release_tag: $release_tag,
      rust_toolchain: $rust_toolchain, builder: "github-actions",
      workflow_url: $workflow_url, deployment: $deployment[0]}' >"$provenance_file"

cd "$release_root"
sha256sum ./*.tar.gz ./*.json ./*.sha256 >SHA256SUMS
