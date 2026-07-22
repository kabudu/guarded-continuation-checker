module roalogic_plic_gateway_revision_component (
  input  logic clk,
  input  logic reset_n,
  input  logic source,
  input  logic edge_mode,
  input  logic claim,
  input  logic complete,
  output logic pending
);
  plic_gateway #(
    .MAX_PENDING_COUNT(3)
  ) dut (
    .rst_n(reset_n),
    .clk(clk),
    .src(source),
    .edge_lvl(edge_mode),
    .ip(pending),
    .claim(claim),
    .complete(complete)
  );

endmodule
