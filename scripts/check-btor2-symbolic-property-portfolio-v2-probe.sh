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

expected_header='schema_version,channels,horizon,logical_queries,proof_members,reused_queries,explicit_members,bitblast_members,direct_evidence_bytes,retained_evidence_bytes,evidence_reduction_pct,high_frame_two,low_frame_zero,verified,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 2 ]]

awk -F, '
  NR == 1 { next }
  NF != 15 || $1 != 1 || $2 != 6 || $3 != 2 || $4 != 12 || $5 != 6 ||
    $6 != 6 || $7 != 0 || $8 != 6 || $9 != 1500 || $10 != 1210 ||
    $11 != "19.333333" || $12 != "true" || $13 != "true" ||
    $14 != "true" || $15 != "accepted" { exit 1 }
' "$results"

echo "btor2_symbolic_property_portfolio_results_v2=PASS rows=1 result=$results"
