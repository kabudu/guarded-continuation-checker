# Closest-method causal strategy comparison

This experiment separates the established conflict-minimisation part of causal
counterexample analysis from the CQ-SAT/GCC-specific continuation-quotient
reuse hypothesis. It is intentionally capable of disproving a speed claim.

## Prior-art boundary

The following ideas are established and are not claimed as CQ-SAT/GCC
inventions:

- Deletion-based extraction removes one observation at a time while preserving
  inconsistency. Modern work continues to optimise this established algorithm
  family; it is not novel here.
- [QuickXplain](https://cdn.aaai.org/AAAI/2004/AAAI04-027.pdf) introduced a
  generic divide-and-conquer method for irreducible conflicts over CP, SAT, and
  description-logic solvers in 2004.
- [Fault Localization on Verification Witnesses](https://www.sosy-lab.org/research/pub/2024-SPIN.Fault_Localization_on_Verification_Witnesses.pdf)
  applies MaxSat, MinUnsat, and arbitrary-UNSAT techniques to reduce software
  verification witnesses and evaluates them at substantial scale.
- Incremental SAT, assumption queries, bounded model checking, minimal UNSAT
  reasoning, and replayable verification witnesses are all established areas.

The narrower research hypothesis tested here is whether a continuation quotient
is a useful reusable decision representation for the exact intervention queries
generated while minimising **temporal primary-input waveform segments**, and
whether that comparison can be bound to a machine-verifiable causal artifact.
The repository does not yet establish that this complete combination is a novel
scholarly contribution.

The SPIN work operates on software-witness statements and has its own
reproduction artifact. CQ-SAT/GCC operates on AIGER input waveform segments.
The tools therefore are not presented as runtime substitutes or directly
comparable products. This experiment compares the shared algorithmic class and
query workload, not the papers' full end-to-end systems.

## Predeclared comparison

Two deterministic extraction strategies use the same ordered event vocabulary:

1. `deletion`: test each event once in source order and retain it when its
   removal permits the target output to be false.
2. `quickxplain`: recursively split the ordered vocabulary using the 2004
   divide-and-conquer conflict algorithm.

Each strategy first discovers a candidate cause using fresh CDCL. A separate
validation phase proves the selected set sufficient and proves every retained
event individually necessary. The exact ordered query transcript is then
replayed through a newly built persistent-CDCL solver and, when admitted by the
fixed structural gate, one continuation quotient. Any answer disagreement
aborts without publishing a successful CSV.

The command compiles that quotient once and shares it across both replays, but
each CSV row is conservatively charged the complete preparation time. Thus a
row's CQ total-speed figure represents a standalone strategy run and does not
hide compilation by amortising it across the two strategies.

The comparison includes:

- a controlled sparse circuit with two causal and fourteen irrelevant inputs;
- a controlled dense circuit in which all sixteen inputs are causal;
- the infusion-pump door-interlock regression; and
- the independently sourced SPI AIGER fixture.

On Linux, Android, and macOS the command refuses overwrite and publishes through
an atomic no-clobber rename; it fails closed on other operating systems. It uses
the existing 512-event and 250-million-work limits and caps each strategy at
`3 * candidates + 2` fresh queries. CQ retains the fixed 256-variable,
4,096-clause, and 20-frontier-bit limits.

The strict CSV begins with `causal_strategy_schema_version=1` in every data row.
It contains exactly one deletion row and one QuickXplain row per invocation;
schema changes require a new version rather than silently changing columns.
`cause_sha256` hashes the domain tag `cq-causal-selection-v1`, then each selected
event's zero-based vocabulary index, input index, start frame, and end frame as
fixed-width little-endian `u64` values followed by its one-byte Boolean value.

## Reproduce one model

```sh
cargo build --release --locked

target/release/continuation-quotient-sat \
  benchmark-aiger-causal-strategies \
  examples/aiger/causal-sparse-16.aag \
  1 16 target/causal-sparse-strategies.csv
```

The checked-in eight-row comparison is
[`results/causal-strategy-comparison-v1.csv`](../results/causal-strategy-comparison-v1.csv).
Timing values came from one arm64 macOS run and are illustrative. Query counts,
causes, digests, agreement, and minimality are deterministic correctness data.

## Result

All eight rows agree across every applicable backend and independently pass the
1-minimality phase. Both strategies select the same cause in all four formulas.

| Formula | Cause/candidates | Deletion search | QuickXplain search | Better search |
| --- | ---: | ---: | ---: | --- |
| Sparse control | 2/16 | 17 | 10 | QuickXplain |
| Dense control | 16/16 | 17 | 32 | Deletion |
| Infusion pump | 2/4 | 5 | 8 | Deletion |
| SPI | 23/26 | 27 | 52 | Deletion |

Persistent CDCL is faster than rebuilding fresh CDCL on every row. CQ is
admitted for six rows and its query-only speedup over persistent CDCL ranges
from 1.77x to 5.82x. After CQ preparation, its total speedup ranges from only
0.017x to 0.099x: it loses on every admitted row.

The defensible conclusion is therefore:

- QuickXplain is useful when the cause is sparse, but it is not a universal
  replacement for deletion.
- Persistent CDCL is the practical default for these short extraction runs.
- CQ demonstrates reusable query acceleration, but the tested workloads do not
  amortise compilation.
- The current evidence supports a certified causal-analysis architecture and a
  research vehicle for CQ reuse. It does **not** support a new conflict
  minimisation algorithm or a production performance claim.

A stronger novelty case requires a predeclared external corpus with many
intervention queries per compiled model, comparison against maintained
MUS/fault-localisation tools where semantics can be aligned, and review by
researchers in SAT-based diagnosis and hardware verification.
