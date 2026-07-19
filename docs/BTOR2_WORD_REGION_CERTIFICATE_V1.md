# BTOR2 exact word-region certificate v1

## Claim

This experimental certificate proves bounded safety without listing every
reachable state. It is exact only for a deliberately small BTOR2 language.
The verifier recovers the transition and property shape directly from the
source, derives every reachable layer as an arithmetic progression, and proves
that the selected bad set is disjoint.

It is not an abstract over-approximation. A successful certificate has the same
bounded SAFE meaning as the explicit search certificate. An unsupported or
wrapping recurrence receives no word-region answer.

## Admitted transition language

Version 1 requires one word state `s`, one Boolean input `i`, no constraints,
and the same literal initial and reset value `r`. It accepts either:

```text
next(s) = if i then r else s + d
```

with nonzero literal `d` and no wrap through the requested horizon, or:

```text
next(s) = if i then r else if s >= c then s else s + d
```

where `c` is an aligned, non-wrapping saturation point. At frame `t`, the exact
reachable region is:

```text
{ r + k*d | 0 <= k <= min(t, saturation_index) }
```

The source proof is syntactic and arithmetic. It does not trust producer-supplied
classification. Version 1 admits bad predicates `s == literal` and
`s >= literal`. Equality disjointness uses range and divisibility; threshold
disjointness uses the exact maximum.

## Certificate and portfolio

The canonical LF text artifact binds:

- SHA-256 of the exact source bytes;
- query horizon and selected bad property;
- source node identifiers and word width;
- independently recoverable recurrence family and literals;
- independently recoverable bad-predicate family and literal; and
- the exact greatest progression index at the requested horizon.

The public `btor2_bounded` Rust API and these commands apply a fixed static
portfolio:

```sh
guarded-continuation-checker check-btor2-bounded \
  INPUT.btor2 BAD_PROPERTY HORIZON OUTPUT.btor2-cert

guarded-continuation-checker verify-btor2-bounded \
  INPUT.btor2 OUTPUT.btor2-cert
```

The producer tries the exact word-region proof first. If it is inapplicable or
the bad set intersects the region, the unchanged query goes to explicit exact
search. Parse errors and region resource-limit errors are not converted into
answers. The verifier dispatches only self-identifying versioned formats and
then checks the selected backend from source.

Both CLI operations report the selected backend, answer, horizon, bad frame,
logical reachable-state count, certificate bytes, and elapsed microseconds.
These observations are outside the deterministic certificate and never affect
backend admission.

## Resource and trust boundary

- maximum word-region horizon: 1,000,000,000;
- maximum word-region certificate: 64 KiB;
- strict UTF-8, LF, no NUL, fixed field order, canonical decimal integers;
- fixed BTOR2 parser limits inherited from word-core v1;
- checked arithmetic for horizon expansion and logical-state accounting; and
- no solver, producer cache, timing measurement, learned gate, or hidden
  per-formula calibration in the verifier.

The trusted code includes the strict BTOR2 parser, the checker-side structural
matcher, integer arithmetic, SHA-256 implementation, and certificate decoder.
The producer recogniser and verifier matcher are separate code paths. They
still share the parser, data model, arithmetic helpers, and Rust crate, so this
is independent checking of producer claims, not process-level implementation
diversity or formal verification.

## Predeclared experiment gates

1. Both answer directions must agree with explicit search on all retained
   boundary queries.
2. SAFE certificates for the 200-step actuator and 254-step saturating timer
   must be at least 99% smaller than explicit layers.
3. UNSAFE and unsupported cases must use exact fallback without changing the
   query.
4. Source, recurrence, predicate, maximum-index, truncation, ordering, encoding,
   and size tampering must be rejected.
5. The maintained Bitwuzla SAT and UNSAT boundary baseline must remain green.
6. The full Rust suite, Linux bundle, dependency audit, and public RTL corpus
   must remain green in hosted CI.

The retained [six-query result](../results/btor2-region-cohort-v1.md) closes
gates 1 and 2 for the narrow cohort. The other gates require the complete branch
validation and hosted CI before merge.

Pinned Bitwuzla 0.9.1 separately checks the progression-intersection boundary
formulas as UNSAT at actuator horizon 200 and saturating horizon 254, then SAT
at horizons 201 and 255. These are solver-diverse checks of the arithmetic
decision, not whole-certificate or transition-recogniser verification.

## Prior-art boundary

Arithmetic progressions and semilinear descriptions of one-counter reachability,
counter-system acceleration, abstract acceleration, symbolic model checking,
and word-level bounded model checking are established. Useful nearby work
includes:

- [Para2: parameterized path reduction, acceleration, and SMT for reachability](https://doi.org/10.1007/s10703-017-0297-4), which represents families of executions with accelerated schemas;
- [Unbounded-Time Safety Verification of Guarded LTI Models with Inputs by Abstract Acceleration](https://doi.org/10.1007/s10817-020-09562-z), which concisely represents multi-step dynamics;
- [BTOR2, BtorMC and Boolector 3.0](https://fmv.jku.at/papers/NiemetzPreinerWolfBiere-CAV18.pdf), which establishes the word-level interchange and bounded-model-checking setting; and
- established one-counter results representing reachable counter values as
  unions of arithmetic progressions.

Therefore the arithmetic progression, recurrence acceleration, source
recognition, or static portfolio is not claimed as novel. The measured result
establishes a useful exact proof-carrying integration primitive. Candidate
novelty, if any, would require a materially broader composition language,
evidence that its independently checkable proof contract is absent from close
certifying systems, and external review.
