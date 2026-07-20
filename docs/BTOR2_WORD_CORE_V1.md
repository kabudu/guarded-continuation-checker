# BTOR2 word semantic core v1

This experimental core preserves bounded firmware counters and timers as
word-level BTOR2 expressions before any future SAT lowering. It is a semantic
and hostile-input trust boundary, not a solver, certificate, production API, or
novelty result.

## Supported boundary

Version 1 accepts canonical newline-terminated BTOR2 text containing:

- bit-vector sorts of width 1 to 64;
- input and state nodes;
- standard `output` observation statements, validated but not treated as bad
  properties;
- `zero`, `one`, `ones`, binary, decimal, and hexadecimal constants;
- word-level Boolean operations, modular add, subtract, and multiply;
- unsigned equality and order comparisons;
- `ite`, `slice`, and zero extension;
- exactly one constant initialiser and next expression per state;
- Boolean constraints; and
- one or more bad properties.

Optional symbols on ordinary expression nodes are accepted. `Btor2Model::inputs`
returns only declared inputs that can reach a next-state expression, constraint,
or bad property. Unused synthesis artifacts such as a clock consumed only by the
BTOR transition convention are validated during parsing but omitted from the
semantic input vector. This support calculation does not remove any live node or
alter model semantics.

The parser requires unique strictly increasing identifiers, prior definitions,
exact operand and result widths, canonical constants, at most 8 MiB, 100,000
lines, 100,000 expression nodes, and valid UTF-8 without NUL or carriage-return
bytes. The CLI opens a regular file once, rejects symbolic links on Unix, bounds
the open file, and parses only the bytes read from that descriptor.

Arrays, signed operations, division, reduction, overflow predicates, fairness,
justice, liveness, and every unlisted operation are rejected. They are never
silently bit-blasted, approximated, or ignored.

## Semantics and example

All arithmetic wraps at the declared width. Comparisons return a one-bit word.
Constraints are checked before transitions and property observations. Missing
inputs, states, initialisers, or next expressions are errors.

Inspect the included eight-bit watchdog timer:

```sh
cargo run --release -- btor2-cli-version
cargo run --release -- \
  inspect-btor2 examples/btor2/watchdog-counter-v1.btor2
```

The model increments `timer` modulo 256 while `kick` is false, resets it to zero
when `kick` is true, and declares `expired` once the timer is at least three.
Library tests independently step this model, check reset and expiry, and verify
declared-width wraparound. CI also checks the model and its fixed expiry witness
with BTOR2Tools commit `d33c73ff1d173f1bfac8ba6b1c6d68ba62c55f8e`.

## Baselines and novelty boundary

BTOR2 is an established word-level model-checking format with official parser,
simulation, witness-checking, and historical BtorMC support. Boolector and
BtorMC are now archived; Boolector's repository identifies Bitwuzla as its
maintained successor. The required baselines are therefore:

- BTOR2Tools for syntax, parser, simulation, and witness agreement;
- historical BtorMC for bounded model-checking semantics; and
- maintained Bitwuzla for word-level bit-vector solving.

Primary references:

- [BTOR2, BtorMC and Boolector 3.0](https://fmv.jku.at/papers/NiemetzPreinerWolfBiere-CAV18.pdf)
- [BTOR2Tools](https://github.com/Boolector/btor2tools)
- [archived Boolector and BtorMC](https://github.com/Boolector/boolector)
- [maintained Bitwuzla](https://github.com/bitwuzla/bitwuzla)

Parsing BTOR2 and evaluating bit-vectors are established techniques. A possible
GCC candidate contribution would require a deterministic, independently checked
certificate that binds exact word-level phase composition to a source model,
resource gate, fail-closed exact fallback, and reconstructed firmware trace. No
such contribution is claimed by this core.

## Completed bounded gate and next boundary

Counter-phase certificate v1 provides a deterministic, source-bound format and
re-recognizes the admitted recurrence during verification. BTOR2Tools checks
sequential witnesses across the watchdog, actuator, and saturating cohort.
Bitwuzla checks endpoint formulas plus both watchdog bounded-search answers.
Exact replay preserves rejected supplied traces, and bounded search provides
both-answer fallback for the one-input, constraint-free subset. The
[exact word-region certificate v1](BTOR2_WORD_REGION_CERTIFICATE_V1.md) now
replaces explicit SAFE layers for two recognised counter families while proving
the same complete reachable set from source. The retained large SAFE artifacts
shrink by more than 99.9%.

The next gate must extend exact composition across interacting states or
multiple inputs without turning the proof back into explicit state enumeration.

The [OpenTitan AON watchdog experiment](OPENTITAN_AON_WATCHDOG_EXPERIMENT_V1.md)
now exercises standard Yosys observations, optional symbols, unused-clock
pruning, exact Boolean wrapper recognition, portfolio fallback, and independent
certificate checking on a production-tagged public RTL core. It closes only this
narrow source-to-proof integration gate.

The [coupled-motion curve certificate v1](BTOR2_MOTION_CURVE_CERTIFICATE_V1.md)
closes the first two-state subcase by preserving a velocity-position polynomial
relation under shared reset. Multi-input control, braking phases, signed words,
and broader interacting-state composition remain open.
