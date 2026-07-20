#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 GCC_BINARY BTOR2TOOLS_BIN_DIR" >&2
  exit 2
fi

gcc_binary=$1
btor2tools_bin_dir=$2
model=examples/btor2/watchdog-counter-v1.btor2
witness=examples/btor2/watchdog-expiry-v1.witness
actuator=examples/btor2/actuator-position-v1.btor2
saturating=examples/btor2/saturating-timer-rejected-v1.btor2
motion=examples/btor2/motion-envelope-v1.btor2
servo=examples/btor2/servo-motion-envelope-v1.btor2
semi_implicit=examples/btor2/semi-implicit-motion-rejected-v1.btor2
braking=examples/btor2/braking-controller-v1.btor2
motor_stop=examples/btor2/motor-emergency-stop-v1.btor2
semi_implicit_braking=examples/btor2/semi-implicit-braking-rejected-v1.btor2
component_controller=examples/btor2/components/braking-controller-v1.btor2
component_plant=examples/btor2/components/motion-plant-v1.btor2
component_fast_plant=examples/btor2/components/fast-motion-plant-v1.btor2
component_motor_controller=examples/btor2/components/motor-stop-controller-v1.btor2
component_motor_plant=examples/btor2/components/motor-plant-v1.btor2
component_semi_implicit=examples/btor2/components/semi-implicit-motion-plant-v1.btor2
component_contract=examples/btor2/components/braking-motion-contract-v1.txt
component_admitted_batch=examples/btor2/components/braking-batch-admitted-v1.txt
component_mixed_batch=examples/btor2/components/braking-batch-mixed-v1.txt
opentitan_small=corpus/rtl/opentitan-aon-timer/generated/watchdog-small.btor2
opentitan_scale=corpus/rtl/opentitan-aon-timer/generated/watchdog-scale.btor2
certificate=${TMPDIR:-/tmp}/gcc-btor2-phase-$$.cert
actuator_witness=${TMPDIR:-/tmp}/gcc-btor2-actuator-$$.witness
saturating_witness=${TMPDIR:-/tmp}/gcc-btor2-saturating-$$.witness
motion_witness=${TMPDIR:-/tmp}/gcc-btor2-motion-$$.witness
servo_witness=${TMPDIR:-/tmp}/gcc-btor2-servo-$$.witness
semi_implicit_witness=${TMPDIR:-/tmp}/gcc-btor2-semi-implicit-$$.witness
braking_witness=${TMPDIR:-/tmp}/gcc-btor2-braking-$$.witness
semi_implicit_braking_witness=${TMPDIR:-/tmp}/gcc-btor2-semi-implicit-braking-$$.witness
motor_stop_witness=${TMPDIR:-/tmp}/gcc-btor2-motor-stop-$$.witness
trap 'rm -f "$certificate" "$actuator_witness" "$saturating_witness" "$motion_witness" "$servo_witness" "$semi_implicit_witness" "$braking_witness" "$semi_implicit_braking_witness" "$motor_stop_witness"' EXIT HUP INT TERM

write_zero_input_witness() {
  output=$1
  final_frame=$2
  symbol=$3
  {
    printf 'sat\nb0\n'
    frame=0
    while [ "$frame" -le "$final_frame" ]; do
      printf '#%s\n@%s\n0 0 %s@%s\n' "$frame" "$frame" "$symbol" "$frame"
      frame=$((frame + 1))
    done
    printf '.\n'
  } >"$output"
}

test -x "$gcc_binary"
test -x "$btor2tools_bin_dir/catbtor"
test -x "$btor2tools_bin_dir/btorsim"

inspection=$($gcc_binary inspect-btor2 "$model")
printf '%s\n' "$inspection"
printf '%s\n' "$inspection" | grep -q '^btor2-inspect status=VALID '
printf '%s\n' "$inspection" | grep -q ' core_version=1 '
printf '%s\n' "$inspection" | grep -q ' states=1 '
printf '%s\n' "$inspection" | grep -q ' bad=1 '
printf '%s\n' "$inspection" | grep -q ' word_semantics=preserved$'

