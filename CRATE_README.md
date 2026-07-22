<p align="center">
  <img src="https://raw.githubusercontent.com/kabudu/guarded-continuation-checker/master/assets/brand/logo-horizontal.svg" width="680" alt="Guarded Continuation Checker, powered by CQ-SAT">
</p>

# Guarded Continuation Checker

Guarded Continuation Checker is an evaluation-ready, proof-carrying bounded
verification platform for embedded firmware and RTL, powered by the CQ-SAT
continuation-quotient backend.

The package provides:

- the `guarded-continuation-checker` command-line application;
- the `guarded_continuation_checker` Rust library for bounded, resource-governed
  production and independent checking of predicate and event-contract
  certificates;
- ASCII and binary AIGER ingestion, multi-file SystemVerilog/Yosys workflows,
  named assumptions, replayable counterexamples and versioned evidence bundles;
- a static specialised-backend gate with exact persistent-CDCL fallback;
- deterministic certificate formats with independent source-bound verification;
- qualified Certifaiger plus `lrat_isa` SAFE checking and `aigsim` UNSAFE trace
  replay in the repository evidence harness; and
- experimental BTOR2 word semantics, counter-phase certificates, exact trace
  replay, proof-carrying exact word regions, coupled-motion curves, resettable
  braking phases, source-separated controller/plant contracts, and both-answer
  bounded reachability with exact environment constraints, static exact-search
  fallbacks, and fixed resource limits.
- an experimental source-replayed BTOR2 repeated-channel property portfolio
  with canonical query and policy files, exact explicit-state or proof-carrying
  bitblast routing, aggregate preflight, a no-clobber CLI, and a typed bounded
  process client. An additive observed CLI exposes diagnostic phase timings
  without making timing part of admission or correctness, and the typed client
  strictly parses the observed schema.

## Status and claim boundary

This package is an **evaluation-ready research prototype**. It is not certified,
production-qualified, a general SAT replacement, or evidence that an entire
device is safe. A `SAFE` result is bounded by the reviewed model, assumptions,
property, reset/startup policy and horizon.

The project publishes negative measurements and tracks the remaining
[production-readiness](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/PRODUCTION_READINESS_GAP.md)
and
[novelty](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/NOVELTY_GAP.md)
gaps explicitly.

The retained public-design evidence covers pinned OpenTitan timer RTL and an
independently maintained CHIPS Alliance Caliptra watchdog module. The Caliptra
baseline preserves five SAFE and four UNSAFE bounded outcomes, deterministic
evidence, independently checked certificates and traces, verified
multi-property witness composition, and hostile-evidence rejection. These are
narrow configured-core results, not assurance for either complete product.

The experimental exact bounded-search interface accepts state-only properties
through retained certificate v1 and current-input-dependent properties through
additive certificate v2. V2 represents asynchronous-reset semantics with a
separate terminal-frame input and preserves v1 encoding unchanged. Both
formats remain bounded research interfaces rather than production assurance.

Additive certificate v3 admits two through eight one-bit semantic inputs and
binds complete packed transition and terminal valuations. Retained v1 and v2
encodings remain byte-identical. The v3 path is also experimental and subject
to the same explicit state, work, horizon, and artifact limits.

Additive certificates v4 and v5 preserve exact BTOR2 constraints and small
word-valued semantic inputs respectively. V5 source-binds input widths and
packs at most eight total input bits by input-node order, then
least-significant bit first within each word. All five formats remain bounded
research interfaces. Hosted amd64 run 29874337371 reproduces the complete v5
Caliptra public-design result and compatibility matrix.

Bounded portfolio v3 source-binds and independently checks the exact
accelerate, brake, and stopped relation, with unchanged exact-search fallback
for every unsupported or intersecting query. This remains a narrow experimental
result, not a production or novelty claim.

The experimental `btor2_component` API separately binds a controller, plant,
and synchronous wiring contract. It supplies an exact phase certificate and a
both-answer composed-search fallback without generating a monolithic source.
The first cost result is deliberately retained as negative against the existing
monolithic specialisation, so no performance or novelty claim follows.

## Installation

Install the complete research build:

```sh
cargo install guarded-continuation-checker --locked
```

For the frozen `firmware-rtl-v1` command boundary, install with the production
feature. This build rejects every research command before dispatch:

```sh
cargo install guarded-continuation-checker --locked \
  --features production-firmware
guarded-continuation-checker production-profile-version
```

