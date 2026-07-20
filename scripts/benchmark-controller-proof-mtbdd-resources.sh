#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2; exit 2
fi
binary=$1
output=$2
trials=${TRIALS:-3}
test -x "$binary"
case $trials in
  '' | *[!0-9]* | 0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
if [ "$trials" -gt 20 ]; then
  echo "TRIALS exceeds the static limit of 20" >&2; exit 2
fi
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2; exit 2
fi

repository=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
manifest=$repository/corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-proof-resources.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
platform=$(uname -s)-$(uname -m)

measure() {
  profile=$1
  operation=$2
  trial=$3
  shift 3
  stdout=$scratch/$profile-$operation-$trial.stdout
  metrics=$scratch/$profile-$operation-$trial.time
  if [ "$time_style" = bsd ]; then
    /usr/bin/time -l "$@" >"$stdout" 2>"$metrics"
    elapsed=$(awk '$2 == "real" { print $1 }' "$metrics")
    peak_bytes=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$metrics")
  else
    /usr/bin/time -f '%e %M' -o "$metrics" "$@" >"$stdout"
    read -r elapsed peak_kib <"$metrics"
    peak_bytes=$((peak_kib * 1024))
  fi
  test -n "$elapsed"
  test -n "$peak_bytes"
  printf '1,%s,%s,%s,%s,%s,%s,%s\n' \
    "$profile" "$operation" "$trial" "$elapsed" "$peak_bytes" "$time_style" "$platform" >>"$output"
}

printf '%s\n' \
  'schema_version,profile,operation,trial,elapsed_seconds,peak_rss_bytes,time_backend,platform' >"$output"
trial=1
while [ "$trial" -le "$trials" ]; do
  compact=$scratch/compact-$trial.mtbdd-plant
  proof=$scratch/proof-$trial.proof-mtbdd-plant
  measure compact create "$trial" "$binary" certify-controller-mtbdd-plant-batch "$manifest" "$compact"
  measure compact verify "$trial" "$binary" verify-controller-mtbdd-plant-batch "$manifest" "$compact"
  measure proof create "$trial" "$binary" certify-controller-proof-mtbdd-plant-batch "$manifest" "$proof"
  measure proof verify "$trial" "$binary" verify-controller-proof-mtbdd-plant-batch "$manifest" "$proof"
  trial=$((trial + 1))
done
echo "controller proof MTBDD resources status=MEASURED trials=$trials output=$output"
