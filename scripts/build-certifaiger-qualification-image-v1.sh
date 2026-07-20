#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
lock="$repo_root/tools/certifaiger-qualification-v1.lock"
base_image=$(sed -n 's/^base_image=//p' "$lock")
qualification_image=$(sed -n 's/^qualification_image=//p' "$lock")

[[ -n "$base_image" && -n "$qualification_image" ]] || {
  echo "qualification lock is missing an image field" >&2
  exit 2
}

actual_id=$(docker image inspect "$base_image" --format '{{.Id}}')
expected_id="sha256:${base_image##*@sha256:}"
[[ "$actual_id" == "$expected_id" ]] || {
  echo "cached base mismatch: expected $expected_id, found $actual_id" >&2
  exit 2
}

docker tag "$base_image" gcc-ubuntu-24.04-base:v1-arm64
docker build \
  --pull=false \
  --tag "$qualification_image" \
  --file "$repo_root/tools/certifaiger-qualification.Dockerfile" \
  "$repo_root"
docker image inspect "$qualification_image" --format '{{.Id}}' \
  > "$repo_root/tools/certifaiger-qualification-image-v1.id"
