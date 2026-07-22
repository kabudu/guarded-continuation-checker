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

expected_header='schema_version,channels,input_nodes,states,classes,non_singleton_classes,reused_channels,model_bytes,model_sha256,deterministic,status'
[[ $(head -n 1 "$results") == "$expected_header" ]]
[[ $(wc -l <"$results" | tr -d ' ') -eq 4 ]]

awk -F, '
  NR == 1 { next }
  NF != 11 || $1 != 1 || $3 != 3 || $9 !~ /^[0-9a-f]{64}$/ ||
  $10 != "true" || $11 != "accepted" { exit 1 }
  $2 == 2 && !($4 == 17 && $5 == "0;1" && $6 == 0 && $7 == 0 && $8 == 17101 &&
                $9 == "ddf8746f51d521f17fadaab0a474c9faa59a50ecb0d4e914a027cf3175f03737") { exit 1 }
  $2 == 4 && !($4 == 25 && $5 == "0+2;1;3" && $6 == 1 && $7 == 1 && $8 == 26156 &&
                $9 == "671136023f99b6bf9e21f140f267d770b42151738148d23f7f06162a2ebb86d1") { exit 1 }
  $2 == 6 && !($4 == 33 && $5 == "0+2+4;1;3+5" && $6 == 2 && $7 == 3 && $8 == 35208 &&
                $9 == "9e83a25e30df82636d54781ae086f8a3a43a446d7128ca120cca166f7240872e") { exit 1 }
  { seen[$2]++ }
  END { if (seen[2] != 1 || seen[4] != 1 || seen[6] != 1) exit 1 }
' "$results"

echo "btor2_symbolic_class_results_v1=PASS rows=3 result=$results"
