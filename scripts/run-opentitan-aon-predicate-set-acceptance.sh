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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-predicate-set.XXXXXXXX")
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
cmp "$committed/watchdog-predicate-set-small.btor2" \
  "$scratch/models/watchdog-predicate-set-small.btor2"
cmp "$committed/watchdog-predicate-set-scale.btor2" \
  "$scratch/models/watchdog-predicate-set-scale.btor2"

printf '%s\n' 'schema_version,case,source_sha256,bad_properties,horizon,expected_answers,actual_answers,expected_route,actual_route,logical_reachable_states,certificate_bytes,separate_certificate_bytes,bytes_saved,certificate_verified,status' >"$report"

run_case() {
  name=$1
  source=$2
  horizon=$3
  expected_answers=$4
  expected_route=$5
  expected_states=$6
  retained=$7
  certificate=$scratch/$name.btor2-set-cert

  produced=$("$binary" check-btor2-predicate-set \
    "$source" 18,22 "$horizon" "$certificate")
  answers=$(printf '%s\n' "$produced" | sed -n 's/.* answers=\([^ ]*\).*/\1/p')
  route=$(printf '%s\n' "$produced" | sed -n 's/.* route=\([^ ]*\).*/\1/p')
  states=$(printf '%s\n' "$produced" | sed -n 's/.* logical_reachable_states=\([^ ]*\).*/\1/p')
  bytes=$(printf '%s\n' "$produced" | sed -n 's/.* certificate_bytes=\([^ ]*\).*/\1/p')
  test "$answers" = "$expected_answers"
  test "$route" = "$expected_route"
  test "$states" = "$expected_states"
  cmp "$retained" "$certificate"
  "$binary" verify-btor2-predicate-set \
    "$source" 18,22 "$horizon" "$certificate" >/dev/null

  "$binary" check-btor2-bounded "$source" 18 "$horizon" "$scratch/$name-18.cert" >/dev/null
  "$binary" check-btor2-bounded "$source" 22 "$horizon" "$scratch/$name-22.cert" >/dev/null
  separate=$(( $(wc -c <"$scratch/$name-18.cert") + $(wc -c <"$scratch/$name-22.cert") ))
  saved=$(( separate - bytes ))
  source_sha256=$(sha256_file "$source")
  printf '1,%s,%s,%s,%s,"%s","%s",%s,%s,%s,%s,%s,%s,true,accepted\n' \
    "$name" "$source_sha256" 18+22 "$horizon" "$expected_answers" \
    "$answers" "$expected_route" "$route" "$states" "$bytes" "$separate" \
    "$saved" >>"$report"
}

run_case shared-safe "$scratch/models/watchdog-predicate-set-small.btor2" 4 \
  18:SAFE:none,22:SAFE:none shared-region 15 \
  "$committed/predicate-set-shared-safe.btor2-set-cert"
run_case mixed-exact "$scratch/models/watchdog-predicate-set-small.btor2" 5 \
  18:UNSAFE:5,22:SAFE:none ordinary-exact 27 \
  "$committed/predicate-set-mixed.btor2-set-cert"
run_case scale-shared "$scratch/models/watchdog-predicate-set-scale.btor2" 1000000000 \
  18:SAFE:none,22:SAFE:none shared-region 500000001500000001 \
  "$committed/predicate-set-scale-safe.btor2-set-cert"

# The original query is external to the certificate and cannot be shortened,
# reordered, or assigned a different horizon by a verifier.
for invalid in 18:4 22,18:4 18,22:5; do
  properties=${invalid%:*}
  horizon=${invalid#*:}
  if "$binary" verify-btor2-predicate-set \
    "$scratch/models/watchdog-predicate-set-small.btor2" "$properties" "$horizon" \
    "$committed/predicate-set-shared-safe.btor2-set-cert" \
    >"$scratch/query.stdout" 2>"$scratch/query.stderr"; then
    echo "query-binding control unexpectedly verified" >&2
    exit 1
  fi
  test ! -s "$scratch/query.stdout"
done

# Source mutation and a one-byte member mutation both fail closed.
sed 's/00000000000000000000000000000101/00000000000000000000000000000110/' \
  "$scratch/models/watchdog-predicate-set-small.btor2" >"$scratch/source-mutated.btor2"
if "$binary" verify-btor2-predicate-set "$scratch/source-mutated.btor2" 18,22 4 \
  "$committed/predicate-set-shared-safe.btor2-set-cert" \
  >"$scratch/source.stdout" 2>"$scratch/source.stderr"; then
  echo "source mutation unexpectedly verified" >&2
  exit 1
fi
test ! -s "$scratch/source.stdout"

sed 's/member=22:ugte:9/member=22:ugte:8/' \
  "$committed/predicate-set-shared-safe.btor2-set-cert" >"$scratch/member-mutated.cert"
if "$binary" verify-btor2-predicate-set \
  "$scratch/models/watchdog-predicate-set-small.btor2" 18,22 4 \
  "$scratch/member-mutated.cert" >"$scratch/member.stdout" 2>"$scratch/member.stderr"; then
  echo "member mutation unexpectedly verified" >&2
  exit 1
fi
test ! -s "$scratch/member.stdout"

# Publication is immutable and refuses an existing or symlink destination.
if "$binary" check-btor2-predicate-set \
  "$scratch/models/watchdog-predicate-set-small.btor2" 18,22 4 \
  "$scratch/shared-safe.btor2-set-cert" \
  >"$scratch/overwrite.stdout" 2>"$scratch/overwrite.stderr"; then
  echo "certificate output unexpectedly overwrote an existing artifact" >&2
  exit 1
fi
test ! -s "$scratch/overwrite.stdout"
ln -s "$scratch/nonexistent" "$scratch/symlink.cert"
if "$binary" check-btor2-predicate-set \
  "$scratch/models/watchdog-predicate-set-small.btor2" 18,22 4 \
  "$scratch/symlink.cert" >"$scratch/symlink.stdout" 2>"$scratch/symlink.stderr"; then
  echo "certificate output unexpectedly followed a symlink" >&2
  exit 1
fi
test ! -s "$scratch/symlink.stdout"

if ! (set -C; cat "$report" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "opentitan_aon_predicate_set_acceptance=ACCEPTED cases=3 hostile_controls=7 output=$output"
