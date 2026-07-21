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
fixture=$repo/corpus/rtl/roalogic-plic-gateway
[[ -x "$binary" && -x "$yosys" && -x "$smtbmc" ]]
command -v z3 >/dev/null 2>&1 || { echo "Z3 is required" >&2; exit 2; }
[[ ! -e "$output" && ! -L "$output" ]] || {
  echo "refusing to overwrite $output" >&2
  exit 2
}

capabilities=$("$binary" btor2-search-v4-capabilities)
case $capabilities in
  "btor2_search_capability_version=1 search_certificate_version=4 min_inputs=1 max_inputs=8 input_width=1 min_constraints=1 "*\
" constraints=exact-all-frame-ordered valuation_order=input-node-ascending valuation_bit=i-maps-input-i terminal_valuation=distinct dead_end_layers=empty unsafe=admissible-trace safe=complete-admissible-layers work_accounting=all-valuations resource_refusal=no-answer unsupported=fail-closed") ;;
  *) echo "unexpected BTOR2 search v4 capability contract" >&2; exit 1 ;;
esac

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-plic-constrained-acceptance.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
model=$scratch/plic.btor2
second_model=$scratch/plic.second.btor2
"$repo/scripts/build-roalogic-plic-gateway-constrained-btor2-v1.sh" "$yosys" "$model" >/dev/null
"$repo/scripts/build-roalogic-plic-gateway-constrained-btor2-v1.sh" "$yosys" "$second_model" >/dev/null
cmp "$model" "$second_model"

inspection=$("$binary" inspect-btor2 "$model")
case $inspection in
  *"sha256=4938d7275d98444bbac5a8c341f702255199a75c47bfb8c4790959a1f5ed7cc3"*\
"nodes=122 inputs=5 states=7 bad=2 constraints=2 max_width=32"*) ;;
  *) echo "constrained PLIC structure disagrees with frozen baseline" >&2; exit 1 ;;
esac

printf '%s\n' \
  'schema_version,horizon,properties,answers,portfolio_status,route,logical_reachable_states,certificate_bytes,model_deterministic,evidence_deterministic,verified,maintained_oracle,status' \
  >"$scratch/result.csv"
for horizon in 0 4 8 16; do
  artifact=$scratch/h${horizon}.btor2-set-cert
  second_artifact=$scratch/h${horizon}.second.btor2-set-cert
  production=$("$binary" check-btor2-predicate-set "$model" 38,43 \
    "$horizon" "$artifact")
  second_production=$("$binary" check-btor2-predicate-set "$model" 38,43 \
    "$horizon" "$second_artifact")
  cmp "$artifact" "$second_artifact"
  case $production in
    *"certificate_version=3 portfolio_version=3"*\
"route=ordinary-exact reason=singleton-or-unsupported-exact"*\
"answers=38:SAFE:none,43:SAFE:none"*) ;;
    *) echo "unexpected constrained PLIC result at horizon $horizon" >&2; exit 1 ;;
  esac
  case $second_production in
    *"answers=38:SAFE:none,43:SAFE:none"*) ;;
    *) echo "unexpected repeated constrained PLIC result at horizon $horizon" >&2; exit 1 ;;
  esac
  verification=$("$binary" verify-btor2-predicate-set "$model" 38,43 \
    "$horizon" "$artifact")
  case $verification in
    *"status=VERIFIED"*"route=ordinary-exact"*\
"answers=38:SAFE:none,43:SAFE:none"*) ;;
    *) echo "constrained PLIC verification failed at horizon $horizon" >&2; exit 1 ;;
  esac
  states=$(printf '%s\n' "$production" | sed -n \
    's/.* logical_reachable_states=\([0-9][0-9]*\) .*/\1/p')
  bytes=$(wc -c <"$artifact" | tr -d ' ')
  printf '1,%s,38+43,38:SAFE:none+43:SAFE:none,accepted,ordinary-exact,%s,%s,true,true,true,true,validated\n' \
    "$horizon" "$states" "$bytes" >>"$scratch/result.csv"
done

direct=$scratch/direct.search-cert
"$binary" search-btor2 "$model" 38 4 "$direct" >/dev/null
grep -q '^search_certificate_version=4$' "$direct"
grep -q '^inputs=2,3,4,5,6$' "$direct"
grep -q '^constraint_count=2$' "$direct"
grep -q '^constraints=20,29$' "$direct"
"$binary" verify-btor2-search "$model" "$direct" >/dev/null

sed 's/^constraints=20,29$/constraints=29,20/' "$direct" >"$scratch/reordered.cert"
sed 's/^constraints=20,29$/constraints=20,20/' "$direct" >"$scratch/duplicated.cert"
sed 's/^constraint_count=2$/constraint_count=1/' "$direct" >"$scratch/wrong-count.cert"
sed '/^constraints=20,29$/d' "$direct" >"$scratch/omitted.cert"
sed 's/^search_certificate_version=4$/search_certificate_version=3/' "$direct" \
  >"$scratch/downgraded.cert"
sed '$d' "$direct" >"$scratch/truncated.cert"
for hostile in reordered duplicated wrong-count omitted downgraded truncated; do
  set +e
  "$binary" verify-btor2-search "$model" "$scratch/$hostile.cert" \
    >"$scratch/$hostile.out" 2>"$scratch/$hostile.err"
  hostile_exit=$?
  set -e
  [[ $hostile_exit -eq 2 && ! -s "$scratch/$hostile.out" ]]
done
set +e
"$binary" search-btor2 "$model" 38 4 "$direct" \
  >"$scratch/no-clobber.out" 2>"$scratch/no-clobber.err"
no_clobber_exit=$?
set -e
[[ $no_clobber_exit -eq 2 ]]

smt2=$scratch/plic.smt2
"$yosys" -Q -q -p "
  read_verilog -formal -sv $fixture/upstream/plic_gateway.sv $fixture/wrapper-constrained-predicate-set.sv;
  prep -top roalogic_plic_gateway_constrained_predicate_set; async2sync; dffunmap;
  setundef -zero -init; write_smt2 -wires $smt2
"
"$smtbmc" -s z3 -t 17 "$smt2" >"$scratch/smtbmc.log"
grep -q 'Status: PASSED' "$scratch/smtbmc.log"
grep -q 'Checking assertions in step 16' "$scratch/smtbmc.log"

if ! (set -C; cat "$scratch/result.csv" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "roalogic_plic_gateway_constrained_acceptance_v1=PASS accepted=4 hostile=7 oracle=Yosys-Z3 output=$output"
