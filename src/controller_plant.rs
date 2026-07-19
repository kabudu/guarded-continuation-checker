//! Exact sampled-control composition of a verified controller transducer and plant.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::aiger_obligation::AigerTransition;
use crate::controller_transducer::{ControllerTransducerObligation, verify_controller_transducer};

pub const CONTROLLER_PLANT_VERSION: u32 = 1;
pub const MAX_PLANT_INPUTS: usize = 12;
pub const MAX_EXTERNAL_PLANT_INPUTS: usize = 8;
pub const MAX_PLANT_LATCHES: usize = 8;
pub const MAX_PRODUCT_STATES: usize = 4_096;
pub const MAX_COMPOSITION_HORIZON: usize = 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantWiring {
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

fn validate_wiring(
    controller: &ControllerTransducerObligation,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
) -> Result<Vec<usize>, ControllerPlantError> {
    plant
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    if plant.inputs.len() > MAX_PLANT_INPUTS
        || plant.latches.len() > MAX_PLANT_LATCHES
        || wiring.plant_sensor_outputs.len() != controller.relevant_inputs.len()
        || wiring.plant_action_inputs.len() != controller.observed_outputs.len()
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
    verify_controller_transducer(controller_model, controller_source_sha256, controller)
        .map_err(|error| reject(error.to_string()))?;
    let external_inputs = validate_wiring(controller, plant, wiring)?;
    if horizon > MAX_COMPOSITION_HORIZON
        || initial_controller_state >= controller.state_count
        || initial_plant_state >= plant.state_count()
        || bad_plant_output >= plant.outputs.len()
        || controller
            .state_count
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
            let cell_index = cell_for_pattern(controller, sensors)?;
            let row = controller.cells[cell_index]
                .rows
                .get(state.controller)
                .ok_or_else(|| reject("controller transducer state row is missing"))?;
            let action = usize::try_from(row.outcome.outputs)
                .map_err(|_| reject("controller action pattern exceeds usize"))?;
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
                        controller: row.outcome.target,
                        plant: next_plant,
                    };
                    next.entry(target).or_insert(Some(Predecessor {
                        previous: state,
                        sensor_pattern: sensors,
                        action_pattern: action,
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
        assert_eq!(result.trace[0].plant_input, 2);
        assert!(!result.trace[0].bad);
        assert!(result.trace[1].bad);
    }
}
