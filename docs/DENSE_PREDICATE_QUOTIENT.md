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
