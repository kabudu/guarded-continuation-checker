module dense_interrupt_arbiter(
    input  wire       clk,
    input  wire [7:0] irq,
    input  wire       mask_override,
    output wire       unsafe_nested_priority
);
    reg [1:0] active_class = 2'b00;

    always @(posedge clk) begin
        if (mask_override)
            active_class <= 2'b00;
        else if (|irq[7:6])
            active_class <= 2'b11;
        else if (|irq[5:4])
            active_class <= 2'b10;
        else if (|irq[3:2])
            active_class <= 2'b01;
        else if (|irq[1:0])
            active_class <= 2'b00;
    end

    assign unsafe_nested_priority = active_class[1] & active_class[0]
        & irq[7] & irq[6] & irq[5] & irq[4]
        & irq[3] & irq[2] & irq[1] & irq[0] & ~mask_override;
endmodule
