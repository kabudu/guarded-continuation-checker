#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 GCC_BINARY YOSYS_BINARY OUTPUT.csv" >&2
  exit 2
fi

binary=$1
yosys=$2
output=$3
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
[[ -x "$binary" && -x "$yosys" ]]
[[ ! -e "$output" && ! -L "$output" ]] || {
  echo "refusing to overwrite $output" >&2
  exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-caliptra-word-probe.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
first=$scratch/first.btor2
second=$scratch/second.btor2
"$repo/scripts/build-caliptra-wdt-btor2-v1.sh" "$yosys" "$first" >/dev/null
"$repo/scripts/build-caliptra-wdt-btor2-v1.sh" "$yosys" "$second" >/dev/null
cmp "$first" "$second"

inspection=$("$binary" inspect-btor2 "$first")
case $inspection in
  *"status=VALID"*\
"sha256=07285dcc19a7b35ff7e8439baf6b343f9d39f67e6efba0fbb39168948410a0cb"*\
"nodes=34 inputs=1 states=2 bad=3 constraints=0 max_width=32"*) ;;
  *) echo "Caliptra BTOR2 inspection disagrees with frozen structure" >&2; exit 1 ;;
esac

printf '%s\n' \
  'schema_version,horizon,properties,portfolio_status,reason,model_deterministic,status' \
  >"$scratch/result.csv"
for horizon in 2 3 5 1000000000; do
  artifact=$scratch/h${horizon}.btor2-set-cert
  log=$scratch/h${horizon}.log
  set +e
  "$binary" check-btor2-predicate-set "$first" 23,26,30 "$horizon" \
    "$artifact" >"$log" 2>&1
  status=$?
  set -e
  [[ $status -eq 2 && ! -e "$artifact" ]]
  if [[ $horizon -eq 1000000000 ]]; then
    reason=search-horizon-limit
    grep -q 'explicit search fallback error: search horizon exceeds limit' "$log"
  else
    reason=input-dependent-bad-property
    grep -q 'explicit search fallback error: bounded search requires a state-only bad property' "$log"
  fi
  printf '1,%s,23+26+30,refused,%s,true,retained-negative\n' \
    "$horizon" "$reason" >>"$scratch/result.csv"
done

if ! (set -C; cat "$scratch/result.csv" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "caliptra_wdt_word_probe=RETAINED_NEGATIVE rows=4 output=$output"
