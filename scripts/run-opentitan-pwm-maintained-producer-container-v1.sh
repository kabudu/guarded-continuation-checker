#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 RIC3 MODELS EVIDENCE LOGS MEASURE [CONCURRENCY]" >&2
  exit 2
fi

ric3=$1
models=$2
evidence=$3
logs=$4
measure=$5
concurrency=${6:-}

case "$concurrency" in
  1|4) ;;
  *) echo "CONCURRENCY must be 1 or 4" >&2; exit 2 ;;
esac
[[ -x "$ric3" && -x "$measure" && -d "$models" && -d "$evidence" && -d "$logs" ]]

is_safe() {
  local mask=$1
  local query=$2
  (( query == 4 )) ||
    (( query == 1 && (mask & 1) != 0 )) ||
    (( query == 2 && (mask & 2) != 0 )) ||
    (( query == 3 && mask == 3 ))
}

run_one() {
  local mask=$1
  local query=$2
  local name=mask${mask}-query${query}
  local log=$logs/$name.producer.log
  if is_safe "$mask" "$query"; then
    "$measure" "$ric3" check "$models/$name.aag" \
      --cert "$evidence/$name.witness.aag" --ui false ic3 >"$log" 2>&1
    grep -q '^UNSAT$' "$log"
  else
    "$measure" "$ric3" check "$models/$name.aag" \
      --cert "$evidence/$name.trace.aag" --ui false bmc >"$log" 2>&1
    grep -q '^SAT$' "$log"
  fi
  grep -q '^child_rusage_v1_peak_rss_bytes=[1-9][0-9]*$' "$log"
}

pids=()
wait_batch() {
  local failed=0
  local pid
  for pid in "${pids[@]}"; do
    wait "$pid" || failed=1
  done
  pids=()
  (( failed == 0 ))
}

for mask in 0 1 2 3; do
  for query in 0 1 2 3 4; do
    run_one "$mask" "$query" &
    pids+=("$!")
    if (( ${#pids[@]} == concurrency )); then
      wait_batch
    fi
  done
done
if (( ${#pids[@]} > 0 )); then
  wait_batch
fi
