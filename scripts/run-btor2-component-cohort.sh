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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-component-cohort.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
contract=examples/btor2/components/braking-motion-contract-v1.txt

printf '%s\n' 'schema_version,case,horizon,expected,component_result,component_backend,reason,monolithic_result,monolithic_backend,explicit_bytes,component_bytes,monolithic_portfolio_bytes,component_vs_explicit_reduction_percent,component_verified,monolithic_verified,status' >"$output"

run_case() {
  name=$1
  controller=$2
  plant=$3
  monolithic=$4
  horizon=$5
  expected=$6
  expected_component_backend=$7
  expected_monolithic_backend=$8
  expected_reason=$9
  component_certificate=$scratch/$name-$horizon.component-cert
  monolithic_certificate=$scratch/$name-$horizon.btor2-cert
  explicit_certificate=$scratch/$name-$horizon.search-cert

  component_output=$("$binary" check-btor2-components \
    "$controller" "$plant" "$contract" "$horizon" "$component_certificate")
  monolithic_output=$("$binary" check-btor2-bounded \
    "$monolithic" 31 "$horizon" "$monolithic_certificate")
  "$binary" search-btor2 "$monolithic" 31 "$horizon" "$explicit_certificate" >/dev/null

  component_result=$(printf '%s\n' "$component_output" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  component_backend=$(printf '%s\n' "$component_output" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  reason=$(printf '%s\n' "$component_output" | sed -n 's/.* reason=\([^ ]*\).*/\1/p')
  monolithic_result=$(printf '%s\n' "$monolithic_output" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  monolithic_backend=$(printf '%s\n' "$monolithic_output" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  test "$component_result" = "$expected"
  test "$monolithic_result" = "$expected"
  test "$component_backend" = "$expected_component_backend"
  test "$monolithic_backend" = "$expected_monolithic_backend"
  test "$reason" = "$expected_reason"
  "$binary" verify-btor2-components \
    "$controller" "$plant" "$contract" "$component_certificate" >/dev/null
  "$binary" verify-btor2-bounded "$monolithic" "$monolithic_certificate" >/dev/null

  explicit_bytes=$(wc -c <"$explicit_certificate" | tr -d ' ')
  component_bytes=$(wc -c <"$component_certificate" | tr -d ' ')
  monolithic_bytes=$(wc -c <"$monolithic_certificate" | tr -d ' ')
  reduction=$(awk -v old="$explicit_bytes" -v new="$component_bytes" \
    'BEGIN { printf "%.2f", 100 * (old - new) / old }')
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,true,ok\n' \
    "$name" "$horizon" "$expected" "$component_result" "$component_backend" \
    "$reason" "$monolithic_result" "$monolithic_backend" "$explicit_bytes" \
    "$component_bytes" "$monolithic_bytes" "$reduction" >>"$output"
}

run_case braking-base \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/motion-plant-v1.btor2 \
  examples/btor2/braking-controller-v1.btor2 \
  255 SAFE phase-contract braking-phases exact-phase-contract-safe
run_case braking-base \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/motion-plant-v1.btor2 \
  examples/btor2/braking-controller-v1.btor2 \
  256 UNSAFE composed-search explicit-search specialised-inapplicable-or-intersecting
run_case braking-fast-plant \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/fast-motion-plant-v1.btor2 \
  examples/btor2/fast-braking-controller-v1.btor2 \
  127 SAFE phase-contract braking-phases exact-phase-contract-safe
run_case braking-fast-plant \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/fast-motion-plant-v1.btor2 \
  examples/btor2/fast-braking-controller-v1.btor2 \
  128 UNSAFE composed-search explicit-search specialised-inapplicable-or-intersecting
run_case motor-stop \
  examples/btor2/components/motor-stop-controller-v1.btor2 \
  examples/btor2/components/motor-plant-v1.btor2 \
  examples/btor2/motor-emergency-stop-v1.btor2 \
  159 SAFE phase-contract braking-phases exact-phase-contract-safe
run_case motor-stop \
  examples/btor2/components/motor-stop-controller-v1.btor2 \
  examples/btor2/components/motor-plant-v1.btor2 \
  examples/btor2/motor-emergency-stop-v1.btor2 \
  160 UNSAFE composed-search explicit-search specialised-inapplicable-or-intersecting
run_case semi-implicit-control \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/semi-implicit-motion-plant-v1.btor2 \
  examples/btor2/semi-implicit-braking-rejected-v1.btor2 \
  127 SAFE composed-search explicit-search specialised-inapplicable-or-intersecting
run_case semi-implicit-control \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/semi-implicit-motion-plant-v1.btor2 \
  examples/btor2/semi-implicit-braking-rejected-v1.btor2 \
  128 UNSAFE composed-search explicit-search specialised-inapplicable-or-intersecting

echo "btor2_component_cohort=PASS output=$output"
