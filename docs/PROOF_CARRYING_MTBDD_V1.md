# Proof-carrying controller MTBDD v1 experiment

Status: experimental. The fixed representation, structural gates, exact
composition, complete-artifact reuse baseline, and maintained bounded oracle
pass on the first pinned public controller.

## Hypothesis and fixed order

A reduced ordered multi-terminal binary decision diagram can preserve the exact
joint next-state and selected-output function while sharing suffix decisions
across controller states and sensed inputs.

The variable order is fixed as controller-state bits from least to most
significant, followed by selected sensed inputs in manifest order. It is not
learned, timed, or selected per formula. A 240-node interleaved order observed
during exploration is deliberately not selected. The frozen state-first order
measured 254 nodes, compared with 1,028 global cube leaves and 16,109
state-conditioned cube leaves.

## Static gates

- at most 6 state bits and 12 selected sensed inputs;
- at most 512 non-terminal nodes and 1,024 distinct outcome terminals;
- at most 131,072 exhaustive state/input assignments;
- exact complete next-state and up to 4 selected outputs for every assignment;
- source and boundary binding;
- deterministic canonical encoding no larger than 1 MiB;
- independent structural validation and exhaustive source equivalence checking;
- complete retained truncation and one-bit mutation rejection;
- exact composition and direct-controller agreement for both answers;
- complete repeated-artifact and shared-artifact baselines; and
- static fallback when any gate is exceeded.

The MTBDD is the checkable functional certificate. V1 uses exhaustive bounded
equivalence checking rather than claiming a separately logged SAT proof. A
later global miter proof is required before this can be described as a
proof-logged artifact.

## First result

The fixed state-first representation passes the structural and exactness gates
on the pinned public washing controller:

- 131,072 assignments checked;
- 153 exact outcome terminals;
- 254 non-terminal decision nodes;
- 6,217 encoded bytes;
- 479,660,375 ns production; and
- 475,873,750 ns independent exhaustive verification.

The retained times are one reference run and are not selection inputs. The
artifact round-trips canonically, binds the source and boundary, and rejects
every truncation and one-bit mutation in the downstream API test. Reproduce the
machine-readable row with:

```sh
cargo run --release --example public_washing_controller_mtbdd
```

This is a real representation breakthrough relative to both cube vocabularies,
but not yet a product or novelty breakthrough. Representative physical plant
composition, closest-system analysis, and independent external evaluation
remain open.

## First exact compositions

The verified public-controller MTBDD now composes with two separately supplied
repository-authored appliance monitors through the public Rust API:

- a water/fault exclusion monitor remains SAFE through horizon 32; and
- a fill-only water-valve monitor becomes UNSAFE when the controller reaches
  rinse and requests water without the fill-mode output.

Both complete results, including the unsafe frame-10 trace, exactly equal an
independent direct-controller baseline. The direct baseline omits the synthesized
clock port only after exhaustively proving that every omitted-input value has the
same next-state and selected-output outcome for every retained state and sensor
pattern. This avoids treating an assumed-constant clock input as evidence.

These minimal monitors exercise both answers and the real public controller
boundary. They are not yet representative physical plant models and do not
close the representative-environment or product-quality gates.

## Maintained model-checker oracle

Pinned SymbiYosys, maintained Yosys, and Z3 now check the same generated AIGER
transition, fixed sensor pattern, initial state, horizon, and two monitor
properties independently of the Rust implementation. The water/fault exclusion
property passes through depth 32. The fill-only water-valve property fails at
step 10, exactly matching GCC's shortest bad frame.

The synthesis recipe explicitly resolves the upstream controller's
uninitialised `next_State` register before AIGER generation. The oracle reads
the generated AIGER model without overriding latch initialisation, so GCC and
the formal jobs use the same zero-initialised state interpretation. An earlier
model that left these latches nondeterministic produced a different result and
was rejected rather than reported as agreement. The retained machine-readable
result is `results/public-washing-controller-mtbdd-oracle-v1.csv`; reproduce it
with:

```sh
scripts/test-public-washing-controller-oracle.sh /path/to/sby.py
```

The script verifies pinned file digests and regenerates the AIGER model byte for
byte before both formal jobs. This supplies a separate solver and checking path,
but does not independently validate Yosys synthesis because Yosys both produces
and imports the AIGER model in this path.

## Complete-artifact reuse baseline

The strong baseline independently verifies one complete source-bound MTBDD
plant artifact per member. The shared path verifies one complete artifact with
one MTBDD and all ordered source-bound member results. Three interleaved
release-mode trials produce these medians:

| Members | Shared/repeated bytes | Shared/repeated checking time |
|---:|---:|---:|
| 1 | 1.000 | 0.998 |
| 2 | 0.523 | 0.499 |
| 4 | 0.285 | 0.251 |
| 8 | 0.166 | 0.126 |
| 16 | 0.106 | 0.062 |

All member answers and complete results agree. At 16 members, the shared
artifact is 89.4% smaller and verification is 93.8% faster than independently
checking repeated complete artifacts. Verification remains approximately 0.48
seconds because the source equivalence check is performed once per shared
artifact. Timings are observations, not portfolio inputs.

Reproduce with:

```sh
cargo run --release --example public_washing_controller_mtbdd_reuse_benchmark
```

The canonical batch codec binds every plant digest, wiring vector, initial
state, property, horizon, and complete result. The downstream API test rejects
every truncation and one-bit mutation of its retained artifact and rejects
reordered source digests.

Reduced ordered decision diagrams, multi-terminal BDDs, AIGs, and equivalence
checking are established. No novelty claim is made without a closest-system
comparison showing a distinct reusable artifact or verification result.
