//! Exact source-separated controller and plant verification certificates.

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind, WordValues};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

pub const COMPONENT_CONTRACT_VERSION: u32 = 1;
pub const COMPONENT_CERTIFICATE_VERSION: u32 = 1;
pub const CONTROLLER_OBLIGATION_VERSION: u32 = 1;
pub const REUSABLE_COMPONENT_BATCH_VERSION: u32 = 1;
pub const COMPONENT_BATCH_PORTFOLIO_VERSION: u32 = 1;
pub const MAX_COMPONENT_CONTRACT_BYTES: usize = 4096;
pub const MAX_CONTROLLER_OBLIGATION_BYTES: usize = 2048;
pub const MAX_COMPONENT_PHASE_HORIZON: u32 = 1_000_000_000;
pub const MAX_COMPONENT_SEARCH_HORIZON: u32 = 256;
pub const MAX_COMPONENT_STATES_PER_LAYER: usize = 65_536;
pub const MAX_COMPONENT_TOTAL_STATES: usize = 262_144;
pub const MAX_COMPONENT_NODE_STEPS: u64 = 30_000_000;
pub const MAX_COMPONENT_CERTIFICATE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_COMPONENT_BATCH_MEMBERS: usize = 64;
pub const MAX_REUSABLE_COMPONENT_BATCH_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_COMPONENT_BATCH_PORTFOLIO_BYTES: usize = 65 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentContract {
    pub controller_reset_input: NodeId,
    pub controller_velocity_input: NodeId,
    pub controller_braking_state: NodeId,
    pub controller_brake_output: NodeId,
    pub plant_reset_input: NodeId,
    pub plant_brake_input: NodeId,
    pub plant_velocity_state: NodeId,
    pub plant_position_state: NodeId,
    pub plant_bad_property: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerObligation {
    pub controller_sha256: String,
    pub reset_input: NodeId,
    pub velocity_input: NodeId,
    pub braking_state: NodeId,
    pub brake_output: NodeId,
    pub velocity_width: u32,
    pub brake_velocity: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentResult {
    Safe,
    Unsafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentBackend {
    PhaseContract,
    ComposedSearch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentSelectionReason {
    ExactPhaseContractSafe,
    SpecialisedInapplicableOrIntersecting,
}

impl ComponentSelectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactPhaseContractSafe => "exact-phase-contract-safe",
            Self::SpecialisedInapplicableOrIntersecting => {
                "specialised-inapplicable-or-intersecting"
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ComponentState {
    pub controller: Vec<(NodeId, u64)>,
    pub plant: Vec<(NodeId, u64)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentPhaseCertificate {
    pub controller_sha256: String,
    pub plant_sha256: String,
    pub contract_sha256: String,
    pub query_horizon: u32,
    pub width: u32,
    pub acceleration: u64,
    pub brake_velocity: u64,
    pub deceleration: u64,
    pub position_threshold: u64,
    pub switch_frame: u64,
    pub stop_frame: u64,
    pub max_velocity: u64,
    pub max_position: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentSearchCertificate {
    pub controller_sha256: String,
    pub plant_sha256: String,
    pub contract_sha256: String,
    pub query_horizon: u32,
    pub result: ComponentResult,
    pub bad_frame: Option<u32>,
    pub witness_resets: Vec<bool>,
    pub layers: Vec<Vec<ComponentState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentCertificate {
    Phase(ComponentPhaseCertificate),
    Search(ComponentSearchCertificate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentProduction {
    pub certificate: ComponentCertificate,
    pub selection_reason: ComponentSelectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentSummary {
    pub backend: ComponentBackend,
    pub result: ComponentResult,
    pub query_horizon: u32,
    pub bad_frame: Option<u32>,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Copy)]
pub struct ComponentBatchInput<'a> {
    pub plant_source: &'a [u8],
    pub contract_source: &'a [u8],
    pub horizon: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NaiveComponentBatchCertificate {
    pub controller_sha256: String,
    pub members: Vec<ComponentCertificate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReusedPhaseMember {
    pub plant_sha256: String,
    pub contract_sha256: String,
    pub query_horizon: u32,
    pub acceleration: u64,
    pub deceleration: u64,
    pub position_threshold: u64,
    pub switch_frame: u64,
    pub stop_frame: u64,
    pub max_velocity: u64,
    pub max_position: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReusableBatchMember {
    ReusedPhase(ReusedPhaseMember),
    ExactFallback(ComponentCertificate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReusableComponentBatchCertificate {
    pub controller_obligation: ControllerObligation,
    pub members: Vec<ReusableBatchMember>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentBatchSelectionReason {
    FullyAdmittedReuse,
    SingletonOrExactFallback,
}

impl ComponentBatchSelectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FullyAdmittedReuse => "fully-admitted-reuse",
            Self::SingletonOrExactFallback => "singleton-or-exact-fallback",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentBatchPortfolioCertificate {
    Reusable(ReusableComponentBatchCertificate),
    Ordinary(NaiveComponentBatchCertificate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentBatchPortfolioProduction {
    pub certificate: ComponentBatchPortfolioCertificate,
    pub selection_reason: ComponentBatchSelectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentBatchSummary {
    pub members: Vec<ComponentSummary>,
    pub safe: usize,
    pub unsafe_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentError(pub String);

impl fmt::Display for ComponentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ComponentError {}

#[derive(Clone, Copy, Debug)]
struct PhaseShape {
    width: u32,
    acceleration: u64,
    brake_velocity: u64,
    deceleration: u64,
    position_threshold: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Endpoint {
    velocity: u64,
    position: u64,
    max_velocity: u64,
    switch_frame: u64,
    stop_frame: u64,
}

struct Composition<'a> {
    controller: Arc<Btor2Model>,
    plant: Btor2Model,
    contract: &'a ComponentContract,
}

fn reject(message: impl Into<String>) -> ComponentError {
    ComponentError(message.into())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn canonical_text<'a>(
    bytes: &'a [u8],
    label: &str,
    limit: usize,
) -> Result<&'a str, ComponentError> {
    if bytes.len() > limit {
        return Err(reject(format!("{label} exceeds byte limit")));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject(format!("{label} is not UTF-8")))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(format!(
            "{label} must be canonical LF text without NUL"
        )));
    }
    Ok(text)
}

fn parse_number<T: std::str::FromStr + fmt::Display>(
    value: &str,
    label: &str,
) -> Result<T, ComponentError> {
    let parsed = value
        .parse::<T>()
        .map_err(|_| reject(format!("invalid {label}")))?;
    if parsed.to_string() != value {
        return Err(reject(format!("noncanonical {label}")));
    }
    Ok(parsed)
}

pub fn parse_contract(bytes: &[u8]) -> Result<ComponentContract, ComponentError> {
    let text = canonical_text(bytes, "component contract", MAX_COMPONENT_CONTRACT_BYTES)?;
    let mut lines = text.lines();
    fn take<'a>(lines: &mut std::str::Lines<'a>, key: &str) -> Result<&'a str, ComponentError> {
        lines
            .next()
            .and_then(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    let version: u32 = parse_number(
        take(&mut lines, "component_contract_version")?,
        "contract version",
    )?;
    if version != COMPONENT_CONTRACT_VERSION {
        return Err(reject("unsupported component contract version"));
    }
    let contract = ComponentContract {
        controller_reset_input: parse_number(
            take(&mut lines, "controller_reset_input")?,
            "controller reset input",
        )?,
        controller_velocity_input: parse_number(
            take(&mut lines, "controller_velocity_input")?,
            "controller velocity input",
        )?,
        controller_braking_state: parse_number(
            take(&mut lines, "controller_braking_state")?,
            "controller braking state",
        )?,
        controller_brake_output: parse_number(
            take(&mut lines, "controller_brake_output")?,
            "controller brake output",
        )?,
        plant_reset_input: parse_number(
            take(&mut lines, "plant_reset_input")?,
            "plant reset input",
        )?,
        plant_brake_input: parse_number(
            take(&mut lines, "plant_brake_input")?,
            "plant brake input",
        )?,
        plant_velocity_state: parse_number(
            take(&mut lines, "plant_velocity_state")?,
            "plant velocity state",
        )?,
        plant_position_state: parse_number(
            take(&mut lines, "plant_position_state")?,
            "plant position state",
        )?,
        plant_bad_property: parse_number(
            take(&mut lines, "plant_bad_property")?,
            "plant bad property",
        )?,
    };
    if lines.next().is_some() {
        return Err(reject("component contract has trailing fields"));
    }
    Ok(contract)
}

fn node_width(model: &Btor2Model, id: NodeId) -> Option<u32> {
    model.nodes().get(&id).map(|node| node.width)
}

fn depends_on_input(model: &Btor2Model, id: NodeId, memo: &mut BTreeMap<NodeId, bool>) -> bool {
    if let Some(value) = memo.get(&id) {
        return *value;
    }
    let value = match model.nodes()[&id].kind {
        NodeKind::Input => true,
        NodeKind::State | NodeKind::Constant(_) => false,
        NodeKind::Unary(_, operand)
        | NodeKind::Slice { value: operand, .. }
        | NodeKind::Uext { value: operand, .. } => depends_on_input(model, operand, memo),
        NodeKind::Binary(_, left, right) => {
            depends_on_input(model, left, memo) || depends_on_input(model, right, memo)
        }
        NodeKind::Ite(condition, then_value, else_value) => {
            depends_on_input(model, condition, memo)
                || depends_on_input(model, then_value, memo)
                || depends_on_input(model, else_value, memo)
        }
    };
    memo.insert(id, value);
    value
}

fn build_composition<'a>(
    controller_source: &[u8],
    plant_source: &[u8],
    contract: &'a ComponentContract,
) -> Result<Composition<'a>, ComponentError> {
    let controller =
        Arc::new(btor2::parse_bytes(controller_source).map_err(|error| reject(error.to_string()))?);
    build_composition_with_controller(&controller, plant_source, contract)
}

fn build_composition_with_controller<'a>(
    controller: &Arc<Btor2Model>,
    plant_source: &[u8],
    contract: &'a ComponentContract,
) -> Result<Composition<'a>, ComponentError> {
    let controller = Arc::clone(controller);
    let plant = btor2::parse_bytes(plant_source).map_err(|error| reject(error.to_string()))?;
    if controller.inputs()
        != [
            contract.controller_reset_input,
            contract.controller_velocity_input,
        ]
        || controller.states() != [contract.controller_braking_state]
        || plant.inputs() != [contract.plant_reset_input, contract.plant_brake_input]
        || plant.states() != [contract.plant_velocity_state, contract.plant_position_state]
        || !controller.constraints().is_empty()
        || !plant.constraints().is_empty()
    {
        return Err(reject(
            "component state or input vectors do not match contract",
        ));
    }
    let velocity_width = node_width(&plant, contract.plant_velocity_state)
        .ok_or_else(|| reject("plant velocity state is absent"))?;
    if node_width(&controller, contract.controller_reset_input) != Some(1)
        || node_width(&controller, contract.controller_velocity_input) != Some(velocity_width)
        || node_width(&controller, contract.controller_braking_state) != Some(1)
        || node_width(&controller, contract.controller_brake_output) != Some(1)
        || node_width(&plant, contract.plant_reset_input) != Some(1)
        || node_width(&plant, contract.plant_brake_input) != Some(1)
        || node_width(&plant, contract.plant_position_state) != Some(velocity_width)
    {
        return Err(reject("component wire widths do not match contract"));
    }
    let bad_expression = plant
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == contract.plant_bad_property).then_some(*expression))
        .ok_or_else(|| reject("plant bad property is absent"))?;
    if depends_on_input(&plant, bad_expression, &mut BTreeMap::new()) {
        return Err(reject(
            "component search requires a state-only plant property",
        ));
    }
    Ok(Composition {
        controller,
        plant,
        contract,
    })
}

fn values_from_pairs(pairs: &[(NodeId, u64)]) -> WordValues {
    pairs.iter().copied().collect()
}

fn validate_state(
    composition: &Composition<'_>,
    state: &ComponentState,
) -> Result<(), ComponentError> {
    fn validate_side(
        model: &Btor2Model,
        expected: &[NodeId],
        actual: &[(NodeId, u64)],
        label: &str,
    ) -> Result<(), ComponentError> {
        if actual.len() != expected.len()
            || actual
                .iter()
                .zip(expected)
                .any(|((actual, _), expected)| actual != expected)
        {
            return Err(reject(format!(
                "component {label} state vector is not canonical"
            )));
        }
        for (id, value) in actual {
            let width = node_width(model, *id)
                .ok_or_else(|| reject(format!("component {label} state is absent")))?;
            if width < 64 && *value >= (1u64 << width) {
                return Err(reject(format!(
                    "component {label} state value exceeds its width"
                )));
            }
        }
        Ok(())
    }

    validate_side(
        &composition.controller,
        composition.controller.states(),
        &state.controller,
        "controller",
    )?;
    validate_side(
        &composition.plant,
        composition.plant.states(),
        &state.plant,
        "plant",
    )
}

fn state_key(controller: &WordValues, plant: &WordValues) -> ComponentState {
    ComponentState {
        controller: controller.iter().map(|(id, value)| (*id, *value)).collect(),
        plant: plant.iter().map(|(id, value)| (*id, *value)).collect(),
    }
}

fn initial_state(composition: &Composition<'_>) -> Result<ComponentState, ComponentError> {
    let controller = composition
        .controller
        .initial_state()
        .map_err(|error| reject(error.to_string()))?;
    let plant = composition
        .plant
        .initial_state()
        .map_err(|error| reject(error.to_string()))?;
    Ok(state_key(&controller, &plant))
}

fn controller_inputs(
    composition: &Composition<'_>,
    state: &ComponentState,
    reset: bool,
) -> Result<WordValues, ComponentError> {
    let velocity = state
        .plant
        .iter()
        .find_map(|(id, value)| {
            (*id == composition.contract.plant_velocity_state).then_some(*value)
        })
        .ok_or_else(|| reject("component plant velocity state is absent"))?;
    Ok(WordValues::from([
        (
            composition.contract.controller_reset_input,
            u64::from(reset),
        ),
        (composition.contract.controller_velocity_input, velocity),
    ]))
}

fn brake_output(
    composition: &Composition<'_>,
    state: &ComponentState,
    reset: bool,
) -> Result<u64, ComponentError> {
    composition
        .controller
        .evaluate(
            composition.contract.controller_brake_output,
            &values_from_pairs(&state.controller),
            &controller_inputs(composition, state, reset)?,
        )
        .map_err(|error| reject(error.to_string()))
}

fn plant_inputs(
    composition: &Composition<'_>,
    state: &ComponentState,
    reset: bool,
) -> Result<WordValues, ComponentError> {
    Ok(WordValues::from([
        (composition.contract.plant_reset_input, u64::from(reset)),
        (
            composition.contract.plant_brake_input,
            brake_output(composition, state, reset)?,
        ),
    ]))
}

fn step(
    composition: &Composition<'_>,
    state: &ComponentState,
    reset: bool,
) -> Result<ComponentState, ComponentError> {
    validate_state(composition, state)?;
    let controller = composition
        .controller
        .step(
            &values_from_pairs(&state.controller),
            &controller_inputs(composition, state, reset)?,
        )
        .map_err(|error| reject(error.to_string()))?;
    let plant = composition
        .plant
        .step(
            &values_from_pairs(&state.plant),
            &plant_inputs(composition, state, reset)?,
        )
        .map_err(|error| reject(error.to_string()))?;
    Ok(state_key(&controller, &plant))
}

fn bad_active(
    composition: &Composition<'_>,
    state: &ComponentState,
) -> Result<bool, ComponentError> {
    validate_state(composition, state)?;
    Ok(composition
        .plant
        .active_bad(
            &values_from_pairs(&state.plant),
            &plant_inputs(composition, state, false)?,
        )
        .map_err(|error| reject(error.to_string()))?
        .contains(&composition.contract.plant_bad_property))
}

#[derive(Default)]
struct SearchBudget {
    states: usize,
    node_steps: u64,
}

impl SearchBudget {
    fn add_layer(
        &mut self,
        composition: &Composition<'_>,
        states: usize,
    ) -> Result<(), ComponentError> {
        if states > MAX_COMPONENT_STATES_PER_LAYER {
            return Err(reject("component layer exceeds state limit"));
        }
        self.states = self
            .states
            .checked_add(states)
            .filter(|value| *value <= MAX_COMPONENT_TOTAL_STATES)
            .ok_or_else(|| reject("component search exceeds total state limit"))?;
        let nodes = composition
            .controller
            .nodes()
            .len()
            .checked_add(composition.plant.nodes().len())
            .ok_or_else(|| reject("component node count overflowed"))? as u64;
        self.node_steps = self
            .node_steps
            .checked_add(
                (states as u64)
                    .checked_mul(2)
                    .and_then(|value| value.checked_mul(nodes))
                    .ok_or_else(|| reject("component work estimate overflowed"))?,
            )
            .filter(|value| *value <= MAX_COMPONENT_NODE_STEPS)
            .ok_or_else(|| reject("component search exceeds node-step limit"))?;
        Ok(())
    }
}

fn produce_search(
    controller_source: &[u8],
    plant_source: &[u8],
    contract_source: &[u8],
    contract: &ComponentContract,
    horizon: u32,
) -> Result<ComponentSearchCertificate, ComponentError> {
    if horizon > MAX_COMPONENT_SEARCH_HORIZON {
        return Err(reject("component search horizon exceeds limit"));
    }
    let composition = build_composition(controller_source, plant_source, contract)?;
    let initial = initial_state(&composition)?;
    if bad_active(&composition, &initial)? {
        return Ok(ComponentSearchCertificate {
            controller_sha256: digest(controller_source),
            plant_sha256: digest(plant_source),
            contract_sha256: digest(contract_source),
            query_horizon: horizon,
            result: ComponentResult::Unsafe,
            bad_frame: Some(0),
            witness_resets: Vec::new(),
            layers: Vec::new(),
        });
    }
    let mut layers = vec![vec![initial]];
    let mut predecessors = Vec::<BTreeMap<ComponentState, (ComponentState, bool)>>::new();
    let mut budget = SearchBudget::default();
    budget.add_layer(&composition, 1)?;
    for frame in 0..horizon {
        let mut next = BTreeSet::new();
        let mut prior = BTreeMap::new();
        for state in &layers[frame as usize] {
            for reset in [false, true] {
                let successor = step(&composition, state, reset)?;
                if next.insert(successor.clone()) {
                    prior.insert(successor, (state.clone(), reset));
                }
            }
        }
        budget.add_layer(&composition, next.len())?;
        let layer: Vec<_> = next.into_iter().collect();
        let mut bad_state = None;
        for state in &layer {
            if bad_active(&composition, state)? {
                bad_state = Some(state);
                break;
            }
        }
        if let Some(bad_state) = bad_state {
            let mut resets = Vec::with_capacity(frame as usize + 1);
            let mut cursor = bad_state.clone();
            for current in (0..=frame as usize).rev() {
                let map = if current == frame as usize {
                    &prior
                } else {
                    &predecessors[current]
                };
                let (parent, reset) = map
                    .get(&cursor)
                    .cloned()
                    .ok_or_else(|| reject("component predecessor chain is absent"))?;
                resets.push(reset);
                cursor = parent;
            }
            resets.reverse();
            return Ok(ComponentSearchCertificate {
                controller_sha256: digest(controller_source),
                plant_sha256: digest(plant_source),
                contract_sha256: digest(contract_source),
                query_horizon: horizon,
                result: ComponentResult::Unsafe,
                bad_frame: Some(frame + 1),
                witness_resets: resets,
                layers: Vec::new(),
            });
        }
        predecessors.push(prior);
        layers.push(layer);
    }
    Ok(ComponentSearchCertificate {
        controller_sha256: digest(controller_source),
        plant_sha256: digest(plant_source),
        contract_sha256: digest(contract_source),
        query_horizon: horizon,
        result: ComponentResult::Safe,
        bad_frame: None,
        witness_resets: Vec::new(),
        layers,
    })
}

fn verify_search(
    controller_source: &[u8],
    plant_source: &[u8],
    contract_source: &[u8],
    contract: &ComponentContract,
    certificate: &ComponentSearchCertificate,
) -> Result<ComponentSummary, ComponentError> {
    if certificate.controller_sha256 != digest(controller_source)
        || certificate.plant_sha256 != digest(plant_source)
        || certificate.contract_sha256 != digest(contract_source)
        || certificate.query_horizon > MAX_COMPONENT_SEARCH_HORIZON
    {
        return Err(reject(
            "component search source binding or horizon is invalid",
        ));
    }
    let composition = build_composition(controller_source, plant_source, contract)?;
    verify_search_composition(&composition, certificate)
}

fn verify_search_composition(
    composition: &Composition<'_>,
    certificate: &ComponentSearchCertificate,
) -> Result<ComponentSummary, ComponentError> {
    let initial = initial_state(composition)?;
    match certificate.result {
        ComponentResult::Unsafe => {
            let frame = certificate
                .bad_frame
                .ok_or_else(|| reject("unsafe component certificate lacks bad frame"))?;
            if !certificate.layers.is_empty()
                || certificate.witness_resets.len() != frame as usize
                || frame > certificate.query_horizon
            {
                return Err(reject("unsafe component witness shape is invalid"));
            }
            let mut state = initial;
            if frame == 0 && !bad_active(composition, &state)? {
                return Err(reject("component initial state is not bad"));
            }
            for reset in &certificate.witness_resets {
                state = step(composition, &state, *reset)?;
            }
            if !bad_active(composition, &state)? {
                return Err(reject("component witness does not reach bad property"));
            }
            Ok(ComponentSummary {
                backend: ComponentBackend::ComposedSearch,
                result: ComponentResult::Unsafe,
                query_horizon: certificate.query_horizon,
                bad_frame: Some(frame),
                logical_reachable_states: u64::from(frame) + 1,
            })
        }
        ComponentResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_resets.is_empty()
                || certificate.layers.len() != certificate.query_horizon as usize + 1
                || certificate.layers.first() != Some(&vec![initial])
            {
                return Err(reject("safe component layer shape is invalid"));
            }
            let mut budget = SearchBudget::default();
            let mut total = 0u64;
            for (frame, layer) in certificate.layers.iter().enumerate() {
                if layer.is_empty() || !layer.windows(2).all(|pair| pair[0] < pair[1]) {
                    return Err(reject("safe component layer is not canonical or safe"));
                }
                for state in layer {
                    if bad_active(composition, state)? {
                        return Err(reject("safe component layer contains a bad state"));
                    }
                }
                budget.add_layer(composition, layer.len())?;
                total = total
                    .checked_add(layer.len() as u64)
                    .ok_or_else(|| reject("component logical state count overflowed"))?;
                if frame + 1 < certificate.layers.len() {
                    let mut expected = BTreeSet::new();
                    for state in layer {
                        for reset in [false, true] {
                            expected.insert(step(composition, state, reset)?);
                        }
                    }
                    if expected.into_iter().collect::<Vec<_>>() != certificate.layers[frame + 1] {
                        return Err(reject("component successor completeness check failed"));
                    }
                }
            }
            Ok(ComponentSummary {
                backend: ComponentBackend::ComposedSearch,
                result: ComponentResult::Safe,
                query_horizon: certificate.query_horizon,
                bad_frame: None,
                logical_reachable_states: total,
            })
        }
    }
}

fn constant(model: &Btor2Model, id: NodeId) -> Option<u64> {
    match model.nodes().get(&id)?.kind {
        NodeKind::Constant(value) => Some(value),
        _ => None,
    }
}

fn zero(model: &Btor2Model, id: NodeId) -> bool {
    constant(model, id) == Some(0)
}

fn reset_advance(model: &Btor2Model, state: NodeId, reset: NodeId) -> Option<NodeId> {
    match model.nodes().get(&model.next_value(state)?)?.kind {
        NodeKind::Ite(condition, reset_value, advance)
            if condition == reset && zero(model, reset_value) =>
        {
            Some(advance)
        }
        _ => None,
    }
}

fn controller_obligation_shape(
    controller: &Btor2Model,
    contract: &ComponentContract,
) -> Result<(u32, u64), ComponentError> {
    if controller.inputs()
        != [
            contract.controller_reset_input,
            contract.controller_velocity_input,
        ]
        || controller.states() != [contract.controller_braking_state]
        || !controller.constraints().is_empty()
        || controller
            .bad_properties()
            .iter()
            .any(|(_, expression, _)| !zero(controller, *expression))
    {
        return Err(reject(
            "controller vectors or properties are outside obligation language",
        ));
    }
    let velocity_width = node_width(controller, contract.controller_velocity_input)
        .ok_or_else(|| reject("controller velocity input is absent"))?;
    if velocity_width == 0
        || velocity_width > 64
        || node_width(controller, contract.controller_reset_input) != Some(1)
        || node_width(controller, contract.controller_braking_state) != Some(1)
        || node_width(controller, contract.controller_brake_output) != Some(1)
    {
        return Err(reject("controller obligation widths are invalid"));
    }
    let initialiser = controller
        .initialiser(contract.controller_braking_state)
        .ok_or_else(|| reject("controller braking initialiser is absent"))?;
    if !zero(controller, initialiser) {
        return Err(reject(
            "controller braking state does not initialise to zero",
        ));
    }
    let advance = reset_advance(
        controller,
        contract.controller_braking_state,
        contract.controller_reset_input,
    )
    .ok_or_else(|| reject("controller reset transition is outside obligation language"))?;
    if advance != contract.controller_brake_output {
        return Err(reject("controller output is not its latched next control"));
    }
    let guard = match controller
        .nodes()
        .get(&contract.controller_brake_output)
        .map(|node| &node.kind)
    {
        Some(NodeKind::Binary(BinaryOp::Or, left, right))
            if *left == contract.controller_braking_state =>
        {
            *right
        }
        Some(NodeKind::Binary(BinaryOp::Or, left, right))
            if *right == contract.controller_braking_state =>
        {
            *left
        }
        _ => {
            return Err(reject(
                "controller brake output is outside obligation language",
            ));
        }
    };
    let brake_velocity = match controller.nodes().get(&guard).map(|node| &node.kind) {
        Some(NodeKind::Binary(BinaryOp::Ugte, velocity, threshold))
            if *velocity == contract.controller_velocity_input =>
        {
            constant(controller, *threshold)
                .filter(|value| *value != 0)
                .ok_or_else(|| reject("controller threshold is not a nonzero literal"))?
        }
        _ => return Err(reject("controller guard is outside obligation language")),
    };
    Ok((velocity_width, brake_velocity))
}

pub fn produce_controller_obligation(
    controller_source: &[u8],
    contract_source: &[u8],
) -> Result<ControllerObligation, ComponentError> {
    let contract = parse_contract(contract_source)?;
    let controller =
        Arc::new(btor2::parse_bytes(controller_source).map_err(|error| reject(error.to_string()))?);
    let (velocity_width, brake_velocity) = controller_obligation_shape(&controller, &contract)?;
    Ok(ControllerObligation {
        controller_sha256: digest(controller_source),
        reset_input: contract.controller_reset_input,
        velocity_input: contract.controller_velocity_input,
        braking_state: contract.controller_braking_state,
        brake_output: contract.controller_brake_output,
        velocity_width,
        brake_velocity,
    })
}

pub fn verify_controller_obligation(
    controller_source: &[u8],
    obligation: &ControllerObligation,
) -> Result<(), ComponentError> {
    if obligation.controller_sha256 != digest(controller_source) {
        return Err(reject("controller obligation source binding is invalid"));
    }
    let controller =
        Arc::new(btor2::parse_bytes(controller_source).map_err(|error| reject(error.to_string()))?);
    let projection = ComponentContract {
        controller_reset_input: obligation.reset_input,
        controller_velocity_input: obligation.velocity_input,
        controller_braking_state: obligation.braking_state,
        controller_brake_output: obligation.brake_output,
        plant_reset_input: 0,
        plant_brake_input: 0,
        plant_velocity_state: 0,
        plant_position_state: 0,
        plant_bad_property: 0,
    };
    let (velocity_width, brake_velocity) = controller_obligation_shape(&controller, &projection)?;
    if velocity_width != obligation.velocity_width || brake_velocity != obligation.brake_velocity {
        return Err(reject("controller obligation claim does not match source"));
    }
    Ok(())
}

pub fn encode_controller_obligation(
    obligation: &ControllerObligation,
) -> Result<String, ComponentError> {
    if !valid_digest(&obligation.controller_sha256) {
        return Err(reject("controller obligation digest is not canonical"));
    }
    let text = format!(
        "controller_obligation_version={CONTROLLER_OBLIGATION_VERSION}\ncontroller_sha256={}\nreset_input={}\nvelocity_input={}\nbraking_state={}\nbrake_output={}\nvelocity_width={}\nbrake_velocity={}\nstatus=complete\n",
        obligation.controller_sha256,
        obligation.reset_input,
        obligation.velocity_input,
        obligation.braking_state,
        obligation.brake_output,
        obligation.velocity_width,
        obligation.brake_velocity,
    );
    if text.len() > MAX_CONTROLLER_OBLIGATION_BYTES {
        return Err(reject("controller obligation exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode_controller_obligation(bytes: &[u8]) -> Result<ControllerObligation, ComponentError> {
    let text = canonical_text(
        bytes,
        "controller obligation",
        MAX_CONTROLLER_OBLIGATION_BYTES,
    )?;
    let mut lines = text.lines();
    fn take<'a>(lines: &mut std::str::Lines<'a>, key: &str) -> Result<&'a str, ComponentError> {
        lines
            .next()
            .and_then(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    let version = parse_number::<u32>(
        take(&mut lines, "controller_obligation_version")?,
        "controller obligation version",
    )?;
    if version != CONTROLLER_OBLIGATION_VERSION {
        return Err(reject("unsupported controller obligation version"));
    }
    let obligation = ControllerObligation {
        controller_sha256: take(&mut lines, "controller_sha256")?.to_string(),
        reset_input: parse_number(take(&mut lines, "reset_input")?, "reset input")?,
        velocity_input: parse_number(take(&mut lines, "velocity_input")?, "velocity input")?,
        braking_state: parse_number(take(&mut lines, "braking_state")?, "braking state")?,
        brake_output: parse_number(take(&mut lines, "brake_output")?, "brake output")?,
        velocity_width: parse_number(take(&mut lines, "velocity_width")?, "velocity width")?,
        brake_velocity: parse_number(take(&mut lines, "brake_velocity")?, "brake velocity")?,
    };
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "controller obligation is incomplete or has trailing fields",
        ));
    }
    if encode_controller_obligation(&obligation)? != text {
        return Err(reject("controller obligation is not canonical"));
    }
    Ok(obligation)
}

fn add_literal(model: &Btor2Model, expression: NodeId, state: NodeId) -> Option<u64> {
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Add, left, right) if left == state => constant(model, right),
        NodeKind::Binary(BinaryOp::Add, left, right) if right == state => constant(model, left),
        _ => None,
    }
}

fn is_sum(model: &Btor2Model, expression: NodeId, left: NodeId, right: NodeId) -> bool {
    matches!(
        model.nodes().get(&expression).map(|node| &node.kind),
        Some(NodeKind::Binary(BinaryOp::Add, first, second))
            if (*first, *second) == (left, right) || (*first, *second) == (right, left)
    )
}

fn recognise_phase(composition: &Composition<'_>) -> Option<PhaseShape> {
    let c = &composition.controller;
    let p = &composition.plant;
    let k = composition.contract;
    if !zero(c, c.initialiser(k.controller_braking_state)?)
        || !zero(p, p.initialiser(k.plant_velocity_state)?)
        || !zero(p, p.initialiser(k.plant_position_state)?)
    {
        return None;
    }
    let controller_advance =
        reset_advance(c, k.controller_braking_state, k.controller_reset_input)?;
    if controller_advance != k.controller_brake_output {
        return None;
    }
    let (braking, guard) = match c.nodes().get(&k.controller_brake_output)?.kind {
        NodeKind::Binary(BinaryOp::Or, left, right) if left == k.controller_braking_state => {
            (left, right)
        }
        NodeKind::Binary(BinaryOp::Or, left, right) if right == k.controller_braking_state => {
            (right, left)
        }
        _ => return None,
    };
    let _ = braking;
    let brake_velocity = match c.nodes().get(&guard)?.kind {
        NodeKind::Binary(BinaryOp::Ugte, velocity, threshold)
            if velocity == k.controller_velocity_input =>
        {
            constant(c, threshold).filter(|value| *value != 0)?
        }
        _ => return None,
    };
    let velocity_advance = reset_advance(p, k.plant_velocity_state, k.plant_reset_input)?;
    let (brake_expression, acceleration_expression) = match p.nodes().get(&velocity_advance)?.kind {
        NodeKind::Ite(input, brake, acceleration) if input == k.plant_brake_input => {
            (brake, acceleration)
        }
        _ => return None,
    };
    let acceleration = add_literal(p, acceleration_expression, k.plant_velocity_state)
        .filter(|value| *value != 0)?;
    let (near_zero, zero_value, subtraction) = match p.nodes().get(&brake_expression)?.kind {
        NodeKind::Ite(guard, zero_value, subtraction) if zero(p, zero_value) => {
            (guard, zero_value, subtraction)
        }
        _ => return None,
    };
    let _ = zero_value;
    let literal = match p.nodes().get(&subtraction)?.kind {
        NodeKind::Binary(BinaryOp::Sub, state, literal) if state == k.plant_velocity_state => {
            literal
        }
        _ => return None,
    };
    let deceleration = constant(p, literal).filter(|value| *value != 0)?;
    if !matches!(
        p.nodes().get(&near_zero).map(|node| &node.kind),
        Some(NodeKind::Binary(BinaryOp::Ulte, state, bound))
            if *state == k.plant_velocity_state && constant(p, *bound) == Some(deceleration)
    ) {
        return None;
    }
    let position_advance = reset_advance(p, k.plant_position_state, k.plant_reset_input)?;
    if !is_sum(
        p,
        position_advance,
        k.plant_position_state,
        k.plant_velocity_state,
    ) {
        return None;
    }
    let bad_expression = p
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == k.plant_bad_property).then_some(*expression))?;
    let position_threshold = match p.nodes().get(&bad_expression)?.kind {
        NodeKind::Binary(BinaryOp::Ugte, state, threshold) if state == k.plant_position_state => {
            constant(p, threshold)?
        }
        _ => return None,
    };
    Some(PhaseShape {
        width: p.nodes()[&k.plant_velocity_state].width,
        acceleration,
        brake_velocity,
        deceleration,
        position_threshold,
    })
}

// Checker path: rebuild every source-side interface obligation directly from
// the contract and certificate rather than trusting the producer's candidate.
fn checker_phase_shape(
    composition: &Composition<'_>,
    certificate: &ComponentPhaseCertificate,
) -> Result<PhaseShape, ComponentError> {
    let controller = &composition.controller;
    let plant = &composition.plant;
    let contract = composition.contract;
    if !zero(
        controller,
        controller
            .initialiser(contract.controller_braking_state)
            .ok_or_else(|| reject("controller braking initialiser is absent"))?,
    ) || !zero(
        plant,
        plant
            .initialiser(contract.plant_velocity_state)
            .ok_or_else(|| reject("plant velocity initialiser is absent"))?,
    ) || !zero(
        plant,
        plant
            .initialiser(contract.plant_position_state)
            .ok_or_else(|| reject("plant position initialiser is absent"))?,
    ) {
        return Err(reject("component initial values are not literal zero"));
    }
    let controller_next = reset_advance(
        controller,
        contract.controller_braking_state,
        contract.controller_reset_input,
    )
    .ok_or_else(|| reject("controller latch update is outside contract language"))?;
    if controller_next != contract.controller_brake_output {
        return Err(reject("controller output is not the latched next control"));
    }
    let guard = match controller.nodes()[&contract.controller_brake_output].kind {
        NodeKind::Binary(BinaryOp::Or, left, right)
            if left == contract.controller_braking_state =>
        {
            right
        }
        NodeKind::Binary(BinaryOp::Or, left, right)
            if right == contract.controller_braking_state =>
        {
            left
        }
        _ => {
            return Err(reject(
                "controller brake output is outside contract language",
            ));
        }
    };
    let brake_velocity = match controller.nodes()[&guard].kind {
        NodeKind::Binary(BinaryOp::Ugte, velocity, threshold)
            if velocity == contract.controller_velocity_input =>
        {
            constant(controller, threshold)
                .filter(|value| *value != 0)
                .ok_or_else(|| reject("controller brake threshold is not nonzero literal"))?
        }
        _ => {
            return Err(reject(
                "controller velocity guard is outside contract language",
            ));
        }
    };
    let velocity_next = reset_advance(
        plant,
        contract.plant_velocity_state,
        contract.plant_reset_input,
    )
    .ok_or_else(|| reject("plant velocity update is outside contract language"))?;
    let (brake_expression, acceleration_expression) = match plant.nodes()[&velocity_next].kind {
        NodeKind::Ite(input, brake, accelerate) if input == contract.plant_brake_input => {
            (brake, accelerate)
        }
        _ => {
            return Err(reject(
                "plant control selection is outside contract language",
            ));
        }
    };
    let acceleration = add_literal(
        plant,
        acceleration_expression,
        contract.plant_velocity_state,
    )
    .filter(|value| *value != 0)
    .ok_or_else(|| reject("plant acceleration is not nonzero literal addition"))?;
    let (near_zero, subtraction) = match plant.nodes()[&brake_expression].kind {
        NodeKind::Ite(guard, zero_value, subtraction) if zero(plant, zero_value) => {
            (guard, subtraction)
        }
        _ => {
            return Err(reject(
                "plant braking expression is outside contract language",
            ));
        }
    };
    let literal = match plant.nodes()[&subtraction].kind {
        NodeKind::Binary(BinaryOp::Sub, state, literal)
            if state == contract.plant_velocity_state =>
        {
            literal
        }
        _ => return Err(reject("plant deceleration is outside contract language")),
    };
    let deceleration = constant(plant, literal)
        .filter(|value| *value != 0)
        .ok_or_else(|| reject("plant deceleration is not a nonzero literal"))?;
    match plant.nodes()[&near_zero].kind {
        NodeKind::Binary(BinaryOp::Ulte, state, bound)
            if state == contract.plant_velocity_state
                && constant(plant, bound) == Some(deceleration) => {}
        _ => return Err(reject("plant saturation guard does not match deceleration")),
    }
    let position_next = reset_advance(
        plant,
        contract.plant_position_state,
        contract.plant_reset_input,
    )
    .ok_or_else(|| reject("plant position update is outside contract language"))?;
    if !is_sum(
        plant,
        position_next,
        contract.plant_position_state,
        contract.plant_velocity_state,
    ) {
        return Err(reject("plant position is not updated from old velocity"));
    }
    let bad_expression = plant
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == contract.plant_bad_property).then_some(*expression))
        .ok_or_else(|| reject("plant bad property is absent"))?;
    let position_threshold = match plant.nodes()[&bad_expression].kind {
        NodeKind::Binary(BinaryOp::Ugte, state, threshold)
            if state == contract.plant_position_state =>
        {
            constant(plant, threshold)
                .ok_or_else(|| reject("plant bad threshold is not literal"))?
        }
        _ => return Err(reject("plant bad property is outside contract language")),
    };
    let shape = PhaseShape {
        width: plant.nodes()[&contract.plant_velocity_state].width,
        acceleration,
        brake_velocity,
        deceleration,
        position_threshold,
    };
    if shape.width != certificate.width
        || shape.acceleration != certificate.acceleration
        || shape.brake_velocity != certificate.brake_velocity
        || shape.deceleration != certificate.deceleration
        || shape.position_threshold != certificate.position_threshold
    {
        return Err(reject("component phase constants do not match sources"));
    }
    Ok(shape)
}

fn contract_matches_obligation(
    contract: &ComponentContract,
    obligation: &ControllerObligation,
) -> bool {
    contract.controller_reset_input == obligation.reset_input
        && contract.controller_velocity_input == obligation.velocity_input
        && contract.controller_braking_state == obligation.braking_state
        && contract.controller_brake_output == obligation.brake_output
}

fn checker_reused_phase_shape(
    plant_source: &[u8],
    contract: &ComponentContract,
    obligation: &ControllerObligation,
    member: &ReusedPhaseMember,
) -> Result<PhaseShape, ComponentError> {
    if !contract_matches_obligation(contract, obligation) {
        return Err(reject(
            "component contract controller projection does not match obligation",
        ));
    }
    let plant = btor2::parse_bytes(plant_source).map_err(|error| reject(error.to_string()))?;
    if plant.inputs() != [contract.plant_reset_input, contract.plant_brake_input]
        || plant.states() != [contract.plant_velocity_state, contract.plant_position_state]
        || !plant.constraints().is_empty()
    {
        return Err(reject("plant state or input vectors do not match contract"));
    }
    if node_width(&plant, contract.plant_reset_input) != Some(1)
        || node_width(&plant, contract.plant_brake_input) != Some(1)
        || node_width(&plant, contract.plant_velocity_state) != Some(obligation.velocity_width)
        || node_width(&plant, contract.plant_position_state) != Some(obligation.velocity_width)
    {
        return Err(reject(
            "plant wire widths do not match controller obligation",
        ));
    }
    let bad_expression = plant
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == contract.plant_bad_property).then_some(*expression))
        .ok_or_else(|| reject("plant bad property is absent"))?;
    if depends_on_input(&plant, bad_expression, &mut BTreeMap::new()) {
        return Err(reject("reused phase requires a state-only plant property"));
    }
    if !zero(
        &plant,
        plant
            .initialiser(contract.plant_velocity_state)
            .ok_or_else(|| reject("plant velocity initialiser is absent"))?,
    ) || !zero(
        &plant,
        plant
            .initialiser(contract.plant_position_state)
            .ok_or_else(|| reject("plant position initialiser is absent"))?,
    ) {
        return Err(reject("plant initial values are not literal zero"));
    }
    let velocity_next = reset_advance(
        &plant,
        contract.plant_velocity_state,
        contract.plant_reset_input,
    )
    .ok_or_else(|| reject("plant velocity update is outside contract language"))?;
    let (brake_expression, acceleration_expression) = match plant.nodes()[&velocity_next].kind {
        NodeKind::Ite(input, brake, accelerate) if input == contract.plant_brake_input => {
            (brake, accelerate)
        }
        _ => {
            return Err(reject(
                "plant control selection is outside contract language",
            ));
        }
    };
    let acceleration = add_literal(
        &plant,
        acceleration_expression,
        contract.plant_velocity_state,
    )
    .filter(|value| *value != 0)
    .ok_or_else(|| reject("plant acceleration is not nonzero literal addition"))?;
    let (near_zero, subtraction) = match plant.nodes()[&brake_expression].kind {
        NodeKind::Ite(guard, zero_value, subtraction) if zero(&plant, zero_value) => {
            (guard, subtraction)
        }
        _ => {
            return Err(reject(
                "plant braking expression is outside contract language",
            ));
        }
    };
    let literal = match plant.nodes()[&subtraction].kind {
        NodeKind::Binary(BinaryOp::Sub, state, literal)
            if state == contract.plant_velocity_state =>
        {
            literal
        }
        _ => return Err(reject("plant deceleration is outside contract language")),
    };
    let deceleration = constant(&plant, literal)
        .filter(|value| *value != 0)
        .ok_or_else(|| reject("plant deceleration is not a nonzero literal"))?;
    match plant.nodes()[&near_zero].kind {
        NodeKind::Binary(BinaryOp::Ulte, state, bound)
            if state == contract.plant_velocity_state
                && constant(&plant, bound) == Some(deceleration) => {}
        _ => return Err(reject("plant saturation guard does not match deceleration")),
    }
    let position_next = reset_advance(
        &plant,
        contract.plant_position_state,
        contract.plant_reset_input,
    )
    .ok_or_else(|| reject("plant position update is outside contract language"))?;
    if !is_sum(
        &plant,
        position_next,
        contract.plant_position_state,
        contract.plant_velocity_state,
    ) {
        return Err(reject("plant position is not updated from old velocity"));
    }
    let position_threshold = match plant.nodes()[&bad_expression].kind {
        NodeKind::Binary(BinaryOp::Ugte, state, threshold)
            if state == contract.plant_position_state =>
        {
            constant(&plant, threshold)
                .ok_or_else(|| reject("plant bad threshold is not literal"))?
        }
        _ => return Err(reject("plant bad property is outside contract language")),
    };
    let shape = PhaseShape {
        width: obligation.velocity_width,
        acceleration,
        brake_velocity: obligation.brake_velocity,
        deceleration,
        position_threshold,
    };
    if member.acceleration != shape.acceleration
        || member.deceleration != shape.deceleration
        || member.position_threshold != shape.position_threshold
    {
        return Err(reject("reused phase constants do not match plant source"));
    }
    Ok(shape)
}

fn ceil_div(numerator: u128, denominator: u128) -> Option<u128> {
    numerator
        .checked_add(denominator.checked_sub(1)?)?
        .checked_div(denominator)
}

fn mask(width: u32) -> u128 {
    if width == 64 {
        u128::from(u64::MAX)
    } else {
        (1u128 << width) - 1
    }
}

fn producer_endpoint(shape: PhaseShape, horizon: u64) -> Option<Endpoint> {
    let a = u128::from(shape.acceleration);
    let d = u128::from(shape.deceleration);
    let switch = ceil_div(u128::from(shape.brake_velocity), a)?;
    let peak = switch.checked_mul(a)?;
    let braking = ceil_div(peak, d)?;
    let stop = switch.checked_add(braking)?;
    let h = u128::from(horizon);
    let accelerated = h.min(switch);
    let accelerating_position = a
        .checked_mul(accelerated.checked_mul(accelerated.saturating_sub(1))?)?
        .checked_div(2)?;
    let braked = h.saturating_sub(switch).min(braking);
    let braking_position = braked.checked_mul(peak)?.checked_sub(
        d.checked_mul(braked.checked_mul(braked.saturating_sub(1))?)?
            .checked_div(2)?,
    )?;
    let position = accelerating_position.checked_add(braking_position)?;
    let velocity = if h < switch {
        h.checked_mul(a)?
    } else {
        peak.saturating_sub(braked.checked_mul(d)?)
    };
    let max_velocity = h.min(switch).checked_mul(a)?;
    if position > mask(shape.width)
        || velocity > mask(shape.width)
        || max_velocity > mask(shape.width)
        || switch > u128::from(u64::MAX)
        || stop > u128::from(u64::MAX)
    {
        return None;
    }
    Some(Endpoint {
        velocity: velocity as u64,
        position: position as u64,
        max_velocity: max_velocity as u64,
        switch_frame: switch as u64,
        stop_frame: stop as u64,
    })
}

fn average_sum(first: u128, last: u128, count: u128) -> Option<u128> {
    let pair = first.checked_add(last)?;
    if pair.is_multiple_of(2) {
        pair.checked_div(2)?.checked_mul(count)
    } else if count.is_multiple_of(2) {
        count.checked_div(2)?.checked_mul(pair)
    } else {
        None
    }
}

fn checker_endpoint(shape: PhaseShape, horizon: u64) -> Option<Endpoint> {
    let a = u128::from(shape.acceleration);
    let d = u128::from(shape.deceleration);
    let threshold = u128::from(shape.brake_velocity);
    let switch = ceil_div(threshold, a)?;
    let peak = switch.checked_mul(a)?;
    if switch == 0 || (switch - 1).checked_mul(a)? >= threshold || peak < threshold {
        return None;
    }
    let braking = ceil_div(peak, d)?;
    if braking == 0 || (braking - 1).checked_mul(d)? >= peak || braking.checked_mul(d)? < peak {
        return None;
    }
    let stop = switch.checked_add(braking)?;
    let h = u128::from(horizon);
    let accelerated = h.min(switch);
    let accelerating_position = average_sum(
        0,
        accelerated.saturating_sub(1).checked_mul(a)?,
        accelerated,
    )?;
    let braked = h.saturating_sub(switch).min(braking);
    let braking_position = if braked == 0 {
        0
    } else {
        average_sum(
            peak,
            peak.checked_sub(braked.saturating_sub(1).checked_mul(d)?)?,
            braked,
        )?
    };
    let position = accelerating_position.checked_add(braking_position)?;
    let velocity = if h < switch {
        h.checked_mul(a)?
    } else if braked == braking {
        0
    } else {
        peak.checked_sub(braked.checked_mul(d)?)?
    };
    let max_velocity = h.min(switch).checked_mul(a)?;
    if position > mask(shape.width)
        || velocity > mask(shape.width)
        || max_velocity > mask(shape.width)
        || switch > u128::from(u64::MAX)
        || stop > u128::from(u64::MAX)
    {
        return None;
    }
    Some(Endpoint {
        velocity: velocity as u64,
        position: position as u64,
        max_velocity: max_velocity as u64,
        switch_frame: switch as u64,
        stop_frame: stop as u64,
    })
}

fn logical_state_count(horizon: u32, stop: u64) -> Option<u64> {
    let h = u64::from(horizon);
    let capped = h.min(stop);
    let prefix = capped
        .checked_add(1)?
        .checked_mul(capped.checked_add(2)?)?
        .checked_div(2)?;
    prefix.checked_add(h.saturating_sub(stop).checked_mul(stop.checked_add(1)?)?)
}

fn try_produce_phase(
    controller_source: &[u8],
    plant_source: &[u8],
    contract_source: &[u8],
    contract: &ComponentContract,
    horizon: u32,
) -> Result<Option<ComponentPhaseCertificate>, ComponentError> {
    if horizon > MAX_COMPONENT_PHASE_HORIZON {
        return Err(reject("component phase horizon exceeds limit"));
    }
    let composition = build_composition(controller_source, plant_source, contract)?;
    let Some(shape) = recognise_phase(&composition) else {
        return Ok(None);
    };
    let Some(endpoint) = producer_endpoint(shape, u64::from(horizon)) else {
        return Ok(None);
    };
    if endpoint.position >= shape.position_threshold {
        return Ok(None);
    }
    Ok(Some(ComponentPhaseCertificate {
        controller_sha256: digest(controller_source),
        plant_sha256: digest(plant_source),
        contract_sha256: digest(contract_source),
        query_horizon: horizon,
        width: shape.width,
        acceleration: shape.acceleration,
        brake_velocity: shape.brake_velocity,
        deceleration: shape.deceleration,
        position_threshold: shape.position_threshold,
        switch_frame: endpoint.switch_frame,
        stop_frame: endpoint.stop_frame,
        max_velocity: endpoint.max_velocity,
        max_position: endpoint.position,
    }))
}

fn verify_phase(
    controller_source: &[u8],
    plant_source: &[u8],
    contract_source: &[u8],
    contract: &ComponentContract,
    certificate: &ComponentPhaseCertificate,
) -> Result<ComponentSummary, ComponentError> {
    if certificate.controller_sha256 != digest(controller_source)
        || certificate.plant_sha256 != digest(plant_source)
        || certificate.contract_sha256 != digest(contract_source)
        || certificate.query_horizon > MAX_COMPONENT_PHASE_HORIZON
    {
        return Err(reject(
            "component phase source binding or horizon is invalid",
        ));
    }
    let composition = build_composition(controller_source, plant_source, contract)?;
    verify_phase_composition(&composition, certificate)
}

fn verify_phase_composition(
    composition: &Composition<'_>,
    certificate: &ComponentPhaseCertificate,
) -> Result<ComponentSummary, ComponentError> {
    let shape = checker_phase_shape(composition, certificate)?;
    let endpoint = checker_endpoint(shape, u64::from(certificate.query_horizon))
        .ok_or_else(|| reject("component phase arithmetic is not exact"))?;
    if endpoint.switch_frame != certificate.switch_frame
        || endpoint.stop_frame != certificate.stop_frame
        || endpoint.max_velocity != certificate.max_velocity
        || endpoint.position != certificate.max_position
        || endpoint.position >= certificate.position_threshold
    {
        return Err(reject("component phase claim is not exact and safe"));
    }
    Ok(ComponentSummary {
        backend: ComponentBackend::PhaseContract,
        result: ComponentResult::Safe,
        query_horizon: certificate.query_horizon,
        bad_frame: None,
        logical_reachable_states: logical_state_count(
            certificate.query_horizon,
            endpoint.stop_frame,
        )
        .ok_or_else(|| reject("component logical state count overflowed"))?,
    })
}

pub fn produce(
    controller_source: &[u8],
    plant_source: &[u8],
    contract_source: &[u8],
    horizon: u32,
) -> Result<ComponentProduction, ComponentError> {
    let contract = parse_contract(contract_source)?;
    if let Some(certificate) = try_produce_phase(
        controller_source,
        plant_source,
        contract_source,
        &contract,
        horizon,
    )? {
        return Ok(ComponentProduction {
            certificate: ComponentCertificate::Phase(certificate),
            selection_reason: ComponentSelectionReason::ExactPhaseContractSafe,
        });
    }
    produce_search(
        controller_source,
        plant_source,
        contract_source,
        &contract,
        horizon,
    )
    .map(|certificate| ComponentProduction {
        certificate: ComponentCertificate::Search(certificate),
        selection_reason: ComponentSelectionReason::SpecialisedInapplicableOrIntersecting,
    })
}

pub fn verify(
    controller_source: &[u8],
    plant_source: &[u8],
    contract_source: &[u8],
    certificate: &ComponentCertificate,
) -> Result<ComponentSummary, ComponentError> {
    let contract = parse_contract(contract_source)?;
    match certificate {
        ComponentCertificate::Phase(certificate) => verify_phase(
            controller_source,
            plant_source,
            contract_source,
            &contract,
            certificate,
        ),
        ComponentCertificate::Search(certificate) => verify_search(
            controller_source,
            plant_source,
            contract_source,
            &contract,
            certificate,
        ),
    }
}

pub fn produce_naive_component_batch(
    controller_source: &[u8],
    inputs: &[ComponentBatchInput<'_>],
) -> Result<NaiveComponentBatchCertificate, ComponentError> {
    if inputs.is_empty() || inputs.len() > MAX_COMPONENT_BATCH_MEMBERS {
        return Err(reject("component batch member count is outside limit"));
    }
    let mut members = Vec::with_capacity(inputs.len());
    for input in inputs {
        members.push(
            produce(
                controller_source,
                input.plant_source,
                input.contract_source,
                input.horizon,
            )?
            .certificate,
        );
    }
    Ok(NaiveComponentBatchCertificate {
        controller_sha256: digest(controller_source),
        members,
    })
}

pub fn verify_naive_component_batch(
    controller_source: &[u8],
    inputs: &[ComponentBatchInput<'_>],
    certificate: &NaiveComponentBatchCertificate,
) -> Result<ComponentBatchSummary, ComponentError> {
    if inputs.is_empty()
        || inputs.len() > MAX_COMPONENT_BATCH_MEMBERS
        || certificate.members.len() != inputs.len()
        || certificate.controller_sha256 != digest(controller_source)
    {
        return Err(reject("component batch binding or member count is invalid"));
    }
    let controller =
        Arc::new(btor2::parse_bytes(controller_source).map_err(|error| reject(error.to_string()))?);
    let mut members = Vec::with_capacity(inputs.len());
    let mut safe = 0usize;
    let mut unsafe_count = 0usize;
    for (input, member) in inputs.iter().zip(&certificate.members) {
        let member_horizon = match member {
            ComponentCertificate::Phase(member) => member.query_horizon,
            ComponentCertificate::Search(member) => member.query_horizon,
        };
        if member_horizon != input.horizon {
            return Err(reject("component batch horizon binding is invalid"));
        }
        let contract = parse_contract(input.contract_source)?;
        let composition =
            build_composition_with_controller(&controller, input.plant_source, &contract)?;
        let summary = match member {
            ComponentCertificate::Phase(member) => {
                if member.controller_sha256 != digest(controller_source)
                    || member.plant_sha256 != digest(input.plant_source)
                    || member.contract_sha256 != digest(input.contract_source)
                    || member.query_horizon > MAX_COMPONENT_PHASE_HORIZON
                {
                    return Err(reject(
                        "component phase source binding or horizon is invalid",
                    ));
                }
                verify_phase_composition(&composition, member)?
            }
            ComponentCertificate::Search(member) => {
                if member.controller_sha256 != digest(controller_source)
                    || member.plant_sha256 != digest(input.plant_source)
                    || member.contract_sha256 != digest(input.contract_source)
                    || member.query_horizon > MAX_COMPONENT_SEARCH_HORIZON
                {
                    return Err(reject(
                        "component search source binding or horizon is invalid",
                    ));
                }
                verify_search_composition(&composition, member)?
            }
        };
        match summary.result {
            ComponentResult::Safe => safe += 1,
            ComponentResult::Unsafe => unsafe_count += 1,
        }
        members.push(summary);
    }
    Ok(ComponentBatchSummary {
        members,
        safe,
        unsafe_count,
    })
}

pub fn produce_reusable_component_batch(
    controller_source: &[u8],
    inputs: &[ComponentBatchInput<'_>],
) -> Result<ReusableComponentBatchCertificate, ComponentError> {
    if inputs.is_empty() || inputs.len() > MAX_COMPONENT_BATCH_MEMBERS {
        return Err(reject("component batch member count is outside limit"));
    }
    let controller_obligation =
        produce_controller_obligation(controller_source, inputs[0].contract_source)?;
    let mut members = Vec::with_capacity(inputs.len());
    for input in inputs {
        let contract = parse_contract(input.contract_source)?;
        if !contract_matches_obligation(&contract, &controller_obligation) {
            return Err(reject(
                "component contract controller projection does not match shared obligation",
            ));
        }
        let production = produce(
            controller_source,
            input.plant_source,
            input.contract_source,
            input.horizon,
        )?;
        let member = match production.certificate {
            ComponentCertificate::Phase(phase) => {
                if phase.controller_sha256 != controller_obligation.controller_sha256
                    || phase.width != controller_obligation.velocity_width
                    || phase.brake_velocity != controller_obligation.brake_velocity
                {
                    return Err(reject(
                        "phase certificate does not match shared controller obligation",
                    ));
                }
                ReusableBatchMember::ReusedPhase(ReusedPhaseMember {
                    plant_sha256: phase.plant_sha256,
                    contract_sha256: phase.contract_sha256,
                    query_horizon: phase.query_horizon,
                    acceleration: phase.acceleration,
                    deceleration: phase.deceleration,
                    position_threshold: phase.position_threshold,
                    switch_frame: phase.switch_frame,
                    stop_frame: phase.stop_frame,
                    max_velocity: phase.max_velocity,
                    max_position: phase.max_position,
                })
            }
            fallback => ReusableBatchMember::ExactFallback(fallback),
        };
        members.push(member);
    }
    Ok(ReusableComponentBatchCertificate {
        controller_obligation,
        members,
    })
}

pub fn verify_reusable_component_batch(
    controller_source: &[u8],
    inputs: &[ComponentBatchInput<'_>],
    certificate: &ReusableComponentBatchCertificate,
) -> Result<ComponentBatchSummary, ComponentError> {
    if inputs.is_empty()
        || inputs.len() > MAX_COMPONENT_BATCH_MEMBERS
        || certificate.members.len() != inputs.len()
    {
        return Err(reject("component batch binding or member count is invalid"));
    }
    verify_controller_obligation(controller_source, &certificate.controller_obligation)?;
    let mut members = Vec::with_capacity(inputs.len());
    let mut safe = 0usize;
    let mut unsafe_count = 0usize;
    for (input, member) in inputs.iter().zip(&certificate.members) {
        let summary = match member {
            ReusableBatchMember::ReusedPhase(member) => {
                if member.query_horizon != input.horizon
                    || member.query_horizon > MAX_COMPONENT_PHASE_HORIZON
                    || member.plant_sha256 != digest(input.plant_source)
                    || member.contract_sha256 != digest(input.contract_source)
                {
                    return Err(reject("reused phase source binding or horizon is invalid"));
                }
                let contract = parse_contract(input.contract_source)?;
                let shape = checker_reused_phase_shape(
                    input.plant_source,
                    &contract,
                    &certificate.controller_obligation,
                    member,
                )?;
                let endpoint = checker_endpoint(shape, u64::from(member.query_horizon))
                    .ok_or_else(|| reject("reused phase arithmetic is not exact"))?;
                if endpoint.switch_frame != member.switch_frame
                    || endpoint.stop_frame != member.stop_frame
                    || endpoint.max_velocity != member.max_velocity
                    || endpoint.position != member.max_position
                    || endpoint.position >= member.position_threshold
                {
                    return Err(reject("reused phase claim is not exact and safe"));
                }
                ComponentSummary {
                    backend: ComponentBackend::PhaseContract,
                    result: ComponentResult::Safe,
                    query_horizon: member.query_horizon,
                    bad_frame: None,
                    logical_reachable_states: logical_state_count(
                        member.query_horizon,
                        endpoint.stop_frame,
                    )
                    .ok_or_else(|| reject("component logical state count overflowed"))?,
                }
            }
            ReusableBatchMember::ExactFallback(member) => {
                let horizon = match member {
                    ComponentCertificate::Phase(member) => member.query_horizon,
                    ComponentCertificate::Search(member) => member.query_horizon,
                };
                if horizon != input.horizon {
                    return Err(reject("component batch horizon binding is invalid"));
                }
                verify(
                    controller_source,
                    input.plant_source,
                    input.contract_source,
                    member,
                )?
            }
        };
        match summary.result {
            ComponentResult::Safe => safe += 1,
            ComponentResult::Unsafe => unsafe_count += 1,
        }
        members.push(summary);
    }
    Ok(ComponentBatchSummary {
        members,
        safe,
        unsafe_count,
    })
}

pub fn produce_component_batch_portfolio(
    controller_source: &[u8],
    inputs: &[ComponentBatchInput<'_>],
) -> Result<ComponentBatchPortfolioProduction, ComponentError> {
    let reusable = produce_reusable_component_batch(controller_source, inputs)?;
    if reusable.members.len() >= 2
        && reusable
            .members
            .iter()
            .all(|member| matches!(member, ReusableBatchMember::ReusedPhase(_)))
    {
        return Ok(ComponentBatchPortfolioProduction {
            certificate: ComponentBatchPortfolioCertificate::Reusable(reusable),
            selection_reason: ComponentBatchSelectionReason::FullyAdmittedReuse,
        });
    }
    let obligation = &reusable.controller_obligation;
    let members = reusable
        .members
        .into_iter()
        .map(|member| match member {
            ReusableBatchMember::ReusedPhase(member) => {
                ComponentCertificate::Phase(ComponentPhaseCertificate {
                    controller_sha256: obligation.controller_sha256.clone(),
                    plant_sha256: member.plant_sha256,
                    contract_sha256: member.contract_sha256,
                    query_horizon: member.query_horizon,
                    width: obligation.velocity_width,
                    acceleration: member.acceleration,
                    brake_velocity: obligation.brake_velocity,
                    deceleration: member.deceleration,
                    position_threshold: member.position_threshold,
                    switch_frame: member.switch_frame,
                    stop_frame: member.stop_frame,
                    max_velocity: member.max_velocity,
                    max_position: member.max_position,
                })
            }
            ReusableBatchMember::ExactFallback(member) => member,
        })
        .collect();
    Ok(ComponentBatchPortfolioProduction {
        certificate: ComponentBatchPortfolioCertificate::Ordinary(NaiveComponentBatchCertificate {
            controller_sha256: obligation.controller_sha256.clone(),
            members,
        }),
        selection_reason: ComponentBatchSelectionReason::SingletonOrExactFallback,
    })
}

pub fn verify_component_batch_portfolio(
    controller_source: &[u8],
    inputs: &[ComponentBatchInput<'_>],
    certificate: &ComponentBatchPortfolioCertificate,
) -> Result<ComponentBatchSummary, ComponentError> {
    match certificate {
        ComponentBatchPortfolioCertificate::Reusable(certificate) => {
            if certificate.members.len() < 2
                || certificate
                    .members
                    .iter()
                    .any(|member| !matches!(member, ReusableBatchMember::ReusedPhase(_)))
            {
                return Err(reject(
                    "reusable portfolio route violates static selection gate",
                ));
            }
            verify_reusable_component_batch(controller_source, inputs, certificate)
        }
        ComponentBatchPortfolioCertificate::Ordinary(certificate) => {
            verify_naive_component_batch(controller_source, inputs, certificate)
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str, label: &str) -> Result<Vec<u8>, ComponentError> {
    if !value.len().is_multiple_of(2)
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        return Err(reject(format!("{label} is not canonical lowercase hex")));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|text| u8::from_str_radix(text, 16).ok())
                .ok_or_else(|| reject(format!("invalid {label}")))
        })
        .collect()
}

pub fn encode_reusable_component_batch(
    certificate: &ReusableComponentBatchCertificate,
) -> Result<String, ComponentError> {
    if certificate.members.is_empty() || certificate.members.len() > MAX_COMPONENT_BATCH_MEMBERS {
        return Err(reject("component batch member count is outside limit"));
    }
    let obligation = encode_controller_obligation(&certificate.controller_obligation)?;
    let mut text = format!(
        "reusable_component_batch_version={REUSABLE_COMPONENT_BATCH_VERSION}\ncontroller_obligation_hex={}\nmember_count={}\n",
        hex_encode(obligation.as_bytes()),
        certificate.members.len(),
    );
    for member in &certificate.members {
        match member {
            ReusableBatchMember::ReusedPhase(member) => {
                if !valid_digest(&member.plant_sha256) || !valid_digest(&member.contract_sha256) {
                    return Err(reject("reused phase digest is not canonical"));
                }
                text.push_str(&format!(
                    "member=reused-phase\nplant_sha256={}\ncontract_sha256={}\nquery_horizon={}\nacceleration={}\ndeceleration={}\nposition_threshold={}\nswitch_frame={}\nstop_frame={}\nmax_velocity={}\nmax_position={}\n",
                    member.plant_sha256,
                    member.contract_sha256,
                    member.query_horizon,
                    member.acceleration,
                    member.deceleration,
                    member.position_threshold,
                    member.switch_frame,
                    member.stop_frame,
                    member.max_velocity,
                    member.max_position,
                ));
            }
            ReusableBatchMember::ExactFallback(member) => {
                text.push_str("member=exact-fallback\ncertificate_hex=");
                text.push_str(&hex_encode(encode(member)?.as_bytes()));
                text.push('\n');
            }
        }
        if text.len() > MAX_REUSABLE_COMPONENT_BATCH_BYTES {
            return Err(reject("reusable component batch exceeds byte limit"));
        }
    }
    text.push_str("status=complete\n");
    if text.len() > MAX_REUSABLE_COMPONENT_BATCH_BYTES {
        return Err(reject("reusable component batch exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode_reusable_component_batch(
    bytes: &[u8],
) -> Result<ReusableComponentBatchCertificate, ComponentError> {
    let text = canonical_text(
        bytes,
        "reusable component batch",
        MAX_REUSABLE_COMPONENT_BATCH_BYTES,
    )?;
    let mut lines = text.lines();
    fn take<'a>(lines: &mut std::str::Lines<'a>, key: &str) -> Result<&'a str, ComponentError> {
        lines
            .next()
            .and_then(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    let version: u32 = parse_number(
        take(&mut lines, "reusable_component_batch_version")?,
        "reusable component batch version",
    )?;
    if version != REUSABLE_COMPONENT_BATCH_VERSION {
        return Err(reject("unsupported reusable component batch version"));
    }
    let obligation_bytes = hex_decode(
        take(&mut lines, "controller_obligation_hex")?,
        "controller obligation",
    )?;
    if obligation_bytes.len() > MAX_CONTROLLER_OBLIGATION_BYTES {
        return Err(reject("controller obligation exceeds byte limit"));
    }
    let controller_obligation = decode_controller_obligation(&obligation_bytes)?;
    let count: usize = parse_number(take(&mut lines, "member_count")?, "member count")?;
    if count == 0 || count > MAX_COMPONENT_BATCH_MEMBERS {
        return Err(reject("component batch member count is outside limit"));
    }
    let mut members = Vec::with_capacity(count);
    for _ in 0..count {
        let member = match take(&mut lines, "member")? {
            "reused-phase" => {
                let member = ReusedPhaseMember {
                    plant_sha256: take(&mut lines, "plant_sha256")?.to_string(),
                    contract_sha256: take(&mut lines, "contract_sha256")?.to_string(),
                    query_horizon: parse_number(
                        take(&mut lines, "query_horizon")?,
                        "query horizon",
                    )?,
                    acceleration: parse_number(take(&mut lines, "acceleration")?, "acceleration")?,
                    deceleration: parse_number(take(&mut lines, "deceleration")?, "deceleration")?,
                    position_threshold: parse_number(
                        take(&mut lines, "position_threshold")?,
                        "position threshold",
                    )?,
                    switch_frame: parse_number(take(&mut lines, "switch_frame")?, "switch frame")?,
                    stop_frame: parse_number(take(&mut lines, "stop_frame")?, "stop frame")?,
                    max_velocity: parse_number(take(&mut lines, "max_velocity")?, "max velocity")?,
                    max_position: parse_number(take(&mut lines, "max_position")?, "max position")?,
                };
                if !valid_digest(&member.plant_sha256) || !valid_digest(&member.contract_sha256) {
                    return Err(reject("reused phase digest is not canonical"));
                }
                ReusableBatchMember::ReusedPhase(member)
            }
            "exact-fallback" => {
                let certificate = hex_decode(
                    take(&mut lines, "certificate_hex")?,
                    "component certificate",
                )?;
                if certificate.len() > MAX_COMPONENT_CERTIFICATE_BYTES {
                    return Err(reject("component certificate exceeds byte limit"));
                }
                ReusableBatchMember::ExactFallback(decode(&certificate)?)
            }
            _ => return Err(reject("unknown reusable component batch member")),
        };
        members.push(member);
    }
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "reusable component batch is incomplete or has trailing fields",
        ));
    }
    let certificate = ReusableComponentBatchCertificate {
        controller_obligation,
        members,
    };
    if encode_reusable_component_batch(&certificate)? != text {
        return Err(reject("reusable component batch is not canonical"));
    }
    Ok(certificate)
}

pub fn encode_component_batch_portfolio(
    certificate: &ComponentBatchPortfolioCertificate,
) -> Result<String, ComponentError> {
    if let ComponentBatchPortfolioCertificate::Reusable(certificate) = certificate {
        if certificate.members.len() < 2
            || certificate
                .members
                .iter()
                .any(|member| !matches!(member, ReusableBatchMember::ReusedPhase(_)))
        {
            return Err(reject(
                "reusable portfolio route violates static selection gate",
            ));
        }
        return encode_reusable_component_batch(certificate);
    }
    let mut text =
        format!("component_batch_portfolio_version={COMPONENT_BATCH_PORTFOLIO_VERSION}\n");
    match certificate {
        ComponentBatchPortfolioCertificate::Reusable(_) => unreachable!(),
        ComponentBatchPortfolioCertificate::Ordinary(certificate) => {
            if certificate.members.is_empty()
                || certificate.members.len() > MAX_COMPONENT_BATCH_MEMBERS
                || !valid_digest(&certificate.controller_sha256)
            {
                return Err(reject("ordinary portfolio batch is not canonical"));
            }
            text.push_str(&format!(
                "route=ordinary\ncontroller_sha256={}\nmember_count={}\n",
                certificate.controller_sha256,
                certificate.members.len(),
            ));
            for member in &certificate.members {
                text.push_str("certificate_hex=");
                text.push_str(&hex_encode(encode(member)?.as_bytes()));
                text.push('\n');
                if text.len() > MAX_COMPONENT_BATCH_PORTFOLIO_BYTES {
                    return Err(reject("component batch portfolio exceeds byte limit"));
                }
            }
        }
    }
    text.push_str("status=complete\n");
    if text.len() > MAX_COMPONENT_BATCH_PORTFOLIO_BYTES {
        return Err(reject("component batch portfolio exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode_component_batch_portfolio(
    bytes: &[u8],
) -> Result<ComponentBatchPortfolioCertificate, ComponentError> {
    if bytes.starts_with(b"reusable_component_batch_version=") {
        let certificate = decode_reusable_component_batch(bytes)?;
        if certificate.members.len() < 2
            || certificate
                .members
                .iter()
                .any(|member| !matches!(member, ReusableBatchMember::ReusedPhase(_)))
        {
            return Err(reject(
                "reusable portfolio route violates static selection gate",
            ));
        }
        return Ok(ComponentBatchPortfolioCertificate::Reusable(certificate));
    }
    let text = canonical_text(
        bytes,
        "component batch portfolio",
        MAX_COMPONENT_BATCH_PORTFOLIO_BYTES,
    )?;
    let mut lines = text.lines();
    fn take<'a>(lines: &mut std::str::Lines<'a>, key: &str) -> Result<&'a str, ComponentError> {
        lines
            .next()
            .and_then(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    let version: u32 = parse_number(
        take(&mut lines, "component_batch_portfolio_version")?,
        "component batch portfolio version",
    )?;
    if version != COMPONENT_BATCH_PORTFOLIO_VERSION {
        return Err(reject("unsupported component batch portfolio version"));
    }
    let certificate = match take(&mut lines, "route")? {
        "ordinary" => {
            let controller_sha256 = take(&mut lines, "controller_sha256")?.to_string();
            if !valid_digest(&controller_sha256) {
                return Err(reject("ordinary portfolio digest is not canonical"));
            }
            let count: usize = parse_number(take(&mut lines, "member_count")?, "member count")?;
            if count == 0 || count > MAX_COMPONENT_BATCH_MEMBERS {
                return Err(reject("component batch member count is outside limit"));
            }
            let mut members = Vec::with_capacity(count);
            for _ in 0..count {
                let bytes = hex_decode(
                    take(&mut lines, "certificate_hex")?,
                    "component certificate",
                )?;
                if bytes.len() > MAX_COMPONENT_CERTIFICATE_BYTES {
                    return Err(reject("component certificate exceeds byte limit"));
                }
                members.push(decode(&bytes)?);
            }
            ComponentBatchPortfolioCertificate::Ordinary(NaiveComponentBatchCertificate {
                controller_sha256,
                members,
            })
        }
        _ => return Err(reject("unknown component batch portfolio route")),
    };
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "component batch portfolio is incomplete or has trailing fields",
        ));
    }
    if encode_component_batch_portfolio(&certificate)? != text {
        return Err(reject("component batch portfolio is not canonical"));
    }
    Ok(certificate)
}

fn encode_state(state: &ComponentState) -> String {
    fn side(values: &[(NodeId, u64)]) -> String {
        values
            .iter()
            .map(|(id, value)| format!("{id}:{value}"))
            .collect::<Vec<_>>()
            .join(",")
    }
    format!("{}|{}", side(&state.controller), side(&state.plant))
}

fn decode_state(value: &str) -> Result<ComponentState, ComponentError> {
    fn side(value: &str) -> Result<Vec<(NodeId, u64)>, ComponentError> {
        if value.is_empty() {
            return Err(reject("component state side is empty"));
        }
        value
            .split(',')
            .map(|entry| {
                let (id, value) = entry
                    .split_once(':')
                    .ok_or_else(|| reject("invalid component state entry"))?;
                Ok((
                    parse_number(id, "component state id")?,
                    parse_number(value, "component state value")?,
                ))
            })
            .collect()
    }
    let (controller, plant) = value
        .split_once('|')
        .ok_or_else(|| reject("invalid component state"))?;
    Ok(ComponentState {
        controller: side(controller)?,
        plant: side(plant)?,
    })
}

pub fn encode(certificate: &ComponentCertificate) -> Result<String, ComponentError> {
    let mut text = String::new();
    match certificate {
        ComponentCertificate::Phase(c) => {
            for digest in [&c.controller_sha256, &c.plant_sha256, &c.contract_sha256] {
                if !valid_digest(digest) {
                    return Err(reject("component phase digest is not canonical"));
                }
            }
            text = format!(
                "component_certificate_version={COMPONENT_CERTIFICATE_VERSION}\nbackend=phase-contract\ncontroller_sha256={}\nplant_sha256={}\ncontract_sha256={}\nquery_horizon={}\nwidth={}\nacceleration={}\nbrake_velocity={}\ndeceleration={}\nposition_threshold={}\nswitch_frame={}\nstop_frame={}\nmax_velocity={}\nmax_position={}\nresult=SAFE\nstatus=complete\n",
                c.controller_sha256,
                c.plant_sha256,
                c.contract_sha256,
                c.query_horizon,
                c.width,
                c.acceleration,
                c.brake_velocity,
                c.deceleration,
                c.position_threshold,
                c.switch_frame,
                c.stop_frame,
                c.max_velocity,
                c.max_position,
            );
        }
        ComponentCertificate::Search(c) => {
            for digest in [&c.controller_sha256, &c.plant_sha256, &c.contract_sha256] {
                if !valid_digest(digest) {
                    return Err(reject("component search digest is not canonical"));
                }
            }
            let result = match c.result {
                ComponentResult::Safe => "SAFE",
                ComponentResult::Unsafe => "UNSAFE",
            };
            let bad_frame = c
                .bad_frame
                .map_or_else(|| "none".to_string(), |v| v.to_string());
            let resets = c
                .witness_resets
                .iter()
                .map(|value| if *value { "1" } else { "0" })
                .collect::<Vec<_>>()
                .join(",");
            text.push_str(&format!(
                "component_certificate_version={COMPONENT_CERTIFICATE_VERSION}\nbackend=composed-search\ncontroller_sha256={}\nplant_sha256={}\ncontract_sha256={}\nquery_horizon={}\nresult={result}\nbad_frame={bad_frame}\nwitness_resets={resets}\nlayer_count={}\n",
                c.controller_sha256,
                c.plant_sha256,
                c.contract_sha256,
                c.query_horizon,
                c.layers.len(),
            ));
            for (index, layer) in c.layers.iter().enumerate() {
                text.push_str(&format!("layer={index},{}\n", layer.len()));
                for state in layer {
                    text.push_str(&format!("state={}\n", encode_state(state)));
                }
            }
            text.push_str("status=complete\n");
        }
    }
    if text.len() > MAX_COMPONENT_CERTIFICATE_BYTES {
        return Err(reject("component certificate exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode(bytes: &[u8]) -> Result<ComponentCertificate, ComponentError> {
    let text = canonical_text(
        bytes,
        "component certificate",
        MAX_COMPONENT_CERTIFICATE_BYTES,
    )?;
    let mut lines = text.lines();
    fn take<'a>(lines: &mut std::str::Lines<'a>, key: &str) -> Result<&'a str, ComponentError> {
        lines
            .next()
            .and_then(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    let version: u32 = parse_number(
        take(&mut lines, "component_certificate_version")?,
        "component certificate version",
    )?;
    if version != COMPONENT_CERTIFICATE_VERSION {
        return Err(reject("unsupported component certificate version"));
    }
    let backend = take(&mut lines, "backend")?;
    let controller_sha256 = take(&mut lines, "controller_sha256")?.to_string();
    let plant_sha256 = take(&mut lines, "plant_sha256")?.to_string();
    let contract_sha256 = take(&mut lines, "contract_sha256")?.to_string();
    for digest in [&controller_sha256, &plant_sha256, &contract_sha256] {
        if !valid_digest(digest) {
            return Err(reject("component certificate digest is not canonical"));
        }
    }
    let query_horizon = parse_number(take(&mut lines, "query_horizon")?, "query horizon")?;
    let certificate = if backend == "phase-contract" {
        let phase = ComponentPhaseCertificate {
            controller_sha256,
            plant_sha256,
            contract_sha256,
            query_horizon,
            width: parse_number(take(&mut lines, "width")?, "width")?,
            acceleration: parse_number(take(&mut lines, "acceleration")?, "acceleration")?,
            brake_velocity: parse_number(take(&mut lines, "brake_velocity")?, "brake velocity")?,
            deceleration: parse_number(take(&mut lines, "deceleration")?, "deceleration")?,
            position_threshold: parse_number(
                take(&mut lines, "position_threshold")?,
                "position threshold",
            )?,
            switch_frame: parse_number(take(&mut lines, "switch_frame")?, "switch frame")?,
            stop_frame: parse_number(take(&mut lines, "stop_frame")?, "stop frame")?,
            max_velocity: parse_number(take(&mut lines, "max_velocity")?, "max velocity")?,
            max_position: parse_number(take(&mut lines, "max_position")?, "max position")?,
        };
        if take(&mut lines, "result")? != "SAFE" {
            return Err(reject("phase contract result must be SAFE"));
        }
        ComponentCertificate::Phase(phase)
    } else if backend == "composed-search" {
        let result = match take(&mut lines, "result")? {
            "SAFE" => ComponentResult::Safe,
            "UNSAFE" => ComponentResult::Unsafe,
            _ => return Err(reject("invalid component search result")),
        };
        let bad_frame = match take(&mut lines, "bad_frame")? {
            "none" => None,
            value => Some(parse_number(value, "bad frame")?),
        };
        let witness = take(&mut lines, "witness_resets")?;
        let witness_resets = if witness.is_empty() {
            Vec::new()
        } else {
            witness
                .split(',')
                .map(|value| match value {
                    "0" => Ok(false),
                    "1" => Ok(true),
                    _ => Err(reject("invalid component reset witness")),
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let layer_count: usize = parse_number(take(&mut lines, "layer_count")?, "layer count")?;
        if layer_count > MAX_COMPONENT_SEARCH_HORIZON as usize + 1 {
            return Err(reject("component layer count exceeds limit"));
        }
        let mut layers = Vec::with_capacity(layer_count);
        let mut total = 0usize;
        for expected_index in 0..layer_count {
            let header = lines
                .next()
                .ok_or_else(|| reject("missing component layer"))?
                .strip_prefix("layer=")
                .ok_or_else(|| reject("invalid component layer header"))?;
            let (index, count) = header
                .split_once(',')
                .ok_or_else(|| reject("invalid component layer header"))?;
            if parse_number::<usize>(index, "layer index")? != expected_index {
                return Err(reject("component layer index is not canonical"));
            }
            let count: usize = parse_number(count, "layer state count")?;
            if count > MAX_COMPONENT_STATES_PER_LAYER {
                return Err(reject("component layer exceeds state limit"));
            }
            total = total
                .checked_add(count)
                .filter(|value| *value <= MAX_COMPONENT_TOTAL_STATES)
                .ok_or_else(|| reject("component certificate exceeds total state limit"))?;
            let mut layer = Vec::with_capacity(count);
            for _ in 0..count {
                layer.push(decode_state(take(&mut lines, "state")?)?);
            }
            layers.push(layer);
        }
        ComponentCertificate::Search(ComponentSearchCertificate {
            controller_sha256,
            plant_sha256,
            contract_sha256,
            query_horizon,
            result,
            bad_frame,
            witness_resets,
            layers,
        })
    } else {
        return Err(reject("unknown component certificate backend"));
    };
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "component certificate is incomplete or has trailing fields",
        ));
    }
    if encode(&certificate)? != text {
        return Err(reject("component certificate is not canonical"));
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROLLER: &[u8] =
        include_bytes!("../examples/btor2/components/braking-controller-v1.btor2");
    const PLANT: &[u8] = include_bytes!("../examples/btor2/components/motion-plant-v1.btor2");
    const SEMI_IMPLICIT: &[u8] =
        include_bytes!("../examples/btor2/components/semi-implicit-motion-plant-v1.btor2");
    const CONTRACT: &[u8] =
        include_bytes!("../examples/btor2/components/braking-motion-contract-v1.txt");

    #[test]
    fn phase_contract_proves_safe_boundary_and_round_trips() {
        let production = produce(CONTROLLER, PLANT, CONTRACT, 255).unwrap();
        assert_eq!(
            production.selection_reason,
            ComponentSelectionReason::ExactPhaseContractSafe
        );
        let encoded = encode(&production.certificate).unwrap();
        assert!(encoded.len() < 600);
        let summary = verify(
            CONTROLLER,
            PLANT,
            CONTRACT,
            &decode(encoded.as_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(summary.backend, ComponentBackend::PhaseContract);
        assert_eq!(summary.result, ComponentResult::Safe);
        assert_eq!(summary.logical_reachable_states, 32_896);
    }

    #[test]
    fn controller_obligation_is_source_bound_canonical_and_reusable() {
        let obligation = produce_controller_obligation(CONTROLLER, CONTRACT).unwrap();
        assert_eq!(obligation.velocity_width, 16);
        assert_eq!(obligation.brake_velocity, 256);
        let encoded = encode_controller_obligation(&obligation).unwrap();
        assert!(encoded.len() < 350);
        let decoded = decode_controller_obligation(encoded.as_bytes()).unwrap();
        verify_controller_obligation(CONTROLLER, &decoded).unwrap();

        let mut changed_source = CONTROLLER.to_vec();
        let position = changed_source
            .iter()
            .position(|byte| *byte == b'8')
            .unwrap();
        changed_source[position] = b'9';
        assert!(verify_controller_obligation(&changed_source, &decoded).is_err());

        let mut changed_claim = decoded;
        changed_claim.brake_velocity += 1;
        assert!(verify_controller_obligation(CONTROLLER, &changed_claim).is_err());
    }

    #[test]
    fn every_controller_obligation_mutation_and_truncation_fails_closed() {
        let encoded = encode_controller_obligation(
            &produce_controller_obligation(CONTROLLER, CONTRACT).unwrap(),
        )
        .unwrap()
        .into_bytes();
        for end in 0..encoded.len() {
            assert!(decode_controller_obligation(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(obligation) = decode_controller_obligation(&mutated) {
                assert!(verify_controller_obligation(CONTROLLER, &obligation).is_err());
            }
        }
    }

    #[test]
    fn exact_composed_search_preserves_unsafe_and_near_neighbour_answers() {
        let unsafe_production = produce(CONTROLLER, PLANT, CONTRACT, 256).unwrap();
        assert_eq!(
            unsafe_production.selection_reason,
            ComponentSelectionReason::SpecialisedInapplicableOrIntersecting
        );
        let summary = verify(CONTROLLER, PLANT, CONTRACT, &unsafe_production.certificate).unwrap();
        assert_eq!(summary.backend, ComponentBackend::ComposedSearch);
        assert_eq!(summary.result, ComponentResult::Unsafe);
        assert_eq!(summary.bad_frame, Some(256));

        for (horizon, expected) in [(127, ComponentResult::Safe), (128, ComponentResult::Unsafe)] {
            let production = produce(CONTROLLER, SEMI_IMPLICIT, CONTRACT, horizon).unwrap();
            assert!(matches!(
                production.certificate,
                ComponentCertificate::Search(_)
            ));
            assert_eq!(
                verify(CONTROLLER, SEMI_IMPLICIT, CONTRACT, &production.certificate)
                    .unwrap()
                    .result,
                expected
            );
        }
    }

    #[test]
    fn naive_batch_baseline_preserves_mixed_exact_answers() {
        let inputs = [
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 255,
            },
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 256,
            },
            ComponentBatchInput {
                plant_source: SEMI_IMPLICIT,
                contract_source: CONTRACT,
                horizon: 127,
            },
        ];
        let certificate = produce_naive_component_batch(CONTROLLER, &inputs).unwrap();
        let summary = verify_naive_component_batch(CONTROLLER, &inputs, &certificate).unwrap();
        assert_eq!(summary.safe, 2);
        assert_eq!(summary.unsafe_count, 1);
        assert_eq!(summary.members[0].backend, ComponentBackend::PhaseContract);
        assert_eq!(summary.members[1].backend, ComponentBackend::ComposedSearch);
        assert_eq!(summary.members[2].backend, ComponentBackend::ComposedSearch);

        let mut reordered = inputs;
        reordered.swap(0, 1);
        assert!(verify_naive_component_batch(CONTROLLER, &reordered, &certificate).is_err());
    }

    #[test]
    fn reusable_batch_shares_controller_proof_and_preserves_exact_fallback() {
        let inputs = [
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 255,
            },
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 256,
            },
            ComponentBatchInput {
                plant_source: SEMI_IMPLICIT,
                contract_source: CONTRACT,
                horizon: 127,
            },
        ];
        let certificate = produce_reusable_component_batch(CONTROLLER, &inputs).unwrap();
        assert!(matches!(
            certificate.members[0],
            ReusableBatchMember::ReusedPhase(_)
        ));
        assert!(matches!(
            certificate.members[1],
            ReusableBatchMember::ExactFallback(ComponentCertificate::Search(_))
        ));
        assert!(matches!(
            certificate.members[2],
            ReusableBatchMember::ExactFallback(ComponentCertificate::Search(_))
        ));
        let summary = verify_reusable_component_batch(CONTROLLER, &inputs, &certificate).unwrap();
        assert_eq!(summary.safe, 2);
        assert_eq!(summary.unsafe_count, 1);
        assert_eq!(summary.members[0].backend, ComponentBackend::PhaseContract);
        assert_eq!(summary.members[1].backend, ComponentBackend::ComposedSearch);
        assert_eq!(summary.members[2].backend, ComponentBackend::ComposedSearch);
    }

    #[test]
    fn reusable_batch_rejects_shared_and_member_tampering() {
        let inputs = [
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 254,
            },
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 255,
            },
        ];
        let certificate = produce_reusable_component_batch(CONTROLLER, &inputs).unwrap();

        let mut changed_obligation = certificate.clone();
        changed_obligation.controller_obligation.brake_velocity += 1;
        assert!(verify_reusable_component_batch(CONTROLLER, &inputs, &changed_obligation).is_err());

        let mut changed_member = certificate.clone();
        let ReusableBatchMember::ReusedPhase(member) = &mut changed_member.members[0] else {
            panic!("expected reused phase member")
        };
        member.max_position += 1;
        assert!(verify_reusable_component_batch(CONTROLLER, &inputs, &changed_member).is_err());

        let mut reordered = inputs;
        reordered.swap(0, 1);
        assert!(verify_reusable_component_batch(CONTROLLER, &reordered, &certificate).is_err());
    }

    #[test]
    fn reusable_batch_codec_is_canonical_bounded_and_fail_closed() {
        let inputs = [ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 255,
        }];
        let certificate = produce_reusable_component_batch(CONTROLLER, &inputs).unwrap();
        let encoded = encode_reusable_component_batch(&certificate).unwrap();
        let decoded = decode_reusable_component_batch(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, certificate);
        verify_reusable_component_batch(CONTROLLER, &inputs, &decoded).unwrap();

        for end in 0..encoded.len() {
            assert!(decode_reusable_component_batch(&encoded.as_bytes()[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.as_bytes().to_vec();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(decoded) = decode_reusable_component_batch(&mutated) {
                assert!(verify_reusable_component_batch(CONTROLLER, &inputs, &decoded).is_err());
            }
        }
        assert!(
            decode_reusable_component_batch(&vec![b'x'; MAX_REUSABLE_COMPONENT_BATCH_BYTES + 1])
                .is_err()
        );
    }

    #[test]
    fn component_batch_portfolio_statically_selects_only_measured_win_regime() {
        let admitted = [
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 254,
            },
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 255,
            },
        ];
        let production = produce_component_batch_portfolio(CONTROLLER, &admitted).unwrap();
        assert_eq!(
            production.selection_reason,
            ComponentBatchSelectionReason::FullyAdmittedReuse
        );
        assert!(matches!(
            production.certificate,
            ComponentBatchPortfolioCertificate::Reusable(_)
        ));
        let encoded = encode_component_batch_portfolio(&production.certificate).unwrap();
        let decoded = decode_component_batch_portfolio(encoded.as_bytes()).unwrap();
        assert_eq!(
            verify_component_batch_portfolio(CONTROLLER, &admitted, &decoded)
                .unwrap()
                .safe,
            2
        );

        let mixed = [
            admitted[0],
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 256,
            },
        ];
        let production = produce_component_batch_portfolio(CONTROLLER, &mixed).unwrap();
        assert_eq!(
            production.selection_reason,
            ComponentBatchSelectionReason::SingletonOrExactFallback
        );
        assert!(matches!(
            production.certificate,
            ComponentBatchPortfolioCertificate::Ordinary(_)
        ));
        let encoded = encode_component_batch_portfolio(&production.certificate).unwrap();
        let decoded = decode_component_batch_portfolio(encoded.as_bytes()).unwrap();
        let summary = verify_component_batch_portfolio(CONTROLLER, &mixed, &decoded).unwrap();
        assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
    }

    #[test]
    fn component_batch_portfolio_rejects_noncanonical_route_and_mutation() {
        let inputs = [
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 254,
            },
            ComponentBatchInput {
                plant_source: PLANT,
                contract_source: CONTRACT,
                horizon: 255,
            },
        ];
        let production = produce_component_batch_portfolio(CONTROLLER, &inputs).unwrap();
        let encoded = encode_component_batch_portfolio(&production.certificate).unwrap();
        for end in 0..encoded.len() {
            assert!(decode_component_batch_portfolio(&encoded.as_bytes()[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.as_bytes().to_vec();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode_component_batch_portfolio(&mutated) {
                assert!(
                    verify_component_batch_portfolio(CONTROLLER, &inputs, &certificate).is_err()
                );
            }
        }
        let ComponentBatchPortfolioCertificate::Reusable(mut reusable) = production.certificate
        else {
            panic!("expected reusable route")
        };
        reusable.members.push(ReusableBatchMember::ExactFallback(
            produce(CONTROLLER, PLANT, CONTRACT, 256)
                .unwrap()
                .certificate,
        ));
        assert!(
            encode_component_batch_portfolio(&ComponentBatchPortfolioCertificate::Reusable(
                reusable
            ))
            .is_err()
        );

        let singleton = [inputs[0]];
        let ordinary = produce_component_batch_portfolio(CONTROLLER, &singleton).unwrap();
        let encoded = encode_component_batch_portfolio(&ordinary.certificate).unwrap();
        for end in 0..encoded.len() {
            assert!(decode_component_batch_portfolio(&encoded.as_bytes()[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.as_bytes().to_vec();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode_component_batch_portfolio(&mutated) {
                assert!(
                    verify_component_batch_portfolio(CONTROLLER, &singleton, &certificate).is_err()
                );
            }
        }
        assert!(
            decode_component_batch_portfolio(&vec![b'x'; MAX_COMPONENT_BATCH_PORTFOLIO_BYTES + 1])
                .is_err()
        );
    }

    #[test]
    fn rejects_source_contract_and_claim_tampering() {
        let certificate = produce(CONTROLLER, PLANT, CONTRACT, 255)
            .unwrap()
            .certificate;
        assert!(verify(CONTROLLER, SEMI_IMPLICIT, CONTRACT, &certificate).is_err());
        let changed_contract = String::from_utf8(CONTRACT.to_vec())
            .unwrap()
            .replace("controller_brake_output=10", "controller_brake_output=9");
        assert!(verify(CONTROLLER, PLANT, changed_contract.as_bytes(), &certificate).is_err());
        let ComponentCertificate::Phase(mut phase) = certificate else {
            panic!("expected phase certificate")
        };
        phase.stop_frame += 1;
        assert!(
            verify(
                CONTROLLER,
                PLANT,
                CONTRACT,
                &ComponentCertificate::Phase(phase)
            )
            .is_err()
        );
    }

    #[test]
    fn every_phase_byte_mutation_and_truncation_fails_closed() {
        let encoded = encode(
            &produce(CONTROLLER, PLANT, CONTRACT, 255)
                .unwrap()
                .certificate,
        )
        .unwrap()
        .into_bytes();
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode(&mutated) {
                assert!(verify(CONTROLLER, PLANT, CONTRACT, &certificate).is_err());
            }
        }
    }

    #[test]
    fn rejects_hostile_contract_and_certificate_inputs() {
        assert!(parse_contract(b"component_contract_version=1\r\n").is_err());
        assert!(parse_contract(&vec![b'x'; MAX_COMPONENT_CONTRACT_BYTES + 1]).is_err());
        assert!(decode(b"component_certificate_version=1\r\n").is_err());
        assert!(decode(&vec![b'x'; MAX_COMPONENT_CERTIFICATE_BYTES + 1]).is_err());

        let ComponentCertificate::Search(mut missing_state) =
            produce(CONTROLLER, SEMI_IMPLICIT, CONTRACT, 1)
                .unwrap()
                .certificate
        else {
            panic!("expected search certificate")
        };
        missing_state.layers[0][0].plant.clear();
        assert!(
            verify(
                CONTROLLER,
                SEMI_IMPLICIT,
                CONTRACT,
                &ComponentCertificate::Search(missing_state)
            )
            .is_err()
        );

        let ComponentCertificate::Search(mut oversized_state) =
            produce(CONTROLLER, SEMI_IMPLICIT, CONTRACT, 1)
                .unwrap()
                .certificate
        else {
            panic!("expected search certificate")
        };
        oversized_state.layers[0][0].plant[0].1 = u64::MAX;
        assert!(
            verify(
                CONTROLLER,
                SEMI_IMPLICIT,
                CONTRACT,
                &ComponentCertificate::Search(oversized_state)
            )
            .is_err()
        );
    }
}
