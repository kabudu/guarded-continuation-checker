#!/usr/bin/env sh
set -eu

repository=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=${1:-"$repository/results/public-washing-controller-proof-mtbdd-plant-v1.csv"}
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

cd "$repository"
cargo run --release --quiet --example public_washing_controller_proof_plant_benchmark >"$output"
cat "$output"
