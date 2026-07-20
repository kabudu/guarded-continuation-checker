#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
output=$2
test -x "$binary"
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-split-observability.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
manifest_a=corpus/rtl/wmcontroller/physical-plant-split-door-v1.txt
manifest_b=corpus/rtl/wmcontroller/physical-plant-split-lock-v1.txt
evidence=$scratch/controller.controller-evidence
results_a=$scratch/physical-a.plant-results
results_b=$scratch/physical-b.plant-results
policy=$scratch/permissive.policy

capabilities=$("$binary" controller-split-allocation-observability-cli-version)
test "$(printf '%s\n' "$capabilities" | wc -l | tr -d ' ')" -eq 3
printf '%s\n' "$capabilities" | tail -n 1 | grep -q \
  '^controller_split_allocation_observability_cli_version=1 base_observability_cli_version=1 allocator=system scope=policy-through-replay .* overflow=fail-closed timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed$'

"$binary" certify-controller-proof-evidence-v1 "$manifest_a" "$evidence" >/dev/null
"$binary" certify-bound-plant-results-v1 "$manifest_a" "$evidence" "$results_a" >/dev/null
"$binary" certify-bound-plant-results-v1 "$manifest_b" "$evidence" "$results_b" >/dev/null

controller_bytes=$(wc -c <"$evidence" | tr -d ' ')
plant_a_bytes=$(wc -c <"$results_a" | tr -d ' ')
plant_b_bytes=$(wc -c <"$results_b" | tr -d ' ')
total_plant_bytes=$((plant_a_bytes + plant_b_bytes))

cat >"$policy" <<EOF
controller_split_resource_policy_version=1
max_controller_artifact_bytes=$controller_bytes
max_unsat_proof_bytes=1048576
max_batches=2
max_plant_artifact_bytes_per_batch=16777216
max_members_per_batch=1
max_member_horizon=32
max_product_states_per_member=4096
max_transition_evaluations_per_batch=1000000000000
max_total_plant_artifact_bytes=33554432
max_total_members=2
max_total_transition_evaluations=2000000000000
status=complete
EOF

field() {
  printf '%s\n' "$1" | sed -n "s/.* $2=\\([^ ]*\\).*/\\1/p" | tail -n 1
}

check_phases() {
  observed=$1
  phase_sum=$(($(
    field "$observed" policy_and_input_micros
  ) + $(
    field "$observed" controller_admission_micros
  ) + $(
    field "$observed" complete_set_preflight_micros
  ) + $(
    field "$observed" semantic_replay_micros
  )))
  total=$(field "$observed" total_micros)
  test "$phase_sum" -le "$total"
  test "$(field "$observed" timing_calibration)" = none
  test "$(field "$observed" allocation_calls)" -gt 0
  test "$(field "$observed" allocated_bytes)" -gt 0
  test "$(field "$observed" overflow)" = none
}

run_observed() {
  "$binary" verify-bound-plant-result-set-with-resources-allocation-observed-v1 \
    "$evidence" "$policy" "$@"
}

two_batch=$(run_observed \
  "$manifest_a" "$results_a" \
  "$manifest_b" "$results_b")
single_a=$(run_observed "$manifest_a" "$results_a")
single_b=$(run_observed "$manifest_b" "$results_b")
check_phases "$two_batch"
check_phases "$single_a"
check_phases "$single_b"

check_counts() {
  observed=$1
  batches=$2
  members=$3
  manifest_loads=$4
  reads=$5
  rows=$6
  plant_bytes=$7
  test "$(field "$observed" controller_admissions)" -eq 1
  test "$(field "$observed" manifest_loads)" -eq "$manifest_loads"
  test "$(field "$observed" plant_artifact_reads)" -eq "$reads"
  test "$(field "$observed" resource_assessments)" -eq "$reads"
  test "$(field "$observed" batch_verifications)" -eq "$batches"
  test "$(field "$observed" buffered_result_rows)" -eq "$rows"
  test "$(field "$observed" prepared_batches)" -eq "$batches"
  test "$(field "$observed" prepared_members)" -eq "$members"
  test "$(field "$observed" controller_evidence_bytes)" -eq "$controller_bytes"
  test "$(field "$observed" total_plant_artifact_bytes)" -eq "$plant_bytes"
}

check_counts "$two_batch" 2 2 5 4 3 "$total_plant_bytes"
check_counts "$single_a" 1 1 3 2 2 "$plant_a_bytes"
check_counts "$single_b" 1 1 3 2 2 "$plant_b_bytes"

transition_two=$(field "$two_batch" total_transition_evaluation_bound)
transition_a=$(field "$single_a" total_transition_evaluation_bound)
transition_b=$(field "$single_b" total_transition_evaluation_bound)
test "$transition_two" -eq "$((transition_a + transition_b))"

tight=$scratch/tight-controller.policy
sed "s/max_controller_artifact_bytes=$controller_bytes/max_controller_artifact_bytes=$((controller_bytes - 1))/" \
  "$policy" >"$tight"
refusal_stdout=$scratch/refusal.stdout
refusal_stderr=$scratch/refusal.stderr
if "$binary" verify-bound-plant-result-set-with-resources-allocation-observed-v1 \
  "$evidence" "$tight" "$manifest_a" "$results_a" \
  >"$refusal_stdout" 2>"$refusal_stderr"; then
  echo "observed governed split verification unexpectedly succeeded" >&2
  exit 1
else
  refusal_exit=$?
fi
test "$refusal_exit" -eq 3
test ! -s "$refusal_stdout"
grep -q '^error: controller-split-resource refusal=controller-artifact-bytes result=none$' \
  "$refusal_stderr"

aggregate_controller_bytes=$((controller_bytes * 3))
aggregate_plant_bytes=$((total_plant_bytes * 2))
aggregate_transition_bound=$((transition_two * 2))

set -C
printf '%s\n' \
  'schema_version,job,outcome,batches,members,safe,unsafe,controller_admissions,manifest_loads,plant_artifact_reads,resource_assessments,batch_verifications,buffered_result_rows,prepared_batches,prepared_members,controller_evidence_bytes,total_plant_artifact_bytes,total_transition_bound,phase_contract,status' \
  "1,public-washing-two-batch,verified,2,2,0,2,1,5,4,4,2,3,2,2,$controller_bytes,$total_plant_bytes,$transition_two,v1-structural-only,accepted" \
  "1,public-washing-door,verified,1,1,0,1,1,3,2,2,1,2,1,1,$controller_bytes,$plant_a_bytes,$transition_a,v1-structural-only,accepted" \
  "1,public-washing-lock,verified,1,1,0,1,1,3,2,2,1,2,1,1,$controller_bytes,$plant_b_bytes,$transition_b,v1-structural-only,accepted" \
  '1,controller-budget-control,refused,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,no-partial-metrics,accepted' \
  "1,aggregate,summary,4,4,0,4,3,11,8,8,4,7,4,4,$aggregate_controller_bytes,$aggregate_plant_bytes,$aggregate_transition_bound,3-measured-1-refused,accepted" \
  >"$output"
set +C

echo "controller split observability acceptance status=ACCEPTED observed_contract_jobs=5 measured=3 refused=1 discovery=1 fixture_setup_jobs=3 batches=4 members=4 output=$output"
