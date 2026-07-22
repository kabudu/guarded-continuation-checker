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

expected_header='schema_version,channels,horizon,result,bad_frame,variables,clauses,proof_bytes,certificate_bytes,explicit_status,verified,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 7 ]]

awk -F, '
  NR == 1 { next }
  NF != 12 || $1 != 1 || $11 != "true" || $12 != "accepted" { exit 1 }
  $2 == 2 && $3 == 1 && !($4 == "Safe" && $5 == "none" && $6 == 8971 && $7 == 27352 && $8 == 145121 && $9 == 145222 && $10 == "agreed") { exit 1 }
  $2 == 2 && $3 == 2 && !($4 == "Unsafe" && $5 == 2 && $6 == 13456 && $7 == 41027 && $8 == 0 && $9 == 125 && $10 == "agreed") { exit 1 }
  $2 == 4 && $3 == 1 && !($4 == "Safe" && $5 == "none" && $6 == 10937 && $7 == 33668 && $8 == 180005 && $9 == 180106 && $10 == "agreed") { exit 1 }
  $2 == 4 && $3 == 2 && !($4 == "Unsafe" && $5 == 2 && $6 == 16405 && $7 == 50501 && $8 == 0 && $9 == 125 && $10 == "agreed") { exit 1 }
  $2 == 6 && $3 == 1 && !($4 == "Safe" && $5 == "none" && $6 == 12903 && $7 == 39984 && $8 == 215158 && $9 == 215259 && $10 == "agreed") { exit 1 }
  $2 == 6 && $3 == 2 && !($4 == "Unsafe" && $5 == 2 && $6 == 19354 && $7 == 59975 && $8 == 0 && $9 == 125 && $10 == "resource-refused") { exit 1 }
  { seen[$2 ":" $3]++ }
  END {
    if (seen["2:1"] != 1 || seen["2:2"] != 1 || seen["4:1"] != 1 ||
        seen["4:2"] != 1 || seen["6:1"] != 1 || seen["6:2"] != 1) exit 1
  }
' "$results"

echo "btor2_pwm_bitblast_results_v1=PASS rows=6 result=$results"
