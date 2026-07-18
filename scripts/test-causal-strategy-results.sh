#!/usr/bin/env bash
set -euo pipefail

binary=${1:-target/release/continuation-quotient-sat}
expected=results/causal-strategy-comparison-v1.csv

if [[ ! -x "$binary" ]]; then
  echo "CQ-SAT/GCC binary is not executable: $binary" >&2
  exit 66
fi
if [[ ! -f "$expected" ]]; then
  echo "missing checked-in causal strategy result: $expected" >&2
  exit 66
fi

temporary=$(mktemp -d "${TMPDIR:-/tmp}/cq-causal-strategies.XXXXXX")
trap 'rm -rf "$temporary"' EXIT

"$binary" benchmark-aiger-causal-strategies \
  examples/aiger/causal-sparse-16.aag 1 16 "$temporary/sparse.csv"
"$binary" benchmark-aiger-causal-strategies \
  examples/aiger/causal-dense-16.aag 1 16 "$temporary/dense.csv"
"$binary" benchmark-aiger-causal-strategies \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  8 16 "$temporary/pump.csv"
"$binary" benchmark-aiger-causal-strategies \
  examples/aiger/spi-bus-receive-e-08-bits.aag 16 16 "$temporary/spi.csv"

awk 'FNR == 1 { if (NR == 1) print; next } { print }' \
  "$temporary/sparse.csv" "$temporary/dense.csv" \
  "$temporary/pump.csv" "$temporary/spi.csv" > "$temporary/current.csv"

project_deterministic() {
  awk -F, 'NR > 1 {
    output = $1
    for (column = 2; column <= 17; column++) output = output "," $column
    for (column = 26; column <= 28; column++) output = output "," $column
    print output
  }' "$1" | LC_ALL=C sort
}

project_deterministic "$expected" > "$temporary/expected.projection"
project_deterministic "$temporary/current.csv" > "$temporary/current.projection"
diff -u "$temporary/expected.projection" "$temporary/current.projection"

rows=$(wc -l < "$temporary/current.projection" | tr -d ' ')
[[ "$rows" -eq 8 ]]
[[ $(grep -c ',true,true,ok$' "$temporary/current.projection") -eq 8 ]]
echo "causal-strategy-result-tests=PASS rows=$rows"
