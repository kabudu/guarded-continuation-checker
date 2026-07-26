# OpenTitan PWM predicate certificate and portfolio experiment v1

Status: implementation in progress after predeclaration. The local canonical
codec, byte-starting scalar checker, deterministic authentic cycles and static
predicate/exact routing pass. RTL composition, complete resource evidence,
hosted reproduction, maintained baselines and independent assessment remain
open. No novelty or production claim exists.

## Product question

Can GCC turn the exact finite-domain firmware transducer into a deterministic,
bounded and independently checkable artifact, then select it through a static
portfolio without weakening the complete exact fallback?

The mechanism result is insufficient by itself. An in-memory producer value
cannot be exchanged, retained, audited across revisions or checked by another
process. A transducer refusal must also never become a partial result or trigger
an answer-changing recovery path.

## Frozen scope

This cycle retains the pinned OpenTitan PWM source, O0 and O2 RV32IMC images,
complete eight-bit runtime-channel domain, six valid singleton paths, one
250-member invalid predicate, native recorder, exact GCC reference, RTL
fixtures and all frozen work denominators.

The certificate route is version 1 and covers exactly this bounded compiled
MMIO predicate workflow. General symbolic execution, arbitrary input domains
and unrestricted firmware are outside scope.

## Certificate contents

One canonical artifact binds:

- format, semantic-policy and producer versions;
- the exact image length and SHA-256 identity;
- entry, event-count and event-array symbol addresses;
- the canonical invalid domain `a0 = 6..255`;
- the ordered shared control trace, including PC, decoded word, instruction
  width and next PC for every transition;
- the uniform invalid return, ordered MMIO events and event-write locations;
- the six ordered valid singleton behaviors;
- symbolic transition, scalar lane-work and sparse-memory counts; and
- the complete workflow route and exact-fallback policy identity.

No digest, representative lane, producer assertion or class label proves a
semantic claim. The verifier reparses the artifact and independently replays
the complete source-bound obligation.

## Canonical bounded codec

The decoder must:

1. reject an unknown magic, version, policy or route before semantic work;
2. enforce a fixed maximum artifact size before parsing;
3. validate every count and byte length before allocation;
4. reject noncanonical integers, duplicate fields, trailing bytes, truncation
   and noncanonical ordering;
5. bind the caller-supplied image bytes and symbol layout exactly;
6. cap control transitions, valid members, events and event locations at their
   semantic policy limits;
7. round-trip accepted bytes to byte-identical canonical bytes; and
8. return no partial workflow, behavior or RTL answer on any error.

Two clean producer cycles per profile must be byte-identical. Every
single-byte mutation and every truncation of each retained artifact must fail
closed.

## Independent verification

The checker remains algorithmically separate from the vector producer. It:

- checks image identity and symbols from caller-owned inputs;
- reconstructs the six valid paths with the ordinary concrete engine;
- decodes each shared instruction once;
- advances 250 ordinary scalar replay machines;
- checks each lane's own fetched bytes and certified PC sequence;
- requires uniform control, load/store addresses and instruction effects;
- reconstructs sparse-memory accounting;
- compares every return, MMIO event and event-write location; and
- rejects a nonterminating, omitted, repeated or substituted lane.

The checker must accept bytes, image and symbols. It must not require the
producer's in-memory Rust structure.

## Static portfolio

A bounded preflight may inspect and abstractly execute the complete canonical
domain, but it emits no terminal behavior or certificate. It selects:

- `predicate-v1` only when all lanes have one supported control trace, uniform
  addresses and bounded work; or
- `exact-v1` for every structural rejection.

The selected route is recorded before evidence production. A failure after
`predicate-v1` selection returns refusal with no result and never silently
falls back. Forced route changes, source changes between preflight and
production, and policy changes fail closed. Exact fallback executes the
complete unchanged 256-input query.

## Hostile matrix

Controls must cover every encoded claim class plus:

