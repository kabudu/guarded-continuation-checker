#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
trials=${TRIALS:-3}
test -x "$binary"
case $trials in
  '' | *[!0-9]* | 0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
if [ "$trials" -gt 20 ]; then
  echo "TRIALS exceeds the static limit of 20" >&2
  exit 2
fi
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

repository=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
manifest_a=$repository/corpus/rtl/wmcontroller/physical-plant-split-door-v1.txt
manifest_b=$repository/corpus/rtl/wmcontroller/physical-plant-split-lock-v1.txt
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-split-process.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
operating_system=$(uname -s)
architecture=$(uname -m)

check_controller() {
  grep -q '^controller-split-evidence status=CREATED cli_version=1 ' "$1"
}

check_plant() {
  grep -q '^controller-split-plant status=CREATED cli_version=1 artifact_version=1 members=1 ' "$1"
}

check_set() {
  stdout=$1
  test "$(grep -c '^controller-split-resource-batch ' "$stdout")" -eq 2
  grep -q '^controller-split-resource-set status=VERIFIED .* controller_admissions=1 batches=2 members=2 safe=0 unsafe=2 ' "$stdout"
}

measure() {
  operation=$1
  trial=$2
  evidence_bytes=$3
  checker=$4
  shift 4
  stdout=$scratch/$operation-$trial.stdout
  metrics=$scratch/$operation-$trial.time
  if [ "$time_style" = bsd ]; then
    /usr/bin/time -l "$@" >"$stdout" 2>"$metrics"
    elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    /usr/bin/time -f '%e %M' -o "$metrics" "$@" >"$stdout"
    read -r elapsed peak_kib <"$metrics"
    peak_bytes=$((peak_kib * 1024))
  fi
  test -n "$elapsed"
  test -n "$peak_bytes"
  "$checker" "$stdout"
  case $evidence_bytes in
    file:*)
      evidence_path=${evidence_bytes#file:}
      evidence_bytes=$(wc -c <"$evidence_path" | tr -d ' ')
      ;;
  esac
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,true,ok\n' \
    "$operation" "$trial" "$elapsed" "$peak_bytes" "$evidence_bytes" \
    "$time_style" "$operating_system" "$architecture" >>"$output"
}

printf '%s\n' \
  'schema_version,operation,trial,elapsed_seconds,peak_rss_bytes,portable_evidence_bytes,time_backend,operating_system,architecture,answers_agree,status' >"$output"

trial=1
while [ "$trial" -le "$trials" ]; do
  evidence=$scratch/controller-$trial.controller-evidence
  results_a=$scratch/door-$trial.plant-results
  results_b=$scratch/lock-$trial.plant-results
  policy=$scratch/policy-$trial.txt

  measure controller-certify "$trial" "file:$evidence" check_controller "$binary" \
    certify-controller-proof-evidence-v1 "$manifest_a" "$evidence"
  controller_bytes=$(wc -c <"$evidence" | tr -d ' ')

  measure plant-door-certify "$trial" "file:$results_a" check_plant "$binary" \
    certify-bound-plant-results-v1 "$manifest_a" "$evidence" "$results_a"
  plant_a_bytes=$(wc -c <"$results_a" | tr -d ' ')

  measure plant-lock-certify "$trial" "file:$results_b" check_plant "$binary" \
    certify-bound-plant-results-v1 "$manifest_b" "$evidence" "$results_b"
  plant_b_bytes=$(wc -c <"$results_b" | tr -d ' ')

  cat >"$policy" <<EOF
controller_split_resource_policy_version=1
max_controller_artifact_bytes=$controller_bytes
max_unsat_proof_bytes=1048576
max_batches=2
max_plant_artifact_bytes_per_batch=16777216
max_members_per_batch=1
max_member_horizon=32
max_product_states_per_member=4096
max_transition_evaluations_per_batch=1000000000000
max_total_plant_artifact_bytes=$((plant_a_bytes + plant_b_bytes))
max_total_members=2
max_total_transition_evaluations=2000000000000
status=complete
EOF

  measure governed-set-verify "$trial" "$((controller_bytes + plant_a_bytes + plant_b_bytes))" \
    check_set "$binary" verify-bound-plant-result-set-with-resources-v1 \
    "$evidence" "$policy" "$manifest_a" "$results_a" "$manifest_b" "$results_b"
  trial=$((trial + 1))
done

echo "controller split process resources status=MEASURED trials=$trials os=$operating_system architecture=$architecture output=$output"
