#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 YOSYS EXAMPLE_BINARY OUTPUT.csv WORKDIR" >&2
  exit 2
fi
yosys=$1
binary=$2
output=$3
workdir=$4
trials=${TRIALS:-5}
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
[[ -x "$yosys" && -x "$binary" ]] || { echo "Yosys and batch example must be executable" >&2; exit 2; }
[[ -d "$workdir" && ! -L "$workdir" ]] || { echo "WORKDIR must be an existing ordinary directory" >&2; exit 2; }
[[ ! -e "$output" && ! -L "$output" ]] || { echo "refusing to overwrite $output" >&2; exit 2; }
case "$trials" in ''|*[!0-9]*) echo "TRIALS must be an integer" >&2; exit 2;; esac
(( trials >= 3 && trials <= 21 )) || { echo "TRIALS must be between 3 and 21" >&2; exit 2; }

environment=$workdir/environment.btor2
before=$workdir/before.btor2
after=$workdir/after.btor2
"$repo/scripts/build-opentitan-prim-count-query-service-v1.sh" "$yosys" "$environment" "$before" "$after" >/dev/null
result=$workdir/result.csv
printf '%s\n' 'schema_version,trial,revisions,properties_per_revision,total_queries,shared_sections,standalone_bytes,batch_bytes,batch_to_standalone_ratio,bytes_saved,candidate_valuations,produce_nanos,verify_nanos,safe,unsafe,extraction_identical,status' >"$result"
for ((trial = 1; trial <= trials; trial++)); do
  trial_output=$workdir/trial-$trial.csv
  "$binary" "$environment" "$before" "$after" >"$trial_output"
  [[ $(wc -l <"$trial_output" | tr -d ' ') -eq 2 ]]
  row=$(sed -n '2p' "$trial_output")
  printf '%s\n' "$row" | grep -Eq '^1,2,8,16,3,[0-9]+,[0-9]+,0\.[0-9]+,[0-9]+,81924,[0-9]+,[0-9]+,12,4,true,measured$'
  printf '1,%d,%s\n' "$trial" "${row#1,}" >>"$result"
done
(set -C; cp "$result" "$output") 2>/dev/null || { echo "refusing to overwrite $output" >&2; exit 2; }
echo "opentitan_prim_count_revision_batch_v1=MEASURED trials=$trials queries=16 output=$output"
