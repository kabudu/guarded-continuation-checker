#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || {
  echo "usage: $0 RIC3_CHECKOUT VENDOR_DIR OUTPUT_DIR" >&2
  exit 2
}
source_dir=$(cd "$1" && pwd)
vendor_dir=$(cd "$2" && pwd)
output_dir=$3
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
lock="$repo_root/tools/certifaiger-qualification-v1.lock"
[[ ! -e "$output_dir" ]] || { echo "refusing to overwrite $output_dir" >&2; exit 2; }
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)

value() { sed -n "s/^$1=//p" "$lock"; }
verify() {
  local key=$1 path=$2 expected actual
  expected=$(value "$key")
  actual=$(git -C "$source_dir/$path" rev-parse HEAD)
  [[ "$actual" == "$expected" ]] || {
    echo "$path mismatch: expected $expected, found $actual" >&2; exit 2;
  }
}
verify ric3_commit .
verify ric3_aig_rs_commit deps/aig-rs
verify ric3_aiger_commit deps/aig-rs/aiger
verify ric3_bitwuzla_rs_commit deps/bitwuzla-rs
verify ric3_bitwuzla_commit deps/bitwuzla-rs/bitwuzla
verify ric3_btor_rs_commit deps/btor-rs
verify ric3_cadical_rs_commit deps/cadical-rs
verify ric3_cadical_commit deps/cadical-rs/cadical
verify ric3_giputils_commit deps/giputils
verify ric3_kissat_rs_commit deps/kissat-rs
verify ric3_kissat_commit deps/kissat-rs/kissat
verify ric3_logicrs_commit deps/logicrs
verify ric3_fvbench_commit examples/fvbench
[[ "$(sha256sum "$source_dir/Cargo.lock" | cut -d' ' -f1)" == "$(value ric3_cargo_lock_sha256)" ]]
vendor_hash=$(cd "$vendor_dir" && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1)
[[ "$vendor_hash" == "$(value ric3_vendor_sha256)" ]]

qualification_image=${QUALIFICATION_IMAGE:-gcc-ric3-qualification:v1-arm64}
if ! docker run --rm --network none \
  --volume "$source_dir:/source:ro" --volume "$vendor_dir:/vendor:ro" \
  --volume "$output_dir:/out" "$qualification_image" \
  bash -euo pipefail -c '
    # The source checkout may be owned by the hosted runner UID. Normalise the
    # private copy so upstream build scripts can invoke Git without triggering
    # safe.directory rejection inside the root-owned qualification container.
    cp -a --no-preserve=ownership /source /tmp/ric3
    mkdir -p /tmp/ric3/.cargo
    printf "%s\n" "[source.crates-io]" "replace-with = \"vendored-sources\"" \
      "[source.vendored-sources]" "directory = \"/vendor\"" \
      "[net]" "offline = true" > /tmp/ric3/.cargo/config.toml
    cd /tmp/ric3
    cargo test --locked --offline --no-fail-fast
    cargo build --release --locked --offline
    ./target/release/ric3 --version
    cp target/release/ric3 /out/ric3
  ' > "$output_dir/build-test.log" 2>&1; then
  echo "rIC3 offline qualification failed; captured log follows" >&2
  cat "$output_dir/build-test.log" >&2
  exit 1
fi
sha256sum "$output_dir/ric3" > "$output_dir/binary.sha256"
printf 'qualification=pass\n' > "$output_dir/result.txt"
