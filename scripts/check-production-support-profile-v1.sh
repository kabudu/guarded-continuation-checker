#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 GUARDED_CONTINUATION_CHECKER_BINARY" >&2
  exit 2
fi
binary=$1
[[ -x "$binary" && ! -L "$binary" ]] || {
  echo "production profile binary must be an ordinary executable" >&2
  exit 2
}

[[ $($binary production-profile-version) == \
  'production_support_profile=firmware-rtl-v1 firmware_cli_version=2 artifact_schema_version=4' ]]
[[ $($binary firmware-cli-version) == \
  'firmware_cli_version=2 artifact_schema_version=4' ]]

reject() {
  local label=$1
  shift
  local stdout stderr exit_code
  stdout=$(mktemp "${TMPDIR:-/tmp}/gcc-profile-stdout.XXXXXXXX")
  stderr=$(mktemp "${TMPDIR:-/tmp}/gcc-profile-stderr.XXXXXXXX")
  set +e
  "$binary" "$@" >"$stdout" 2>"$stderr"
  exit_code=$?
  set -e
  [[ $exit_code -eq 2 && ! -s "$stdout" ]] || {
    rm -f "$stdout" "$stderr"
    echo "production profile admitted $label" >&2
    exit 1
  }
  grep -qx 'error: command is outside production support profile firmware-rtl-v1' "$stderr"
  rm -f "$stdout" "$stderr"
}

reject predicate predicate-cli-version
reject event-contract event-contract-cli-version
reject btor2 btor2-cli-version
reject revision-local revision-local-cli-version
reject controller controller-mtbdd-cli-version
reject experiment 16 4 1 random min-fill compare
reject empty

set +e
bad_profile=$($binary production-profile-version unexpected 2>&1)
bad_profile_exit=$?
set -e
[[ $bad_profile_exit -eq 2 && $bad_profile == \
  'error: usage: guarded-continuation-checker production-profile-version' ]]

echo 'production_support_profile_v1=PASS supported_commands=8 rejected_research_probes=7'
