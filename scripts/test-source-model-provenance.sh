#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 EXPECTED.csv" >&2
  exit 2
fi

expected=$1
[[ -f "$expected" && ! -L "$expected" ]] || { echo "expected attestation is missing" >&2; exit 2; }
repository=$(cd "$(dirname "$0")/.." && pwd -P)
attester=$repository/scripts/attest-source-model-provenance.sh
source_root=$repository/corpus/rtl/wmcontroller
source_manifest=$source_root/source-model-provenance-v1.txt
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-source-model-test.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

"$attester" "$source_root" "$source_manifest" "$scratch/actual.csv" >/dev/null
diff -u "$expected" "$scratch/actual.csv"

expect_failure() {
  local expected_exit=$1 expected_message=$2
  shift 2
  set +e
  message=$("$@" 2>&1)
  actual_exit=$?
  set -e
  [[ $actual_exit -eq $expected_exit ]] || {
    echo "expected exit $expected_exit, got $actual_exit" >&2
    exit 1
  }
  grep -qx "$expected_message" <<<"$message"
}

cp -R "$source_root" "$scratch/drift"
printf '\n' >>"$scratch/drift/generated/controller.aag"
expect_failure 1 'source-to-model regeneration mismatch for member 0' \
  "$attester" "$scratch/drift" "$scratch/drift/source-model-provenance-v1.txt" \
  "$scratch/drift.csv"

sed 's|source_path=upstream/Controller.v|source_path=../Controller.v|' \
  "$source_manifest" >"$scratch/traversal.txt"
expect_failure 2 'provenance source path is invalid' \
  "$attester" "$source_root" "$scratch/traversal.txt" "$scratch/traversal.csv"

sed 's/tool_revision=b8e7da6f40ae8f552c116bf6c359b07c6533e159/tool_revision=0000000000000000000000000000000000000000/' \
  "$source_manifest" >"$scratch/wrong-tool.txt"
expect_failure 2 'Yosys revision does not match provenance manifest' \
  "$attester" "$source_root" "$scratch/wrong-tool.txt" "$scratch/wrong-tool.csv"

awk '{ printf "%s\r\n", $0 }' "$source_manifest" >"$scratch/crlf.txt"
expect_failure 2 'provenance manifest contains prohibited bytes' \
  "$attester" "$source_root" "$scratch/crlf.txt" "$scratch/crlf.csv"

cp "$source_manifest" "$scratch/nul.txt"
printf '\000' >>"$scratch/nul.txt"
expect_failure 2 'provenance manifest contains prohibited bytes' \
  "$attester" "$source_root" "$scratch/nul.txt" "$scratch/nul.csv"

echo 'source-model provenance tests status=PASS attested=2 mismatch=1 hostile=4'
