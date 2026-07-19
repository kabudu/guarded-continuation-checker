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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-motion-acceptance.XXXXXXXX")
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
  bad=$3
  horizon=$4
  expected=$5
  expected_backend=$6
  expected_reason=$7
  certificate=$scratch/$name.btor2-cert
  produced=$("$binary" check-btor2-bounded "$source" "$bad" "$horizon" "$certificate")
  actual=$(printf '%s\n' "$produced" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  backend=$(printf '%s\n' "$produced" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  reason=$(printf '%s\n' "$produced" | sed -n 's/.* reason=\([^ ]*\).*/\1/p')
  test "$actual" = "$expected"
  test "$backend" = "$expected_backend"
  test "$reason" = "$expected_reason"
  "$binary" verify-btor2-bounded "$source" "$certificate" >/dev/null
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' \
    "$name" "$(sha256_file "$source")" "$bad" "$horizon" "$expected" \
    "$actual" "$expected_backend" "$backend" "$reason" >>"$output"
}

run_case motion-safe examples/btor2/motion-envelope-v1.btor2 21 200 SAFE motion-curve motion-curve-exact-safe
run_case motion-unsafe examples/btor2/motion-envelope-v1.btor2 21 201 UNSAFE explicit-search specialised-inapplicable-or-intersecting
run_case servo-safe examples/btor2/servo-motion-envelope-v1.btor2 21 128 SAFE motion-curve motion-curve-exact-safe
run_case servo-unsafe examples/btor2/servo-motion-envelope-v1.btor2 21 129 UNSAFE explicit-search specialised-inapplicable-or-intersecting
run_case near-neighbour-safe examples/btor2/semi-implicit-motion-rejected-v1.btor2 21 3 SAFE explicit-search specialised-inapplicable-or-intersecting
run_case near-neighbour-unsafe examples/btor2/semi-implicit-motion-rejected-v1.btor2 21 4 UNSAFE explicit-search specialised-inapplicable-or-intersecting

echo "btor2 motion self-service acceptance status=ACCEPTED cases=6 output=$output"
