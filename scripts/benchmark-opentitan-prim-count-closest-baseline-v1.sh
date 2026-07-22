#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 YOSYS RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv MANIFEST.txt WORKDIR" >&2
  exit 2
fi

yosys=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
ric3_output=$(cd "$2" && pwd -P)
certifaiger_output=$(cd "$3" && pwd -P)
output=$4
manifest=$5
workdir=$6
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}

[[ -x "$yosys" && -x "$ric3_output/ric3" ]] || {
  echo "qualified producer inputs are missing" >&2
  exit 2
}
[[ -x "$certifaiger_output/bin/check" && -x "$certifaiger_output/bin/aigsim" ]] || {
  echo "qualified checker inputs are missing" >&2
  exit 2
}
[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
for target in "$output" "$manifest"; do
  [[ ! -e "$target" && ! -L "$target" ]] || {
    echo "refusing to overwrite $target" >&2
    exit 2
  }
done

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

models=$workdir/models
evidence=$workdir/evidence
"$repo/scripts/build-opentitan-prim-count-revision-aiger-v1.sh" \
  "$yosys" "$models" >/dev/null
mkdir "$evidence"

docker run --rm --network none \
  -v "$ric3_output:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out" \
  "$ric3_image" /tools/ric3 check /models/before.aag \
  --cert /out/before-safe.witness.aag --ui false ic3 \
  >"$evidence/before-producer.log" 2>&1
grep -qx 'UNSAT' "$evidence/before-producer.log"
docker run --rm --network none \
  -v "$ric3_output:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out" \
  "$ric3_image" /tools/ric3 check /models/after.aag \
  --cert /out/after-unsafe.trace.aag --ui false bmc \
  >"$evidence/after-producer.log" 2>&1
grep -qx 'SAT' "$evidence/after-producer.log"

docker run --rm --network none \
  -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
  -v "$evidence:/out:ro" "$certifaiger_image" \
  /tools/check /models/before.aag /out/before-safe.witness.aag \
  >"$evidence/before-check.log" 2>&1
grep -q '^check_unsat: Certificate check passed$' "$evidence/before-check.log"
grep -q '^check: valid witness$' "$evidence/before-check.log"
docker run --rm --network none \
  -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
  -v "$evidence:/out:ro" "$certifaiger_image" \
  /tools/aigsim -c -m /models/after.aag /out/after-unsafe.trace.aag \
  >"$evidence/after-check.log" 2>&1

set +e
docker run --rm --network none \
  -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
  -v "$evidence:/out:ro" "$certifaiger_image" \
  /tools/check /models/after.aag /out/before-safe.witness.aag \
  >"$evidence/cross-safe.log" 2>&1
cross_safe_exit=$?
docker run --rm --network none \
  -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
  -v "$evidence:/out:ro" "$certifaiger_image" \
  /tools/aigsim -c -m /models/before.aag /out/after-unsafe.trace.aag \
  >"$evidence/cross-unsafe.log" 2>&1
cross_unsafe_exit=$?
set -e
[[ $cross_safe_exit -ne 0 && $cross_unsafe_exit -ne 0 ]] || {
  echo "cross-revision evidence was unexpectedly reusable" >&2
  exit 1
}

before_model_bytes=$(wc -c <"$models/before.aag" | tr -d ' ')
after_model_bytes=$(wc -c <"$models/after.aag" | tr -d ' ')
safe_witness_bytes=$(wc -c <"$evidence/before-safe.witness.aag" | tr -d ' ')
unsafe_trace_bytes=$(wc -c <"$evidence/after-unsafe.trace.aag" | tr -d ' ')
printf '%s\n' \
  'schema_version,scope,before_model_bytes,after_model_bytes,models_identical,before_safe_witness_bytes,after_unsafe_trace_bytes,cross_revision_evidence_reusable,before_safe_verified,after_unsafe_verified,before_evidence_rejected_by_after,after_evidence_rejected_by_before,revision_regenerated_bytes,status' \
  "1,opentitan-prim-count-semantic-revision,$before_model_bytes,$after_model_bytes,false,$safe_witness_bytes,$unsafe_trace_bytes,false,true,true,true,true,$unsafe_trace_bytes,qualified-no-gcc-byte-win" \
  >"$workdir/result.csv"
if ! (set -C; cp "$workdir/result.csv" "$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
{
  printf 'schema_version=1\n'
  printf 'baseline=maintained-aiger-ric3-certifaiger\n'
  printf 'scope=opentitan-prim-count-semantic-revision\n'
  printf 'before_model_sha256=%s\n' "$(sha256_file "$models/before.aag")"
  printf 'after_model_sha256=%s\n' "$(sha256_file "$models/after.aag")"
  printf 'before_safe_witness_sha256=%s\n' \
    "$(sha256_file "$evidence/before-safe.witness.aag")"
  printf 'after_unsafe_trace_sha256=%s\n' \
    "$(sha256_file "$evidence/after-unsafe.trace.aag")"
  printf 'ric3_binary_sha256=%s\n' "$(sha256_file "$ric3_output/ric3")"
  printf 'certifaiger_check_sha256=%s\n' \
    "$(sha256_file "$certifaiger_output/bin/check")"
  printf 'status=qualified-no-gcc-byte-win\n'
} >"$workdir/manifest.txt"
if ! (set -C; cp "$workdir/manifest.txt" "$manifest") 2>/dev/null; then
  echo "refusing to overwrite $manifest" >&2
  exit 2
fi

echo "opentitan_prim_count_closest_baseline_v1=QUALIFIED evidence_reusable=false byte_win=false output=$output"
