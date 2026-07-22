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
rusage_build_image=${RUSAGE_BUILD_IMAGE:-gcc-rust-1.97-bookworm:v1-arm64}

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
case "$trial" in
  ''|*[!0-9]*) echo "TRIAL must be an integer" >&2; exit 2 ;;
esac

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported timing platform" >&2; exit 2 ;;
esac

run_timed() {
  local stdout=$1
  local metrics=$2
  shift 2
  if [[ $time_style == bsd ]]; then
    /usr/bin/time -l "$@" >"$stdout" 2>"$metrics"
  else
    /usr/bin/time -f '%e %M' -o "$metrics" "$@" >"$stdout"
  fi
}

read_timing() {
  local metrics=$1
  if [[ $time_style == bsd ]]; then
    timing_elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    timing_peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    read -r timing_elapsed timing_peak_kib <"$metrics"
    timing_peak_bytes=$((timing_peak_kib * 1024))
  fi
  [[ -n $timing_elapsed && -n $timing_peak_bytes && $timing_peak_bytes -gt 0 ]]
}

is_safe() {
  local mask=$1
  local query=$2
  (( query == 4 )) ||
    (( query == 1 && (mask & 1) != 0 )) ||
    (( query == 2 && (mask & 2) != 0 )) ||
    (( query == 3 && mask == 3 ))
}

models=$workdir/models
evidence=$workdir/evidence
mkdir "$evidence"
docker run --rm --network none \
  -v "$repo/scripts:/src:ro" -v "$workdir:/out" \
  "$rusage_build_image" cc -O2 -Wall -Wextra -Werror \
  /src/child-rusage-v1.c -o /out/child-rusage-v1
[[ -x "$workdir/child-rusage-v1" ]]

run_timed "$workdir/build-models.log" "$workdir/build-models.time" \
  "$repo/scripts/build-opentitan-pwm-crosstalk-impact-aiger-v1.sh" \
  "$yosys" "$models"
read_timing "$workdir/build-models.time"
synthesis_seconds=$timing_elapsed
synthesis_peak_rss_bytes=$timing_peak_bytes

produce_evidence() {
  local mask query name
  for mask in 0 1 2 3; do
    for query in 0 1 2 3 4; do
      name=mask${mask}-query${query}
      if is_safe "$mask" "$query"; then
        docker run --rm --network none \
          -v "$ric3_output:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out" \
          -v "$workdir/child-rusage-v1:/measure:ro" \
          "$ric3_image" /measure /tools/ric3 check "/models/$name.aag" \
          --cert "/out/$name.witness.aag" --ui false ic3 \
          >"$evidence/$name.producer.log" 2>&1
        grep -q '^UNSAT$' "$evidence/$name.producer.log"
      else
        docker run --rm --network none \
          -v "$ric3_output:/tools:ro" -v "$models:/models:ro" -v "$evidence:/out" \
          -v "$workdir/child-rusage-v1:/measure:ro" \
          "$ric3_image" /measure /tools/ric3 check "/models/$name.aag" \
          --cert "/out/$name.trace.aag" --ui false bmc \
          >"$evidence/$name.producer.log" 2>&1
        grep -q '^SAT$' "$evidence/$name.producer.log"
      fi
      grep -q '^child_rusage_v1_peak_rss_bytes=[1-9][0-9]*$' "$evidence/$name.producer.log"
    done
  done
}
TIMEFORMAT='%R'
{ time produce_evidence; } 2>"$workdir/producer.time"
producer_seconds=$(<"$workdir/producer.time")
total_producer_seconds=$(awk -v synthesis="$synthesis_seconds" -v solving="$producer_seconds" 'BEGIN { printf "%.2f", synthesis + solving }')

check_evidence() {
  local mask query name
  for mask in 0 1 2 3; do
    for query in 0 1 2 3 4; do
      name=mask${mask}-query${query}
      if is_safe "$mask" "$query"; then
        docker run --rm --network none \
          -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
          -v "$evidence:/out:ro" -v "$workdir/child-rusage-v1:/measure:ro" \
          "$certifaiger_image" /measure /tools/check "/models/$name.aag" \
          "/out/$name.witness.aag" >"$evidence/$name.check.log" 2>&1
        grep -q '^check: valid witness$' "$evidence/$name.check.log"
      else
        docker run --rm --network none \
          -v "$certifaiger_output/bin:/tools:ro" -v "$models:/models:ro" \
          -v "$evidence:/out:ro" -v "$workdir/child-rusage-v1:/measure:ro" \
          "$certifaiger_image" /measure /tools/aigsim -c -m "/models/$name.aag" \
          "/out/$name.trace.aag" >"$evidence/$name.check.log" 2>&1
      fi
      grep -q '^child_rusage_v1_peak_rss_bytes=[1-9][0-9]*$' "$evidence/$name.check.log"
    done
  done
}
{ time check_evidence; } 2>"$workdir/checker.time"
checker_seconds=$(<"$workdir/checker.time")

