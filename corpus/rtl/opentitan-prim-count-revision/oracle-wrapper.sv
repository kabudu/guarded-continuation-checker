module opentitan_prim_count_revision_oracle (
  input logic clk
);
  logic reset_n = 1'b0;
  logic [1:0] count;
  logic error;

  always_ff @(posedge clk) begin
    reset_n <= 1'b1;
  end

  `PRIM_COUNT_MODULE dut (
    .clk_i(clk),
    .rst_ni(reset_n),
    .clr_i(1'b0),
    .set_i(1'b0),
    .set_cnt_i(2'b00),
    .en_i(1'b0),
    .step_i(2'b01),
    .cnt_o(count),
    .err_o(error)
  );

  always_comb assert (!(&count));
endmodule
