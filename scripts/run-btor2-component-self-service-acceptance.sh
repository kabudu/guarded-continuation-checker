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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-component-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
contract=examples/btor2/components/braking-motion-contract-v1.txt

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "a SHA-256 utility is required" >&2
    exit 2
  fi
}

printf '%s\n' 'schema_version,case,controller_sha256,plant_sha256,contract_sha256,horizon,expected_result,actual_result,expected_backend,actual_backend,selection_reason,certificate_verified,status' >"$output"

run_case() {
  name=$1
  controller=$2
  plant=$3
  horizon=$4
  expected=$5
  expected_backend=$6
  expected_reason=$7
  certificate=$scratch/$name.component-cert
  produced=$("$binary" check-btor2-components \
    "$controller" "$plant" "$contract" "$horizon" "$certificate")
  actual=$(printf '%s\n' "$produced" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  backend=$(printf '%s\n' "$produced" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  reason=$(printf '%s\n' "$produced" | sed -n 's/.* reason=\([^ ]*\).*/\1/p')
  test "$actual" = "$expected"
  test "$backend" = "$expected_backend"
  test "$reason" = "$expected_reason"
  "$binary" verify-btor2-components \
    "$controller" "$plant" "$contract" "$certificate" >/dev/null
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' \
    "$name" "$(sha256_file "$controller")" "$(sha256_file "$plant")" \
    "$(sha256_file "$contract")" "$horizon" "$expected" "$actual" \
    "$expected_backend" "$backend" "$reason" >>"$output"
}

run_case braking-safe \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/motion-plant-v1.btor2 \
  255 SAFE phase-contract exact-phase-contract-safe
run_case braking-unsafe \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/motion-plant-v1.btor2 \
  256 UNSAFE composed-search specialised-inapplicable-or-intersecting
run_case reused-controller-fast-plant-safe \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/fast-motion-plant-v1.btor2 \
  127 SAFE phase-contract exact-phase-contract-safe
run_case reused-controller-fast-plant-unsafe \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/fast-motion-plant-v1.btor2 \
  128 UNSAFE composed-search specialised-inapplicable-or-intersecting
run_case motor-safe \
  examples/btor2/components/motor-stop-controller-v1.btor2 \
  examples/btor2/components/motor-plant-v1.btor2 \
  159 SAFE phase-contract exact-phase-contract-safe
run_case motor-unsafe \
  examples/btor2/components/motor-stop-controller-v1.btor2 \
  examples/btor2/components/motor-plant-v1.btor2 \
  160 UNSAFE composed-search specialised-inapplicable-or-intersecting
run_case near-neighbour-safe \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/semi-implicit-motion-plant-v1.btor2 \
  127 SAFE composed-search specialised-inapplicable-or-intersecting
run_case near-neighbour-unsafe \
  examples/btor2/components/braking-controller-v1.btor2 \
  examples/btor2/components/semi-implicit-motion-plant-v1.btor2 \
  128 UNSAFE composed-search specialised-inapplicable-or-intersecting

echo "btor2 component self-service acceptance status=ACCEPTED cases=8 output=$output"
