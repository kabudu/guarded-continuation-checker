#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 GCC_BINARY RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv" >&2
  exit 2
fi
gcc_binary=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
ric3_output=$(cd "$2" && pwd -P)
certifaiger_output=$(cd "$3" && pwd -P)
output=$4
manifest=${output%.csv}.manifest-v1.txt
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}
repository=$(cd "$(dirname "$0")/.." && pwd -P)
models=$repository/corpus/rtl/wmcontroller/composed-witness-models-v1
[[ -x "$gcc_binary" && -x "$ric3_output/ric3" ]] || {
  echo "qualified producer inputs are missing" >&2; exit 2;
}
[[ -x "$certifaiger_output/bin/check" ]] || {
  echo "qualified checker inputs are missing" >&2; exit 2;
}
[[ ! -e "$output" && ! -e "$manifest" ]] || {
  echo "refusing to overwrite changing-plant result" >&2; exit 2;
}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-changing-plant-witness.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

sha256sum_portable() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

check_witness() {
  local model=$1 witness=$2 log=$3
  docker run --rm --network none \
    -v "$certifaiger_output/bin:/cert:ro" \
    -v "$models:/models:ro" -v "$scratch:/out:ro" \
    "$certifaiger_image" /cert/check "/models/$model" "/out/$witness" \
    >"$scratch/$log" 2>&1
  grep -q '^check_unsat: Certificate check passed$' "$scratch/$log"
  grep -q '^check: valid witness$' "$scratch/$log"
}

printf '%s\n' \
  'schema_version,plant,multi_model_bytes,single_model_bytes,witness_bytes,composed_bytes,deterministic,individual_verified,composed_verified,status' \
  >"$output"
{
  printf 'schema_version=1\n'
  printf 'baseline=fm-2026-theorem-1\n'
  printf 'scope=changing-plant-distinct-properties\n'
  printf 'model_manifest_sha256=%s\n' \
    "$(sha256sum_portable "$models/manifest-v1.txt")"
  printf 'qualification_lock_sha256=%s\n' \
    "$(sha256sum_portable "$repository/tools/certifaiger-qualification-v1.lock")"
  printf 'gcc_binary_sha256=%s\n' "$(sha256sum_portable "$gcc_binary")"
  printf 'ric3_binary_sha256=%s\n' "$(sha256sum_portable "$ric3_output/ric3")"
  printf 'certifaiger_tree_sha256=%s\n' \
    "$(cd "$certifaiger_output/bin" && find . -type f -print0 | sort -z | xargs -0 shasum -a 256 | shasum -a 256 | awk '{print $1}')"
} >"$manifest"

for plant in nominal sensor-stuck actuator-delay persistent-disturbance; do
  for property in 15 16; do
    docker run --rm --network none \
      -v "$ric3_output:/tools:ro" -v "$models:/models:ro" \
      -v "$scratch:/out" "$ric3_image" \
      /tools/ric3 check "/models/$plant-property-$property.aag" \
      --cert "/out/$plant-property-$property.witness.aag" --ui false ic3 \
      >"$scratch/$plant-property-$property.producer.log" 2>&1
    grep -qx 'UNSAT' "$scratch/$plant-property-$property.producer.log"
    check_witness "$plant-property-$property.aag" \
      "$plant-property-$property.witness.aag" \
      "$plant-property-$property.check.log"
  done
  "$gcc_binary" compose-safety-witnesses-v1 "$models/$plant.aag" \
    "$scratch/$plant.composed.aag" \
    "$scratch/$plant-property-15.witness.aag" \
    "$scratch/$plant-property-16.witness.aag" >/dev/null
  "$gcc_binary" compose-safety-witnesses-v1 "$models/$plant.aag" \
    "$scratch/$plant.second.aag" \
    "$scratch/$plant-property-15.witness.aag" \
    "$scratch/$plant-property-16.witness.aag" >/dev/null
  cmp "$scratch/$plant.composed.aag" "$scratch/$plant.second.aag"
  check_witness "$plant.aag" "$plant.composed.aag" "$plant.composed.check.log"

  multi_model_bytes=$(wc -c <"$models/$plant.aag" | tr -d ' ')
  single_model_bytes=$(wc -c \
    "$models/$plant-property-15.aag" "$models/$plant-property-16.aag" |
    awk 'END {print $1}')
  witness_bytes=$(wc -c \
    "$scratch/$plant-property-15.witness.aag" \
    "$scratch/$plant-property-16.witness.aag" | awk 'END {print $1}')
  composed_bytes=$(wc -c <"$scratch/$plant.composed.aag" | tr -d ' ')
  printf '1,%s,%s,%s,%s,%s,true,true,true,validated\n' \
    "$plant" "$multi_model_bytes" "$single_model_bytes" \
    "$witness_bytes" "$composed_bytes" >>"$output"
  {
    printf 'plant_%s_property_15_witness_sha256=%s\n' "$plant" \
      "$(sha256sum_portable "$scratch/$plant-property-15.witness.aag")"
    printf 'plant_%s_property_16_witness_sha256=%s\n' "$plant" \
      "$(sha256sum_portable "$scratch/$plant-property-16.witness.aag")"
    printf 'plant_%s_composed_sha256=%s\n' "$plant" \
      "$(sha256sum_portable "$scratch/$plant.composed.aag")"
  } >>"$manifest"
done
printf 'status=validated\n' >>"$manifest"
echo "changing-plant composed-witness baseline status=VALIDATED plants=4 properties=2 output=$output"
