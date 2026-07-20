#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 produce|consume" >&2; exit 2; }
mode=$1
case $mode in
  produce)
    for property in 11 12 13 14 15 16; do
      witness="/out/property-${property}.witness.aag"
      [[ ! -e "$witness" ]] || { echo "refusing to overwrite $witness" >&2; exit 2; }
      model="/repo/corpus/rtl/wmcontroller/certified-baseline-v1/property-${property}.aag"
      bmc_witness="/out/.property-${property}.bmc.aag"
      ic3_witness="/out/.property-${property}.ic3.aag"
      bmc_log="/out/property-${property}.bmc.log"
      ic3_log="/out/property-${property}.ic3.log"
      /tools/ric3 check "$model" --cert "$bmc_witness" --ui false bmc > "$bmc_log" 2>&1 &
      bmc_pid=$!
      /tools/ric3 check "$model" --cert "$ic3_witness" --ui false ic3 > "$ic3_log" 2>&1 &
      ic3_pid=$!
      ic3_finished=false
      while true; do
        if ! jobs -pr | grep -qx "$bmc_pid"; then
          wait "$bmc_pid"
          [[ $(tail -1 "$bmc_log") == SAT ]] || { echo "BMC ended without SAT" >&2; exit 2; }
          kill "$ic3_pid" 2>/dev/null || true
          wait "$ic3_pid" 2>/dev/null || true
          mv "$bmc_witness" "$witness"
          cp "$bmc_log" "/out/property-${property}.producer.log"
          break
        fi
        if [[ $ic3_finished == false ]] && ! jobs -pr | grep -qx "$ic3_pid"; then
          wait "$ic3_pid"
          ic3_finished=true
          ic3_answer=$(tail -1 "$ic3_log")
          if [[ $ic3_answer == UNSAT ]]; then
            kill "$bmc_pid" 2>/dev/null || true
            wait "$bmc_pid" 2>/dev/null || true
            mv "$ic3_witness" "$witness"
            cp "$ic3_log" "/out/property-${property}.producer.log"
            break
          fi
          [[ $ic3_answer == SAT ]] || { echo "IC3 ended without SAT or UNSAT" >&2; exit 2; }
        fi
        sleep 0.01
      done
      rm -f "$bmc_witness" "$ic3_witness"
      expected=SAT
      [[ $property -ge 15 ]] && expected=UNSAT
      [[ $(tail -1 "/out/property-${property}.producer.log") == "$expected" ]]
    done
    ;;
  consume)
    for property in 11 12 13 14 15 16; do
      model="/repo/corpus/rtl/wmcontroller/certified-baseline-v1/property-${property}.aag"
      witness="/out/property-${property}.witness.aag"
      [[ -f "$witness" && ! -L "$witness" ]] || { echo "missing witness $witness" >&2; exit 2; }
      if [[ $property -le 14 ]]; then
        case $property in
          11) expected_frame=4 ;;
          12) expected_frame=7 ;;
          13 | 14) expected_frame=15 ;;
        esac
        actual_frame=$(awk 'NR >= 4 && $0 != "." {vectors++} END {print vectors - 1}' "$witness")
        [[ $actual_frame == "$expected_frame" ]] || {
          echo "property $property witness frame mismatch: expected $expected_frame, found $actual_frame" >&2
          exit 2
        }
        /cert/aigsim -c -m "$model" "$witness" \
          > "/out/property-${property}.consumer.log" 2>&1
      else
        /cert/check "$model" "$witness" \
          > "/out/property-${property}.consumer.log" 2>&1
        grep -q '^check: valid witness$' "/out/property-${property}.consumer.log"
      fi
    done
    ;;
  *) echo "unknown mode: $mode" >&2; exit 2 ;;
esac
