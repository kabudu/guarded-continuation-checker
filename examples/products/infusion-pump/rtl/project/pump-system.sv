module infusion_pump_system (
    input wire clock, input wire start, input wire stop, input wire door_open,
    input wire heartbeat, input wire sensor_a, input wire sensor_b, input wire sensor_c,
    output wire bad_illegal_mode, output wire bad_dose_wrap,
    output wire bad_watchdog_state, output wire bad_sensor_vote
);
    wire [1:0] mode;
    wire [3:0] dose;
    wire dose_seen;
    wire [2:0] watchdog_age;
    wire watchdog_tripped;
    wire voted_flow;
    wire sensor_disagreement;
    wire sensors_unanimous_high;
    wire commanded = mode == 2'b01;
    wire delivered_pulse = commanded & !door_open & voted_flow & !watchdog_tripped;

    command_sequencer sequencer(clock, start, stop, door_open, mode);
    dose_accounting accounting(clock, mode == 2'b10, delivered_pulse, dose, dose_seen);
    watchdog_timer watchdog(clock, commanded, heartbeat, watchdog_age, watchdog_tripped);
    sensor_voter voter(clock, sensor_a, sensor_b, sensor_c, voted_flow, sensor_disagreement, sensors_unanimous_high);

    assign bad_illegal_mode = mode == 2'b11;
    assign bad_dose_wrap = commanded & dose_seen & (dose == 4'b0000);
    assign bad_watchdog_state = watchdog_tripped & (watchdog_age != 3'b111);
    assign bad_sensor_vote = !sensor_disagreement & voted_flow & !sensors_unanimous_high;

`ifndef CQ_AIGER_EXPORT
    always @(*) begin
        assert (!bad_illegal_mode);
        assert (!bad_dose_wrap);
        assert (!bad_watchdog_state);
        assert (!bad_sensor_vote);
    end
`endif
endmodule
