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

scratch=${TMPDIR:-/tmp}/gcc-btor2-motion-cohort-$$
mkdir "$scratch"
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

printf '%s\n' 'schema_version,model,bad_property,horizon,expected,actual,backend,explicit_bytes,portfolio_bytes,byte_reduction_percent,verified,status' >"$output"

run_case() {
  name=$1
  model=$2
  bad=$3
  horizon=$4
  expected=$5
  explicit=$scratch/$name-$horizon.search-cert
  portfolio=$scratch/$name-$horizon.btor2-cert
  "$gcc_binary" search-btor2 "$model" "$bad" "$horizon" "$explicit" >/dev/null
  output_line=$("$gcc_binary" check-btor2-bounded "$model" "$bad" "$horizon" "$portfolio")
  actual=$(printf '%s\n' "$output_line" | sed -n 's/.* result=\([^ ]*\).*/\1/p')
  backend=$(printf '%s\n' "$output_line" | sed -n 's/.* backend=\([^ ]*\).*/\1/p')
  test "$actual" = "$expected"
  "$gcc_binary" verify-btor2-bounded "$model" "$portfolio" >/dev/null
  explicit_bytes=$(wc -c <"$explicit" | tr -d ' ')
  portfolio_bytes=$(wc -c <"$portfolio" | tr -d ' ')
  reduction=$(awk -v old="$explicit_bytes" -v new="$portfolio_bytes" 'BEGIN { printf "%.2f", 100 * (old - new) / old }')
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,true,ok\n' \
    "$name" "$bad" "$horizon" "$expected" "$actual" "$backend" \
    "$explicit_bytes" "$portfolio_bytes" "$reduction" >>"$output"
}

run_case motion-envelope examples/btor2/motion-envelope-v1.btor2 21 200 SAFE
run_case motion-envelope examples/btor2/motion-envelope-v1.btor2 21 201 UNSAFE
run_case servo-motion examples/btor2/servo-motion-envelope-v1.btor2 21 128 SAFE
run_case servo-motion examples/btor2/servo-motion-envelope-v1.btor2 21 129 UNSAFE
run_case semi-implicit examples/btor2/semi-implicit-motion-rejected-v1.btor2 21 3 SAFE
run_case semi-implicit examples/btor2/semi-implicit-motion-rejected-v1.btor2 21 4 UNSAFE

echo "btor2_motion_cohort=PASS output=$output"
