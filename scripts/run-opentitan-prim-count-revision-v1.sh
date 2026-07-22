#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 YOSYS YOSYS_SMTBMC GCC_BINARY OUTPUT.csv MANIFEST.txt WORKDIR" >&2
  exit 2
fi

yosys=$1
smtbmc=$2
binary=$3
output=$4
manifest=$5
workdir=$6
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-prim-count-revision

[[ -x "$yosys" && -x "$smtbmc" && -x "$binary" ]] || {
  echo "Yosys, yosys-smtbmc, and GCC must be executable" >&2
  exit 2
}
[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
for path in "$output" "$manifest"; do
  [[ ! -e "$path" && ! -L "$path" ]] || {
    echo "refusing to overwrite $path" >&2
    exit 2
  }
done
command -v z3 >/dev/null || { echo "z3 is required" >&2; exit 2; }

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}
[[ $(sha256_file "$fixture/interface.txt") == \
  27fcab011379dde9971b10ced802136c34fb667cb80ea5808ba06876008b8e3d ]] || {
  echo "pinned interface digest mismatch" >&2
  exit 2
}
[[ $(sha256_file "$fixture/oracle-wrapper.sv") == \
  22858ac5ca7bd506b3a3853a3fe1dce74cde1999e5a78a1c9b50e7d8b0057e4f ]] || {
  echo "pinned oracle digest mismatch" >&2
  exit 2
}

environment=$workdir/environment.btor2
before=$workdir/before.btor2
after=$workdir/after.btor2
"$repo/scripts/build-opentitan-prim-count-revision-v1.sh" \
  "$yosys" "$environment" "$before" "$after" >/dev/null

before_proof=$workdir/before.revision-proof
after_proof=$workdir/after.revision-proof
"$binary" check-btor2-revision-portfolio \
  "$environment" 2,3,4,9,12 "$before" 100 "$fixture/interface.txt" \
  2 left 2 "$before_proof" >"$workdir/before.log"
grep -q 'result=SAFE.*bad_frame=none' "$workdir/before.log"

"$binary" check-btor2-revision-retained-left \
  "$environment" "$before_proof" "$after" 100 "$fixture/interface.txt" \
  2 left 2 "$after_proof" >"$workdir/after.log"
grep -q 'result=UNSAFE.*bad_frame=0' "$workdir/after.log"
grep -q 'produced_local_sections=1 production_reused_local_sections=1' "$workdir/after.log"
grep -q 'verified_local_sections=1 verification_reused_local_sections=1' "$workdir/after.log"

"$binary" verify-btor2-revision-portfolio \
  "$environment" 2,3,4,9,12 "$before" 100 "$fixture/interface.txt" \
  2 left 2 "$before_proof" >"$workdir/verify-before.log"
grep -q 'status=VERIFIED.*result=SAFE' "$workdir/verify-before.log"
"$binary" verify-btor2-revision-retained-left \
  "$environment" "$before_proof" "$after" "$fixture/interface.txt" \
  2 left 2 "$after_proof" >"$workdir/verify-after.log"
grep -q 'status=VERIFIED.*result=UNSAFE.*reused_local_sections=1' "$workdir/verify-after.log"

for revision in before after; do
  source=$fixture/prim-count-$revision-specialized.sv
  smt2=$workdir/$revision.smt2
  "$yosys" -Q -q -p "
    read_verilog -formal -sv -DPRIM_COUNT_MODULE=opentitan_prim_count_$revision \
      $source $fixture/oracle-wrapper.sv;
    prep -top opentitan_prim_count_revision_oracle;
    async2sync; dffunmap; setundef -zero -init; write_smt2 -wires $smt2
  "
  set +e
  "$smtbmc" -s z3 -t 2 "$smt2" >"$workdir/$revision-oracle.log" 2>&1
  oracle_exit=$?
  set -e
  if [[ $revision == before ]]; then
    [[ $oracle_exit -eq 0 ]]
    grep -q 'Status: PASSED' "$workdir/$revision-oracle.log"
  else
    [[ $oracle_exit -eq 1 ]]
    grep -q 'Status: FAILED' "$workdir/$revision-oracle.log"
  fi
done

printf '%s\n' \
  'schema_version,revision,result,bad_frame,retained_environment,independent_oracle,status' \
  '1,34157c7afb84a7be7b1b1250d673f9fa8a3c18ce,SAFE,none,false,Yosys-Z3,accepted' \
  '1,369cffc85db0e6d5a667676a6f89987b94210e70,UNSAFE,0,true,Yosys-Z3,accepted' \
  >"$workdir/result.csv"
if ! (set -C; cp "$workdir/result.csv" "$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

sha256_tool=$(command -v sha256sum || true)
if [[ -n $sha256_tool ]]; then
  "$sha256_tool" "$environment" "$before" "$after" "$before_proof" "$after_proof" \
    | sed "s|$workdir/||" >"$workdir/manifest.txt"
else
  shasum -a 256 "$environment" "$before" "$after" "$before_proof" "$after_proof" \
    | sed "s|$workdir/||" >"$workdir/manifest.txt"
fi
if ! (set -C; cp "$workdir/manifest.txt" "$manifest") 2>/dev/null; then
  echo "refusing to overwrite $manifest" >&2
  exit 2
fi

echo "opentitan_prim_count_revision_v1=PASS semantic_change=true retained_environment=true oracle=Yosys-Z3 output=$output"
