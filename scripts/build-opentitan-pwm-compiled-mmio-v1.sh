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
        -Wl,-Ttext=0x80000000 \
        -Wl,--build-id=none \
        -o "/out/$profile/firmware.elf" \
        "$source/upstream/dif_pwm.c" \
        "$source/compat/firmware_caller.c"
      llvm-objdump \
        -d \
        --no-show-raw-insn \
        "/out/$profile/firmware.elf" \
        > "/out/$profile/firmware.disassembly.txt"
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
        -fno-builtin \
        "$optimization" \
        -I"$source/compat" \
        -o "/out/$profile/native-firmware" \
        "$source/upstream/dif_pwm.c" \
        "$source/compat/firmware_caller.c" \
        "$source/compat/native_main.c"
      "/out/$profile/native-firmware" > "/out/$profile/native-events.txt"
    done
    mkdir -p /out/runtime-channel
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
      -DGCC_RUNTIME_CHANNEL_CONTROL \
      -I"$source/compat" \
      -Wl,-e,gcc_firmware_entry \
      -Wl,-Ttext=0x80000000 \
      -Wl,--build-id=none \
      -o /out/runtime-channel/firmware.elf \
      "$source/upstream/dif_pwm.c" \
      "$source/compat/firmware_caller.c"
    llvm-objcopy \
      -O binary \
      /out/runtime-channel/firmware.elf \
      /out/runtime-channel/firmware.bin
    llvm-nm \
      --defined-only \
      --numeric-sort \
      /out/runtime-channel/firmware.elf \
      > /out/runtime-channel/firmware.symbols.txt
  '

cargo build --quiet --manifest-path "$root/Cargo.toml" --example extract_compiled_mmio
extractor="$root/target/debug/examples/extract_compiled_mmio"
cargo build --quiet --manifest-path "$root/Cargo.toml" --bin guarded-continuation-checker
certifier="$root/target/debug/guarded-continuation-checker"
cargo build --quiet --manifest-path "$root/Cargo.toml" --example hostile_compiled_mmio_files
hostile_checker="$root/target/debug/examples/hostile_compiled_mmio_files"
for profile in o0 o2; do
  "$extractor" \
    "$output_abs/$profile/firmware.bin" \
    "$output_abs/$profile/firmware.symbols.txt" \
    > "$output_abs/$profile/extracted-events.txt"
  awk -F, '
    /^instruction_count=/ { next }
    /^event=/ { print $1 "," $2 "," $3 "," $4; next }
    { print }
  ' "$output_abs/$profile/extracted-events.txt" \
    > "$output_abs/$profile/extracted-events.normalized.txt"
  cmp \
    "$output_abs/$profile/native-events.txt" \
    "$output_abs/$profile/extracted-events.normalized.txt"
  rm "$output_abs/$profile/extracted-events.normalized.txt"
  input_root="$output_abs/$profile/certificate-inputs"
  mkdir -p \
    "$input_root/upstream" \
    "$input_root/compat/hw/top" \
    "$input_root/compat/sw/device/lib/base" \
    "$input_root/compat/sw/device/lib/dif"
  cp \
    "$root/corpus/firmware/opentitan-pwm-dif/LICENSE" \
    "$root/corpus/firmware/opentitan-pwm-dif/PROVENANCE.md" \
    "$root/corpus/firmware/opentitan-pwm-dif/toolchain-v1.txt" \
    "$input_root/"
  cp \
    "$root/corpus/firmware/opentitan-pwm-dif/upstream/dif_pwm.c" \
    "$input_root/upstream/"
  cp \
    "$root/corpus/firmware/opentitan-pwm-dif/compat/assert.h" \
    "$root/corpus/firmware/opentitan-pwm-dif/compat/firmware_caller.c" \
    "$input_root/compat/"
  cp \
    "$root/corpus/firmware/opentitan-pwm-dif/compat/hw/top/pwm_regs.h" \
    "$input_root/compat/hw/top/"
  cp \
    "$root/corpus/firmware/opentitan-pwm-dif/compat/sw/device/lib/base/bitfield.h" \
    "$input_root/compat/sw/device/lib/base/"
  cp \
    "$root/corpus/firmware/opentitan-pwm-dif/compat/sw/device/lib/dif/dif_base.h" \
    "$root/corpus/firmware/opentitan-pwm-dif/compat/sw/device/lib/dif/dif_pwm.h" \
    "$input_root/compat/sw/device/lib/dif/"
  cp \
    "$output_abs/$profile/firmware.bin" \
    "$output_abs/$profile/firmware.symbols.txt" \
    "$input_root/"
  {
    echo "gcc-compiled-mmio-input-manifest-v1"
    echo "upstream_count=3"
    echo "upstream=LICENSE,LICENSE"
    echo "upstream=PROVENANCE.md,PROVENANCE.md"
    echo "upstream=upstream/dif_pwm.c,upstream/dif_pwm.c"
    echo "compatibility_count=6"
    echo "compatibility=compat/assert.h,compat/assert.h"
    echo "compatibility=compat/firmware_caller.c,compat/firmware_caller.c"
    echo "compatibility=compat/hw/top/pwm_regs.h,compat/hw/top/pwm_regs.h"
    echo "compatibility=compat/sw/device/lib/base/bitfield.h,compat/sw/device/lib/base/bitfield.h"
    echo "compatibility=compat/sw/device/lib/dif/dif_base.h,compat/sw/device/lib/dif/dif_base.h"
    echo "compatibility=compat/sw/device/lib/dif/dif_pwm.h,compat/sw/device/lib/dif/dif_pwm.h"
    echo "toolchain=toolchain-v1.txt"
    echo "image=firmware.bin"
    echo "symbols=firmware.symbols.txt"
    echo "status=complete"
  } > "$input_root/inputs.txt"
  "$certifier" compiled-mmio-certify \
    "$input_root" \
    inputs.txt \
    "$output_abs/$profile/compiled-mmio-certificate-v1.bin" \
    > "$output_abs/$profile/compiled-mmio-certificate-v1.txt"
  "$certifier" compiled-mmio-verify \
    "$input_root" \
    inputs.txt \
    "$output_abs/$profile/compiled-mmio-certificate-v1.bin" \
    > "$output_abs/$profile/compiled-mmio-certificate-v1.verify.txt"
  "$hostile_checker" \
    "$input_root" \
    inputs.txt \
    "$output_abs/$profile/compiled-mmio-certificate-v1.bin" \
    > "$output_abs/$profile/compiled-mmio-hostile-v1.txt"
