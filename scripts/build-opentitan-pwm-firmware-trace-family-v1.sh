#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 YOSYS OUTPUT_6.btor2" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-pwm-firmware-trace.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
GCC_PWM_HARNESS="$repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv" \
GCC_PWM_TOP=opentitan_pwm_firmware_trace_harness \
  "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  "$1" "$scratch/unused-2.btor2" "$scratch/unused-4.btor2" "$2"
