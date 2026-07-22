#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 YOSYS YOSYS_SMTBMC GCC_BINARY OUTPUT.csv WORKDIR" >&2
  exit 2
fi

yosys=$1
smtbmc=$2
binary=$3
output=$4
workdir=$5
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/roalogic-plic-gateway
cohort=$fixture/revision-cohort
old_revision=e3483ddb06687799e2df81144659c3ec5eff3278
new_revision=2e8dc667f6ab69befaebdc30de7a9a53e925dbcc

[[ -x "$yosys" && -x "$smtbmc" && -x "$binary" ]] || {
  echo "Yosys, yosys-smtbmc, and GCC must be executable" >&2
  exit 2
}
[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
[[ ! -e "$output" && ! -L "$output" ]] || {
  echo "refusing to overwrite $output" >&2
  exit 2
}
command -v z3 >/dev/null || { echo "z3 is required" >&2; exit 2; }

old_model=$workdir/plic-old.btor2
new_model=$workdir/plic-new.btor2
old_model_second=$workdir/plic-old-second.btor2
new_model_second=$workdir/plic-new-second.btor2
"$repo/scripts/build-roalogic-plic-revision-component-v1.sh" \
  "$yosys" "$old_revision" "$old_model" >/dev/null
"$repo/scripts/build-roalogic-plic-revision-component-v1.sh" \
  "$yosys" "$new_revision" "$new_model" >/dev/null
"$repo/scripts/build-roalogic-plic-revision-component-v1.sh" \
  "$yosys" "$old_revision" "$old_model_second" >/dev/null
"$repo/scripts/build-roalogic-plic-revision-component-v1.sh" \
  "$yosys" "$new_revision" "$new_model_second" >/dev/null
cmp "$old_model" "$old_model_second"
cmp "$new_model" "$new_model_second"

result=$workdir/result.csv
"$repo/target/release/examples/roalogic_plic_revision_reuse" \
  "$cohort/monitor.btor2" "$old_model" "$new_model" "$cohort/interface.txt" \
  >"$result"
grep -q '^1,repeated-pending,UNSAFE,UNSAFE,2,2,.*accepted$' "$result"
grep -q '^1,impossible,SAFE,SAFE,none,none,.*accepted$' "$result"

proof=$workdir/old.revision-proof
"$binary" check-btor2-revision-portfolio \
  "$cohort/monitor.btor2" 7,8 "$old_model" 13 "$cohort/interface.txt" \
  2 left 7 "$proof" >"$workdir/check.log"
grep -q 'backend=revision-local reason=exact-local-relation-admitted result=UNSAFE' \
  "$workdir/check.log"
"$binary" verify-btor2-revision-portfolio \
  "$cohort/monitor.btor2" 7,8 "$old_model" 13 "$cohort/interface.txt" \
  2 left 7 "$proof" >"$workdir/verify.log"
grep -q 'status=VERIFIED.*result=UNSAFE.*bad_frame=2' "$workdir/verify.log"

next_proof=$workdir/new.revision-proof
"$binary" check-btor2-revision-retained-left \
  "$cohort/monitor.btor2" "$proof" "$new_model" 13 "$cohort/interface.txt" \
  2 left 7 "$next_proof" >"$workdir/retained-create.log"
grep -q 'status=CREATED.*result=UNSAFE.*produced_local_sections=1 production_reused_local_sections=1 changed_candidate_valuations=4096' \
  "$workdir/retained-create.log"
grep -q 'verified_local_sections=1 verification_reused_local_sections=1' \
  "$workdir/retained-create.log"
"$binary" verify-btor2-revision-portfolio \
  "$cohort/monitor.btor2" 7,8 "$new_model" 13 "$cohort/interface.txt" \
  2 left 7 "$next_proof" >"$workdir/retained-ordinary-verify.log"
grep -q 'status=VERIFIED.*result=UNSAFE.*bad_frame=2' \
  "$workdir/retained-ordinary-verify.log"

sed 's/wire=right,13,2/wire=left,13,2/' "$cohort/interface.txt" \
  >"$workdir/wrong-direction.txt"
sed '$d' "$proof" >"$workdir/truncated.revision-proof"
for hostile in source-drift direction-drift truncation no-clobber; do
  set +e
  case "$hostile" in
    source-drift)
      "$binary" verify-btor2-revision-portfolio \
        "$cohort/monitor.btor2" 7,8 "$new_model" 13 "$cohort/interface.txt" \
        2 left 7 "$proof" >"$workdir/$hostile.out" 2>"$workdir/$hostile.err"
      ;;
    direction-drift)
      "$binary" verify-btor2-revision-portfolio \
        "$cohort/monitor.btor2" 7,8 "$old_model" 13 "$workdir/wrong-direction.txt" \
        2 left 7 "$proof" >"$workdir/$hostile.out" 2>"$workdir/$hostile.err"
      ;;
    truncation)
      "$binary" verify-btor2-revision-portfolio \
        "$cohort/monitor.btor2" 7,8 "$old_model" 13 "$cohort/interface.txt" \
        2 left 7 "$workdir/truncated.revision-proof" \
        >"$workdir/$hostile.out" 2>"$workdir/$hostile.err"
      ;;
    no-clobber)
      "$binary" check-btor2-revision-portfolio \
        "$cohort/monitor.btor2" 7,8 "$old_model" 13 "$cohort/interface.txt" \
        2 left 7 "$proof" >"$workdir/$hostile.out" 2>"$workdir/$hostile.err"
      ;;
  esac
  hostile_exit=$?
  set -e
  [[ $hostile_exit -eq 2 && ! -s "$workdir/$hostile.out" ]] || {
    echo "hostile control was not rejected: $hostile" >&2
    exit 1
  }
done

for revision in "$old_revision" "$new_revision"; do
  source_file=$workdir/$revision.sv
  cp "$fixture/upstream/plic_gateway.sv" "$source_file"
  if [[ $revision == "$old_revision" ]]; then
    patch -R -s "$source_file" "$cohort/e3483ddb-to-2e8dc667.patch"
  fi
  for property in repeated impossible; do
    if [[ $property == repeated ]]; then
      parameter=1
      expected_status=FAILED
      expected_exit=1
    else
      parameter=0
      expected_status=PASSED
      expected_exit=0
    fi
    smt2=$workdir/$revision-$property.smt2
    "$yosys" -Q -q -p "
      read_verilog -formal -sv $source_file $cohort/oracle-wrapper.sv;
      chparam -set CHECK_REPEATED_PENDING $parameter roalogic_plic_gateway_revision_oracle;
      prep -top roalogic_plic_gateway_revision_oracle;
      async2sync; dffunmap; setundef -zero -init; write_smt2 -wires $smt2
    "
    set +e
    "$smtbmc" -s z3 -t 3 "$smt2" >"$workdir/$revision-$property.log" 2>&1
    oracle_exit=$?
    set -e
    [[ $oracle_exit -eq $expected_exit ]]
    grep -q "Status: $expected_status" "$workdir/$revision-$property.log"
    grep -q 'Checking assertions in step 2' "$workdir/$revision-$property.log"
  done
done

if ! (set -C; cat "$result" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "roalogic_plic_revision_reuse_v1=PASS revisions=2 properties=2 retained_sections=2 self_service=PASS hostile=4 oracle=Yosys-Z3 output=$output"
