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
certificate=${TMPDIR:-/tmp}/gcc-btor2-phase-$$.cert
trap 'rm -f "$certificate"' EXIT HUP INT TERM

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
"$btor2tools_bin_dir/btorsim" -c "$model" "$witness"

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

echo 'btor2tools_baseline=PASS'
