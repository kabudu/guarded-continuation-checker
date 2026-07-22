#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 4 ]] || {
  echo "usage: $0 MODEL WITNESS PRODUCER_LOG ENGINE_FILE" >&2
  exit 2
}

model=$1
witness=$2
producer_log=$3
engine_file=$4
ric3=${RIC3_BINARY:-/tools/ric3}
bmc_witness=${witness}.bmc-candidate
ic3_witness=${witness}.ic3-candidate
bmc_log=${producer_log}.bmc-candidate
ic3_log=${producer_log}.ic3-candidate
bmc_pid=
ic3_pid=

for path in "$witness" "$producer_log" "$engine_file" \
  "$bmc_witness" "$ic3_witness" "$bmc_log" "$ic3_log"; do
  [[ ! -e "$path" && ! -L "$path" ]] || {
    echo "refusing to overwrite $path" >&2
    exit 2
  }
done

cleanup() {
  [[ -z "$bmc_pid" ]] || kill "$bmc_pid" 2>/dev/null || true
  [[ -z "$ic3_pid" ]] || kill "$ic3_pid" 2>/dev/null || true
  [[ -z "$bmc_pid" ]] || wait "$bmc_pid" 2>/dev/null || true
  [[ -z "$ic3_pid" ]] || wait "$ic3_pid" 2>/dev/null || true
  rm -f "$bmc_witness" "$ic3_witness" "$bmc_log" "$ic3_log"
}
trap cleanup EXIT HUP INT TERM

"$ric3" check "$model" --cert "$bmc_witness" --ui false bmc \
  >"$bmc_log" 2>&1 &
bmc_pid=$!
"$ric3" check "$model" --cert "$ic3_witness" --ui false ic3 \
  >"$ic3_log" 2>&1 &
ic3_pid=$!
ic3_finished=false

while true; do
  if ! jobs -pr | grep -qx "$bmc_pid"; then
    wait "$bmc_pid"
    bmc_pid=
    [[ $(tail -1 "$bmc_log") == SAT ]] || {
      echo "BMC ended without a depth-ordered SAT witness" >&2
      exit 2
    }
    kill "$ic3_pid" 2>/dev/null || true
    wait "$ic3_pid" 2>/dev/null || true
    ic3_pid=
    mv "$bmc_witness" "$witness"
    mv "$bmc_log" "$producer_log"
    printf 'bmc\n' >"$engine_file"
    exit 0
  fi

  if [[ "$ic3_finished" == false ]] && \
    ! jobs -pr | grep -qx "$ic3_pid"; then
    wait "$ic3_pid"
    ic3_pid=
    ic3_finished=true
    ic3_answer=$(tail -1 "$ic3_log")
    if [[ "$ic3_answer" == UNSAT ]]; then
      kill "$bmc_pid" 2>/dev/null || true
      wait "$bmc_pid" 2>/dev/null || true
      bmc_pid=
      mv "$ic3_witness" "$witness"
      mv "$ic3_log" "$producer_log"
      printf 'ic3\n' >"$engine_file"
      exit 0
    fi
    [[ "$ic3_answer" == SAT ]] || {
      echo "IC3 ended without SAT or UNSAT" >&2
      exit 2
    }
  fi
  sleep 0.01
done
