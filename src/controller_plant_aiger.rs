//! Bounded-equivalent AIGER export for sampled controller and plant composition.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::aiger_obligation::AigerTransition;
use crate::controller_plant::{ControllerPlantWiring, compose_controller_plant_direct};

pub const CONTROLLER_PLANT_AIGER_EXPORT_VERSION: u32 = 1;
pub const MAX_EXPORT_HORIZON: usize = 62;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantAigerExport {
    pub version: u32,
    pub horizon: usize,
    pub bad_plant_output: usize,
    pub external_plant_inputs: Vec<usize>,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantAigerMultiExport {
    pub version: u32,
    pub horizon: usize,
    pub bad_plant_outputs: Vec<usize>,
    pub external_plant_inputs: Vec<usize>,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantAigerExportError(pub String);

impl fmt::Display for ControllerPlantAigerExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ControllerPlantAigerExportError {}

fn reject(message: impl Into<String>) -> ControllerPlantAigerExportError {
    ControllerPlantAigerExportError(message.into())
}

#[derive(Default)]
struct Builder {
    inputs: Vec<usize>,
    latches: Vec<(usize, usize)>,
    ands: Vec<(usize, usize, usize)>,
    next_variable: usize,
}

impl Builder {
    fn variable(&mut self) -> usize {
        self.next_variable += 1;
        self.next_variable * 2
    }

    fn input(&mut self) -> usize {
        let literal = self.variable();
        self.inputs.push(literal);
        literal
    }

    fn latch(&mut self) -> usize {
        let literal = self.variable();
        self.latches.push((literal, 0));
        literal
    }

    fn and(&mut self, left: usize, right: usize) -> usize {
        if left == 0 || right == 0 {
            return 0;
        }
        if left == 1 {
            return right;
        }
        if right == 1 || left == right {
            return left;
        }
        if left == (right ^ 1) {
            return 0;
        }
        let output = self.variable();
        self.ands.push((output, left, right));
        output
    }

    fn or(&mut self, left: usize, right: usize) -> usize {
        self.and(left ^ 1, right ^ 1) ^ 1
    }

    fn xor(&mut self, left: usize, right: usize) -> usize {
        let one = self.and(left, right ^ 1);
        let two = self.and(left ^ 1, right);
        self.or(one, two)
    }

    fn mux(&mut self, select: usize, when_true: usize, when_false: usize) -> usize {
        let yes = self.and(select, when_true);
        let no = self.and(select ^ 1, when_false);
        self.or(yes, no)
    }

    fn conjunction(&mut self, literals: impl IntoIterator<Item = usize>) -> usize {
        literals
            .into_iter()
            .fold(1, |left, right| self.and(left, right))
    }

    fn set_latch_next(
        &mut self,
        current: usize,
        next: usize,
    ) -> Result<(), ControllerPlantAigerExportError> {
        let entry = self
            .latches
            .iter_mut()
            .find(|entry| entry.0 == current)
            .ok_or_else(|| reject("export latch is missing"))?;
        entry.1 = next;
        Ok(())
    }

    fn finish(self, output: usize) -> Vec<u8> {
        let mut text = format!(
            "aag {} {} {} 1 {}\n",
            self.next_variable,
            self.inputs.len(),
            self.latches.len(),
            self.ands.len()
        );
        for literal in self.inputs {
            text.push_str(&format!("{literal}\n"));
        }
        for (current, next) in self.latches {
            text.push_str(&format!("{current} {next} 0\n"));
        }
        text.push_str(&format!("{output}\n"));
        for (output, left, right) in self.ands {
            text.push_str(&format!("{output} {left} {right}\n"));
        }
        text.into_bytes()
    }

    fn finish_bads(self, bads: &[usize]) -> Vec<u8> {
        let mut text = format!(
            "aag {} {} {} 0 {} {} 0 0 0\n",
            self.next_variable,
            self.inputs.len(),
            self.latches.len(),
            self.ands.len(),
            bads.len(),
        );
        for literal in self.inputs {
            text.push_str(&format!("{literal}\n"));
        }
        for (current, next) in self.latches {
            text.push_str(&format!("{current} {next} 0\n"));
        }
        for bad in bads {
            text.push_str(&format!("{bad}\n"));
        }
        for (output, left, right) in self.ands {
            text.push_str(&format!("{output} {left} {right}\n"));
        }
        text.into_bytes()
    }
}

fn mapped(
    literal: usize,
    map: &BTreeMap<usize, usize>,
) -> Result<usize, ControllerPlantAigerExportError> {
    if literal < 2 {
        return Ok(literal);
    }
    map.get(&(literal / 2))
        .copied()
        .map(|base| base ^ (literal & 1))
        .ok_or_else(|| reject("export references an unmapped AIGER literal"))
}

fn append_gates(
    builder: &mut Builder,
    model: &AigerTransition,
    map: &mut BTreeMap<usize, usize>,
) -> Result<(), ControllerPlantAigerExportError> {
    for gate in &model.ands {
        let left = mapped(gate.left, map)?;
        let right = mapped(gate.right, map)?;
        let output = builder.and(left, right);
        map.insert(gate.output / 2, output);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_bounded_controller_plant_aag(
    controller: &AigerTransition,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_outputs: &[usize],
    horizon: usize,
) -> Result<(Vec<usize>, Builder, Vec<usize>), ControllerPlantAigerExportError> {
    controller
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    plant
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    if initial_controller_state != 0 || initial_plant_state != 0 {
        return Err(reject("AIGER export v1 requires all-zero initial states"));
    }
    if horizon > MAX_EXPORT_HORIZON
        || bad_plant_outputs.is_empty()
        || bad_plant_outputs.len() > 64
        || bad_plant_outputs
            .iter()
            .any(|&output| output >= plant.outputs.len())
        || bad_plant_outputs
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != bad_plant_outputs.len()
    {
        return Err(reject("AIGER export query is outside limits"));
    }
    for &bad_plant_output in bad_plant_outputs {
        compose_controller_plant_direct(
            controller,
            plant,
            wiring,
            initial_controller_state,
            initial_plant_state,
            bad_plant_output,
            horizon,
        )
        .map_err(|error| reject(error.to_string()))?;
    }

    let action_inputs = wiring
        .plant_action_inputs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let external_plant_inputs = (0..plant.inputs.len())
        .filter(|index| !action_inputs.contains(index))
        .collect::<Vec<_>>();
    let mut builder = Builder::default();
    let external_literals = external_plant_inputs
        .iter()
        .map(|_| builder.input())
        .collect::<Vec<_>>();
    let controller_latches = controller
        .latches
        .iter()
        .map(|_| builder.latch())
        .collect::<Vec<_>>();
    let plant_latches = plant
        .latches
        .iter()
        .map(|_| builder.latch())
        .collect::<Vec<_>>();
    let counter = (0..6).map(|_| builder.latch()).collect::<Vec<_>>();

    let mut sensor_map = BTreeMap::new();
    for (&declared, &literal) in plant.inputs.iter().zip(std::iter::repeat(&0)) {
        sensor_map.insert(declared / 2, literal);
    }
    for (latch, &literal) in plant.latches.iter().zip(&plant_latches) {
        sensor_map.insert(latch.current / 2, literal);
    }
    append_gates(&mut builder, plant, &mut sensor_map)?;
    let sensors = wiring
        .plant_sensor_outputs
        .iter()
        .map(|&index| mapped(plant.outputs[index], &sensor_map))
        .collect::<Result<Vec<_>, _>>()?;

    let mut controller_map = BTreeMap::new();
    for (&index, &literal) in wiring.controller_sensor_inputs.iter().zip(&sensors) {
        controller_map.insert(controller.inputs[index] / 2, literal);
    }
    for &declared in &controller.inputs {
        controller_map.entry(declared / 2).or_insert(0);
    }
    for (latch, &literal) in controller.latches.iter().zip(&controller_latches) {
        controller_map.insert(latch.current / 2, literal);
    }
    append_gates(&mut builder, controller, &mut controller_map)?;
    let actions = wiring
        .controller_action_outputs
        .iter()
        .map(|&index| mapped(controller.outputs[index], &controller_map))
        .collect::<Result<Vec<_>, _>>()?;

    let mut plant_map = BTreeMap::new();
    for (external_index, &literal) in external_plant_inputs.iter().zip(&external_literals) {
        plant_map.insert(plant.inputs[*external_index] / 2, literal);
    }
    for (&input_index, &literal) in wiring.plant_action_inputs.iter().zip(&actions) {
        plant_map.insert(plant.inputs[input_index] / 2, literal);
    }
    for (latch, &literal) in plant.latches.iter().zip(&plant_latches) {
        plant_map.insert(latch.current / 2, literal);
    }
    append_gates(&mut builder, plant, &mut plant_map)?;

    let completed_value = horizon + 1;
    let equality_bits = counter
        .iter()
        .enumerate()
        .map(|(bit, &literal)| {
            if completed_value >> bit & 1 == 1 {
                literal
            } else {
                literal ^ 1
            }
        })
        .collect::<Vec<_>>();
    let done = builder.conjunction(equality_bits);

    for ((latch, &current), source) in controller
        .latches
        .iter()
        .zip(&controller_latches)
        .zip(std::iter::repeat(&controller_map))
    {
        let next = mapped(latch.next, source)?;
        let frozen = builder.mux(done, current, next);
        builder.set_latch_next(current, frozen)?;
    }
    for (latch, &current) in plant.latches.iter().zip(&plant_latches) {
        let next = mapped(latch.next, &plant_map)?;
        let frozen = builder.mux(done, current, next);
        builder.set_latch_next(current, frozen)?;
    }
    let mut carry = 1;
    for &current in &counter {
        let incremented = builder.xor(current, carry);
        carry = builder.and(current, carry);
        let next = builder.mux(done, current, incremented);
        builder.set_latch_next(current, next)?;
    }
    let mut bads = Vec::with_capacity(bad_plant_outputs.len());
    for &bad_plant_output in bad_plant_outputs {
        let source_bad = mapped(plant.outputs[bad_plant_output], &plant_map)?;
        bads.push(builder.and(source_bad, done ^ 1));
    }

    Ok((external_plant_inputs, builder, bads))
}

/// Exports one sampled controller/plant property as an unbounded AIGER safety model.
///
/// Frames `0..=horizon` retain the original bad predicate. The next state after
/// the final checked frame is an absorbing completed state whose bad output is
/// false, making unbounded safety equivalent to the bounded source query.
#[allow(clippy::too_many_arguments)]
pub fn export_bounded_controller_plant_aag(
    controller: &AigerTransition,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_output: usize,
    horizon: usize,
) -> Result<ControllerPlantAigerExport, ControllerPlantAigerExportError> {
    let (external_plant_inputs, builder, bads) = build_bounded_controller_plant_aag(
        controller,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        &[bad_plant_output],
        horizon,
    )?;

    Ok(ControllerPlantAigerExport {
        version: CONTROLLER_PLANT_AIGER_EXPORT_VERSION,
        horizon,
        bad_plant_output,
        external_plant_inputs,
        bytes: builder.finish(bads[0]),
    })
}

/// Exports several sampled properties in one AIGER 1.9 model with explicit bad
/// sections. Transition logic and the checked horizon are shared exactly.
#[allow(clippy::too_many_arguments)]
pub fn export_bounded_controller_plant_multi_aag(
    controller: &AigerTransition,
    plant: &AigerTransition,
    wiring: &ControllerPlantWiring,
    initial_controller_state: usize,
    initial_plant_state: usize,
    bad_plant_outputs: &[usize],
    horizon: usize,
) -> Result<ControllerPlantAigerMultiExport, ControllerPlantAigerExportError> {
    let (external_plant_inputs, builder, bads) = build_bounded_controller_plant_aag(
        controller,
        plant,
        wiring,
        initial_controller_state,
        initial_plant_state,
        bad_plant_outputs,
        horizon,
    )?;
    Ok(ControllerPlantAigerMultiExport {
        version: CONTROLLER_PLANT_AIGER_EXPORT_VERSION,
        horizon,
        bad_plant_outputs: bad_plant_outputs.to_vec(),
        external_plant_inputs,
        bytes: builder.finish_bads(&bads),
    })
}
