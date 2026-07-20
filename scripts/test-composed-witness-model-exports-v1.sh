#!/bin/sh
set -eu

repository=$(unset CDPATH; cd -- "$(dirname "$0")/.." && pwd -P)
retained=$repository/corpus/rtl/wmcontroller/composed-witness-models-v1
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-composed-models.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
generated=$scratch/generated

cd "$repository"
cargo run --quiet --locked --example export_composed_witness_plant_family -- \
  "$generated"
diff -ru "$retained" "$generated"
if cargo run --quiet --locked --example export_composed_witness_plant_family -- \
  "$generated" >/dev/null 2>&1; then
  echo "existing composed-witness model directory was overwritten" >&2
  exit 1
fi
echo 'composed-witness model export tests status=PASS models=12 collision=rejected'
