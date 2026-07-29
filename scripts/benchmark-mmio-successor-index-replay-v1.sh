#!/usr/bin/env bash
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 QUALIFICATION_DIR OUTPUT_CSV" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cohort=$1
output=$2
trials=5
repetitions=100
case "$(uname -s)" in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
platform=$(uname -s)-$(uname -m)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-mmio-successor-resources.XXXXXX")
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT HUP INT TERM

cargo build \
  --quiet \
  --release \
  --manifest-path "$root/Cargo.toml" \
  --example benchmark_mmio_replay_routes
consumer="$root/target/release/examples/benchmark_mmio_replay_routes"
image="$cohort/firmware.bin"
symbols="$cohort/firmware.symbols.txt"

measure() {
  route=$1
  trial=$2
  case "$route" in
    graph|graph-successor) artifact="$cohort/decode-graph-v2.bin" ;;
    trace-family) artifact="$cohort/trace-family-v1.bin" ;;
  esac
  stdout="$scratch/$route-$trial.stdout"
  metrics="$scratch/$route-$trial.time"
  if [ "$time_style" = bsd ]; then
    /usr/bin/time -l \
      "$consumer" "$route" "$artifact" "$image" "$symbols" "$repetitions" \
      >"$stdout" 2>"$metrics"
    elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    /usr/bin/time -f '%e %M' -o "$metrics" \
      "$consumer" "$route" "$artifact" "$image" "$symbols" "$repetitions" \
      >"$stdout"
    read -r elapsed peak_kib <"$metrics"
    peak_bytes=$((peak_kib * 1024))
  fi
  test -n "$elapsed"
  test -n "$peak_bytes"
  printf '1,%s,%s,%s,%s,%s,%s,%s\n' \
    "$route" "$trial" "$repetitions" "$elapsed" "$peak_bytes" \
    "$time_style" "$platform" >>"$output"
}

printf '%s\n' \
  'schema_version,route,trial,repetitions,elapsed_seconds,peak_rss_bytes,time_backend,platform' \
  >"$output"

trial=1
while [ "$trial" -le "$trials" ]; do
  case "$trial" in
    1|4) routes='graph graph-successor trace-family' ;;
    2|5) routes='graph-successor trace-family graph' ;;
    3) routes='trace-family graph graph-successor' ;;
  esac
  for route in $routes; do
    measure "$route" "$trial"
  done
  trial=$((trial + 1))
done

echo "MMIO successor-index resources status=MEASURED trials=$trials repetitions=$repetitions output=$output"
