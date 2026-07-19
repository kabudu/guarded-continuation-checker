# BTOR2 coupled-motion curve cohort v1

This experiment tests an exact non-Cartesian representation of two interacting
word states. The admitted models use a shared brake or stop input and the
simultaneous recurrence:

```text
velocity' = velocity + acceleration
position' = position + velocity
```

| Model | Horizon | Answer | Selected backend | Explicit bytes | Portfolio bytes | Reduction |
|---|---:|---|---|---:|---:|---:|
| Motion envelope | 200 | SAFE | motion-curve | 624,272 | 358 | 99.94% |
| Motion envelope | 201 | UNSAFE | explicit-search | 418 | 418 | 0.00% |
| Servo motion | 128 | SAFE | motion-curve | 253,928 | 358 | 99.86% |
| Servo motion | 129 | UNSAFE | explicit-search | 346 | 346 | 0.00% |
| Semi-implicit near-neighbour | 3 | SAFE | explicit-search | 518 | 518 | 0.00% |
| Semi-implicit near-neighbour | 4 | UNSAFE | explicit-search | 217 | 217 | 0.00% |

The exact curve uses one index `k`, the consecutive non-reset steps:

```text
velocity(k) = velocity(0) + acceleration*k
position(k) = position(0) + velocity(0)*k
              + acceleration*k*(k-1)/2
```

At frame `t`, every `k` from zero through `t` is reachable, and no other pair
is reachable. The bad predicate is a conjunction of unsigned velocity and
position thresholds. Both coordinates are monotone inside the admitted
non-wrapping boundary, so endpoint disjointness proves the entire bounded curve
safe.

The semi-implicit source instead adds the newly incremented velocity to
position. It is deliberately rejected because it has a different polynomial
curve. Both of its rows retain explicit exact search.

This is a product-shaped robotics recurrence, not an unmodified public robot
design or external acceptance result. Reproduce the retained CSV with:

```sh
cargo build
scripts/run-btor2-motion-cohort.sh \
  target/debug/guarded-continuation-checker /tmp/btor2-motion-cohort-v1.csv
diff -u results/btor2-motion-cohort-v1.csv \
  /tmp/btor2-motion-cohort-v1.csv
```
