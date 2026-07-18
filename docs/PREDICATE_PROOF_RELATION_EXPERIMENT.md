# Proof-carrying one-step relation experiment

This experiment tests a candidate certificate-v2 primitive: replace exhaustive
enumeration of every projected input assignment with independently checked SAT
and UNSAT obligations for the exact one-step state relation.

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
2. add equivalent proof obligations for terminal safe-state completeness;
3. verify every powered phase from checked one-step relations;
4. add proof-count, aggregate-size, clause-count and wall-clock limits;
5. reject proof swapping, reordering, truncation and decompression/resource
   attacks;
6. compare a standard externally checkable proof format and maintained checker;
7. rerun full certificate and portfolio costs for both answer classes.

Until those close, v1 remains the portfolio evidence format.
