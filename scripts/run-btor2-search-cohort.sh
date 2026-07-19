#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

gcc_binary=$1
output=$2
test -x "$gcc_binary"
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=${TMPDIR:-/tmp}/gcc-btor2-search-cohort-$$
mkdir "$scratch"
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

printf '%s\n' 'schema_version,model,bad_property,horizon,expected,actual,certificate_bytes,verified,status' >"$output"

run_case() {
  name=$1
  model=$2
  bad=$3
  horizon=$4
  expected=$5
  certificate=$scratch/$name-$horizon.search-cert
  "$gcc_binary" search-btor2 "$model" "$bad" "$horizon" "$certificate" >/dev/null
  actual=$(sed -n 's/^result=//p' "$certificate")
  test "$actual" = "$expected"
  "$gcc_binary" verify-btor2-search "$model" "$certificate" >/dev/null
  bytes=$(wc -c <"$certificate" | tr -d ' ')
  printf '1,%s,%s,%s,%s,%s,%s,true,ok\n' \
    "$name" "$bad" "$horizon" "$expected" "$actual" "$bytes" >>"$output"
}

run_case watchdog examples/btor2/watchdog-counter-v1.btor2 13 2 SAFE
run_case watchdog examples/btor2/watchdog-counter-v1.btor2 13 3 UNSAFE
run_case actuator examples/btor2/actuator-position-v1.btor2 13 200 SAFE
run_case actuator examples/btor2/actuator-position-v1.btor2 13 201 UNSAFE
run_case saturating-timer examples/btor2/saturating-timer-rejected-v1.btor2 15 254 SAFE
run_case saturating-timer examples/btor2/saturating-timer-rejected-v1.btor2 15 255 UNSAFE

echo "btor2_search_cohort=PASS output=$output"
