module dense_actuator_interlock(
    input  wire       clk,
    input  wire [3:0] request,
    input  wire [3:0] limit_ok,
    input  wire       emergency_stop,
    input  wire       communications_ok,
    input  wire       thermal_ok,
    input  wire       service_key,
    output wire       unsafe_actuation
);
    reg [2:0] enabled = 3'b000;

    always @(posedge clk) begin
        if (emergency_stop || !communications_ok)
            enabled <= 3'b000;
        else begin
            enabled[0] <= (request[0] & limit_ok[0]) | (request[3] & service_key);
            enabled[1] <= (request[1] & limit_ok[1]) | (request[3] & thermal_ok);
            enabled[2] <= (request[2] & limit_ok[2]) | (request[3] & limit_ok[3]);
        end
    end

    assign unsafe_actuation = (&enabled) & (&request) & (&limit_ok)
        & ~emergency_stop & communications_ok & thermal_ok & service_key;
endmodule
