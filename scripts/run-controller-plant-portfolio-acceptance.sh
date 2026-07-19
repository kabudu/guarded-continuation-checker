#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
test -x "$binary"
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-portfolio.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
admitted_manifest=corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
fallback_manifest=corpus/controller-plant-portfolio/manifest-v1.txt
admitted_artifact=$scratch/admitted.controller-plant
fallback_artifact=$scratch/fallback.controller-plant

"$binary" controller-plant-portfolio-cli-version | grep -q \
  '^controller_plant_portfolio_cli_version=1 .* routing=static fallback=exact unsupported=fail-closed$'
admitted=$("$binary" certify-controller-plant-portfolio \
  "$admitted_manifest" "$admitted_artifact")
fallback=$("$binary" certify-controller-plant-portfolio \
  "$fallback_manifest" "$fallback_artifact")
admitted_verified=$("$binary" verify-controller-plant-portfolio \
  "$admitted_manifest" "$admitted_artifact")
fallback_verified=$("$binary" verify-controller-plant-portfolio \
  "$fallback_manifest" "$fallback_artifact")

printf '%s\n' "$admitted" | grep -q \
  'backend=MTBDD reason=mtbdd-admitted members=6 safe=2 unsafe=4 '
printf '%s\n' "$admitted_verified" | grep -q \
  '^controller-plant-portfolio status=VERIFIED .* backend=MTBDD reason=mtbdd-admitted '
printf '%s\n' "$fallback" | grep -q \
  'backend=DIRECT_EXACT reason=boundary-limit members=2 safe=1 unsafe=1 '
printf '%s\n' "$fallback_verified" | grep -q \
  '^controller-plant-portfolio status=VERIFIED .* backend=DIRECT_EXACT reason=boundary-limit '
printf '%s\n' "$fallback_verified" | grep -q \
  '^controller-plant-portfolio-member index=0 answer=SAFE horizon=4 bad_frame=none '
printf '%s\n' "$fallback_verified" | grep -q \
  '^controller-plant-portfolio-member index=1 answer=UNSAFE horizon=4 bad_frame=0 '

mutated=$scratch/mutated.controller-plant
cp "$fallback_artifact" "$mutated"
printf '\001' | dd of="$mutated" bs=1 seek=40 conv=notrunc 2>/dev/null
if "$binary" verify-controller-plant-portfolio \
  "$fallback_manifest" "$mutated" >/dev/null 2>&1; then
  echo "mutated portfolio unexpectedly verified" >&2
  exit 1
fi
if "$binary" certify-controller-plant-portfolio \
  "$fallback_manifest" "$fallback_artifact" >/dev/null 2>&1; then
  echo "existing portfolio was unexpectedly overwritten" >&2
  exit 1
fi

printf '%s\n' \
  'schema_version,case,expected_backend,actual_backend,expected_answer,actual_answer,verified,status' \
  '1,admitted-route,MTBDD,MTBDD,4-UNSAFE-2-SAFE,4-UNSAFE-2-SAFE,true,accepted' \
  '1,fallback-route,DIRECT_EXACT,DIRECT_EXACT,1-UNSAFE-1-SAFE,1-UNSAFE-1-SAFE,true,accepted' \
  '1,fallback-safe,DIRECT_EXACT,DIRECT_EXACT,SAFE,SAFE,true,accepted' \
  '1,fallback-unsafe,DIRECT_EXACT,DIRECT_EXACT,UNSAFE@0,UNSAFE@0,true,accepted' \
  '1,artifact-mutation,REJECTED,REJECTED,REJECTED,REJECTED,false,accepted' \
  '1,output-collision,REJECTED,REJECTED,REJECTED,REJECTED,false,accepted' \
  >"$output"

echo "controller plant portfolio acceptance status=ACCEPTED cases=6 output=$output"
