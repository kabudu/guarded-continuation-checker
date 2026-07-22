#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
trials=${TRIALS:-5}
test -x "$binary"
case $trials in
  '' | *[!0-9]* | 0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
if [ "$trials" -gt 20 ]; then
  echo "TRIALS exceeds the static limit of 20" >&2
  exit 2
fi
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

repository=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
fixture=$repository/corpus/rtl/opentitan-pwm-channel-family
model=$fixture/generated/symbolic-class-6.btor2
queries=$fixture/trace-queries-v1.txt
policy=$fixture/trace-policy-v1.txt
test -f "$model" && test -f "$queries" && test -f "$policy"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-channel-trace-resource.XXXXXXXX")
working_output=$(mktemp "$output.tmp.XXXXXXXX")
cleanup() {
  rm -rf "$scratch"
  if [ -n "$working_output" ]; then
    rm -f "$working_output"
  fi
}
trap cleanup EXIT HUP INT TERM

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
operating_system=$(uname -s)
architecture=$(uname -m)

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

measure() {
  operation=$1
  trial=$2
  artifact=$3
  shift 3
  stdout=$scratch/$operation-$trial.stdout
  metrics=$scratch/$operation-$trial.time
  if [ "$time_style" = bsd ]; then
    if ! /usr/bin/time -l "$@" >"$stdout" 2>"$metrics"; then
      cat "$metrics" >&2
      exit 1
    fi
    elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    if ! /usr/bin/time -f '%e %M' -o "$metrics" "$@" >"$stdout"; then
      cat "$metrics" >&2
      exit 1
    fi
    read -r elapsed peak_kib <"$metrics"
    peak_bytes=$((peak_kib * 1024))
  fi
  test -n "$elapsed" && test -n "$peak_bytes" && test "$peak_bytes" -gt 0
  test "$(grep -c '^btor2-channel-trace index=' "$stdout")" -eq 42
  test "$(grep -c ' answer=SAFE ' "$stdout")" -eq 6
  test "$(grep -c ' answer=UNSAFE ' "$stdout")" -eq 36
  artifact_bytes=$(wc -c <"$artifact" | tr -d ' ')
  test "$artifact_bytes" -eq 4899434
  artifact_sha256=$(sha256_file "$artifact")
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,%s,42,ok\n' \
    "$operation" "$trial" "$elapsed" "$peak_bytes" "$artifact_bytes" \
    "$time_style" "$operating_system" "$architecture" "$artifact_sha256" \
    >>"$working_output"
}

printf '%s\n' \
  'schema_version,operation,trial,elapsed_seconds,peak_rss_bytes,artifact_bytes,time_backend,operating_system,architecture,artifact_sha256,logical_queries,status' \
  >"$working_output"

reference_sha256=
trial=1
while [ "$trial" -le "$trials" ]; do
  artifact=$scratch/result-$trial.channel-traces
  measure certify "$trial" "$artifact" "$binary" \
    certify-btor2-channel-traces "$model" "$queries" "$policy" "$artifact"
  current_sha256=$(sha256_file "$artifact")
  if [ -z "$reference_sha256" ]; then
    reference_sha256=$current_sha256
  else
    test "$current_sha256" = "$reference_sha256"
  fi
  measure verify "$trial" "$artifact" "$binary" \
    verify-btor2-channel-traces "$model" "$queries" "$policy" "$artifact"
  trial=$((trial + 1))
done

ln "$working_output" "$output"
rm -f "$working_output"
working_output=
echo "btor2_channel_trace_process_resources_v1=MEASURED trials=$trials os=$operating_system architecture=$architecture output=$output"
