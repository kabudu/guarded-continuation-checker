#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
if [[ -e "$output" ]]; then
  echo "Zephyr SiFive GPIO qualification output already exists: $output" >&2
  exit 2
fi
mkdir -p "$output"
output=$(CDPATH='' cd -- "$output" && pwd)
stage=$(mktemp -d "${TMPDIR:-/tmp}/gcc-zephyr-gpio.XXXXXX")
cleanup() {
  rm -rf "$stage"
}
trap cleanup EXIT HUP INT TERM

source_url=https://raw.githubusercontent.com/zephyrproject-rtos/zephyr/v4.2.0/drivers/gpio/gpio_sifive.c
licence_url=https://raw.githubusercontent.com/zephyrproject-rtos/zephyr/v4.2.0/LICENSE
source_sha=8525e5ece8cbb2fe57487568918612c4d91439c5e95b67d81c8e0fe175bfe63d
licence_sha=c6596eb7be8581c18be736c846fb9173b69eccf6ef94c5135893ec56bd92ba08
image="silkeh/clang@sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"

mkdir -p "$stage/upstream"
curl --fail --silent --show-error --location \
  "$source_url" \
  --output "$stage/upstream/gpio_sifive.c"
curl --fail --silent --show-error --location \
  "$licence_url" \
  --output "$stage/upstream/LICENSE"
printf '%s  %s\n' "$source_sha" "$stage/upstream/gpio_sifive.c" \
  | shasum -a 256 -c -
printf '%s  %s\n' "$licence_sha" "$stage/upstream/LICENSE" \
  | shasum -a 256 -c -
cp -R "$root/corpus/firmware/zephyr-sifive-gpio/compat" "$stage/compat"

docker run --rm \
  --user "$(id -u):$(id -g)" \
  -v "$stage:/source:ro" \
  -v "$output:/out" \
  "$image" \
  sh -lc '
    set -eu
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
      -I/source/compat \
      -I/source \
      -Wl,-e,gcc_firmware_entry \
      -Wl,-Ttext=0x80000000 \
      -Wl,--build-id=none \
      -o /out/firmware.elf \
      /source/compat/firmware_caller.c
    llvm-objcopy -O binary /out/firmware.elf /out/firmware.bin
    llvm-nm --defined-only --numeric-sort /out/firmware.elf \
      > /out/firmware.symbols.txt
    llvm-objdump -d --no-show-raw-insn /out/firmware.elf \
      > /out/firmware.disassembly.txt
  '

cargo build \
  --quiet \
  --release \
  --manifest-path "$root/Cargo.toml" \
  --example multisuccessor_mmio_decode_graph_baseline
baseline="$root/target/release/examples/multisuccessor_mmio_decode_graph_baseline"
"$baseline" \
  "$output/firmware.bin" \
  "$output/firmware.symbols.txt" \
  "$output/decode-graph-v2.bin" \
  "$output/branching-dag-v1.bin" \
  "$output/trace-family-v1.bin" \
  "$output/explicit-transcript-v1.bin" \
  > "$output/result.txt"

cp "$stage/upstream/gpio_sifive.c" "$output/"
cp "$stage/upstream/LICENSE" "$output/UPSTREAM_LICENSE.txt"
{
  echo "zephyr_sifive_gpio_dense_qualification_version=1"
  echo "upstream_release=v4.2.0"
  echo "upstream_source_sha256=$source_sha"
  echo "upstream_licence_sha256=$licence_sha"
  cat "$output/result.txt"
  echo "status=complete"
} > "$output/summary.txt"
cat "$output/summary.txt"

