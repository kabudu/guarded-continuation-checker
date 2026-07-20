#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
trials=${TRIALS:-3}
test -x "$binary"
case $trials in
  '' | *[!0-9]* | 0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
if [ "$trials" -gt 100 ]; then
  echo "TRIALS exceeds the static limit of 100" >&2
  exit 2
fi
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-phases.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
admitted=corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
fallback=corpus/controller-plant-portfolio/manifest-v1.txt

printf '%s\n' \
  'schema_version,route,operation,trial,load_micros,artifact_micros,verification_micros,write_micros,elapsed_micros,backend,reason,status' \
  >"$output"

field() {
  key=$1
  shift
  printf '%s\n' "$*" | tr ' ' '\n' | sed -n "s/^${key}=//p"
}

run_case() {
  route=$1
  manifest=$2
  trial=$3
  artifact=$scratch/$route-$trial.controller-plant
  produced=$("$binary" certify-controller-plant-portfolio "$manifest" "$artifact")
  verified=$("$binary" verify-controller-plant-portfolio "$manifest" "$artifact")
  for operation in produce verify; do
    if [ "$operation" = produce ]; then
      row=$produced
    else
      row=$verified
    fi
    summary=$(printf '%s\n' "$row" | sed -n '1p')
    printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,ok\n' \
      "$route" "$operation" "$trial" \
      "$(field load_micros "$summary")" \
      "$(field artifact_micros "$summary")" \
      "$(field verification_micros "$summary")" \
      "$(field write_micros "$summary")" \
      "$(field elapsed_micros "$summary")" \
      "$(field backend "$summary")" \
      "$(field reason "$summary")" >>"$output"
  done
}

trial=1
while [ "$trial" -le "$trials" ]; do
  run_case admitted "$admitted" "$trial"
  run_case fallback "$fallback" "$trial"
  trial=$((trial + 1))
done

echo "controller plant portfolio phases status=MEASURED trials=$trials output=$output"
