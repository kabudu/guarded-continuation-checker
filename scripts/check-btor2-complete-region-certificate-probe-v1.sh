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

expected_header='schema_version,channels,model_bytes,total_nodes,boundary_edges,state_artifact_bytes,complete_artifact_bytes,complete_over_model_percent,complete_artifact_sha256,deterministic,replayed,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 4 ]]

awk -F, '
  NR == 1 { next }
  NF != 12 || $1 != 1 || ($2 != 2 && $2 != 4 && $2 != 6) ||
  $9 !~ /^[0-9a-f]{64}$/ || $10 != "true" || $11 != "true" || $12 != "accepted" { exit 1 }
  $2 == 2 && !($3 == 16314 && $4 == 275 && $5 == 83 && $6 == 372 &&
                $7 == 4312 && $8 == "26.43" && $9 == "7a1e0fa02ded8a10ff9ac7fc54d8c27ddc83391e6d328db8d547da9d15dfb7be") { exit 1 }
  $2 == 4 && !($3 == 27150 && $4 == 436 && $5 == 165 && $6 == 572 &&
                $7 == 7448 && $8 == "27.43" && $9 == "df5153ea7b28ab419d2923441618a7623ba94fb2fbc8bc76a096d62db885c135") { exit 1 }
  $2 == 6 && !($3 == 38051 && $4 == 597 && $5 == 247 && $6 == 772 &&
                $7 == 10584 && $8 == "27.82" && $9 == "4a0bcaebdf67856d6ebf2c5b2d9a56f2d025517470cafc54cb366ee3c5825eac") { exit 1 }
  { seen[$2]++ }
  END { if (seen[2] != 1 || seen[4] != 1 || seen[6] != 1) exit 1 }
' "$results"

echo "btor2_complete_region_certificate_results_v1=PASS rows=3 result=$results"
