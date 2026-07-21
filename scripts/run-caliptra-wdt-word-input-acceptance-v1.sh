#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 GCC_BINARY YOSYS_BINARY YOSYS_SMTBMC OUTPUT.csv" >&2
  exit 2
fi

binary=$1
yosys=$2
smtbmc=$3
output=$4
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/caliptra-wdt
[[ -x "$binary" && -x "$yosys" && -x "$smtbmc" ]]
command -v z3 >/dev/null 2>&1 || { echo "Z3 is required" >&2; exit 2; }
[[ ! -e "$output" && ! -L "$output" ]] || {
  echo "refusing to overwrite $output" >&2
  exit 2
}

capabilities=$("$binary" btor2-search-v5-capabilities)
case $capabilities in
  "btor2_search_capability_version=1 search_certificate_version=5 min_inputs=1 max_inputs=8 max_input_width=8 max_total_input_bits=8 "*\
" constraints=exact-all-frame-ordered-or-none valuation_order=input-node-ascending-then-lsb-first terminal_valuation=distinct dead_end_layers=empty-with-constraints unsafe=admissible-trace safe=complete-admissible-layers work_accounting=all-valuations resource_refusal=no-answer unsupported=fail-closed") ;;
  *) echo "unexpected BTOR2 search v5 capability contract" >&2; exit 1 ;;
esac

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-caliptra-word-input-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
model=$scratch/caliptra.btor2
second_model=$scratch/caliptra.second.btor2
"$repo/scripts/build-caliptra-wdt-word-input-btor2-v1.sh" "$yosys" "$model" >/dev/null
"$repo/scripts/build-caliptra-wdt-word-input-btor2-v1.sh" "$yosys" "$second_model" >/dev/null
cmp "$model" "$second_model"

inspection=$("$binary" inspect-btor2 "$model")
case $inspection in
  *"sha256=b6e0b1db627d4daf3d03f617fd08f807b3b49b4f62b599843d65369047cc34ad"*\
"nodes=22 inputs=2 states=1 bad=1 constraints=1 max_width=32"*) ;;
  *) echo "Caliptra word-input structure disagrees with frozen baseline" >&2; exit 1 ;;
esac

printf '%s\n' \
  'schema_version,horizon,property,expected_answer,actual_answer,bad_frame,certificate_version,input_widths,total_input_bits,constraints,certificate_bytes,model_deterministic,evidence_deterministic,verified,maintained_oracle,status' \
  >"$scratch/result.csv"
for horizon in 0 1 4; do
  if [[ $horizon -eq 0 ]]; then
    expected_answer=SAFE
    expected_frame=none
  else
    expected_answer=UNSAFE
    expected_frame=1
  fi
  artifact=$scratch/h${horizon}.search-cert
  second_artifact=$scratch/h${horizon}.second.search-cert
  production=$("$binary" search-btor2 "$model" 24 "$horizon" "$artifact")
  second_production=$("$binary" search-btor2 "$model" 24 "$horizon" "$second_artifact")
  cmp "$artifact" "$second_artifact"
  case $production in
    *"version=5 result=$expected_answer horizon=$horizon bad_frame=$expected_frame"*) ;;
    *) echo "unexpected Caliptra word-input answer at horizon $horizon" >&2; exit 1 ;;
  esac
  case $second_production in
    *"version=5 result=$expected_answer horizon=$horizon bad_frame=$expected_frame"*) ;;
    *) echo "unexpected repeated Caliptra answer at horizon $horizon" >&2; exit 1 ;;
  esac
  grep -q '^inputs=2,4$' "$artifact"
  grep -q '^input_widths=2,1$' "$artifact"
  grep -q '^total_input_bits=3$' "$artifact"
  grep -q '^constraints=20$' "$artifact"
  verification=$("$binary" verify-btor2-search "$model" "$artifact")
  case $verification in
    *"status=VERIFIED version=5 result=$expected_answer horizon=$horizon bad_frame=$expected_frame"*) ;;
    *) echo "Caliptra word-input verification failed at horizon $horizon" >&2; exit 1 ;;
  esac
  printf '1,%s,24,%s,%s,%s,5,2+1,3,20,%s,true,true,true,true,validated\n' \
    "$horizon" "$expected_answer" "$expected_answer" "$expected_frame" \
    "$(wc -c <"$artifact" | tr -d ' ')" >>"$scratch/result.csv"
done

direct=$scratch/h1.search-cert
sed 's/^input_widths=2,1$/input_widths=1,2/' "$direct" >"$scratch/width-drift.cert"
sed 's/^total_input_bits=3$/total_input_bits=2/' "$direct" >"$scratch/total-width.cert"
sed 's/^inputs=2,4$/inputs=4,2/' "$direct" >"$scratch/input-reorder.cert"
sed 's/^constraints=20$/constraints=19/' "$direct" >"$scratch/constraint-rebind.cert"
sed 's/^terminal_valuation=1$/terminal_valuation=8/' "$direct" >"$scratch/high-bit.cert"
sed 's/^search_certificate_version=5$/search_certificate_version=4/' "$direct" \
  >"$scratch/downgraded.cert"
sed '$d' "$direct" >"$scratch/truncated.cert"
for hostile in width-drift total-width input-reorder constraint-rebind high-bit downgraded truncated; do
  set +e
  "$binary" verify-btor2-search "$model" "$scratch/$hostile.cert" \
    >"$scratch/$hostile.out" 2>"$scratch/$hostile.err"
  hostile_exit=$?
  set -e
  [[ $hostile_exit -eq 2 && ! -s "$scratch/$hostile.out" ]]
done
set +e
"$binary" search-btor2 "$model" 24 1 "$direct" \
  >"$scratch/no-clobber.out" 2>"$scratch/no-clobber.err"
no_clobber_exit=$?
set -e
[[ $no_clobber_exit -eq 2 ]]

smt2=$scratch/caliptra.smt2
"$yosys" -Q -q -p "
  read_verilog -formal -sv $fixture/upstream/wdt.sv $fixture/wrapper-word-input.sv;
  prep -top caliptra_wdt_word_input; async2sync; dffunmap;
  setundef -zero -init; write_smt2 -wires $smt2
"
"$smtbmc" -s z3 -t 1 "$smt2" >"$scratch/safe.log"
grep -q 'Checking assertions in step 0' "$scratch/safe.log"
grep -q 'Status: PASSED' "$scratch/safe.log"
set +e
"$smtbmc" -s z3 -t 2 "$smt2" >"$scratch/unsafe.log"
unsafe_oracle_exit=$?
set -e
[[ $unsafe_oracle_exit -eq 1 ]]
grep -q 'Checking assertions in step 1' "$scratch/unsafe.log"
grep -q 'Status: FAILED' "$scratch/unsafe.log"

if ! (set -C; cat "$scratch/result.csv" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "caliptra_wdt_word_input_acceptance_v1=PASS accepted=3 hostile=7 oracle=Yosys-Z3 output=$output"
