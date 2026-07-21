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
committed_model=$fixture/generated/dual-timer-predicate-set.btor2
committed_certificates=$fixture/generated

test -x "$binary"
test -x "$yosys"
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-aon-dual-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
generated=$scratch/dual-timer.btor2
report=$scratch/acceptance.csv
"$repo/scripts/build-opentitan-aon-dual-timer-btor2.sh" "$yosys" "$generated" >/dev/null
cmp "$committed_model" "$generated"

capabilities=$("$binary" btor2-predicate-set-version)
case "$capabilities" in
  *"btor2_predicate_set_cli_version=3"*"certificate_versions=1,2,3"*"portfolio_versions=1,2,3"*"current_certificate_version=3"*"max_chain_states=16"*) ;;
  *) echo "predicate-set v3 capability contract is unavailable" >&2; exit 1 ;;
esac

printf '%s\n' 'schema_version,case,horizon,expected_answers,actual_answers,route,logical_reachable_states,certificate_bytes,separate_baseline_status,certificate_verified,status' >"$report"

run_case() {
  name=$1
  horizon=$2
  expected_answers=$3
  expected_states=$4
  expected_bytes=$5
  retained=$committed_certificates/predicate-set-v3-dual-$name.btor2-set-cert
  certificate=$scratch/$name.btor2-set-cert

  produced=$("$binary" check-btor2-predicate-set \
    "$generated" 33,37,41 "$horizon" "$certificate")
  case "$produced" in
    *"certificate_version=3 portfolio_version=3"*"route=invariant-chained-regions"*"reason=invariant-chained-recurrences"*) ;;
    *) echo "$name did not select invariant chaining" >&2; exit 1 ;;
  esac
  answers=$(printf '%s\n' "$produced" | sed -n 's/.* answers=\([^ ]*\).*/\1/p')
  states=$(printf '%s\n' "$produced" | sed -n 's/.* logical_reachable_states=\([^ ]*\).*/\1/p')
  bytes=$(printf '%s\n' "$produced" | sed -n 's/.* certificate_bytes=\([^ ]*\).*/\1/p')
  test "$answers" = "$expected_answers"
  test "$states" = "$expected_states"
  test "$bytes" = "$expected_bytes"
  cmp "$retained" "$certificate"
  verified=$("$binary" verify-btor2-predicate-set \
    "$generated" 33,37,41 "$horizon" "$certificate")
  case "$verified" in
    *"certificate_version=3 portfolio_version=3"*"route=invariant-chained-regions"*"answers=$expected_answers"*) ;;
    *) echo "$name verification summary disagrees" >&2; exit 1 ;;
  esac
  printf '1,%s,%s,"%s","%s",invariant-chained-regions,%s,%s,unavailable,true,accepted\n' \
    "$name" "$horizon" "$expected_answers" "$answers" "$states" "$bytes" >>"$report"
}

run_case h4 4 '33:SAFE:none,37:SAFE:none,41:SAFE:none' 15 445
run_case h5 5 '33:SAFE:none,37:UNSAFE:5,41:SAFE:none' 21 454
run_case h7 7 '33:SAFE:none,37:UNSAFE:5,41:UNSAFE:7' 36 463
run_case h9 9 '33:UNSAFE:9,37:UNSAFE:5,41:UNSAFE:7' 55 472
run_case h1b 1000000000 '33:UNSAFE:9,37:UNSAFE:5,41:UNSAFE:7' 500000001500000001 515

# Existing v1 and v2 artifacts retain their exact versioned semantics.
for tuple in \
  'predicate-set-shared-safe.btor2-set-cert:18,22:4:certificate_version=1' \
  'predicate-set-v2-shared-safe.btor2-set-cert:18,22:4:certificate_version=2'; do
  artifact=${tuple%%:*}
  rest=${tuple#*:}
  properties=${rest%%:*}
  rest=${rest#*:}
  horizon=${rest%%:*}
  version=${rest#*:}
  compatible=$("$binary" verify-btor2-predicate-set \
    "$fixture/generated/watchdog-predicate-set-small.btor2" \
    "$properties" "$horizon" "$committed_certificates/$artifact")
  case "$compatible" in
    *"$version"*) ;;
    *) echo "$artifact compatibility verification failed" >&2; exit 1 ;;
  esac
done

# Every external query binding remains mandatory.
for invalid in '33,37:9' '37,33,41:9' '33,37,41:8'; do
  properties=${invalid%:*}
  horizon=${invalid#*:}
  if "$binary" verify-btor2-predicate-set "$generated" "$properties" "$horizon" \
    "$committed_certificates/predicate-set-v3-dual-h9.btor2-set-cert" \
    >"$scratch/query.stdout" 2>"$scratch/query.stderr"; then
    echo "query-binding mutation unexpectedly verified" >&2
    exit 1
  fi
  test ! -s "$scratch/query.stdout"
done

mutate_and_reject() {
  label=$1
  expression=$2
  sed "$expression" \
    "$committed_certificates/predicate-set-v3-dual-h9.btor2-set-cert" \
    >"$scratch/$label.cert"
  if "$binary" verify-btor2-predicate-set "$generated" 33,37,41 9 \
    "$scratch/$label.cert" >"$scratch/$label.stdout" 2>"$scratch/$label.stderr"; then
    echo "$label mutation unexpectedly verified" >&2
    exit 1
  fi
  test ! -s "$scratch/$label.stdout"
}

mutate_and_reject invariant 's/invariant=16:12:0/invariant=16:12:1/'
mutate_and_reject recurrence 's/recurrence=6:32:0:0:1:none:9/recurrence=6:32:0:0:2:none:9/'
mutate_and_reject result 's/33:6:ugte:9:UNSAFE:9/33:6:ugte:9:SAFE:9/'
mutate_and_reject frame 's/33:6:ugte:9:UNSAFE:9/33:6:ugte:9:UNSAFE:8/'
mutate_and_reject witness 's/advance_prefix/none/'
mutate_and_reject source 's/source_sha256=1/source_sha256=2/'
mutate_and_reject route 's/route=invariant_chained_regions/route=ordinary_exact/'

# Truncation never yields an accepted partial certificate.
bytes=$(wc -c <"$committed_certificates/predicate-set-v3-dual-h9.btor2-set-cert")
offset=0
while [ "$offset" -lt "$bytes" ]; do
  dd if="$committed_certificates/predicate-set-v3-dual-h9.btor2-set-cert" \
    of="$scratch/truncated.cert" bs=1 count="$offset" 2>/dev/null
  if "$binary" verify-btor2-predicate-set "$generated" 33,37,41 9 \
    "$scratch/truncated.cert" >"$scratch/truncated.stdout" \
    2>"$scratch/truncated.stderr"; then
    echo "truncated certificate unexpectedly verified at $offset bytes" >&2
    exit 1
  fi
  offset=$((offset + 1))
done

if ! (set -C; cat "$report" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "opentitan_aon_dual_timer_acceptance=ACCEPTED cases=5 compatibility_artifacts=2 hostile_controls=10 output=$output"
