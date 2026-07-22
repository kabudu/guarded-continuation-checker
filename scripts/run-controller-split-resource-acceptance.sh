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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-split-resource.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
manifest_a=corpus/rtl/wmcontroller/physical-plant-split-door-v1.txt
manifest_b=corpus/rtl/wmcontroller/physical-plant-split-lock-v1.txt
evidence=$scratch/controller.controller-evidence
results_a=$scratch/physical-a.plant-results
results_b=$scratch/physical-b.plant-results
policy=$scratch/permissive.policy

"$binary" controller-split-resource-cli-version | grep -q \
  '^controller_split_resource_cli_version=1 .* admission=once .* accounting=conservative-static-per-batch-and-total .* result_on_refusal=none refusal_schema=split-reason-v1 unsupported=fail-closed$'
"$binary" certify-controller-proof-evidence-v1 "$manifest_a" "$evidence" >/dev/null
"$binary" certify-bound-plant-results-v1 "$manifest_a" "$evidence" "$results_a" >/dev/null
"$binary" certify-bound-plant-results-v1 "$manifest_b" "$evidence" "$results_b" >/dev/null

controller_bytes=$(wc -c <"$evidence" | tr -d ' ')
plant_a_bytes=$(wc -c <"$results_a" | tr -d ' ')
plant_b_bytes=$(wc -c <"$results_b" | tr -d ' ')
total_plant_bytes=$((plant_a_bytes + plant_b_bytes))

write_policy() {
  destination=$1
  max_controller_bytes=$2
  max_batches=$3
  max_total_members=$4
  cat >"$destination" <<EOF
controller_split_resource_policy_version=1
max_controller_artifact_bytes=$max_controller_bytes
max_unsat_proof_bytes=1048576
max_batches=$max_batches
max_plant_artifact_bytes_per_batch=16777216
max_members_per_batch=1
max_member_horizon=32
max_product_states_per_member=4096
max_transition_evaluations_per_batch=1000000000000
max_total_plant_artifact_bytes=33554432
max_total_members=$max_total_members
max_total_transition_evaluations=2000000000000
status=complete
EOF
}

write_policy "$policy" "$controller_bytes" 2 2
verified=$("$binary" verify-bound-plant-result-set-with-resources-v1 \
  "$evidence" "$policy" \
  "$manifest_a" "$results_a" \
  "$manifest_b" "$results_b")

field() {
  printf '%s\n' "$1" | tail -n 1 | sed -n "s/.* $2=\\([^ ]*\\).*/\\1/p"
}

test "$(field "$verified" controller_admissions)" = 1
test "$(field "$verified" batches)" = 2
test "$(field "$verified" members)" = 2
test "$(field "$verified" safe)" = 0
test "$(field "$verified" unsafe)" = 2
test "$(field "$verified" controller_evidence_bytes)" = "$controller_bytes"
test "$(field "$verified" total_plant_artifact_bytes)" = "$total_plant_bytes"
transition_bound=$(field "$verified" total_transition_evaluation_bound)
test "$transition_bound" -gt 0

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

controller_sha256=$(sha256_file "$evidence")
plant_a_sha256=$(sha256_file "$results_a")
plant_b_sha256=$(sha256_file "$results_b")
manifest_a_sha256=$(sha256_file "$manifest_a")
manifest_b_sha256=$(sha256_file "$manifest_b")

run_failure() {
  failure_policy=$1
  expected_exit=$2
  expected=$3
  evidence_path=${4:-$evidence}
  result_path=${5:-$results_a}
  stdout=$scratch/failure.stdout
  stderr=$scratch/failure.stderr
  if "$binary" verify-bound-plant-result-set-with-resources-v1 \
    "$evidence_path" "$failure_policy" \
    "$manifest_a" "$result_path" \
    "$manifest_b" "$results_b" >"$stdout" 2>"$stderr"; then
    echo "governed split verification unexpectedly succeeded" >&2
    exit 1
  else
    actual_exit=$?
  fi
  test "$actual_exit" -eq "$expected_exit"
  test ! -s "$stdout"
  grep -q "$expected" "$stderr"
}

tight_controller=$scratch/tight-controller.policy
write_policy "$tight_controller" "$((controller_bytes - 1))" 2 2
run_failure "$tight_controller" 3 \
  '^error: controller-split-resource refusal=controller-artifact-bytes result=none$'

tight_batches=$scratch/tight-batches.policy
write_policy "$tight_batches" "$controller_bytes" 1 1
sed 's/max_total_transition_evaluations=2000000000000/max_total_transition_evaluations=1000000000000/; s/max_total_plant_artifact_bytes=33554432/max_total_plant_artifact_bytes=16777216/' \
  "$tight_batches" >"$scratch/tight-batches-adjusted.policy"
run_failure "$scratch/tight-batches-adjusted.policy" 3 \
  '^error: controller-split-resource refusal=batches result=none$'

tight_members=$scratch/tight-members.policy
write_policy "$tight_members" "$controller_bytes" 2 1
run_failure "$tight_members" 3 \
  '^error: controller-split-resource refusal=total-members result=none$'

malformed=$scratch/malformed.policy
sed 's/max_batches=2/max_batches=02/' "$policy" >"$malformed"
run_failure "$malformed" 2 \
  '^error: controller split resource policy batches is noncanonical$'

corrupt=$scratch/corrupt.plant-results
cp "$results_a" "$corrupt"
printf '\001' | dd of="$corrupt" bs=1 seek=40 conv=notrunc 2>/dev/null
run_failure "$policy" 2 '^error: bound plant result artifact integrity mismatch$' \
  "$evidence" "$corrupt"

printf '%s\n' \
  'schema_version,job,outcome,batches,members,safe,unsafe,controller_evidence_bytes,total_plant_artifact_bytes,total_transition_bound,controller_evidence_sha256,plant_a_sha256,plant_b_sha256,manifest_a_sha256,manifest_b_sha256,exit_code,result,status' \
  "1,public-washing-two-batch,verified,2,2,0,2,$controller_bytes,$total_plant_bytes,$transition_bound,$controller_sha256,$plant_a_sha256,$plant_b_sha256,$manifest_a_sha256,$manifest_b_sha256,0,answers-retained,accepted" \
  '1,controller-budget-control,refused,0,0,0,0,0,0,0,none,none,none,none,none,3,none,accepted' \
  '1,batch-budget-control,refused,0,0,0,0,0,0,0,none,none,none,none,none,3,none,accepted' \
  '1,total-member-control,refused,0,0,0,0,0,0,0,none,none,none,none,none,3,none,accepted' \
  '1,malformed-policy-control,invalid,0,0,0,0,0,0,0,none,none,none,none,none,2,none,accepted' \
  '1,corrupt-evidence-control,invalid,0,0,0,0,0,0,0,none,none,none,none,none,2,none,accepted' \
  "1,aggregate,summary,2,2,0,2,$controller_bytes,$total_plant_bytes,$transition_bound,$controller_sha256,$plant_a_sha256,$plant_b_sha256,$manifest_a_sha256,$manifest_b_sha256,0,1-verified-3-refused-2-invalid,accepted" \
  >"$output"

echo "controller split resource acceptance status=ACCEPTED jobs=6 verified=1 refused=3 invalid=2 safe=0 unsafe=2 output=$output"
