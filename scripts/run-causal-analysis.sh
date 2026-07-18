#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "usage: $0 INPUT.aag OUTPUT_DIR [HORIZON] [MAX_BOUND_BITS]" >&2
  exit 64
fi

input=$1
output=$2
horizon=${3:-8}
max_bound_bits=${4:-16}
binary=${CQ_SAT_BINARY:-target/release/continuation-quotient-sat}

if [[ ! -x "$binary" ]]; then
  echo "CQ-SAT/GCC binary is not executable: $binary" >&2
  echo "build it with: cargo build --release --locked" >&2
  exit 66
fi

"$binary" explain-aiger-counterexample \
  "$input" "$horizon" "$max_bound_bits" "$output"
"$binary" verify-aiger-causal-bundle "$input" "$output"

echo "verified causal evidence bundle: $output"
