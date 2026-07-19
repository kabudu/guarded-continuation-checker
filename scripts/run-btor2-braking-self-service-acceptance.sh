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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-braking-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

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

printf '%s\n' 'schema_version,case,source_sha256,bad_property,horizon,expected_result,actual_result,expected_backend,actual_backend,selection_reason,certificate_verified,status' >"$output"

run_case() {
  name=$1
  source=$2
  horizon=$3
  expected=$4
  expected_backend=$5
  expected_reason=$6
  certificate=$scratch/$name.btor2-cert
  produced=$("$binary" check-btor2-bounded "$source" 31 "$horizon" "$certificate")
  actual=$(printf '%s\n' "$produced" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  backend=$(printf '%s\n' "$produced" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  reason=$(printf '%s\n' "$produced" | sed -n 's/.* reason=\([^ ]*\).*/\1/p')
  test "$actual" = "$expected"
  test "$backend" = "$expected_backend"
  test "$reason" = "$expected_reason"
  "$binary" verify-btor2-bounded "$source" "$certificate" >/dev/null
  printf '1,%s,%s,31,%s,%s,%s,%s,%s,%s,true,accepted\n' \
    "$name" "$(sha256_file "$source")" "$horizon" "$expected" "$actual" \
    "$expected_backend" "$backend" "$reason" >>"$output"
}

run_case braking-safe examples/btor2/braking-controller-v1.btor2 255 SAFE braking-phases braking-phases-exact-safe
run_case braking-unsafe examples/btor2/braking-controller-v1.btor2 256 UNSAFE explicit-search specialised-inapplicable-or-intersecting
run_case motor-safe examples/btor2/motor-emergency-stop-v1.btor2 159 SAFE braking-phases braking-phases-exact-safe
run_case motor-unsafe examples/btor2/motor-emergency-stop-v1.btor2 160 UNSAFE explicit-search specialised-inapplicable-or-intersecting
run_case near-neighbour-safe examples/btor2/semi-implicit-braking-rejected-v1.btor2 127 SAFE explicit-search specialised-inapplicable-or-intersecting
run_case near-neighbour-unsafe examples/btor2/semi-implicit-braking-rejected-v1.btor2 128 UNSAFE explicit-search specialised-inapplicable-or-intersecting

echo "btor2 braking self-service acceptance status=ACCEPTED cases=6 output=$output"
