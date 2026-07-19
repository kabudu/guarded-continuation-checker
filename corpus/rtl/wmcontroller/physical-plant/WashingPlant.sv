module WashingPlant (
    input wire clock,
    input wire fill_water,
    input wire heat_water,
    input wire wash,
    input wire rinse,
    input wire spin,
    input wire controller_fault,
    input wire water_intake,
    output wire sig_Lid_Closed,
    output wire sig_Coin,
    output wire sig_Cancel,
    output wire sig_Time_Out,
    output wire sig_Out_Of_Balance,
    output wire sig_Motor_Failure,
    output wire sig_Full,
    output wire sig_Temperature,
    output wire sig_Wash_Completed,
    output wire sig_Rinse_Completed,
    output wire sig_Spin_Completed,
    output wire bad_dry_heat,
    output wire bad_spin_with_water,
    output wire bad_uncommanded_water
);
    reg full = 1'b0;
    reg hot = 1'b0;
    reg washed = 1'b0;
    reg rinsed = 1'b0;
    reg spun = 1'b0;

    always @(posedge clock) begin
        if (controller_fault) begin
            full <= 1'b0;
            hot <= 1'b0;
            washed <= 1'b0;
            rinsed <= 1'b0;
            spun <= 1'b0;
        end else begin
            if (fill_water && water_intake)
                full <= 1'b1;
            if (full && heat_water)
                hot <= 1'b1;
            if (hot && wash)
                washed <= 1'b1;
            if (washed && rinse && water_intake)
                rinsed <= 1'b1;
            if (rinsed && spin && !water_intake)
                spun <= 1'b1;
            if (spun) begin
                full <= 1'b0;
                hot <= 1'b0;
                washed <= 1'b0;
                rinsed <= 1'b0;
                spun <= 1'b0;
            end
        end
    end

    assign sig_Lid_Closed = 1'b1;
    assign sig_Coin = 1'b1;
    assign sig_Cancel = 1'b0;
    assign sig_Time_Out = 1'b0;
    assign sig_Out_Of_Balance = 1'b0;
    assign sig_Motor_Failure = 1'b0;
    assign sig_Full = full;
    assign sig_Temperature = hot;
    assign sig_Wash_Completed = washed;
    assign sig_Rinse_Completed = rinsed;
    assign sig_Spin_Completed = spun;

    assign bad_dry_heat = heat_water && !full;
    assign bad_spin_with_water = spin && water_intake;
    assign bad_uncommanded_water = water_intake && !fill_water && !rinse;
endmodule
