# Composed-witness baseline v1

Status: predeclared experiment. No result or novelty claim.

## Purpose

FM 2026's *Certifying Constraints in Hardware Model Checking* is the closest
disconfirming baseline found by the bounded prior-art audit. Its `aigmerge`
construction combines arbitrary witness circuits and shares the reset,
transition, base, and step checks for several properties. Comparing GCC only
with independent per-property Certifaiger runs would therefore be a straw
baseline.

This experiment asks a narrower question:

> Can one exact, independently checked controller-local GCC artifact remain
> byte-identical while separately supplied plant models change, without
> rebuilding a whole-circuit witness, and does that provide a measurable
> advantage over a faithful FM 2026 composed-witness construction?

The experiment can reject this hypothesis. It cannot establish patentability
or scholarly novelty by itself.

## Upstream availability audit

The audit on 20 July 2026 inspected every current public branch of:

- `arminbiere/aiger`: `master` and `development`;
- `Froleyks/certifaiger`: `main` and `QBF`; and
- GitHub's public code index for the exact token `aigmerge`.

No released `aigmerge` source was found. The FM 2026 paper says the tool may
eventually be added to the AIGER utilities. The maintained Certifaiger checker
alone is not a substitute because it checks a supplied witness but does not
perform the paper's composition.

The baseline may advance by either of two routes:

1. pin an authors' release at an immutable revision, or
2. implement the paper's construction as a clearly labelled GCC-maintained
   baseline, obtain an independent code review, and validate every output with
   the already qualified Certifaiger plus formally verified `lrat_isa` path.

No measurements from an unreviewed paper-derived implementation may support a
novelty statement.

## Faithful baseline semantics

For model `M`, constraint witness `W_D`, and property witness `W_P`, use the
paper's Theorem 1 construction. Non-model inputs and latches introduced by the
witnesses must be pairwise disjoint. The combined witness contains the union of
their private inputs and latches, combines their reset and transition
relations, and conjoins the witness properties and constraints. Repeated
composition with `D = true` forms the multi-property baseline.

The implementation must preserve all AIGER 1.9 semantics used by Certifaiger,
including latch initialisation, constraints, bad properties, justice and
fairness sections where present, symbols, and comments. Unsupported sections
must fail closed rather than be dropped.

## Frozen experiment family

The first comparison uses the public washing-controller source and at least
four independently supplied physical-plant variants:

1. the retained nominal plant;
2. a sensor-stuck variant;
3. an actuator-delay variant; and
4. a nondeterministic disturbance variant.

Each plant must have a source-to-model attestation and at least two SAFE
properties so that witness-circuit composition is meaningful. UNSAFE members
remain in the product acceptance corpus, but AIGER counterexample traces are
reported separately and are not misrepresented as inductive witness circuits.

The controller source, generated model, relevant inputs, observed outputs, and
controller-local GCC equivalence artifact must be byte-identical across all
plant rows. Each whole-circuit external model and witness is regenerated in a
clean directory.

## Measurements

Record producer and consumer wall time and peak RSS separately. Also record:

- controller-local reusable bytes;
- plant-local and whole-circuit bytes;
- bytes regenerated after replacing exactly one plant;
- aggregate and marginal witness bytes;
- checker and dependency footprint;
- deterministic digests across two clean directories and two architectures;
- SAFE and UNSAFE answers and shortest bad frames;
- exact number and class of SAT, QBF, and proof-checking obligations; and
- whether a whole-circuit witness rebuild was required after the plant change.

The incremental comparison starts from a verified four-plant package, replaces
only plant 3, and verifies the revised package. Timing observations may not
select a route or alter the workload.

## Hostile controls

Both paths must reject:

- controller, plant, wiring, property, bound, or source substitution;
- private latch or input collision;
- missing, duplicated, or reordered witness members;
- a composed witness checked against the wrong whole-circuit model;
- truncation, mutation, malformed counts, integer overflow, and trailing data;
- unsupported AIGER sections that would otherwise be lost;
- stale controller evidence after controller replacement;
- stale plant evidence after plant replacement; and
- output collision or partial overwrite.

