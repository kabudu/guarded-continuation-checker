#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
lock="$repo_root/tools/certifaiger-qualification-v1.lock"
base=$(sed -n 's/^ric3_rust_image=//p' "$lock")
expected="sha256:${base##*@sha256:}"
actual=$(docker image inspect "$base" --format '{{.Id}}')
[[ "$actual" == "$expected" ]] || {
  echo "Rust base mismatch: expected $expected, found $actual" >&2
  exit 2
}
docker tag "$base" gcc-rust-1.97-bookworm:v1-arm64
docker build --pull=false --tag gcc-ric3-qualification:v1-arm64 \
  --file "$repo_root/tools/ric3-qualification.Dockerfile" "$repo_root"
docker image inspect gcc-ric3-qualification:v1-arm64 --format '{{.Id}}' \
  > "$repo_root/tools/ric3-qualification-image-v1.id"
