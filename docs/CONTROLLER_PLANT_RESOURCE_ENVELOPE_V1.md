# Controller/plant verification resource envelope v1

Status: experimental public Rust API, canonical file CLI, and typed process
client. This is a deterministic preflight mechanism, not production acceptance
evidence.

## Purpose

The equivalent-certificate comparison identified one reproducible GCC profile:
its verifier uses substantially less peak memory than the standard external
consumer on the frozen washing-controller batch. That result does not justify
automatic routing by itself. A product integration first needs an explicit way
to refuse a verification job whose conservative workload exceeds deployment
limits.

`ControllerPlantResourceEnvelope` supplies that boundary for the existing
controller MTBDD plant portfolio. It contains five caller-selected hard limits:

- artifact bytes;
- ordered batch members;
- maximum horizon for any member;
- maximum controller/plant product states for any member; and
- total conservative transition evaluations.

The assessment uses no measured time, learned threshold, formula-specific
calibration, or trial solve. For each member, the transition bound covers the
complete controller/plant state product, every requested frame, every external
plant input pattern and, on the direct-exact route, every omitted controller
input evaluation. Checked arithmetic rejects an unrepresentable bound.

## Public API

Callers construct a validated envelope with
`ControllerPlantResourceEnvelope::new`. The limits cannot exceed GCC's frozen
static artifact, batch, horizon, or product-state boundaries. Two public entry
points then provide separate integration choices:

- `assess_controller_mtbdd_plant_portfolio_resources` validates the canonical
  outer portfolio and returns its deterministic backend and workload bound
  without controller/plant reachability replay.
- `verify_controller_mtbdd_plant_portfolio_with_resources` performs the same
  preflight and invokes the existing independent exact verifier only when every
  limit fits.

The governed result keeps the resource assessment separate from the ordinary
verification summary. A resource refusal is an error and is never converted
into SAFE or UNSAFE. The existing MTBDD admission and direct-exact fallback
semantics remain unchanged.

## Self-service policy and CLI

Policy v1 is canonical newline-terminated text with these ordered fields:

```text
controller_plant_resource_policy_version=1
max_artifact_bytes=16777216
max_members=2
max_member_horizon=32
max_product_states_per_member=4096
max_transition_evaluations=270336
status=complete
```

The executable exposes a separate capability contract and governed verifier:

```sh
guarded-continuation-checker controller-plant-resource-cli-version
guarded-continuation-checker verify-controller-plant-portfolio-resources \
  MANIFEST.txt POLICY.txt INPUT.controller-plant
```

`ControllerPlantResourceTool` validates the complete capability contract,
invokes the command without a shell under the existing execution controls, and
returns typed resource, aggregate, member, and invocation observations. Policy
files are bounded to 4 KiB. Invalid UTF-8, NUL, CRLF, missing, trailing,
misordered, noncanonical numeric, zero, and over-limit values fail closed.

## Retained mechanism tests

The Rust integration test covers:

- an admitted MTBDD artifact;
- a boundary-rejected artifact using exact direct fallback;
- exact SAFE and UNSAFE replay on the governed path;
- equality at each inclusive resource boundary;
- refusal one unit below artifact, horizon, product-state, and transition
  requirements;
- empty-member refusal; and
- artifact-size refusal before a corrupt artifact reaches integrity parsing.

## Open gates

This first slice does not yet close the production resource-governance row.
Before it can be used as a production deployment policy, GCC still needs:

1. stable machine-readable refusal classes instead of a shared tool-error exit;
2. Linux process-limit tests showing the conservative envelope agrees with
   enforced memory, file, output, and deadline controls;
3. multi-job aggregation with admitted and refused workload counters; and
4. simulated constrained firmware-CI acceptance followed by independent use.

The arithmetic envelope is conservative. Admission means the requested static
work bound fits the caller's policy, not that a wall-clock deadline or exact
peak-memory amount is guaranteed. Existing subprocess controls remain necessary.
