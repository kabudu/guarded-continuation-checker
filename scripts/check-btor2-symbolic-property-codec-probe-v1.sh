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

expected_header='schema_version,channels,horizon,logical_queries,proof_members,structural_bytes,evidence_bytes,artifact_bytes,direct_evidence_bytes,artifact_vs_direct_pct,artifact_sha256,roundtrip,verified,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 2 ]]

awk -F, '
  NR == 1 { next }
  NF != 14 || $1 != 1 || $2 != 6 || $3 != 2 || $4 != 12 || $5 != 6 ||
    $6 != 460 || $7 != 750 || $8 != 1568 || $9 != 1500 ||
    $10 != "4.533333" ||
    $11 != "31db59025d13872959c11783d6f1887fd98f3bac9e0234f3da7fb88ed52e3486" ||
    $12 != "true" || $13 != "true" || $14 != "accepted" { exit 1 }
' "$results"

echo "btor2_symbolic_property_codec_results_v1=PASS rows=1 result=$results"
