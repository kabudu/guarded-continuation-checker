#!/usr/bin/env bash
set -euo pipefail

report_failure() {
  local status=$?
  echo "opentitan resource failure status=$status line=${BASH_LINENO[0]} command=$BASH_COMMAND" >&2
  exit "$status"
}
trap report_failure ERR

if [[ $# -ne 6 ]]; then
  echo "usage: $0 GCC_BINARY YOSYS_BINARY RIC3_OUTPUT CERTIFAIGER_OUTPUT OUTPUT.csv MANIFEST.txt" >&2
  exit 2
fi

gcc_binary=$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")
yosys_binary=$(cd "$(dirname "$2")" && pwd -P)/$(basename "$2")
ric3_output=$(cd "$3" && pwd -P)
certifaiger_output=$(cd "$4" && pwd -P)
output=$5
manifest=$6
repository=$(cd "$(dirname "$0")/.." && pwd -P)
trials=${TRIALS:-3}
ric3_image=${RIC3_IMAGE:-gcc-ric3-qualification:v1-amd64}
certifaiger_image=${CERTIFAIGER_IMAGE:-gcc-certifaiger-qualification:v1-amd64}
runtime_image=${GCC_RUNTIME_IMAGE:-ubuntu:24.04}
repetitions=10

[[ "$trials" =~ ^[1-9][0-9]*$ && "$trials" -le 5 ]] || {
  echo "TRIALS must be in 1..=5" >&2
  exit 2
}
[[ -x "$gcc_binary" && -x "$yosys_binary" && -x "$ric3_output/ric3" ]] || {
  echo "required resource producer is unavailable" >&2
  exit 2
}
[[ -x "$certifaiger_output/bin/runlim" && -x "$certifaiger_output/bin/check" ]] || {
  echo "qualified runlim and checker are required" >&2
  exit 2
}
for target in "$output" "$manifest"; do
  [[ ! -e "$target" && ! -L "$target" ]] || {
    echo "refusing to overwrite $target" >&2
    exit 2
  }
done

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-resources.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
models=$scratch/models
"$repository/scripts/build-opentitan-aon-dual-timer-aiger.sh" \
  "$yosys_binary" "$models" >/dev/null
models=$(cd "$models" && pwd -P)
gcc_directory=$(cd "$(dirname "$gcc_binary")" && pwd -P)
source_model=$repository/corpus/rtl/opentitan-aon-timer/generated/dual-timer-predicate-set.btor2

metric() {
  local name=$1 file=$2
  awk -v field="$name:" '$2 == field {print $3}' "$file"
}

normalised_wall() {
  local file=$1
  awk -v repetitions="$repetitions" '$2 == "real:" {printf "%.6f", $3 / repetitions}' "$file"
}

validate_measurement() {
  local file=$1
  local real space
  real=$(metric real "$file")
  space=$(metric space "$file")
  awk -v real="$real" -v space="$space" \
    'BEGIN { exit !(real > 0 && space > 0) }'
}

tree_bytes() {
  find "$1" -type f -exec wc -c {} + | awk 'END {print $1}'
}

gcc_tool_bytes=$(wc -c <"$gcc_binary" | tr -d ' ')
ric3_tool_bytes=$(wc -c <"$ric3_output/ric3" | tr -d ' ')
external_producer_tool_bytes=$(awk -v ric3="$ric3_tool_bytes" \
  -v composer="$gcc_tool_bytes" 'BEGIN {print ric3 + composer}')
external_consumer_tool_bytes=$(tree_bytes "$certifaiger_output/bin")

printf '%s\n' \
  'schema_version,trial,horizon,route,operation,wall_seconds,peak_space_mb,model_bytes,evidence_bytes,producer_tool_bytes,consumer_tool_bytes,answers_agree,status' \
  >"$scratch/result.csv"

for trial in $(seq 1 "$trials"); do
  for horizon in 4 5; do
    trial_dir=$scratch/trial-${trial}-h${horizon}
    mkdir "$trial_dir"
    trial_dir=$(cd "$trial_dir" && pwd -P)

    docker run --rm --network none --user "$(id -u):$(id -g)" \
      -v "$gcc_directory:/gcc:ro" -v "$certifaiger_output/bin:/cert:ro" \
      -v "$repository:/repo:ro" -v "$trial_dir:/out" "$runtime_image" \
      /cert/runlim -p -r 300 --sample-rate=1000 \
      -o /out/gcc-producer.runlim \
      /repo/scripts/opentitan-dual-timer-resource-container-v1.sh \
      gcc-produce "$horizon" "$repetitions"

    docker run --rm --network none --user "$(id -u):$(id -g)" \
      -v "$gcc_directory:/gcc:ro" -v "$certifaiger_output/bin:/cert:ro" \
      -v "$repository:/repo:ro" -v "$trial_dir:/out" "$runtime_image" \
      /cert/runlim -p -r 300 --sample-rate=1000 \
      -o /out/gcc-consumer.runlim \
      /repo/scripts/opentitan-dual-timer-resource-container-v1.sh \
      gcc-consume "$horizon" "$repetitions"

    docker run --rm --network none \
      -v "$ric3_output:/tools:ro" -v "$certifaiger_output/bin:/cert:ro" \
      -v "$gcc_directory:/gcc:ro" -v "$repository:/repo:ro" \
      -v "$models:/models:ro" -v "$trial_dir:/out" "$ric3_image" \
      /cert/runlim -p -r 300 --sample-rate=1000 \
      -o /out/external-producer.runlim \
      /repo/scripts/opentitan-dual-timer-resource-container-v1.sh \
      external-produce "$horizon" "$repetitions"

    docker run --rm --network none \
      -v "$certifaiger_output/bin:/cert:ro" -v "$repository:/repo:ro" \
      -v "$models:/models:ro" -v "$trial_dir:/out" "$certifaiger_image" \
      /cert/runlim -p -r 300 --sample-rate=1000 \
      -o /out/external-consumer.runlim \
      /repo/scripts/opentitan-dual-timer-resource-container-v1.sh \
      external-consume "$horizon" "$repetitions" >/dev/null

    for measurement in gcc-producer gcc-consumer \
      external-producer external-consumer; do
      validate_measurement "$trial_dir/$measurement.runlim"
    done
    echo "opentitan resources phase=measured trial=$trial horizon=$horizon"

    gcc_first=$trial_dir/gcc-repeat-1
    external_first=$trial_dir/external-repeat-1
    for repetition in $(seq 2 "$repetitions"); do
      cmp "$gcc_first/gcc.cert" \
        "$trial_dir/gcc-repeat-$repetition/gcc.cert"
      for property in wake bark bite; do
        cmp "$external_first/h${horizon}-${property}.evidence.aag" \
          "$trial_dir/external-repeat-$repetition/h${horizon}-${property}.evidence.aag"
      done
      cmp "$external_first/h${horizon}-composed.aag" \
        "$trial_dir/external-repeat-$repetition/h${horizon}-composed.aag"
    done
    echo "opentitan resources phase=deterministic trial=$trial horizon=$horizon"
    gcc_evidence_bytes=$(wc -c <"$gcc_first/gcc.cert" | tr -d ' ')
    if [[ "$horizon" == 4 ]]; then
      external_evidence_bytes=$(wc -c <"$external_first/h4-composed.aag" | tr -d ' ')
      external_model_bytes=$(wc -c "$models"/h4-{wake,bark,bite,safe-set}.aag | awk 'END {print $1}')
    else
      external_evidence_bytes=$(wc -c \
        "$external_first/h5-composed.aag" \
        "$external_first/h5-bark.evidence.aag" | \
        awk 'END {print $1}')
      external_model_bytes=$(wc -c "$models"/h5-{wake,bark,bite,safe-set}.aag | awk 'END {print $1}')
    fi

    for operation in producer consumer; do
      printf '1,%s,%s,gcc,%s,%s,%s,%s,%s,%s,%s,true,measured\n' \
        "$trial" "$horizon" "$operation" \
        "$(normalised_wall "$trial_dir/gcc-$operation.runlim")" \
        "$(metric space "$trial_dir/gcc-$operation.runlim")" \
        "$(wc -c <"$source_model" | tr -d ' ')" "$gcc_evidence_bytes" \
        "$gcc_tool_bytes" "$gcc_tool_bytes" >>"$scratch/result.csv"
      printf '1,%s,%s,external,%s,%s,%s,%s,%s,%s,%s,true,measured\n' \
        "$trial" "$horizon" "$operation" \
        "$(normalised_wall "$trial_dir/external-$operation.runlim")" \
        "$(metric space "$trial_dir/external-$operation.runlim")" \
        "$external_model_bytes" "$external_evidence_bytes" \
        "$external_producer_tool_bytes" \
        "$external_consumer_tool_bytes" >>"$scratch/result.csv"
    done
  done
done

{
  printf 'schema_version=1\n'
  printf 'scope=opentitan-dual-timer-h4-h5-complete-evidence\n'
  printf 'trials=%s\n' "$trials"
  printf 'sequential_invocations_per_sample=%s\n' "$repetitions"
  printf 'source_to_model_setup=excluded-common-pinned-yosys\n'
  printf 'runlim_wall_limit_seconds=300\n'
  printf 'runlim_sample_rate_microseconds=1000\n'
  printf 'status=measured\n'
} >"$scratch/manifest.txt"

if ! (set -C; cat "$scratch/result.csv" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
if ! (set -C; cat "$scratch/manifest.txt" >"$manifest") 2>/dev/null; then
  echo "refusing to overwrite $manifest" >&2
  exit 2
fi
echo "opentitan_dual_timer_resources=MEASURED trials=$trials horizons=2 output=$output"
