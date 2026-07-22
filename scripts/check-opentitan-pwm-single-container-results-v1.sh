#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
trials=$repo/results/opentitan-pwm-single-container-baseline-arm64-v1.csv
summary=$repo/results/opentitan-pwm-single-container-baseline-arm64-v1.summary.csv
manifest=$repo/results/opentitan-pwm-single-container-baseline-arm64-v1.manifest.txt
expected_model_set_sha256=1e9c81c03f78b32b266c5d367cf484c1e56deba0808d1c4c59d460cb47d65e0e
expected_evidence_set_sha256=d38d815128058d44282c2a34b6c9a1e84cf02cb9a337e5e8a7206576a97da90f

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

manifest_value() {
  local key=$1
  local count
  count=$(grep -c "^$key=" "$manifest")
  [[ $count -eq 1 ]]
  sed -n "s/^$key=//p" "$manifest"
}

[[ $(wc -l <"$trials" | tr -d ' ') -eq 11 ]]
[[ $(wc -l <"$summary" | tr -d ' ') -eq 3 ]]
[[ $(awk -F, 'NR > 1 && $3 == "single-container-sequential" { count++ } END { print count + 0 }' "$trials") -eq 5 ]]
[[ $(awk -F, 'NR > 1 && $3 == "single-container-parallel-4" { count++ } END { print count + 0 }' "$trials") -eq 5 ]]
[[ $(awk -F, 'NR > 1 && $5 == 1 && $6 == 1 && $9 == 20 && $20 == 9 && $21 == 11 && $22 == 3 && $23 == "true" && $24 == "true" && $27 == "qualified-maintained-single-container-baseline" { count++ } END { print count + 0 }' "$trials") -eq 10 ]]
[[ $(awk -F, -v model="$expected_model_set_sha256" -v evidence="$expected_evidence_set_sha256" 'NR > 1 && $25 == model && $26 == evidence { count++ } END { print count + 0 }' "$trials") -eq 10 ]]
[[ $(awk -F, 'NR > 1 && $4 == 1 { count++ } END { print count + 0 }' "$trials") -eq 5 ]]
[[ $(awk -F, 'NR > 1 && $4 == 4 { count++ } END { print count + 0 }' "$trials") -eq 5 ]]

median_for() {
  local mode=$1
  local column=$2
  awk -F, -v mode="$mode" -v column="$column" 'NR > 1 && $3 == mode { print $column }' \
    "$trials" | sort -n | sed -n '3p'
}

for mode in single-container-sequential single-container-parallel-4; do
  summary_row=$(awk -F, -v mode="$mode" '$2 == mode { print; count++ } END { if (count != 1) exit 1 }' "$summary")
  IFS=, read -r _ _ _ _ _ _ model_bytes evidence_bytes synthesis producer total checker synthesis_rss producer_rss checker_rss model_hash evidence_hash result_status <<<"$summary_row"
  [[ $model_bytes == "$(median_for "$mode" 10)" ]]
  [[ $evidence_bytes == "$(median_for "$mode" 11)" ]]
  [[ $synthesis == "$(median_for "$mode" 12)" ]]
  [[ $producer == "$(median_for "$mode" 13)" ]]
  [[ $total == "$(median_for "$mode" 14)" ]]
  [[ $checker == "$(median_for "$mode" 15)" ]]
  [[ $synthesis_rss == "$(median_for "$mode" 16)" ]]
  [[ $producer_rss == "$(median_for "$mode" 17)" ]]
  [[ $checker_rss == "$(median_for "$mode" 18)" ]]
  [[ $model_hash == "$expected_model_set_sha256" ]]
  [[ $evidence_hash == "$expected_evidence_set_sha256" ]]
  [[ $result_status == qualified-maintained-single-container-median ]]
done

[[ $(manifest_value trials_sha256) == "$(sha256_file "$trials")" ]]
[[ $(manifest_value summary_sha256) == "$(sha256_file "$summary")" ]]
[[ $(manifest_value harness_sha256) == "$(sha256_file "$repo/scripts/benchmark-opentitan-pwm-single-container-baseline-v1.sh")" ]]
[[ $(manifest_value producer_runner_sha256) == "$(sha256_file "$repo/scripts/run-opentitan-pwm-maintained-producer-container-v1.sh")" ]]
[[ $(manifest_value checker_runner_sha256) == "$(sha256_file "$repo/scripts/run-opentitan-pwm-maintained-checker-container-v1.sh")" ]]
[[ $(manifest_value matrix_runner_sha256) == "$(sha256_file "$repo/scripts/run-opentitan-pwm-single-container-matrix-v1.sh")" ]]
[[ $(manifest_value model_set_sha256) == "$expected_model_set_sha256" ]]
[[ $(manifest_value evidence_set_sha256) == "$expected_evidence_set_sha256" ]]
[[ $(manifest_value selective_reruns_allowed) == false ]]
[[ $(manifest_value status) == qualified-maintained-single-container-matrix ]]

echo "opentitan_pwm_single_container_retained_v1=PASS trials=10"
