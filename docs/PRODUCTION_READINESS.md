# Production-readiness gates

CQ-SAT/GCC remains a research preview until every required gate below has direct,
repeatable evidence. A release tag or a green example does not by itself make the
tool production-grade.

## Current evidence

- Exact SAT/BMC answers are regression-tested against brute force, Varisat, cold
  BMC, and selected independent SymbiYosys/Z3 models.
- RTL synthesis has bounded input size, a wall-clock timeout, isolated staging,
  fixed scripts, portable hierarchy lowering, and atomic manifest publication.
- Single- and multi-file sources are snapshotted with ordered provenance.
- Unsafe bounded results preserve named inputs and state for replay.
- RTL safety reports and manifests declare artifact schema version 1.

## Required before a production claim

- [ ] Explicit environment assumptions and constraints with independently
  checked semantics.
- [ ] Include directories, parameter overrides, memories, and declared clock or
  reset policy for representative embedded RTL projects.
- [ ] Stable, versioned CLI and artifact schemas with compatibility tests.
- [ ] Memory and process-tree limits in addition to the current wall timeout and
  model-size limits.
- [ ] Parser and CLI fuzzing with a persistent regression corpus.
- [ ] Differential validation across a substantial public and design-partner RTL
  corpus, including SAT and UNSAT properties and multiple Yosys versions.
- [ ] Security review of source ingestion, subprocess isolation, artifacts, and
  dependency supply chain.
- [ ] Operational documentation for installation, upgrades, support, incident
  response, and result retention.
- [ ] Independent technical review and successful design-partner pilots.
- [ ] Standards applicability documented without implying ISO 26262, IEC 61508,
  IEC 62304, or other certification that has not been obtained.

Until these boxes have evidence, external messaging must say “research preview”
or “design-partner evaluation”, never “production-certified”.
