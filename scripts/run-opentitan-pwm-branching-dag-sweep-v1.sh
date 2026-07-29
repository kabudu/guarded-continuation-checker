#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
if [[ -e "$output" ]]; then
  echo "branching DAG output already exists: $output" >&2
  exit 2
fi
mkdir -p "$output"
output=$(CDPATH='' cd -- "$output" && pwd)
image="silkeh/clang@sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"
stage=$(mktemp -d "${TMPDIR:-/tmp}/gcc-branching-source.XXXXXX")
cleanup() {
  rm -rf "$stage"
}
trap cleanup EXIT HUP INT TERM
cp -R "$root/corpus/firmware/opentitan-pwm-dif/compat" "$stage/compat"
cp -R "$root/corpus/firmware/opentitan-pwm-dif/upstream" "$stage/upstream"

for classes in 1 2 4 8 16 32; do
  mkdir -p "$output/classes-$classes"
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -v "$stage:/source:ro" \
    -v "$output/classes-$classes:/out" \
    "$image" \
    sh -lc "
      set -eu
      source=/source
      clang \
        --target=riscv32-unknown-elf \
        -march=rv32imc \
        -mabi=ilp32 \
        -std=c11 \
        -ffreestanding \
        -fno-builtin \
        -nostdlib \
        -fuse-ld=lld \
        -O2 \
        -DGCC_BRANCH_CLASSES=$classes \
        -I\"\$source/compat\" \
        -Wl,-e,gcc_firmware_entry \
        -Wl,-Ttext=0x80000000 \
        -Wl,--build-id=none \
        -o /out/firmware.elf \
        \"\$source/upstream/dif_pwm.c\" \
        \"\$source/compat/branching_firmware_caller.c\"
      llvm-objcopy -O binary /out/firmware.elf /out/firmware.bin
      llvm-nm --defined-only --numeric-sort /out/firmware.elf \
        > /out/firmware.symbols.txt
    "
done

cargo build \
  --quiet \
  --release \
  --manifest-path "$root/Cargo.toml" \
  --example branching_mmio_dag_baseline
baseline="$root/target/release/examples/branching_mmio_dag_baseline"

for classes in 1 2 4 8 16 32; do
  cohort="$output/classes-$classes"
  "$baseline" \
    "$cohort/firmware.bin" \
    "$cohort/firmware.symbols.txt" \
    "$cohort/branching-dag-v1.bin" \
    "$cohort/explicit-transcript-v1.bin" \
    > "$cohort/result.txt"
done

{
  echo "opentitan-pwm-branching-dag-sweep-v1"
  for classes in 1 2 4 8 16 32; do
    echo "classes=$classes"
    cat "$output/classes-$classes/result.txt"
  done
  echo "status=complete"
} > "$output/sweep-summary.txt"

cat "$output/sweep-summary.txt"
