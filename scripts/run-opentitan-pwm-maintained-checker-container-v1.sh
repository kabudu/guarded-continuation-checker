#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: $0 CHECK AIGSIM MODELS EVIDENCE LOGS MEASURE CONCURRENCY" >&2
  exit 2
fi

check=$1
aigsim=$2
models=$3
evidence=$4
logs=$5
measure=$6
concurrency=$7

case "$concurrency" in
  1|4) ;;
  *) echo "CONCURRENCY must be 1 or 4" >&2; exit 2 ;;
esac
[[ -x "$check" && -x "$aigsim" && -x "$measure" && -d "$models" && -d "$evidence" && -d "$logs" ]]

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
  local log=$logs/$name.check.log
  if is_safe "$mask" "$query"; then
    "$measure" "$check" "$models/$name.aag" \
      "$evidence/$name.witness.aag" >"$log" 2>&1
    grep -q '^check: valid witness$' "$log"
  else
    "$measure" "$aigsim" -c -m "$models/$name.aag" \
      "$evidence/$name.trace.aag" >"$log" 2>&1
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
