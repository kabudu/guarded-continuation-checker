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
"$btor2tools_bin_dir/btorsim" -c "$model" "$witness"

echo 'btor2tools_baseline=PASS'
