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
cargo build --quiet --manifest-path "$root/Cargo.toml" --example certify_compiled_mmio
certifier="$root/target/debug/examples/certify_compiled_mmio"
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
  "$certifier" \
    "$output_abs/$profile" \
    "$root/corpus/firmware/opentitan-pwm-dif" \
    "$root/corpus/firmware/opentitan-pwm-dif/toolchain-v1.txt" \
    "$output_abs/$profile/compiled-mmio-certificate-v1.bin" \
    > "$output_abs/$profile/compiled-mmio-certificate-v1.txt"
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
        /out/o2/firmware.elf \
        /out/o2/firmware.bin \
        /out/o2/firmware.disassembly.txt \
        /out/o2/firmware.symbols.txt \
        /out/o2/native-events.txt \
        /out/o2/extracted-events.txt \
        /out/o2/compiled-mmio-certificate-v1.bin \
        /out/o2/compiled-mmio-certificate-v1.txt \
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
