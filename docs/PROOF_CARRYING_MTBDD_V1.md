# Proof-carrying controller MTBDD v1 experiment

Status: experimental. The fixed representation and structural gates pass on the
first pinned public controller. Composition and product gates remain open.

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
but not yet a product or novelty breakthrough. Exact plant composition, strong
complete-artifact baselines, a maintained independent oracle, and closest-system
analysis remain open.

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
close the maintained-oracle or product-quality gates.

Reduced ordered decision diagrams, multi-terminal BDDs, AIGs, and equivalence
checking are established. No novelty claim is made without a closest-system
comparison showing a distinct reusable artifact or verification result.
