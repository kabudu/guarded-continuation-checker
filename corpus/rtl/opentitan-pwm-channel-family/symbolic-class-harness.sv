// GCC-authored formal harness for grouped firmware register traffic.
// SPDX-License-Identifier: Apache-2.0

module opentitan_pwm_symbolic_class_harness #(
  parameter int unsigned NOutputs = 6
) (
  input  logic                clk_i,
  input  logic [1:0]          class_enable_i,
  input  logic [1:0]          class_invert_i,
  input  logic [1:0]          class_write_i,
  output logic [NOutputs-1:0] pwm_o,
  output logic [3:0]          step_o
);
  logic [3:0] step_q = 4'd0;
  logic rst_ni;
  logic cfg_cntr_en_q;
  logic cfg_cntr_en_qe;
  logic [3:0] cfg_dc_resn_q;
  logic cfg_dc_resn_qe;
  logic [26:0] cfg_clk_div_q;
  logic cfg_clk_div_qe;
  logic [NOutputs-1:0] pwm_en_q;
  logic [NOutputs-1:0] pwm_en_qe;
  logic [NOutputs-1:0] invert_q;
  logic [NOutputs-1:0] invert_qe;
  logic [NOutputs*16-1:0] phase_delay_q;
  logic [NOutputs-1:0] phase_delay_qe;
  logic [NOutputs-1:0] blink_en_q;
  logic [NOutputs-1:0] blink_en_qe;
  logic [NOutputs-1:0] htbt_en_q;
  logic [NOutputs-1:0] htbt_en_qe;
  logic [NOutputs*16-1:0] duty_cycle_a_q;
  logic [NOutputs-1:0] duty_cycle_a_qe;
  logic [NOutputs*16-1:0] duty_cycle_b_q;
  logic [NOutputs-1:0] duty_cycle_b_qe;
  logic [NOutputs*16-1:0] blink_param_x_q;
  logic [NOutputs-1:0] blink_param_x_qe;
  logic [NOutputs*16-1:0] blink_param_y_q;
  logic [NOutputs-1:0] blink_param_y_qe;

  assign rst_ni = step_q != 4'd0;
  assign step_o = step_q;

  always_ff @(posedge clk_i) begin
    if (step_q != 4'hf) step_q <= step_q + 4'd1;
  end

  always_comb begin
    cfg_cntr_en_q = 1'b1;
    cfg_cntr_en_qe = step_q == 4'd1;
    cfg_dc_resn_q = 4'd3;
    cfg_dc_resn_qe = 1'b0;
    cfg_clk_div_q = 27'd0;
    cfg_clk_div_qe = 1'b0;
    pwm_en_q = '0;
    pwm_en_qe = '0;
    invert_q = '0;
    invert_qe = '0;
    phase_delay_q = '0;
    phase_delay_qe = '0;
    blink_en_q = '0;
    blink_en_qe = '0;
    htbt_en_q = '0;
    htbt_en_qe = '0;
    duty_cycle_a_q = '0;
    duty_cycle_a_qe = '0;
    duty_cycle_b_q = '0;
    duty_cycle_b_qe = '0;
    blink_param_x_q = '0;
    blink_param_x_qe = '0;
    blink_param_y_q = '0;
    blink_param_y_qe = '0;

    for (int unsigned ii = 0; ii < NOutputs; ii++) begin
      // Firmware contract: all even channels use class 0 register traffic and
      // all odd channels use class 1. The primary inputs remain symbolic.
      pwm_en_q[ii] = class_enable_i[ii[0]];
      pwm_en_qe[ii] = class_write_i[ii[0]];
      invert_q[ii] = class_invert_i[ii[0]];
      invert_qe[ii] = class_write_i[ii[0]];
      phase_delay_q[ii*16 +: 16] = ii[0] ? 16'd2 : 16'd0;
      phase_delay_qe[ii] = class_write_i[ii[0]];
      blink_en_q[ii] = 1'b1;
      blink_en_qe[ii] = class_write_i[ii[0]];
      htbt_en_q[ii] = ii[0];
      htbt_en_qe[ii] = class_write_i[ii[0]];
      duty_cycle_a_q[ii*16 +: 16] = ii[0] ? 16'd6 : 16'd4;
      duty_cycle_a_qe[ii] = class_write_i[ii[0]];
      duty_cycle_b_q[ii*16 +: 16] = ii[0] ? 16'd10 : 16'd8;
      duty_cycle_b_qe[ii] = class_write_i[ii[0]];
      blink_param_x_q[ii*16 +: 16] = ii[0] ? 16'd1 : 16'd2;
      blink_param_x_qe[ii] = class_write_i[ii[0]];
      blink_param_y_q[ii*16 +: 16] = ii[0] ? 16'd1 : 16'd0;
      blink_param_y_qe[ii] = class_write_i[ii[0]];
    end
  end

  pwm_core_flat #(
    .NOutputs (NOutputs),
    .PhaseCntDw(4),
    .BeatCntDw (3)
  ) u_pwm_core (
    .clk_core_i(clk_i),
    .rst_core_ni(rst_ni),
    .cfg_cntr_en_q_i(cfg_cntr_en_q),
    .cfg_cntr_en_qe_i(cfg_cntr_en_qe),
    .cfg_dc_resn_q_i(cfg_dc_resn_q),
    .cfg_dc_resn_qe_i(cfg_dc_resn_qe),
    .cfg_clk_div_q_i(cfg_clk_div_q),
    .cfg_clk_div_qe_i(cfg_clk_div_qe),
    .pwm_en_q_i(pwm_en_q),
    .pwm_en_qe_i(pwm_en_qe),
    .invert_q_i(invert_q),
    .invert_qe_i(invert_qe),
    .pwm_param_phase_delay_q_i(phase_delay_q),
    .pwm_param_phase_delay_qe_i(phase_delay_qe),
    .pwm_param_blink_en_q_i(blink_en_q),
    .pwm_param_blink_en_qe_i(blink_en_qe),
    .pwm_param_htbt_en_q_i(htbt_en_q),
    .pwm_param_htbt_en_qe_i(htbt_en_qe),
    .duty_cycle_a_q_i(duty_cycle_a_q),
    .duty_cycle_a_qe_i(duty_cycle_a_qe),
    .duty_cycle_b_q_i(duty_cycle_b_q),
    .duty_cycle_b_qe_i(duty_cycle_b_qe),
    .blink_param_x_q_i(blink_param_x_q),
    .blink_param_x_qe_i(blink_param_x_qe),
    .blink_param_y_q_i(blink_param_y_q),
    .blink_param_y_qe_i(blink_param_y_qe),
    .alert_test_i(2'b00),
    .pwm_o(pwm_o)
  );
endmodule
