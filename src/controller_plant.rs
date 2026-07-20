//! Exact sampled-control composition of a verified controller transducer and plant.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::aiger_obligation::{AigerOutcome, AigerTransition};
use crate::controller_mtbdd::{
    ControllerMtbddArtifact, ControllerMtbddSummary, evaluate_controller_mtbdd_unchecked,
    validate_controller_mtbdd_structure, verify_controller_mtbdd,
};
use crate::controller_mtbdd_proof::{
    ControllerMtbddEquivalenceProof, verify_controller_mtbdd_equivalence_proof,
};
use crate::controller_transducer::{
    ControllerTransducerObligation, ControllerTransducerSummary, verify_controller_transducer,
};

pub const CONTROLLER_PLANT_VERSION: u32 = 1;
pub const MAX_PLANT_INPUTS: usize = 12;
pub const MAX_EXTERNAL_PLANT_INPUTS: usize = 8;
pub const MAX_PLANT_LATCHES: usize = 8;
pub const MAX_PRODUCT_STATES: usize = 4_096;
pub const MAX_COMPOSITION_HORIZON: usize = 1_024;
pub const MAX_DIRECT_CONTROLLER_INPUTS: usize = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantWiring {
    pub controller_sensor_inputs: Vec<usize>,
    pub controller_action_outputs: Vec<usize>,
    pub plant_sensor_outputs: Vec<usize>,
    pub plant_action_inputs: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPlantAnswer {
    Safe,
    Unsafe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantTraceStep {
    pub frame: usize,
    pub controller_state: usize,
    pub plant_state: usize,
    pub sensor_pattern: usize,
    pub action_pattern: usize,
    pub controller_input: u64,
    pub plant_input: u64,
    pub bad: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantResult {
    pub version: u32,
    pub answer: ControllerPlantAnswer,
    pub horizon: usize,
    pub bad_frame: Option<usize>,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub trace: Vec<ControllerPlantTraceStep>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPlantBackend {
    ProofCarryingTransducer,
    DirectExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPlantSelectionReason {
    VerifiedArtifact,
    ArtifactUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantPortfolioResult {
    pub backend: ControllerPlantBackend,
    pub selection_reason: ControllerPlantSelectionReason,
    pub result: ControllerPlantResult,
}

#[derive(Debug)]
pub struct VerifiedControllerTransducer<'a> {
    obligation: &'a ControllerTransducerObligation,
    summary: ControllerTransducerSummary,
}

#[derive(Debug)]
pub struct VerifiedControllerMtbdd<'a> {
    artifact: &'a ControllerMtbddArtifact,
    summary: ControllerMtbddSummary,
}

impl VerifiedControllerMtbdd<'_> {
    pub fn summary(&self) -> &ControllerMtbddSummary {
        &self.summary
    }
}

impl VerifiedControllerTransducer<'_> {
    pub fn summary(&self) -> &ControllerTransducerSummary {
        &self.summary
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ControllerPlantBatchInput<'a> {
    pub plant: &'a AigerTransition,
    pub wiring: &'a ControllerPlantWiring,
    pub initial_controller_state: usize,
    pub initial_plant_state: usize,
    pub bad_plant_output: usize,
    pub horizon: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantBatchResult {
    pub members: Vec<ControllerPlantResult>,
    pub safe: usize,
    pub unsafe_count: usize,
    pub controller_cells: usize,
    pub controller_proof_bytes: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantError(pub String);

impl fmt::Display for ControllerPlantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ControllerPlantError {}

fn reject(message: impl Into<String>) -> ControllerPlantError {
    ControllerPlantError(message.into())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProductState {
    controller: usize,
    plant: usize,
}

#[derive(Clone, Copy)]
struct Predecessor {
    previous: ProductState,
    sensor_pattern: usize,
    action_pattern: usize,
    controller_input: u64,
    plant_input: u64,
}

fn projected_bits(value: u128, indices: &[usize]) -> usize {
    indices
        .iter()
        .enumerate()
        .fold(0usize, |projected, (bit, &index)| {
            projected | (((value >> index) as usize & 1) << bit)
        })
}

fn cell_for_pattern(
    obligation: &ControllerTransducerObligation,
    pattern: usize,
) -> Result<usize, ControllerPlantError> {
    let pattern_count = 1usize << obligation.relevant_inputs.len();
    if obligation.cells.len() == pattern_count
        && obligation.cells.get(pattern).is_some_and(|cell| {
            cell.cube
                .iter()
                .enumerate()
                .all(|(bit, required)| *required == Some(pattern >> bit & 1 == 1))
        })
    {
        return Ok(pattern);
    }
    let mut found = None;
    for (index, cell) in obligation.cells.iter().enumerate() {
        let allowed =
            cell.cube.iter().enumerate().all(|(bit, required)| {
                required.is_none_or(|value| (pattern >> bit & 1 == 1) == value)
            });
        if allowed && found.replace(index).is_some() {
            return Err(reject(
                "controller transducer cells overlap during composition",
            ));
        }
    }
    found.ok_or_else(|| reject("controller transducer does not cover sensed input"))
}

fn declared_plant_input(
    action_inputs: &[usize],
    external_inputs: &[usize],
    action_pattern: usize,
    external_pattern: usize,
) -> u64 {
    let action = action_inputs
        .iter()
        .enumerate()
        .fold(0u64, |input, (bit, &declared)| {
            input | (u64::from(action_pattern >> bit & 1 == 1) << declared)
        });
    external_inputs
        .iter()
        .enumerate()
        .fold(action, |input, (bit, &declared)| {
            input | (u64::from(external_pattern >> bit & 1 == 1) << declared)
        })
}

fn declared_controller_input(sensor_inputs: &[usize], sensor_pattern: usize) -> u64 {
    sensor_inputs
        .iter()
        .enumerate()
        .fold(0u64, |input, (bit, &declared)| {
            input | (u64::from(sensor_pattern >> bit & 1 == 1) << declared)
        })
}

fn validate_wiring_boundary(
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
) -> Result<Vec<usize>, ControllerPlantError> {
    plant
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    if wiring.controller_sensor_inputs != relevant_inputs
        || wiring.controller_action_outputs != observed_outputs
        || plant.inputs.len() > MAX_PLANT_INPUTS
        || plant.latches.len() > MAX_PLANT_LATCHES
        || wiring.plant_sensor_outputs.len() != relevant_inputs.len()
        || wiring.plant_action_inputs.len() != observed_outputs.len()
        || wiring
            .plant_sensor_outputs
            .iter()
            .any(|&output| output >= plant.outputs.len())
        || wiring
            .plant_sensor_outputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || wiring
            .plant_action_inputs
            .iter()
            .any(|&input| input >= plant.inputs.len())
        || wiring
            .plant_action_inputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("controller-plant wiring is invalid"));
    }
    let actions = wiring
        .plant_action_inputs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let external = (0..plant.inputs.len())
        .filter(|input| !actions.contains(input))
        .collect::<Vec<_>>();
    if external.len() > MAX_EXTERNAL_PLANT_INPUTS {
        return Err(reject(
            "controller-plant external input count exceeds limit",
        ));
    }
    Ok(external)
}

fn sensor_pattern(
    plant: &AigerTransition,
    sensor_outputs: &[usize],
    plant_state: usize,
) -> Result<usize, ControllerPlantError> {
    let mut expected = None;
    for input in 0..(1usize << plant.inputs.len()) {
        let (_, outputs) = plant
            .evaluate(plant_state, input as u64)
            .map_err(|error| reject(error.to_string()))?;
        let sensors = projected_bits(outputs, sensor_outputs);
        if expected.is_some_and(|value| value != sensors) {
            return Err(reject(
                "plant sensors depend on same-step inputs; sampled composition is inapplicable",
            ));
        }
        expected = Some(sensors);
    }
    expected.ok_or_else(|| reject("plant sensor evaluation produced no pattern"))
}

fn rebuild_trace(
    layers: &[BTreeMap<ProductState, Option<Predecessor>>],
    final_state: ProductState,
    final_step: ControllerPlantTraceStep,
) -> Result<Vec<ControllerPlantTraceStep>, ControllerPlantError> {
    let frame = final_step.frame;
    let mut state = final_state;
    let mut reverse = Vec::with_capacity(frame);
    for layer in (1..=frame).rev() {
        let predecessor = layers[layer]
            .get(&state)
            .and_then(|entry| *entry)
            .ok_or_else(|| reject("controller-plant trace predecessor is missing"))?;
        reverse.push(ControllerPlantTraceStep {
            frame: layer - 1,
            controller_state: predecessor.previous.controller,
            plant_state: predecessor.previous.plant,
            sensor_pattern: predecessor.sensor_pattern,
            action_pattern: predecessor.action_pattern,
            controller_input: predecessor.controller_input,
            plant_input: predecessor.plant_input,
            bad: false,
        });
        state = predecessor.previous;
    }
    reverse.reverse();
    reverse.push(final_step);
    Ok(reverse)
}

#[allow(clippy::too_many_arguments)]
fn explore_controller_plant(
    controller: &ControllerTransducerObligation,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantResult, ControllerPlantError> {
    explore_controller_plant_function(
        controller.state_count,
        &controller.relevant_inputs,
        &controller.observed_outputs,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_output,
        horizon,
        |state, sensors| {
            let cell_index = cell_for_pattern(controller, sensors)?;
            controller.cells[cell_index]
                .rows
                .get(state)
                .map(|row| row.outcome)
                .ok_or_else(|| reject("controller transducer state row is missing"))
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn explore_controller_plant_function<F>(
    controller_state_count: usize,
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
    outcome: F,
) -> Result<ControllerPlantResult, ControllerPlantError>
where
    F: Fn(usize, usize) -> Result<AigerOutcome, ControllerPlantError>,
{
    let external_inputs =
        validate_wiring_boundary(relevant_inputs, observed_outputs, plant, wiring)?;
    if horizon > MAX_COMPOSITION_HORIZON
        || initial_controller_state >= controller_state_count
        || initial_plant_state >= plant.state_count()
        || bad_plant_output >= plant.outputs.len()
        || controller_state_count
            .checked_mul(plant.state_count())
            .is_none_or(|states| states > MAX_PRODUCT_STATES)
    {
        return Err(reject("controller-plant query is outside static limits"));
    }
    let initial = ProductState {
        controller: initial_controller_state,
        plant: initial_plant_state,
    };
    let mut layers = vec![BTreeMap::from([(initial, None)])];
    let mut reachable_product_states = 1usize;
    let mut explored_transitions = 0usize;
    let mut sensor_cache = BTreeMap::new();
    for frame in 0..=horizon {
        let current = layers[frame].keys().copied().collect::<Vec<_>>();
        let mut next = BTreeMap::new();
        for state in current {
            let sensors = if let Some(&cached) = sensor_cache.get(&state.plant) {
                cached
            } else {
                let sensors = sensor_pattern(plant, &wiring.plant_sensor_outputs, state.plant)?;
                sensor_cache.insert(state.plant, sensors);
                sensors
            };
            let controller_outcome = outcome(state.controller, sensors)?;
            let action = usize::try_from(controller_outcome.outputs)
                .map_err(|_| reject("controller action pattern exceeds usize"))?;
            let controller_input =
                declared_controller_input(&wiring.controller_sensor_inputs, sensors);
            for external in 0..(1usize << external_inputs.len()) {
                let plant_input = declared_plant_input(
                    &wiring.plant_action_inputs,
                    &external_inputs,
                    action,
                    external,
                );
                let (next_plant, plant_outputs) = plant
                    .evaluate(state.plant, plant_input)
                    .map_err(|error| reject(error.to_string()))?;
                explored_transitions = explored_transitions
                    .checked_add(1)
                    .ok_or_else(|| reject("controller-plant transition count overflow"))?;
                let bad = plant_outputs >> bad_plant_output & 1 == 1;
                if bad {
                    let final_step = ControllerPlantTraceStep {
                        frame,
                        controller_state: state.controller,
                        plant_state: state.plant,
                        sensor_pattern: sensors,
                        action_pattern: action,
                        controller_input,
                        plant_input,
                        bad: true,
                    };
                    return Ok(ControllerPlantResult {
                        version: CONTROLLER_PLANT_VERSION,
                        answer: ControllerPlantAnswer::Unsafe,
                        horizon,
                        bad_frame: Some(frame),
                        reachable_product_states,
                        explored_transitions,
                        trace: rebuild_trace(&layers, state, final_step)?,
                    });
                }
                if frame < horizon {
                    let target = ProductState {
                        controller: controller_outcome.target,
                        plant: next_plant,
                    };
                    next.entry(target).or_insert(Some(Predecessor {
                        previous: state,
                        sensor_pattern: sensors,
                        action_pattern: action,
                        controller_input,
                        plant_input,
                    }));
                }
            }
        }
        if frame < horizon {
            reachable_product_states = reachable_product_states
                .checked_add(next.len())
                .ok_or_else(|| reject("controller-plant reachable count overflow"))?;
            layers.push(next);
        }
    }
    Ok(ControllerPlantResult {
        version: CONTROLLER_PLANT_VERSION,
        answer: ControllerPlantAnswer::Safe,
        horizon,
        bad_frame: None,
        reachable_product_states,
        explored_transitions,
        trace: Vec::new(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn compose_controller_plant(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    controller: &ControllerTransducerObligation,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantResult, ControllerPlantError> {
    let verified =
        verify_controller_for_composition(controller_model, controller_source_sha256, controller)?;
    compose_verified_controller_plant(
        &verified,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_output,
        horizon,
    )
}

pub fn verify_controller_for_composition<'a>(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    controller: &'a ControllerTransducerObligation,
) -> Result<VerifiedControllerTransducer<'a>, ControllerPlantError> {
    let summary =
        verify_controller_transducer(controller_model, controller_source_sha256, controller)
            .map_err(|error| reject(error.to_string()))?;
    Ok(VerifiedControllerTransducer {
        obligation: controller,
        summary,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn compose_verified_controller_plant(
    controller: &VerifiedControllerTransducer<'_>,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantResult, ControllerPlantError> {
    explore_controller_plant(
        controller.obligation,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_output,
        horizon,
    )
}

pub fn verify_mtbdd_for_composition<'a>(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    artifact: &'a ControllerMtbddArtifact,
) -> Result<VerifiedControllerMtbdd<'a>, ControllerPlantError> {
    let summary = verify_controller_mtbdd(controller_model, controller_source_sha256, artifact)
        .map_err(|error| reject(error.to_string()))?;
    Ok(VerifiedControllerMtbdd { artifact, summary })
}

pub fn verify_proof_carrying_mtbdd_for_composition<'a>(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    artifact: &'a ControllerMtbddArtifact,
    proof: &ControllerMtbddEquivalenceProof,
) -> Result<VerifiedControllerMtbdd<'a>, ControllerPlantError> {
    verify_controller_mtbdd_equivalence_proof(
        controller_model,
        controller_source_sha256,
        artifact,
        proof,
    )
    .map_err(|error| reject(error.to_string()))?;
    let state_bits =
        validate_controller_mtbdd_structure(artifact).map_err(|error| reject(error.to_string()))?;
    let summary = ControllerMtbddSummary {
        state_bits,
        inputs: artifact.relevant_inputs.len(),
        outputs: artifact.observed_outputs.len(),
        terminals: artifact.terminals.len(),
        nodes: artifact.nodes.len(),
        // The UNSAT miter establishes equivalence without replaying individual
        // state/input assignments. Do not report represented scope as work.
        assignments_checked: 0,
    };
    Ok(VerifiedControllerMtbdd { artifact, summary })
}

#[allow(clippy::too_many_arguments)]
pub fn compose_verified_mtbdd_plant(
    controller: &VerifiedControllerMtbdd<'_>,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantResult, ControllerPlantError> {
    explore_controller_plant_function(
        controller.artifact.state_count,
        &controller.artifact.relevant_inputs,
        &controller.artifact.observed_outputs,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_output,
        horizon,
        |state, sensors| {
            evaluate_controller_mtbdd_unchecked(
                controller.artifact,
                controller.summary.state_bits,
                state,
                sensors,
            )
            .map_err(|error| reject(error.to_string()))
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compose_controller_mtbdd_plant(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    artifact: &ControllerMtbddArtifact,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantResult, ControllerPlantError> {
    let verified =
        verify_mtbdd_for_composition(controller_model, controller_source_sha256, artifact)?;
    compose_verified_mtbdd_plant(
        &verified,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_output,
        horizon,
    )
}

pub fn compose_controller_plant_batch(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    controller: &ControllerTransducerObligation,
    members: &[ControllerPlantBatchInput<'_>],
) -> Result<ControllerPlantBatchResult, ControllerPlantError> {
    if members.is_empty() || members.len() > 64 {
        return Err(reject(
            "controller-plant batch member count is outside limit",
        ));
    }
    let verified =
        verify_controller_for_composition(controller_model, controller_source_sha256, controller)?;
    let mut results = Vec::with_capacity(members.len());
    let mut safe = 0usize;
    let mut unsafe_count = 0usize;
    let mut reachable_product_states = 0usize;
    let mut explored_transitions = 0usize;
    for member in members {
        let result = compose_verified_controller_plant(
            &verified,
            member.plant,
            member.wiring,
            member.initial_controller_state,
            member.initial_plant_state,
            member.bad_plant_output,
            member.horizon,
        )?;
        match result.answer {
            ControllerPlantAnswer::Safe => safe += 1,
            ControllerPlantAnswer::Unsafe => unsafe_count += 1,
        }
        reachable_product_states = reachable_product_states
            .checked_add(result.reachable_product_states)
            .ok_or_else(|| reject("controller-plant batch reachable count overflow"))?;
        explored_transitions = explored_transitions
            .checked_add(result.explored_transitions)
            .ok_or_else(|| reject("controller-plant batch transition count overflow"))?;
        results.push(result);
    }
    Ok(ControllerPlantBatchResult {
        members: results,
        safe,
        unsafe_count,
        controller_cells: verified.summary.cells,
        controller_proof_bytes: verified.summary.proof_bytes,
        reachable_product_states,
        explored_transitions,
    })
}

fn validate_direct_controller_boundary(
    controller_model: &AigerTransition,
    wiring: &ControllerPlantWiring,
) -> Result<Vec<usize>, ControllerPlantError> {
    controller_model
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    let sensor_set = wiring
        .controller_sensor_inputs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if controller_model.inputs.len() > MAX_DIRECT_CONTROLLER_INPUTS
        || wiring.controller_sensor_inputs.len() > controller_model.inputs.len()
        || sensor_set.len() != wiring.controller_sensor_inputs.len()
        || sensor_set
            .iter()
            .any(|&input| input >= controller_model.inputs.len())
        || wiring
            .controller_sensor_inputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || wiring.controller_action_outputs.len() != wiring.plant_action_inputs.len()
        || wiring
            .controller_action_outputs
            .iter()
            .any(|&output| output >= controller_model.outputs.len())
        || wiring
            .controller_action_outputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("direct controller boundary is outside exact limits"));
    }
    Ok((0..controller_model.inputs.len())
        .filter(|input| !sensor_set.contains(input))
        .collect())
}

#[allow(clippy::too_many_arguments)]
pub fn compose_controller_plant_direct(
    controller_model: &AigerTransition,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantResult, ControllerPlantError> {
    let omitted_inputs = validate_direct_controller_boundary(controller_model, wiring)?;
    explore_controller_plant_function(
        controller_model.state_count(),
        &wiring.controller_sensor_inputs,
        &wiring.controller_action_outputs,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_output,
        horizon,
        |source, pattern| {
            let controller_input =
                declared_controller_input(&wiring.controller_sensor_inputs, pattern);
            let mut expected = None;
            for omitted_pattern in 0..(1usize << omitted_inputs.len()) {
                let complete_input =
                    controller_input | declared_controller_input(&omitted_inputs, omitted_pattern);
                let (target, outputs) = controller_model
                    .evaluate(source, complete_input)
                    .map_err(|error| reject(error.to_string()))?;
                let outcome = AigerOutcome {
                    target,
                    outputs: projected_bits(outputs, &wiring.controller_action_outputs) as u128,
                };
                if expected.is_some_and(|value| value != outcome) {
                    return Err(reject(
                        "omitted direct controller inputs affect the exact outcome",
                    ));
                }
                expected = Some(outcome);
            }
            expected.ok_or_else(|| reject("direct controller outcome is missing"))
        },
    )
}

pub fn compose_controller_plant_direct_batch(
    controller_model: &AigerTransition,
    members: &[ControllerPlantBatchInput<'_>],
) -> Result<ControllerPlantBatchResult, ControllerPlantError> {
    if members.is_empty() || members.len() > 64 {
        return Err(reject(
            "controller-plant batch member count is outside limit",
        ));
    }
    let mut results = Vec::with_capacity(members.len());
    let mut safe = 0usize;
    let mut unsafe_count = 0usize;
    let mut reachable_product_states = 0usize;
    let mut explored_transitions = 0usize;
    for member in members {
        let result = compose_controller_plant_direct(
            controller_model,
            member.plant,
            member.wiring,
            member.initial_controller_state,
            member.initial_plant_state,
            member.bad_plant_output,
            member.horizon,
        )?;
        match result.answer {
            ControllerPlantAnswer::Safe => safe += 1,
            ControllerPlantAnswer::Unsafe => unsafe_count += 1,
        }
        reachable_product_states = reachable_product_states
            .checked_add(result.reachable_product_states)
            .ok_or_else(|| reject("controller-plant batch reachable count overflow"))?;
        explored_transitions = explored_transitions
            .checked_add(result.explored_transitions)
            .ok_or_else(|| reject("controller-plant batch transition count overflow"))?;
        results.push(result);
    }
    Ok(ControllerPlantBatchResult {
        members: results,
        safe,
        unsafe_count,
        controller_cells: 0,
        controller_proof_bytes: 0,
        reachable_product_states,
        explored_transitions,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn compose_controller_plant_portfolio(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    controller: Option<&ControllerTransducerObligation>,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantPortfolioResult, ControllerPlantError> {
    if let Some(controller) = controller {
        Ok(ControllerPlantPortfolioResult {
            backend: ControllerPlantBackend::ProofCarryingTransducer,
            selection_reason: ControllerPlantSelectionReason::VerifiedArtifact,
            result: compose_controller_plant(
                controller_model,
                controller_source_sha256,
                controller,
                plant,
                wiring,
                initial_controller_state,
                initial_plant_state,
                bad_plant_output,
                horizon,
            )?,
        })
    } else {
        Ok(ControllerPlantPortfolioResult {
            backend: ControllerPlantBackend::DirectExact,
            selection_reason: ControllerPlantSelectionReason::ArtifactUnavailable,
            result: compose_controller_plant_direct(
                controller_model,
                plant,
                wiring,
                initial_controller_state,
                initial_plant_state,
                bad_plant_output,
                horizon,
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiger_obligation::AigerLatch;
    use crate::controller_transducer::produce_controller_transducer;

    fn controller() -> AigerTransition {
        AigerTransition {
            max_variable: 2,
            inputs: vec![2],
            latches: vec![AigerLatch {
                current: 4,
                next: 2,
            }],
            outputs: vec![2],
            ands: vec![],
        }
    }

    fn plant() -> AigerTransition {
        AigerTransition {
            max_variable: 2,
            inputs: vec![2],
            latches: vec![AigerLatch {
                current: 4,
                next: 2,
            }],
            outputs: vec![4, 4],
            ands: vec![],
        }
    }

    #[test]
    fn exact_closed_loop_preserves_safe_and_unsafe_answers() {
        let controller_model = controller();
        let digest = [5; 32];
        let obligation =
            produce_controller_transducer(&controller_model, digest, &[0], &[0]).unwrap();
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: vec![0],
            controller_action_outputs: vec![0],
            plant_sensor_outputs: vec![0],
            plant_action_inputs: vec![0],
        };
        let safe = compose_controller_plant(
            &controller_model,
            digest,
            &obligation,
            &plant(),
            &wiring,
            0,
            0,
            1,
            8,
        )
        .unwrap();
        assert_eq!(safe.answer, ControllerPlantAnswer::Safe);
        let unsafe_result = compose_controller_plant(
            &controller_model,
            digest,
            &obligation,
            &plant(),
            &wiring,
            0,
            1,
            1,
            8,
        )
        .unwrap();
        assert_eq!(unsafe_result.answer, ControllerPlantAnswer::Unsafe);
        assert_eq!(unsafe_result.bad_frame, Some(0));
        assert_eq!(unsafe_result.trace.len(), 1);
        assert!(unsafe_result.trace[0].bad);
    }

    #[test]
    fn same_step_sensor_dependency_is_rejected() {
        let controller_model = controller();
        let digest = [6; 32];
        let obligation =
            produce_controller_transducer(&controller_model, digest, &[0], &[0]).unwrap();
        let input_sensor_plant = AigerTransition {
            outputs: vec![2, 4],
            ..plant()
        };
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: vec![0],
            controller_action_outputs: vec![0],
            plant_sensor_outputs: vec![0],
            plant_action_inputs: vec![0],
        };
        assert!(
            compose_controller_plant(
                &controller_model,
                digest,
                &obligation,
                &input_sensor_plant,
                &wiring,
                0,
                0,
                1,
                1,
            )
            .is_err()
        );
    }

    #[test]
    fn nondeterministic_external_input_reconstructs_exact_bad_trace() {
        let controller_model = controller();
        let digest = [8; 32];
        let obligation =
            produce_controller_transducer(&controller_model, digest, &[0], &[0]).unwrap();
        let disturbed_plant = AigerTransition {
            max_variable: 3,
            inputs: vec![2, 4],
            latches: vec![AigerLatch {
                current: 6,
                next: 4,
            }],
            outputs: vec![6, 6],
            ands: vec![],
        };
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: vec![0],
            controller_action_outputs: vec![0],
            plant_sensor_outputs: vec![0],
            plant_action_inputs: vec![0],
        };
        let result = compose_controller_plant(
            &controller_model,
            digest,
            &obligation,
            &disturbed_plant,
            &wiring,
            0,
            0,
            1,
            2,
        )
        .unwrap();
        assert_eq!(result.answer, ControllerPlantAnswer::Unsafe);
        assert_eq!(result.bad_frame, Some(1));
        assert_eq!(result.trace.len(), 2);
        assert_eq!(result.trace[0].controller_input, 0);
        assert_eq!(result.trace[0].plant_input, 2);
        assert!(!result.trace[0].bad);
        assert!(result.trace[1].bad);
    }

    #[test]
    fn transducer_and_independent_direct_baseline_agree_on_every_small_query() {
        let controller_model = controller();
        let digest = [10; 32];
        let obligation =
            produce_controller_transducer(&controller_model, digest, &[0], &[0]).unwrap();
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: vec![0],
            controller_action_outputs: vec![0],
            plant_sensor_outputs: vec![0],
            plant_action_inputs: vec![0],
        };
        for initial_controller in 0..2 {
            for initial_plant in 0..2 {
                for horizon in 0..=8 {
                    let accelerated = compose_controller_plant(
                        &controller_model,
                        digest,
                        &obligation,
                        &plant(),
                        &wiring,
                        initial_controller,
                        initial_plant,
                        1,
                        horizon,
                    )
                    .unwrap();
                    let direct = compose_controller_plant_direct(
                        &controller_model,
                        &plant(),
                        &wiring,
                        initial_controller,
                        initial_plant,
                        1,
                        horizon,
                    )
                    .unwrap();
                    assert_eq!(accelerated, direct);
                }
            }
        }
    }

    #[test]
    fn portfolio_routes_statically_and_never_masks_invalid_evidence() {
        let controller_model = controller();
        let digest = [11; 32];
        let obligation =
            produce_controller_transducer(&controller_model, digest, &[0], &[0]).unwrap();
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: vec![0],
            controller_action_outputs: vec![0],
            plant_sensor_outputs: vec![0],
            plant_action_inputs: vec![0],
        };
        let admitted = compose_controller_plant_portfolio(
            &controller_model,
            digest,
            Some(&obligation),
            &plant(),
            &wiring,
            0,
            0,
            1,
            8,
        )
        .unwrap();
        assert_eq!(
            admitted.backend,
            ControllerPlantBackend::ProofCarryingTransducer
        );
        let fallback = compose_controller_plant_portfolio(
            &controller_model,
            digest,
            None,
            &plant(),
            &wiring,
            0,
            0,
            1,
            8,
        )
        .unwrap();
        assert_eq!(fallback.backend, ControllerPlantBackend::DirectExact);
        assert_eq!(admitted.result, fallback.result);

        let mut tampered = obligation;
        tampered.cells[0].rows[0].proof.pop();
        assert!(
            compose_controller_plant_portfolio(
                &controller_model,
                digest,
                Some(&tampered),
                &plant(),
                &wiring,
                0,
                0,
                1,
                8,
            )
            .is_err()
        );
    }

    #[test]
    fn batch_verifies_controller_once_and_preserves_every_member_result() {
        let controller_model = controller();
        let plant_model = plant();
        let digest = [12; 32];
        let obligation =
            produce_controller_transducer(&controller_model, digest, &[0], &[0]).unwrap();
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: vec![0],
            controller_action_outputs: vec![0],
            plant_sensor_outputs: vec![0],
            plant_action_inputs: vec![0],
        };
        let members = [
            ControllerPlantBatchInput {
                plant: &plant_model,
                wiring: &wiring,
                initial_controller_state: 0,
                initial_plant_state: 0,
                bad_plant_output: 1,
                horizon: 8,
            },
            ControllerPlantBatchInput {
                plant: &plant_model,
                wiring: &wiring,
                initial_controller_state: 0,
                initial_plant_state: 1,
                bad_plant_output: 1,
                horizon: 8,
            },
        ];
        let batch =
            compose_controller_plant_batch(&controller_model, digest, &obligation, &members)
                .unwrap();
        assert_eq!((batch.safe, batch.unsafe_count), (1, 1));
        for (member, result) in members.iter().zip(&batch.members) {
            assert_eq!(
                *result,
                compose_controller_plant(
                    &controller_model,
                    digest,
                    &obligation,
                    member.plant,
                    member.wiring,
                    member.initial_controller_state,
                    member.initial_plant_state,
                    member.bad_plant_output,
                    member.horizon,
                )
                .unwrap()
            );
        }
        assert!(
            compose_controller_plant_batch(&controller_model, digest, &obligation, &[]).is_err()
        );
    }
}
