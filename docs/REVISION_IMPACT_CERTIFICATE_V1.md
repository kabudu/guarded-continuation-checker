# Revision impact certificate v1

## Status

Predeclared before implementation. The first bounded certificate core is now
implemented as a public Rust module. It validates a canonical dependency DAG,
complete mask-major counterfactual table, support isolation, and exhaustive
inclusion-minimal invalidating sets. Its verifier requires a caller-supplied
independent semantic evaluator for every observation. The first exact adapter
now produces every old/new combination through the existing two-component
revision engine, binds each observation to the SHA-256 of its canonical
revision-local evidence, and independently decodes and verifies every retained
artifact. The first strict file CLI now produces and independently verifies the
aggregate bundle from old/new component sources, old/new interface contracts,
and a canonical bounded-query manifest. It uses bounded non-symlink reads and
atomic no-clobber publication. The public OpenTitan cohort and the remaining
gates are still open.

The public `RevisionImpactTool` adds a shell-free process boundary with runtime
deadlines, output limits, file-size limits, process-group containment where
available, strict capability negotiation, exact response parsing, and
per-invocation success or failure metrics. Its default file limit is 64 MiB so
the client can admit the complete advertised v1 bundle range.

Independent verification also emits deterministic `verification-v1` work:
parsed scenario-evidence bytes, semantic replays, component validations,
composed pair checks, final transition checks, and result comparisons. Every
counter uses checked arithmetic. Replay, validation, comparison, and byte
totals must equal the complete evidence table before verification succeeds.
These logical counters are reproducible evidence; elapsed microseconds remain
ordinary local telemetry and are not part of certificate identity.

The file boundary is exercised at the inclusive 32-query limit on a one-atom
cohort. Query 33, a 16,385-byte manifest, CRLF text, and a manifest symlink are
refused before output creation. Aggregate API tests separately cover each
source, scenario-evidence, bundle, combination, and query policy ceiling.
This is a falsification experiment, not a novelty claim or
production-supported interface.

The exact adapter now also has a canonical aggregate bundle and explicit
caller policy for total source bytes, per-scenario evidence bytes, aggregate
bundle bytes, combinations, and queries. Production and verification apply the
same limits, and the decoder validates lengths and counts before retaining
scenario evidence.

## Product question

After a firmware or robotics RTL revision, can GCC produce a deterministic,
independently checkable certificate that tells a release pipeline:

- which component evidence remains valid and byte-identical;
- which source, interface, or property boundaries invalidate reuse;
- the exact SAFE or UNSAFE result after the revision;
- the smallest counterfactual change sets sufficient to alter reuse validity;
  and
- which obligations must be regenerated, with exact fail-closed fallback when
  the bounded analysis is not admitted?

The intended use is selective formal regression in firmware CI. The result is
not a heuristic file-diff report. Every retained section must be re-bound to its
original source and independently validated before it can be reused.

## Closest prior art and claim boundary

Incremental verification is established. Chockler et al. reuse and patch IC3
proofs and counterexamples after hardware changes, reporting speedups of up to
two orders of magnitude. Regression verification also reuses abstraction
precisions across 1,119 Linux driver revisions. More recent work identifies
which SysML proofs remain valid after model mutations, and Rtl2lean builds
machine-checked hierarchical RTL lemmas for reuse.

Primary references:

