#!/usr/bin/env sh
set -eu

if [ "$#" -ne 3 ]; then
  echo "usage: $0 GCC_BINARY SBY.py OUTPUT.csv" >&2
  exit 2
fi

binary=$1
sby=$2
output=$3
trials=${TRIALS:-3}
test -x "$binary"
test -f "$sby"
case $trials in
  '' | *[!0-9]* | 0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
if [ "$trials" -gt 100 ]; then
  echo "TRIALS exceeds the static limit of 100" >&2
  exit 2
fi
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

repository=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
manifest=$repository/corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
oracle=$repository/scripts/test-public-washing-controller-physical-oracle.sh
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-resource.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
platform=$(uname -s)-$(uname -m)

measure() {
  case_name=$1
  trial=$2
  shift 2
  stdout=$scratch/$case_name-$trial.stdout
  metrics=$scratch/$case_name-$trial.time
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
  printf '1,%s,%s,%s,%s,%s,%s\n' \
    "$case_name" "$trial" "$elapsed" "$peak_bytes" "$time_style" "$platform" >>"$output"
}

printf '%s\n' \
  'schema_version,case,trial,elapsed_seconds,peak_rss_bytes,time_backend,platform' >"$output"

trial=1
while [ "$trial" -le "$trials" ]; do
  artifact=$scratch/batch-$trial.mtbdd-plant
  measure gcc_produce "$trial" "$binary" \
    certify-controller-mtbdd-plant-batch "$manifest" "$artifact"
  measure gcc_verify "$trial" "$binary" \
    verify-controller-mtbdd-plant-batch "$manifest" "$artifact"
  measure symbiyosys_oracle "$trial" "$oracle" "$sby"
  trial=$((trial + 1))
done

echo "controller MTBDD process resources status=MEASURED trials=$trials output=$output"
