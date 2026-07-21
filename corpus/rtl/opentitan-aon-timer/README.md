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
thresholds 5 and 9. Predicate-set v2 proves both SAFE through frame 4 with a
348-byte shared exact certificate versus 598 bytes separately. At frame 5, one
357-byte artifact preserves bark UNSAFE at frame 5 and bite SAFE versus 517
bytes separately. At a billion-frame horizon, it records exact UNSAFE frames 5
and 9 in 384 bytes where the separate bounded search refuses both queries. The
scale model uses thresholds 2,000,000,000 and 4,000,000,000 and proves both SAFE
through frame 1,000,000,000 with a 384-byte shared exact certificate versus 652
bytes separately. Retained v1 artifacts remain verification fixtures.

`wrapper-dual-timer-predicate-set.sv` enables the unchanged core's wake-up and
watchdog paths together. Its pinned model contains a direct watchdog recurrence,
a zero-prescaler invariant, and a wake-up recurrence guarded by that invariant.
Properties 33, 37, and 41 reach their first bad frames at 9, 5, and 7
respectively. This is the predeclared public target for GCC's multi-recurrence
invariant-chaining experiment. The current v2 portfolio refuses the complete
query rather than treating the guarded wake-up property as a one-state case.

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
scripts/build-opentitan-aon-dual-timer-btor2.sh \
  "$(command -v yosys)" /tmp/opentitan-aon-dual-timer.btor2
```

The build intentionally refuses a different source digest, a different Yosys
revision, symlink output directories, and overwrites. The acceptance script
reproduces all four watchdog models and the current plus compatibility
certificates byte for byte, independently verifies each certificate, and runs
source, recogniser, overwrite, invalid-observation, output-path, query-binding,
member-integrity, and publication hostile controls. The separate dual-timer
builder currently reproduces its pre-implementation model probe.

This is evidence for a bounded mechanism exercised on real public RTL. It is not
a verification of the complete OpenTitan AON timer, the Earlgrey product, its
register generator, clocking, resets, lifecycle system, or physical integration.
See [PROVENANCE.md](PROVENANCE.md) for the exact source boundary.
