#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 OUTPUT_DIR" >&2; exit 2; }
output=$1
[[ ! -e "$output" ]] || { echo "refusing to overwrite $output" >&2; exit 2; }
mkdir -p "$output"
output=$(cd "$output" && pwd)
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

clone_at() {
  local url=$1 commit=$2 destination=$3
  git init "$destination"
  git -C "$destination" remote add origin "$url"
  git -C "$destination" fetch --depth 1 origin "$commit"
  git -C "$destination" checkout --detach FETCH_HEAD
}

mkdir -p /tmp/certifaiger-input
clone_at https://github.com/Froleyks/certifaiger.git \
  3b8d9e9937234b5e064923bd00f20d3eb97ccc3f /tmp/certifaiger-input/certifaiger
clone_at https://github.com/arminbiere/aiger.git \
  1876b273dc603d000d11da8ebbc099353ac42c6f /tmp/certifaiger-input/aiger
clone_at https://github.com/arminbiere/cadical.git \
  c60730422e758ef1cebe7aeddf2dda31c996bf04 /tmp/certifaiger-input/cadical
clone_at https://github.com/lammich/lrat_isa.git \
  99b832b501f473f7890f20b99755b5ace86eae48 /tmp/certifaiger-input/lrat_isa
clone_at https://github.com/arminbiere/runlim.git \
  188f1e07fa233b787589900e0184092b49167706 /tmp/certifaiger-input/runlim
clone_at https://github.com/gipsyh/rIC3.git \
  7149d568785b039134f0b2baa58358c8af63e70d /tmp/ric3
git -C /tmp/ric3 submodule update --init --recursive
cargo vendor --locked --manifest-path /tmp/ric3/Cargo.toml /tmp/ric3-vendor \
  > /tmp/ric3-vendor-config.toml

docker pull ubuntu:24.04
docker tag ubuntu:24.04 gcc-ubuntu-24.04-base:v1-amd64
docker build --pull=false \
  --build-arg QUALIFICATION_BASE=gcc-ubuntu-24.04-base:v1-amd64 \
  --tag gcc-certifaiger-qualification:v1-amd64 \
  --file "$repo_root/tools/certifaiger-qualification.Dockerfile" "$repo_root"
docker pull rust:1.97.0-bookworm
docker tag rust:1.97.0-bookworm gcc-rust-1.97-bookworm:v1-amd64
docker build --pull=false \
  --build-arg QUALIFICATION_BASE=gcc-rust-1.97-bookworm:v1-amd64 \
  --tag gcc-ric3-qualification:v1-amd64 \
  --file "$repo_root/tools/ric3-qualification.Dockerfile" "$repo_root"
docker image inspect ubuntu:24.04 rust:1.97.0-bookworm \
  gcc-certifaiger-qualification:v1-amd64 gcc-ric3-qualification:v1-amd64 \
  --format '{{.RepoTags}} {{.RepoDigests}} {{.Id}} {{.Architecture}}' \
  > "$output/images.txt"

QUALIFICATION_IMAGE=gcc-certifaiger-qualification:v1-amd64 \
  "$repo_root/scripts/qualify-certifaiger-v1.sh" \
  /tmp/certifaiger-input /tmp/certifaiger-output
QUALIFICATION_IMAGE=gcc-ric3-qualification:v1-amd64 \
  "$repo_root/scripts/qualify-ric3-v1.sh" \
  /tmp/ric3 /tmp/ric3-vendor /tmp/ric3-output

cd "$repo_root"
cargo test --locked --test controller_plant_bounded_aiger_api
cargo build --release --locked
mkdir -p /tmp/gcc-output
cp target/release/guarded-continuation-checker /tmp/gcc-output/
sha256sum /tmp/gcc-output/guarded-continuation-checker \
  /tmp/ric3-output/ric3 /tmp/certifaiger-output/bin/* > "$output/binaries.sha256"

CERTIFAIGER_IMAGE=gcc-certifaiger-qualification:v1-amd64 \
  scripts/test-composed-witness-baseline-v1.sh \
  /tmp/gcc-output/guarded-continuation-checker \
  /tmp/certifaiger-input/certifaiger /tmp/certifaiger-output \
  "$output/composed-witness-baseline-amd64-v1.csv"

TRIALS=3 RIC3_IMAGE=gcc-ric3-qualification:v1-amd64 \
  CERTIFAIGER_IMAGE=gcc-certifaiger-qualification:v1-amd64 \
  scripts/benchmark-certified-evidence-v1.sh \
  /tmp/ric3-output /tmp/certifaiger-output "$output/certified-evidence-amd64.csv"
TRIALS=3 GCC_RUNTIME_IMAGE=gcc-ubuntu-24.04-base:v1-amd64 \
  scripts/benchmark-gcc-proof-runlim-v1.sh \
  /tmp/gcc-output /tmp/certifaiger-output "$output/gcc-proof-amd64.csv"
RIC3_IMAGE=gcc-ric3-qualification:v1-amd64 \
  CERTIFAIGER_IMAGE=gcc-certifaiger-qualification:v1-amd64 \
  scripts/test-certified-evidence-hostile-v1.sh \
  /tmp/ric3-output /tmp/certifaiger-output "$output/hostile-amd64.csv"
