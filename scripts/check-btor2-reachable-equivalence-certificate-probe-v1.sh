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

expected_header='schema_version,channels,horizon,classes,representatives,reused_channels,artifact_bytes,artifact_sha256,produce_micros,verify_micros,deterministic,replayed,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 4 ]]

awk -F, '
  NR == 1 { next }
  NF != 13 || $1 != 1 || ($2 != 2 && $2 != 4 && $2 != 6) || $3 != 63 ||
  $8 !~ /^[0-9a-f]{64}$/ || $9 !~ /^[0-9]+$/ || $10 !~ /^[0-9]+$/ ||
  $11 != "true" || $12 != "true" || $13 != "accepted" { exit 1 }
  $2 == 2 && !($4 == 2 && $5 == 2 && $6 == 0 && $7 == 220 &&
                $8 == "30e20c2120dbe065f5609da94daf83162f7c322d67d70071ebfd08e2b6655613") { exit 1 }
  $2 == 4 && !($4 == 4 && $5 == 4 && $6 == 0 && $7 == 324 &&
                $8 == "bffecaf978cd0cd17bfcfaef94594ba99ded337514db42904ab3f16f2339d1b3") { exit 1 }
  $2 == 6 && !($4 == 4 && $5 == 4 && $6 == 2 && $7 == 420 &&
                $8 == "3db19ca2f188c4a58f4916b7bd2001b3633cc8105c300b24825205c8430d17be") { exit 1 }
  { seen[$2]++ }
  END { if (seen[2] != 1 || seen[4] != 1 || seen[6] != 1) exit 1 }
' "$results"

echo "btor2_reachable_equivalence_certificate_results_v1=PASS rows=3 result=$results"
