module roalogic_plic_gateway_revision_oracle #(
  parameter CHECK_REPEATED_PENDING = 1
) (
  input logic clk,
  input logic reset_n,
  input logic source,
  input logic edge_mode,
  input logic claim,
  input logic complete
);
  logic pending;
  logic pending_seen;

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

  always_ff @(posedge clk or negedge reset_n) begin
    if (!reset_n)
      pending_seen <= 1'b0;
    else
      pending_seen <= pending;
  end

  generate
    if (CHECK_REPEATED_PENDING) begin : gen_repeated
      always_comb assert (!(pending_seen && pending));
    end else begin : gen_impossible
      always_comb assert (!(1'b0 && pending));
    end
  endgenerate
endmodule
