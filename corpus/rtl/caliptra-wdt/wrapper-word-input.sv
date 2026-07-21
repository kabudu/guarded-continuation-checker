module caliptra_wdt_word_input (
  input logic clk,
  input logic reset,
  input logic [1:0] timeout_period,
  output logic t1_timeout
);
  logic unused_t2_timeout;
  logic unused_fatal_timeout;

  wdt #(
    .WDT_TIMEOUT_PERIOD_NUM_DWORDS(1)
  ) dut (
    .clk(clk),
    .cptra_rst_b(~reset),
    .timer1_en(1'b1),
    .timer2_en(1'b0),
    .timer1_restart(1'b0),
    .timer2_restart(1'b0),
    .timer1_timeout_period({30'b0, timeout_period}),
    .timer2_timeout_period(32'd3),
    .wdt_timer1_timeout_serviced(1'b0),
    .wdt_timer2_timeout_serviced(1'b0),
    .t1_timeout(t1_timeout),
    .t2_timeout(unused_t2_timeout),
    .fatal_timeout(unused_fatal_timeout)
  );

  always_comb begin
    assume (timeout_period != 2'b00);
    assert (!t1_timeout);
  end
endmodule
