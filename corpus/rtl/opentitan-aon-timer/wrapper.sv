module opentitan_aon_watchdog_bounded #(
  parameter logic [31:0] BiteThreshold = 32'd9
) (
  input  logic clk,
  input  logic reset,
  output logic bad
);
  aon_timer_reg_pkg::aon_timer_reg2hw_t regs;
  logic [31:0] count;
  logic count_we;
  logic [31:0] count_next;
  logic reset_req;

  initial count = 32'd0;

  always_comb begin
    regs = '0;
    regs.wdog_ctrl.enable.q = 1'b1;
    regs.wdog_bark_thold.q = BiteThreshold - 1'b1;
    regs.wdog_bite_thold.q = BiteThreshold;
    regs.wdog_count.q = count;
  end

  always_ff @(posedge clk) begin
    if (reset) count <= 32'd0;
    else if (count_we) count <= count_next;
  end

  aon_timer_core core(
    .clk_aon_i(clk),
    .rst_aon_ni(~reset),
    .lc_escalate_en_i(12'haaa),
    .sleep_mode_i(1'b0),
    .reg2hw_i(regs),
    .wkup_count_reg_wr_o(),
    .wkup_count_wr_data_o(),
    .wdog_count_reg_wr_o(count_we),
    .wdog_count_wr_data_o(count_next),
    .wkup_intr_o(),
    .wdog_intr_o(),
    .wdog_reset_req_o(reset_req)
  );

  assign bad = reset_req;
  always_comb assert (!bad);
endmodule
