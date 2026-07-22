# Revision-local proof portfolio CLI v1

## Status

This is an experimental self-service interface for the revision-local component
proof v1 falsification study. It is not a production-supported workflow or a
novelty claim.

## Produce

```sh
guarded-continuation-checker check-btor2-revision-portfolio \
  LEFT.btor2 LEFT_OUTPUTS \
  RIGHT.btor2 RIGHT_OUTPUTS \
  INTERFACE.txt HORIZON BAD_SIDE BAD_OUTPUT \
  OUTPUT.revision-proof
```

`LEFT_OUTPUTS` and `RIGHT_OUTPUTS` are nonempty, comma-separated, strictly
increasing BTOR2 node identifiers. They must include every node used as a wire
source. The side containing `BAD_OUTPUT` must also project that one-bit node for
the revision-local backend. `BAD_SIDE` is `left` or `right`.

The command reads bounded regular files and statically assesses the two source
structures. An admitted query produces the four-section revision-local
artifact. A stable structural rejection produces exact direct evidence over
the unchanged sources, interface, and query. Selection never uses candidate
timing or trial solving.

The command verifies the newly produced artifact before creating the output and
will not overwrite an existing path.

## Produce with a retained left component

```sh
guarded-continuation-checker check-btor2-revision-retained-left \
  LEFT.btor2 PREVIOUS.revision-proof \
  RIGHT.btor2 RIGHT_OUTPUTS \
  INTERFACE.txt HORIZON BAD_SIDE BAD_OUTPUT \
  OUTPUT.revision-proof
```

The previous artifact must use the revision-local backend. The command decodes
its bounded outer envelope, binds and independently validates the embedded left
relation against `LEFT.btor2` once, and retains it as an opaque validated
artifact. It then produces and validates only the changed right relation,
composes the supplied interface, produces the final evidence, verifies the
result through the retained path, and writes a normal canonical portfolio.

The resulting artifact remains compatible with
`verify-btor2-revision-portfolio`, which can independently check both sides
from scratch. Stable observations distinguish produced and reused local
sections, changed-side candidate valuations, composition checks, and final
transition checks.

This optimised command fails closed if the previous artifact used direct
fallback or the new pair is outside revision-local admission. Operators can
then use `check-btor2-revision-portfolio`, which retains the exact direct
fallback. An optimisation refusal never produces a partial answer.

## Verify

```sh
guarded-continuation-checker verify-btor2-revision-portfolio \
  LEFT.btor2 LEFT_OUTPUTS \
  RIGHT.btor2 RIGHT_OUTPUTS \
  INTERFACE.txt HORIZON BAD_SIDE BAD_OUTPUT \
  INPUT.revision-proof
```

Verification decodes the bounded canonical portfolio, requires the embedded
query to equal the command-line query, recomputes the static route, rejects a
forced or mismatched backend, and independently checks the selected nested
certificate against both original sources and the supplied interface.

## Stable observations

Both commands report:

- `portfolio_version=1`;
- `backend=revision-local` or `backend=direct-exact`;
- the stable selection `reason`;
- `result=SAFE` or `result=UNSAFE`;
- the query horizon and earliest bad frame;
- canonical certificate bytes; and
- elapsed microseconds, which are observational and never affect routing.

## Resource envelope

The revision-local route admits at most eight state bits, eight input bits, and
eight projected output bits per component, 65,536 local valuations, four
million static pair checks, 65,536 composed pairs, horizon 32, 65,536 reachable
states per layer, 262,144 total states, and four million final transition
checks.

The direct route admits at most 12 joint semantic input bits, 64 state nodes per
component, 4,096 joint input valuations, and the same horizon, reachable-state,
and transition-check limits. Exceeding the direct envelope fails closed without
an answer.

## Current limitations

The workflow now has local evidence from two authentic revisions of the public
Roa Logic PLIC gateway and maintained Yosys/Z3 agreement. Whole-process
baselines, broad hostile corpora, cross-platform hosted replication, and tagged
compatibility remain open.

The Rust API and file CLI both support revised proof production and verification
from an opaque validated unchanged component. A long-lived multi-revision
service session and whole-process amortisation baseline remain open integration
gates.
