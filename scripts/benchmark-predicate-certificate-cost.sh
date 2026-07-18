#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$repo/results/predicate-certificate-cost-reproduction}"
repeats="${2:-10}"

mkdir -p "$output_dir"
cd "$repo"
cargo build --release
binary="$repo/target/release/continuation-quotient-sat"

"$binary" benchmark-aiger-predicate-certificate-cost \
  examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag 0 \
  examples/predicate-certificate-cost/interrupt-h8-avoidable.transcript \
  "$repeats" "$output_dir/interrupt-h8.csv"

"$binary" benchmark-aiger-predicate-certificate-cost \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag 0 \
  examples/predicate-certificate-cost/actuator-h16-avoidable.transcript \
  "$repeats" "$output_dir/actuator-h16.csv"

"$binary" benchmark-aiger-predicate-certificate-cost \
  examples/products/mobile-robot/firmware/dense-sensor-fusion.aag 0 \
  examples/predicate-certificate-cost/sensor-h32-avoidable.transcript \
  "$repeats" "$output_dir/sensor-h32.csv"

"$binary" benchmark-aiger-predicate-certificate-cost \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag 0 \
  examples/predicate-certificate-cost/actuator-h1-unavoidable.transcript \
  "$repeats" "$output_dir/actuator-h1-unavoidable.csv"

"$binary" benchmark-aiger-predicate-certificate-v2-cost \
  examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag 0 \
  examples/predicate-certificate-cost/interrupt-h8-avoidable.transcript \
  "$repeats" "$output_dir/interrupt-h8-v2.csv"

"$binary" benchmark-aiger-predicate-certificate-v2-cost \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag 0 \
  examples/predicate-certificate-cost/actuator-h16-avoidable.transcript \
  "$repeats" "$output_dir/actuator-h16-v2.csv"

"$binary" benchmark-aiger-predicate-certificate-v2-cost \
  examples/products/mobile-robot/firmware/dense-sensor-fusion.aag 0 \
  examples/predicate-certificate-cost/sensor-h32-avoidable.transcript \
  "$repeats" "$output_dir/sensor-h32-v2.csv"

"$binary" benchmark-aiger-predicate-certificate-v2-cost \
  examples/products/actuator-controller/firmware/dense-actuator-interlock.aag 0 \
  examples/predicate-certificate-cost/actuator-h1-unavoidable.transcript \
  "$repeats" "$output_dir/actuator-h1-unavoidable-v2.csv"
