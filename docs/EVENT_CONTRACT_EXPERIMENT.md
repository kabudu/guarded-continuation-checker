# Bounded event-contract experiment

This experiment asks whether GCC can preserve realistic bounded scheduler and
environment rules during CQ-SAT relation composition. Earlier predicate
transcripts were cubes: each named input was fixed to `0`, `1`, or left free.
A cube cannot directly express rules such as "at most one of these interrupts"
or "either recovery input may occur, but not both". Event-contract v1 adds a
strict named CNF language for those rules.

This is an experimental command and file format. It is not part of predicate
CLI v1, certificate v1/v2, or the released portfolio.

## Exact semantics

An event contract partitions a bounded horizon into contiguous phases. Each
phase carries CNF over the AIG's relevant named primary inputs. At frame `f`, an
input valuation is admissible exactly when it satisfies every clause belonging
to the phase covering `f`. A separate terminal CNF constrains inputs at the
terminal bad-output check.

CQ-SAT converts each CNF predicate to a BDD, projects the transition relation
through it, composes each phase relation by its declared length, and recovers a
concrete input/state trace when an avoiding execution exists. Independent replay
checks every frame against the original AIG, the phase predicate, the transition
function, and the terminal property.

The control path independently encodes the same per-frame CNF into exact CDCL.
A benchmark is published only when both answer paths agree and witness
replay succeeds.

## Canonical bounded format

```text
event_contract_version=1
horizon=8
phase_count=1
phase_0=0,8
phase_0_clause_count=2
phase_0_clause_0=irq[0]|irq[1]
phase_0_clause_1=!irq[0]|!irq[1]
terminal_clause_count=1
terminal_clause_0=!mask_override|irq[7]
```

`phase_N=start,length` entries must form a contiguous partition from frame zero
through `horizon`. Literals use exact AIG symbol names, `!` means negation, and
`|` separates literals in one clause. Conjunction is represented by multiple
clauses. The parser rejects unknown or ambiguous fields, unknown inputs,
duplicate clauses, duplicate literals, tautologies, empty clauses, noncanonical
line endings, symlinks, nonfiles, and inputs over 1 MiB. Bounds are 64 clauses
per predicate, 16 literals per clause, and 64 phases.

## Self-service command

```sh
guarded-continuation-checker benchmark-aiger-event-contract \
  INPUT.aag|INPUT.aig OUTPUT_INDEX CONTRACT.txt REPEATS OUTPUT.csv
```

The command accepts 1 to 100 repeats and refuses to overwrite output. It records
source and contract SHA-256 bindings, dimensions, compile cost, both query costs,
the logical result, answer agreement, and witness validity.

Three product-shaped contracts are retained under `examples/event-contracts/`:

- interrupt priority and mask interactions across two phases;
- actuator command mutual exclusion and interlock rules; and
- robot sensor, braking, and recovery interactions.

## Release-mode result

The experiment ran on 19 July 2026 using Rust 1.97.0 on Apple Silicon. Values
below are medians of ten raw query trials. Compile cost is paid once and shown
separately. All 30 rows returned `avoidable`, agreed with CDCL, and replayed a
valid witness.

| Contract | Horizon | Inputs | Latches | Clauses | Compile | CQ-SAT query | CDCL query | CQ/CDCL |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Interrupt priority | 8 | 9 | 2 | 8 | 0.589 ms | 0.208 ms | 0.190 ms | 1.09x slower |
| Actuator interlock | 16 | 12 | 3 | 10 | 0.306 ms | 2.971 ms | 0.269 ms | 11.03x slower |
| Robot recovery | 32 | 16 | 4 | 8 | 0.235 ms | 22.906 ms | 0.633 ms | 36.20x slower |

Raw evidence and reproduction instructions are in
[`results/event-contract-cnf-v1`](../results/event-contract-cnf-v1/README.md).

## What the result establishes

The experiment closes one semantic gap: bounded event constraints need not be
expanded into a list of individual allowed input assignments, and rules that
are not cubes survive exact phase composition and witness recovery.

It does not establish a performance advantage. The larger two rows are decisive
negative results, and even the smallest row is slightly slower than CDCL. The
current implementation must not be admitted universally. The next portfolio
slice requires a timing-free structural gate, exact CDCL fallback, and evidence
that the gate excludes these loss regimes.

## Prior-art boundary

CNF assumptions, bounded model checking, BDD transition relations, and witness
replay are established techniques. CBMC already supports bounded program
verification, proof harness assumptions, and function contracts that abstract
code using preconditions and postconditions. BTOR2 and AIGER already define
model and witness ecosystems for bounded hardware verification.

The candidate research question is therefore narrower: can a deterministic,
independently checkable artifact bind named bounded event contracts to powered
predicate relations, a static exact portfolio decision, and a replayed embedded
counterexample? This experiment does not answer that question because it emits
no event-contract certificate and has no portfolio integration.

Starting points:

- [CBMC documentation](https://diffblue.github.io/cbmc/)
- [CBMC code contracts](https://diffblue.github.io/cbmc/contracts-mainpage.html)
- [CBMC source and releases](https://github.com/diffblue/cbmc)
- [BTOR2, BtorMC and Boolector 3.0](https://fmv.jku.at/papers/NiemetzPreinerWolfBiere-CAV18.pdf)
- [AIGER format and witness ecosystem](https://fmv.jku.at/aiger/)

No novelty claim is made for this slice.

## Next falsifiable step

Define certificate v3 around the event-contract semantics, then require an
independent verifier to rebuild each CNF-constrained one-step obligation and
check deterministic phase composition without using the producer BDD. Integrate
it behind a predeclared structural gate with persistent-CDCL fallback. The gate
must be evaluated on unseen contracts and must report every rejected and losing
row. If it cannot exclude the measured loss regimes without timing calibration,
the specialised path remains an offline proof artifact rather than a production
portfolio backend.
