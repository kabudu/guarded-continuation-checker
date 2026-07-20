module washing_controller_safe_oracle;
    wire fill_water;
    wire fault;
    wire water_intake;

    Controller controller (
        .clock(1'b0),
        .sig_Lid_Closed(1'b1),
        .sig_Coin(1'b1),
        .sig_Cancel(1'b0),
        .sig_Time_Out(1'b0),
        .sig_Out_Of_Balance(1'b0),
        .sig_Motor_Failure(1'b0),
        .sig_Full(1'b1),
        .sig_Temperature(1'b1),
        .sig_Wash_Completed(1'b1),
        .sig_Rinse_Completed(1'b1),
        .sig_Spin_Completed(1'b1),
        .fill_Water_Operation(fill_water),
        .fault(fault),
        .water_Intake(water_intake)
    );

    always @* begin
        assert (!(water_intake && fault));
    end
endmodule
