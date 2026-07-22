// Derived from OpenTitan pwm_core.sv at
// 376021484b3cab4ef0d352f73d16f0b7a80c0970.
// This retains the old per-channel clear equation under one deterministic
// shared-register write sequence. See PROVENANCE.md for the source boundary.
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
  logic invert_qe;
  logic pwm_param_qe;

  assign pwm_en = 1'b1;
  assign pwm_en_qe = (phase_q == 3'd0) || (phase_q == 3'd2);
  assign invert_qe = 1'b0;
  assign pwm_param_qe = 1'b0;

  // Parent pwm_core.sv clears on every shared register write enable.
  assign clear_o = pwm_en_qe | invert_qe | pwm_param_qe;
  assign phase_two_o = phase_q == 3'd2;
  assign phase_four_o = phase_q == 3'd4;
  assign raw_pwm_o = phase_q >= 3'd2;
endmodule
