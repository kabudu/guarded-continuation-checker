#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
image="silkeh/clang@sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"

if [ -e "$output" ]; then
  echo "compiled MMIO output already exists: $output" >&2
  exit 2
fi

mkdir -p "$output"
cleanup() {
  status=$?
  if [ "$status" -ne 0 ]; then
    rm -rf "$output"
  fi
  exit "$status"
}
trap cleanup EXIT HUP INT TERM

output_abs=$(CDPATH= cd -- "$output" && pwd)

docker run --rm \
  -v "$root:/work:ro" \
  -v "$output_abs:/out" \
  "$image" \
  sh -lc '
    set -eu
    source=/work/corpus/firmware/opentitan-pwm-dif
    for profile in o0 o2; do
      mkdir -p "/out/$profile"
      if [ "$profile" = o0 ]; then
        optimization=-O0
      else
        optimization=-O2
      fi
      clang \
        --target=riscv32-unknown-elf \
        -march=rv32imc \
        -mabi=ilp32 \
        -std=c11 \
        -ffreestanding \
        -fno-builtin \
        -nostdlib \
        -fuse-ld=lld \
        "$optimization" \
        -I"$source/compat" \
        -Wl,-e,gcc_firmware_entry \
        -Wl,--build-id=none \
        -o "/out/$profile/firmware.elf" \
        "$source/upstream/dif_pwm.c" \
        "$source/compat/firmware_caller.c"
      llvm-objdump \
        -d \
        --no-show-raw-insn \
        "/out/$profile/firmware.elf" \
        > "/out/$profile/firmware.disassembly.txt"
    done
    {
      echo "compiled_mmio_build_version=1"
      echo "image_digest=sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"
      echo "clang_version=21.1.5"
      echo "llvm_objdump_version=21.1.5"
      echo "target=riscv32-unknown-elf"
      echo "march=rv32imc"
      echo "mabi=ilp32"
      echo "profiles=o0,o2"
      sha256sum \
        /out/o0/firmware.elf \
        /out/o0/firmware.disassembly.txt \
        /out/o2/firmware.elf \
        /out/o2/firmware.disassembly.txt |
        sed "s#  /out/#  #"
      echo "status=complete"
    } > /out/manifest.txt
  '

trap - EXIT HUP INT TERM
echo "compiled-mmio-build=PASS output=$output_abs"
