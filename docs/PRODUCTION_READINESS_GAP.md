# Production-readiness gap register

CQ-SAT/GCC v0.28.0 is an evaluation-ready research prototype. This register is
the authoritative checklist for any future production-grade claim. A gate is
closed only by linked, reproducible evidence; passing unit tests alone is not
sufficient.

| Gate | Current evidence | Closure requirement | Status |
|---|---|---|---|
| Exact specialised backend | Original-AIG trace replay and CQ/CDCL/Yosys agreement on bounded fixtures | Independent certificate verifier covers positive and negative answers without trusting the specialised backend | Open |
| Portfolio integration | Dense predicate commands are explicit experiments | Public API selects the backend only through a versioned static rule and falls back exactly to CDCL on every rejection or resource failure | Open |
| Resource governance | Static input, latch, horizon, node and cache bounds | Wall-clock, memory, output-size and child-process limits with stable machine-readable failure classes | Open |
| Hostile inputs | AIGER parser bounds and existing isolated RTL workflow | Fuzzing corpus for predicate certificates, path-safe external-tool invocation, denial-of-service tests and documented threat model | Open |
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
