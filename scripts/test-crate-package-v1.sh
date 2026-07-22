#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$repo"

version=$(sed -n '/^\[package\]$/,/^\[/s/^version = "\([^"]*\)"/\1/p' Cargo.toml)
[[ -n "$version" ]]
package="guarded-continuation-checker-$version"
crate="target/package/$package.crate"

package_args=(--locked)
if [[ ${GCC_PACKAGE_ALLOW_DIRTY:-0} == 1 ]]; then
  package_args+=(--allow-dirty)
fi
cargo package "${package_args[@]}"
[[ -f "$crate" && ! -L "$crate" && -s "$crate" ]]

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-crate-package.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
mkdir "$scratch/source" "$scratch/research-root" "$scratch/production-root"
tar -xzf "$crate" -C "$scratch/source"
source="$scratch/source/$package"

file_count=$(find "$source" -type f | wc -l | tr -d ' ')
crate_bytes=$(wc -c <"$crate" | tr -d ' ')
(( file_count <= 64 )) || {
  echo "crate payload contains too many files: $file_count" >&2
  exit 1
}
(( crate_bytes <= 786432 )) || {
  echo "crate payload exceeds 768 KiB: $crate_bytes bytes" >&2
  exit 1
}
for excluded in .github corpus docs examples packaging results scripts tests; do
  if [[ -e "$source/$excluded" ]] && \
     find "$source/$excluded" -mindepth 1 \( -type f -o -type l \) \
       -print -quit | grep -q .; then
    echo "crate payload contains excluded content below $excluded" >&2
    exit 1
  fi
done

for file in Cargo.toml Cargo.lock CRATE_README.md LICENSE README.md \
  src/lib.rs src/main.rs; do
  [[ -f "$source/$file" && ! -L "$source/$file" ]]
done
grep -q '^name = "guarded-continuation-checker"$' "$source/Cargo.toml"
grep -q '^readme = "CRATE_README.md"$' "$source/Cargo.toml"
grep -q 'evaluation-ready research prototype' "$source/CRATE_README.md"

cargo fetch --manifest-path "$source/Cargo.toml" --locked
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$repo/target} \
  cargo check --manifest-path "$source/Cargo.toml" --locked --offline --lib \
    --features research-qatq-transport

CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$repo/target} \
  cargo install --path "$source" --root "$scratch/research-root" --locked --offline
binary="$scratch/research-root/bin/guarded-continuation-checker"
[[ -x "$binary" ]]
"$binary" firmware-cli-version | grep -q \
  '^firmware_cli_version=2 artifact_schema_version=4$'
"$binary" predicate-cli-version | grep -q '^predicate_cli_version=1 '
"$binary" btor2-cli-version | grep -q '^btor2_cli_version=1 '

CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$repo/target} \
  cargo install --path "$source" --root "$scratch/production-root" --locked \
    --offline --features production-firmware
production_binary="$scratch/production-root/bin/guarded-continuation-checker"
[[ -x "$production_binary" ]]
"$repo/scripts/check-production-support-profile-v1.sh" "$production_binary" \
  >/dev/null

printf 'crate-package-v1=PASS version=%s files=%s bytes=%s production_profile=PASS\n' \
  "$version" "$file_count" "$crate_bytes"
