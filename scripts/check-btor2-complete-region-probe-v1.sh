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

expected_header='schema_version,channels,total_states,shared_states,local_states_pattern,shared_nodes,local_nodes_pattern,aggregate_nodes,shared_to_channel_edges,channel_to_aggregate_edges,state_artifact_bytes,state_artifact_sha256,deterministic,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 4 ]]

awk -F, '
  NR == 1 { next }
  NF != 14 || $1 != 1 || ($2 != 2 && $2 != 4 && $2 != 6) ||
  $12 !~ /^[0-9a-f]{64}$/ || $13 != "true" || $14 != "accepted" { exit 1 }
  $2 == 2 && !($3 == 16 && $4 == 8 && $5 == "2:6" && $6 == 155 &&
                $7 == "30:89" && $8 == 1 && $9 == 82 && $10 == 2 &&
                $11 == 372 && $12 == "8beb780b2e183650dc0ccb7c054561477e0052462d43cc75ae84c82d98bc7f63") { exit 1 }
  $2 == 4 && !($3 == 26 && $4 == 10 && $5 == "2:6:2:6" && $6 == 198 &&
                $7 == "30:89:27:89" && $8 == 3 && $9 == 163 && $10 == 4 &&
                $11 == 572 && $12 == "5dabf7a97fa3895ddcad79af8af3ec99d8a0e47d00843cf0833ed6e74673446b") { exit 1 }
  $2 == 6 && !($3 == 36 && $4 == 12 && $5 == "2:6:2:6:2:6" && $6 == 241 &&
                $7 == "30:89:27:89:27:89" && $8 == 5 && $9 == 244 && $10 == 6 &&
                $11 == 772 && $12 == "09fe42e30f79833c0935907d540590aed2205d188b602154bdf3a4d631cc5133") { exit 1 }
  { seen[$2]++ }
  END { if (seen[2] != 1 || seen[4] != 1 || seen[6] != 1) exit 1 }
' "$results"

echo "btor2_complete_region_probe_results_v1=PASS rows=3 result=$results"