[[ $(rg -l '^UNSAT$' "$evidence" --glob '*.producer.log' | wc -l | tr -d ' ') -eq 9 ]]
[[ $(rg -l '^SAT$' "$evidence" --glob '*.producer.log' | wc -l | tr -d ' ') -eq 11 ]]
[[ $(find "$evidence" -type f -name '*.aag' | wc -l | tr -d ' ') -eq 20 ]]
model_bytes=$(wc -c "$models"/*.aag | awk 'END { print $1 }')
evidence_bytes=$(wc -c "$evidence"/*.aag | awk 'END { print $1 }')
producer_peak_rss_bytes=$(sed -n 's/^child_rusage_v1_peak_rss_bytes=//p' "$evidence"/*.producer.log | sort -n | tail -n 1)
checker_peak_rss_bytes=$(sed -n 's/^child_rusage_v1_peak_rss_bytes=//p' "$evidence"/*.check.log | sort -n | tail -n 1)
model_set_sha256=$(
  for path in "$models"/*.aag; do
    printf '%s  %s\n' "$(sha256_file "$path")" "$(basename "$path")"
  done | shasum -a 256 | awk '{print $1}'
)
evidence_set_sha256=$(
  for path in "$evidence"/*.aag; do
    printf '%s  %s\n' "$(sha256_file "$path")" "$(basename "$path")"
  done | shasum -a 256 | awk '{print $1}'
)

printf '%s\n' \
  'schema_version,trial,combinations,queries,observations,model_bytes,evidence_bytes,synthesis_seconds,producer_orchestration_seconds,total_producer_seconds,checker_orchestration_seconds,synthesis_peak_rss_bytes,producer_peak_rss_bytes,checker_peak_rss_bytes,time_backend,safe,unsafe,semantic_change_sets,answers_match_gcc,all_evidence_verified,model_set_sha256,evidence_set_sha256,status' \
  "1,$trial,4,5,20,$model_bytes,$evidence_bytes,$synthesis_seconds,$producer_seconds,$total_producer_seconds,$checker_seconds,$synthesis_peak_rss_bytes,$producer_peak_rss_bytes,$checker_peak_rss_bytes,$time_style,9,11,3,true,true,$model_set_sha256,$evidence_set_sha256,qualified-maintained-baseline" \
  >"$workdir/result.csv"

{
  printf 'schema_version=1\n'
  printf 'baseline=maintained-aiger-ric3-certifaiger\n'
  printf 'scope=opentitan-pwm-crosstalk-two-atom-five-query-revision-impact\n'
  printf 'yosys_version=%s\n' "$("$yosys" -V)"
  printf 'ric3_image=%s\n' "$ric3_image"
  printf 'certifaiger_image=%s\n' "$certifaiger_image"
  printf 'rusage_build_image=%s\n' "$rusage_build_image"
  printf 'rusage_source_sha256=%s\n' "$(sha256_file "$repo/scripts/child-rusage-v1.c")"
  printf 'rusage_binary_sha256=%s\n' "$(sha256_file "$workdir/child-rusage-v1")"
  printf 'ric3_binary_sha256=%s\n' "$(sha256_file "$ric3_output/ric3")"
  printf 'certifaiger_check_sha256=%s\n' "$(sha256_file "$certifaiger_output/bin/check")"
  printf 'certifaiger_aigsim_sha256=%s\n' "$(sha256_file "$certifaiger_output/bin/aigsim")"
  printf 'model_set_sha256=%s\n' "$model_set_sha256"
  printf 'evidence_set_sha256=%s\n' "$evidence_set_sha256"
  printf 'synthesis_scope=all-four-source-combinations-times-five-properties\n'
  printf 'memory_scope=max-native-synthesis-or-individual-container-child-process\n'
  printf 'status=qualified-maintained-baseline\n'
} >"$workdir/manifest.txt"

if ! (set -C; cp "$workdir/result.csv" "$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
if ! (set -C; cp "$workdir/manifest.txt" "$manifest") 2>/dev/null; then
  echo "refusing to overwrite $manifest" >&2
  exit 2
fi

echo "opentitan_pwm_crosstalk_impact_baseline_v1=QUALIFIED observations=20 safe=9 unsafe=11 semantic_change_sets=3 output=$output"
