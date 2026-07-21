module caliptra_wdt_predicate_set (
  input logic clk,
  input logic reset,
  output logic t1_timeout,
  output logic t2_timeout,
  output logic fatal_timeout
);
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

  always_comb begin
    assert (!t1_timeout);
    assert (!t2_timeout);
    assert (!fatal_timeout);
  end
endmodule
