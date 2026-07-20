#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
test -x "$binary"
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-mtbdd-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
manifest=corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
artifact=$scratch/physical-plant.mtbdd-plant

"$binary" controller-mtbdd-cli-version | grep -q \
  '^controller_mtbdd_cli_version=1 .* unsupported=fail-closed$'
produced=$("$binary" certify-controller-mtbdd-plant-batch \
  "$manifest" "$artifact")
verified=$("$binary" verify-controller-mtbdd-plant-batch \
  "$manifest" "$artifact")
printf '%s\n' "$produced" | grep -q \
  '^controller-mtbdd-plant-batch status=CREATED .* members=6 safe=2 unsafe=4 '
printf '%s\n' "$verified" | grep -q \
  '^controller-mtbdd-plant-batch status=VERIFIED .* members=6 safe=2 unsafe=4 '

printf '%s\n' \
  'schema_version,case,expected_answer,actual_answer,expected_bad_frame,actual_bad_frame,artifact_verified,status' \
  >"$output"
printf '%s\n' "$verified" |
  sed -n 's/^controller-mtbdd-plant-member index=\([0-9][0-9]*\) answer=\([^ ]*\) horizon=[^ ]* bad_frame=\([^ ]*\) .*/\1,\2,\3/p' |
  while IFS=, read -r member actual bad_frame; do
    case $member in
      0) expected=UNSAFE; expected_frame=4 ;;
      1) expected=UNSAFE; expected_frame=7 ;;
      2 | 3) expected=UNSAFE; expected_frame=15 ;;
      4 | 5) expected=SAFE; expected_frame=none ;;
      *) echo "unexpected member $member" >&2; exit 1 ;;
    esac
    test "$actual" = "$expected"
    test "$bad_frame" = "$expected_frame"
    printf '1,member-%s,%s,%s,%s,%s,true,accepted\n' \
      "$member" "$expected" "$actual" "$expected_frame" "$bad_frame" >>"$output"
  done

test "$(wc -l <"$output" | tr -d ' ')" = 7

mismatched=$scratch/mismatched.txt
sed '0,/bad_plant_output=11/s//bad_plant_output=12/' "$manifest" >"$mismatched"
if "$binary" verify-controller-mtbdd-plant-batch \
  "$mismatched" "$artifact" >/dev/null 2>&1; then
  echo "manifest drift unexpectedly verified" >&2
  exit 1
fi
printf '%s\n' '1,manifest-drift,REJECTED,REJECTED,none,none,false,accepted' >>"$output"

mutated=$scratch/mutated.mtbdd-plant
cp "$artifact" "$mutated"
printf '\001' | dd of="$mutated" bs=1 seek=100 conv=notrunc 2>/dev/null
if "$binary" verify-controller-mtbdd-plant-batch \
  "$manifest" "$mutated" >/dev/null 2>&1; then
  echo "artifact mutation unexpectedly verified" >&2
  exit 1
fi
printf '%s\n' '1,artifact-mutation,REJECTED,REJECTED,none,none,false,accepted' >>"$output"

if "$binary" certify-controller-mtbdd-plant-batch \
  "$manifest" "$artifact" >/dev/null 2>&1; then
  echo "existing artifact was unexpectedly overwritten" >&2
  exit 1
fi
printf '%s\n' '1,output-collision,REJECTED,REJECTED,none,none,false,accepted' >>"$output"

test "$(wc -l <"$output" | tr -d ' ')" = 10

echo "controller MTBDD self-service acceptance status=ACCEPTED cases=9 output=$output"
