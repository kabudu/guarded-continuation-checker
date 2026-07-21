module opentitan_aon_dual_timer_predicate_set #(
  parameter logic [63:0] WakeThreshold = 64'd7,
  parameter logic [31:0] BarkThreshold = 32'd5,
  parameter logic [31:0] BiteThreshold = 32'd9
) (
  input  logic clk,
  input  logic reset,
  output logic wake_bad,
  output logic bark_bad,
  output logic bite_bad
);
  aon_timer_reg_pkg::aon_timer_reg2hw_t regs;
  logic [63:0] wake_count;
  logic [31:0] watchdog_count;
  logic wake_count_we;
  logic [63:0] wake_count_next;
  logic watchdog_count_we;
  logic [31:0] watchdog_count_next;

  initial wake_count = 64'd0;
  initial watchdog_count = 32'd0;

  always_comb begin
    regs = '0;
    regs.wkup_ctrl.enable.q = 1'b1;
    regs.wkup_ctrl.prescaler.q = 12'd0;
    regs.wkup_thold_hi.q = WakeThreshold[63:32];
    regs.wkup_thold_lo.q = WakeThreshold[31:0];
    regs.wkup_count_hi.q = wake_count[63:32];
    regs.wkup_count_lo.q = wake_count[31:0];
    regs.wdog_ctrl.enable.q = 1'b1;
    regs.wdog_bark_thold.q = BarkThreshold;
    regs.wdog_bite_thold.q = BiteThreshold;
    regs.wdog_count.q = watchdog_count;
  end

  always_ff @(posedge clk) begin
    if (reset) begin
      wake_count <= 64'd0;
      watchdog_count <= 32'd0;
    end else begin
      if (wake_count_we) wake_count <= wake_count_next;
      if (watchdog_count_we) watchdog_count <= watchdog_count_next;
    end
  end

  aon_timer_core core(
    .clk_aon_i(clk),
    .rst_aon_ni(~reset),
    .lc_escalate_en_i(12'haaa),
    .sleep_mode_i(1'b0),
    .reg2hw_i(regs),
    .wkup_count_reg_wr_o(wake_count_we),
    .wkup_count_wr_data_o(wake_count_next),
    .wdog_count_reg_wr_o(watchdog_count_we),
    .wdog_count_wr_data_o(watchdog_count_next),
    .wkup_intr_o(wake_bad),
    .wdog_intr_o(bark_bad),
    .wdog_reset_req_o(bite_bad)
  );

  always_comb begin
    assert (!wake_bad);
    assert (!bark_bad);
    assert (!bite_bad);
  end
endmodule
