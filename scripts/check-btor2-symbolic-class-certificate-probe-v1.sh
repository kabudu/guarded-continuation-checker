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

expected_header='schema_version,channels,classes,artifact_bytes,byte_ratio,verified,deterministic,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 4 ]]

awk -F, '
  NR == 1 { next }
  NF != 8 || $1 != 1 || $5 !~ /^0[.][0-9]{8}$/ ||
  $6 != "true" || $7 != "true" || $8 != "accepted" { exit 1 }
  $2 == 2 && !($3 == "0;1" && $4 == 232 && $5 == "0.01356646") { exit 1 }
  $2 == 4 && !($3 == "0+2;1;3" && $4 == 348 && $5 == "0.01330479") { exit 1 }
  $2 == 6 && !($3 == "0+2+4;1;3+5" && $4 == 460 && $5 == "0.01306521") { exit 1 }
  { seen[$2]++ }
  END { if (seen[2] != 1 || seen[4] != 1 || seen[6] != 1) exit 1 }
' "$results"

echo "btor2_symbolic_class_certificate_results_v1=PASS rows=3 result=$results"
