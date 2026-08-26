#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
staging_dir="$(mktemp -d)"
target="$project_dir/src-tauri/binaries/xytz-x86_64-unknown-linux-gnu"
trap 'rm -rf -- "$staging_dir"' EXIT

GOBIN="$staging_dir" go install github.com/xdagiz/xytz@v0.9.1
install -m 0755 "$staging_dir/xytz" "$target"
"$target" --version
