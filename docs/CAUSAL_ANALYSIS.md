# Certified causal counterexample analysis

CQ-SAT/GCC includes an experimental causal-analysis command for input-driven
ASCII AIGER safety models. It does not claim to invent counterexample
minimisation. Delta debugging and causality-based explanations are established
research areas. This experiment tests the narrower question of whether an exact
continuation quotient can reuse work across the intervention queries required to
produce a machine-checkable minimal sufficient cause.

## Claim boundary

Given the earliest counterexample within a requested horizon, the command groups
each primary input waveform into maximal constant segments. The complete set of
segments fixes the observed input trace. It then adds the negation of the named
bad output and removes segments while the remaining observations still make
that target-false alternative unsatisfiable.

The retained segments are therefore a **sufficient cause under the documented
intervention model**: fixing those observations makes the same bad output at the
original counterexample's earliest frame unavoidable. Other permitted input
values may make a different output—or the same output—fail earlier; that is not
excluded by this intervention model. The certificate is **1-minimal**—removing
any one retained segment makes an alternative with the target output false at
that frame satisfiable. It is not guaranteed to have globally minimum
cardinality, establish physical-world causation, locate the defective RTL
statement, or assign legal or engineering responsibility.

Unknown initial latch values are fixed to the counterexample and treated as
context, not candidate causes. Declared AIGER initial values remain part of the
model. The verifier rejects a certificate if its source digest, earliest target,
candidate segmentation, sufficiency, or per-event minimality check disagrees.

## Run and verify

```sh
cargo build --release --locked

target/release/continuation-quotient-sat \
  explain-aiger-counterexample \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  8 16 target/causal/door-interlock

target/release/continuation-quotient-sat \
  verify-aiger-causal-bundle \
  examples/products/infusion-pump/firmware/door-interlock-regression.aag \
  target/causal/door-interlock
```

The exact file and validation contract is specified in
[Causal evidence bundle v1](CAUSAL_BUNDLE_V1.md). For a self-contained run, use
`scripts/run-causal-analysis.sh INPUT.aag OUTPUT_DIR [HORIZON] [MAX_BOUND_BITS]`.
The separate [closest-method comparison](CAUSAL_STRATEGY_COMPARISON.md) explains
how deletion and QuickXplain are compared without claiming either as new.

`MAX_BOUND_BITS` controls the CQ admission threshold and cannot exceed 20. A
rejected quotient does not weaken correctness: persistent and fresh CDCL remain
the exact analysis and comparison paths. When CQ is admitted, every intervention
answer must agree with both CDCL paths or the command fails without publishing a
successful result.

The command publishes an atomic, no-overwrite evidence directory containing a
SHA-256-bound manifest, certificate, and metrics report. The bundle verifier
rejects missing, additional, non-regular, modified, or source-mismatched files.
The certificate records only model indexes and Boolean values for causes. Input
names are resolved from the digest-bound AIGER model during verification. Treat
certificates as potentially confidential because even a reduced trace may reveal
product behaviour.

## Bounds and failure behaviour

- ASCII AIGER parsing retains the existing 256 MiB input limit and two-million
  BMC-variable limit.
- At most 512 input segments are accepted.
- A conservative query-work estimate must not exceed 250 million clause/variable
  units.
- CQ compilation is considered only for at most 256 variables and 4,096 clauses.
- CQ frontier bounds above 20 bits are refused.
- Malformed, oversized, duplicate, source-mismatched, non-sufficient, or
  non-minimal certificates fail with exit status 2.
- An input without a primary input or without a counterexample in the requested
  horizon is rejected rather than given a misleading explanation.

The causal bundle and certificate are versioned contracts, but the analysis
remains an evaluation-ready research capability rather than a generally
validated production fault-localisation product. Wider claims require external
RTL validation, independent review, and demonstrated amortised benefit on a
predeclared corpus.

## Interpreting measurements

The CSV compares fresh CDCL, persistent CDCL, and CQ query time over the exact
same deterministic minimisation sequence. `cq_query_speedup` excludes quotient
compilation; `cq_amortized_speedup` includes it. A useful result must report both.
The implementation must not be described as a performance improvement when only
query-only speedup exceeds one.
