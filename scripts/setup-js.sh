#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
node_version=$(cat "$project_root/.node-version")
pnpm_version=12.11.2
install_root="${JS_TOOLCHAIN_HOME:-$HOME/.local/share/poprako-js}"

case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) platform=darwin-arm64 ;;
    Darwin-x86_64) platform=darwin-x64 ;;
    Linux-x86_64) platform=linux-x64 ;;
    Linux-aarch64) platform=linux-arm64 ;;
    *) echo "Unsupported JavaScript toolchain platform" >&2; exit 1 ;;
esac

node_root="$install_root/node-$node_version"
pnpm_root="$install_root/pnpm-$pnpm_version"
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM

verify() {
    expected="$1"
    file="$2"
    actual=$(shasum -a 256 "$file" | cut -d ' ' -f 1)
    [ "$actual" = "$expected" ] || { echo "Checksum mismatch: $file" >&2; exit 1; }
}

if [ ! -x "$node_root/bin/node" ]; then
    archive="node-v$node_version-$platform.tar.gz"
    curl --fail --silent --show-error --location --retry 3 --connect-timeout 20 \
        "https://nodejs.org/dist/v$node_version/$archive" -o "$work_dir/$archive"
    curl --fail --silent --show-error --location --retry 3 --connect-timeout 20 \
        "https://nodejs.org/dist/v$node_version/SHASUMS256.txt" -o "$work_dir/SHASUMS256.txt"
    checksum=$(awk -v archive="$archive" '$2 == archive {print $1}' "$work_dir/SHASUMS256.txt")
    verify "$checksum" "$work_dir/$archive"
    mkdir -p "$node_root"
    # Extract only the independent runtime and license, never npm or Corepack.
    tar -xzf "$work_dir/$archive" -C "$node_root" --strip-components=1 \
        "node-v$node_version-$platform/bin/node" "node-v$node_version-$platform/LICENSE"
fi

if [ ! -x "$pnpm_root/pnpm" ]; then
    package="@pnpm/exe.$platform"
    curl --fail --silent --show-error --location --retry 3 --connect-timeout 20 \
        "https://registry.npmjs.org/$package/$pnpm_version" -o "$work_dir/pnpm.json"
    archive_url=$("$node_root/bin/node" -p "JSON.parse(require('node:fs').readFileSync(process.argv[1])).dist.tarball" "$work_dir/pnpm.json")
    curl --fail --silent --show-error --location --retry 3 --connect-timeout 20 \
        "$archive_url" -o "$work_dir/pnpm.tgz"
    "$node_root/bin/node" --input-type=module - "$work_dir/pnpm.json" "$work_dir/pnpm.tgz" <<'JS'
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
const metadata = JSON.parse(readFileSync(process.argv[2], "utf8"));
const actual = createHash("sha512").update(readFileSync(process.argv[3])).digest("base64");
assert.equal(`sha512-${actual}`, metadata.dist.integrity, "pnpm archive integrity mismatch");
JS
    mkdir -p "$pnpm_root"
    tar -xzf "$work_dir/pnpm.tgz" -C "$pnpm_root" --strip-components=1
fi

[ "$("$node_root/bin/node" --version)" = "v$node_version" ]
[ "$("$pnpm_root/pnpm" --version)" = "$pnpm_version" ]
[ ! -e "$node_root/bin/npm" ]
[ ! -e "$node_root/bin/npx" ]
[ ! -d "$node_root/lib/node_modules/npm" ]
printf 'export PATH="%s/bin:%s:$PATH"\n' "$node_root" "$pnpm_root" > "$install_root/env"
if [ -n "${GITHUB_PATH:-}" ]; then
    printf '%s\n%s\n' "$node_root/bin" "$pnpm_root" >> "$GITHUB_PATH"
fi
printf 'Node.js %s and pnpm %s ready. Activate with: . "%s/env"\n' "$node_version" "$pnpm_version" "$install_root"
