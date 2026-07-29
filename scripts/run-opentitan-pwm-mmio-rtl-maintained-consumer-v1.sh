#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 ANGR_IMAGE YOSYS Z3 FIRMWARE_ELF OUTPUT_DIRECTORY" >&2
  exit 2
fi

angr_image=$1
yosys=$2
z3=$3
firmware=$4
output=$5
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
expected_yosys=b8e7da6f40ae8f552c116bf6c359b07c6533e159
expected_z3='Z3 version 4.16.0'

[[ -x $yosys && -x $z3 ]] || {
  echo "Yosys and Z3 must be executable" >&2
  exit 2
}
[[ $($yosys -V) == *"git sha1 $expected_yosys,"* ]] || {
  echo "Yosys revision mismatch" >&2
  exit 2
}
[[ $($z3 --version) == "$expected_z3"* ]] || {
  echo "Z3 version mismatch" >&2
  exit 2
}
[[ -f $firmware && ! -L $firmware ]] || {
  echo "firmware ELF must be an ordinary file" >&2
  exit 2
}
[[ ! -e $output && ! -L $output ]] || {
  echo "refusing to overwrite maintained consumer output" >&2
  exit 2
}
docker image inspect "$angr_image" >/dev/null

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-mmio-maintained-consumer.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
mkdir "$scratch/result"

docker run --rm \
  -v "$repo/scripts/angr-guarded-mmio-domain-baseline-v1.py:/baseline.py:ro" \
  -v "$firmware:/firmware.elf:ro" \
  "$angr_image" \
  python /baseline.py /firmware.elf \
  >"$scratch/result/angr.txt"

GCC_PWM_HARNESS="$repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv" \
GCC_PWM_TOP=opentitan_pwm_firmware_trace_harness \
GCC_PWM_OUTPUT_FORMAT=smt2 \
  "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  "$yosys" \
  "$scratch/unused-2.smt2" \
  "$scratch/unused-4.smt2" \
  "$scratch/firmware-trace-6.smt2" \
  >"$scratch/result/yosys.txt"

python3 "$repo/scripts/mmio-rtl-yosys-z3-baseline-v1.py" \
  "$scratch/result/angr.txt" \
  "$scratch/firmware-trace-6.smt2" \
  "$z3" \
  >"$scratch/result/maintained.txt"

{
  echo "trust_boundary_consumer_version=1"
  echo "route=maintained-regeneration"
  echo "firmware_behaviors=7"
  grep -E '^(maintained_valid_rtl_members|maintained_invalid_rtl_members|maintained_rtl_transitions|maintained_rtl_observations|maintained_phase_cycle_classes|maintained_nonzero_traces)=' \
    "$scratch/result/maintained.txt" |
    sed 's/^maintained_//'
  grep '^maintained_rtl_trace=' "$scratch/result/maintained.txt" |
    sed 's/^maintained_rtl_trace=/rtl_trace_phase_cycle=/'
  echo "status=complete"
} >"$scratch/result/summary.txt"

grep -q '^valid_rtl_members=6$' "$scratch/result/summary.txt"
grep -q '^invalid_rtl_members=0$' "$scratch/result/summary.txt"
grep -q '^rtl_transitions=198$' "$scratch/result/summary.txt"
grep -q '^rtl_observations=204$' "$scratch/result/summary.txt"
[[ $(grep -c '^rtl_trace_phase_cycle=' "$scratch/result/summary.txt") -eq 6 ]]

cp "$scratch/firmware-trace-6.smt2" "$scratch/result/"
mv "$scratch/result" "$output"
cat "$output/summary.txt"