- [Incremental Formal Verification of Hardware](https://www.cs.utexas.edu/~hunt/FMCAD/fmcad11/papers/8.pdf)
- [Reusing Precisions for Efficient Regression Verification](https://arxiv.org/abs/1305.6915)
- [Incremental and Formal Verification of SysML Models](https://doi.org/10.1007/s42979-024-03027-5)
- [Rtl2lean](https://arxiv.org/abs/2607.16855)

GCC cannot claim novelty for incremental checking, proof reuse, change-impact
propagation, reusable lemmas, dependency slicing, counterfactual analysis, or
minimal cores individually. A candidate distinction survives only if the
combined certificate provides deterministic portable evidence for exact
component-local reuse, independently checked counterfactual invalidation, both
answer classes, and a practical self-service firmware workflow that the closest
maintained baseline does not provide.

## Frozen v1 boundary

V1 admits one to eight named changed atoms and one to 32 bounded safety queries.
Inputs are canonical component sources, validated local relation artifacts,
strict interface contracts, queries, and an old-to-new revision mapping. Every
name, digest, dependency edge, query support, limit, and ordering rule is bound
into the certificate.

A component section is reusable only when its canonical evidence is
byte-identical and its source binding is unchanged. Semantic equivalence under
different source bytes is recorded separately and cannot silently reuse the old
source-bound artifact.

For at most eight changed source or interface atoms, the producer enumerates
every old/new counterfactual combination. The independent verifier recomputes
every admitted result and derives all inclusion-minimal invalidating sets from
the complete table. A claimed set is minimal only when it invalidates reuse and
removing any member restores validity. No greedy or probabilistic result is
accepted as minimal.

## Resource policy

- one to eight changed component, interface, or property atoms;
- at most eight changed source or interface atoms;
- at most 256 counterfactual combinations;
- at most 32 queries;
- at most 64 dependency edges;
- at most 64 minimal invalidating sets;
- at most 64 MiB of total input artifacts;
- at most 16 MiB for one local artifact;
- at most 64 MiB for one impact certificate; and
- checked work counters for parsing, semantic replay, combinations, component
  validations, composed transitions, and result comparisons.

Exceeding any limit returns a typed refusal before certificate publication. The
portfolio may then run the existing complete exact workflow, but must never
present a fallback result as a revision-impact certificate.

## Research file CLI

The query manifest is canonical UTF-8 with LF endings, no NUL bytes, a required
header, and one strictly ordered query per subsequent line:

```text
gcc-btor2-revision-impact-queries-v1
0,right,10
1,right,10
```

Each query uses `HORIZON,BAD_SIDE,BAD_OUTPUT`. The ordering key is horizon,
then `left` before `right`, then the nonzero BTOR2 bad-output node identifier.
The manifest contains one to 32 unique queries and must end with LF.

Create an aggregate bundle:

```console
guarded-continuation-checker check-btor2-revision-impact \
  LEFT_OLD.btor2 LEFT_NEW.btor2 LEFT_OUTPUTS \
  RIGHT_OLD.btor2 RIGHT_NEW.btor2 RIGHT_OUTPUTS \
  INTERFACE_OLD.txt INTERFACE_NEW.txt QUERIES.txt \
  OUTPUT.revision-impact
```

Independently decode and verify the same bound inputs:

```console
guarded-continuation-checker verify-btor2-revision-impact \
  LEFT_OLD.btor2 LEFT_NEW.btor2 LEFT_OUTPUTS \
  RIGHT_OLD.btor2 RIGHT_NEW.btor2 RIGHT_OUTPUTS \
  INTERFACE_OLD.txt INTERFACE_NEW.txt QUERIES.txt \
  INPUT.revision-impact
```

`LEFT_OUTPUTS` and `RIGHT_OUTPUTS` are strictly increasing comma-separated
nonzero node identifiers. Production verifies every retained scenario before
creating the output. Verification binds every supplied source, interface, and
query again. Existing output, malformed manifests, source drift, query drift,
and certificate mutation fail closed; no fallback result is emitted.

Capability discovery is a single canonical line:

```console
guarded-continuation-checker btor2-revision-impact-cli-version
```

It binds CLI, impact, and query-manifest versions; all source, evidence, bundle,
atom, combination, and query limits; `exact-counterfactual-v1` semantics; no
routing; no fallback; the `verification-v1` work schema; and fail-closed
handling of unsupported inputs. The typed
Rust client refuses any missing, reordered, noncanonical, or changed field
before exposing the executable as compatible.

## Predeclared gates

1. **Exactness:** every old, new, and counterfactual result agrees with the
   existing exact revision-local path and a maintained model checker.
2. **Independent verification:** the checker recomputes source bindings,
   retained evidence validity, dependency propagation, every counterfactual
   result, and minimal invalidating sets without calling producer logic.
3. **Both answers:** retained cohorts contain SAFE-to-UNSAFE,
   UNSAFE-to-SAFE, unchanged SAFE, and unchanged UNSAFE queries.
4. **No stale reuse:** changed source, relation, interface, property, ordering,
   or hidden coupling can never be classified as reusable.
5. **Minimality:** exhaustive bounded enumeration proves every reported
   invalidating set is sufficient and inclusion-minimal, and that none is
   omitted.
6. **Determinism:** repeated production emits byte-identical certificates,
   ordering, reason codes, work counts, and logical results.
7. **Resource governance:** every declared dimension passes at its inclusive
   boundary and refuses one step beyond it before publication.
8. **Hostile input:** truncation, trailing data, noncanonical integers,
   duplicate names, cycles, missing dependencies, digest substitution, result
   mutation, set mutation, and count inflation fail closed without panic.
9. **Exact fallback:** unsupported shapes use the existing complete portfolio;
   malformed or semantically invalid inputs do not fall back.
10. **Maintained baseline:** compare total source-to-answer producer time,
    independent checking time, peak RSS, and transferred bytes with a pinned
    full-rebuild proof-producing route at identical query scope.
11. **Public semantic revision:** reproduce the pinned OpenTitan `prim_count`
    behaviour-changing revision and then a larger upstream-derived subsystem
    whose whole transition semantics genuinely change.
12. **Self-service integration:** a versioned Rust API, file CLI, typed bounded
    process client, machine-readable capabilities, atomic no-clobber output,
    and a release-build acceptance pipeline all reproduce without per-formula
    calibration.
13. **Portability:** Linux amd64, macOS arm64, and Windows amd64 produce the
    same canonical certificate and logical result.

## Decision rule

The mechanism advances only if all integrity gates pass. A performance or
novelty hypothesis advances only if it beats the full-rebuild baseline on a
real semantic revision after charging initial evidence production,
counterfactual checking, independent verification, and transfer. A positive
repository-authored fixture is insufficient. Negative results remain retained.
