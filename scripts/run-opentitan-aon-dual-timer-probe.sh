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
committed=$fixture/generated/dual-timer-predicate-set.btor2

test -x "$binary"
test -x "$yosys"
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-aon-dual-probe.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
generated=$scratch/dual-timer.btor2
"$repo/scripts/build-opentitan-aon-dual-timer-btor2.sh" "$yosys" "$generated" >/dev/null
cmp "$committed" "$generated"

inspection=$("$binary" inspect-btor2 "$generated")
case "$inspection" in
  *"status=VALID"*"sha256=1fb74ecf07eaeac782f3a9131456f97992b0fa72f3c273310d4f4fec8fe6e57f"*"nodes=44 inputs=1 states=3 bad=3 constraints=0 max_width=64"*) ;;
  *) echo "dual-timer inspection disagrees with retained structure" >&2; exit 1 ;;
esac

expect_refusal() {
  horizon=$1
  expected=$2
  certificate=$scratch/query-$horizon.cert
  if "$binary" check-btor2-predicate-set "$generated" 33,37,41 "$horizon" \
    "$certificate" >"$scratch/query-$horizon.stdout" \
    2>"$scratch/query-$horizon.stderr"; then
    echo "dual-timer pre-implementation query unexpectedly succeeded" >&2
    exit 1
  fi
  test ! -s "$scratch/query-$horizon.stdout"
  test ! -e "$certificate"
  grep -F "$expected" "$scratch/query-$horizon.stderr" >/dev/null
}

expect_refusal 4 "exact member 41 failed: explicit search fallback error: bounded search requires a state-only bad property"
expect_refusal 1000000000 "exact member 33 failed: explicit search fallback error: search horizon exceeds limit"

report=$scratch/probe.csv
printf '%s\n' \
  'schema_version,source_sha256,nodes,inputs,states,bad_properties,max_width,small_query_status,scale_query_status,status' \
  '1,1fb74ecf07eaeac782f3a9131456f97992b0fa72f3c273310d4f4fec8fe6e57f,44,1,3,"33+37+41",64,fail-closed-input-dependent,fail-closed-search-limit,accepted' \
  >"$report"
if ! (set -C; cat "$report" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "opentitan_aon_dual_timer_probe=ACCEPTED output=$output"
