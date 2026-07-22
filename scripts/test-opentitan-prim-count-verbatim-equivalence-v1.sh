#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 YOSYS_SLANG BEFORE_CHECKOUT AFTER_CHECKOUT OUTPUT.csv WORKDIR" >&2
  exit 2
fi

yosys=$1
before_checkout=$2
after_checkout=$3
output=$4
workdir=$5
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-prim-count-revision
before_revision=34157c7afb84a7be7b1b1250d673f9fa8a3c18ce
after_revision=369cffc85db0e6d5a667676a6f89987b94210e70
expected_yosys=b8e7da6f40ae8f552c116bf6c359b07c6533e159

[[ -x "$yosys" ]] || { echo "Slang-enabled Yosys must be executable" >&2; exit 2; }
[[ -d "$workdir" && ! -L "$workdir" ]] || {
  echo "WORKDIR must be an existing ordinary directory" >&2
  exit 2
}
[[ ! -e "$output" && ! -L "$output" ]] || {
  echo "refusing to overwrite $output" >&2
  exit 2
}
[[ $($yosys -V) == *"git sha1 $expected_yosys,"* ]] || {
  echo "Yosys revision mismatch" >&2
  exit 2
}
$yosys -Q -p 'help read_slang' >"$workdir/read-slang-help.log"
grep -q 'Read SystemVerilog sources' "$workdir/read-slang-help.log"
[[ $(git -C "$before_checkout" rev-parse HEAD) == "$before_revision" ]] || {
  echo "before checkout revision mismatch" >&2
  exit 2
}
[[ $(git -C "$after_checkout" rev-parse HEAD) == "$after_revision" ]] || {
  echo "after checkout revision mismatch" >&2
  exit 2
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

[[ $(sha256_file "$before_checkout/hw/ip/prim/rtl/prim_count.sv") == \
  a864392c228b1d4f4a6b4dc3baae21db3fb3afa2869d440a72623cfc9a061a45 ]]
[[ $(sha256_file "$after_checkout/hw/ip/prim/rtl/prim_count.sv") == \
  f7256b26530637658956353adf2fa99bc4fdfd25ffcc44474f62138d9cd7d78b ]]
[[ $(sha256_file "$fixture/verbatim-prelude.sv") == \
  549fff5dd038ebc4a4fa2e5623d34c9736d22e3a9861db7fd99c61488f29f216 ]]

run_equivalence() {
  local label=$1
  local checkout=$2
  local specialised_module=opentitan_prim_count_$label
  "$yosys" -Q -q -p "
    read_slang --single-unit --top prim_count --top $specialised_module \
      $fixture/verbatim-prelude.sv \
      $checkout/hw/ip/prim/rtl/prim_count_pkg.sv \
      $checkout/hw/ip/prim/rtl/prim_count.sv \
      $fixture/prim-count-$label-specialized.sv;
    proc; opt; async2sync; dffunmap;
    equiv_make prim_count $specialised_module equiv_$label;
    hierarchy -check -top equiv_$label; opt;
    equiv_simple; equiv_induct -undef -seq 12; equiv_status -assert
  " >"$workdir/$label.log" 2>&1
}

run_equivalence before "$before_checkout"
run_equivalence after "$after_checkout"

printf '%s\n' \
  'schema_version,revision,source_sha256,configuration,sequential_equivalence,status' \
  '1,34157c7afb84a7be7b1b1250d673f9fa8a3c18ce,a864392c228b1d4f4a6b4dc3baae21db3fb3afa2869d440a72623cfc9a061a45,Width=2+OutSelDnCnt=1+CrossCnt,PASS,accepted' \
  '1,369cffc85db0e6d5a667676a6f89987b94210e70,f7256b26530637658956353adf2fa99bc4fdfd25ffcc44474f62138d9cd7d78b,Width=2+OutSelDnCnt=1+CrossCnt,PASS,accepted' \
  >"$workdir/result.csv"
if ! (set -C; cp "$workdir/result.csv" "$output") 2>/dev/null; then
  echo "refusing to overwrite $output" >&2
  exit 2
fi

echo "opentitan_prim_count_verbatim_equivalence_v1=PASS revisions=2 frontend=Yosys-Slang output=$output"
