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

expected_header='schema_version,channels,logical_queries,proof_members,reused_queries,direct_evidence_bytes,retained_evidence_bytes,evidence_reduction_pct,answers_agree,unsafe_assignments_replayed,horizon2_safe_refused,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 4 ]]

awk -F, '
  NR == 1 { next }
  NF != 12 || $1 != 1 || $9 != "true" || $12 != "accepted" { exit 1 }
  $2 == 2 && !($3 == 4 && $4 == 4 && $5 == 0 && $6 == 1840 && $7 == 2072 &&
                $8 == "-12.608696" && $10 == 2 && $11 == "false") { exit 1 }
  $2 == 4 && !($3 == 8 && $4 == 6 && $5 == 2 && $6 == 4056 && $7 == 3390 &&
                $8 == "16.420118" && $10 == 4 && $11 == "false") { exit 1 }
  $2 == 6 && !($3 == 12 && $4 == 6 && $5 == 6 && $6 == 6636 && $7 == 3778 &&
                $8 == "43.068113" && $10 == 6 && $11 == "true") { exit 1 }
  { seen[$2]++ }
  END { if (seen[2] != 1 || seen[4] != 1 || seen[6] != 1) exit 1 }
' "$results"

echo "btor2_symbolic_property_portfolio_results_v1=PASS rows=3 result=$results"
