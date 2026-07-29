#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
if [[ -e "$output" ]]; then
  echo "explicit transcript output already exists: $output" >&2
  exit 2
fi
mkdir -p "$output"
output=$(CDPATH='' cd -- "$output" && pwd)

"$root/scripts/build-opentitan-pwm-compiled-mmio-v1.sh" "$output/firmware"

cargo build \
  --quiet \
  --release \
  --manifest-path "$root/Cargo.toml" \
  --example explicit_mmio_transcript_baseline \
  --example verify_explicit_mmio_transcript_baseline

producer="$root/target/release/examples/explicit_mmio_transcript_baseline"
consumer="$root/target/release/examples/verify_explicit_mmio_transcript_baseline"
image="$output/firmware/runtime-channel/firmware.bin"
symbols="$output/firmware/runtime-channel/firmware.symbols.txt"

for cycle in 1 2; do
  "$producer" \
    "$image" \
    "$symbols" \
    "$output/predicate-cycle-$cycle.bin" \
    "$output/explicit-cycle-$cycle.bin" \
    > "$output/producer-cycle-$cycle.txt"
done

cmp "$output/predicate-cycle-1.bin" "$output/predicate-cycle-2.bin"
cmp "$output/explicit-cycle-1.bin" "$output/explicit-cycle-2.bin"
cmp "$output/producer-cycle-1.txt" "$output/producer-cycle-2.txt"

for route in predicate explicit; do
  artifact="$output/$route-cycle-1.bin"
  "$consumer" "$route" "$artifact" "$image" "$symbols" \
    > "$output/$route-warmup.txt"
  for trial in 1 2 3 4 5; do
    /usr/bin/time \
      -f 'wall_seconds=%e
peak_rss_kib=%M' \
      -o "$output/$route-timing-$trial.txt" \
      "$consumer" "$route" "$artifact" "$image" "$symbols" \
      > "$output/$route-consumer-$trial.txt"
  done
done

{
  echo "explicit-transcript-hosted-result-v1"
  echo "platform=$(uname -sm)"
  sha256sum \
    "$output/predicate-cycle-1.bin" \
    "$output/explicit-cycle-1.bin"
  cat "$output/producer-cycle-1.txt"
  for route in predicate explicit; do
    for trial in 1 2 3 4 5; do
      echo "route=$route trial=$trial"
      cat "$output/$route-timing-$trial.txt"
    done
  done
  echo "clean_cycles_byte_identical=true"
  echo "status=complete"
} > "$output/result-summary.txt"

cat "$output/result-summary.txt"
