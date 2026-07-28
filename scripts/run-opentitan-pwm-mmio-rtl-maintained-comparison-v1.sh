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
expected_yosys=b8e7da6f40ae8f552c116bf6c359b07c6533e159
expected_z3='Z3 version 4.16.0'
python_image='python:3.13-slim@sha256:6771159cd4fa5d9bba1258caf0b82e6b73458c694d178ad97c5e925c2d0e1a91'
model_btor2="$repo/corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"

[[ -x $yosys && -x $z3 ]] || {
  echo "Yosys and Z3 must be executable" >&2
  exit 2
}
[[ $($yosys -V) == *"git sha1 $expected_yosys,"* ]] || {
  echo "Yosys revision mismatch" >&2
  exit 2
}
[[ $($z3 --version) == "$expected_z3"* ]] || {
  echo "Z3 version mismatch" >&2
  exit 2
}
[[ ! -e $output && ! -L $output ]] || {
  echo "refusing to overwrite output directory" >&2
  exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-mmio-rtl-maintained.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
mkdir -p "$scratch/result"

case $(uname -s) in
  Darwin)
    time_style=bsd
    ;;
  Linux)
    time_style=gnu
    ;;
  *)
    echo "unsupported timing platform" >&2
    exit 2
    ;;
esac
platform="$(uname -s)-$(uname -m)"

run_timed() {
  local operation=$1
  local profile=$2
  local stdout=$3
  shift 3
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
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$operation" "$profile" "$elapsed" "$user" "$system" "$peak_bytes" \
    "$time_style" "$platform" complete >>"$scratch/result/resources.csv"
}

append_reported_resources() {
  local operation=$1
  local profile=$2
  local report=$3
  local prefix=$4
  local reported_platform=$5
  local wall user system peak
  wall=$(sed -n "s/^${prefix}_wall_seconds=//p" "$report")
  user=$(sed -n "s/^${prefix}_user_seconds=//p" "$report")
  system=$(sed -n "s/^${prefix}_system_seconds=//p" "$report")
  peak=$(sed -n "s/^${prefix}_peak_rss_bytes=//p" "$report")
  [[ -n $wall && -n $user && -n $system && -n $peak ]]
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$operation" "$profile" "$wall" "$user" "$system" "$peak" \
    self-reported "$reported_platform" complete \
    >>"$scratch/result/resources.csv"
}

printf '%s\n' \
  'schema_version,operation,profile,wall_seconds,user_seconds,system_seconds,peak_rss_bytes,time_backend,platform,status' \
  >"$scratch/result/resources.csv"

"$repo/scripts/build-opentitan-pwm-guarded-mmio-reference-v1.sh" \
  "$scratch/build" >"$scratch/result/build.log"

for profile in o0 o2; do
  run_timed angr-container "$profile" "$scratch/result/$profile-angr.txt" \
    docker run --rm \
      -v "$repo/scripts/angr-guarded-mmio-domain-baseline-v1.py:/baseline.py:ro" \
      -v "$scratch/build:/build:ro" \
      -v "$scratch/result:/result" \
      "$python_image" \
      sh -lc "
        set -eu
        python -m pip install \
          --disable-pip-version-check \
          --no-cache-dir \
          --quiet \
          angr==9.3.0
        python /baseline.py /build/$profile/firmware.elf
        if [ '$profile' = o0 ]; then
          cp /build/o0/firmware.elf /tmp/bad-magic.elf
          python -c 'p=\"/tmp/bad-magic.elf\"; b=bytearray(open(p,\"rb\").read()); b[0]=0; open(p,\"wb\").write(b)'
          if python /baseline.py /tmp/bad-magic.elf >/tmp/bad-magic.out 2>&1; then
            echo 'changed firmware was accepted' >&2
            exit 1
          fi
          cp /build/o0/firmware.elf /tmp/bad-symbol.elf
          python -c 'p=\"/tmp/bad-symbol.elf\"; b=open(p,\"rb\").read(); old=b\"gcc_firmware_entry\"; new=b\"xcc_firmware_entry\"; assert b.count(old) >= 1; open(p,\"wb\").write(b.replace(old,new))'
          if python /baseline.py /tmp/bad-symbol.elf >/tmp/bad-symbol.out 2>&1; then
            echo 'changed symbol was accepted' >&2
            exit 1
          fi
          printf '%s\n' \
            'hostile_control=firmware-magic,status=refused' \
            'hostile_control=firmware-symbol,status=refused' \
            'hostile_controls=2' \
            'status=complete' \
            >/result/firmware-hostile.txt
        fi
      "
  grep -E '^(behavior|event|coverage|disjoint|status)=' \
    "$scratch/result/$profile-angr.txt" \
    >"$scratch/result/$profile-angr-semantics.txt"
  append_reported_resources \
    angr-analysis "$profile" "$scratch/result/$profile-angr.txt" \
    analysis linux-container
done
cmp \
  "$scratch/result/o0-angr-semantics.txt" \
  "$scratch/result/o2-angr-semantics.txt"

run_timed rtl-synthesis shared "$scratch/result/yosys.log" \
  env \
    GCC_PWM_HARNESS="$repo/corpus/rtl/opentitan-pwm-channel-family/firmware-trace-harness.sv" \
    GCC_PWM_TOP=opentitan_pwm_firmware_trace_harness \
    GCC_PWM_OUTPUT_FORMAT=smt2 \
    "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
    "$yosys" \
    "$scratch/unused-2.smt2" \
    "$scratch/unused-4.smt2" \
    "$scratch/firmware-trace-6.smt2"

