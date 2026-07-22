`define ASSERT_INIT(NAME, PROPERTY)
`define ASSERT(NAME, PROPERTY)
`define ASSUME(NAME, PROPERTY)
`define PRIM_COUNT_MODULE prim_count

module prim_buf #(
  parameter int Width = 1
) (
  input  logic [Width-1:0] in_i,
  output logic [Width-1:0] out_o
);
  assign out_o = in_i;
endmodule

module prim_flop #(
  parameter int Width = 1,
  parameter logic [Width-1:0] ResetValue = '0
) (
  input  logic clk_i,
  input  logic rst_ni,
  input  logic [Width-1:0] d_i,
  output logic [Width-1:0] q_o
);
  always_ff @(posedge clk_i or negedge rst_ni) begin
    if (!rst_ni) q_o <= ResetValue;
    else q_o <= d_i;
  end
endmodule
