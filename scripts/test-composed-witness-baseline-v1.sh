#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 GCC_BINARY CERTIFAIGER_SOURCE CERTIFAIGER_OUTPUT OUTPUT.csv" >&2
  exit 2
fi
gcc_binary=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
certifaiger_source=$(cd "$2" && pwd -P)
certifaiger_output=$(cd "$3" && pwd -P)
output=$4
manifest=${output%.csv}.manifest-v1.txt
image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}
expected_commit=3b8d9e9937234b5e064923bd00f20d3eb97ccc3f
expected_model_sha256=4cac8169723e6896e4738edf20710539a6c15001630fe0b667817ef22878823c
expected_witness_sha256=48eba4929b0728299af2a982807ae35cd189b6096020c3a9c2dead5db9af533c
[[ -x "$gcc_binary" ]] || { echo "GCC binary is not executable" >&2; exit 2; }
[[ -f "$certifaiger_source/tests/01_model.aag" ]] || {
  echo "qualified Certifaiger source fixture is missing" >&2; exit 2;
}
[[ -x "$certifaiger_output/bin/check" ]] || {
  echo "qualified Certifaiger checker is missing" >&2; exit 2;
}
[[ ! -e "$output" && ! -e "$manifest" ]] || {
  echo "refusing to overwrite composed-witness result" >&2; exit 2;
}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-composed-witness.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
model=$certifaiger_source/tests/01_model.aag
witness=$certifaiger_source/tests/01_witness.aag

sha256sum_portable() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

[[ $(git -C "$certifaiger_source" rev-parse HEAD) == "$expected_commit" ]] || {
  echo "Certifaiger source revision is not the pinned baseline" >&2; exit 2;
}
[[ $(sha256sum_portable "$model") == "$expected_model_sha256" ]] || {
  echo "Certifaiger model fixture digest mismatch" >&2; exit 2;
}
[[ $(sha256sum_portable "$witness") == "$expected_witness_sha256" ]] || {
  echo "Certifaiger witness fixture digest mismatch" >&2; exit 2;
}

"$gcc_binary" compose-safety-witnesses-v1 \
  "$model" "$scratch/first.aag" "$witness" "$witness" >/dev/null
"$gcc_binary" compose-safety-witnesses-v1 \
  "$model" "$scratch/second.aag" "$witness" "$witness" >/dev/null
cmp "$scratch/first.aag" "$scratch/second.aag"

# Produce a second, independently valid encoding of the same invariant by
# adding one redundant AND with true. The immutable fixture shape and digest
# above make these exact line edits fail if upstream bytes drift.
awk '
  NR == 1 { print "aag 33 1 4 1 28"; next }
  NR == 7 { print "66"; next }
  NR == 35 { print "66 65 1" }
  { print }
' "$witness" >"$scratch/distinct.aag"
"$gcc_binary" compose-safety-witnesses-v1 \
  "$model" "$scratch/distinct-first.aag" \
  "$witness" "$scratch/distinct.aag" >/dev/null
"$gcc_binary" compose-safety-witnesses-v1 \
  "$model" "$scratch/distinct-second.aag" \
  "$witness" "$scratch/distinct.aag" >/dev/null
cmp "$scratch/distinct-first.aag" "$scratch/distinct-second.aag"

docker run --rm --network none \
  -v "$certifaiger_output/bin:/cert:ro" \
  -v "$certifaiger_source/tests:/fixtures:ro" \
  -v "$scratch:/out:ro" "$image" \
  /cert/check /fixtures/01_model.aag /out/first.aag \
  >"$scratch/check.log" 2>&1
grep -q '^check_unsat: Certificate check passed$' "$scratch/check.log"
grep -q '^check: valid witness$' "$scratch/check.log"
docker run --rm --network none \
  -v "$certifaiger_output/bin:/cert:ro" \
  -v "$certifaiger_source/tests:/fixtures:ro" \
  -v "$scratch:/out:ro" "$image" \
  /cert/check /fixtures/01_model.aag /out/distinct.aag \
  >"$scratch/distinct-source-check.log" 2>&1
grep -q '^check: valid witness$' "$scratch/distinct-source-check.log"
docker run --rm --network none \
  -v "$certifaiger_output/bin:/cert:ro" \
  -v "$certifaiger_source/tests:/fixtures:ro" \
  -v "$scratch:/out:ro" "$image" \
  /cert/check /fixtures/01_model.aag /out/distinct-first.aag \
  >"$scratch/distinct-check.log" 2>&1
grep -q '^check_unsat: Certificate check passed$' "$scratch/distinct-check.log"
grep -q '^check: valid witness$' "$scratch/distinct-check.log"

model_bytes=$(wc -c <"$model" | tr -d ' ')
witness_bytes=$(wc -c <"$witness" | tr -d ' ')
artifact_bytes=$(wc -c <"$scratch/first.aag" | tr -d ' ')
distinct_bytes=$(wc -c <"$scratch/distinct.aag" | tr -d ' ')
distinct_artifact_bytes=$(wc -c <"$scratch/distinct-first.aag" | tr -d ' ')
printf '%s\n' \
  'schema_version,case,model_bytes,witness_count,input_witness_bytes,composed_bytes,deterministic,certifaiger,lrat_isa,status' \
  "1,upstream-01-self-composition,$model_bytes,2,$((witness_bytes * 2)),$artifact_bytes,true,pass,pass,validated" \
  "1,syntactically-distinct-equivalent,$model_bytes,2,$((witness_bytes + distinct_bytes)),$distinct_artifact_bytes,true,pass,pass,validated" \
  >"$output"
{
  printf 'schema_version=1\n'
  printf 'baseline=fm-2026-theorem-1\n'
  printf 'scope=safety-equivalent-composition\n'
  printf 'certifaiger_commit=%s\n' "$expected_commit"
  printf 'model_sha256=%s\n' "$(sha256sum_portable "$model")"
  printf 'witness_sha256=%s\n' "$(sha256sum_portable "$witness")"
  printf 'composed_sha256=%s\n' "$(sha256sum_portable "$scratch/first.aag")"
  printf 'distinct_witness_sha256=%s\n' "$(sha256sum_portable "$scratch/distinct.aag")"
  printf 'distinct_composed_sha256=%s\n' \
    "$(sha256sum_portable "$scratch/distinct-first.aag")"
  printf 'status=validated\n'
} >"$manifest"
echo "composed-witness baseline status=VALIDATED case=upstream-01-self-composition output=$output"
