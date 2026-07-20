# Controller/plant verification resource envelope v1

Status: experimental public Rust API. This is a deterministic preflight
mechanism, not production acceptance evidence.

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
Before it can be used as a self-service deployment policy, GCC still needs:

1. a canonical bounded policy file and CLI capability contract;
2. typed process-client support and stable refusal classes;
3. hostile policy parser and no-clobber coverage;
4. Linux process-limit tests showing the conservative envelope agrees with
   enforced memory, file, output, and deadline controls;
5. multi-job aggregation with admitted and refused workload counters; and
6. simulated constrained firmware-CI acceptance followed by independent use.

The arithmetic envelope is conservative. Admission means the requested static
work bound fits the caller's policy, not that a wall-clock deadline or exact
peak-memory amount is guaranteed. Existing subprocess controls remain necessary.

