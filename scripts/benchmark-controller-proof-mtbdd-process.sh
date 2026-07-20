#!/bin/sh
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
  echo "TRIALS exceeds the static limit of 20" >&2; exit 2
fi
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2; exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-proof-process.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
manifest=corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
printf '%s\n' \
  'schema_version,profile,operation,trial,artifact_bytes,elapsed_micros,assignments_checked,answers_agree,status' >"$output"

field() {
  key=$1
  shift
  printf '%s\n' "$*" | sed -n "1s/.* ${key}=\([^ ]*\).*/\1/p"
}

trial=1
while [ "$trial" -le "$trials" ]; do
  compact=$scratch/compact-$trial.mtbdd-plant
  proof=$scratch/proof-$trial.proof-mtbdd-plant
  compact_create=$("$binary" certify-controller-mtbdd-plant-batch "$manifest" "$compact")
  compact_verify=$("$binary" verify-controller-mtbdd-plant-batch "$manifest" "$compact")
  proof_create=$("$binary" certify-controller-proof-mtbdd-plant-batch "$manifest" "$proof")
  proof_verify=$("$binary" verify-controller-proof-mtbdd-plant-batch "$manifest" "$proof")

  compact_answers=$(printf '%s\n' "$compact_verify" | sed -n 's/^controller-mtbdd-plant-member /member /p')
  proof_answers=$(printf '%s\n' "$proof_verify" | sed -n 's/^controller-proof-mtbdd-plant-member /member /p')
  test "$compact_answers" = "$proof_answers"

  for profile in compact proof; do
    for operation in create verify; do
      case $profile-$operation in
        compact-create) row=$compact_create ;;
        compact-verify) row=$compact_verify ;;
        proof-create) row=$proof_create ;;
        proof-verify) row=$proof_verify ;;
      esac
      printf '1,%s,%s,%s,%s,%s,%s,true,ok\n' \
        "$profile" "$operation" "$trial" \
        "$(field artifact_bytes "$row")" \
        "$(field elapsed_micros "$row")" \
        "$(field assignments_checked "$row")" >>"$output"
    done
  done
  trial=$((trial + 1))
done

echo "controller proof MTBDD process benchmark status=MEASURED trials=$trials output=$output"
