#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 CQ_BINARY CORPUS_DIR OUTPUT_DIR SBY_PY" >&2
  exit 2
fi

cq_binary=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
corpus_dir=$(cd "$2" && pwd)
output_dir=$3
sby_py=$4

[[ -x "$cq_binary" ]] || { echo "CQ binary is not executable: $cq_binary" >&2; exit 2; }
[[ "$sby_py" == - || -f "$sby_py" ]] || { echo "SymbiYosys entrypoint is missing: $sby_py" >&2; exit 2; }
[[ -f "$corpus_dir/manifest.tsv" ]] || { echo "corpus manifest is missing" >&2; exit 2; }

mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$corpus_dir" && sha256sum --check --strict SHA256SUMS)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$corpus_dir" && shasum -a 256 --check SHA256SUMS)
else
  echo "sha256sum or shasum is required" >&2
  exit 2
fi

results_tmp="$output_dir/results.csv.tmp"
printf 'case,expected,cq_status,cq_exit,oracle_status,yosys,agreement\n' > "$results_tmp"

header=true
while IFS=$'\t' read -r case_id config_name expected extra; do
  if $header; then
    [[ "$case_id" == case && "$config_name" == config && "$expected" == expected && -z "${extra:-}" ]] || {
      echo "invalid corpus manifest header" >&2; exit 2;
    }
    header=false
    continue
  fi
  [[ -n "$case_id" ]] || continue
  [[ "$case_id" =~ ^[a-z0-9][a-z0-9-]*$ && "$config_name" =~ ^[a-z0-9][a-z0-9-]*\.conf$ ]] || {
    echo "invalid corpus manifest row: $case_id" >&2; exit 2;
  }
  [[ "$expected" == SAFE || "$expected" == UNSAFE ]] || {
    echo "invalid expected status for $case_id" >&2; exit 2;
  }
  [[ -z "${extra:-}" ]] || { echo "extra manifest field for $case_id" >&2; exit 2; }

  config="$corpus_dir/$config_name"
  [[ -f "$config" ]] || { echo "missing config for $case_id" >&2; exit 2; }
  case_dir="$output_dir/$case_id"
  artifact_dir="$case_dir/cq-artifact"
  mkdir -p "$case_dir"

  set +e
  "$cq_binary" firmware-rtl-config-safety-gate "$config" "$artifact_dir" >"$case_dir/cq.stdout" 2>"$case_dir/cq.stderr"
  cq_exit=$?
  set -e
  expected_exit=0
  [[ "$expected" == UNSAFE ]] && expected_exit=1
  [[ $cq_exit -eq $expected_exit ]] || {
    echo "CQ exit mismatch for $case_id: expected $expected_exit, got $cq_exit" >&2; exit 1;
  }
  "$cq_binary" firmware-artifact-validate "$artifact_dir" >>"$case_dir/cq.stdout" 2>>"$case_dir/cq.stderr"
  cq_status=$(sed -n 's/^status=//p' "$artifact_dir/safety-report.txt" | head -n 1)
  [[ "$cq_status" == "$expected" ]] || { echo "CQ status mismatch for $case_id" >&2; exit 1; }

  top=$(sed -n 's/^top=//p' "$config")
  horizon=$(sed -n 's/^horizon=//p' "$config")
  [[ "$top" =~ ^[A-Za-z_][A-Za-z0-9_]*$ && "$horizon" =~ ^[1-9][0-9]*$ ]] || {
    echo "invalid top or horizon for $case_id" >&2; exit 2;
  }
  sources=()
  while IFS= read -r source; do
    sources[${#sources[@]}]=$source
  done < <(sed -n 's/^source=//p' "$config")
  [[ ${#sources[@]} -gt 0 ]] || { echo "no sources for $case_id" >&2; exit 2; }
  oracle_status=NOT_RUN
  if [[ "$sby_py" != - ]]; then
    sby_file="$case_dir/oracle.sby"
    {
    printf '[options]\nmode bmc\ndepth %s\n\n[engines]\nsmtbmc z3\n\n[script]\n' "$horizon"
    printf 'read -formal -sv -D SBY'
    for source in "${sources[@]}"; do printf ' %s' "$(basename "$source")"; done
    printf '\nprep -top %s\n\n[files]\n' "$top"
    for source in "${sources[@]}"; do
      [[ "$source" != /* && "$source" != *..* ]] || { echo "unsafe source path for $case_id" >&2; exit 2; }
      printf '%s\n' "$corpus_dir/$source"
    done
    } > "$sby_file"

    set +e
    python3 "$sby_py" -f -d "$case_dir/oracle" "$sby_file" >"$case_dir/oracle.stdout" 2>"$case_dir/oracle.stderr"
    oracle_exit=$?
    set -e
    oracle_status=ERROR
    [[ -f "$case_dir/oracle/PASS" ]] && oracle_status=SAFE
    [[ -f "$case_dir/oracle/FAIL" ]] && oracle_status=UNSAFE
    [[ "$oracle_status" == "$expected" ]] || {
      echo "oracle status mismatch for $case_id: expected $expected, got $oracle_status (exit $oracle_exit)" >&2; exit 1;
    }
  fi

  yosys=$(sed -n 's/^yosys=//p' "$artifact_dir/run-manifest.txt" | tr ',' ';')
  agreement=true
  [[ "$oracle_status" == NOT_RUN ]] && agreement=not-applicable
  printf '%s,%s,%s,%s,%s,%s,%s\n' "$case_id" "$expected" "$cq_status" "$cq_exit" "$oracle_status" "$yosys" "$agreement" >> "$results_tmp"
done < "$corpus_dir/manifest.tsv"

$header && { echo "corpus manifest contains no header" >&2; exit 2; }
mv "$results_tmp" "$output_dir/results.csv"
echo "RTL corpus passed: $output_dir/results.csv"
