# Public washing-controller transducer experiment v1

Status: predeclared experiment. No admission or performance result is assumed.

## Public source

The candidate is `src/Controller.v` from
`yasnakateb/WMController` at immutable revision
`a81fadd25b07e3e415a57f997f7106f67e2fb24b`, licensed under GPL-2.0. The
upstream source must remain byte-identical. Repository-authored synthesis and
environment files must be identified separately.

The source describes an eight-state washing-machine controller driven by lid,
coin, cancellation, timeout, balance, motor, water-level, temperature, and
cycle-completion signals. It drives water intake, operational modes, fault, and
coin-return outputs.

## Fixed synthesis interpretation

Yosys 0.67 reads the unmodified Verilog, selects `Controller`, lowers processes,
flattens, optimises, replaces otherwise unspecified values with zero, maps to an
AIG, and emits ASCII AIGER. The `setundef -zero` step is a declared synthesis
interpretation required because upstream `next_State` has no initializer. It
must not be described as an upstream guarantee.

The measured structural boundary before the experiment is 6 latches, 12 AIGER
inputs including the source clock port, 14 outputs, and 122 AND gates. The clock
port is excluded from the selected sensed-input boundary. All 11 functional
inputs are retained.

## Static admission gates

- at most 6 controller latches and 64 explicit controller states;
- exactly the 11 functional sensed inputs, without learned selection;
- at most 4 selected actuator or mode outputs;
- at most 256 canonical cells, 4,096 row proofs, and 8 MiB of proof bytes;
- at most 300 seconds for production on the reference development machine;
- deterministic byte-identical production;
- independent source-bound verification;
- exact agreement with direct evaluation on every input pattern and state;
- no timing-based or formula-specific portfolio calibration; and
- fail-closed fallback if any bound is exceeded.

Admission fails if the canonical partition needs more than 64 cells, because
each cell requires one complete row for every one of the 64 controller states.
The limit will not be raised in response to this candidate's result.

If admitted, the next gate composes the same verified controller artifact with
at least two repository-authored physical environments, retains both SAFE and
UNSAFE outcomes, compares complete repeated and shared artifacts, and checks the
bounded result against a maintained Yosys plus SymbiYosys solver workflow.

This public-source experiment can strengthen practical evidence. It cannot by
itself establish algorithmic novelty or production readiness.

## Result

The candidate is rejected. Exhaustive exact evaluation finds:

- all 64 synthesized controller states reachable from state zero;
- 592 distinct complete response signatures across the 2,048 functional input
  patterns; and
- 1,028 leaves in the fixed false-before-true canonical cube partition.

This exceeds both the 256-cell limit and, because every cell has 64 state rows,
the 4,096-proof limit. Production stops at the cell gate and emits no artifact.
The retained machine-readable result is
`results/public-washing-controller-admission-v1.csv` and can be reproduced with:

```sh
cargo run --release --example public_washing_controller_admission
```

The negative result falsifies simple six-latch scaling for this public source.
Raising the cell or proof limits would violate the predeclared experiment. The
next admissible directions are a public controller with cleaner synchronous
state semantics or a new proof vocabulary that represents non-cube input
equivalence without enumerating 1,028 leaves.

The follow-up fixed-order MTBDD experiment takes the second route without
changing the failed cube gate. It represents the same 131,072 exact assignments
with 153 outcome terminals and 254 decision nodes in 6,217 bytes. Independent
exhaustive source checking passes. See
[proof-carrying controller MTBDD v1](PROOF_CARRYING_MTBDD_V1.md). The cube result
remains rejected; the MTBDD is a separate experimental backend.
