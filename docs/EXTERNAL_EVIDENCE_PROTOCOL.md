# External review and design-partner evidence protocol

This protocol defines the evidence required to close CQ-SAT/GCC's remaining
production gates. Publishing this protocol does not close them. Only completed,
independently attributable review records and pilot results can do that.

The review target must be one immutable release tag. Any code, schema, CLI,
isolation-profile, solver, synthesis-flow, or security-boundary change after the
review invalidates affected conclusions and requires documented impact review.

## Independence

An independent reviewer must not have authored or approved the implementation
being assessed and must be free to report adverse findings. Paid work is allowed
when scope, evidence access, and the right to report findings to the maintainer
are contractually preserved.

The security assessor must have demonstrated experience reviewing Linux
containers, shell boundaries, Rust parsers or native tools, and CI supply chains.
The technical reviewer must have demonstrated formal-verification, SAT/BMC, RTL
synthesis, or embedded safety-assurance experience. One person may satisfy both
roles only if both competencies and both review records are explicit.

Self-review, automated scanning alone, an AI-generated review, CI success, and a
partner testimonial without inspectable evidence do not satisfy independence.

## Independent security assessment

The assessor receives the release source, lockfile, public corpus, threat model,
operations runbook, isolation profile, evidence-schema contract, and the exact
CI commands. At minimum, the assessment must cover:

- project-config, RTL, include, AIGER, assumption, CLI, and artifact parsers;
- path traversal, symlink and special-file handling, races, partial publication,
  oversized input/output, malformed UTF-8 or binary data, and parser panics;
- Yosys process-tree containment, time, memory, file, process, descriptor, and
  model-size limits, including cleanup after interruption;
- isolation-wrapper argument handling, bind mounts, network state, privilege and
  capability state, seccomp, cgroup probes, image pinning, watchdog behaviour,
  signal handling, output validation, and trusted-host assumptions;
- artifact digest binding, manifest/validator disagreement, result-status
  confusion, incomplete evidence, replay, and malicious artifact-store changes;
- Rust and GitHub Actions dependency integrity, immutable pins, secret exposure,
  untrusted-fork behaviour, and release/tag controls; and
- denial of service and confidentiality impact on the documented single-tenant,
  ephemeral Linux deployment.

The assessor must attempt the documented attacks, not only read the controls.
At least the public malformed-input corpus, watchdog self-test, SAFE/UNSAFE
isolation runs, signal cleanup, output-overlap rejection, symlink rejection, and
artifact-tampering rejection must be reproduced on a clean Linux host.

The signed or otherwise attributable report must state target tag and commit,
environment, methods, tested commands, findings with severity and reproduction,
limitations, and a conclusion against the published threat model. Every
critical or high finding must be fixed and independently retested. Medium and
lower findings require an explicit disposition, owner, and deadline.

## Independent technical review

The technical reviewer must examine model semantics and result correctness,
including:

- RTL-to-AIGER synthesis scripts, hierarchy, parameters, includes, memories,
  reset/startup handling, environment assumptions, and named trace mapping;
- the exact meaning and bounded horizon of `SAFE` and `UNSAFE`;
- witness reconstruction and replay for every supported backend path;
- admission/fallback behaviour and the rule that refusal or tool failure can
  never become SAFE;
- independent-oracle ownership and the absence of shared implementation logic
  that would make a differential check circular;
- schema/CLI compatibility and evidence sufficient to reproduce a result; and
- documented exclusions and external assurance wording.

The reviewer must independently reproduce the public corpus and select at least
three additional adversarial models not authored by the maintainer: one SAFE,
one UNSAFE, and one expected tool/input failure. Result agreement, witness
replay, and exit semantics must be recorded. Material findings require correction
and independent retest before the technical-review gate can close.

## Design-partner pilot cohort

The production gate requires a cohort—not a single demonstration—with all of
the following minimum coverage:

- at least two independent embedded-product organisations;
- at least three distinct RTL projects across at least two product or control
  domains;
- at least 30 reviewed project/property configurations in aggregate, including
  at least five known SAFE, five known UNSAFE, and five expected rejection or
  tool-failure cases;
- single- and multi-file hierarchy, includes, parameter overrides, memories,
  reset/startup policy, and explicit environment assumptions somewhere in the
  cohort;
- at least two independently provisioned ephemeral Linux workers; and
- repetition on the same immutable CQ release, with every retained bundle
  passing the matching validator.

A configuration counts only when the partner owns and reviews the requirement,
model boundary, assumptions, bad outputs, and horizon. Synthetic examples may
supplement but not count toward the 30 partner configurations.

Before transfer, the maintainer and partner must agree confidential-data scope,
retention and deletion, authorised operators, incident contacts, allowed report
redaction, and whether execution occurs on a partner-owned worker. Confidential
RTL, traces, credentials, or identity data must never be attached to a public
GitHub issue or committed to this repository.

Each expected SAFE or UNSAFE result must be compared with an independently owned
oracle or pre-existing reviewed result. Expected failure cases must compare the
failure class and confirm that no SAFE evidence was published. Any disagreement
is unresolved until both systems' semantics and inputs are reconciled; silently
dropping disagreements is prohibited.

## Pilot acceptance criteria

The cohort passes only when all of these are true:

1. zero unresolved SAFE/UNSAFE disagreements;
2. zero tool/input failures reported as SAFE;
3. every UNSAFE result has a replayed, partner-triaged counterexample;
4. repeated runs agree on result and validate their complete evidence bundles;
5. all resource-limit, timeout, crash, and unsupported-input events are recorded
   as failures rather than omitted from the denominator;
6. no critical or high security finding remains open;
7. partner operators complete install, qualification, upgrade/rollback, incident,
   and evidence-retention exercises from the published runbook; and
8. an independent reviewer signs the aggregate conclusion and limitations.

Runtime and memory must be reported for every configuration, but no universal
speed threshold is imposed: acceptable latency is project-specific. The cohort
report must publish counts, result classes, disagreement/failure counts,
toolchain versions, resource distributions, and limitations. Confidential RTL,
property text, traces, and partner identity may remain private; redacted evidence
indices and attributable private review records must still exist.

## Evidence register

The maintainer must keep a versioned register with one row per review or pilot
configuration containing an opaque ID, partner/reviewer custodian, target tag,
environment ID, input and requirement record digests, expected result source,
CQ result, oracle result, exit class, validated-bundle digest, isolation-report
digest, runtime, peak memory, disposition, and review status.

[`EXTERNAL_EVIDENCE_REGISTER.csv`](EXTERNAL_EVIDENCE_REGISTER.csv) is the
canonical machine-readable schema. It is intentionally header-only until real
independent evidence exists. Records must use opaque custodian and environment
IDs; confidential identities and source material belong in the access-controlled
review record referenced by the final column, not in the public repository.

The production-readiness checklist may be changed only by a PR that links the
attributable reports and aggregate register, demonstrates every acceptance
criterion, and receives independent approval. Until then, external messaging
remains “research preview” or “design-partner evaluation”.