done
if "$extractor" \
  "$output_abs/runtime-channel/firmware.bin" \
  "$output_abs/runtime-channel/firmware.symbols.txt" \
  > "$output_abs/runtime-channel/extracted-events.txt" \
  2> "$output_abs/runtime-channel/refusal.txt"; then
  echo "runtime-selected channel unexpectedly produced an event stream" >&2
  exit 1
fi
grep "runtime-unknown" "$output_abs/runtime-channel/refusal.txt" >/dev/null
echo "runtime_channel_refusal=PASS" \
  > "$output_abs/runtime-channel/status.txt"
awk -F, '
  /^instruction_count=/ { next }
  /^event=/ { print $1 "," $2 "," $3 "," $4; next }
  { print }
' "$output_abs/o0/extracted-events.txt" \
  > "$output_abs/o0/extracted-events.normalized.txt"
awk -F, '
  /^instruction_count=/ { next }
  /^event=/ { print $1 "," $2 "," $3 "," $4; next }
  { print }
' "$output_abs/o2/extracted-events.txt" \
  > "$output_abs/o2/extracted-events.normalized.txt"
cmp \
  "$output_abs/o0/extracted-events.normalized.txt" \
  "$output_abs/o2/extracted-events.normalized.txt"
rm \
  "$output_abs/o0/extracted-events.normalized.txt" \
  "$output_abs/o2/extracted-events.normalized.txt"

docker run --rm \
  -v "$output_abs:/out" \
  "$image" \
  sh -lc '
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
        /out/o0/firmware.bin \
        /out/o0/firmware.disassembly.txt \
        /out/o0/firmware.symbols.txt \
        /out/o0/native-events.txt \
        /out/o0/extracted-events.txt \
        /out/o0/compiled-mmio-certificate-v1.bin \
        /out/o0/compiled-mmio-certificate-v1.txt \
        /out/o0/compiled-mmio-certificate-v1.verify.txt \
        /out/o0/compiled-mmio-hostile-v1.txt \
        /out/o2/firmware.elf \
        /out/o2/firmware.bin \
        /out/o2/firmware.disassembly.txt \
        /out/o2/firmware.symbols.txt \
        /out/o2/native-events.txt \
        /out/o2/extracted-events.txt \
        /out/o2/compiled-mmio-certificate-v1.bin \
        /out/o2/compiled-mmio-certificate-v1.txt \
        /out/o2/compiled-mmio-certificate-v1.verify.txt \
        /out/o2/compiled-mmio-hostile-v1.txt \
        /out/runtime-channel/firmware.elf \
        /out/runtime-channel/firmware.bin \
        /out/runtime-channel/firmware.symbols.txt \
        /out/runtime-channel/refusal.txt \
        /out/runtime-channel/status.txt |
        sed "s#  /out/#  #"
      echo "status=complete"
    } > /out/manifest.txt
  '

trap - EXIT HUP INT TERM
echo "compiled-mmio-build=PASS output=$output_abs"