"$btor2tools_bin_dir/catbtor" "$model" >/dev/null
"$btor2tools_bin_dir/catbtor" "$actuator" >/dev/null
"$btor2tools_bin_dir/catbtor" "$saturating" >/dev/null
"$btor2tools_bin_dir/catbtor" "$motion" >/dev/null
"$btor2tools_bin_dir/catbtor" "$servo" >/dev/null
"$btor2tools_bin_dir/catbtor" "$semi_implicit" >/dev/null
"$btor2tools_bin_dir/catbtor" "$braking" >/dev/null
"$btor2tools_bin_dir/catbtor" "$motor_stop" >/dev/null
"$btor2tools_bin_dir/catbtor" "$semi_implicit_braking" >/dev/null
"$btor2tools_bin_dir/catbtor" "$component_controller" >/dev/null
"$btor2tools_bin_dir/catbtor" "$component_plant" >/dev/null
"$btor2tools_bin_dir/catbtor" "$component_fast_plant" >/dev/null
"$btor2tools_bin_dir/catbtor" "$component_motor_controller" >/dev/null
"$btor2tools_bin_dir/catbtor" "$component_motor_plant" >/dev/null
"$btor2tools_bin_dir/catbtor" "$component_semi_implicit" >/dev/null
"$btor2tools_bin_dir/catbtor" "$opentitan_small" >/dev/null
"$btor2tools_bin_dir/catbtor" "$opentitan_scale" >/dev/null
"$btor2tools_bin_dir/btorsim" -c "$model" "$witness"
write_zero_input_witness "$actuator_witness" 201 home
write_zero_input_witness "$saturating_witness" 255 reset
"$btor2tools_bin_dir/btorsim" -c "$actuator" "$actuator_witness"
"$btor2tools_bin_dir/btorsim" -c "$saturating" "$saturating_witness"
write_zero_input_witness "$motion_witness" 201 brake
write_zero_input_witness "$servo_witness" 129 stop
write_zero_input_witness "$semi_implicit_witness" 4 stop
"$btor2tools_bin_dir/btorsim" -c "$motion" "$motion_witness"
"$btor2tools_bin_dir/btorsim" -c "$servo" "$servo_witness"
"$btor2tools_bin_dir/btorsim" -c "$semi_implicit" "$semi_implicit_witness"
write_zero_input_witness "$braking_witness" 256 reset
write_zero_input_witness "$semi_implicit_braking_witness" 128 reset
"$btor2tools_bin_dir/btorsim" -c "$braking" "$braking_witness"
"$btor2tools_bin_dir/btorsim" -c "$semi_implicit_braking" "$semi_implicit_braking_witness"
write_zero_input_witness "$motor_stop_witness" 160 reset
"$btor2tools_bin_dir/btorsim" -c "$motor_stop" "$motor_stop_witness"

"$gcc_binary" certify-btor2-counter-phase "$model" 13 \
  1:2,0:1000000003 "$certificate"
"$gcc_binary" verify-btor2-counter-phase "$model" "$certificate"
rm -f "$certificate"
"$gcc_binary" certify-btor2-counter-phase "$actuator" 13 0:201 "$certificate"
"$gcc_binary" verify-btor2-counter-phase "$actuator" "$certificate"
rm -f "$certificate"
if "$gcc_binary" certify-btor2-counter-phase "$saturating" 15 0:255 "$certificate"; then
  echo 'saturating near-neighbour unexpectedly admitted' >&2
  exit 1
