# Proof-carrying one-step relation experiment

This experiment tests candidate certificate-v2 primitives that replace
exhaustive enumeration of every projected input assignment with independently
checked SAT and UNSAT obligations for the exact one-step state relation and
terminal safe-state set.

It is a strong performance result on a bounded synthetic/product-shaped cohort,
not yet a certificate-v2 format, production claim, or scholarly novelty claim.

## Exact obligation

For each source state `s`, the BDD producer claims a target set `R(s)`.

- For every `t` in `R(s)`, the producer supplies a concrete declared-input
  witness. The checker evaluates the original AIG directly and requires that the
  constraints hold and `next(s, input) = t`.
- To prove completeness, the producer constructs the original one-step AIG
  Tseitin CNF with `state = s`, the input constraints, and one exclusion clause
  `next != t` for every `t` in `R(s)`. It emits a native Varisat UNSAT proof.
- The independent `varisat-checker` loads the rebuilt CNF and checks the proof.
  UNSAT establishes that no allowed input reaches a target outside `R(s)`.

Witnesses prevent extra edges; the UNSAT proof prevents omitted edges. Powered
phase composition remains deterministic arithmetic over the checked base
relation.

For the terminal set, every claimed safe state carries a concrete input witness.
One completeness formula requires a state outside the claimed set and an allowed
input that makes the selected bad output false. Its checked UNSAT proof shows
that no safe state was omitted.

The producer and checker are distinct components but currently come from the
same Varisat 0.2.2 release family. This is a smaller trust path than trusting the
BDD producer, but external proof-format/checker diversity remains an open gate.

## Adversarial checks

The regression suite:

- compares the proof-derived relation with the existing exhaustive direct-AIG
  checker at 9, 12 and 16 relevant inputs;
- rejects a truncated native proof; and
- removes a real claimed target and confirms that the completeness obligation
  becomes satisfiable, so no UNSAT proof can be produced.
- compares terminal proof sets against exhaustive checking under unconstrained
  and fully constrained inputs; and
- removes a real safe state and confirms the terminal obligation becomes
  satisfiable.

## Benchmark

```sh
continuation-quotient-sat benchmark-aiger-predicate-proof-relation \
  INPUT.aag|INPUT.aig REPEATED_PHASE_TRANSCRIPT.txt REPEATS OUTPUT.csv
```

Schema v1 preserves every trial. It records BDD relation production, witness
and proof generation, independent proof checking, exhaustive checking,
exhaustive evaluation count, witness count, proof bytes and exact agreement.
The transcript must contain one repeated constraint phase. The command accepts
1–100 repeats and refuses to overwrite output.

Terminal obligations have a separate answer-preserving schema:

```sh
continuation-quotient-sat benchmark-aiger-predicate-proof-terminal \
  INPUT.aag|INPUT.aig OUTPUT_INDEX REPEATED_PHASE_TRANSCRIPT.txt REPEATS OUTPUT.csv
```

## Release result

Release-mode results were measured on 18 July 2026. Values are medians of ten
raw trials.

| Model | Inputs | States | Producer | Proof generation | Proof checking | Exhaustive checking | Checker speedup | Witnesses | Proof bytes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Interrupt | 9 | 4 | 0.082 ms | 0.219 ms | 0.114 ms | 0.296 ms | 2.59x | 16 | 3,542 |
| Actuator | 12 | 8 | 0.231 ms | 1.876 ms | 0.164 ms | 3.753 ms | 22.88x | 64 | 5,418 |
| Sensor fusion | 16 | 16 | 0.440 ms | 8.841 ms | 0.540 ms | 151.417 ms | 280.32x | 256 | 20,894 |

Raw evidence:

- [`predicate-proof-relation-interrupt-v1.csv`](../results/predicate-proof-relation-interrupt-v1.csv)
- [`predicate-proof-relation-actuator-v1.csv`](../results/predicate-proof-relation-actuator-v1.csv)
- [`predicate-proof-relation-sensor-v1.csv`](../results/predicate-proof-relation-sensor-v1.csv)

## Terminal result

Terminal proofs are intentionally reported separately. On unconstrained cases
where the first input tried makes every state safe, exhaustive checking is
already faster. Under constrained inputs, the old checker still scans the full
projected space to find the sole allowed pattern or establish an unsafe state.

| Model/constraints | Safe states | Proof generation | Proof checking | Exhaustive checking | Checker speedup | Proof bytes |
|---|---:|---:|---:|---:|---:|---:|
| Interrupt/unconstrained | 4/4 | 0.027 ms | 0.025 ms | 0.001 ms | 0.03x | 695 |
| Actuator/unconstrained | 8/8 | 0.026 ms | 0.020 ms | 0.001 ms | 0.05x | 536 |
| Sensor/unconstrained | 16/16 | 0.033 ms | 0.031 ms | 0.002 ms | 0.06x | 959 |
| Interrupt/constrained | 4/4 | 0.028 ms | 0.029 ms | 0.003 ms | 0.09x | 1,271 |
| Actuator/constrained | 7/8 | 0.026 ms | 0.025 ms | 0.032 ms | 1.26x | 1,031 |
| Sensor/constrained | 16/16 | 0.034 ms | 0.038 ms | 0.997 ms | 26.20x | 1,443 |

All six ten-trial CSVs are retained under `results/` with the
`predicate-proof-terminal-*-v1.csv` prefix. The negative unconstrained rows
show that terminal proofs are an assurance mechanism, not a universal speed
optimisation. Even their worst measured proof-check cost is only 0.038 ms.

Reproduce into a new directory:

```sh
./scripts/benchmark-predicate-proof-relations.sh /tmp/cq-proof-relations 10
```

## Interpretation

The proof checker removes the exponential dependence on projected-input count
from relation reconstruction on this cohort. At 16 inputs it checks 16 source
proofs and 256 direct witnesses instead of evaluating 1,048,576 state/input
pairs. Verification improves by 280.32x while evidence grows to 20.9 KiB.

Generation is slower than a bare BDD query, but certificate generation already
has a separate evidence cost. The useful result is that independent checking no
longer dominates CDCL by two orders of magnitude: relation-proof checking is
approximately 0.54 ms on the case where certificate-v1 checking was about
136 ms.

## Remaining v2 work

Before this can replace certificate v1:

1. freeze a canonical, bounded proof bundle and source/obligation binding;
2. verify every powered phase from checked one-step relations;
3. add proof-count, aggregate-size, clause-count and wall-clock limits;
4. reject proof swapping, reordering, truncation and decompression/resource
   attacks;
5. compare a standard externally checkable proof format and maintained checker;
6. rerun full certificate and portfolio costs for both answer classes.

Until those close, v1 remains the portfolio evidence format.
