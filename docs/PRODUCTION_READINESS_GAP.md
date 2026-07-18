# Production-readiness gap register

CQ-SAT/GCC v0.28.0 is an evaluation-ready research prototype. This register is
the authoritative checklist for any future production-grade claim. A gate is
closed only by linked, reproducible evidence; passing unit tests alone is not
sufficient.

| Gate | Current evidence | Closure requirement | Status |
|---|---|---|---|
| Exact specialised backend | Certificate v1 independently rebuilds source support, phase powers and terminal sets; positive traces replay and negative claims prove empty safe intersection | Preserve this coverage across the public compatibility corpus and future API changes | Closed for bounded v1 |
| Portfolio integration | Counterfactual portfolio v1 uses the versioned static rule; its strict consumer rechecks certificates or CDCL answers; forced resource exhaustion and both answer directions are covered across the three-product compatibility cohort | Preserve the contract and coverage across future API changes | Closed for bounded v1 |
| Resource governance | Static input, latch, horizon, node and cache bounds | Wall-clock, memory, output-size and child-process limits with stable machine-readable failure classes | Open |
| Hostile inputs | Strict certificate/transcript sizes, canonical syntax, unknown/duplicate/truncation rejection and symlink refusal | Add parser fuzz corpus, denial-of-service tests and a documented certificate threat model | In progress |
| Stable interface | Research CLI commands and CSV schemas | Versioned API/CLI contract, compatibility tests, migration policy and deprecation window | Open |
| Observability | Benchmark timings and status fields | Structured logs/metrics for admission, fallback, cache use, resource rejection, certificate generation and verification | Open |
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
