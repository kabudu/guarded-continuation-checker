#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
model="$root/corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"

if [ -e "$output" ]; then
  echo "MMIO-to-RTL output already exists: $output" >&2
  exit 2
fi

"$root/scripts/build-opentitan-pwm-guarded-mmio-reference-v1.sh" "$output"

cargo build \
  --quiet \
  --manifest-path "$root/Cargo.toml" \
  --example compiled_mmio_pwm_rtl
mapper="$root/target/debug/examples/compiled_mmio_pwm_rtl"

for profile in o0 o2; do
  "$mapper" \
    "$output/$profile/firmware.bin" \
    "$output/$profile/firmware.symbols.txt" \
    "$model" \
    "$output/$profile/compiled-mmio-rtl-certificate-v1.bin" \
    > "$output/$profile/mmio-rtl-replay-v1.txt"
  grep -E '^(compiled_mmio_pwm_rtl_version|rtl_model_sha256|valid_rtl_members|invalid_rtl_members|rtl_trace_base|rtl_trace_phase_cycle|composed_certificate_(valid_rtl_members|invalid_rtl_members|rtl_transitions|rtl_observations|clean_cycles_identical|source_drift_cases|semantic_drift_cases)|status)=' \
    "$output/$profile/mmio-rtl-replay-v1.txt" \
    > "$output/$profile/mmio-rtl-semantics-v1.txt"
done

cmp \
  "$output/o0/mmio-rtl-semantics-v1.txt" \
  "$output/o2/mmio-rtl-semantics-v1.txt"

phase_rows=$(grep -c '^rtl_trace_phase_cycle=' "$output/o0/mmio-rtl-semantics-v1.txt")
if [ "$phase_rows" -ne 6 ]; then
  echo "phase-cycle trace count differs from six valid firmware members" >&2
  exit 1
fi
phase_classes=$(
  grep '^rtl_trace_phase_cycle=' "$output/o0/mmio-rtl-semantics-v1.txt" |
    cut -d, -f2- |
    LC_ALL=C sort -u |
    wc -l |
    tr -d ' '
)
if [ "$phase_classes" -lt 2 ]; then
  echo "phase-cycle continuation did not distinguish valid firmware classes" >&2
  exit 1
fi
nonzero_rows=$(
  grep '^rtl_trace_phase_cycle=' "$output/o0/mmio-rtl-semantics-v1.txt" |
    grep -Ec ':0[1-9a-f]'
)
if [ "$nonzero_rows" -ne 6 ]; then
  echo "phase-cycle continuation left a valid firmware trace observationally empty" >&2
  exit 1
fi
if grep '^rtl_trace_base=' "$output/o0/mmio-rtl-semantics-v1.txt" |
  grep -Eq ':0[1-9a-f]'; then
  echo "frozen base window unexpectedly changed its negative result" >&2
  exit 1
fi

{
  echo "opentitan_pwm_mmio_rtl_mapping_version=1"
  sha256sum \
    "$model" \
    "$output/o0/firmware.bin" \
    "$output/o0/firmware.symbols.txt" \
    "$output/o0/compiled-mmio-rtl-certificate-v1.bin" \
    "$output/o0/mmio-rtl-replay-v1.txt" \
    "$output/o2/firmware.bin" \
    "$output/o2/firmware.symbols.txt" \
    "$output/o2/compiled-mmio-rtl-certificate-v1.bin" \
    "$output/o2/mmio-rtl-replay-v1.txt"
  echo "profiles_semantically_identical=true"
  echo "phase_cycle_classes=$phase_classes"
  echo "phase_cycle_nonzero_traces=$nonzero_rows"
  echo "status=complete"
} > "$output/mmio-rtl-manifest-v1.txt"

echo "opentitan-pwm-mmio-rtl-mapping=PASS output=$output"
