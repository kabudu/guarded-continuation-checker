module opentitan_prim_count_query_oracle #(
  parameter integer PROPERTY_INDEX = 0
) (
  input logic clk
);
  logic reset_n = 1'b0;
  logic [1:0] count;
  logic error;
  logic [7:0] properties;

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

  assign properties = {
    ^count,
    count[1],
    count[0],
    error,
    count == 2'b11,
    count == 2'b10,
    count == 2'b01,
    count == 2'b00
  };

  always_comb assert (!properties[PROPERTY_INDEX]);
endmodule
