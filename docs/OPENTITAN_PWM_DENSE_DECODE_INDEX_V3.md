# OpenTitan PWM dense decode index v3

## Question

Can GCC retain the exact multi-successor graph result while removing the
consumer regression caused by balanced-tree lookup and reconstruction on
every scalar replay step?

Version 2 established strong controlled-cohort representation and unique-decode
reductions, but failed its frozen local resource gate. This experiment changes
only the in-memory verification index and coverage accounting. It does not
change artifact bytes, source binding, scalar semantics, cohort, branch
selection, terminal evidence or refusal policy.

## Mechanism

The version 2 verifier stores graph nodes in a balanced map keyed by program
counter. During every scalar replay step it performs a tree lookup and inserts
the observed transition into another tree for canonical reconstruction.

Version 3 derives two bounded in-memory structures after decoding the unchanged
version 2 artifact:

1. a dense halfword-aligned source-offset table mapping each possible
   instruction location in the bound firmware image to its canonical graph
   node; and
2. one flat edge-coverage bitmap addressed by canonical node and sorted-edge
   position.

At each replay step, the machine's current program counter indexes the source
table in constant time. The independently replayed transition must still match
one sorted declared edge. The verifier marks that exact edge in the bitmap.
After all 256 inputs, every declared edge must have been covered and every
terminal plus scalar-work count must reconstruct canonically.

The dense table is derived from, and bounded by, the caller-supplied firmware
image. It is not serialized or trusted. Invalid alignment, duplicate source
locations, out-of-image nodes, missing edges, additional edges and uncovered
content still refuse.

## Frozen cohort and baselines

The firmware cohort, compiler, all 256 inputs and class counts 1, 2, 4, 8, 16
and 32 are identical to versions 1 and 2.

For every class count compare:

1. the retained balanced-tree version 2 verifier;
2. the dense-index version 3 verifier; and
3. the canonical non-sharing trace-family verifier.

All routes consume the same frozen version 2 graph or trace-family bytes.
Producer work and artifact size are not rerun as a new result.

## Measurements

Retain:

- exact verification result identity;
- scalar replay and unique-decode identity;
- byte identity before and after the verifier change;
- version 2 hostile-case equivalence;
- five warm whole-process consumer trials per route and class count;
- median elapsed time and median peak RSS; and
- all lookup-table and edge-bitmap allocation bounds.

The process measurements remain near local timer resolution. Medians are
predeclared here to prevent one unrelated host-allocation outlier from deciding
the experiment.

## Predeclared gates

Version 3 succeeds only if:

1. graph artifact bytes and SHA-256 identities remain exactly equal to version
   2 at all six class counts;
2. both graph verifiers return identical complete semantics, terminal tables,
   scalar work, unique decode counts and edge counts;
3. every version 2 hostile category still refuses under the dense verifier;
4. every dense source slot is empty or names exactly one source-bound node,
   and every declared edge is covered by at least one independent replay;
5. at 16 and 32 classes, dense-verifier median elapsed time is no more than
   1.25 times the trace-family median;
6. at every class count, dense-verifier median elapsed time is no greater than
   the balanced-tree verifier median;
7. at every class count, dense-verifier median peak RSS is no more than 1.25
   times the trace-family median peak RSS; and
8. the artifact format, producer, scalar replay count, trial count, cohort and
   losing measurements remain unchanged and visible.

No threshold, route, class count, trial count, timing statistic or allocation
definition may change after the first complete version 3 resource cycle.

## Claim boundary

Passing would establish an implementation result on the controlled cohort:
source-offset indexing and flat coverage can preserve the version 2
proof-carrying graph while removing its measured consumer regression.

It would not broaden firmware support, establish universal performance,
validate arbitrary control-flow graphs, change the production support profile
or qualify the mechanism for release. An unrelated public-firmware cohort and
hosted reproduction would remain necessary.
