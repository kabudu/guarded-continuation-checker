# Production-readiness gap register

Guarded Continuation Checker v0.28.0 is an evaluation-ready research prototype. This register is
the authoritative checklist for any future production-grade claim. A gate is
closed only by linked, reproducible evidence; passing unit tests alone is not
sufficient.

| Gate | Current evidence | Closure requirement | Status |
|---|---|---|---|
| Exact specialised backend | Certificate v1 independently rebuilds source support, phase powers and terminal sets; canonical candidate v2 checks relation/terminal proofs, phase powers and traces without exhaustive input enumeration across the three-product and multiphase cohort | Preserve both formats across compatibility changes; close v2 reliability and cost gates before portfolio replacement | Closed for bounded v1 and candidate-v2 semantics |
| Portfolio integration | Counterfactual portfolio v1 uses the versioned static rule; its strict consumer rechecks certificates or CDCL answers; forced resource exhaustion and both answer directions are covered across the three-product compatibility cohort | Preserve the contract and coverage across future API changes | Closed for bounded v1 |
| Resource governance | Static dimensions; Rust API applies deadlines, stream/file caps, Unix process groups and non-macOS Unix address-space limits; macOS group regression and a Rust 1.97 Linux container pass; [hosted run 29661860245](https://github.com/kabudu/guarded-continuation-checker/actions/runs/29661860245) passes the Ubuntu limit regression, public RTL corpus and dependency audit; metrics record enforced controls and stable failure classes | Retain these regressions across supported releases; macOS remains development-only for hard memory evidence | Closed for predicate API v1 on supported Linux; macOS limitation explicit |
| Input reliability | Strict certificate/transcript sizes and syntax; v2 reliability boundary; structural native-proof preflight; 5,000 deterministic parser transformations; invalid-UTF8, sparse-oversize and oversized-proof tests; 128 proof transformations; canonical no-clobber DIMACS export plus 40 individual and four aggregate proofs accepted by pinned CaDiCaL/DRAT-trim under Linux process limits | Add continuous robustness coverage; extend checker diversity from completeness obligations to the whole certificate | Closed for bounded v2 parsing and external completeness checking; whole-certificate diversity open |
| Stable interface | Predicate CLI v1 freezes discovery, commands and exit meanings; typed Rust API v1 validates capabilities and exposes shell-free v1/v2 production/verification; a separate integration-test crate proves discovery, production and checking against the real binary | Preserve API compatibility across a tagged release; add an in-process verifier only if deployment evidence shows the process boundary is unsuitable | Closed for candidate CLI/Rust API v1; release evidence open |
| Observability | Portfolio/cost reports preserve backend decisions and timings; predicate Rust API metrics schema v1 records every observed operation's duration, stream sizes, configured limits, exit status and stable failure class with canonical CSV output | Add cache/resource counters inside the executable and demonstrate multi-job aggregation in a product pipeline | In progress |
| Cross-platform distribution | Rust package and Linux/macOS CI paths | Reproducible signed artifacts and smoke tests for supported Linux targets and macOS, with an SBOM and provenance | Open |
| Real product validity | Public synthetic/product-shaped fixtures | Multiple unmodified public firmware/robotics designs plus independent self-service evaluation outcomes | Open |
| Operational guidance | Evaluation and isolation documentation | Installation, sizing, failure handling, upgrade/rollback and incident-response runbooks | Open |
| Release governance | Claim-bounded tagged releases | Production release checklist requires every row above closed or explicitly excludes the capability from production support | Open |

## Event-contract experimental boundary

Event-contract v1 is not a production-supported interface. It has exact
agreement and replay evidence across three product-shaped fixtures, strict
parser bounds, explicit negative performance data, and independently checked
one-step/terminal proof primitives. Experimental certificate v3 now adds a
deterministic format, source/contract binding, independent whole-contract
verification, both answer classes, a 1,000-input parser mutation corpus, and
proof swap/truncation rejection. It does not yet have a stable public API,
portfolio fallback integration, hard per-invocation
resource governance, compatibility commitment, public-design evidence, or
external acceptance evidence. These gates must close before event contracts can
change any production-readiness row above.

V3 completeness obligations now also pass a deterministic DIMACS export and a
maintained CaDiCaL plus DRAT-trim baseline across both answers. This removes one
checker-diversity gap, but the external tools are an evaluation harness rather
than a shipped runtime dependency and do not close the resource, compatibility,
portfolio, or acceptance gaps above.

## Rules

- No timing-based per-formula calibration.
- A specialised-backend error is never silently converted into a positive or
  negative verification answer.
- Fallback must preserve the original query and environmental assumptions.
- Performance evidence must report all negative rows and setup costs.
- “Production grade” remains prohibited wording while any required gate is
  open.
