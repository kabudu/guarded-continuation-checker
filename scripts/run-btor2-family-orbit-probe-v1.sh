#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT.csv" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-pwm-crosstalk-impact

cargo run --quiet --release --manifest-path "$repo/Cargo.toml" \
  --example btor2_family_orbit_probe -- \
  "$fixture/generated/core-after.btor2" \
  "$fixture/generated/channel-after.btor2" \
  "$fixture/family-parameters-v1.txt" \
  "$1"
