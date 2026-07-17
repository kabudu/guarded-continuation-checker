module corpus_always01_safe(input wire clock, input wire reset, output wire bad);
    wire [3:0] count;
    reg [3:0] expected;
    reg valid = 0;
    uut_always01 dut(clock, reset, count);
    always @(posedge clock) begin
        if (reset) begin expected <= 0; valid <= 1; end
        else if (valid) expected <= expected + 1;
    end
    assign bad = valid && !reset && count != expected;
`ifdef SBY
    reg formal_past_valid = 0;
    always @(posedge clock) formal_past_valid <= 1;
    always @* begin
        if (!formal_past_valid) assume(reset); else assume(!reset);
        assert(!bad);
    end
`endif
endmodule

module corpus_always01_unsafe(input wire clock, input wire reset, output wire bad);
    wire [3:0] count;
    reg valid = 0;
    uut_always01 dut(clock, reset, count);
    always @(posedge clock) if (reset) valid <= 1;
    assign bad = valid && !reset && count == 3;
`ifdef SBY
    reg formal_past_valid = 0;
    always @(posedge clock) formal_past_valid <= 1;
    always @* begin
        if (!formal_past_valid) assume(reset); else assume(!reset);
        assert(!bad);
    end
`endif
endmodule

module corpus_always02_safe(input wire clock, input wire reset, output wire bad);
    wire [3:0] count;
    reg [3:0] expected;
    reg valid = 0;
    uut_always02 dut(clock, reset, count);
    always @(posedge clock) begin
        if (reset) begin expected <= 0; valid <= 1; end
        else if (valid) expected <= expected + 1;
    end
    assign bad = valid && !reset && count != expected;
`ifdef SBY
    reg formal_past_valid = 0;
    always @(posedge clock) formal_past_valid <= 1;
    always @* begin
        if (!formal_past_valid) assume(reset); else assume(!reset);
        assert(!bad);
    end
`endif
endmodule

module corpus_always02_unsafe(input wire clock, input wire reset, output wire bad);
    wire [3:0] count;
    reg valid = 0;
    uut_always02 dut(clock, reset, count);
    always @(posedge clock) if (reset) valid <= 1;
    assign bad = valid && !reset && count == 4;
`ifdef SBY
    reg formal_past_valid = 0;
    always @(posedge clock) formal_past_valid <= 1;
    always @* begin
        if (!formal_past_valid) assume(reset); else assume(!reset);
        assert(!bad);
    end
`endif
endmodule

module corpus_dff0_safe(input wire clk, output wire bad);
    wire n0, n0_inv, n1, n1_inv;
    dff0_test dut0(n0, n0_inv, clk);
    dff1_test dut1(n1, n1_inv, clk);
    assign bad = n0 == n1;
`ifdef SBY
    always @* assert(!bad);
`endif
endmodule

module corpus_dff0_unsafe(input wire clk, output wire bad);
    wire n1, n1_inv;
    dff0_test dut(n1, n1_inv, clk);
    assign bad = n1;
`ifdef SBY
    always @* assert(!bad);
`endif
endmodule

module corpus_dff997_safe(input wire clk, input wire wire4, output wire bad);
    wire [1:0] y, inverse_y;
    reg valid = 0;
    dff_test_997 dut(y, clk, wire4);
    dff_test_997 inverse_dut(inverse_y, clk, !wire4);
    always @(posedge clk) valid <= 1;
    assign bad = valid && y == inverse_y;
`ifdef SBY
    always @* assert(!bad);
`endif
endmodule

module corpus_dff997_unsafe(input wire clk, input wire wire4, output wire bad);
    wire [1:0] y;
    reg valid = 0;
    dff_test_997 dut(y, clk, wire4);
    always @(posedge clk) valid <= 1;
    assign bad = valid && y != 0;
`ifdef SBY
    always @* assert(!bad);
`endif
endmodule

module corpus_retime_safe(input wire clk, input wire [7:0] a, output wire bad);
    wire z;
    reg [3:0] age = 0;
    retime_test dut(clk, a, z);
    always @(posedge clk) age <= age + 1;
    assign bad = age < 4 && !z;
`ifdef SBY
    always @* assert(!bad);
`endif
endmodule

module corpus_retime_unsafe(input wire clk, input wire [7:0] a, output wire bad);
    wire z;
    retime_test dut(clk, a, z);
    assign bad = !z;
`ifdef SBY
    always @* assert(!bad);
`endif
endmodule

module corpus_arrays_safe(
    input wire clock, input wire reset, input wire we,
    input wire [3:0] addr, input wire [3:0] wr_data, output wire bad
);
    wire [3:0] rd_data;
    reg [3:0] expected [15:0];
    reg [15:0] known = 0;
    reg [3:0] read_expected;
    reg read_known = 0;
    integer index;
    uut_arrays01 dut(clock, we, addr, wr_data, rd_data);
    always @(posedge clock) begin
        if (reset) begin
            known <= 0;
            read_known <= 0;
            for (index = 0; index < 16; index = index + 1) expected[index] <= 0;
        end else begin
            if (we) begin expected[addr] <= wr_data; known[addr] <= 1; end
            read_expected <= expected[addr];
            read_known <= known[addr];
        end
    end
    assign bad = read_known && rd_data != read_expected;
`ifdef SBY
    reg formal_past_valid = 0;
    always @(posedge clock) formal_past_valid <= 1;
    always @* begin
        if (!formal_past_valid) assume(reset); else assume(!reset);
        assert(!bad);
    end
`endif
endmodule

module corpus_arrays_unsafe(
    input wire clock, input wire reset, input wire we,
    input wire [3:0] addr, input wire [3:0] wr_data, output wire bad
);
    wire [3:0] rd_data;
    reg [15:0] written = 0;
    reg read_written = 0;
    uut_arrays01 dut(clock, we, addr, wr_data, rd_data);
    always @(posedge clock) begin
        if (reset) begin written <= 0; read_written <= 0; end
        else begin
            if (we) written[addr] <= 1;
            read_written <= written[addr];
        end
    end
    assign bad = read_written;
`ifdef SBY
    reg formal_past_valid = 0;
    always @(posedge clock) formal_past_valid <= 1;
    always @* begin
        if (!formal_past_valid) assume(reset); else assume(!reset);
        assert(!bad);
    end
`endif
endmodule
