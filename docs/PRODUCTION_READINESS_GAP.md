# Production-readiness gap register

CQ-SAT/GCC v0.28.0 is an evaluation-ready research prototype. This register is
the authoritative checklist for any future production-grade claim. A gate is
closed only by linked, reproducible evidence; passing unit tests alone is not
sufficient.

| Gate | Current evidence | Closure requirement | Status |
|---|---|---|---|
| Exact specialised backend | Certificate v1 independently rebuilds source support, phase powers and terminal sets; canonical candidate v2 checks relation/terminal proofs, phase powers and traces without exhaustive input enumeration across the three-product and multiphase cohort | Preserve both formats across compatibility changes; close v2 reliability and cost gates before portfolio replacement | Closed for bounded v1 and candidate-v2 semantics |
| Portfolio integration | Counterfactual portfolio v1 uses the versioned static rule; its strict consumer rechecks certificates or CDCL answers; forced resource exhaustion and both answer directions are covered across the three-product compatibility cohort | Preserve the contract and coverage across future API changes | Closed for bounded v1 |
| Resource governance | Static model/certificate dimensions; Rust API applies configurable per-call deadlines and 1-B–64-MiB stdout/stderr caps with stable typed timeout/output-limit errors; firmware synthesis already has process-tree, file and Linux memory containment | Add OS memory and process-tree containment to predicate API jobs, plus stable fleet metrics for every rejection class | In progress |
| Input reliability | Strict certificate/transcript sizes and syntax; v2 reliability boundary; structural native-proof preflight; 5,000 deterministic parser transformations; invalid-UTF8, sparse-oversize and oversized-proof tests; 128 proof transformations | Add continuous robustness coverage and process-level time/memory enforcement | Closed for bounded v2 parsing; broader resource governance in progress |
| Stable interface | Predicate CLI v1 freezes discovery, commands and exit meanings; typed Rust API v1 validates capabilities and exposes shell-free v1/v2 production/verification; a separate integration-test crate proves discovery, production and checking against the real binary | Preserve API compatibility across a tagged release; add an in-process verifier only if deployment evidence shows the process boundary is unsuitable | Closed for candidate CLI/Rust API v1; release evidence open |
| Observability | Portfolio reports expose admission, backend, reason and gate/backend/verifier timings; certificate-cost schemas preserve every v1/v2 producer/checker/CDCL trial, artifact/proof size, proof count and direct evaluations | Add stable structured runtime logs/metrics for cache use, resource rejection and fleet aggregation | In progress |
| Cross-platform distribution | Rust package and Linux/macOS CI paths | Reproducible signed artifacts and smoke tests for supported Linux targets and macOS, with an SBOM and provenance | Open |
| Real product validity | Public synthetic/product-shaped fixtures | Multiple unmodified public firmware/robotics designs plus independent self-service evaluation outcomes | Open |
| Operational guidance | Evaluation and isolation documentation | Installation, sizing, failure handling, upgrade/rollback and incident-response runbooks | Open |
| Release governance | Claim-bounded tagged releases | Production release checklist requires every row above closed or explicitly excludes the capability from production support | Open |

## Rules

- No timing-based per-formula calibration.
- A specialised-backend error is never silently converted into a positive or
  negative verification answer.
- Fallback must preserve the original query and environmental assumptions.
- Performance evidence must report all negative rows and setup costs.
- “Production grade” remains prohibited wording while any required gate is
  open.
