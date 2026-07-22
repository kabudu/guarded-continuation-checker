module opentitan_aon_dual_timer_bounded_aiger #(
  parameter integer Horizon = 4,
  parameter logic [2:0] PropertyMask = 3'b111
) (
  input logic clk,
  input logic reset,
  output logic [63:0] observed_wake_count,
  output logic [31:0] observed_watchdog_count,
  output logic [3:0] observed_frame
);
  aon_timer_reg_pkg::aon_timer_reg2hw_t regs;
  logic [63:0] wake_count;
  logic [31:0] watchdog_count;
  logic wake_count_we;
  logic [63:0] wake_count_next;
  logic watchdog_count_we;
  logic [31:0] watchdog_count_next;
  logic wake_bad;
  logic bark_bad;
  logic bite_bad;
  logic [3:0] frame;

  initial wake_count = 64'd0;
  initial watchdog_count = 32'd0;
  initial frame = 4'd0;

  assign observed_wake_count = wake_count;
  assign observed_watchdog_count = watchdog_count;
  assign observed_frame = frame;

  always_comb begin
    regs = '0;
    regs.wkup_ctrl.enable.q = 1'b1;
    regs.wkup_ctrl.prescaler.q = 12'd0;
    regs.wkup_thold_hi.q = 32'd0;
    regs.wkup_thold_lo.q = 32'd7;
    regs.wkup_count_hi.q = wake_count[63:32];
    regs.wkup_count_lo.q = wake_count[31:0];
    regs.wdog_ctrl.enable.q = 1'b1;
    regs.wdog_bark_thold.q = 32'd5;
    regs.wdog_bite_thold.q = 32'd9;
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
    if (frame <= Horizon[3:0]) frame <= frame + 4'd1;
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

  if (PropertyMask[0]) begin : wake_property
    always_comb assert (!(frame <= Horizon[3:0] && wake_bad));
  end
  if (PropertyMask[1]) begin : bark_property
    always_comb assert (!(frame <= Horizon[3:0] && bark_bad));
  end
  if (PropertyMask[2]) begin : bite_property
    always_comb assert (!(frame <= Horizon[3:0] && bite_bad));
  end
endmodule
