module constrained_door_interlock;
    wire motor_request;
    wire door_open;
    wire bad;

    infusion_pump_controller controller(motor_request, door_open, bad);

    always @(*) begin
        assume (!door_open);
        assert (!bad);
    end
endmodule
