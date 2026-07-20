// Repository-authored one-step actuator-delay plant for the baseline.
module WashingPlant(
    clock, fill_water, spin, controller_fault, water_intake,
    door_event, imbalance_event, motor_failure_event,
    sig_lid_closed, sig_coin, sig_cancel, sig_timeout,
    sig_out_of_balance, sig_motor_failure, sig_full, sig_temperature,
    sig_wash_completed, sig_rinse_completed, sig_spin_completed,
    bad_door_water, bad_overfill, bad_unbalanced_spin, bad_motor_spin,
    bad_fault_actuation, bad_conflicting_actions
);
    input clock, fill_water, spin, controller_fault, water_intake;
    input door_event, imbalance_event, motor_failure_event;
    output sig_lid_closed, sig_coin, sig_cancel, sig_timeout;
    output sig_out_of_balance, sig_motor_failure, sig_full, sig_temperature;
    output sig_wash_completed, sig_rinse_completed, sig_spin_completed;
    output bad_door_water, bad_overfill, bad_unbalanced_spin, bad_motor_spin;
    output bad_fault_actuation, bad_conflicting_actions;

    reg [1:0] water_level = 2'd0;
    reg cycle_tick = 1'b0;
    reg door_open = 1'b0;
    reg imbalance = 1'b0;
    reg motor_failed = 1'b0;
    wire applied_fill = fill_water && cycle_tick;
    wire applied_spin = spin && cycle_tick;
    wire applied_intake = water_intake && cycle_tick;

    always @(posedge clock) begin
        cycle_tick <= ~cycle_tick;
        door_open <= door_event;
        imbalance <= imbalance_event;
        motor_failed <= motor_failure_event;
        if (controller_fault)
            water_level <= 2'd0;
        else if (applied_intake && !applied_spin) begin
            if (water_level != 2'd3)
                water_level <= water_level + 1'b1;
        end else if (applied_spin && !applied_intake) begin
            if (water_level != 2'd0)
                water_level <= water_level - 1'b1;
        end
    end

    assign sig_lid_closed = !door_open;
    assign sig_coin = 1'b1;
    assign sig_cancel = 1'b0;
    assign sig_timeout = 1'b0;
    assign sig_out_of_balance = imbalance;
    assign sig_motor_failure = motor_failed;
    assign sig_full = (water_level == 2'd3);
    assign sig_temperature = water_level[1];
    assign sig_wash_completed = cycle_tick;
    assign sig_rinse_completed = cycle_tick;
    assign sig_spin_completed = (water_level == 2'd0);
    assign bad_door_water = door_open && applied_intake;
    assign bad_overfill = sig_full && applied_intake;
    assign bad_unbalanced_spin = imbalance && applied_spin;
    assign bad_motor_spin = motor_failed && applied_spin;
    assign bad_fault_actuation = controller_fault &&
        (applied_fill || applied_spin || applied_intake);
    assign bad_conflicting_actions = applied_fill && applied_spin;
endmodule
