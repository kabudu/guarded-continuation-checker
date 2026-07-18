#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$repo/results/predicate-proof-relation-reproduction}"
repeats="${2:-10}"

mkdir -p "$output_dir"
cd "$repo"
cargo build --release
binary="$repo/target/release/continuation-quotient-sat"

"$binary" benchmark-aiger-predicate-proof-relation \
  examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
  examples/predicate-certificate-cost/interrupt-h8-avoidable.transcript \
  "$repeats" "$output_dir/interrupt.csv"

"$binary" benchmark-aiger-predicate-proof-relation \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
  examples/predicate-certificate-cost/actuator-h16-avoidable.transcript \
  "$repeats" "$output_dir/actuator.csv"

"$binary" benchmark-aiger-predicate-proof-relation \
  examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
  examples/predicate-certificate-cost/sensor-h32-avoidable.transcript \
  "$repeats" "$output_dir/sensor.csv"

for entry in \
  "interrupt-controller/firmware/dense-interrupt-arbiter.aag interrupt-h8-avoidable.transcript interrupt-unconstrained.csv" \
  "actuator-controller/firmware/dense-actuator-interlock.aag actuator-h16-avoidable.transcript actuator-unconstrained.csv" \
  "mobile-robot/firmware/dense-sensor-fusion.aag sensor-h32-avoidable.transcript sensor-unconstrained.csv" \
  "interrupt-controller/firmware/dense-interrupt-arbiter.aag interrupt-terminal-constrained.transcript interrupt-constrained.csv" \
  "actuator-controller/firmware/dense-actuator-interlock.aag actuator-terminal-constrained.transcript actuator-constrained.csv" \
  "mobile-robot/firmware/dense-sensor-fusion.aag sensor-terminal-constrained.transcript sensor-constrained.csv"
do
  read -r model transcript output <<<"$entry"
  "$binary" benchmark-aiger-predicate-proof-terminal \
    "examples/products/$model" 0 "examples/predicate-certificate-cost/$transcript" \
    "$repeats" "$output_dir/$output"
done
