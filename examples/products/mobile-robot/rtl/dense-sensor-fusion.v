module dense_sensor_fusion(
    input  wire       clk,
    input  wire [7:0] obstacle,
    input  wire [3:0] quality,
    input  wire [2:0] motion_enable,
    input  wire       recovery_mode,
    output wire       unsafe_motion_consensus
);
    reg [3:0] fused_mode = 4'b0000;

    always @(posedge clk) begin
        if (recovery_mode)
            fused_mode <= 4'b0000;
        else begin
            fused_mode[0] <= (obstacle[0] | obstacle[1]) & quality[0];
            fused_mode[1] <= (obstacle[2] | obstacle[3]) & quality[1];
            fused_mode[2] <= (obstacle[4] | obstacle[5]) & quality[2];
            fused_mode[3] <= (obstacle[6] | obstacle[7]) & quality[3];
        end
    end

    assign unsafe_motion_consensus = (&fused_mode) & (&obstacle) & (&quality)
        & (&motion_enable) & ~recovery_mode;
endmodule
