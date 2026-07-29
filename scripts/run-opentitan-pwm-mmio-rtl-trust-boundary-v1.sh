#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 YOSYS Z3 OUTPUT_DIRECTORY" >&2
  exit 2
fi

yosys=$1
z3=$2
output=$3
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
model="$repo/corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
expected_semantic=e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf

[[ -x $yosys && -x $z3 ]] || {
  echo "Yosys and Z3 must be executable" >&2
  exit 2
}
[[ ! -e $output && ! -L $output ]] || {
  echo "refusing to overwrite trust-boundary output" >&2
  exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-mmio-trust-boundary.XXXXXXXX")
angr_image="gcc-angr-mmio-maintained-v1:$(basename "$scratch" | tr '[:upper:]' '[:lower:]')"
cleanup() {
  docker image rm --force "$angr_image" >/dev/null 2>&1 || true
  rm -rf "$scratch"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$scratch/result"

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *)
    echo "unsupported timing platform" >&2
    exit 2
    ;;
esac
platform="$(uname -s)-$(uname -m)"

run_timed() {
  local operation=$1
  local phase=$2
  local trial=$3
  local stdout=$4
  shift 4
  local timing="$stdout.time"
  local elapsed user system peak_bytes peak_kib
  if [[ $time_style == bsd ]]; then
    /usr/bin/time -l "$@" >"$stdout" 2>"$timing"
    elapsed=$(awk '$2 == "real" { print $1 }' "$timing")
    user=$(awk '$4 == "user" { print $3 }' "$timing")
    system=$(awk '$6 == "sys" { print $5 }' "$timing")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$timing")
  else
    /usr/bin/time -f '%e %U %S %M' -o "$timing" "$@" >"$stdout"
    read -r elapsed user system peak_kib <"$timing"
    peak_bytes=$((peak_kib * 1024))
  fi
  [[ -n $elapsed && -n $user && -n $system && -n $peak_bytes ]]
  printf '1,%s,%s,o2,%s,%s,%s,%s,%s,%s,%s,complete\n' \
    "$operation" "$phase" "$trial" "$elapsed" "$user" "$system" \
    "$peak_bytes" "$time_style" "$platform" \
    >>"$scratch/result/resources.csv"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

file_bytes() {
  wc -c <"$1" | tr -d ' '
}

normalize_summary() {
  grep -E '^(firmware_behaviors|valid_rtl_members|invalid_rtl_members|rtl_transitions|rtl_observations|phase_cycle_classes|nonzero_traces|rtl_trace_phase_cycle)=' \
    "$1"
}

printf '%s\n' \
  'schema_version,operation,phase,profile,trial,wall_seconds,user_seconds,system_seconds,peak_rss_bytes,time_backend,platform,status' \
  >"$scratch/result/resources.csv"

run_timed input-preparation setup 0 "$scratch/result/input-preparation.txt" \
  "$repo/scripts/build-opentitan-pwm-guarded-mmio-reference-v1.sh" \
  "$scratch/build"

run_timed gcc-consumer-tool setup 0 "$scratch/result/gcc-tool-build.txt" \
  env CARGO_TARGET_DIR="$scratch/gcc-target" \
  cargo build --quiet --release \
    --manifest-path "$repo/Cargo.toml" \
    --example verify_compiled_mmio_pwm_rtl
gcc_consumer="$scratch/gcc-target/release/examples/verify_compiled_mmio_pwm_rtl"

run_timed gcc-producer-tool setup 0 "$scratch/result/gcc-producer-build.txt" \
  env CARGO_TARGET_DIR="$scratch/gcc-target" \
  cargo build --quiet \
    --manifest-path "$repo/Cargo.toml" \
    --example compiled_mmio_pwm_rtl
gcc_producer="$scratch/gcc-target/debug/examples/compiled_mmio_pwm_rtl"

run_timed maintained-consumer-tool setup 0 "$scratch/result/angr-image-build.txt" \
  docker build --no-cache \
    --file "$repo/containers/angr-mmio-maintained-v1.Dockerfile" \
    --tag "$angr_image" \
    "$repo/containers"

certificate="$scratch/compiled-mmio-rtl-certificate-v1.bin"
run_timed gcc-certificate-production producer 0 "$scratch/result/gcc-producer.txt" \
  "$gcc_producer" \
  "$scratch/build/o2/firmware.bin" \
  "$scratch/build/o2/firmware.symbols.txt" \
  "$model" \
  "$certificate"

gcc_payload_bytes=$((
  $(file_bytes "$certificate") +
    $(file_bytes "$scratch/build/o2/firmware.bin") +
    $(file_bytes "$scratch/build/o2/firmware.symbols.txt") +
    $(file_bytes "$model")
))
maintained_payload_bytes=$((
  $(file_bytes "$scratch/build/o2/firmware.elf") +
    $(file_bytes "$repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv") +
    $(file_bytes "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_chan.sv") +
    $(file_bytes "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_core.sv") +
    $(file_bytes "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_reg_pkg.sv")
))
{
  echo "schema_version=1"
  echo "gcc_certificate_bytes=$(file_bytes "$certificate")"
  echo "gcc_firmware_bytes=$(file_bytes "$scratch/build/o2/firmware.bin")"
  echo "gcc_symbols_bytes=$(file_bytes "$scratch/build/o2/firmware.symbols.txt")"
  echo "gcc_btor2_bytes=$(file_bytes "$model")"
  echo "gcc_payload_bytes=$gcc_payload_bytes"
  echo "maintained_firmware_elf_bytes=$(file_bytes "$scratch/build/o2/firmware.elf")"
  echo "maintained_rtl_source_bytes=$((maintained_payload_bytes - $(file_bytes "$scratch/build/o2/firmware.elf")))"
  echo "maintained_payload_bytes=$maintained_payload_bytes"
  echo "gcc_consumer_executable_bytes=$(file_bytes "$gcc_consumer")"
  echo "maintained_angr_image_bytes=$(docker image inspect "$angr_image" --format '{{.Size}}')"
  echo "status=complete"
} >"$scratch/result/payloads.txt"

"$gcc_consumer" \
  "$certificate" \
  "$scratch/build/o2/firmware.bin" \
  "$scratch/build/o2/firmware.symbols.txt" \
  "$model" \
  >"$scratch/gcc-warmup.txt"
"$repo/scripts/run-opentitan-pwm-mmio-rtl-maintained-consumer-v1.sh" \
  "$angr_image" "$yosys" "$z3" \
  "$scratch/build/o2/firmware.elf" \
  "$scratch/maintained-warmup" \
  >"$scratch/maintained-warmup.txt"

for trial in 1 2 3 4 5; do
  run_timed gcc-consumer measured "$trial" \
    "$scratch/result/gcc-trial-$trial.txt" \
    "$gcc_consumer" \
      "$certificate" \
      "$scratch/build/o2/firmware.bin" \
      "$scratch/build/o2/firmware.symbols.txt" \
      "$model"
  run_timed maintained-consumer measured "$trial" \
    "$scratch/result/maintained-trial-$trial.txt" \
    "$repo/scripts/run-opentitan-pwm-mmio-rtl-maintained-consumer-v1.sh" \
      "$angr_image" "$yosys" "$z3" \
      "$scratch/build/o2/firmware.elf" \
      "$scratch/maintained-trial-$trial"

  normalize_summary "$scratch/result/gcc-trial-$trial.txt" \
    >"$scratch/gcc-semantic-$trial.txt"
  normalize_summary "$scratch/result/maintained-trial-$trial.txt" \
    >"$scratch/maintained-semantic-$trial.txt"
  cmp "$scratch/gcc-semantic-$trial.txt" \
    "$scratch/maintained-semantic-$trial.txt"
done

cp "$scratch/gcc-semantic-1.txt" "$scratch/result/semantic-summary.txt"
trace_hash=$(
  grep '^rtl_trace_phase_cycle=' "$scratch/result/semantic-summary.txt" |
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum | awk '{print $1}'
    else
      shasum -a 256 | awk '{print $1}'
    fi
)
[[ $trace_hash == "$expected_semantic" ]] || {
  echo "trust-boundary semantic identity differs" >&2
  exit 1
}

gcc_median=$(
  awk -F, '$2 == "gcc-consumer" { print $6 }' \
    "$scratch/result/resources.csv" |
    sort -n | sed -n '3p'
)
maintained_median=$(
  awk -F, '$2 == "maintained-consumer" { print $6 }' \
    "$scratch/result/resources.csv" |
    sort -n | sed -n '3p'
)
gcc_peak=$(
  awk -F, '$2 == "gcc-consumer" { print $9 }' \
    "$scratch/result/resources.csv" |
    sort -n | tail -n 1
)
maintained_peak=$(
  awk -F, '$2 == "maintained-consumer" { print $9 }' \
    "$scratch/result/resources.csv" |
    sort -n | head -n 1
)
awk -v gcc="$gcc_median" -v maintained="$maintained_median" \
  'BEGIN { exit !(gcc * 2 <= maintained) }'
awk -v gcc="$gcc_peak" -v maintained="$maintained_peak" \
  'BEGIN { exit !(gcc * 2 <= maintained) }'

dd if="$certificate" of="$scratch/bad-certificate.bin" bs=1 \
  count=$(( $(file_bytes "$certificate") - 1 )) >/dev/null 2>&1
if "$gcc_consumer" \
  "$scratch/bad-certificate.bin" \
  "$scratch/build/o2/firmware.bin" \
  "$scratch/build/o2/firmware.symbols.txt" \
  "$model" >/dev/null 2>&1; then
  echo "truncated GCC certificate was accepted" >&2
  exit 1
fi
cp "$scratch/build/o2/firmware.bin" "$scratch/bad-firmware.bin"
printf '\000' | dd of="$scratch/bad-firmware.bin" bs=1 seek=0 conv=notrunc \
  >/dev/null 2>&1
if "$gcc_consumer" \
  "$certificate" "$scratch/bad-firmware.bin" \
  "$scratch/build/o2/firmware.symbols.txt" \
  "$model" >/dev/null 2>&1; then
  echo "changed GCC firmware was accepted" >&2
  exit 1
fi
sed 's/gcc_firmware_entry/xcc_firmware_entry/' \
  "$scratch/build/o2/firmware.symbols.txt" >"$scratch/bad-symbols.txt"
cmp -s "$scratch/build/o2/firmware.symbols.txt" "$scratch/bad-symbols.txt" && {
  echo "symbol mutation did not change its input" >&2
  exit 1
}
if "$gcc_consumer" \
  "$certificate" "$scratch/build/o2/firmware.bin" \
  "$scratch/bad-symbols.txt" "$model" >/dev/null 2>&1; then
  echo "changed GCC symbols were accepted" >&2
  exit 1
fi
dd if="$model" of="$scratch/bad-model.btor2" bs=1 \
  count=$(( $(file_bytes "$model") - 1 )) >/dev/null 2>&1
if "$gcc_consumer" \
  "$certificate" "$scratch/build/o2/firmware.bin" \
  "$scratch/build/o2/firmware.symbols.txt" \
  "$scratch/bad-model.btor2" >/dev/null 2>&1; then
  echo "changed GCC model was accepted" >&2
  exit 1
fi
printf '%s\n' \
  'hostile_control=certificate,status=refused' \
  'hostile_control=firmware,status=refused' \
  'hostile_control=symbols,status=refused' \
  'hostile_control=btor2,status=refused' \
  'hostile_controls=4' \
  'status=complete' \
  >"$scratch/result/gcc-hostile.txt"

python3 "$repo/scripts/test-mmio-rtl-maintained-hostile-v1.py" \
  "$repo/scripts/mmio-rtl-yosys-z3-baseline-v1.py" \
  "$scratch/maintained-trial-1/angr.txt" \
  >"$scratch/result/maintained-report-hostile.txt"

cp "$scratch/build/o2/firmware.elf" "$scratch/bad-firmware.elf"
printf '\000' | dd of="$scratch/bad-firmware.elf" bs=1 seek=0 conv=notrunc \
  >/dev/null 2>&1
if "$repo/scripts/run-opentitan-pwm-mmio-rtl-maintained-consumer-v1.sh" \
  "$angr_image" "$yosys" "$z3" \
  "$scratch/bad-firmware.elf" \
  "$scratch/bad-firmware-output" >/dev/null 2>&1; then
  echo "changed maintained firmware was accepted" >&2
  exit 1
fi

hostile_source_repo="$scratch/hostile-source-repo"
mkdir -p \
  "$hostile_source_repo/scripts" \
  "$hostile_source_repo/corpus/rtl/opentitan-pwm-channel-family"
cp \
  "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  "$hostile_source_repo/scripts/"
cp -R \
  "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child" \
  "$hostile_source_repo/corpus/rtl/opentitan-pwm-channel-family/"
cp \
  "$repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv" \
  "$hostile_source_repo/corpus/rtl/opentitan-pwm-channel-family/"
printf '\n' \
  >>"$hostile_source_repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_chan.sv"
if GCC_PWM_HARNESS="$hostile_source_repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv" \
  GCC_PWM_TOP=opentitan_pwm_firmware_trace_harness \
  GCC_PWM_OUTPUT_FORMAT=smt2 \
  "$hostile_source_repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  "$yosys" \
  "$scratch/hostile-2.smt2" \
  "$scratch/hostile-4.smt2" \
  "$scratch/hostile-6.smt2" >/dev/null 2>&1; then
  echo "changed maintained RTL source was accepted" >&2
  exit 1
fi
if "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  /bin/false \
  "$scratch/tool-2.smt2" \
  "$scratch/tool-4.smt2" \
  "$scratch/tool-6.smt2" >/dev/null 2>&1; then
  echo "changed maintained Yosys identity was accepted" >&2
  exit 1
fi
if python3 "$repo/scripts/mmio-rtl-yosys-z3-baseline-v1.py" \
  "$scratch/maintained-trial-1/angr.txt" \
  "$scratch/maintained-trial-1/firmware-trace-6.smt2" \
  /bin/false >/dev/null 2>&1; then
  echo "changed maintained Z3 identity was accepted" >&2
  exit 1
fi
printf '%s\n' \
  'hostile_control=firmware,status=refused' \
  'hostile_control=rtl-source,status=refused' \
  'hostile_control=yosys-identity,status=refused' \
  'hostile_control=z3-identity,status=refused' \
  'hostile_control=report-schema,status=refused,count=6' \
  'hostile_controls=10' \
  'status=complete' \
  >"$scratch/result/maintained-hostile.txt"

{
  echo "trust_boundary_comparison_version=1"
  echo "profile=o2"
  echo "trials=5"
  echo "semantic_sha256=$trace_hash"
  echo "gcc_payload_bytes=$gcc_payload_bytes"
  echo "maintained_payload_bytes=$maintained_payload_bytes"
  echo "gcc_median_wall_seconds=$gcc_median"
  echo "maintained_median_wall_seconds=$maintained_median"
  echo "gcc_max_peak_rss_bytes=$gcc_peak"
  echo "maintained_min_peak_rss_bytes=$maintained_peak"
  echo "gcc_wall_gate=true"
  echo "gcc_memory_gate=true"
  echo "gcc_hostile_controls=4"
  echo "maintained_hostile_controls=10"
  echo "status=complete"
} >"$scratch/result/manifest-v1.txt"

{
  echo "trust_boundary_identity_version=2"
  echo "repository_revision=$(git -C "$repo" rev-parse HEAD)"
  echo "profile=o2"
  echo "trials=5"
  echo "semantic_sha256=$trace_hash"
  echo "certificate_sha256=$(sha256_file "$certificate")"
  echo "firmware_bin_sha256=$(sha256_file "$scratch/build/o2/firmware.bin")"
  echo "firmware_symbols_sha256=$(sha256_file "$scratch/build/o2/firmware.symbols.txt")"
  echo "firmware_elf_sha256=$(sha256_file "$scratch/build/o2/firmware.elf")"
  echo "btor2_sha256=$(sha256_file "$model")"
  echo "pwm_chan_sha256=$(sha256_file "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_chan.sv")"
  echo "pwm_core_sha256=$(sha256_file "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_core.sv")"
  echo "pwm_reg_pkg_sha256=$(sha256_file "$repo/corpus/rtl/opentitan-pwm-channel-family/upstream-child/pwm_reg_pkg.sv")"
  echo "pwm_harness_sha256=$(sha256_file "$repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv")"
  echo "gcc_consumer_sha256=$(sha256_file "$gcc_consumer")"
  echo "angr_dockerfile_sha256=$(sha256_file "$repo/containers/angr-mmio-maintained-v1.Dockerfile")"
  echo "angr_base_image_sha256=6771159cd4fa5d9bba1258caf0b82e6b73458c694d178ad97c5e925c2d0e1a91"
  echo "angr_version=9.3.0"
  echo "yosys_revision=b8e7da6f40ae8f552c116bf6c359b07c6533e159"
  echo "z3_version=4.16.0"
  echo "gcc_payload_bytes=$gcc_payload_bytes"
  echo "maintained_payload_bytes=$maintained_payload_bytes"
  echo "gcc_hostile_controls=4"
  echo "maintained_hostile_controls=10"
  echo "status=complete"
} >"$scratch/result/identity-manifest-v2.txt"

cp "$certificate" "$scratch/result/"
mv "$scratch/result" "$output"
echo "opentitan_pwm_mmio_rtl_trust_boundary_v1=PASS output=$output"
