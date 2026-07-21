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
mkdir "$scratch/source" "$scratch/root"
tar -xzf "$crate" -C "$scratch/source"
source="$scratch/source/$package"

for file in Cargo.toml Cargo.lock CRATE_README.md LICENSE README.md \
  src/lib.rs src/main.rs; do
  [[ -f "$source/$file" && ! -L "$source/$file" ]]
done
grep -q '^name = "guarded-continuation-checker"$' "$source/Cargo.toml"
grep -q '^readme = "CRATE_README.md"$' "$source/Cargo.toml"
grep -q 'evaluation-ready research prototype' "$source/CRATE_README.md"

CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$repo/target} \
  cargo install --path "$source" --root "$scratch/root" --locked --offline
binary="$scratch/root/bin/guarded-continuation-checker"
[[ -x "$binary" ]]
"$binary" firmware-cli-version | grep -q \
  '^firmware_cli_version=2 artifact_schema_version=4$'
"$binary" predicate-cli-version | grep -q '^predicate_cli_version=1 '
"$binary" btor2-cli-version | grep -q '^btor2_cli_version=1 '

printf 'crate-package-v1=PASS version=%s bytes=%s\n' \
  "$version" "$(wc -c <"$crate" | tr -d ' ')"
