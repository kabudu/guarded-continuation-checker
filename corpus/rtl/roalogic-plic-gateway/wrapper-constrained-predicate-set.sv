module roalogic_plic_gateway_constrained_predicate_set (
  input logic clk,
  input logic reset_n,
  input logic source,
  input logic edge_mode,
  input logic claim,
  input logic complete,
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

  logic source_seen;
  logic claim_in_flight;
  always_ff @(posedge clk or negedge reset_n) begin
    if (!reset_n) begin
      source_seen <= 1'b0;
      claim_in_flight <= 1'b0;
    end else begin
      source_seen <= source_seen | source;
      if (pending && claim)
        claim_in_flight <= 1'b1;
      else if (complete)
        claim_in_flight <= 1'b0;
    end
  end

  always_comb begin
    assume (!(claim && complete));
    assume (!complete || claim_in_flight);
    assert (!(pending && !source_seen));
    assert (!(pending && claim_in_flight));
  end
endmodule
