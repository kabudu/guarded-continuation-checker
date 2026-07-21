# Revision-local component proof v1 plan

## Status

This document predeclares a bounded falsification experiment. No implementation
result, novelty claim, production claim, or release commitment follows from the
plan.

The first semantic primitive is now implemented behind the public
`revision_local` library module. It exhaustively produces and verifies a
source-bound local relation for components with at most eight state bits, eight
semantic input bits, eight selected output bits, 65,536 candidate valuations,
and 30 million estimated node steps. Constraints filter admissible rows. The
verifier independently iterates every state and input valuation and rejects
omitted, extra, reordered, or false rows with left-side or right-side
attribution. Canonical local-relation encoding now round-trips through a
versioned bounded binary codec, rejects every truncation, preflights hostile
counts before allocation, and has a downstream public API test. This is not yet
a compact independently encoded local proof because checking uses exhaustive
semantic re-evaluation rather than compact proof checking. Interface
composition now has a canonical bounded contract, exact constraint-preserving
row join, width and multiple-driver rejection, a four-million pair-check cap,
and a 65,536-pair output cap. Ordinary composition verifies both sources first;
an unforgeable validated handle permits intentional reuse without silently
accepting raw evidence. Final answer evidence, exact v5 fallback, revision-work
observations, and public revision evidence remain incomplete.

## Hypothesis

For two source-separated constrained BTOR2 components connected through a
word-level interface, GCC can retain the unchanged component's canonical local
certificate byte-for-byte across a revision of the other component. A separate
interface certificate can then prove compatibility and the final bounded SAFE
or UNSAFE answer without trusting a generated monolithic model.

The verifier must identify whether rejection originates in the unchanged
component evidence, changed component evidence, interface contract, wiring, or
final property. Attribution is diagnostic evidence, not permission to accept a
partially verified result.

## Explicit non-claims

The experiment does not claim invention of:

- assume-guarantee reasoning;
- incremental or compositional model checking;
- proof caching;
- word-level BTOR2 verification;
- proof-carrying hardware;
- unsatisfiable cores or blame attribution; or
- component transition relations.

The candidate is only the exact certificate invariant and its measured
revision-local reuse behavior. The hypothesis is rejected if the closest
maintained baseline exposes the same behavior or if GCC achieves it only by
hiding a global rebuild in production or verification.

## Frozen v1 scope

- exactly two independently supplied canonical BTOR2 component sources;
- one canonical interface contract supplied separately;
- one component revision per experiment member;
- one to eight interface bits, including at least one word-valued field;
- explicit BTOR2 constraints on both sides of the interface;
- bounded horizons from zero through 32;
- SAFE and UNSAFE members in every retained cohort;
- unchanged exact word-input search v5 fallback outside static admission; and
- no timing, trial solving, or per-formula calibration in backend selection.

## Certificate split

The candidate artifact has four canonical, independently hashed sections:

1. left component source binding and complete local relation evidence;
2. right component source binding and complete local relation evidence;
3. interface assumption, guarantee, width, direction, and wiring evidence; and
4. final bounded answer evidence with a replayable witness or completeness
   obligation.

Changing one component must not alter the encoded bytes or digest of the other
component's section. The verifier reparses all supplied sources and does not
trust producer metadata to determine which section changed.

## Predeclared gates

| Gate | Required result |
|---|---|
| Exactness | Every answer and earliest UNSAFE frame agrees with exact composed search v5 and a maintained solver control |
| Local completeness | An independent verifier proves every admitted local interface relation complete, not sampled |
| Revision reuse | The unchanged component section and digest are byte-identical across every admitted revision pair |
| No hidden rebuild | Producer and verifier observations show no parsing, solving, or proof checking of the unchanged local obligation after trusted validation is loaded |
| Constraint integrity | Interface composition admits only valuations satisfying both source constraints and the explicit contract |
| Both answers | Each public cohort contains retained SAFE and UNSAFE revision pairs |
| Attribution | Every hostile mutation maps deterministically to the smallest declared failing section, with no accepted ambiguity |
| Static fallback | Unsupported width, coupling, resource, or contract shapes route unchanged to exact v5 or fail closed |
| Strong baseline | Compare artifact bytes, producer time and peak memory, checker time and peak memory, and revision rebuild work with full rebuild and composed-witness baselines |
| Public evidence | At least one revision-pinned public firmware or RTL subsystem supplies two real revisions and repository-authored wiring is labelled separately |
| Reproducibility | Certificate bytes and answers agree on Linux, macOS, and Windows |
| Hostile input | Stale proof, hidden coupling, width drift, direction swap, constraint drift, source drift, truncation, reordering, count, and size attacks fail closed |

## Falsification controls

The experiment must include:

- a revision that changes only internal logic while preserving the interface;
- a revision that changes an interface width;
- a revision that weakens a source constraint;
- a hidden cross-component dependency not declared in the contract;
- a stale unchanged-side certificate paired with a changed source;
- a SAFE-to-UNSAFE boundary change and an UNSAFE-to-SAFE boundary change;
- a monolithic near-neighbour where local completeness is more expensive than
  exact fallback; and
- an unsupported case that demonstrates exact fallback rather than refusal or
  an optimistic answer.

## Baselines

The ordinary baseline rebuilds and checks exact composed word-input search v5
evidence for every revision pair. The proof-carrying baseline follows the
published FM 2026 constrained composed-witness construction where the input
format and answer class overlap. Btor2-Cert and a maintained BTOR2 or SMT model
checker remain semantic controls where available. Any paper-derived baseline
code must be identified as GCC's implementation, not the authors' released
tool.

## Implementation order

1. Freeze canonical component-local relation and interface-contract schemas.
2. Implement producer and separately routed verifier paths with explicit work
   observations.
3. Add exact v5 fallback before admitting any specialised answer.
4. Add hostile controls and every-prefix plus bounded mutation testing.
5. Run repository-authored SAFE and UNSAFE fixtures.
6. Import the revision-pinned public cohort and maintained-tool controls.
7. Run strong baselines and cross-platform packaging.
8. Reject or retain the candidate strictly from the predeclared gates.
