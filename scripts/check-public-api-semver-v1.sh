#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

command -v cargo >/dev/null
command -v curl >/dev/null
command -v jq >/dev/null

current=$(sed -n '/^\[package\]$/,/^\[/s/^version = "\([^"]*\)"/\1/p' Cargo.toml)
latest=$(
  curl --fail --silent --show-error \
    --user-agent guarded-continuation-checker-semver-gate/1 \
    https://crates.io/api/v1/crates/guarded-continuation-checker |
    jq -er '.crate.max_stable_version'
)

semver='^([0-9]+)\.([0-9]+)\.([0-9]+)$'
[[ $current =~ $semver ]] || {
  echo "candidate crate version is not stable SemVer: $current" >&2
  exit 2
}
current_major=${BASH_REMATCH[1]}
current_minor=${BASH_REMATCH[2]}
current_patch=${BASH_REMATCH[3]}
[[ $latest =~ $semver ]] || {
  echo "published crate version is not stable SemVer: $latest" >&2
  exit 2
}
latest_major=${BASH_REMATCH[1]}
latest_minor=${BASH_REMATCH[2]}
latest_patch=${BASH_REMATCH[3]}

if ((current_major < latest_major)) ||
  ((current_major == latest_major && current_minor < latest_minor)) ||
  ((current_major == latest_major && current_minor == latest_minor && current_patch < latest_patch)); then
  echo "candidate version $current precedes published version $latest" >&2
  exit 2
fi

if ((current_major > latest_major)); then
  release_type=major
elif ((current_minor > latest_minor)); then
  # Treat pre-1.0 minor releases as constrained minor transitions instead of
  # allowing cargo-semver-checks to skip all major-change lints.
  release_type=minor
else
  release_type=patch
fi

cargo semver-checks check-release \
  --baseline-version "$latest" \
  --release-type "$release_type"
printf 'public-api-semver-v1=PASS baseline=%s candidate=%s release_type=%s\n' \
  "$latest" "$current" "$release_type"
