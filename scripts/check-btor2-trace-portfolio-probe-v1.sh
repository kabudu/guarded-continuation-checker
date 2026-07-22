#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 RESULTS.csv" >&2
  exit 2
fi
results=$1
[[ -f "$results" && ! -L "$results" ]] || {
  echo "results must be an ordinary file" >&2
  exit 2
}

expected_header='schema_version,channels,horizon,predicates_per_channel,logical_queries,representative_evaluations,direct_singleton_evaluations,reused_queries,candidate_evaluation_bound,direct_evaluation_bound,work_reduction_percent,candidate_median_micros,direct_median_micros,speedup,exact_agreement,trials,selection,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 5 ]]

awk -F, '
  NR == 1 { next }
  NF != 18 || $1 != 1 || $2 != 6 || $3 != 4095 ||
  ($4 != 16 && $4 != 256 && $4 != 4096 && $4 != 8192) ||
  $5 != $4 * 6 || $6 != $4 * 2 || $7 != $4 * 2 || $8 != $4 * 2 ||
  $9 != $4 * 4 || $10 != $4 * 6 || $11 != "33.333333" ||
  $12 !~ /^[0-9]+$/ || $13 !~ /^[0-9]+$/ || $14 !~ /^[0-9]+[.][0-9]+$/ ||
  $15 != "true" || $16 != 5 || $17 != "none" || $18 != "accepted" { exit 1 }
  { seen[$4]++ }
  END {
    if (seen[16] != 1 || seen[256] != 1 || seen[4096] != 1 || seen[8192] != 1) exit 1
  }
' "$results"

echo "btor2_trace_portfolio_results_v1=PASS rows=4 result=$results"
