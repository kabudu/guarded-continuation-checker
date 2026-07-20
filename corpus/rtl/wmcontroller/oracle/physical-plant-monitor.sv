module washing_controller_physical_plant_oracle (
    input wire door_event,
    input wire imbalance_event,
    input wire motor_failure_event
);
    wire fill_water;
    wire spin;
    wire controller_fault;
    wire water_intake;
    wire lid_closed;
    wire coin;
    wire cancel;
    wire timeout;
    wire out_of_balance;
    wire motor_failure;
    wire full;
    wire temperature;
    wire wash_completed;
    wire rinse_completed;
    wire spin_completed;
    wire bad_door_water;
    wire bad_overfill;
    wire bad_unbalanced_spin;
    wire bad_motor_spin;
    wire bad_fault_actuation;
    wire bad_conflicting_actions;

    Controller controller (
        .clock(1'b0),
        .sig_Lid_Closed(lid_closed),
        .sig_Coin(coin),
        .sig_Cancel(cancel),
        .sig_Time_Out(timeout),
        .sig_Out_Of_Balance(out_of_balance),
        .sig_Motor_Failure(motor_failure),
        .sig_Full(full),
        .sig_Temperature(temperature),
        .sig_Wash_Completed(wash_completed),
        .sig_Rinse_Completed(rinse_completed),
        .sig_Spin_Completed(spin_completed),
        .fill_Water_Operation(fill_water),
        .spin_Operation(spin),
        .fault(controller_fault),
        .water_Intake(water_intake)
    );

    WashingPlant plant (
        .clock(1'b0),
        .fill_water(fill_water),
        .spin(spin),
        .controller_fault(controller_fault),
        .water_intake(water_intake),
        .door_event(door_event),
        .imbalance_event(imbalance_event),
        .motor_failure_event(motor_failure_event),
        .sig_lid_closed(lid_closed),
        .sig_coin(coin),
        .sig_cancel(cancel),
        .sig_timeout(timeout),
        .sig_out_of_balance(out_of_balance),
        .sig_motor_failure(motor_failure),
        .sig_full(full),
        .sig_temperature(temperature),
        .sig_wash_completed(wash_completed),
        .sig_rinse_completed(rinse_completed),
        .sig_spin_completed(spin_completed),
        .bad_door_water(bad_door_water),
        .bad_overfill(bad_overfill),
        .bad_unbalanced_spin(bad_unbalanced_spin),
        .bad_motor_spin(bad_motor_spin),
        .bad_fault_actuation(bad_fault_actuation),
        .bad_conflicting_actions(bad_conflicting_actions)
    );

    always @* begin
        assert_door_water: assert (!bad_door_water);
        assert_overfill: assert (!bad_overfill);
        assert_unbalanced_spin: assert (!bad_unbalanced_spin);
        assert_motor_spin: assert (!bad_motor_spin);
        assert_fault_actuation: assert (!bad_fault_actuation);
        assert_conflicting_actions: assert (!bad_conflicting_actions);
    end
endmodule
