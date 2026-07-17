module command_sequencer (
    input  wire       clock,
    input  wire       start,
    input  wire       stop,
    input  wire       door_open,
    output reg  [1:0] mode = 2'b00
);
    always @(posedge clock) begin
        case (mode)
            2'b00: if (start && !door_open) mode <= 2'b01;
            2'b01: if (stop || door_open) mode <= 2'b10;
            2'b10: mode <= 2'b00;
            default: mode <= 2'b00;
        endcase
    end
endmodule

module dose_accounting (
    input  wire       clock,
    input  wire       clear,
    input  wire       pulse,
    output reg  [3:0] dose = 4'b0000,
    output reg        dose_seen = 1'b0
);
    always @(posedge clock) begin
        if (clear) begin
            dose <= 4'b0000;
            dose_seen <= 1'b0;
        end else if (pulse && dose != 4'b1111) begin
            dose <= dose + 1'b1;
            dose_seen <= 1'b1;
        end
    end
endmodule

module watchdog_timer (
    input  wire       clock,
    input  wire       active,
    input  wire       heartbeat,
    output reg  [2:0] age = 3'b000,
    output reg        tripped = 1'b0
);
    always @(posedge clock) begin
        if (!active || heartbeat) begin
            age <= 3'b000;
            tripped <= 1'b0;
        end else if (age == 3'b110) begin
            age <= 3'b111;
            tripped <= 1'b1;
        end else if (!tripped) begin
            age <= age + 1'b1;
        end
    end
endmodule

module sensor_voter (
    input  wire clock,
    input  wire sensor_a,
    input  wire sensor_b,
    input  wire sensor_c,
    output reg  voted_flow = 1'b0,
    output reg  disagreement = 1'b0,
    output reg  unanimous_high = 1'b0
);
    wire majority = (sensor_a & sensor_b) | (sensor_a & sensor_c) | (sensor_b & sensor_c);
    always @(posedge clock) begin
        voted_flow <= majority;
        disagreement <= (sensor_a != sensor_b) | (sensor_a != sensor_c);
        unanimous_high <= sensor_a & sensor_b & sensor_c;
    end
endmodule

module infusion_pump_system (
    input  wire clock,
    input  wire start,
    input  wire stop,
    input  wire door_open,
    input  wire heartbeat,
    input  wire sensor_a,
    input  wire sensor_b,
    input  wire sensor_c,
    output wire bad_illegal_mode,
    output wire bad_dose_wrap,
    output wire bad_watchdog_state,
    output wire bad_sensor_vote
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

    // These are reachability properties, not combinational tautologies.  Their
    // safety follows from the cooperating modules' initialized transition rules.
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
