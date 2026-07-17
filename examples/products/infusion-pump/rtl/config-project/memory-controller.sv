`include "pump-widths.svh"

module infusion_pump_memory #(
    parameter DEPTH = 4
) (
    input  wire                          clk,
    input  wire                          rst_n,
    input  wire                          write_enable,
    input  wire [`PUMP_ADDRESS_BITS-1:0] address,
    input  wire [`PUMP_WORD_BITS-1:0]    write_data,
    output wire                          bad
);
    reg [`PUMP_WORD_BITS-1:0] memory [0:DEPTH-1];
    reg [`PUMP_WORD_BITS-1:0] read_data;
    reg [`PUMP_WORD_BITS-1:0] expected_data;
    reg [`PUMP_ADDRESS_BITS-1:0] expected_address;
    reg check_pending;
    integer index;

    initial begin
        read_data = 0;
        expected_data = 0;
        expected_address = 0;
        check_pending = 0;
        for (index = 0; index < DEPTH; index = index + 1)
            memory[index] = 0;
    end

    always @(posedge clk) begin
        if (!rst_n) begin
            read_data <= 0;
            check_pending <= 0;
        end else begin
            if (write_enable) begin
                memory[address] <= write_data;
                expected_data <= write_data;
                expected_address <= address;
                check_pending <= 1;
            end
            read_data <= memory[address];
        end
    end

    assign bad = (address >= DEPTH) ||
                 (check_pending && memory[expected_address] != expected_data);

`ifdef SBY
    reg reset_seen = 0;
    always @(posedge clk)
        reset_seen <= 1;

    always @* begin
        if (!reset_seen)
            assume(!rst_n);
        else
            assume(rst_n);
        assert(!bad);
    end
`endif
endmodule
