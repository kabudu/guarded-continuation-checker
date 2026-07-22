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

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-proof-portfolio.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
proof_manifest=corpus/rtl/wmcontroller/physical-plant-batch-v1.txt
direct_manifest=corpus/proof-mtbdd-portfolio/direct-manifest-v1.txt
proof_artifact=$scratch/public.proof-mtbdd-portfolio
direct_artifact=$scratch/direct.proof-mtbdd-portfolio
policy=$scratch/permissive.policy

cat >"$policy" <<'EOF'
controller_proof_mtbdd_resource_policy_version=1
max_artifact_bytes=16777216
max_equivalence_artifact_bytes=2097152
max_unsat_proof_bytes=1048576
max_members=64
max_member_horizon=1024
max_product_states_per_member=4096
max_transition_evaluations=18446744073709551615
status=complete
EOF

"$binary" controller-proof-mtbdd-portfolio-cli-version | grep -q \
  '^controller_proof_mtbdd_portfolio_cli_version=1 .* routing=static fallback=exact proof_failure=fail-closed .* result_on_refusal=none refusal_schema=proof-reason-v1 unsupported=fail-closed$'
"$binary" certify-controller-proof-mtbdd-portfolio \
  "$proof_manifest" "$proof_artifact" >/dev/null
"$binary" certify-controller-proof-mtbdd-portfolio \
  "$direct_manifest" "$direct_artifact" >/dev/null

verify_governed() {
  if [ "$(uname -s)" = Linux ]; then
    command -v prlimit >/dev/null
    prlimit --as=67108864 -- \
      "$binary" verify-controller-proof-mtbdd-portfolio-resources "$@"
  else
    "$binary" verify-controller-proof-mtbdd-portfolio-resources "$@"
  fi
}

verify_attested() {
  if [ "$(uname -s)" = Linux ]; then
    command -v prlimit >/dev/null
    prlimit --as=67108864 -- \
      "$binary" verify-controller-proof-mtbdd-portfolio-resources-attested "$@"
  else
    "$binary" verify-controller-proof-mtbdd-portfolio-resources-attested "$@"
  fi
}

proof=$(verify_governed "$proof_manifest" "$policy" "$proof_artifact")
direct=$(verify_governed "$direct_manifest" "$policy" "$direct_artifact")
attested=$(verify_attested \
  "$proof_manifest" "$policy" "$proof_artifact" \
  corpus/rtl/wmcontroller/source-model-provenance-v1.txt \
  results/public-washing-source-model-attestation-v1.csv)

