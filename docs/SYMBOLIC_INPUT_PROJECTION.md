# Exact symbolic input projection

CQ-SAT/GCC's second firmware research cycle removes the original eight-input
ceiling when a controller declares a wide sensor bus but its transition and bad
outputs depend on a small semantic support.

For every AIG input, the compiler propagates an exact 64-bit dependency mask
through the topologically ordered AND graph. It unions the masks at every latch
next-state function and bad output, then constructs the explicit CIQ table over
only that projected support. Inputs outside the support are existentially free:
changing them cannot alter a successor state or bad-output value.

This is exact cone-support projection, not an approximate feature selector and
not a general symbolic representation for densely dependent input logic.

## Invariants and bounds

- Up to 64 inputs may be declared; at most eight may remain in the combined
  transition/property support.
- Support is derived from AIG dependencies, without timing calibration or
  training data.
- Causal candidates outside the proven support are removed before enumeration.
- CIQ and persistent CDCL replay the same remaining causal transcript.
- Recovered projected patterns are lifted into complete declared-input vectors,
  then the original AIG is independently re-evaluated at every frame.
- A support wider than eight fails closed before table construction.
- Existing state, horizon, table, cache, report, and causal-work bounds remain.

The projection currently unions every latch transition and every bad output.
Per-property and per-phase support could be smaller, but is intentionally
deferred until it can be independently certified.

## Controlled robotics result

The mobile-robot obstacle-stop regression declares 16 sensor and control
inputs. Its isolated transition/property cone depends on two:
`drive_enabled` and `obstacle_near`. Ten independent arm64 macOS trials per
horizon replayed every causal transcript 100 times against persistent CDCL.

| Horizon | Targets | Minimum | Median | Maximum |
| ---: | ---: | ---: | ---: | ---: |
| 8 | 9 | 2.35x | 2.46x | 2.53x |
| 16 | 17 | 3.85x | 4.00x | 4.08x |
| 32 | 33 | 6.61x | 6.76x | 7.07x |
| 64 | 65 | 9.67x | 10.74x | 11.80x |

All 1,240 published target rows agreed with fresh-CDCL verification and all
lifted witnesses validated against the complete 16-input AIG. The checked-in
summary is
[`symbolic-input-projection-scaling-v1.csv`](../results/symbolic-input-projection-scaling-v1.csv).

Regenerate it with:

```sh
scripts/run-symbolic-input-projection-scaling.sh \
  target/symbolic-input-projection-scaling
```

The harness refuses overwrite and verifies every raw report before aggregation.

## Interpretation and next boundary

This establishes a useful exact regime: wide firmware or robotics interfaces
whose individual safety property has a narrow input cone. It does not establish
an advantage for controllers whose transition/property support is itself wide.

The next boundary is predicate projection within a genuinely wide support. A
first bounded exact BDD prototype is documented in
[`DENSE_PREDICATE_QUOTIENT.md`](DENSE_PREDICATE_QUOTIENT.md): it removes input
enumeration, composes temporal relations, and recovers witnesses. Promotion
still requires a maintained symbolic model-checker baseline and broader
fixtures, with no per-formula calibration.