All parsers and subprocesses remain under the repository's existing byte,
time, memory, process-group, and no-network policies.

## Advancement and falsification gates

The candidate distinction advances only if all semantic and hostile gates pass
and GCC demonstrates both of these properties:

1. the independently checked controller-local evidence remains byte-identical
   after the predeclared plant replacement; and
2. its marginal re-verification cost has a material, replicated advantage in
   bytes, checker memory, or checked work over the faithful composed-witness
   baseline at the same evidence scope.

The candidate is falsified for this regime if the composed-witness baseline can
reuse equivalent controller semantics without rebuilding evidence, or if GCC's
apparent advantage comes from weaker source binding, omitted proof obligations,
different answer classes, a smaller bound, or unmeasured producer work. A
runtime win alone is insufficient. Failure to obtain or independently review a
faithful baseline leaves the gate open and produces no positive result.

## Implementation sequence

1. Freeze and independently replay the four attested plant variants. Complete:
   all four source-to-model regenerations are byte-identical, all 24 bounded
   answers are independently replayed, and every plant retains two SAFE
   properties.
2. Obtain or implement the exact composed-witness construction. In progress:
   the safety-only v1 paper-derived baseline now coalesces shared mapped model
   state, keeps private variables disjoint, unions reset and transition
   functions, conjoins safety and constraint semantics, and hash-conses gates.
   A pinned upstream witness self-composition and a separately encoded but
   equivalent witness pair are accepted by Certifaiger 10.2.0 with every SAT
   obligation checked through `lrat_isa`. Composition of distinct property
   witnesses and independent implementation review remain open.
3. Add parser, collision, section-preservation, and deterministic-byte tests.
   In progress: canonical ASCII AIGER, strict byte and count limits, truncation,
   mapping-kind, comment-mapping, liveness, symlink, and output-collision
   controls pass. Version 1 rejects liveness and comment-based mappings instead
   of dropping them; broader section support remains open.
4. Validate every SAFE output with qualified Certifaiger and `lrat_isa`.
   Complete on local arm64 and hosted amd64 Linux for the first comparison
   family: all eight individual
   witnesses and all four distinct-property compositions pass Certifaiger
   10.2.0 with `lrat_isa` checking the generated proofs. The composed witness
   total is 9,665 bytes versus 19,164 bytes for the eight separate witnesses,
   a 49.57% reduction. This is disconfirming evidence against a broad GCC
   evidence-size advantage.
   Hosted run
   [29769666175](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29769666175)
   reproduced every CSV row and all twelve individual/composed witness SHA-256
   values exactly. Platform-specific executable and checker-tree digests are
   retained separately and are not represented as reproducible binaries.
5. Run the predeclared replacement experiment on arm64 and hosted amd64 Linux.
   In progress: GCC now has canonical controller-only evidence and a separate
   plant-results artifact bound to the controller-evidence SHA-256. A typed
   admitted-controller capability independently checks the controller proof
   once and reuses the verified state for subsequent plant batches. The public
   API rejects stale controller evidence, source drift, cross-plant
   substitution, mutation, and truncation. The current test uses synthetic
   models only; it is mechanism evidence, not the required four-plant
   measurement.
6. Retain manifests, raw measurements, hostile results, and tool provenance.
7. Update the novelty register with the result, including a negative result.

## Interim result boundary

The safety-only merger now demonstrates the actual multi-property construction,
not only self-composition. Four shared models contain properties 15 and 16. For
each model, pinned rIC3 generates one witness from each corresponding
single-property model; GCC's paper-derived baseline composes them; and the
qualified independent checker accepts the result against the shared model.

This does not close the experiment. Producer and consumer resource measurements,
the predeclared one-plant replacement, hosted amd64 reproduction, broader AIGER
section support, and independent implementation review remain open. Most
importantly, the result confirms that established composed witnesses already
remove almost half of the repeated evidence bytes. Only cross-plant reuse of
controller-local semantics remains a plausible distinction.
