#!/bin/sh
set -eu

output_dir=${1:-results/event-contract-certificate-v3-balanced-v1}
repeats=${2:-10}
binary=${GCC_BINARY:-target/release/guarded-continuation-checker}

if [ -e "$output_dir" ]; then
    echo "refusing to overwrite output directory: $output_dir" >&2
    exit 2
fi

mkdir -p "$output_dir"

run_case() {
    name=$1
    model=$2
    contract=$3
    "$binary" benchmark-aiger-event-contract-certificate-v3-cost \
        "$model" 0 "$contract" "$repeats" "$output_dir/$name.csv"
}

run_case interrupt-priority-avoidable \
    examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
    examples/event-contracts/interrupt-priority-v1.contract
run_case actuator-interlock-avoidable \
    examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
    examples/event-contracts/actuator-interlock-v1.contract
run_case robot-recovery-avoidable \
    examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
    examples/event-contracts/robot-recovery-v1.contract
run_case interrupt-hazard-unavoidable \
    examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
    examples/event-contracts/interrupt-hazard-h1-v1.contract
run_case actuator-hazard-unavoidable \
    examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
    examples/event-contracts/actuator-h1-unavoidable-v1.contract
run_case robot-hazard-unavoidable \
    examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
    examples/event-contracts/robot-hazard-h1-v1.contract
