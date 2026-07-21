#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 YOSYS_BINARY OUTPUT_DIRECTORY" >&2
  exit 2
fi

yosys=$1
output=$2
repo=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-aon-timer
top=opentitan_aon_dual_timer_bounded_aiger
upstream=$fixture/upstream/aon_timer_core.sv
expected_upstream=226ed77228b49c3d9231027410b5572ae7812bb0ed76dc6679c18ef028895d2b
expected_yosys=b8e7da6f40ae8f552c116bf6c359b07c6533e159

test -x "$yosys"
if [ -e "$output" ] || [ -L "$output" ]; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "sha256sum or shasum is required" >&2
    exit 2
  fi
}

test "$(sha256_file "$upstream")" = "$expected_upstream" || {
  echo "pinned OpenTitan source digest mismatch" >&2
  exit 2
}
case $("$yosys" -V) in
  *"git sha1 $expected_yosys,"*) ;;
  *) echo "Yosys revision mismatch" >&2; exit 2 ;;
esac

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-aiger.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
mkdir "$scratch/out"
sed -f "$fixture/normalize-yosys.sed" \
  "$upstream" >"$scratch/aon_timer_core.sv"
cp "$fixture/compat/lc_ctrl_pkg.sv" \
  "$fixture/compat/aon_timer_reg_pkg.sv" \
  "$fixture/wrapper-dual-timer-bounded-aiger.sv" "$scratch/"

build_model() {
  horizon=$1
  mask=$2
  label=$3
  model=$scratch/out/h${horizon}-${label}.aag
  (
    cd "$scratch"
    "$yosys" -Q -q -p "
    read_verilog -sv \
      lc_ctrl_pkg.sv \
      aon_timer_reg_pkg.sv \
      aon_timer_core.sv \
      wrapper-dual-timer-bounded-aiger.sv;
    hierarchy -check -top $top;
    chparam -set Horizon $horizon -set PropertyMask $mask $top;
    hierarchy -check -top $top;
    proc; flatten; async2sync; opt;
    techmap; opt; dffunmap; simplemap; dffunmap; aigmap; clean;
    setundef -zero -init;
    write_aiger -ascii -zinit -symbols \
      -ywmap out/h${horizon}-${label}.ywmap \
      out/h${horizon}-${label}.aag
  "
  )
  test -s "$model"
  header=$(sed -n '1p' "$model")
  bads=$(printf '%s\n' "$header" | awk '{print ($1 == "aag" && NF >= 7) ? $7 : 0}')
  expected=$(printf '%s' "$mask" | awk '{n=$1; c=0; while(n){c+=n%2; n=int(n/2)}; print c}')
  test "$bads" -eq "$expected"
}

for horizon in 4 5 7 9; do
  build_model "$horizon" 1 wake
  build_model "$horizon" 2 bark
  build_model "$horizon" 4 bite
done
build_model 4 7 safe-set
build_model 5 5 safe-set
build_model 7 4 safe-set

(
  cd "$scratch/out"
  for file in *.aag *.ywmap; do
    shasum -a 256 "$file"
  done
) >"$scratch/out/SHA256SUMS"
mv "$scratch/out" "$output"
echo "opentitan_aon_dual_timer_aiger=GENERATED models=15 output=$output"
