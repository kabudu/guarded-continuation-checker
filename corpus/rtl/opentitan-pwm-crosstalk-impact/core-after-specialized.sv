// Derived from OpenTitan pwm_core.sv at
// 86db2898288664d8d5e8fc635b48951ef63e3439.
// This retains the new captured enable/invert state and selective clear
// equation under one deterministic shared-register write sequence.
module opentitan_pwm_core_impact (
  input  logic clk_i,
  input  logic scenario_enable_i,
  output logic clear_o,
  output logic phase_two_o,
  output logic phase_four_o,
  output logic raw_pwm_o
);
  logic [2:0] phase_q = 3'd0;

  always_comb assume (scenario_enable_i);

  always_ff @(posedge clk_i) begin
    if (scenario_enable_i && phase_q < 3'd4) phase_q <= phase_q + 3'd1;
  end

  logic pwm_en;
  logic pwm_en_qe;
  logic invert;
  logic invert_qe;
  logic pwm_param_qe;
  logic pwm_en_q = 1'b0;
  logic invert_q = 1'b0;

  assign pwm_en = 1'b1;
  assign pwm_en_qe = (phase_q == 3'd0) || (phase_q == 3'd2);
  assign invert = 1'b0;
  assign invert_qe = 1'b0;
  assign pwm_param_qe = 1'b0;

  // Child pwm_core.sv captures each channel's effective shared-register state.
  always_ff @(posedge clk_i) begin
    if (pwm_en_qe) pwm_en_q <= pwm_en;
    if (invert_qe) invert_q <= invert;
  end

  // Child pwm_core.sv clears only on an effective channel-local change.
  assign clear_o = (pwm_en_qe & pwm_en & ~pwm_en_q) |
                   (invert_qe & (invert ^ invert_q)) |
                   pwm_param_qe;
  assign phase_two_o = phase_q == 3'd2;
  assign phase_four_o = phase_q == 3'd4;
  assign raw_pwm_o = phase_q >= 3'd2;
endmodule
