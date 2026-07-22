#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT.csv" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cargo run --quiet --release --manifest-path "$repo/Cargo.toml" \
  --example btor2_trace_portfolio_probe -- "$1"