for profile in o0 o2; do
  run_timed rtl-translation-and-smt "$profile" \
    "$scratch/result/$profile-maintained-rtl.txt" \
    python3 "$repo/scripts/mmio-rtl-yosys-z3-baseline-v1.py" \
      "$scratch/result/$profile-angr.txt" \
      "$scratch/firmware-trace-6.smt2" \
      "$z3"
  sed -n 's/^maintained_rtl_trace=/rtl_trace_phase_cycle=/p' \
    "$scratch/result/$profile-maintained-rtl.txt" \
    >"$scratch/result/$profile-maintained-semantics.txt"
  append_reported_resources \
    rtl-translation-and-smt-analysis "$profile" \
    "$scratch/result/$profile-maintained-rtl.txt" replay "$platform"
done
cmp \
  "$scratch/result/o0-maintained-semantics.txt" \
  "$scratch/result/o2-maintained-semantics.txt"

python3 "$repo/scripts/test-mmio-rtl-maintained-hostile-v1.py" \
  "$repo/scripts/mmio-rtl-yosys-z3-baseline-v1.py" \
  "$scratch/result/o0-angr.txt" \
  >"$scratch/result/translator-hostile.txt"
grep -q '^hostile_controls=6$' "$scratch/result/translator-hostile.txt"
grep -q '^hostile_controls=2$' "$scratch/result/firmware-hostile.txt"

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
  "$scratch/hostile-6.smt2" \
  >"$scratch/hostile-source.stdout" \
  2>"$scratch/hostile-source.stderr"; then
  echo "changed RTL source was accepted" >&2
  exit 1
fi
[[ ! -e $scratch/hostile-2.smt2 ]]
printf '%s\n' \
  'hostile_control=rtl-source,status=refused' \
  'hostile_controls=1' \
  'status=complete' \
  >"$scratch/result/source-hostile.txt"

if "$repo/scripts/build-opentitan-pwm-authentic-channel-family-v1.sh" \
  /bin/false \
  "$scratch/tool-2.smt2" \
  "$scratch/tool-4.smt2" \
  "$scratch/tool-6.smt2" \
  >"$scratch/yosys-identity.stdout" \
  2>"$scratch/yosys-identity.stderr"; then
  echo "changed Yosys identity was accepted" >&2
  exit 1
fi
if python3 "$repo/scripts/mmio-rtl-yosys-z3-baseline-v1.py" \
  "$scratch/result/o0-angr.txt" \
  "$scratch/firmware-trace-6.smt2" \
  /bin/false \
  >"$scratch/z3-identity.stdout" \
  2>"$scratch/z3-identity.stderr"; then
  echo "changed Z3 identity was accepted" >&2
  exit 1
fi
printf '%s\n' \
  'hostile_control=yosys-identity,status=refused' \
  'hostile_control=z3-identity,status=refused' \
  'hostile_controls=2' \
  'status=complete' \
  >"$scratch/result/tool-hostile.txt"

cargo build \
  --quiet \
  --manifest-path "$repo/Cargo.toml" \
  --example compiled_mmio_pwm_rtl
mapper="$repo/target/debug/examples/compiled_mmio_pwm_rtl"
for profile in o0 o2; do
  run_timed gcc-production-and-verification "$profile" \
    "$scratch/result/$profile-gcc.txt" \
    "$mapper" \
      "$scratch/build/$profile/firmware.bin" \
      "$scratch/build/$profile/firmware.symbols.txt" \
      "$model_btor2"
  grep '^rtl_trace_phase_cycle=' "$scratch/result/$profile-gcc.txt" \
    >"$scratch/result/$profile-gcc-semantics.txt"
  cmp \
    "$scratch/result/$profile-maintained-semantics.txt" \
    "$scratch/result/$profile-gcc-semantics.txt"
done

if [[ $(grep -c '^behavior=' "$scratch/result/o0-angr.txt") -ne 7 ]]; then
  echo "maintained firmware partition does not contain seven behaviors" >&2
  exit 1
fi
if [[ $(grep -c '^maintained_rtl_trace=' \
  "$scratch/result/o0-maintained-rtl.txt") -ne 6 ]]; then
  echo "maintained RTL result does not contain six valid members" >&2
  exit 1
fi
grep -q '^maintained_invalid_rtl_members=0$' \
  "$scratch/result/o0-maintained-rtl.txt"
grep -q '^maintained_rtl_transitions=198$' \
  "$scratch/result/o0-maintained-rtl.txt"
grep -q '^maintained_rtl_observations=204$' \
  "$scratch/result/o0-maintained-rtl.txt"
grep -q '^maintained_phase_cycle_classes=2$' \
  "$scratch/result/o0-maintained-rtl.txt"
grep -q '^maintained_nonzero_traces=6$' \
  "$scratch/result/o0-maintained-rtl.txt"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

{
  echo "opentitan_pwm_mmio_rtl_maintained_comparison_version=1"
  echo "angr_version=9.3.0"
  echo "yosys_revision=$expected_yosys"
  echo "z3_version=$($z3 --version)"
  echo "firmware_behaviors=7"
  echo "valid_rtl_members=6"
  echo "invalid_rtl_members=0"
  echo "rtl_transitions=198"
  echo "rtl_observations=204"
  echo "phase_cycle_classes=2"
  echo "nonzero_traces=6"
  echo "hostile_controls=11"
  echo "profiles_semantically_identical=true"
  echo "gcc_maintained_agreement=true"
  printf 'rtl_smt2_sha256=%s\n' \
    "$(sha256_file "$scratch/firmware-trace-6.smt2")"
  printf 'semantic_sha256=%s\n' \
    "$(sha256_file "$scratch/result/o0-maintained-semantics.txt")"
  echo "status=complete"
} >"$scratch/result/semantic-manifest-v1.txt"

mv "$scratch/result" "$output"
echo "opentitan_pwm_mmio_rtl_maintained_comparison_v1=PASS output=$output"
