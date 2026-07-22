module opentitan_prim_count_query_component (
  input  logic       clk,
  input  logic       reset_n,
  input  logic       clear,
  input  logic       set,
  input  logic [1:0] set_count,
  input  logic       enable,
  input  logic [1:0] step,
  output logic       count_eq_0,
  output logic       count_eq_1,
  output logic       count_eq_2,
  output logic       count_eq_3,
  output logic       error,
  output logic       count_bit_0,
  output logic       count_bit_1,
  output logic       count_parity
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

  assign count_eq_0 = count == 2'b00;
  assign count_eq_1 = count == 2'b01;
  assign count_eq_2 = count == 2'b10;
  assign count_eq_3 = count == 2'b11;
  assign count_bit_0 = count[0];
  assign count_bit_1 = count[1];
  assign count_parity = count[0] ^ count[1];
endmodule
