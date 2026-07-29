#!/usr/bin/env bash
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 SWEEP_DIR OUTPUT_CSV" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
sweep=$1
output=$2
trials=5
case "$(uname -s)" in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
platform=$(uname -s)-$(uname -m)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-dense-decode-resources.XXXXXX")
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT HUP INT TERM

cargo build \
  --quiet \
  --release \
  --manifest-path "$root/Cargo.toml" \
  --example verify_multisuccessor_mmio_decode_graph
consumer="$root/target/release/examples/verify_multisuccessor_mmio_decode_graph"

measure() {
  classes=$1
  route=$2
  trial=$3
  artifact=$4
  image=$5
  symbols=$6
  stdout="$scratch/classes-$classes-$route-$trial.stdout"
  metrics="$scratch/classes-$classes-$route-$trial.time"
  if [ "$time_style" = bsd ]; then
    /usr/bin/time -l \
      "$consumer" "$route" "$artifact" "$image" "$symbols" \
      >"$stdout" 2>"$metrics"
    elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    /usr/bin/time -f '%e %M' -o "$metrics" \
      "$consumer" "$route" "$artifact" "$image" "$symbols" \
      >"$stdout"
    read -r elapsed peak_kib <"$metrics"
    peak_bytes=$((peak_kib * 1024))
  fi
  test -n "$elapsed"
  test -n "$peak_bytes"
  printf '3,%s,%s,%s,%s,%s,%s,%s\n' \
    "$classes" "$route" "$trial" "$elapsed" "$peak_bytes" "$time_style" "$platform" \
    >>"$output"
}

printf '%s\n' \
  'schema_version,classes,route,trial,elapsed_seconds,peak_rss_bytes,time_backend,platform' \
  >"$output"

for classes in 1 2 4 8 16 32; do
  cohort="$sweep/classes-$classes"
  image="$cohort/firmware.bin"
  symbols="$cohort/firmware.symbols.txt"
  for route in graph graph-btree trace-family; do
    case "$route" in
      graph|graph-btree)
        artifact="$cohort/multisuccessor-decode-graph-v2.bin"
        ;;
      trace-family)
        artifact="$cohort/trace-family-v1.bin"
        ;;
    esac
    "$consumer" "$route" "$artifact" "$image" "$symbols" \
      >"$scratch/classes-$classes-$route-warmup.stdout"
    trial=1
    while [ "$trial" -le "$trials" ]; do
      measure "$classes" "$route" "$trial" "$artifact" "$image" "$symbols"
      trial=$((trial + 1))
    done
  done
done

echo "dense decode index resources status=MEASURED trials=$trials output=$output"
