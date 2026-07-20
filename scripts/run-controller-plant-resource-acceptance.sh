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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-controller-resource.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
admitted_manifest=corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
fallback_manifest=corpus/controller-plant-portfolio/manifest-v1.txt
admitted_artifact=$scratch/admitted.controller-plant
fallback_artifact=$scratch/fallback.controller-plant
permissive=$scratch/permissive.policy

cat >"$permissive" <<'EOF'
controller_plant_resource_policy_version=1
max_artifact_bytes=16777216
max_members=64
max_member_horizon=1024
max_product_states_per_member=4096
max_transition_evaluations=18446744073709551615
status=complete
EOF

"$binary" controller-plant-resource-cli-version | grep -q \
  '^controller_plant_resource_cli_version=1 .* refusal_exit=3 .* result_on_refusal=none refusal_schema=reason-v1 unsupported=fail-closed$'
"$binary" certify-controller-plant-portfolio \
  "$admitted_manifest" "$admitted_artifact" >/dev/null
"$binary" certify-controller-plant-portfolio \
  "$fallback_manifest" "$fallback_artifact" >/dev/null

verify_governed() {
  if [ "$(uname -s)" = Linux ]; then
    command -v prlimit >/dev/null
    prlimit --as=67108864 -- \
      "$binary" verify-controller-plant-portfolio-resources "$@"
  else
    "$binary" verify-controller-plant-portfolio-resources "$@"
  fi
}

admitted=$(verify_governed \
  "$admitted_manifest" "$permissive" "$admitted_artifact")
fallback=$(verify_governed \
  "$fallback_manifest" "$permissive" "$fallback_artifact")

field() {
  printf '%s\n' "$1" | sed -n "1s/.* $2=\\([^ ]*\\).*/\\1/p"
}

test "$(field "$admitted" backend)" = MTBDD
test "$(field "$admitted" safe)" = 2
test "$(field "$admitted" unsafe)" = 4
test "$(field "$fallback" backend)" = DIRECT_EXACT
test "$(field "$fallback" safe)" = 1
test "$(field "$fallback" unsafe)" = 1

members_policy=$scratch/members.policy
sed 's/max_members=64/max_members=5/' "$permissive" >"$members_policy"
transitions_policy=$scratch/transitions.policy
sed 's/max_transition_evaluations=18446744073709551615/max_transition_evaluations=1/' \
  "$permissive" >"$transitions_policy"
malformed_policy=$scratch/malformed.policy
sed 's/max_members=64/max_members=064/' "$permissive" >"$malformed_policy"
corrupt_artifact=$scratch/corrupt.controller-plant
cp "$fallback_artifact" "$corrupt_artifact"
printf '\001' | dd of="$corrupt_artifact" bs=1 seek=40 conv=notrunc 2>/dev/null

run_failure() {
  manifest=$1
  policy=$2
  artifact=$3
  expected_exit=$4
  expected=$5
  if failure=$(verify_governed "$manifest" "$policy" "$artifact" 2>&1); then
    echo "governed verification unexpectedly succeeded" >&2
    exit 1
  else
    actual_exit=$?
  fi
  test "$actual_exit" -eq "$expected_exit"
  printf '%s\n' "$failure" | grep -q "$expected"
}

run_failure "$admitted_manifest" "$members_policy" "$admitted_artifact" 3 \
  '^error: controller-plant-resource refusal=members result=none$'
run_failure "$fallback_manifest" "$transitions_policy" "$fallback_artifact" 3 \
  '^error: controller-plant-resource refusal=transition-evaluations result=none$'
run_failure "$fallback_manifest" "$malformed_policy" "$fallback_artifact" 2 \
  '^error: controller plant resource policy members is noncanonical$'
run_failure "$fallback_manifest" "$permissive" "$corrupt_artifact" 2 \
  '^error: controller MTBDD portfolio integrity mismatch$'

admitted_bytes=$(field "$admitted" artifact_bytes)
fallback_bytes=$(field "$fallback" artifact_bytes)
admitted_bound=$(field "$admitted" transition_evaluation_bound)
fallback_bound=$(field "$fallback" transition_evaluation_bound)
total_bytes=$((admitted_bytes + fallback_bytes))
total_bound=$((admitted_bound + fallback_bound))

printf '%s\n' \
  'schema_version,job,outcome,backend,reason,safe,unsafe,artifact_bytes,transition_bound,exit_code,result,status' \
  "1,public-washing-batch,verified,MTBDD,none,2,4,$admitted_bytes,$admitted_bound,0,answers-retained,accepted" \
  "1,direct-fallback-batch,verified,DIRECT_EXACT,none,1,1,$fallback_bytes,$fallback_bound,0,answers-retained,accepted" \
  '1,member-budget-control,refused,NONE,members,0,0,0,0,3,none,accepted' \
  '1,transition-budget-control,refused,NONE,transition-evaluations,0,0,0,0,3,none,accepted' \
  '1,malformed-policy-control,invalid,NONE,noncanonical-policy,0,0,0,0,2,none,accepted' \
  '1,corrupt-evidence-control,invalid,NONE,integrity,0,0,0,0,2,none,accepted' \
  "1,aggregate,summary,MIXED,none,3,5,$total_bytes,$total_bound,0,2-verified-2-refused-2-invalid,accepted" \
  >"$output"

echo "controller plant resource acceptance status=ACCEPTED jobs=6 verified=2 refused=2 invalid=2 safe=3 unsafe=5 output=$output"
