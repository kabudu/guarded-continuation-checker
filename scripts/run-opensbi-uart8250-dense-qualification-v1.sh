#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
if [[ -e "$output" ]]; then
  echo "OpenSBI UART qualification output already exists: $output" >&2
  exit 2
fi
mkdir -p "$output"
output=$(CDPATH='' cd -- "$output" && pwd)
stage=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opensbi-uart.XXXXXX")
cleanup() {
  rm -rf "$stage"
}
trap cleanup EXIT HUP INT TERM

source_url=https://raw.githubusercontent.com/riscv-software-src/opensbi/v1.8.1/lib/utils/serial/uart8250.c
licence_url=https://raw.githubusercontent.com/riscv-software-src/opensbi/v1.8.1/COPYING.BSD
source_sha=cc0d3dd2f6eb51714c69695e979a2d4989c578c8e5e5d9c864b1401561430e64
licence_sha=82d13fb1bf6bb162629deeea9eb9c117e74548d3b707e478967691fe79a68e21
image="silkeh/clang@sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"

mkdir -p "$stage/upstream"
curl --fail --silent --show-error --location \
  "$source_url" \
  --output "$stage/upstream/uart8250.c"
curl --fail --silent --show-error --location \
  "$licence_url" \
  --output "$stage/upstream/COPYING.BSD"
printf '%s  %s\n' "$source_sha" "$stage/upstream/uart8250.c" \
  | shasum -a 256 -c -
printf '%s  %s\n' "$licence_sha" "$stage/upstream/COPYING.BSD" \
  | shasum -a 256 -c -
cp -R "$root/corpus/firmware/opensbi-uart8250/compat" "$stage/compat"

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

cp "$stage/upstream/uart8250.c" "$output/"
cp "$stage/upstream/COPYING.BSD" "$output/"
{
  echo "opensbi_uart8250_dense_qualification_version=1"
  echo "upstream_release=v1.8.1"
  echo "upstream_source_sha256=$source_sha"
  echo "upstream_licence_sha256=$licence_sha"
  cat "$output/result.txt"
  echo "status=complete"
} > "$output/summary.txt"
cat "$output/summary.txt"
