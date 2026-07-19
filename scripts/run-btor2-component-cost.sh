#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_RELEASE_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
test -x "$binary"
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

trials=21
middle=11
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-component-cost.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
contract=examples/btor2/components/braking-motion-contract-v1.txt

median() {
  sort -n "$1" | sed -n "${middle}p"
}

printf '%s\n' 'schema_version,case,horizon,trials,explicit_bytes,monolithic_bytes,component_bytes,explicit_verify_median_micros,monolithic_verify_median_micros,component_verify_median_micros,component_vs_explicit_speedup,component_vs_monolithic_ratio,status' >"$output"

run_case() {
  name=$1
  controller=$2
  plant=$3
  monolithic=$4
  horizon=$5
  explicit=$scratch/$name.search-cert
  monolithic_certificate=$scratch/$name.btor2-cert
  component=$scratch/$name.component-cert
  explicit_times=$scratch/$name-explicit.times
  monolithic_times=$scratch/$name-monolithic.times
  component_times=$scratch/$name-component.times

  "$binary" search-btor2 "$monolithic" 31 "$horizon" "$explicit" >/dev/null
  "$binary" check-btor2-bounded "$monolithic" 31 "$horizon" "$monolithic_certificate" \
    | grep -q ' backend=braking-phases '
  "$binary" check-btor2-components \
    "$controller" "$plant" "$contract" "$horizon" "$component" \
    | grep -q ' backend=phase-contract '
  iteration=0
  while [ "$iteration" -lt "$trials" ]; do
    "$binary" verify-btor2-search "$monolithic" "$explicit" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$explicit_times"
    "$binary" verify-btor2-bounded "$monolithic" "$monolithic_certificate" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$monolithic_times"
    "$binary" verify-btor2-components \
      "$controller" "$plant" "$contract" "$component" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$component_times"
    iteration=$((iteration + 1))
  done
  for times in "$explicit_times" "$monolithic_times" "$component_times"; do
    test "$(wc -l <"$times" | tr -d ' ')" -eq "$trials"
  done
  explicit_median=$(median "$explicit_times")
  monolithic_median=$(median "$monolithic_times")
  component_median=$(median "$component_times")
  explicit_speedup=$(awk -v slow="$explicit_median" -v fast="$component_median" \
    'BEGIN { if (fast == 0) print "inf"; else printf "%.2f", slow / fast }')
  monolithic_ratio=$(awk -v mono="$monolithic_median" -v component="$component_median" \
    'BEGIN { if (mono == 0) print "inf"; else printf "%.2f", component / mono }')
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,ok\n' \
    "$name" "$horizon" "$trials" \
    "$(wc -c <"$explicit" | tr -d ' ')" \
    "$(wc -c <"$monolithic_certificate" | tr -d ' ')" \
    "$(wc -c <"$component" | tr -d ' ')" \
    "$explicit_median" "$monolithic_median" "$component_median" \
    "$explicit_speedup" "$monolithic_ratio" >>"$output"
}

run_case braking-base \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/motion-plant-v1.btor2 \
  examples/btor2/braking-controller-v1.btor2 255
run_case braking-fast-plant \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/fast-motion-plant-v1.btor2 \
  examples/btor2/fast-braking-controller-v1.btor2 127
run_case motor-stop \
  examples/btor2/components/motor-stop-controller-v1.btor2 \
  examples/btor2/components/motor-plant-v1.btor2 \
  examples/btor2/motor-emergency-stop-v1.btor2 159

echo "btor2_component_cost=PASS trials=$trials output=$output"
