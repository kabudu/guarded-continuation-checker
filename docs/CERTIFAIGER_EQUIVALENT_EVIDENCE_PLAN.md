# Equivalent-evidence Certifaiger comparison plan

Status: predeclared experiment plan. No result or novelty claim.

## Why this is the next gate

The identical-query SymbiYosys baseline is negative on runtime, but its formal
path emits no independently replayable certificate. That leaves GCC's strongest
claimed operational distinction, portable source-bound batch evidence, compared
against a tool with a different evidence contract.

Certifaiger is the appropriate established control. Its public implementation
checks AIGER witness circuits by reducing simulation, safety, and inductiveness
conditions to SAT. It can stream LRAT proofs to an external checker. The
[HWMCC 2025 rules](https://hwmcc.github.io/2025/) require bit-level SAFE
certificates to pass Certifaiger and UNSAFE traces to pass `aigsim`. The
[Certifaiger repository](https://github.com/Froleyks/certifaiger) documents the
witness format and independent checking obligations. This is current
competition practice, not a historical straw baseline.

## Exact scope

Use the existing public washing-controller and physical-plant family. Export six
single-property AIGER 1.9 models that preserve, byte-for-byte where applicable:

- controller and plant transition functions;
- controller and plant initial states;
- sensor/action wiring;
- each selected bad output; and
- the existing horizon of 32.

Encode the bound with a checked frame counter and an absorbing completed state,
so unbounded safety of the exported model is equivalent to the original bounded
query. Before benchmarking, an independent small-state replay must prove each
exported model agrees with the original query on SAFE/UNSAFE and shortest bad
frame.

The external path must use the pinned interface expected by HWMCC:

1. a certifying model checker produces one SAFE witness circuit or one UNSAFE
   AIGER trace per property;
2. Certifaiger checks every SAFE witness;
3. `aigsim` checks every UNSAFE trace; and
4. source, model, property, bound, tool revision, and certificate digests are
   retained in a canonical manifest.

The GCC path uses one proof-carrying controller MTBDD batch artifact and its
fresh independent verifier. Both paths must run in separate producer and
consumer processes without producer caches.

## Measurements

Record these separately for producer and consumer:

- wall time and peak RSS;
- total and per-property evidence bytes;
- model/export bytes that cross the trust boundary;
- checker executable and dependency footprint;
- number and type of SAT, QBF, or proof-checking obligations;
- exact answers and shortest bad frames;
- deterministic reproduction across two clean directories; and
- rejection of mutation, truncation, source drift, property substitution,
  member reordering, stale evidence, and output collision.

No timing observation may influence routing. All unsupported or malformed cases
fail closed.

## Advancement and falsification gates

The evidence-delivery distinction advances only if GCC preserves every answer
and trace while demonstrating at least one independently useful advantage over
the competition-standard certified path, such as smaller aggregate evidence,
lower fresh-checker resources, or one source-bound batch replacing repeated
per-property evidence.

The distinction is falsified for this regime if the Certifaiger plus `aigsim`
path provides equivalently bound, independently checkable evidence with no
material disadvantage in transfer size, checker resources, or integration. A
GCC win against an uncertified solver is irrelevant. A speed win caused by a
weaker bound, omitted source binding, missing SAFE proof, or unvalidated UNSAFE
trace is invalid.

## Pre-implementation gates

- Pin auditable source revisions and licenses for the producer, Certifaiger,
  AIGER utilities, SAT/QBF solvers, and optional LRAT checker.
- Build and run upstream tests in an isolated Linux container.
- Freeze the six exported model digests only after independent semantic replay.
- Add no GCC portfolio route until the complete external comparison and hostile
  controls pass on hosted Linux.
