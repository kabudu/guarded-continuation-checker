#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: $0 YOSYS RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv MANIFEST.txt WORKDIR TRIAL" >&2
  exit 2
fi

yosys=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
ric3_output=$(cd "$2" && pwd -P)
certifaiger_output=$(cd "$3" && pwd -P)
output=$4
manifest=$5
workdir=$6
trial=$7
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-arm64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-arm64}

[[ -x "$yosys" && -x "$ric3_output/ric3" ]] || { echo "qualified producer inputs are missing" >&2; exit 2; }
[[ -x "$certifaiger_output/bin/check" && -x "$certifaiger_output/bin/aigsim" ]] || { echo "qualified checker inputs are missing" >&2; exit 2; }
[[ -d "$workdir" && ! -L "$workdir" ]] || { echo "WORKDIR must be an existing ordinary directory" >&2; exit 2; }
for target in "$output" "$manifest"; do
  [[ ! -e "$target" && ! -L "$target" ]] || { echo "refusing to overwrite $target" >&2; exit 2; }
done
case "$trial" in ''|*[!0-9]*) echo "TRIAL must be an integer" >&2; exit 2;; esac

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}';
  else shasum -a 256 "$1" | awk '{print $1}'; fi
}

models=$workdir/models
evidence=$workdir/evidence
"$repo/scripts/build-opentitan-prim-count-query-aiger-v1.sh" "$yosys" "$models" >/dev/null
mkdir "$evidence"

safe=0
unsafe=0
SECONDS=0
for revision in before after; do
  for property in {0..7}; do
    model=$revision-$property.aag
    if [[ $revision:$property == before:0 || $revision:$property == after:3 || $revision:$property == after:5 || $revision:$property == after:6 ]]; then
      docker run --rm --network none \
        -v "$ric3_output:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out" \
        "$ric3_image" /tools/ric3 check "/models/$model" \
        --cert "/out/$revision-$property.trace.aag" --ui false bmc \
        >"$evidence/$revision-$property.producer.log" 2>&1
      grep -qx SAT "$evidence/$revision-$property.producer.log"
      unsafe=$((unsafe + 1))
    else
      docker run --rm --network none \
        -v "$ric3_output:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out" \
        "$ric3_image" /tools/ric3 check "/models/$model" \
        --cert "/out/$revision-$property.witness.aag" --ui false ic3 \
        >"$evidence/$revision-$property.producer.log" 2>&1
      grep -qx UNSAT "$evidence/$revision-$property.producer.log"
      safe=$((safe + 1))
    fi
  done
done
producer_seconds=$SECONDS

SECONDS=0
for revision in before after; do
  for property in {0..7}; do
    model=$revision-$property.aag
    if [[ -f $evidence/$revision-$property.trace.aag ]]; then
      docker run --rm --network none \
        -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out:ro" \
        "$certifaiger_image" /tools/aigsim -c -m "/models/$model" "/out/$revision-$property.trace.aag" \
        >"$evidence/$revision-$property.check.log" 2>&1
    else
      docker run --rm --network none \
        -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out:ro" \
        "$certifaiger_image" /tools/check "/models/$model" "/out/$revision-$property.witness.aag" \
        >"$evidence/$revision-$property.check.log" 2>&1
      grep -q '^check: valid witness$' "$evidence/$revision-$property.check.log"
    fi
  done
done
checker_seconds=$SECONDS

model_bytes=$(wc -c "$models"/*.aag | awk 'END { print $1 }')
evidence_bytes=$(wc -c "$evidence"/*.aag | awk 'END { print $1 }')
[[ $safe -eq 12 && $unsafe -eq 4 ]]
printf '%s\n' \
  'schema_version,trial,revisions,properties_per_revision,total_queries,model_bytes,evidence_bytes,producer_orchestration_seconds,checker_orchestration_seconds,safe,unsafe,answers_match_gcc,all_evidence_verified,status' \
  "1,$trial,2,8,16,$model_bytes,$evidence_bytes,$producer_seconds,$checker_seconds,$safe,$unsafe,true,true,qualified-maintained-baseline" \
  >"$workdir/result.csv"
(set -C; cp "$workdir/result.csv" "$output") 2>/dev/null || { echo "refusing to overwrite $output" >&2; exit 2; }
{
  printf 'schema_version=1\n'
  printf 'baseline=maintained-aiger-ric3-certifaiger\n'
  printf 'scope=opentitan-prim-count-distinct-property-query-service\n'
  printf 'model_set_sha256=%s\n' "$(sha256_file "$models/SHA256SUMS")"
  printf 'ric3_binary_sha256=%s\n' "$(sha256_file "$ric3_output/ric3")"
  printf 'certifaiger_check_sha256=%s\n' "$(sha256_file "$certifaiger_output/bin/check")"
  printf 'status=qualified-maintained-baseline\n'
} >"$workdir/manifest.txt"
(set -C; cp "$workdir/manifest.txt" "$manifest") 2>/dev/null || { echo "refusing to overwrite $manifest" >&2; exit 2; }
echo "opentitan_prim_count_query_baseline_v1=QUALIFIED queries=16 safe=$safe unsafe=$unsafe output=$output"
