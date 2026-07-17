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
- RTL safety reports and manifests declare compatibility-locked artifact schema
  version 3 and firmware CLI contract version 2.
- Named constant input assumptions are applied at every bounded frame, reject
  unresolved names, and are cross-checked by an independent SBY/Z3 model.
- Linux synthesis runs in a dedicated process group with a 2 GiB address-space
  limit, 512 MiB file limit, and group-wide timeout termination. Adversarial CI
  probes verify memory refusal, file truncation, and descendant cleanup.
- Artifact schema v3 has exact ordering and compatibility tests plus a strict
  executable bundle validator. Firmware CLI contract v2 fixes product command
  signatures and exit meanings. Direct AIGER input is bounded to 256 MiB.
- CI executes 25,000 deterministic mutations over persistent AIGER,
  assumptions, project-config, and CLI parser regression corpora.

## Required before a production claim

- [x] Explicit constant environment assumptions and constraints with
  independently checked semantics.
- [x] Include directories, parameter overrides, memories, and declared clock or
  reset policy for representative embedded RTL projects, with an independent
  SymbiYosys/Z3 oracle.
- [x] Stable artifact schema v3 and firmware CLI contract v2 with executable
  version queries, validators, and compatibility tests.
- [x] Linux memory, file-size, and process-tree limits in addition to wall-clock
  and model-size limits. macOS is explicitly development-only for this gate.
- [x] Bounded parser and CLI mutation fuzzing with persistent regression corpora.
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
