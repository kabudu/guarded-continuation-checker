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

scratch=${TMPDIR:-/tmp}/gcc-btor2-braking-cohort-$$
mkdir "$scratch"
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

printf '%s\n' 'schema_version,model,bad_property,horizon,expected,actual,backend,reason,explicit_bytes,portfolio_bytes,byte_reduction_percent,verified,status' >"$output"

run_case() {
  name=$1
  model=$2
  horizon=$3
  expected=$4
  expected_backend=$5
  expected_reason=$6
  explicit=$scratch/$name-$horizon.search-cert
  portfolio=$scratch/$name-$horizon.btor2-cert
  "$binary" search-btor2 "$model" 31 "$horizon" "$explicit" >/dev/null
  produced=$("$binary" check-btor2-bounded "$model" 31 "$horizon" "$portfolio")
  actual=$(printf '%s\n' "$produced" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  backend=$(printf '%s\n' "$produced" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  reason=$(printf '%s\n' "$produced" | sed -n 's/.* reason=\([^ ]*\).*/\1/p')
  test "$actual" = "$expected"
  test "$backend" = "$expected_backend"
  test "$reason" = "$expected_reason"
  "$binary" verify-btor2-bounded "$model" "$portfolio" >/dev/null
  explicit_bytes=$(wc -c <"$explicit" | tr -d ' ')
  portfolio_bytes=$(wc -c <"$portfolio" | tr -d ' ')
  reduction=$(awk -v old="$explicit_bytes" -v new="$portfolio_bytes" 'BEGIN { printf "%.2f", 100 * (old - new) / old }')
  printf '1,%s,31,%s,%s,%s,%s,%s,%s,%s,%s,true,ok\n' \
    "$name" "$horizon" "$expected" "$actual" "$backend" "$reason" \
    "$explicit_bytes" "$portfolio_bytes" "$reduction" >>"$output"
}

run_case braking-controller examples/btor2/braking-controller-v1.btor2 255 SAFE braking-phases braking-phases-exact-safe
run_case braking-controller examples/btor2/braking-controller-v1.btor2 256 UNSAFE explicit-search specialised-inapplicable-or-intersecting
run_case motor-emergency-stop examples/btor2/motor-emergency-stop-v1.btor2 159 SAFE braking-phases braking-phases-exact-safe
run_case motor-emergency-stop examples/btor2/motor-emergency-stop-v1.btor2 160 UNSAFE explicit-search specialised-inapplicable-or-intersecting
run_case semi-implicit-braking examples/btor2/semi-implicit-braking-rejected-v1.btor2 127 SAFE explicit-search specialised-inapplicable-or-intersecting
run_case semi-implicit-braking examples/btor2/semi-implicit-braking-rejected-v1.btor2 128 UNSAFE explicit-search specialised-inapplicable-or-intersecting

echo "btor2_braking_cohort=PASS output=$output"