fi
"$gcc_binary" certify-btor2-counter-trace "$saturating" 15 0:255 "$certificate"
grep -q '^replay_certificate_version=1$' "$certificate"
"$gcc_binary" verify-btor2-counter-trace "$saturating" "$certificate"
rm -f "$certificate"
"$gcc_binary" search-btor2 "$model" 13 2 "$certificate"
grep -q '^result=SAFE$' "$certificate"
"$gcc_binary" verify-btor2-search "$model" "$certificate"
rm -f "$certificate"
"$gcc_binary" search-btor2 "$model" 13 3 "$certificate"
grep -q '^result=UNSAFE$' "$certificate"
"$gcc_binary" verify-btor2-search "$model" "$certificate"
rm -f "$certificate"

check_bounded() {
  source=$1
  bad=$2
  horizon=$3
  expected=$4
  backend=$5
  "$gcc_binary" check-btor2-bounded "$source" "$bad" "$horizon" "$certificate"
  grep -q "^${backend}_certificate_version=1$" "$certificate"
  grep -q "^result=${expected}$" "$certificate"
  "$gcc_binary" verify-btor2-bounded "$source" "$certificate"
  rm -f "$certificate"
}

check_bounded "$model" 13 2 SAFE region
check_bounded "$model" 13 3 UNSAFE search
check_bounded "$actuator" 13 200 SAFE region
check_bounded "$actuator" 13 201 UNSAFE search
check_bounded "$saturating" 15 254 SAFE region
check_bounded "$saturating" 15 255 UNSAFE search
check_bounded "$motion" 21 200 SAFE motion
check_bounded "$motion" 21 201 UNSAFE search
check_bounded "$servo" 21 128 SAFE motion
check_bounded "$servo" 21 129 UNSAFE search
check_bounded "$semi_implicit" 21 3 SAFE search
check_bounded "$semi_implicit" 21 4 UNSAFE search
check_bounded "$braking" 31 255 SAFE braking
check_bounded "$braking" 31 256 UNSAFE search
check_bounded "$semi_implicit_braking" 31 127 SAFE search
check_bounded "$semi_implicit_braking" 31 128 UNSAFE search
check_bounded "$motor_stop" 31 159 SAFE braking
check_bounded "$motor_stop" 31 160 UNSAFE search

check_components() {
  controller=$1
  plant=$2
  horizon=$3
  expected=$4
  backend=$5
  "$gcc_binary" check-btor2-components \
    "$controller" "$plant" "$component_contract" "$horizon" "$certificate"
  grep -q '^component_certificate_version=1$' "$certificate"
  grep -q "^backend=${backend}$" "$certificate"
  grep -q "^result=${expected}$" "$certificate"
  "$gcc_binary" verify-btor2-components \
    "$controller" "$plant" "$component_contract" "$certificate"
  rm -f "$certificate"
}

check_components "$component_controller" "$component_plant" 255 SAFE phase-contract
check_components "$component_controller" "$component_plant" 256 UNSAFE composed-search
check_components "$component_controller" "$component_fast_plant" 127 SAFE phase-contract
check_components "$component_controller" "$component_fast_plant" 128 UNSAFE composed-search
check_components "$component_motor_controller" "$component_motor_plant" 159 SAFE phase-contract
check_components "$component_motor_controller" "$component_motor_plant" 160 UNSAFE composed-search
check_components "$component_controller" "$component_semi_implicit" 127 SAFE composed-search
check_components "$component_controller" "$component_semi_implicit" 128 UNSAFE composed-search

"$gcc_binary" check-btor2-component-batch \
  "$component_controller" "$component_admitted_batch" "$certificate"
grep -q '^reusable_component_batch_version=1$' "$certificate"
"$gcc_binary" verify-btor2-component-batch \
  "$component_controller" "$component_admitted_batch" "$certificate"
rm -f "$certificate"
"$gcc_binary" check-btor2-component-batch \
  "$component_controller" "$component_mixed_batch" "$certificate"
grep -q '^component_batch_portfolio_version=1$' "$certificate"
grep -q '^route=ordinary$' "$certificate"
"$gcc_binary" verify-btor2-component-batch \
  "$component_controller" "$component_mixed_batch" "$certificate"
rm -f "$certificate"

echo 'btor2tools_baseline=PASS'
