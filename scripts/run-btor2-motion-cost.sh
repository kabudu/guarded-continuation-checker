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
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-motion-cost.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

median() {
  sort -n "$1" | sed -n "${middle}p"
}

printf '%s\n' 'schema_version,model,horizon,trials,explicit_bytes,motion_bytes,explicit_verify_median_micros,motion_verify_median_micros,verify_speedup,status' >"$output"

run_case() {
  name=$1
  source=$2
  bad=$3
  horizon=$4
  explicit=$scratch/$name.search-cert
  motion=$scratch/$name.motion-cert
  explicit_times=$scratch/$name-explicit.times
  motion_times=$scratch/$name-motion.times
  "$binary" search-btor2 "$source" "$bad" "$horizon" "$explicit" >/dev/null
  "$binary" check-btor2-bounded "$source" "$bad" "$horizon" "$motion" \
    | grep -q ' backend=motion-curve '
  iteration=0
  while [ "$iteration" -lt "$trials" ]; do
    "$binary" verify-btor2-search "$source" "$explicit" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$explicit_times"
    "$binary" verify-btor2-bounded "$source" "$motion" \
      | sed -n 's/.* elapsed_micros=\([0-9][0-9]*\).*/\1/p' >>"$motion_times"
    iteration=$((iteration + 1))
  done
  test "$(wc -l <"$explicit_times" | tr -d ' ')" -eq "$trials"
  test "$(wc -l <"$motion_times" | tr -d ' ')" -eq "$trials"
  explicit_median=$(median "$explicit_times")
  motion_median=$(median "$motion_times")
  speedup=$(awk -v slow="$explicit_median" -v fast="$motion_median" \
    'BEGIN { if (fast == 0) print "inf"; else printf "%.2f", slow / fast }')
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,ok\n' \
    "$name" "$horizon" "$trials" \
    "$(wc -c <"$explicit" | tr -d ' ')" \
    "$(wc -c <"$motion" | tr -d ' ')" \
    "$explicit_median" "$motion_median" "$speedup" >>"$output"
}

run_case motion-envelope examples/btor2/motion-envelope-v1.btor2 21 200
run_case servo-motion examples/btor2/servo-motion-envelope-v1.btor2 21 128

echo "btor2_motion_cost=PASS trials=$trials output=$output"
