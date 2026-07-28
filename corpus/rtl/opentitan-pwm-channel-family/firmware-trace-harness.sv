// GCC-authored per-channel firmware trace harness.
// SPDX-License-Identifier: Apache-2.0

module opentitan_pwm_firmware_trace_harness #(
  parameter int unsigned NOutputs = 6
) (
  input  logic                         clk_i,
  input  logic [NOutputs-1:0]          channel_write_i,
  input  logic [NOutputs-1:0]          channel_enable_i,
  input  logic [NOutputs-1:0]          channel_invert_i,
  input  logic [NOutputs-1:0]          blink_enable_i,
  input  logic [NOutputs-1:0]          heartbeat_enable_i,
  input  logic [15:0]                  phase_delay_0_i,
  input  logic [15:0]                  phase_delay_1_i,
  input  logic [15:0]                  phase_delay_2_i,
  input  logic [15:0]                  phase_delay_3_i,
  input  logic [15:0]                  phase_delay_4_i,
  input  logic [15:0]                  phase_delay_5_i,
  input  logic [15:0]                  duty_cycle_a_0_i,
  input  logic [15:0]                  duty_cycle_a_1_i,
  input  logic [15:0]                  duty_cycle_a_2_i,
  input  logic [15:0]                  duty_cycle_a_3_i,
  input  logic [15:0]                  duty_cycle_a_4_i,
  input  logic [15:0]                  duty_cycle_a_5_i,
  input  logic [15:0]                  duty_cycle_b_0_i,
  input  logic [15:0]                  duty_cycle_b_1_i,
  input  logic [15:0]                  duty_cycle_b_2_i,
  input  logic [15:0]                  duty_cycle_b_3_i,
  input  logic [15:0]                  duty_cycle_b_4_i,
  input  logic [15:0]                  duty_cycle_b_5_i,
  input  logic [15:0]                  blink_parameter_x_0_i,
  input  logic [15:0]                  blink_parameter_x_1_i,
  input  logic [15:0]                  blink_parameter_x_2_i,
  input  logic [15:0]                  blink_parameter_x_3_i,
  input  logic [15:0]                  blink_parameter_x_4_i,
  input  logic [15:0]                  blink_parameter_x_5_i,
  input  logic [15:0]                  blink_parameter_y_0_i,
  input  logic [15:0]                  blink_parameter_y_1_i,
  input  logic [15:0]                  blink_parameter_y_2_i,
  input  logic [15:0]                  blink_parameter_y_3_i,
  input  logic [15:0]                  blink_parameter_y_4_i,
  input  logic [15:0]                  blink_parameter_y_5_i,
  output logic [NOutputs-1:0]          pwm_o,
  output logic [3:0]                   step_o
);
  logic [3:0] step_q = 4'd0;
  logic rst_ni;
  logic [NOutputs*16-1:0] phase_delay;
  logic [NOutputs*16-1:0] duty_cycle_a;
  logic [NOutputs*16-1:0] duty_cycle_b;
  logic [NOutputs*16-1:0] blink_parameter_x;
  logic [NOutputs*16-1:0] blink_parameter_y;

  assign rst_ni = step_q != 4'd0;
  assign step_o = step_q;
  assign phase_delay = {
    phase_delay_5_i, phase_delay_4_i, phase_delay_3_i,
    phase_delay_2_i, phase_delay_1_i, phase_delay_0_i
  };
  assign duty_cycle_a = {
    duty_cycle_a_5_i, duty_cycle_a_4_i, duty_cycle_a_3_i,
    duty_cycle_a_2_i, duty_cycle_a_1_i, duty_cycle_a_0_i
  };
  assign duty_cycle_b = {
    duty_cycle_b_5_i, duty_cycle_b_4_i, duty_cycle_b_3_i,
    duty_cycle_b_2_i, duty_cycle_b_1_i, duty_cycle_b_0_i
  };
  assign blink_parameter_x = {
    blink_parameter_x_5_i, blink_parameter_x_4_i, blink_parameter_x_3_i,
    blink_parameter_x_2_i, blink_parameter_x_1_i, blink_parameter_x_0_i
  };
  assign blink_parameter_y = {
    blink_parameter_y_5_i, blink_parameter_y_4_i, blink_parameter_y_3_i,
    blink_parameter_y_2_i, blink_parameter_y_1_i, blink_parameter_y_0_i
  };

  always_ff @(posedge clk_i) begin
    if (step_q != 4'hf) step_q <= step_q + 4'd1;
  end

  pwm_core_flat #(
    .NOutputs (NOutputs),
    .PhaseCntDw(4),
    .BeatCntDw (3)
  ) u_pwm_core (
    .clk_core_i(clk_i),
    .rst_core_ni(rst_ni),
    .cfg_cntr_en_q_i(1'b1),
    .cfg_cntr_en_qe_i(step_q == 4'd1),
    .cfg_dc_resn_q_i(4'd3),
    .cfg_dc_resn_qe_i(1'b0),
    .cfg_clk_div_q_i(27'd0),
    .cfg_clk_div_qe_i(1'b0),
    .pwm_en_q_i(channel_enable_i),
    .pwm_en_qe_i(channel_write_i),
    .invert_q_i(channel_invert_i),
    .invert_qe_i(channel_write_i),
    .pwm_param_phase_delay_q_i(phase_delay),
    .pwm_param_phase_delay_qe_i(channel_write_i),
    .pwm_param_blink_en_q_i(blink_enable_i),
    .pwm_param_blink_en_qe_i(channel_write_i),
    .pwm_param_htbt_en_q_i(heartbeat_enable_i),
    .pwm_param_htbt_en_qe_i(channel_write_i),
    .duty_cycle_a_q_i(duty_cycle_a),
    .duty_cycle_a_qe_i(channel_write_i),
    .duty_cycle_b_q_i(duty_cycle_b),
    .duty_cycle_b_qe_i(channel_write_i),
    .blink_param_x_q_i(blink_parameter_x),
    .blink_param_x_qe_i(channel_write_i),
    .blink_param_y_q_i(blink_parameter_y),
    .blink_param_y_qe_i(channel_write_i),
    .alert_test_i(2'b00),
    .pwm_o(pwm_o)
  );
endmodule