The optional `research-qatq-transport` feature exposes a bounded, exact QatQ
0.1.5 transport envelope for research evaluation of large canonical evidence
batches. It is not enabled by default, is not included by
`production-firmware`, and does not change certificate semantics. Recover the
canonical bytes and run their existing independent verifier before trusting an
answer.

The public `revision_impact` module is a research interface for bounded
firmware-regression experiments. It encodes complete old/new
counterfactual tables, derives inclusion-minimal invalidating sets, binds every
observation to canonical revision-local evidence, and requires independent
semantic verification. It is not part of the `firmware-rtl-v1` production
support profile.

To evaluate an unreleased reviewed repository revision instead:

```sh
cargo install --git https://github.com/kabudu/guarded-continuation-checker \
  --rev <reviewed-commit> --locked
```

For self-service Linux evaluation, prefer the repository's reproducible static
`firmware-rtl-v1` candidate. It includes an SPDX SBOM, deterministic provenance,
internal checksums, and GitHub Sigstore attestations. No binary should be
treated as an authenticated release without verifying its attestation policy.

Confirm the versioned interfaces before integrating:

```sh
guarded-continuation-checker firmware-cli-version
guarded-continuation-checker predicate-cli-version
guarded-continuation-checker event-contract-cli-version
guarded-continuation-checker btor2-cli-version
guarded-continuation-checker btor2-channel-property-cli-version
guarded-continuation-checker controller-mtbdd-cli-version
```

## Rust API

```rust,no_run
use guarded_continuation_checker::{
    Btor2ChannelPropertyTool, ControllerMtbddTool, ControllerPlantPortfolioTool,
    EventContractTool, ExecutionPolicy, PredicateTool,
};

# fn discover() -> Result<(), Box<dyn std::error::Error>> {
let policy = ExecutionPolicy::default();
let _channel_properties = Btor2ChannelPropertyTool::discover(
    "guarded-continuation-checker",
)?;
let tool = PredicateTool::discover_with_policy(
    "guarded-continuation-checker",
    policy,
)?;
let capabilities = tool.capabilities();
assert_eq!(capabilities.cli_version, 1);
let event_contracts = EventContractTool::discover_with_policy(
    "guarded-continuation-checker",
    policy,
)?;
assert_eq!(event_contracts.capabilities().cli_version, 1);
let controller_mtbdd = ControllerMtbddTool::discover_with_policy(
    "guarded-continuation-checker",
    policy,
)?;
assert_eq!(controller_mtbdd.capabilities().cli_version, 1);
let controller_portfolio = ControllerPlantPortfolioTool::discover_with_policy(
    "guarded-continuation-checker",
    policy,
)?;
assert_eq!(controller_portfolio.capabilities().cli_version, 1);
# Ok(())
# }
```

The typed client invokes the executable without a shell, validates its
machine-readable capability contract, applies bounded execution policy and
reports stable failure classes and invocation metrics. See the
[Rust API contract](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/PREDICATE_RUST_API_V1.md)
for predicate certificate examples and the
[event-contract API contract](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/EVENT_CONTRACT_CLI_V1.md)
for certificate v3, exact portfolio fallback, and report replay. The
[controller MTBDD CLI contract](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/CONTROLLER_MTBDD_CLI_V1.md)
covers source-bound controller-plus-plant batch production and verification.
The
[exact controller plant portfolio](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/CONTROLLER_MTBDD_PLANT_PORTFOLIO_V1.md)
uses deterministic MTBDD admission with direct exact fallback and downgrade
detection.
Its typed summaries include non-routing phase observations so integrations can
attribute loading, artifact, replay, publication, and total command cost.
The retained
[process-resource baseline](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/CONTROLLER_MTBDD_PROCESS_RESOURCES_V1.md)
records a negative small-batch speed result and lower peak-memory observations
without equating the different GCC and formal-oracle scopes.

## Self-service evaluation

Teams can keep private RTL on a partner-owned ephemeral Linux worker, follow the
published isolation and operations guidance, compare results with an
independently owned oracle, and return only a non-confidential outcome and
suitability assessment. Start with the
[design-partner brief](https://github.com/kabudu/guarded-continuation-checker/blob/master/docs/DESIGN_PARTNER_BRIEF.md).

## Licence and security

The code is available under Apache-2.0. Report suspected vulnerabilities through
[a private GitHub Security Advisory](https://github.com/kabudu/guarded-continuation-checker/security/advisories/new),
not a public issue.
