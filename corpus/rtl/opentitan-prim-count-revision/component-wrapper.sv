module opentitan_prim_count_revision_component (
  input  logic       clk,
  input  logic       reset_n,
  input  logic       clear,
  input  logic       set,
  input  logic [1:0] set_count,
  input  logic       enable,
  input  logic [1:0] step,
  output logic       count_is_full,
  output logic       error
);
  logic [1:0] count;

  `PRIM_COUNT_MODULE dut (
    .clk_i(clk),
    .rst_ni(reset_n),
    .clr_i(clear),
    .set_i(set),
    .set_cnt_i(set_count),
    .en_i(enable),
    .step_i(step),
    .cnt_o(count),
    .err_o(error)
  );

  assign count_is_full = &count;
endmodule
