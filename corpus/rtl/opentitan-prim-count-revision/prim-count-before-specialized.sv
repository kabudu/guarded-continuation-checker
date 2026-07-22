// SPDX-License-Identifier: Apache-2.0
// Auditable specialization of OpenTitan prim_count at parent revision
// 34157c7afb84a7be7b1b1250d673f9fa8a3c18ce for Width=2,
// OutSelDnCnt=1 and CntStyle=CrossCnt.
module opentitan_prim_count_before (
  input  logic       clk_i,
  input  logic       rst_ni,
  input  logic       clr_i,
  input  logic       set_i,
  input  logic [1:0] set_cnt_i,
  input  logic       en_i,
  input  logic [1:0] step_i,
  output logic [1:0] cnt_o,
  output logic       err_o
);
  logic [1:0] up_cnt_q;
  logic [1:0] max_val;
  logic [1:0] down_cnt;
  logic       cmp_valid;
  logic [1:0] sum;
  logic       msb;

  always_ff @(posedge clk_i or negedge rst_ni) begin
    if (!rst_ni) begin
      up_cnt_q <= 2'b00;
      max_val <= 2'b11;
      down_cnt <= 2'b00;
      cmp_valid <= 1'b0;
    end else begin
      if (clr_i || set_i) begin
        up_cnt_q <= 2'b00;
      end else if (en_i && (up_cnt_q < max_val)) begin
        up_cnt_q <= up_cnt_q + step_i;
      end
      if (set_i) begin
        max_val <= set_cnt_i;
      end
      if (clr_i) begin
        down_cnt <= 2'b00;
      end else if (set_i) begin
        down_cnt <= set_cnt_i;
      end else if (en_i && (down_cnt > 2'b00)) begin
        down_cnt <= down_cnt - step_i;
      end
      if (clr_i) begin
        cmp_valid <= 1'b0;
      end else if (!cmp_valid && set_i) begin
        cmp_valid <= 1'b1;
      end
    end
  end

  assign {msb, sum} = down_cnt + up_cnt_q;
  assign cnt_o = down_cnt;
  assign err_o = cmp_valid ? ((max_val != sum) | msb) : 1'b0;
endmodule
