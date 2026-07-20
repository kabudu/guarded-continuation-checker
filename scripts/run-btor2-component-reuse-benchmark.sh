#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 admitted|mixed|all OUTPUT.csv" >&2
  exit 2
fi

cohort=$1
output=$2
case "$cohort" in
  admitted|mixed|all) ;;
  *) echo "cohort must be admitted, mixed, or all" >&2; exit 2 ;;
esac
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

cargo build --locked --release --example btor2_component_reuse_benchmark
target/release/examples/btor2_component_reuse_benchmark "$cohort" >"$output"
test "$(wc -l <"$output" | tr -d ' ')" -ge 2
echo "btor2_component_reuse_benchmark=PASS cohort=$cohort output=$output"
