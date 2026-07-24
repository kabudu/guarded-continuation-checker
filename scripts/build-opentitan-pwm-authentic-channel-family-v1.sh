#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 YOSYS OUTPUT_2.btor2 OUTPUT_4.btor2 OUTPUT_6.btor2" >&2
  exit 2
fi

yosys=$1
shift
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture=$repo/corpus/rtl/opentitan-pwm-channel-family
upstream=$fixture/upstream-child
expected_yosys=b8e7da6f40ae8f552c116bf6c359b07c6533e159
harness=${GCC_PWM_HARNESS:-$fixture/authentic-harness.sv}
top=${GCC_PWM_TOP:-opentitan_pwm_authentic_harness}
output_format=${GCC_PWM_OUTPUT_FORMAT:-btor2}
harness_name=$(basename "$harness")

[[ -x "$yosys" ]] || { echo "Yosys must be executable" >&2; exit 2; }
[[ $($yosys -V) == *"git sha1 $expected_yosys,"* ]] || {
  echo "Yosys revision mismatch" >&2
  exit 2
}
for output in "$@"; do
  [[ ! -e "$output" && ! -L "$output" ]] || {
    echo "refusing to overwrite $output" >&2
    exit 2
  }
done

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}
check_digest() {
  [[ $(sha256_file "$2") == "$1" ]] || {
    echo "pinned fixture digest mismatch: $2" >&2
    exit 2
  }
}

