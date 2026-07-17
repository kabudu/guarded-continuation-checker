module infusion_pump_controller (
    input  wire motor_request,
    input  wire door_open,
    output wire bad
);
    reg requested_motor_active = 1'b0;

    always @($global_clock)
        requested_motor_active <= motor_request & ~door_open;

    wire delivered_motor_enable = requested_motor_active & ~door_open;
    assign bad = delivered_motor_enable & door_open;

`ifndef CQ_AIGER_EXPORT
    always @(*) assert (!bad);
`endif
endmodule
