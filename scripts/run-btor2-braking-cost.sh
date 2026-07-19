#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_RELEASE_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
test -x "$binary"
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

trials=21
middle=11
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-braking-cost.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

median() {
  sort -n "$1" | sed -n "${middle}p"
}

printf '%s\n' 'schema_version,model,horizon,trials,explicit_bytes,braking_bytes,explicit_verify_median_micros,braking_verify_median_micros,verify_speedup,status' >"$output"

run_case() {
  name=$1
  source=$2
  horizon=$3
  explicit=$scratch/$name.search-cert
  braking=$scratch/$name.braking-cert
  explicit_times=$scratch/$name-explicit.times
  braking_times=$scratch/$name-braking.times
  "$binary" search-btor2 "$source" 31 "$horizon" "$explicit" >/dev/null
  "$binary" check-btor2-bounded "$source" 31 "$horizon" "$braking" \
    | grep -q ' backend=braking-phases '
  iteration=0
  while [ "$iteration" -lt "$trials" ]; do
    "$binary" verify-btor2-search "$source" "$explicit" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$explicit_times"
    "$binary" verify-btor2-bounded "$source" "$braking" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$braking_times"
    iteration=$((iteration + 1))
  done
  test "$(wc -l <"$explicit_times" | tr -d ' ')" -eq "$trials"
  test "$(wc -l <"$braking_times" | tr -d ' ')" -eq "$trials"
  explicit_median=$(median "$explicit_times")
  braking_median=$(median "$braking_times")
  speedup=$(awk -v slow="$explicit_median" -v fast="$braking_median" \
    'BEGIN { if (fast == 0) print "inf"; else printf "%.2f", slow / fast }')
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,ok\n' \
    "$name" "$horizon" "$trials" \
    "$(wc -c <"$explicit" | tr -d ' ')" \
    "$(wc -c <"$braking" | tr -d ' ')" \
    "$explicit_median" "$braking_median" "$speedup" >>"$output"
}

run_case braking-controller examples/btor2/braking-controller-v1.btor2 255
run_case motor-emergency-stop examples/btor2/motor-emergency-stop-v1.btor2 159

echo "btor2_braking_cost=PASS trials=$trials output=$output"
