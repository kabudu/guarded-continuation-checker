# OpenTitan PWM channel trace maintained baseline v1

Date: 2026-07-22

The retained local comparison uses:

- Yosys commit `b8e7da6f40ae8f552c116bf6c359b07c6533e159`;
- Z3 4.16.0, 64 bit; and
- the authenticated OpenTitan PWM child source at commit
  `86db2898288664d8d5e8fc635b48951ef63e3439`.

Pinned Yosys independently compiles the authenticated symbolic-class harness
to one SMT transition system for each channel count. The baseline then asks Z3
every exact frame of every ordered trace query. This determines the first
reachable bad frame independently of GCC's monitor builder, solver portfolio,
proof producer, and proof verifier.

All 84 rows agree on SAFE or UNSAFE and on the earliest bad frame. The complete
retained result is
[`opentitan-pwm-trace-maintained-v1.csv`](opentitan-pwm-trace-maintained-v1.csv).

The generated SMT model digests are:

| Model | SHA-256 |
|---|---|
| symbolic-class-2.smt2 | `7a167c04e4eadd528c53c9cc74a19deb29763de0366cdad61cc3cd1d74152020` |
| symbolic-class-4.smt2 | `e33cf0f76e6ae6cff42e61b9c2339cc0fc1657f5998471cef540a908ab17b6d6` |
| symbolic-class-6.smt2 | `cf874f611c5b635b593ee875765a6f9b28cfdb987204cb3b4c2204b6a20b2063` |

Reproduce from a release build with:

```console
cargo build --release --locked --example btor2_channel_trace_cohort
scripts/run-opentitan-pwm-trace-maintained-baseline-v1.sh \
  /path/to/pinned/yosys \
  /path/to/z3-4.16.0 \
  target/release/examples/btor2_channel_trace_cohort \
  /new/output/directory
```

The script refuses tool-version drift and existing output paths. It compares
the independently generated rows with GCC and with the checked-in result before
publishing its output directory.

## Correctness finding and cost

The first maintained run falsified GCC's earlier earliest-frame assumption. A
horizon-wide SAT witness can be valid while reaching the property later than
another trace. The corrected trace artifact therefore carries a witness scoped
to the first bad frame plus a checked UNSAT certificate for the immediately
preceding horizon. That prefix certificate proves no earlier trace exists.

This stronger claim is expensive. The complete artifact sizes are 2,138,457,
4,007,729, and 4,899,434 bytes for 2, 4, and 6 channels. The mechanism is exact,
but proof-size reduction is now an open product requirement. No performance or
novelty advantage is claimed from this baseline.
