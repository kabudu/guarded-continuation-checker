# Dense predicate quotient experiment

The dense predicate quotient is an exact bounded experiment for AIGER models
whose transition/property interface depends on 9–16 primary inputs. It compiles
the AIG into a reduced ordered BDD over input, current-latch, and next-latch
variables. Existential input projection produces a relation over at most four
latches; repeated temporal phases are composed by exact relation powers. A
reported avoiding trace is reconstructed frame by frame and replayed against
the original AIG.

This removes the earlier `2^inputs` table. The 16-input dense sensor-consensus
fixture has 65,536 concrete input patterns but compiles to 159 BDD nodes under
the fixed ordering. Bounds are fail closed: 9–16 relevant inputs, 1–4 latches,
100,000 BDD nodes, horizon 64, and bounded relation/witness caches.

## Controlled result

The first experiment compares safe-input witness queries with persistent CDCL
on the same 16-input constraints. Ten trials at each reuse count produced:

| Reuses | Minimum | Median | Maximum |
|---:|---:|---:|---:|
| 1 | 0.468x | 0.497x | 0.547x |
| 10 | 0.622x | 0.652x | 0.726x |
| 100 | 1.658x | 1.834x | 1.904x |
| 1,000 | 4.211x | 4.357x | 4.410x |

The result is deliberately mixed: compilation and uncached BDD traversal lose
at low reuse, while exact bounded memoisation wins once the same predicate is
queried repeatedly. This is static workload amortisation, not a general solver
speedup and not an admission rule based on per-formula timing.

The temporal prototype also powers a 32-frame dense relation and reconstructs
an exact avoiding trace. It is available as:

```sh
continuation-quotient-sat query-aiger-predicate-quotient \
  INPUT.aag 32 0
```

## Evidence boundary

This cycle establishes compact exact representation, temporal composition, and
witness recovery on controlled fixtures. The
`benchmark-aiger-predicate-symbolic` command runs the identical existential
bounded query through maintained Yosys as an external agreement and workload
baseline. Its process-level timing includes Yosys startup and must not be
presented as an in-process query comparison. On ten 32-frame trials with ten
queries per trial, all answers agreed and all CQ-SAT witnesses replayed; the
end-to-end Yosys/predicate time ratio ranged from 262.30x to 407.55x, with a
394.73x median. Most of this gap is process and generic-model setup overhead.
Broad controller generality and scholarly novelty remain release gates for
promoting this backend into the default CQ-SAT/GCC portfolio.

## State-dependent product matrix

Three separately authored RTL controllers exercise interrupt arbitration (9
relevant inputs, 2 latches), actuator interlocks (12 inputs, 3 latches), and
mobile-robot sensor fusion (16 inputs, 4 latches). Synthesis retains one clock
input outside the exact support. Regression tests prove the declared/relevant
widths and show that each safety predicate is observably latch-state dependent.

The benchmark fixes every relevant input at every frame and releases one input
only at the terminal frame. CQ-SAT/GCC and persistent CDCL therefore solve the
same constrained temporal transcript; maintained Yosys independently confirms
each existential result. Ten trials, 100 in-process queries per trial:

| Controller | Inputs | H8 | H16 | H32 | H64 |
|---|---:|---:|---:|---:|---:|
| Interrupt arbiter | 9 | 1.54x | 1.92x | 2.15x | 2.35x |
| Actuator interlock | 12 | 0.97x | 1.28x | 1.48x | 1.60x |
| Sensor fusion | 16 | 0.81x | 1.05x | 1.21x | 1.40x |

These are median end-to-end ratios versus persistent CDCL, including each
backend's compilation/setup. The actuator H8 and sensor-fusion H8/H16 rows do
not establish a robust win. The exact raw summary is
[`dense-predicate-product-matrix-v1.csv`](../results/dense-predicate-product-matrix-v1.csv)
and can be regenerated with `scripts/run-dense-predicate-product-matrix.sh`.

## Static admission boundary

The prototype does not time both backends and choose the winner. Its
predeclared conservative gate requires at least 100 expected queries, 1–4
latches, and:

- 9–10 relevant inputs: horizon at least 8;
- 11–13 relevant inputs: horizon at least 16;
- 14–16 relevant inputs: horizon at least 32.

This excludes every robustly negative matrix row. It is intentionally based on
the controlled evidence rather than claimed as a universal performance model;
unsupported or rejected workloads retain persistent CDCL.

Exactness tests cover both directions. Avoidable transcripts must produce an
original-AIG-valid concrete trace. Separately, each product fixture is solved
for an unsafe trace, its complete projected input transcript is fixed, and
CQ-SAT/GCC, persistent CDCL, and Yosys must all report that avoiding the bad
terminal state is impossible.
