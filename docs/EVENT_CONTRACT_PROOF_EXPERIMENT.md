# Proof-carrying event-contract primitive

This experiment tests the highest-risk primitive needed for event-contract
certificate v3: independently checking exact one-step relations and terminal
safe-state sets under named CNF predicates without trusting the producer BDD.

It extends the proof construction used by predicate certificate v2. CNF event
rules replace cube constraints, but the evidence obligation remains dual:

- every claimed relation edge has a concrete declared-input witness;
- direct evaluation of the original AIG must take that edge and satisfy the CNF;
- one checked UNSAT proof per source state excludes every omitted target;
- every claimed terminal safe state has a concrete satisfying input; and
- one checked UNSAT proof excludes every omitted terminal safe state.

Concrete witnesses reject invented edges and safe states. Completeness proofs
reject omissions. The checker evaluates witnesses directly and rebuilds all SAT
obligations from the source model and parsed predicate. It does not call the BDD
producer while checking evidence.

This remains a feasibility experiment, not certificate v3, portfolio
integration, production support, or a novelty claim.

## Exact obligation encoding

For a source state `s`, CNF predicate `P`, and claimed target set `R(s)`, the
relation completeness obligation is:

```text
AIG transition encoding
AND current_state = s
AND P(declared inputs)
AND next_state not in R(s)
```

UNSAT proves that no admissible input reaches an omitted target. The terminal
obligation similarly asks for an admissible `(state, input)` outside the claimed
safe-state set where the selected bad output is false. UNSAT proves terminal
completeness.

The producer uses Varisat 0.2.2 native proof generation. The checker structurally
preflights the proof stream and validates it with `varisat-checker` 0.2.2. This
removes the BDD from the trusted answer path but does not yet provide
implementation diversity. The existing external CaDiCaL and DRAT-trim baseline
must be repeated for the new CNF obligations before v3 can enter a portfolio.

## Command

```sh
guarded-continuation-checker benchmark-aiger-event-contract-proofs \
  INPUT.aag|INPUT.aig OUTPUT_INDEX CONTRACT.txt REPEATS OUTPUT.csv
```

The command accepts 1 to 100 repeats and refuses overwrite. Schema v1 binds the
source and contract digests and records producer, proof-generation, and
proof-verification costs plus obligation, witness, and proof-byte counts.

## Release-mode result

The experiment ran on 19 July 2026 using Rust 1.97.0 on Apple Silicon. Values
are medians of ten raw trials.

| Contract | Inputs | States | Obligations | Witnesses | Producer | Proof generation | Proof verification | Proof bytes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Interrupt priority | 9 | 4 | 9 | 36 | 0.315 ms | 0.224 ms | 0.261 ms | 7,773 |
| Actuator interlock | 12 | 8 | 17 | 136 | 4.066 ms | 0.379 ms | 0.350 ms | 11,408 |
| Robot recovery | 16 | 16 | 33 | 528 | 28.907 ms | 1.053 ms | 1.051 ms | 33,936 |

All 30 trials generated and independently checked every obligation and replayed
every witness. A targeted adversarial regression removes a real relation target;
the resulting completeness formula becomes satisfiable and cannot produce an
UNSAT proof.

Raw evidence and reproduction instructions are retained under
[`results/event-contract-proof-v1`](../results/event-contract-proof-v1/README.md).

## Interpretation

The event-contract query experiment was 1.09x to 36.20x slower than exact CDCL,
so these results do not make CQ-SAT a universal solving backend. They establish
a different capability: exact evidence for a dense non-cube environmental
contract can be checked in 0.261 to 1.051 ms on this cohort, even when BDD
production itself is expensive.

This supports two distinct deployment paths:

1. admit CQ-SAT only where a timing-free structural rule predicts useful query
   reuse, with exact CDCL fallback everywhere else; and
2. use proof-carrying event summaries as offline assurance artifacts even where
   CDCL is the faster answer engine.

Neither path is production-ready yet.

## Remaining certificate-v3 gates

1. Freeze canonical bytes binding the source AIG, original named contract,
   predicates, relation witnesses, completeness proofs, powered rows, terminal
   set, answer, and optional concrete trace.
2. Parse with hard aggregate limits and reject symlinks, races, truncation,
   reordering, duplicate evidence, invalid UTF-8, proof swapping, and source or
   contract substitution.
3. Independently recompute every powered phase and final composition.
4. Cover avoidable and unavoidable answers, deterministic output, and semantic
   tampering across the complete three-product cohort.
5. Repeat the external proof-checker baseline for every v3 completeness
   obligation.
6. Add a stable API and exact fail-closed portfolio path only after reliability
   and unseen-contract selection gates pass.

No novelty claim is made for SAT proof carrying, CNF assumptions, relational
composition, or this primitive.
