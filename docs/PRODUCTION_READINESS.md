# Production-readiness gates

Guarded Continuation Checker remains a research preview until every required gate below has direct,
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
  version 4 and firmware CLI contract version 2.
- Named constant input assumptions are applied at every bounded frame, reject
  unresolved names, and are cross-checked by an independent SBY/Z3 model.
- Linux synthesis runs in a dedicated process group with a 2 GiB address-space
  limit, 512 MiB file limit, and group-wide timeout termination. Adversarial CI
  probes verify memory refusal, file truncation, and descendant cleanup.
- Artifact schema v4 has exact ordering, SHA-256 evidence binding, symlink
  rejection, compatibility tests, and a strict
  executable bundle validator. Firmware CLI contract v2 fixes product command
  signatures and exit meanings. Direct AIGER input is bounded to 256 MiB.
- CI executes 25,000 deterministic mutations over persistent AIGER,
  assumptions, project-config, and CLI parser regression corpora.
- Certified causal analysis has exact segment semantics, conservative resource
  limits, three-way CQ/persistent-CDCL/fresh-CDCL agreement, a replaying
  certificate verifier, and durable atomic no-clobber evidence bundles. Its
  research novelty and external RTL usefulness remain unvalidated.
- A revision-pinned public corpus combines five unmodified Yosys RTL sources
  with twelve separately owned properties (six SAFE and six UNSAFE). All twelve
  match expected results on Yosys 0.67+post and digest-pinned Yosys 0.36; the
  modern run also agrees with an independent SymbiYosys/Z3 oracle.
- CI actions are pinned to immutable commits, Rust dependencies are checksum-
  locked, RustSec auditing is required on every PR, and Dependabot monitors both
  Cargo and GitHub Actions dependencies.
- Hostile-RTL isolation profile v1 runs a non-root, networkless, read-only,
  capability-free container with seccomp and cgroup-v2 limits, probes those
  controls before parsing, and validates output in a second read-only container.

## Required before a production claim

- [x] Explicit constant environment assumptions and constraints with
  independently checked semantics.
- [x] Include directories, parameter overrides, memories, and declared clock or
  reset policy for representative embedded RTL projects, with an independent
  SymbiYosys/Z3 oracle.
- [x] Stable artifact schema v4 and firmware CLI contract v2 with executable
  version queries, validators, and compatibility tests.
- [x] Linux memory, file-size, and process-tree limits in addition to wall-clock
  and model-size limits. macOS is explicitly development-only for this gate.
- [x] Bounded parser and CLI mutation fuzzing with persistent regression corpora.
- [x] Differential validation across a small revision-pinned public RTL corpus,
  including SAFE and UNSAFE properties and multiple Yosys versions.
- [ ] Differential validation across a substantial design-partner RTL corpus,
  including the cohort, coverage, oracle, and acceptance requirements in
  `EXTERNAL_EVIDENCE_PROTOCOL.md`.
- [x] In-repository security review covering source ingestion, subprocess and
  hostile-RTL isolation, artifact integrity, and dependency supply chain.
- [ ] Independent external security assessment of the documented threat model
  and hostile-RTL isolation boundary against `EXTERNAL_EVIDENCE_PROTOCOL.md`.
- [x] Operational documentation and executable qualification for installation,
  upgrades, rollback, support, incident response, restoration, and result
  retention.
- [ ] Independent technical review and successful design-partner pilots meeting
  the independence, reproduction, evidence-register, and acceptance protocol.
- [x] Standards applicability and permitted assurance wording documented for
  ISO 26262, IEC 61508, IEC 62304, FDA infusion-pump guidance, and IEC
  81001-5-1 without implying conformity, certification, or tool qualification.

Until these boxes have evidence, external messaging must say “research preview”
or “design-partner evaluation”, never “production-certified”.
