module opentitan_prim_count_revision_impact_environment_oracle (
  input logic observed_full
);
  // environment-wrapper.sv defines observed_bad as exactly observed_full.
  // This assertion exports that unchanged public-cohort obligation to AIGER.
  always_comb assert (!observed_full);
endmodule
