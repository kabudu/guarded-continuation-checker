module opentitan_prim_count_revision_environment (
  input  logic       observed_full,
  output logic       clk,
  output logic       reset_n,
  output logic       clear,
  output logic       set,
  output logic [1:0] set_count,
  output logic       enable,
  output logic [1:0] step,
  output logic       observed_bad,
  output logic       retained_state
);
  initial retained_state = 1'b0;

  always_ff @(posedge observed_full) begin
    retained_state <= 1'b1;
  end

  assign clk = 1'b0;
  assign reset_n = retained_state;
  assign clear = 1'b0;
  assign set = 1'b0;
  assign set_count = 2'b00;
  assign enable = 1'b0;
  assign step = 2'b01;
  assign observed_bad = observed_full;
endmodule
