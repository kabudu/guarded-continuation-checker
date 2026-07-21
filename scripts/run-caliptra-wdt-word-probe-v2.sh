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
  'schema_version,horizon,properties,expected_answers,actual_answers,portfolio_status,route,certificate_bytes,model_deterministic,evidence_deterministic,verified,status' \
  >"$scratch/result.csv"
for horizon in 2 3 5; do
  case $horizon in
    2) expected='23:SAFE:none+26:SAFE:none+30:SAFE:none' ;;
    3) expected='23:SAFE:none+26:SAFE:none+30:UNSAFE:3' ;;
    5) expected='23:UNSAFE:5+26:UNSAFE:5+30:UNSAFE:3' ;;
  esac
  artifact=$scratch/h${horizon}.btor2-set-cert
  second_artifact=$scratch/h${horizon}.second.btor2-set-cert
  production=$("$binary" check-btor2-predicate-set "$first" 23,26,30 \
    "$horizon" "$artifact")
  second_production=$("$binary" check-btor2-predicate-set "$first" 23,26,30 \
    "$horizon" "$second_artifact")
  cmp "$artifact" "$second_artifact"
  case $production in
    *"certificate_version=3 portfolio_version=3"*\
"route=ordinary-exact reason=singleton-or-unsupported-exact"*) ;;
    *) echo "unexpected Caliptra predicate-set route at horizon $horizon" >&2; exit 1 ;;
  esac
  case $second_production in
    *"certificate_version=3 portfolio_version=3"*\
"route=ordinary-exact reason=singleton-or-unsupported-exact"*) ;;
    *) echo "unexpected second Caliptra route at horizon $horizon" >&2; exit 1 ;;
  esac
  actual=$(printf '%s\n' "$production" | sed -n 's/.* answers=\([^ ]*\) .*/\1/p' | tr ',' '+')
  [[ $actual == "$expected" ]]
  verification=$("$binary" verify-btor2-predicate-set "$first" 23,26,30 \
    "$horizon" "$artifact")
  case $verification in
    *"status=VERIFIED"*"route=ordinary-exact"*) ;;
    *) echo "Caliptra predicate-set verification failed at horizon $horizon" >&2; exit 1 ;;
  esac
  verified=$(printf '%s\n' "$verification" | sed -n 's/.* answers=\([^ ]*\) .*/\1/p' | tr ',' '+')
  [[ $verified == "$expected" ]]
  printf '2,%s,23+26+30,%s,%s,accepted,ordinary-exact,%s,true,true,true,validated\n' \
    "$horizon" "$expected" "$actual" \
    "$(wc -c <"$artifact" | tr -d ' ')" >>"$scratch/result.csv"
done

horizon=1000000000
artifact=$scratch/h${horizon}.btor2-set-cert
log=$scratch/h${horizon}.log
set +e
"$binary" check-btor2-predicate-set "$first" 23,26,30 "$horizon" \
  "$artifact" >"$log" 2>&1
status=$?
set -e
[[ $status -eq 2 && ! -e "$artifact" ]]
grep -q 'explicit search fallback error: search horizon exceeds limit' "$log"
printf '%s\n' \
  '2,1000000000,23+26+30,not-applicable,not-applicable,refused,search-horizon-limit,0,true,true,true,retained-negative' \
  >>"$scratch/result.csv"

if ! (set -C; cat "$scratch/result.csv" >"$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi
echo "caliptra_wdt_word_probe_v2=VALIDATED accepted=3 refused=1 output=$output"