check_digest 618998be0948d1570e7bd5fc4db6332470f02dba9b7154aa71edc8929202d855 "$upstream/pwm_core.sv"
check_digest 0b6a8cac19d1e8ae4b04ab63fd146a105b85e2ce690084beaa24aa950faca68a "$upstream/pwm_chan.sv"
check_digest 59651b3b72ea1862524935dc099fd6fdd3b5c2926c03b6d7d31b7785be3324a7 "$upstream/pwm_reg_pkg.sv"
[[ -f "$harness" && ! -L "$harness" ]] || {
  echo "PWM harness must be an ordinary file" >&2
  exit 2
}
[[ $top =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || {
  echo "PWM top module is invalid" >&2
  exit 2
}
[[ $output_format == btor2 || $output_format == smt2 ]] || {
  echo "PWM output format must be btor2 or smt2" >&2
  exit 2
}
[[ $harness_name =~ ^[A-Za-z0-9_.-]+[.]sv$ ]] || {
  echo "PWM harness filename is invalid" >&2
  exit 2
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/gcc-opentitan-pwm-family.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
cp "$upstream"/*.sv "$scratch/"
cp "$harness" "$scratch/$harness_name"

# Pinned Yosys aborts while simplifying the generated packed struct. Lower the
# authenticated reg2hw interface to width-equivalent flat ports while leaving
# every core state and transition equation unchanged.
awk '
  /input pwm_reg_pkg::pwm_reg2hw_t reg2hw,/ {
    print "  input logic cfg_cntr_en_q_i, input logic cfg_cntr_en_qe_i,"
    print "  input logic [3:0] cfg_dc_resn_q_i, input logic cfg_dc_resn_qe_i,"
    print "  input logic [26:0] cfg_clk_div_q_i, input logic cfg_clk_div_qe_i,"
    print "  input logic [NOutputs-1:0] pwm_en_q_i, pwm_en_qe_i, invert_q_i, invert_qe_i,"
    print "  input logic [NOutputs*16-1:0] pwm_param_phase_delay_q_i,"
    print "  input logic [NOutputs-1:0] pwm_param_phase_delay_qe_i,"
    print "  input logic [NOutputs-1:0] pwm_param_blink_en_q_i, pwm_param_blink_en_qe_i,"
    print "  input logic [NOutputs-1:0] pwm_param_htbt_en_q_i, pwm_param_htbt_en_qe_i,"
    print "  input logic [NOutputs*16-1:0] duty_cycle_a_q_i, duty_cycle_b_q_i,"
    print "  input logic [NOutputs-1:0] duty_cycle_a_qe_i, duty_cycle_b_qe_i,"
    print "  input logic [NOutputs*16-1:0] blink_param_x_q_i, blink_param_y_q_i,"
    print "  input logic [NOutputs-1:0] blink_param_x_qe_i, blink_param_y_qe_i,"
    print "  input logic [1:0] alert_test_i,"
    next
  }
  {
    gsub(/reg2hw[.]cfg[.]cntr_en[.]qe/, "cfg_cntr_en_qe_i")
    gsub(/reg2hw[.]cfg[.]cntr_en[.]q/, "cfg_cntr_en_q_i")
    gsub(/reg2hw[.]cfg[.]dc_resn[.]qe/, "cfg_dc_resn_qe_i")
    gsub(/reg2hw[.]cfg[.]dc_resn[.]q/, "cfg_dc_resn_q_i")
    gsub(/reg2hw[.]cfg[.]clk_div[.]qe/, "cfg_clk_div_qe_i")
    gsub(/reg2hw[.]cfg[.]clk_div[.]q/, "cfg_clk_div_q_i")
    gsub(/reg2hw[.]pwm_en\[ii\][.]qe/, "pwm_en_qe_i[ii]")
    gsub(/reg2hw[.]pwm_en\[ii\][.]q/, "pwm_en_q_i[ii]")
    gsub(/reg2hw[.]invert\[ii\][.]qe/, "invert_qe_i[ii]")
    gsub(/reg2hw[.]invert\[ii\][.]q/, "invert_q_i[ii]")
    gsub(/reg2hw[.]pwm_param\[ii\][.]phase_delay[.]qe/, "pwm_param_phase_delay_qe_i[ii]")
    gsub(/reg2hw[.]pwm_param\[ii\][.]phase_delay[.]q/, "pwm_param_phase_delay_q_i[ii*16 +: 16]")
    gsub(/reg2hw[.]pwm_param\[ii\][.]blink_en[.]qe/, "pwm_param_blink_en_qe_i[ii]")
    gsub(/reg2hw[.]pwm_param\[ii\][.]blink_en[.]q/, "pwm_param_blink_en_q_i[ii]")
    gsub(/reg2hw[.]pwm_param\[ii\][.]htbt_en[.]qe/, "pwm_param_htbt_en_qe_i[ii]")
    gsub(/reg2hw[.]pwm_param\[ii\][.]htbt_en[.]q/, "pwm_param_htbt_en_q_i[ii]")
    gsub(/reg2hw[.]duty_cycle\[ii\][.]a[.]qe/, "duty_cycle_a_qe_i[ii]")
    gsub(/reg2hw[.]duty_cycle\[ii\][.]a[.]q/, "duty_cycle_a_q_i[ii*16 +: 16]")
    gsub(/reg2hw[.]duty_cycle\[ii\][.]b[.]qe/, "duty_cycle_b_qe_i[ii]")
    gsub(/reg2hw[.]duty_cycle\[ii\][.]b[.]q/, "duty_cycle_b_q_i[ii*16 +: 16]")
    gsub(/reg2hw[.]blink_param\[ii\][.]x[.]qe/, "blink_param_x_qe_i[ii]")
    gsub(/reg2hw[.]blink_param\[ii\][.]x[.]q/, "blink_param_x_q_i[ii*16 +: 16]")
    gsub(/reg2hw[.]blink_param\[ii\][.]y[.]qe/, "blink_param_y_qe_i[ii]")
    gsub(/reg2hw[.]blink_param\[ii\][.]y[.]q/, "blink_param_y_q_i[ii*16 +: 16]")
    gsub(/reg2hw[.]alert_test/, "alert_test_i")
    gsub(/module pwm_core /, "module pwm_core_flat ")
    gsub(/endmodule : pwm_core/, "endmodule : pwm_core_flat")
    print
  }
' "$scratch/pwm_core.sv" >"$scratch/pwm_core_flat.sv"
[[ $(grep -c 'reg2hw' "$scratch/pwm_core_flat.sv") -eq 0 ]] || {
  echo "unlowered reg2hw reference remains" >&2
  exit 2
}

build_one() {
  local channels=$1
  local output=$2
  local raw=$scratch/pwm-$channels.raw.$output_format
  local writer
  if [[ $output_format == btor2 ]]; then
    writer="write_btor -x $raw"
  else
    writer="write_smt2 -wires $raw"
  fi
  (
    cd "$scratch"
    "$yosys" -Q -q -p "
      read_verilog -formal -sv pwm_chan.sv pwm_core_flat.sv $harness_name;
      chparam -set NOutputs $channels $top;
      hierarchy -check -top $top;
      proc; opt; flatten; opt; async2sync; dffunmap; setundef -zero -init;
      clean -purge; $writer
    "
  )
  local canonical=$scratch/canonical-$channels.$output_format
  if [[ $output_format == btor2 ]]; then
    awk -v channels="$channels" -v revision="$expected_yosys" '
      /^; BTOR description generated by Yosys / {
        printf "; BTOR description generated by pinned Yosys %s for exact OpenTitan PWM child source with %s channels.\n", revision, channels
        next
      }
      { sub(/ ; [^;]*[.]sv:.*/, "") }
      { print }
    ' "$raw" >"$canonical"
  else
    cp "$raw" "$canonical"
  fi
  (set -C; cp "$canonical" "$output") 2>/dev/null || {
    echo "refusing to overwrite $output" >&2
    exit 2
  }
}

build_one 2 "$1"
build_one 4 "$2"
build_one 6 "$3"
echo "opentitan_pwm_authentic_channel_family_v1=GENERATED channels=2,4,6 format=$output_format"
