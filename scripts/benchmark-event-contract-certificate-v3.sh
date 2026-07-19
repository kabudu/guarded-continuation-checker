#!/bin/sh
set -eu

output_dir=${1:-results/event-contract-certificate-v3-cost}
repeats=${2:-10}
binary=${GCC_BINARY:-target/release/guarded-continuation-checker}

if [ -e "$output_dir" ]; then
    echo "refusing to overwrite output directory: $output_dir" >&2
    exit 2
fi

mkdir -p "$output_dir"

"$binary" benchmark-aiger-event-contract-certificate-v3-cost \
    examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
    0 examples/event-contracts/interrupt-priority-v1.contract "$repeats" \
    "$output_dir/interrupt-priority.csv"

"$binary" benchmark-aiger-event-contract-certificate-v3-cost \
    examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
    0 examples/event-contracts/actuator-interlock-v1.contract "$repeats" \
    "$output_dir/actuator-interlock.csv"

"$binary" benchmark-aiger-event-contract-certificate-v3-cost \
    examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
    0 examples/event-contracts/robot-recovery-v1.contract "$repeats" \
    "$output_dir/robot-recovery.csv"

"$binary" benchmark-aiger-event-contract-certificate-v3-cost \
    examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
    0 examples/event-contracts/actuator-h1-unavoidable-v1.contract "$repeats" \
    "$output_dir/actuator-h1-unavoidable.csv"
