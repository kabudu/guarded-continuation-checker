# Governed proof-carrying MTBDD portfolio v1

Status: experimental Rust API, canonical file policy and CLI, proof/direct
portfolio, and strict typed portfolio process client. Public-product acceptance
and compatibility gates remain open.

## Problem

The controller/plant resource envelope merged in v0.28 development governs the
ordinary MTBDD/direct portfolio. Its admitted MTBDD verifier establishes
controller equivalence by exhaustive assignment replay. GCC also has a
proof-carrying MTBDD artifact whose independently checked UNSAT miter avoids
that replay, but it does not yet share the governed self-service boundary.

This experiment joins those mechanisms without weakening either one. A caller
must be able to bound proof checking and plant composition separately, reject
excess work before either begins, and preserve the original exact query when a
static portfolio boundary is unsupported.

## First API slice

`ControllerProofMtbddResourceEnvelope` contains the ordinary composition
envelope plus caller-selected limits for:

- the encoded equivalence artifact; and
- its embedded UNSAT proof.

`assess_controller_proof_mtbdd_plant_resources` decodes the bounded canonical
outer artifact, checks both proof limits, binds every ordered member, and
computes the same conservative horizon, product-state, external-input, and
transition bounds used by the ordinary governed path. It performs no UNSAT
proof checking or controller/plant reachability replay.

`verify_controller_proof_mtbdd_plant_artifact_with_resources` runs the existing
independent proof-carrying verifier only after that assessment succeeds. The
governed result keeps the deterministic resource assessment separate from the
logical verification summary. The retained test proves both answer classes and
requires `assignments_checked=0`.

## Self-service contract

Policy v1 is canonical LF text with ordered fields for the outer artifact,
equivalence artifact, embedded UNSAT proof, members, horizon, product states,
and transition evaluations. The executable exposes separate version discovery
and verification commands:

```sh
guarded-continuation-checker controller-proof-mtbdd-resource-cli-version
guarded-continuation-checker verify-controller-proof-mtbdd-plant-resources \
  MANIFEST.txt POLICY.txt INPUT.proof-mtbdd-plant
```

Valid resource refusal returns exit code 3, no logical answer, and one of seven
`proof-reason-v1` values. Malformed policy, corrupt evidence, boundary drift,
and ordinary tool failures remain exit code 2.
`ControllerProofMtbddResourceTool` discovers and validates the full contract,
invokes the verifier without a shell under the shared execution policy, checks
every aggregate and member field, and maps valid refusals to typed invocation
metrics. CRLF, NUL, missing, trailing, noncanonical, zero, and over-limit policy
controls fail closed.

## Static proof/direct portfolio

The `GCCPGP01` outer artifact records the selected backend, structural reason,
controller boundary, and complete payload. Production selects the
proof-carrying MTBDD backend only when MTBDD construction is admitted. The
three existing structural rejections select the direct exact payload. Proof
production, proof encoding, malformed-model, semantic, and checker failures
propagate and never trigger fallback.

Verification replays the same structural admission decision. A direct payload
for an admitted MTBDD is rejected as a downgrade. Both routes bind the complete
ordered member query and return identical SAFE and UNSAFE results. The proof
route requires zero exhaustive controller assignments. Every mutation of the
retained outer artifact fails closed.

The outer portfolio has its own capability, certification, verification, and
governed-verification commands:

```sh
guarded-continuation-checker controller-proof-mtbdd-portfolio-cli-version
guarded-continuation-checker certify-controller-proof-mtbdd-portfolio \
  MANIFEST.txt OUTPUT.proof-mtbdd-portfolio
guarded-continuation-checker verify-controller-proof-mtbdd-portfolio \
  MANIFEST.txt INPUT.proof-mtbdd-portfolio
guarded-continuation-checker verify-controller-proof-mtbdd-portfolio-resources \
  MANIFEST.txt POLICY.txt INPUT.proof-mtbdd-portfolio
```

The admitted proof route passes all four file interfaces, preserves both answer
classes, reports its structural route, and checks zero exhaustive controller
assignments. A proof budget below the embedded proof returns the same typed
exit-code-3 refusal contract before proof checking.

`ControllerProofMtbddPortfolioTool` validates the complete capability tuple and
every response field without invoking a shell. It reports bounded process
metrics, converts exit-code-3 refusals into typed reasons, rejects inconsistent
backend/reason pairs, and accepts the exact fallback route with zero proof bytes.

## Public workflow acceptance

`scripts/run-governed-proof-portfolio-acceptance.sh` exercises six release-build
jobs through documented file interfaces. The positive jobs cover the pinned
public washing controller with six stateful plant properties and a structurally
rejected seven-latch controller routed to exact replay. They preserve 3 SAFE and
5 UNSAFE results. The public proof route checks zero exhaustive assignments.

The negative jobs require a typed proof-budget refusal, a typed transition-budget
refusal, malformed-policy rejection, and corrupt-artifact rejection. Refusal and
invalid-input rows carry no logical answer. The retained Linux result is
[`results/governed-proof-mtbdd-portfolio-acceptance-linux-v1.csv`](../results/governed-proof-mtbdd-portfolio-acceptance-linux-v1.csv).
CI regenerates it under a 64 MiB address-space ceiling and requires byte equality.
The same six rows are byte-identical on macOS and Linux; timing is deliberately
excluded from the acceptance record. The two positive rows freeze canonical
artifact SHA-256 fingerprints. They establish the baseline for a future tagged
release compatibility test; they do not by themselves constitute compatibility
history.

## Predeclared gates

| Gate | Required result |
|---|---|
| Exactness | Every member and trace agrees with the existing proof verifier and direct exact baseline |
| Proof governance | Artifact, equivalence-artifact, and embedded-proof limits reject before proof checking |
| Composition governance | Member, horizon, product-state, and transition limits reject before semantic replay |
| Query binding | Ordered source digests, wiring, initial states, property, and horizon cannot drift |
| Stable self-service API | Implemented experimentally with policy, capability, CLI response, typed process client, and refusal classes versioned |
| Static portfolio | Library, file CLI, strict typed process client, and proof/direct acceptance implemented |
| Hostile input | Truncation, mutation, noncanonical policy, boundary drift, and oversize inputs fail closed |
| Public product | Passed locally and in Linux for the revision-pinned washing controller and physical-plant batch |
| Strong baseline | Report proof checking against exhaustive equivalence and maintained proof consumers at identical scope |
| Resource evidence | Local Linux enforces process limits around every governed job; hosted reproduction remains required |
| Compatibility | Frozen v1 fixtures survive at least one subsequent release |
| Independent acceptance | A non-repository-authored constrained workflow reports outcome and suitability |

## Claim boundary

Resource admission, SAT miters, UNSAT proof checking, MTBDDs, exact fallback,
and process limits are established techniques. This integration is production
hardening, not a novelty result. It advances the reusable proof-carrying product
path while the separate novelty register still requires a distinguishing
algorithmic result, closest-prior-art review, and independent evidence.
