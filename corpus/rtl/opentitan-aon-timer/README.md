# OpenTitan AON watchdog bounded acceptance target

This corpus target carries a production-tagged OpenTitan always-on watchdog
core through a pinned SystemVerilog-to-BTOR2 export and GCC's proof-carrying
bounded portfolio.

The wrapper deliberately fixes one narrow product configuration:

- watchdog enabled;
- wake-up timer disabled;
- sleep mode disabled;
- lifecycle escalation held at OpenTitan's encoded `Off` value;
- the external register file reduced to the fields read by this core;
- one Boolean reset input retained as the only semantic input;
- a 32-bit watchdog counter and configurable bite threshold;
- `wdog_reset_req_o` exposed as the bad property.

`watchdog-small.btor2` uses threshold 9. GCC proves SAFE through frame 8 with a
299-byte exact word-region certificate, then finds the reset-free counterexample
at frame 9 through exact explicit fallback. `watchdog-scale.btor2` uses threshold
4,000,000,000. GCC proves SAFE through frame 1,000,000,000 with a 326-byte
certificate while representing 500,000,001,500,000,001 logical reachable
states.

The separate `wrapper-predicate-set.sv` exposes both the watchdog bark and bite
outputs as ordered bad properties over the same counter. Its small model uses
thresholds 5 and 9. Through frame 4, a 324-byte shared recurrence certificate
proves both SAFE versus 598 bytes separately. At frame 5, bark is UNSAFE and
bite remains SAFE, so the static portfolio preserves both answers through exact
fallback. Its scale model uses thresholds 2,000,000,000 and 4,000,000,000 and
proves both SAFE through frame 1,000,000,000 with a 360-byte shared certificate
versus 652 bytes separately.

Run the self-service acceptance from the repository root:

```sh
mkdir /tmp/gcc-opentitan-models
scripts/build-opentitan-aon-watchdog-btor2.sh \
  "$(command -v yosys)" /tmp/gcc-opentitan-models
scripts/run-opentitan-aon-watchdog-acceptance.sh \
  target/debug/guarded-continuation-checker \
  "$(command -v yosys)" \
  /tmp/opentitan-aon-watchdog.csv
scripts/run-opentitan-aon-predicate-set-acceptance.sh \
  target/debug/guarded-continuation-checker \
  "$(command -v yosys)" \
  /tmp/opentitan-aon-predicate-set.csv
```

The build intentionally refuses a different source digest, a different Yosys
revision, symlink output directories, and overwrites. The acceptance script
reproduces all four models and all six certificates byte for byte, independently
verifies each certificate, and runs source, recogniser, overwrite, invalid
observation, output-path, query-binding, member-integrity, and publication
hostile controls.

This is evidence for a bounded mechanism exercised on real public RTL. It is not
a verification of the complete OpenTitan AON timer, the Earlgrey product, its
register generator, clocking, resets, lifecycle system, or physical integration.
See [PROVENANCE.md](PROVENANCE.md) for the exact source boundary.
