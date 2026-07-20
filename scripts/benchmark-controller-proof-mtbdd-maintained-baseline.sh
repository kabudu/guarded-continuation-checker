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
if [ "$trials" -gt 20 ]; then
  echo "TRIALS exceeds the static limit of 20" >&2
  exit 2
fi
if [ -e "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

repository=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
manifest=$repository/corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
oracle=$repository/scripts/test-public-washing-controller-physical-oracle.sh
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-proof-maintained.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

case $(uname -s) in
  Darwin) time_style=bsd ;;
  Linux) time_style=gnu ;;
  *) echo "unsupported resource-measurement platform" >&2; exit 2 ;;
esac
platform=$(uname -s)-$(uname -m)

check_proof_answers() {
  stdout=$1
  grep -q '^controller-proof-mtbdd-plant-batch status=.* members=6 safe=2 unsafe=4 .* assignments_checked=0 ' "$stdout"
  expected='0,UNSAFE,4
1,UNSAFE,7
2,UNSAFE,15
3,UNSAFE,15
4,SAFE,none
5,SAFE,none'
  actual=$(sed -n 's/^controller-proof-mtbdd-plant-member index=\([0-9][0-9]*\) answer=\([^ ]*\) horizon=[^ ]* bad_frame=\([^ ]*\) .*/\1,\2,\3/p' "$stdout")
  test "$actual" = "$expected"
}

check_oracle_answers() {
  stdout=$1
  grep -q '^public-washing-physical-oracle=PASS unsafe=4 safe=2 depth=32$' "$stdout"
}

measure() {
  profile=$1
  operation=$2
  trial=$3
  evidence_bytes=$4
  answer_checker=$5
  shift 5
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
  "$answer_checker" "$stdout"
  printf '1,%s,%s,%s,%s,%s,%s,%s,%s,true,ok\n' \
    "$profile" "$operation" "$trial" "$elapsed" "$peak_bytes" \
    "$evidence_bytes" "$time_style" "$platform" >>"$output"
}

printf '%s\n' \
  'schema_version,profile,operation,trial,elapsed_seconds,peak_rss_bytes,portable_evidence_bytes,time_backend,platform,answers_agree,status' >"$output"

trial=1
while [ "$trial" -le "$trials" ]; do
  artifact=$scratch/batch-$trial.proof-mtbdd-plant
  measure proof create "$trial" pending check_proof_answers "$binary" \
    certify-controller-proof-mtbdd-plant-batch "$manifest" "$artifact"
  artifact_bytes=$(wc -c <"$artifact" | tr -d ' ')
  sed -i.bak "s/,pending,\([^,]*,[^,]*,true,ok\)$/,$artifact_bytes,\1/" "$output"
  rm -f "$output.bak"
  measure proof verify "$trial" "$artifact_bytes" check_proof_answers "$binary" \
    verify-controller-proof-mtbdd-plant-batch "$manifest" "$artifact"
  measure maintained-formal oracle "$trial" 0 check_oracle_answers "$oracle" "$sby"
  trial=$((trial + 1))
done

echo "controller proof MTBDD maintained baseline status=MEASURED trials=$trials output=$output"
