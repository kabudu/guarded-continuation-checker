#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: $0 YOSYS RIC3_OUTPUT CERTIFAIGER_OUTPUT TRIALS.csv SUMMARY.csv MANIFEST.txt WORKDIR" >&2
  exit 2
fi

yosys=$1
ric3_output=$2
certifaiger_output=$3
output=$4
summary=$5
manifest=$6
workdir=$7
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
expected_model_set_sha256=1e9c81c03f78b32b266c5d367cf484c1e56deba0808d1c4c59d460cb47d65e0e

[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
for target in "$output" "$summary" "$manifest"; do
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

first=true
for mode in single-container-sequential single-container-parallel-4; do
  for trial in 1 2 3 4 5; do
    trial_dir=$workdir/$mode-trial-$trial
    trial_csv=$trial_dir.csv
    trial_manifest=$trial_dir.manifest.txt
    mkdir "$trial_dir"
    "$repo/scripts/benchmark-opentitan-pwm-single-container-baseline-v1.sh" \
      "$yosys" "$ric3_output" "$certifaiger_output" "$mode" \
      "$trial_csv" "$trial_manifest" "$trial_dir" "$trial"
    if $first; then
      head -n 1 "$trial_csv" >"$workdir/trials.csv"
      first=false
    fi
    tail -n 1 "$trial_csv" >>"$workdir/trials.csv"
  done
done

[[ $(wc -l <"$workdir/trials.csv" | tr -d ' ') -eq 11 ]]
[[ $(awk -F, 'NR > 1 && $27 == "qualified-maintained-single-container-baseline" { count++ } END { print count + 0 }' "$workdir/trials.csv") -eq 10 ]]
[[ $(awk -F, 'NR > 1 && $23 == "true" && $24 == "true" { count++ } END { print count + 0 }' "$workdir/trials.csv") -eq 10 ]]
[[ $(awk -F, -v expected="$expected_model_set_sha256" 'NR > 1 && $25 == expected { count++ } END { print count + 0 }' "$workdir/trials.csv") -eq 10 ]]
[[ $(awk -F, 'NR > 1 { print $26 }' "$workdir/trials.csv" | sort -u | wc -l | tr -d ' ') -eq 1 ]]
evidence_set_sha256=$(awk -F, 'NR == 2 { print $26 }' "$workdir/trials.csv")

median_for() {
  local mode=$1
  local column=$2
  awk -F, -v mode="$mode" -v column="$column" 'NR > 1 && $3 == mode { print $column }' \
    "$workdir/trials.csv" | sort -n | sed -n '3p'
}

{
  printf '%s\n' 'schema_version,mode,trials,concurrency,producer_containers_per_trial,checker_containers_per_trial,model_bytes,evidence_bytes,median_synthesis_seconds,median_producer_orchestration_seconds,median_total_producer_seconds,median_checker_orchestration_seconds,median_synthesis_peak_rss_bytes,median_producer_child_peak_rss_bytes,median_checker_child_peak_rss_bytes,model_set_sha256,evidence_set_sha256,status'
  for mode in single-container-sequential single-container-parallel-4; do
    if [[ $mode == single-container-sequential ]]; then concurrency=1; else concurrency=4; fi
    printf '1,%s,5,%s,1,1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,qualified-maintained-single-container-median\n' \
      "$mode" "$concurrency" \
      "$(median_for "$mode" 10)" "$(median_for "$mode" 11)" \
      "$(median_for "$mode" 12)" "$(median_for "$mode" 13)" \
      "$(median_for "$mode" 14)" "$(median_for "$mode" 15)" \
      "$(median_for "$mode" 16)" "$(median_for "$mode" 17)" \
      "$(median_for "$mode" 18)" "$expected_model_set_sha256" \
      "$evidence_set_sha256"
  done
} >"$workdir/summary.csv"

{
  printf 'schema_version=1\n'
  printf 'experiment=opentitan-pwm-single-container-maintained-matrix\n'
  printf 'sequential_trials=5\n'
  printf 'parallel_4_trials=5\n'
  printf 'selective_reruns_allowed=false\n'
  printf 'model_set_sha256=%s\n' "$expected_model_set_sha256"
  printf 'evidence_set_sha256=%s\n' "$evidence_set_sha256"
  printf 'trials_sha256=%s\n' "$(sha256_file "$workdir/trials.csv")"
  printf 'summary_sha256=%s\n' "$(sha256_file "$workdir/summary.csv")"
  printf 'harness_sha256=%s\n' "$(sha256_file "$repo/scripts/benchmark-opentitan-pwm-single-container-baseline-v1.sh")"
  printf 'producer_runner_sha256=%s\n' "$(sha256_file "$repo/scripts/run-opentitan-pwm-maintained-producer-container-v1.sh")"
  printf 'checker_runner_sha256=%s\n' "$(sha256_file "$repo/scripts/run-opentitan-pwm-maintained-checker-container-v1.sh")"
  printf 'matrix_runner_sha256=%s\n' "$(sha256_file "$repo/scripts/run-opentitan-pwm-single-container-matrix-v1.sh")"
  printf 'status=qualified-maintained-single-container-matrix\n'
} >"$workdir/manifest.txt"

for source_target in \
  "$workdir/trials.csv:$output" \
  "$workdir/summary.csv:$summary" \
  "$workdir/manifest.txt:$manifest"; do
  source=${source_target%%:*}
  target=${source_target#*:}
  if ! (set -C; cp "$source" "$target") 2>/dev/null; then
    echo "refusing to overwrite $target" >&2
    exit 2
  fi
done

echo "opentitan_pwm_single_container_matrix_v1=QUALIFIED trials=10 evidence_set_sha256=$evidence_set_sha256 output=$output summary=$summary"