field() {
  printf '%s\n' "$1" | sed -n "1s/.* $2=\\([^ ]*\\).*/\\1/p"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

test "$(field "$proof" backend)" = PROOF_MTBDD
test "$(field "$proof" reason)" = MTBDD_ADMITTED
test "$(field "$proof" safe)" = 2
test "$(field "$proof" unsafe)" = 4
test "$(field "$proof" assignments_checked)" = 0
test "$(field "$attested" provenance)" = BOUND
test "$(field "$attested" source_model_members)" = 2
test "$(field "$attested" safe)" = 2
test "$(field "$attested" unsafe)" = 4
test "$(field "$direct" backend)" = DIRECT_EXACT
test "$(field "$direct" reason)" = BOUNDARY_LIMIT
test "$(field "$direct" safe)" = 1
test "$(field "$direct" unsafe)" = 1
test "$(field "$direct" equivalence_artifact_bytes)" = 0
test "$(field "$direct" unsat_proof_bytes)" = 0

proof_limit=$scratch/proof-limit.policy
proof_bytes=$(field "$proof" unsat_proof_bytes)
test "$proof_bytes" -gt 1
sed "s/max_unsat_proof_bytes=1048576/max_unsat_proof_bytes=$((proof_bytes - 1))/" \
  "$policy" >"$proof_limit"
transition_limit=$scratch/transition-limit.policy
sed 's/max_transition_evaluations=18446744073709551615/max_transition_evaluations=1/' \
  "$policy" >"$transition_limit"
malformed=$scratch/malformed.policy
sed 's/max_members=64/max_members=064/' "$policy" >"$malformed"
corrupt=$scratch/corrupt.proof-mtbdd-portfolio
cp "$proof_artifact" "$corrupt"
printf '\001' | dd of="$corrupt" bs=1 seek=40 conv=notrunc 2>/dev/null

run_failure() {
  manifest=$1
  selected_policy=$2
  artifact=$3
  expected_exit=$4
  expected=$5
  if failure=$(verify_governed "$manifest" "$selected_policy" "$artifact" 2>&1); then
    echo "governed proof portfolio verification unexpectedly succeeded" >&2
    exit 1
  else
    actual_exit=$?
  fi
  test "$actual_exit" -eq "$expected_exit"
  printf '%s\n' "$failure" | grep -q "$expected"
}

run_failure "$proof_manifest" "$proof_limit" "$proof_artifact" 3 \
  '^error: controller-proof-mtbdd-resource refusal=unsat-proof-bytes result=none$'
run_failure "$direct_manifest" "$transition_limit" "$direct_artifact" 3 \
  '^error: controller-proof-mtbdd-resource refusal=transition-evaluations result=none$'
run_failure "$proof_manifest" "$malformed" "$proof_artifact" 2 \
  '^error: controller proof MTBDD resource policy members is noncanonical$'
run_failure "$proof_manifest" "$policy" "$corrupt" 2 \
  '^error: proof-carrying controller MTBDD portfolio integrity mismatch$'

proof_artifact_bytes=$(field "$proof" artifact_bytes)
direct_artifact_bytes=$(field "$direct" artifact_bytes)
proof_sha256=$(sha256_file "$proof_artifact")
direct_sha256=$(sha256_file "$direct_artifact")
proof_transition_bound=$(field "$proof" transition_evaluation_bound)
direct_transition_bound=$(field "$direct" transition_evaluation_bound)
equivalence_bytes=$(field "$proof" equivalence_artifact_bytes)
total_artifact_bytes=$((proof_artifact_bytes + direct_artifact_bytes))
total_transition_bound=$((proof_transition_bound + direct_transition_bound))

printf '%s\n' \
  'schema_version,job,outcome,backend,reason,safe,unsafe,artifact_bytes,artifact_sha256,equivalence_bytes,unsat_proof_bytes,transition_bound,assignments_checked,exit_code,result,status' \
  "1,public-washing-batch,verified,PROOF_MTBDD,MTBDD_ADMITTED,2,4,$proof_artifact_bytes,$proof_sha256,$equivalence_bytes,$proof_bytes,$proof_transition_bound,0,0,answers-retained,accepted" \
  "1,direct-fallback-batch,verified,DIRECT_EXACT,BOUNDARY_LIMIT,1,1,$direct_artifact_bytes,$direct_sha256,0,0,$direct_transition_bound,0,0,answers-retained,accepted" \
  '1,proof-budget-control,refused,NONE,unsat-proof-bytes,0,0,0,none,0,0,0,0,3,none,accepted' \
  '1,transition-budget-control,refused,NONE,transition-evaluations,0,0,0,none,0,0,0,0,3,none,accepted' \
  '1,malformed-policy-control,invalid,NONE,noncanonical-policy,0,0,0,none,0,0,0,0,2,none,accepted' \
  '1,corrupt-evidence-control,invalid,NONE,integrity,0,0,0,none,0,0,0,0,2,none,accepted' \
  "1,aggregate,summary,MIXED,none,3,5,$total_artifact_bytes,see-member-rows,$equivalence_bytes,$proof_bytes,$total_transition_bound,0,0,2-verified-2-refused-2-invalid,accepted" \
  >"$output"

echo "governed proof portfolio acceptance status=ACCEPTED jobs=6 verified=2 refused=2 invalid=2 safe=3 unsafe=5 output=$output"