- magic, version, policy, route and image substitution;
- symbol, domain, member count and member ordering drift;
- transition insertion, omission, duplication, reordering and substitution;
- instruction width, PC, decoded word and next-PC drift;
- branch, load, store and indirect-target divergence;
- return, event field, event order and event-location drift;
- sparse-memory and all work-counter drift;
- over-limit counts, lengths and arithmetic overflow;
- all truncations, extensions and single-byte mutations;
- forced exact-to-predicate and predicate-to-exact routing;
- source replacement after preflight; and
- producer refusal after admission without partial output.

The three retained exact-state, live-memory and live-slice negative controls
must remain negative.

## Resource and compatibility policy

The artifact limit is 4 MiB. The semantic transition limit remains
`MAX_RV32_STEPS`; lane count is exactly 250; valid members are exactly six;
MMIO events are capped at 32. Producer and verifier report artifact bytes,
decoded transitions, scalar-equivalent lane operations, wall time and peak
RSS separately.

The public Rust byte APIs and wire format remain experimental until a tagged
release freezes them. Once admitted, version 1 follows the repository's
published compatibility and deprecation policy. Future formats must use new
versions and must not reinterpret version 1 bytes.

## Predeclared gates

The cycle passes only if:

1. O0 and O2 certificate semantics agree with native and exact references for
   all 256 inputs;
2. independent verification starts from bytes and caller-owned source inputs;
3. two clean cycles per profile are byte-identical;
4. every mutation and truncation in the hostile matrix refuses with no result;
5. static portfolio selection preserves predicate admission and exact fallback
   on positive and negative controls;
6. forced routing and source drift fail closed;
7. producer plus verifier remain within the frozen fourfold transition gates;
8. all lane work, wall time, peak RSS and artifact bytes remain visible;
9. valid classes compose with the retained independently checked RTL answers
   and invalid lanes produce no RTL answer;
10. hosted Linux reproduces identities, semantics and resource bounds;
11. the public API compatibility gate passes; and
12. identical-scope angr, CBMC and Veritesting baselines and independent
    assessment remain explicit prerequisites for novelty.

Failure is retained and the complete exact route remains selected. Passing
would establish an exchangeable bounded product mechanism, not a universal
solver, scholarly novelty or production readiness.

## Local implementation result

The version 1 codec now uses fixed-width canonical fields, a 4 MiB outer limit,
count validation before allocation and a SHA-256 corruption check. Accepted
bytes are re-encoded and must be byte-identical. Semantic acceptance still
requires the independent 250-machine scalar replay; the checksum is not proof.

The synthetic hostile suite changes every artifact byte and tests every
truncation. It also rejects image and symbol substitution. Portfolio controls
select `predicate-v1` for a uniform invalid domain, select the complete
`exact-v1` reference for a lane-varying terminal neighbor, and reject source
changes and forced route substitution.

Two clean authentic cycles per profile are byte-identical. Every single-byte
mutation, truncation and one-byte extension of both retained artifacts is
rejected:

| Profile | Artifact bytes | Hostile codec cases | Verifier decoded transitions | Verifier lane steps | Whole-cycle wall time | Peak RSS | Route |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| O0 | 17,284 | 34,569 | 12,781 | 303,250 | 0.47 s | 10.81 MiB | `predicate-v1` |
| O2 | 7,339 | 14,679 | 4,408 | 112,000 | 0.06 s | 4.39 MiB | `predicate-v1` |

The decoded transition figures remain within the frozen complete-cycle gate
when combined with production. The public downstream Rust test exchanges
bytes, decodes them, performs independent semantic replay and exercises
portfolio production and verification without private module access.

The first scalar-checker layout held all 250 complete machine memories at once
and exposed a 793.39 MiB O0 peak. The corrected checker decodes the trace once,
replays one ordinary scalar lane at a time and retains only the first lane's
effect transcript. This preserves every semantic comparison and lane-work
count while reducing the measured O0 peak to 10.81 MiB.

This closes local portions of gates 1 through 8 and 11, including the complete
authentic byte-mutation and truncation matrix. Valid-class RTL composition,
hosted Linux resource reproduction, maintained symbolic baselines and
independent assessment remain open. The feature is not admitted to the
production support profile.
