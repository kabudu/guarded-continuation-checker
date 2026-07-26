# OpenTitan PWM channel-pair trace maintained baseline v1

Date: 2026-07-26

The retained comparison uses:

- Yosys commit `b8e7da6f40ae8f552c116bf6c359b07c6533e159`;
- Z3 4.16.0, 64 bit; and
- the authenticated OpenTitan PWM child source at commit
  `86db2898288664d8d5e8fc635b48951ef63e3439`.

Pinned Yosys independently compiles the authentic six-channel harness into one
SMT transition system. The baseline asks Z3 every exact frame of all twelve
ordered equality and difference histories. It constructs the channel relation
and temporal pattern independently of GCC's BTOR2 relation builder, structural
classes, solver selection, proof producer and verifier.

All twelve rows agree on SAFE or UNSAFE and on the earliest bad frame. The
cohort contains six SAFE and six UNSAFE results, including counterexamples at
frames zero and one. The retained rows are in
[`opentitan-pwm-channel-pair-trace-maintained-v1.csv`](opentitan-pwm-channel-pair-trace-maintained-v1.csv).

The generated `symbolic-class-6.smt2` SHA-256 is
`cf874f611c5b635b593ee875765a6f9b28cfdb987204cb3b4c2204b6a20b2063`.

Reproduce with:

```console
cargo build --release --locked \
  --example btor2_channel_pair_trace_cohort
scripts/run-opentitan-pwm-channel-pair-trace-maintained-baseline-v1.sh \
  /path/to/pinned/yosys \
  /path/to/z3-4.16.0 \
  target/release/examples/btor2_channel_pair_trace_cohort \
  /new/output/directory
```

The script refuses tool-version drift and existing or linked output paths. It
compares the independently generated rows with GCC and the checked-in result
before publishing its output directory.

## Finding

The maintained route confirms the narrow structural-constant result but does
not establish algorithmic novelty. Equality of channels admitted to the same
verified class is an expected consequence of congruence. The result closes an
independent semantic gate and strengthens the product capability only.
