#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 YOSYS OUTPUT_2.btor2 OUTPUT_4.btor2 OUTPUT_6.btor2" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
GCC_PWM_HARNESS="$repo/corpus/rtl/opentitan-pwm-channel-family/symbolic-class-harness.sv" \
GCC_PWM_TOP=opentitan_pwm_symbolic_class_harness \
  "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" "$@"
