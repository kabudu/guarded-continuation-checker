#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 OUTPUT_DIR" >&2
  exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=$1
image="silkeh/clang@sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"

if [ -e "$output" ]; then
  echo "guarded MMIO reference output already exists: $output" >&2
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

output_abs=$(CDPATH='' cd -- "$output" && pwd)

docker run --rm \
  --user "$(id -u):$(id -g)" \
  -v "$root:/work:ro" \
  -v "$output_abs:/out" \
  "$image" \
  sh -lc '
    set -eu
    source=/work/corpus/firmware/opentitan-pwm-dif
    mkdir -p \
      /out/source/upstream \
      /out/source/compat/hw/top \
      /out/source/compat/sw/device/lib/base \
      /out/source/compat/sw/device/lib/dif
    cp \
      "$source/LICENSE" \
      "$source/PROVENANCE.md" \
      "$source/toolchain-v1.txt" \
      /out/source/
    cp "$source/upstream/dif_pwm.c" /out/source/upstream/
    cp \
      "$source/compat/assert.h" \
      "$source/compat/firmware_caller.c" \
      "$source/compat/native_runtime_reference.c" \
      /out/source/compat/
    cp "$source/compat/hw/top/pwm_regs.h" /out/source/compat/hw/top/
    cp "$source/compat/sw/device/lib/base/bitfield.h" \
      /out/source/compat/sw/device/lib/base/
    cp \
      "$source/compat/sw/device/lib/dif/dif_base.h" \
      "$source/compat/sw/device/lib/dif/dif_pwm.h" \
      /out/source/compat/sw/device/lib/dif/
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
        -Wall \
        -Wextra \
        -Werror \
        -ffreestanding \
        -fno-builtin \
        -nostdlib \
        -fuse-ld=lld \
        "$optimization" \
        -DGCC_RUNTIME_CHANNEL_CONTROL \
        -I"$source/compat" \
        -Wl,-e,gcc_firmware_entry \
        -Wl,-Ttext=0x80000000 \
        -Wl,--build-id=none \
        -o "/out/$profile/firmware.elf" \
        "$source/upstream/dif_pwm.c" \
        "$source/compat/firmware_caller.c"
      llvm-objcopy \
        -O binary \
        "/out/$profile/firmware.elf" \
        "/out/$profile/firmware.bin"
      llvm-nm \
        --defined-only \
        --numeric-sort \
        "/out/$profile/firmware.elf" \
        > "/out/$profile/firmware.symbols.txt"
      clang \
        -std=c11 \
        -Wall \
        -Wextra \
        -Werror \
        -fno-builtin \
        "$optimization" \
        -DGCC_RUNTIME_CHANNEL_CONTROL \
        -I"$source/compat" \
        -o "/out/$profile/native-reference" \
        "$source/upstream/dif_pwm.c" \
        "$source/compat/firmware_caller.c" \
        "$source/compat/native_runtime_reference.c"
      "/out/$profile/native-reference" > "/out/$profile/native-reference.txt"
    done
  '

cargo build \
  --quiet \
  --manifest-path "$root/Cargo.toml" \
  --example exact_compiled_mmio_reference
reference="$root/target/debug/examples/exact_compiled_mmio_reference"

for profile in o0 o2; do
  "$reference" \
    "$output_abs/$profile/firmware.bin" \
    "$output_abs/$profile/firmware.symbols.txt" \
    > "$output_abs/$profile/exact-reference.txt"
  grep -E '^(input_behavior|input_event)=' \
    "$output_abs/$profile/exact-reference.txt" \
    > "$output_abs/$profile/exact-reference-semantics.txt"
  cmp \
    "$output_abs/$profile/native-reference.txt" \
    "$output_abs/$profile/exact-reference-semantics.txt"
done

cmp \
  "$output_abs/o0/exact-reference-semantics.txt" \
  "$output_abs/o2/exact-reference-semantics.txt"

(
  cd "$output_abs"
  {
    echo "guarded_mmio_reference_build_version=1"
    echo "image_digest=sha256:47a73461b8cfb57f0b22988e69cd57992581a35d1a15bc2220eb3a21ab1fc5d3"
    echo "clang_version=21.1.5"
    echo "target=riscv32-unknown-elf"
    echo "march=rv32imc"
    echo "mabi=ilp32"
    echo "profiles=o0,o2"
    find source -type f -print | LC_ALL=C sort | xargs sha256sum
    sha256sum \
      o0/firmware.elf \
      o0/firmware.bin \
      o0/firmware.symbols.txt \
      o0/native-reference.txt \
      o0/exact-reference.txt \
      o2/firmware.elf \
      o2/firmware.bin \
      o2/firmware.symbols.txt \
      o2/native-reference.txt \
      o2/exact-reference.txt
    echo "status=complete"
  } > manifest.txt
)

trap - EXIT HUP INT TERM
echo "guarded-mmio-reference-build=PASS output=$output_abs"
