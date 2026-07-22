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

expected_header='schema_version,channels,horizon,logical_queries,proof_members,explicit_members,bitblast_members,projected_work,tighter_limit_refused,exact_limit_admitted,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 7 ]]

awk -F, '
  NR == 1 { next }
  NF != 11 || $1 != 1 || $9 != "true" || $10 != "true" ||
    $11 != "accepted" { exit 1 }
  $2 == 2 && $3 == 1 && ($4 != 4 || $5 != 4 || $6 != 4 || $7 != 0 || $8 != 1251200) { exit 1 }
  $2 == 2 && $3 == 2 && ($4 != 4 || $5 != 4 || $6 != 0 || $7 != 4 || $8 != 1918560) { exit 1 }
  $2 == 4 && $3 == 1 && ($4 != 8 || $5 != 6 || $6 != 6 || $7 != 0 || $8 != 4008000) { exit 1 }
  $2 == 4 && $3 == 2 && ($4 != 8 || $5 != 6 || $6 != 0 || $7 != 6 || $8 != 4515984) { exit 1 }
  $2 == 6 && $3 == 1 && ($4 != 12 || $5 != 6 || $6 != 6 || $7 != 0 || $8 != 6937920) { exit 1 }
  $2 == 6 && $3 == 2 && ($4 != 12 || $5 != 6 || $6 != 0 || $7 != 6 || $8 != 6189840) { exit 1 }
  { seen[$2 ":" $3]++; if ($8 !~ /^[1-9][0-9]*$/) exit 1 }
  END {
    if (seen["2:1"] != 1 || seen["2:2"] != 1 || seen["4:1"] != 1 ||
        seen["4:2"] != 1 || seen["6:1"] != 1 || seen["6:2"] != 1) exit 1
  }
' "$results"

echo "btor2_symbolic_property_preflight_results_v1=PASS rows=6 result=$results"
