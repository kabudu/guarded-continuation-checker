# GCC documentation map

This index keeps detailed contracts, qualification evidence and research
records out of the project landing page while preserving direct routes to the
material needed by evaluators, integrators and contributors.

## Start here

- [Architecture and trust boundary](ARCHITECTURE.md)
- [Production support profile v1](PRODUCTION_SUPPORT_PROFILE_V1.md)
- [Production readiness](PRODUCTION_READINESS.md)
- [Production-readiness gap register](PRODUCTION_READINESS_GAP.md)
- [Novelty gap register](NOVELTY_GAP.md)
- [Operations](OPERATIONS.md)
- [Reproducibility](REPRODUCIBILITY.md)
- [Compatibility and migration](COMPATIBILITY_AND_MIGRATION.md)

## Firmware and RTL evaluation

- [Firmware CLI contract v2](FIRMWARE_CLI_V2.md)
- [RTL artifact schema v4](ARTIFACT_SCHEMA_V4.md)
- [Isolation profile v1](ISOLATION_PROFILE_V1.md)
- [Linux evaluation bundle v1](LINUX_EVALUATION_BUNDLE_V1.md)
- [Linux production candidate v1](LINUX_PRODUCTION_CANDIDATE_V1.md)
- [External evidence protocol](EXTERNAL_EVIDENCE_PROTOCOL.md)
- [Design-partner brief](DESIGN_PARTNER_BRIEF.md)
- [Pilot intake template](PILOT_INTAKE_TEMPLATE.md)
- [Outcome report template](OUTCOME_REPORT_TEMPLATE.md)

The executable product examples are under
[`examples/products`](../examples/products). The infusion-pump example covers
single-file, multi-file, configured and constrained RTL safety gates.

## Public interfaces and evidence

- [Predicate Rust API v1](PREDICATE_RUST_API_V1.md)
- [Predicate CLI v1](PREDICATE_CLI_V1.md)
- [Event-contract CLI and Rust API v1](EVENT_CONTRACT_CLI_V1.md)
- [Controller MTBDD CLI v1](CONTROLLER_MTBDD_CLI_V1.md)
- [Controller proof MTBDD CLI v1](CONTROLLER_PROOF_MTBDD_CLI_V1.md)
- [Split-evidence CLI v1](CONTROLLER_SPLIT_EVIDENCE_CLI_V1.md)
- [Revision-local CLI v1](REVISION_LOCAL_CLI_V1.md)
- [Source-to-model attestation v1](SOURCE_MODEL_ATTESTATION_V1.md)
- [Process-client observability v1](PROCESS_CLIENT_OBSERVABILITY_V1.md)

These interfaces are research surfaces unless the production support profile
explicitly includes them.

## Architecture and controller composition

- [Controller MTBDD plant portfolio v1](CONTROLLER_MTBDD_PLANT_PORTFOLIO_V1.md)
- [Controller plant resource envelope v1](CONTROLLER_PLANT_RESOURCE_ENVELOPE_V1.md)
- [Governed proof MTBDD portfolio v1](GOVERNED_PROOF_MTBDD_PORTFOLIO_V1.md)
- [Proof-carrying MTBDD equivalence v1](PROOF_CARRYING_MTBDD_EQUIVALENCE_V1.md)
- [Proof-carrying controller transducer v1](PROOF_CARRYING_CONTROLLER_TRANSDUCER_V1.md)
- [Public washing-controller experiment v1](PUBLIC_WASHING_CONTROLLER_EXPERIMENT_V1.md)
- [Public washing physical plant v1](PUBLIC_WASHING_PHYSICAL_PLANT_V1.md)

## Compiled firmware and MMIO research

- [Compiled MMIO contract v1](OPENTITAN_PWM_COMPILED_MMIO_CONTRACT_V1.md)
- [Firmware transaction contract v1](OPENTITAN_PWM_FIRMWARE_TRANSACTION_CONTRACT_V1.md)
- [Exact explicit-transcript baseline v1](OPENTITAN_PWM_EXPLICIT_TRANSCRIPT_BASELINE_V1.md)
- [Predicate certificate experiment v1](OPENTITAN_PWM_PREDICATE_CERTIFICATE_EXPERIMENT_V1.md)
- [Live-state quotient experiment v1](OPENTITAN_PWM_LIVE_STATE_QUOTIENT_EXPERIMENT_V1.md)
- [Live-slice quotient experiment v1](OPENTITAN_PWM_LIVE_SLICE_QUOTIENT_EXPERIMENT_V1.md)
- [Branching continuation DAG v1](OPENTITAN_PWM_BRANCHING_CONTINUATION_DAG_V1.md)
- [Multi-successor decode graph v2](OPENTITAN_PWM_MULTISUCCESSOR_DECODE_GRAPH_V2.md)

The documents retain both passing and falsified mechanisms. A research result
does not enter the production support profile merely by existing in the
repository.

## BTOR2, revision and transport research

- [BTOR2 component contract v1](BTOR2_COMPONENT_CONTRACT_V1.md)
- [BTOR2 channel property CLI v1](BTOR2_CHANNEL_PROPERTY_CLI_V1.md)
- [Revision impact certificate v1](REVISION_IMPACT_CERTIFICATE_V1.md)
- [Revision batch certificate v1](REVISION_BATCH_CERTIFICATE_V1.md)
- [QatQ transport qualification v1](QATQ_TRANSPORT_QUALIFICATION_V1.md)

## Research record

- [Findings](FINDINGS.md)
- [Prior-art audit v1](PRIOR_ART_AUDIT_V1.md)
- [Firmware and robotics research roadmap](FIRMWARE_ROBOTICS_RESEARCH_ROADMAP.md)
- [Standards applicability](STANDARDS_APPLICABILITY.md)

Curated machine-readable results are under [`results`](../results). Experiment
documents contain their exact claim boundaries, frozen gates and reproduction
commands.

## Project policy

- [Brand and naming](BRAND.md)
- [Release-note style](RELEASE_NOTES_STYLE.md)
- [Website publication](WEBSITE.md)
- [Security policy and threat model](../SECURITY.md)
- [Contributing](../CONTRIBUTING.md)
