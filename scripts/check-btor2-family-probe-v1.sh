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

expected_header='schema_version,trial,channels,properties,safe,unsafe,family_model_artifact_bytes,expanded_model_bytes,family_portfolio_bytes,direct_portfolio_bytes,family_produce_micros,direct_produce_micros,family_verify_micros,direct_verify_micros,family_evidence_bytes,direct_evidence_bytes,family_sha256,direct_sha256,answers_equal,deterministic,process_scope,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 16 ]]

awk -F, '
  NR == 1 { next }
  NF != 22 || $1 != 1 || $2 < 1 || $2 > 5 ||
  ($3 != 2 && $3 != 4 && $3 != 6) || $4 != $3 * 5 ||
  $5 + $6 != $4 || $7 >= $8 || $9 <= $10 || $15 != $16 ||
  $17 !~ /^[0-9a-f]{64}$/ || $18 !~ /^[0-9a-f]{64}$/ ||
  $19 != "true" || $20 != "true" ||
  $21 != "single-process-release" || $22 != "accepted" { exit 1 }
  {
    key = $3 ":" $2
    if (seen[key]++) exit 1
    trials[$3]++
    if (!(($3 in family_hash) || (family_hash[$3] = $17))) exit 1
    if (!(($3 in direct_hash) || (direct_hash[$3] = $18))) exit 1
    if (family_hash[$3] != $17 || direct_hash[$3] != $18) exit 1
    family_artifact[$3] = $7
    expanded[$3] = $8
    family_portfolio[$3] = $9
    direct_portfolio[$3] = $10
    evidence[$3] = $15
  }
  END {
    if (trials[2] != 5 || trials[4] != 5 || trials[6] != 5) exit 1
    if (family_artifact[2] != 356 || expanded[2] != 1596 ||
        family_portfolio[2] != 5990 || direct_portfolio[2] != 5698 || evidence[2] != 5412) exit 1
    if (family_artifact[4] != 488 || expanded[4] != 2754 ||
        family_portfolio[4] != 13584 || direct_portfolio[4] != 13200 || evidence[4] != 12754) exit 1
    if (family_artifact[6] != 620 || expanded[6] != 3918 ||
        family_portfolio[6] != 23098 || direct_portfolio[6] != 22622 || evidence[6] != 22016) exit 1
  }
' "$results"

echo "btor2_family_probe_results_v1=PASS rows=15 result=$results"
