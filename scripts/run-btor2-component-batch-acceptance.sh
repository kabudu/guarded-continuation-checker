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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-component-batch-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
controller=examples/btor2/components/braking-controller-v1.btor2
wrong_controller=examples/btor2/components/motor-stop-controller-v1.btor2
admitted=examples/btor2/components/braking-batch-admitted-v1.txt
mixed=examples/btor2/components/braking-batch-mixed-v1.txt

printf '%s\n' 'schema_version,case,expected_route,actual_route,safe,unsafe,artifact_verified,rejection_observed,status' >"$output"

run_case() {
  name=$1
  manifest=$2
  expected_route=$3
  expected_safe=$4
  expected_unsafe=$5
  artifact=$scratch/$name.component-batch
  produced=$("$binary" check-btor2-component-batch \
    "$controller" "$manifest" "$artifact")
  route=$(printf '%s\n' "$produced" | sed -n 's/.* route=\([^ ]*\).*/\1/p')
  safe=$(printf '%s\n' "$produced" | sed -n 's/.* safe=\([0-9][0-9]*\).*/\1/p')
  unsafe=$(printf '%s\n' "$produced" | sed -n 's/.* unsafe=\([0-9][0-9]*\).*/\1/p')
  test "$route" = "$expected_route"
  test "$safe" = "$expected_safe"
  test "$unsafe" = "$expected_unsafe"
  "$binary" verify-btor2-component-batch \
    "$controller" "$manifest" "$artifact" >/dev/null
  printf '1,%s,%s,%s,%s,%s,true,false,accepted\n' \
    "$name" "$expected_route" "$route" "$safe" "$unsafe" >>"$output"
}

run_case admitted-batch "$admitted" reusable 2 0
run_case mixed-batch "$mixed" ordinary 1 1

admitted_artifact=$scratch/admitted-batch.component-batch
if "$binary" verify-btor2-component-batch \
  "$controller" "$mixed" "$admitted_artifact" >/dev/null 2>&1; then
  echo "query drift unexpectedly verified" >&2
  exit 1
fi
printf '%s\n' '1,query-drift,rejected,rejected,0,0,false,true,accepted' >>"$output"

if "$binary" verify-btor2-component-batch \
  "$wrong_controller" "$admitted" "$admitted_artifact" >/dev/null 2>&1; then
  echo "controller drift unexpectedly verified" >&2
  exit 1
fi
printf '%s\n' '1,controller-drift,rejected,rejected,0,0,false,true,accepted' >>"$output"

mutated=$scratch/mutated.component-batch
sed '1s/version/versionx/' "$admitted_artifact" >"$mutated"
if "$binary" verify-btor2-component-batch \
  "$controller" "$admitted" "$mutated" >/dev/null 2>&1; then
  echo "mutated artifact unexpectedly verified" >&2
  exit 1
fi
printf '%s\n' '1,artifact-mutation,rejected,rejected,0,0,false,true,accepted' >>"$output"

echo "btor2 component batch acceptance status=ACCEPTED cases=5 output=$output"
