module opentitan_pwm_crosstalk_impact_oracle #(
  parameter int unsigned PROPERTY_INDEX = 0
) (
  input logic clk_i,
  input logic scenario_enable_i
);
  logic clear;
  logic phase_two;
  logic phase_four;
  logic raw_pwm;
  logic core_only_bad;
  logic channel_only_bad;
  logic joint_bad;
  logic reset_safe_bad;
  logic impossible_bad;
  logic selected_bad;

  opentitan_pwm_core_impact core (
    .clk_i,
    .scenario_enable_i,
    .clear_o(clear),
    .phase_two_o(phase_two),
    .phase_four_o(phase_four),
    .raw_pwm_o(raw_pwm)
  );

  opentitan_pwm_channel_impact channel (
    .clk_i,
    .clear_i(clear),
    .phase_two_i(phase_two),
    .phase_four_i(phase_four),
    .raw_pwm_i(raw_pwm),
    .core_only_bad_o(core_only_bad),
    .channel_only_bad_o(channel_only_bad),
    .joint_bad_o(joint_bad),
    .reset_safe_bad_o(reset_safe_bad),
    .impossible_bad_o(impossible_bad)
  );

  always_comb begin
    case (PROPERTY_INDEX)
      0: selected_bad = impossible_bad;
      1: selected_bad = core_only_bad;
      2: selected_bad = channel_only_bad;
      3: selected_bad = joint_bad;
      4: selected_bad = reset_safe_bad;
      default: selected_bad = 1'b1;
    endcase
  end

  always_comb assert (!selected_bad);
endmodule
