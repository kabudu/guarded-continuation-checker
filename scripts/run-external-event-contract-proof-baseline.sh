#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 TOOL_DIR OUTPUT_DIR BINARY" >&2
  exit 2
fi

tools=$1
output=$2
binary=$3
cadical="$tools/bin/cadical"
drat_trim="$tools/bin/drat-trim"

if [[ ! -x "$binary" || ! -x "$cadical" || ! -x "$drat_trim" ]]; then
  echo "binary, CaDiCaL and DRAT-trim must be executable regular files" >&2
  exit 2
fi
if [[ -e "$output" ]]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-event-contract-external.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$output"

run_case() {
  name=$1
  model=$2
  contract=$3
  certificate="$scratch/$name.cert3"
  bundle="$scratch/$name-obligations"

  "$binary" certify-aiger-event-contract-v3 \
    "$model" 0 "$contract" "$certificate"
  "$binary" verify-aiger-event-contract-certificate-v3 \
    "$model" "$contract" "$certificate"
  "$binary" export-aiger-event-contract-v3-obligations \
    "$model" "$contract" "$certificate" "$bundle"
  scripts/run-external-predicate-proof-baseline.sh \
    "$bundle" "$cadical" "$drat_trim" "$output/$name.csv"
}

run_case interrupt-priority \
  examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
  examples/event-contracts/interrupt-priority-v1.contract
run_case actuator-interlock \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
  examples/event-contracts/actuator-interlock-v1.contract
run_case robot-recovery \
  examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
  examples/event-contracts/robot-recovery-v1.contract
run_case actuator-h1-unavoidable \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
  examples/event-contracts/actuator-h1-unavoidable-v1.contract

cp "$tools/versions.txt" "$output/tool-versions.txt"
echo "external event contract proof baseline status=VALID output=$output"
