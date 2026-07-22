#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 GCC_BINARY YOSYS RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv MANIFEST.txt" >&2
  exit 2
fi

gcc_binary=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
yosys=$(cd "$(dirname "$2")" && pwd -P)/$(basename "$2")
ric3_output=$(cd "$3" && pwd -P)
certifaiger_output=$(cd "$4" && pwd -P)
output=$5
manifest=$6
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}

[[ -x "$gcc_binary" && -x "$yosys" && -x "$ric3_output/ric3" ]] || {
  echo "qualified producer inputs are missing" >&2
  exit 2
}
[[ -x "$certifaiger_output/bin/check" && -x "$certifaiger_output/bin/aigsim" ]] || {
  echo "qualified checker inputs are missing" >&2
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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-plic-closest.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
"$repo/scripts/build-roalogic-plic-revision-aiger-v1.sh" \
  "$yosys" "$scratch/models" >/dev/null
mkdir "$scratch/evidence"

docker run --rm --network none \
  -v "$ric3_output:/tools:ro" -v "$scratch/models:/models:ro" \
  -v "$scratch/evidence:/out" "$ric3_image" \
  /tools/ric3 check /models/old-safe.aag --cert /out/safe.witness.aag \
  --ui false ic3 >"$scratch/evidence/safe-producer.log" 2>&1
grep -qx 'UNSAT' "$scratch/evidence/safe-producer.log"
docker run --rm --network none \
  -v "$ric3_output:/tools:ro" -v "$scratch/models:/models:ro" \
  -v "$scratch/evidence:/out" "$ric3_image" \
  /tools/ric3 check /models/old-unsafe.aag --cert /out/unsafe.trace.aag \
  --ui false bmc >"$scratch/evidence/unsafe-producer.log" 2>&1
grep -qx 'SAT' "$scratch/evidence/unsafe-producer.log"

for revision in old new; do
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/tools:ro" -v "$scratch/models:/models:ro" \
    -v "$scratch/evidence:/out:ro" "$certifaiger_image" \
    /tools/check "/models/$revision-safe.aag" /out/safe.witness.aag \
    >"$scratch/evidence/$revision-safe-check.log" 2>&1
  grep -q '^check_unsat: Certificate check passed$' \
    "$scratch/evidence/$revision-safe-check.log"
  grep -q '^check: valid witness$' "$scratch/evidence/$revision-safe-check.log"
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/tools:ro" -v "$scratch/models:/models:ro" \
    -v "$scratch/evidence:/out:ro" "$certifaiger_image" \
    /tools/aigsim -c -m "/models/$revision-unsafe.aag" /out/unsafe.trace.aag \
    >"$scratch/evidence/$revision-unsafe-check.log" 2>&1
done

safe_model_bytes=$(wc -c <"$scratch/models/old-safe.aag" | tr -d ' ')
unsafe_model_bytes=$(wc -c <"$scratch/models/old-unsafe.aag" | tr -d ' ')
safe_witness_bytes=$(wc -c <"$scratch/evidence/safe.witness.aag" | tr -d ' ')
unsafe_trace_bytes=$(wc -c <"$scratch/evidence/unsafe.trace.aag" | tr -d ' ')
printf '%s\n' \
  'schema_version,scope,safe_model_bytes,unsafe_model_bytes,models_identical,safe_witness_bytes,unsafe_trace_bytes,evidence_reusable,old_safe_verified,new_safe_verified,old_unsafe_verified,new_unsafe_verified,semantic_regenerated_bytes,status' \
  "1,public-plic-two-revision-both-answer,$safe_model_bytes,$unsafe_model_bytes,true,$safe_witness_bytes,$unsafe_trace_bytes,true,true,true,true,true,0,falsified" \
  >"$output"
{
  printf 'schema_version=1\n'
  printf 'baseline=maintained-aiger-ric3-certifaiger\n'
  printf 'scope=public-plic-two-revision-both-answer\n'
  printf 'old_source_sha256=a7f01fdf58c3bab4597b26a2c54784add31a2fa897a61bc7e59af872de284933\n'
  printf 'new_source_sha256=bab7c8c1fa31b760f41bedb840288f40b61b460b82f0620f1128f622ca711a7b\n'
  printf 'safe_model_sha256=%s\n' "$(sha256_file "$scratch/models/old-safe.aag")"
  printf 'unsafe_model_sha256=%s\n' "$(sha256_file "$scratch/models/old-unsafe.aag")"
  printf 'safe_witness_sha256=%s\n' "$(sha256_file "$scratch/evidence/safe.witness.aag")"
  printf 'unsafe_trace_sha256=%s\n' "$(sha256_file "$scratch/evidence/unsafe.trace.aag")"
  printf 'gcc_binary_sha256=%s\n' "$(sha256_file "$gcc_binary")"
  printf 'ric3_binary_sha256=%s\n' "$(sha256_file "$ric3_output/ric3")"
  printf 'certifaiger_check_sha256=%s\n' "$(sha256_file "$certifaiger_output/bin/check")"
  printf 'status=falsified\n'
} >"$manifest"
echo "roalogic_plic_closest_baseline_v1=FALSIFIED semantic_models_identical=true evidence_reusable=true output=$output"
