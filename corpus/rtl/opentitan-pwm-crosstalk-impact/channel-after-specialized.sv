// Derived from OpenTitan pwm_chan.sv at
// 86db2898288664d8d5e8fc635b48951ef63e3439.
// The pulse state models channel timing state cleared by clr_chan_cntr_i. The
// visible output retains the child's registered glitch-prevention equation.
module opentitan_pwm_channel_impact (
  input  logic clk_i,
  input  logic clear_i,
  input  logic phase_two_i,
  input  logic phase_four_i,
  input  logic raw_pwm_i,
  output logic core_only_bad_o,
  output logic channel_only_bad_o,
  output logic joint_bad_o,
  output logic reset_safe_bad_o,
  output logic impossible_bad_o
);
  logic pulse_active_q = 1'b1;
  logic pwm_d;
  logic pwm_q = 1'b0;

  always_ff @(posedge clk_i) begin
    if (clear_i) pulse_active_q <= 1'b0;
    else if (!raw_pwm_i) pulse_active_q <= 1'b1;
    pwm_q <= pwm_d;
  end

  assign pwm_d = raw_pwm_i & pulse_active_q;
  assign core_only_bad_o = phase_four_i & ~pulse_active_q;
  assign channel_only_bad_o = phase_two_i & pwm_q;
  assign joint_bad_o = channel_only_bad_o | (phase_four_i & ~pwm_q);
  assign reset_safe_bad_o = ~raw_pwm_i & pwm_q;
  assign impossible_bad_o = 1'b1;
endmodule
