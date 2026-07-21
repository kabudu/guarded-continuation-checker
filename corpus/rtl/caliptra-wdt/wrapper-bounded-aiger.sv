module caliptra_wdt_bounded_aiger #(
  parameter integer Horizon = 3,
  parameter logic [2:0] PropertyMask = 3'b111
) (
  input logic clk,
  input logic reset,
  output logic observed_t1_timeout,
  output logic observed_t2_timeout,
  output logic observed_fatal_timeout,
  output logic [3:0] observed_frame
);
  logic t1_timeout;
  logic t2_timeout;
  logic fatal_timeout;
  logic [3:0] frame;

  initial frame = 4'd0;

  assign observed_t1_timeout = t1_timeout;
  assign observed_t2_timeout = t2_timeout;
  assign observed_fatal_timeout = fatal_timeout;
  assign observed_frame = frame;

  always_ff @(posedge clk) begin
    if (frame <= Horizon[3:0]) frame <= frame + 4'd1;
  end

  wdt #(
    .WDT_TIMEOUT_PERIOD_NUM_DWORDS(1)
  ) dut (
    .clk(clk),
    .cptra_rst_b(~reset),
    .timer1_en(1'b1),
    .timer2_en(1'b0),
    .timer1_restart(1'b0),
    .timer2_restart(1'b0),
    .timer1_timeout_period(32'd3),
    .timer2_timeout_period(32'd2),
    .wdt_timer1_timeout_serviced(1'b0),
    .wdt_timer2_timeout_serviced(1'b0),
    .t1_timeout(t1_timeout),
    .t2_timeout(t2_timeout),
    .fatal_timeout(fatal_timeout)
  );

  if (PropertyMask[0]) begin : timer1_property
    always_comb assert (!(frame <= Horizon[3:0] && t1_timeout));
  end
  if (PropertyMask[1]) begin : timer2_property
    always_comb assert (!(frame <= Horizon[3:0] && t2_timeout));
  end
  if (PropertyMask[2]) begin : fatal_property
    always_comb assert (!(frame <= Horizon[3:0] && fatal_timeout));
  end
endmodule
