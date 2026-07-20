#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
  echo "usage: $0 GCC_BINARY YOSYS_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
yosys=$2
output=$3
repo=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-aon-timer
committed=$fixture/generated

test -x "$binary"
test -x "$yosys"
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
mkdir "$scratch/models"
report=$scratch/acceptance.csv

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

while read -r expected relative; do
  test "$(sha256_file "$fixture/$relative")" = "$expected"
done <"$fixture/SHA256SUMS"

"$repo/scripts/build-opentitan-aon-watchdog-btor2.sh" \
  "$yosys" "$scratch/models" >/dev/null
cmp "$committed/watchdog-small.btor2" "$scratch/models/watchdog-small.btor2"
cmp "$committed/watchdog-scale.btor2" "$scratch/models/watchdog-scale.btor2"

printf '%s\n' 'schema_version,case,source_sha256,bad_property,horizon,expected_result,actual_result,expected_backend,actual_backend,logical_reachable_states,certificate_bytes,certificate_verified,status' >"$report"

run_case() {
  name=$1
  source=$2
  bad=$3
  horizon=$4
  expected_result=$5
  expected_backend=$6
  expected_states=$7
  retained=$8
  certificate=$scratch/$name.btor2-cert

  produced=$("$binary" check-btor2-bounded \
    "$source" "$bad" "$horizon" "$certificate")
  result=$(printf '%s\n' "$produced" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  backend=$(printf '%s\n' "$produced" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  states=$(printf '%s\n' "$produced" | sed -n 's/.* logical_reachable_states=\([^ ]*\).*/\1/p')
  bytes=$(printf '%s\n' "$produced" | sed -n 's/.* certificate_bytes=\([^ ]*\).*/\1/p')
  test "$result" = "$expected_result"
  test "$backend" = "$expected_backend"
  test "$states" = "$expected_states"
  cmp "$retained" "$certificate"
  "$binary" verify-btor2-bounded "$source" "$certificate" >/dev/null

  source_sha256=$(sed -n 's/^source_sha256=//p' "$certificate")
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,accepted\n' \
    "$name" "$source_sha256" "$bad" "$horizon" "$expected_result" \
    "$result" "$expected_backend" "$backend" "$states" "$bytes" >>"$report"
}

run_case small-safe "$scratch/models/watchdog-small.btor2" 15 8 SAFE \
  word-region 45 "$committed/small-safe.btor2-cert"
run_case small-unsafe "$scratch/models/watchdog-small.btor2" 15 9 UNSAFE \
  explicit-search 10 "$committed/small-unsafe.btor2-cert"
run_case scale-safe "$scratch/models/watchdog-scale.btor2" 15 1000000000 SAFE \
  word-region 500000001500000001 "$committed/scale-safe.btor2-cert"

# Exact near-neighbour control: XOR is not an identity and must route to exact
# fallback rather than being accepted by the word-region recogniser.
sed 's/14 and 1 12 13/14 xor 1 12 13/' \
  "$scratch/models/watchdog-small.btor2" >"$scratch/near-neighbour.btor2"
near_cert=$scratch/near-neighbour.btor2-cert
near=$("$binary" check-btor2-bounded \
  "$scratch/near-neighbour.btor2" 15 8 "$near_cert")
printf '%s\n' "$near" | grep -q ' backend=explicit-search '
"$binary" verify-btor2-bounded "$scratch/near-neighbour.btor2" "$near_cert" >/dev/null

# A certificate is bound to the exact source bytes.
sed 's/00000000000000000000000000001001/00000000000000000000000000001010/' \
  "$scratch/models/watchdog-small.btor2" >"$scratch/source-mutated.btor2"
if "$binary" verify-btor2-bounded \
  "$scratch/source-mutated.btor2" "$committed/small-safe.btor2-cert" \
  >"$scratch/tamper.stdout" 2>"$scratch/tamper.stderr"; then
  echo "source-mutated certificate unexpectedly verified" >&2
  exit 1
fi
test ! -s "$scratch/tamper.stdout"

# Observation references must be prior valid expressions.
sed 's/10 output 9 bad/10 output 99 bad/' \
  "$scratch/models/watchdog-small.btor2" >"$scratch/output-mutated.btor2"
if "$binary" inspect-btor2 "$scratch/output-mutated.btor2" \
  >"$scratch/output.stdout" 2>"$scratch/output.stderr"; then
  echo "invalid BTOR2 output reference unexpectedly parsed" >&2
  exit 1
fi
test ! -s "$scratch/output.stdout"

# Reproduction is immutable and refuses to overwrite its own results.
if "$repo/scripts/build-opentitan-aon-watchdog-btor2.sh" \
  "$yosys" "$scratch/models" >"$scratch/overwrite.stdout" 2>"$scratch/overwrite.stderr"; then
  echo "model build unexpectedly overwrote an existing artifact" >&2
  exit 1
fi
test ! -s "$scratch/overwrite.stdout"

# Output paths are file-system data, never Yosys command syntax.
unusual=$scratch/'models;invalid-command'
mkdir "$unusual"
"$repo/scripts/build-opentitan-aon-watchdog-btor2.sh" \
  "$yosys" "$unusual" >/dev/null
cmp "$committed/watchdog-small.btor2" "$unusual/watchdog-small.btor2"
cmp "$committed/watchdog-scale.btor2" "$unusual/watchdog-scale.btor2"

if ! (set -C; cat "$report" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "opentitan_aon_watchdog_acceptance=ACCEPTED cases=3 hostile_controls=5 output=$output"
