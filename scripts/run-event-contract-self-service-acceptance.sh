#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 BINARY OUTPUT.csv" >&2
    exit 2
fi

binary=$1
output=$2
if [ ! -x "$binary" ]; then
    echo "GCC binary must be executable" >&2
    exit 2
fi
if [ -e "$output" ]; then
    echo "refusing to overwrite output: $output" >&2
    exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-event-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT
temporary="$scratch/acceptance.csv"
printf '%s\n' 'schema_version,case,input_sha256,contract_sha256,expected_result,actual_result,backend,reason,certificate_verified,report_replayed,status' >"$temporary"

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        echo "a SHA-256 utility (sha256sum or shasum) is required" >&2
        exit 2
    fi
}

run_case() {
    name=$1
    model=$2
    contract=$3
    expected=$4
    report="$scratch/$name.report"
    certificate="$scratch/$name.cert3"

    "$binary" verify-aiger-event-contract-portfolio \
        "$model" 0 "$contract" "$report" "$certificate" >/dev/null
    "$binary" verify-aiger-event-contract-portfolio-report \
        "$model" 0 "$contract" "$report" "$certificate" >/dev/null
    actual=$(sed -n 's/^result=//p' "$report")
    backend=$(sed -n 's/^backend=//p' "$report")
    reason=$(sed -n 's/^reason=//p' "$report")
    verified=$(sed -n 's/^certificate_verified=//p' "$report")
    if [ "$actual" != "$expected" ] || [ "$verified" != "1" ]; then
        echo "acceptance mismatch for $name" >&2
        exit 1
    fi
    printf '1,%s,%s,%s,%s,%s,%s,%s,true,true,accepted\n' \
        "$name" \
        "$(sha256_file "$model")" \
        "$(sha256_file "$contract")" \
        "$expected" "$actual" "$backend" "$reason" >>"$temporary"
}

run_case interrupt-priority-avoidable \
    examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
    examples/event-contracts/interrupt-priority-v1.contract avoidable
run_case actuator-interlock-avoidable \
    examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
    examples/event-contracts/actuator-interlock-v1.contract avoidable
run_case robot-recovery-avoidable \
    examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
    examples/event-contracts/robot-recovery-v1.contract avoidable
run_case interrupt-hazard-unavoidable \
    examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag \
    examples/event-contracts/interrupt-hazard-h1-v1.contract unavoidable
run_case actuator-hazard-unavoidable \
    examples/products/actuator-controller/firmware/dense-actuator-interlock.aag \
    examples/event-contracts/actuator-h1-unavoidable-v1.contract unavoidable
run_case robot-hazard-unavoidable \
    examples/products/mobile-robot/firmware/dense-sensor-fusion.aag \
    examples/event-contracts/robot-hazard-h1-v1.contract unavoidable

mv "$temporary" "$output"
echo "event-contract self-service acceptance status=ACCEPTED cases=6 output=$output"
