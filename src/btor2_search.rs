//! Exact bounded reachability certificates for the strict BTOR2 semantic core.

use crate::btor2::{self, Btor2Model, NodeId, NodeKind, WordValues};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const SEARCH_CERTIFICATE_V1_VERSION: u32 = 1;
pub const SEARCH_CERTIFICATE_V2_VERSION: u32 = 2;
pub const SEARCH_CERTIFICATE_V3_VERSION: u32 = 3;
pub const SEARCH_CERTIFICATE_V4_VERSION: u32 = 4;
pub const SEARCH_CERTIFICATE_VERSION: u32 = 5;
pub const MAX_SEARCH_INPUTS: usize = 8;
pub const MAX_SEARCH_INPUT_BITS: usize = 8;
pub const MAX_SEARCH_HORIZON: u32 = 256;
pub const MAX_STATES_PER_LAYER: usize = 65_536;
pub const MAX_TOTAL_STATES: usize = 262_144;
pub const MAX_SEARCH_NODE_STEPS: u64 = 20_000_000;
pub const MAX_SEARCH_CERTIFICATE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SearchState(pub Vec<(NodeId, u64)>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchResult {
    Safe,
    Unsafe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchCertificate {
    pub certificate_version: u32,
    pub source_sha256: String,
    pub query_horizon: u32,
    pub bad_property: NodeId,
    pub input: NodeId,
    pub inputs: Vec<NodeId>,
    pub input_widths: Vec<u32>,
    pub constraints: Vec<NodeId>,
    pub result: SearchResult,
    pub bad_frame: Option<u32>,
    pub witness_inputs: Vec<bool>,
    pub terminal_input: Option<bool>,
    pub witness_valuations: Vec<u16>,
    pub terminal_valuation: Option<u16>,
    pub layers: Vec<Vec<SearchState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchSummary {
    pub result: SearchResult,
    pub query_horizon: u32,
    pub bad_frame: Option<u32>,
    pub reachable_states: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchError(pub String);

impl fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for SearchError {}

fn reject(message: impl Into<String>) -> SearchError {
    SearchError(message.into())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn state_key(values: &WordValues) -> SearchState {
    SearchState(values.iter().map(|(id, value)| (*id, *value)).collect())
}

fn state_values(state: &SearchState) -> WordValues {
    state.0.iter().copied().collect()
}

fn validate_state_shape(model: &Btor2Model, state: &SearchState) -> Result<(), SearchError> {
    if state.0.len() != model.states().len()
        || !state
            .0
            .iter()
            .zip(model.states())
            .all(|((id, value), expected)| {
                let Some(node) = model.nodes().get(id) else {
                    return false;
                };
                let width = node.width;
                let invalid_mask = if width == 64 {
                    0
                } else {
                    !((1u64 << width) - 1)
                };
                id == expected && (*value & invalid_mask) == 0
            })
    {
        return Err(reject("search state does not match source state vector"));
    }
    Ok(())
}

fn validate_model(
    model: &Btor2Model,
    bad_property: NodeId,
) -> Result<(Vec<NodeId>, Vec<u32>, Vec<NodeId>, NodeId, bool), SearchError> {
    if model.inputs().is_empty() || model.inputs().len() > MAX_SEARCH_INPUTS {
        return Err(reject(
            "bounded search requires between one and eight semantic inputs",
        ));
    }
    let input_widths = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].width)
        .collect::<Vec<_>>();
    let total_input_bits = input_widths
        .iter()
        .try_fold(0usize, |total, width| total.checked_add(*width as usize))
        .ok_or_else(|| reject("bounded search input width overflowed"))?;
    if input_widths
        .iter()
        .any(|width| *width == 0 || *width as usize > MAX_SEARCH_INPUT_BITS)
        || total_input_bits > MAX_SEARCH_INPUT_BITS
    {
        return Err(reject(
            "bounded search requires at most eight total semantic input bits",
        ));
    }
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))
        .ok_or_else(|| reject("unknown bad property"))?;
    fn depends_on_input(model: &Btor2Model, id: NodeId, memo: &mut BTreeMap<NodeId, bool>) -> bool {
        if let Some(result) = memo.get(&id) {
            return *result;
        }
        let result = match model.nodes()[&id].kind {
            NodeKind::Input => true,
            NodeKind::State | NodeKind::Constant(_) => false,
            NodeKind::Unary(_, value)
            | NodeKind::Slice { value, .. }
            | NodeKind::Uext { value, .. } => depends_on_input(model, value, memo),
            NodeKind::Binary(_, left, right) => {
                depends_on_input(model, left, memo) || depends_on_input(model, right, memo)
            }
            NodeKind::Concat { high, low } => {
                depends_on_input(model, high, memo) || depends_on_input(model, low, memo)
            }
            NodeKind::Ite(condition, then_value, else_value) => {
                depends_on_input(model, condition, memo)
                    || depends_on_input(model, then_value, memo)
                    || depends_on_input(model, else_value, memo)
            }
        };
        memo.insert(id, result);
        result
    }
    let input_dependent = depends_on_input(model, expression, &mut BTreeMap::new());
    Ok((
        model.inputs().to_vec(),
        input_widths,
        model.constraints().iter().map(|(id, _)| *id).collect(),
        expression,
        input_dependent,
    ))
}

fn uses_packed_valuations(version: u32) -> bool {
    matches!(
        version,
        SEARCH_CERTIFICATE_V3_VERSION | SEARCH_CERTIFICATE_V4_VERSION | SEARCH_CERTIFICATE_VERSION
    )
}

fn uses_constraints(version: u32) -> bool {
    matches!(
        version,
        SEARCH_CERTIFICATE_V4_VERSION | SEARCH_CERTIFICATE_VERSION
    )
}

fn total_input_bits(input_widths: &[u32]) -> usize {
    input_widths.iter().map(|width| *width as usize).sum()
}

fn valuation_count(input_widths: &[u32]) -> usize {
    1usize << total_input_bits(input_widths)
}

fn input_values(inputs: &[NodeId], input_widths: &[u32], valuation: u16) -> WordValues {
    let mut offset = 0usize;
    inputs
        .iter()
        .zip(input_widths)
        .map(|(input, width)| {
            let mask = (1u16 << *width) - 1;
            let value = (valuation >> offset) & mask;
            offset += *width as usize;
            (*input, u64::from(value))
        })
        .collect()
}

fn valuation_is_canonical(input_widths: &[u32], valuation: u16) -> bool {
    usize::from(valuation) < valuation_count(input_widths)
}

fn valuation_is_admissible(
    model: &Btor2Model,
    state: &SearchState,
    inputs: &[NodeId],
    input_widths: &[u32],
    valuation: u16,
) -> Result<bool, SearchError> {
    let state = state_values(state);
    let inputs = input_values(inputs, input_widths, valuation);
    for (_, expression) in model.constraints() {
        if model
            .evaluate(*expression, &state, &inputs)
            .map_err(|error| reject(error.to_string()))?
            == 0
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn bad_active(
    model: &Btor2Model,
    state: &SearchState,
    inputs: &[NodeId],
    input_widths: &[u32],
    valuation: u16,
    bad_property: NodeId,
) -> Result<bool, SearchError> {
    Ok(model
        .active_bad(
            &state_values(state),
            &input_values(inputs, input_widths, valuation),
        )
        .map_err(|error| reject(error.to_string()))?
        .contains(&bad_property))
}

#[derive(Default)]
struct SearchBudget {
    total_states: usize,
    node_steps: u64,
}

impl SearchBudget {
    fn add_layer(
        &mut self,
        model: &Btor2Model,
        states: usize,
        valuations: usize,
    ) -> Result<(), SearchError> {
        if states > MAX_STATES_PER_LAYER {
            return Err(reject("reachable layer exceeds state limit"));
        }
        self.total_states = self
            .total_states
            .checked_add(states)
            .filter(|total| *total <= MAX_TOTAL_STATES)
            .ok_or_else(|| reject("search exceeds total reachable-state limit"))?;
        self.node_steps = self
            .node_steps
            .checked_add(
                (states as u64)
                    .checked_mul(valuations as u64)
                    .and_then(|value| value.checked_mul(model.nodes().len() as u64))
                    .and_then(|value| value.checked_mul(model.states().len().max(1) as u64))
                    .ok_or_else(|| reject("search node-step estimate overflowed"))?,
            )
            .filter(|work| *work <= MAX_SEARCH_NODE_STEPS)
            .ok_or_else(|| reject("search exceeds node-step limit"))?;
        Ok(())
    }
}

pub fn produce(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<SearchCertificate, SearchError> {
    if horizon > MAX_SEARCH_HORIZON {
        return Err(reject("search horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let (inputs, input_widths, constraints, _, input_dependent) =
        validate_model(&model, bad_property)?;
    let certificate_version = if input_widths.iter().any(|width| *width > 1) {
        SEARCH_CERTIFICATE_VERSION
    } else if !constraints.is_empty() {
        SEARCH_CERTIFICATE_V4_VERSION
    } else if inputs.len() > 1 {
        SEARCH_CERTIFICATE_V3_VERSION
    } else if input_dependent {
        SEARCH_CERTIFICATE_V2_VERSION
    } else {
        SEARCH_CERTIFICATE_V1_VERSION
    };
    let input = inputs[0];
    let valuations = valuation_count(&input_widths);
    let initial = state_key(
        &model
            .initial_state()
            .map_err(|error| reject(error.to_string()))?,
    );
    let terminal_valuations = if input_dependent || !constraints.is_empty() {
        valuations
    } else {
        1
    };
    let mut initial_bad_valuation = None;
    for terminal_valuation in 0..terminal_valuations {
        if !valuation_is_admissible(
            &model,
            &initial,
            &inputs,
            &input_widths,
            terminal_valuation as u16,
        )? {
            continue;
        }
        if bad_active(
            &model,
            &initial,
            &inputs,
            &input_widths,
            terminal_valuation as u16,
            bad_property,
        )? {
            initial_bad_valuation = Some(terminal_valuation as u16);
            break;
        }
    }
    if let Some(terminal_valuation) = initial_bad_valuation {
        return Ok(SearchCertificate {
            certificate_version,
            source_sha256: digest(source),
            query_horizon: horizon,
            bad_property,
            input,
            inputs: if uses_packed_valuations(certificate_version) {
                inputs
            } else {
                Vec::new()
            },
            input_widths: if certificate_version == SEARCH_CERTIFICATE_VERSION {
                input_widths
            } else {
                Vec::new()
            },
            constraints,
            result: SearchResult::Unsafe,
            bad_frame: Some(0),
            witness_inputs: Vec::new(),
            terminal_input: (certificate_version == SEARCH_CERTIFICATE_V2_VERSION)
                .then_some(terminal_valuation != 0),
            witness_valuations: Vec::new(),
            terminal_valuation: uses_packed_valuations(certificate_version)
                .then_some(terminal_valuation),
            layers: Vec::new(),
        });
    }
    let mut layers = vec![vec![initial.clone()]];
    let mut predecessors = Vec::<BTreeMap<SearchState, (SearchState, u16)>>::new();
    let mut budget = SearchBudget::default();
    budget.add_layer(&model, 1, valuations)?;
    for frame in 0..horizon {
        let mut next = BTreeSet::new();
        let mut prior = BTreeMap::new();
        for state in &layers[frame as usize] {
            for valuation in 0..valuations {
                if !valuation_is_admissible(
                    &model,
                    state,
                    &inputs,
                    &input_widths,
                    valuation as u16,
                )? {
                    continue;
                }
                let values = model
                    .step(
                        &state_values(state),
                        &input_values(&inputs, &input_widths, valuation as u16),
                    )
                    .map_err(|error| reject(error.to_string()))?;
                let target = state_key(&values);
                if next.insert(target.clone()) {
                    prior.insert(target, (state.clone(), valuation as u16));
                }
            }
        }
        budget.add_layer(&model, next.len(), valuations)?;
        let next = next.into_iter().collect::<Vec<_>>();
        let mut bad_state = None;
        for state in &next {
            for terminal_valuation in 0..terminal_valuations {
                if !valuation_is_admissible(
                    &model,
                    state,
                    &inputs,
                    &input_widths,
                    terminal_valuation as u16,
                )? {
                    continue;
                }
                if bad_active(
                    &model,
                    state,
                    &inputs,
                    &input_widths,
                    terminal_valuation as u16,
                    bad_property,
                )? {
                    bad_state = Some((state.clone(), terminal_valuation as u16));
                    break;
                }
            }
            if bad_state.is_some() {
                break;
            }
        }
        if let Some((bad_state, terminal_valuation)) = bad_state {
            predecessors.push(prior);
            let mut cursor = bad_state;
            let mut witness = Vec::with_capacity((frame + 1) as usize);
            for predecessor_layer in predecessors.iter().rev() {
                let (previous, valuation) = predecessor_layer
                    .get(&cursor)
                    .ok_or_else(|| reject("internal predecessor chain is incomplete"))?;
                witness.push(*valuation);
                cursor = previous.clone();
            }
            witness.reverse();
            return Ok(SearchCertificate {
                certificate_version,
                source_sha256: digest(source),
                query_horizon: horizon,
                bad_property,
                input,
                inputs: if uses_packed_valuations(certificate_version) {
                    inputs
                } else {
                    Vec::new()
                },
                input_widths: if certificate_version == SEARCH_CERTIFICATE_VERSION {
                    input_widths
                } else {
                    Vec::new()
                },
                constraints,
                result: SearchResult::Unsafe,
                bad_frame: Some(frame + 1),
                witness_inputs: if uses_packed_valuations(certificate_version) {
                    Vec::new()
                } else {
                    witness.iter().map(|value| *value != 0).collect()
                },
                terminal_input: (certificate_version == SEARCH_CERTIFICATE_V2_VERSION)
                    .then_some(terminal_valuation != 0),
                witness_valuations: if uses_packed_valuations(certificate_version) {
                    witness
                } else {
                    Vec::new()
                },
                terminal_valuation: uses_packed_valuations(certificate_version)
                    .then_some(terminal_valuation),
                layers: Vec::new(),
            });
        }
        predecessors.push(prior);
        layers.push(next);
    }
    Ok(SearchCertificate {
        certificate_version,
        source_sha256: digest(source),
        query_horizon: horizon,
        bad_property,
        input,
        inputs: if uses_packed_valuations(certificate_version) {
            inputs
        } else {
            Vec::new()
        },
        input_widths: if certificate_version == SEARCH_CERTIFICATE_VERSION {
            input_widths
        } else {
            Vec::new()
        },
        constraints,
        result: SearchResult::Safe,
        bad_frame: None,
        witness_inputs: Vec::new(),
        terminal_input: None,
        witness_valuations: Vec::new(),
        terminal_valuation: None,
        layers,
    })
}

pub fn verify(
    source: &[u8],
    certificate: &SearchCertificate,
) -> Result<SearchSummary, SearchError> {
    if certificate.source_sha256 != digest(source) {
        return Err(reject("search certificate source digest mismatch"));
    }
    if certificate.query_horizon > MAX_SEARCH_HORIZON {
        return Err(reject("search certificate horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let (inputs, input_widths, constraints, _, input_dependent) =
        validate_model(&model, certificate.bad_property)?;
    let input = inputs[0];
    if input != certificate.input {
        return Err(reject("search certificate input does not match source"));
    }
    let valuations = valuation_count(&input_widths);
    let all_one_bit = input_widths.iter().all(|width| *width == 1);
    match certificate.certificate_version {
        SEARCH_CERTIFICATE_V1_VERSION
            if inputs.len() != 1 || !all_one_bit || input_dependent || !constraints.is_empty() =>
        {
            return Err(reject(
                "search certificate v1 requires a state-only bad property",
            ));
        }
        SEARCH_CERTIFICATE_V1_VERSION => {
            if certificate.terminal_input.is_some()
                || !certificate.inputs.is_empty()
                || !certificate.witness_valuations.is_empty()
                || certificate.terminal_valuation.is_some()
                || !certificate.input_widths.is_empty()
                || !certificate.constraints.is_empty()
            {
                return Err(reject("search certificate v1 has v2 or v3 fields"));
            }
        }
        SEARCH_CERTIFICATE_V2_VERSION => {
            if inputs.len() != 1 || !all_one_bit || !input_dependent || !constraints.is_empty() {
                return Err(reject(
                    "search certificate v2 requires an input-dependent bad property",
                ));
            }
            if !certificate.inputs.is_empty()
                || !certificate.witness_valuations.is_empty()
                || certificate.terminal_valuation.is_some()
                || !certificate.input_widths.is_empty()
                || !certificate.constraints.is_empty()
            {
                return Err(reject("search certificate v2 has v3 fields"));
            }
        }
        SEARCH_CERTIFICATE_V3_VERSION => {
            if inputs.len() < 2
                || !all_one_bit
                || !constraints.is_empty()
                || certificate.inputs != inputs
                || !certificate.input_widths.is_empty()
                || !certificate.constraints.is_empty()
                || certificate.terminal_input.is_some()
                || !certificate.witness_inputs.is_empty()
            {
                return Err(reject("search certificate v3 input binding is invalid"));
            }
        }
        SEARCH_CERTIFICATE_V4_VERSION => {
            if constraints.is_empty()
                || !all_one_bit
                || certificate.inputs != inputs
                || !certificate.input_widths.is_empty()
                || certificate.constraints != constraints
                || certificate.terminal_input.is_some()
                || !certificate.witness_inputs.is_empty()
            {
                return Err(reject(
                    "search certificate v4 input or constraint binding is invalid",
                ));
            }
        }
        SEARCH_CERTIFICATE_VERSION => {
            if all_one_bit
                || certificate.inputs != inputs
                || certificate.input_widths != input_widths
                || certificate.constraints != constraints
                || certificate.terminal_input.is_some()
                || !certificate.witness_inputs.is_empty()
            {
                return Err(reject(
                    "search certificate v5 input width or constraint binding is invalid",
                ));
            }
        }
        _ => return Err(reject("unsupported search certificate version")),
    }
    let initial = state_key(
        &model
            .initial_state()
            .map_err(|error| reject(error.to_string()))?,
    );
    match certificate.result {
        SearchResult::Unsafe => {
            let witness_len = if uses_packed_valuations(certificate.certificate_version) {
                certificate.witness_valuations.len()
            } else {
                certificate.witness_inputs.len()
            };
            if !certificate.layers.is_empty()
                || certificate.bad_frame != Some(witness_len as u32)
                || witness_len > certificate.query_horizon as usize
                || (certificate.certificate_version == SEARCH_CERTIFICATE_V2_VERSION
                    && certificate.terminal_input.is_none())
                || (uses_packed_valuations(certificate.certificate_version)
                    && certificate.terminal_valuation.is_none())
            {
                return Err(reject("UNSAFE search certificate shape is invalid"));
            }
            if certificate
                .witness_valuations
                .iter()
                .chain(certificate.terminal_valuation.iter())
                .any(|valuation| !valuation_is_canonical(&input_widths, *valuation))
            {
                return Err(reject("search certificate v3 valuation is noncanonical"));
            }
            let witness_work = (witness_len as u64)
                .checked_mul(model.nodes().len() as u64)
                .and_then(|value| value.checked_mul(model.states().len().max(1) as u64))
                .ok_or_else(|| reject("UNSAFE witness node-step estimate overflowed"))?;
            if witness_work > MAX_SEARCH_NODE_STEPS {
                return Err(reject("UNSAFE witness exceeds node-step limit"));
            }
            let mut state = state_values(&initial);
            let witness = if uses_packed_valuations(certificate.certificate_version) {
                certificate.witness_valuations.clone()
            } else {
                certificate
                    .witness_inputs
                    .iter()
                    .map(|bit| u16::from(*bit))
                    .collect()
            };
            for valuation in witness {
                let current = state_key(&state);
                if !valuation_is_admissible(&model, &current, &inputs, &input_widths, valuation)? {
                    return Err(reject(
                        "UNSAFE witness uses an inadmissible transition valuation",
                    ));
                }
                state = model
                    .step(&state, &input_values(&inputs, &input_widths, valuation))
                    .map_err(|error| reject(error.to_string()))?;
            }
            let final_state = state_key(&state);
            let terminal_valuation = if uses_packed_valuations(certificate.certificate_version) {
                certificate.terminal_valuation.unwrap_or(0)
            } else {
                u16::from(certificate.terminal_input.unwrap_or(false))
            };
            if !valuation_is_admissible(
                &model,
                &final_state,
                &inputs,
                &input_widths,
                terminal_valuation,
            )? {
                return Err(reject(
                    "UNSAFE witness uses an inadmissible terminal valuation",
                ));
            }
            if !bad_active(
                &model,
                &final_state,
                &inputs,
                &input_widths,
                terminal_valuation,
                certificate.bad_property,
            )? {
                return Err(reject("UNSAFE witness does not reach the bad property"));
            }
            Ok(SearchSummary {
                result: SearchResult::Unsafe,
                query_horizon: certificate.query_horizon,
                bad_frame: certificate.bad_frame,
                reachable_states: witness_len + 1,
            })
        }
        SearchResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_inputs.is_empty()
                || certificate.terminal_input.is_some()
                || !certificate.witness_valuations.is_empty()
                || certificate.terminal_valuation.is_some()
                || certificate.layers.len() != certificate.query_horizon as usize + 1
                || certificate.layers.first() != Some(&vec![initial])
            {
                return Err(reject("SAFE search certificate shape is invalid"));
            }
            let mut budget = SearchBudget::default();
            for (frame, layer) in certificate.layers.iter().enumerate() {
                if (layer.is_empty() && !uses_constraints(certificate.certificate_version))
                    || !layer.windows(2).all(|pair| pair[0] < pair[1])
                {
                    return Err(reject(format!("reachable layer {frame} is noncanonical")));
                }
                for state in layer {
                    validate_state_shape(&model, state)?;
                }
                budget.add_layer(&model, layer.len(), valuations)?;
                for state in layer {
                    let terminal_valuations = if input_dependent || !constraints.is_empty() {
                        valuations
                    } else {
                        1
                    };
                    for terminal_valuation in 0..terminal_valuations {
                        if !valuation_is_admissible(
                            &model,
                            state,
                            &inputs,
                            &input_widths,
                            terminal_valuation as u16,
                        )? {
                            continue;
                        }
                        if bad_active(
                            &model,
                            state,
                            &inputs,
                            &input_widths,
                            terminal_valuation as u16,
                            certificate.bad_property,
                        )? {
                            return Err(reject(format!(
                                "reachable layer {frame} contains a bad valuation"
                            )));
                        }
                    }
                }
                if frame + 1 < certificate.layers.len() {
                    let mut expected = BTreeSet::new();
                    for state in layer {
                        for valuation in 0..valuations {
                            if !valuation_is_admissible(
                                &model,
                                state,
                                &inputs,
                                &input_widths,
                                valuation as u16,
                            )? {
                                continue;
                            }
                            let target = model
                                .step(
                                    &state_values(state),
                                    &input_values(&inputs, &input_widths, valuation as u16),
                                )
                                .map_err(|error| reject(error.to_string()))?;
                            expected.insert(state_key(&target));
                        }
                    }
                    if expected.iter().cloned().collect::<Vec<_>>() != certificate.layers[frame + 1]
                    {
                        return Err(reject(format!(
                            "reachable layer {} is not the complete successor set",
                            frame + 1
                        )));
                    }
                }
            }
            Ok(SearchSummary {
                result: SearchResult::Safe,
                query_horizon: certificate.query_horizon,
                bad_frame: None,
                reachable_states: budget.total_states,
            })
        }
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn encode(certificate: &SearchCertificate) -> Result<String, SearchError> {
    if !matches!(
        certificate.certificate_version,
        SEARCH_CERTIFICATE_V1_VERSION
            | SEARCH_CERTIFICATE_V2_VERSION
            | SEARCH_CERTIFICATE_V3_VERSION
            | SEARCH_CERTIFICATE_V4_VERSION
            | SEARCH_CERTIFICATE_VERSION
    ) {
        return Err(reject("unsupported search certificate version"));
    }
    if certificate.certificate_version == SEARCH_CERTIFICATE_V1_VERSION
        && certificate.terminal_input.is_some()
    {
        return Err(reject("search certificate v1 has a terminal input"));
    }
    if !valid_digest(&certificate.source_sha256) {
        return Err(reject("search source digest is not canonical"));
    }
    let result = match certificate.result {
        SearchResult::Safe => "SAFE",
        SearchResult::Unsafe => "UNSAFE",
    };
    let bad_frame = certificate
        .bad_frame
        .map_or_else(|| "none".to_string(), |frame| frame.to_string());
    let mut lines = vec![
        format!(
            "search_certificate_version={}",
            certificate.certificate_version
        ),
        format!("source_sha256={}", certificate.source_sha256),
        format!("query_horizon={}", certificate.query_horizon),
        format!("bad_property={}", certificate.bad_property),
    ];
    if uses_packed_valuations(certificate.certificate_version) {
        let minimum_inputs = if certificate.certificate_version == SEARCH_CERTIFICATE_V3_VERSION {
            2
        } else {
            1
        };
        let effective_widths = if certificate.certificate_version == SEARCH_CERTIFICATE_VERSION {
            certificate.input_widths.clone()
        } else {
            vec![1; certificate.inputs.len()]
        };
        if certificate.inputs.len() < minimum_inputs
            || certificate.inputs.len() > MAX_SEARCH_INPUTS
            || !certificate.inputs.windows(2).all(|pair| pair[0] < pair[1])
            || certificate.input != certificate.inputs[0]
            || certificate
                .witness_valuations
                .iter()
                .chain(certificate.terminal_valuation.iter())
                .any(|valuation| !valuation_is_canonical(&effective_widths, *valuation))
            || !certificate.witness_inputs.is_empty()
            || certificate.terminal_input.is_some()
            || (certificate.certificate_version == SEARCH_CERTIFICATE_V3_VERSION
                && (!certificate.constraints.is_empty() || !certificate.input_widths.is_empty()))
            || (certificate.certificate_version == SEARCH_CERTIFICATE_V4_VERSION
                && (certificate.constraints.is_empty()
                    || !certificate.input_widths.is_empty()
                    || !certificate
                        .constraints
                        .windows(2)
                        .all(|pair| pair[0] < pair[1])))
            || (certificate.certificate_version == SEARCH_CERTIFICATE_VERSION
                && (certificate.input_widths.len() != certificate.inputs.len()
                    || certificate.input_widths.iter().all(|width| *width == 1)
                    || certificate
                        .input_widths
                        .iter()
                        .any(|width| *width == 0 || *width as usize > MAX_SEARCH_INPUT_BITS)
                    || total_input_bits(&certificate.input_widths) > MAX_SEARCH_INPUT_BITS
                    || !certificate
                        .constraints
                        .windows(2)
                        .all(|pair| pair[0] < pair[1])))
        {
            return Err(reject("search certificate packed fields are noncanonical"));
        }
        lines.push(format!("input_count={}", certificate.inputs.len()));
        lines.push(format!(
            "inputs={}",
            certificate
                .inputs
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
        if certificate.certificate_version == SEARCH_CERTIFICATE_VERSION {
            lines.push(format!(
                "input_widths={}",
                certificate
                    .input_widths
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            lines.push(format!(
                "total_input_bits={}",
                total_input_bits(&certificate.input_widths)
            ));
        }
        if uses_constraints(certificate.certificate_version) {
            lines.push(format!(
                "constraint_count={}",
                certificate.constraints.len()
            ));
            lines.push(format!(
                "constraints={}",
                certificate
                    .constraints
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        lines.push(format!("result={result}"));
        lines.push(format!("bad_frame={bad_frame}"));
        lines.push(format!(
            "witness_valuations={}",
            certificate
                .witness_valuations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
        lines.push(format!(
            "terminal_valuation={}",
            certificate
                .terminal_valuation
                .map_or_else(|| "none".to_string(), |value| value.to_string())
        ));
    } else {
        if !certificate.inputs.is_empty()
            || !certificate.input_widths.is_empty()
            || !certificate.constraints.is_empty()
            || !certificate.witness_valuations.is_empty()
            || certificate.terminal_valuation.is_some()
        {
            return Err(reject("search certificate v1 or v2 has v3 fields"));
        }
        let witness = certificate
            .witness_inputs
            .iter()
            .map(|bit| if *bit { '1' } else { '0' })
            .collect::<String>();
        lines.push(format!("input={}", certificate.input));
        lines.push(format!("result={result}"));
        lines.push(format!("bad_frame={bad_frame}"));
        lines.push(format!("witness={witness}"));
    }
    if certificate.certificate_version == SEARCH_CERTIFICATE_V2_VERSION {
        lines.push(format!(
            "terminal_input={}",
            certificate
                .terminal_input
                .map_or("none", |value| if value { "1" } else { "0" })
        ));
    }
    lines.push(format!("layer_count={}", certificate.layers.len()));
    for (frame, layer) in certificate.layers.iter().enumerate() {
        lines.push(format!("layer_{frame}_count={}", layer.len()));
        for (index, state) in layer.iter().enumerate() {
            let fields = state
                .0
                .iter()
                .map(|(id, value)| format!("{id}:{value}"))
                .collect::<Vec<_>>()
                .join(",");
            lines.push(format!("layer_{frame}_state_{index}={fields}"));
        }
    }
    lines.push("status=complete".to_string());
    let text = format!("{}\n", lines.join("\n"));
    if text.len() > MAX_SEARCH_CERTIFICATE_BYTES {
        return Err(reject("encoded search certificate exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode(bytes: &[u8]) -> Result<SearchCertificate, SearchError> {
    if bytes.len() > MAX_SEARCH_CERTIFICATE_BYTES {
        return Err(reject("search certificate exceeds byte limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("search certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(
            "search certificate must be canonical LF text without NUL",
        ));
    }
    let mut lines = text.lines();
    fn take(lines: &mut std::str::Lines<'_>, key: &str) -> Result<String, SearchError> {
        let line = lines
            .next()
            .ok_or_else(|| reject(format!("missing {key}")))?;
        line.strip_prefix(&format!("{key}="))
            .map(str::to_string)
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    fn number<T: std::str::FromStr>(value: String, key: &str) -> Result<T, SearchError> {
        value.parse().map_err(|_| reject(format!("invalid {key}")))
    }
    let version: u32 = number(take(&mut lines, "search_certificate_version")?, "version")?;
    if !matches!(
        version,
        SEARCH_CERTIFICATE_V1_VERSION
            | SEARCH_CERTIFICATE_V2_VERSION
            | SEARCH_CERTIFICATE_V3_VERSION
            | SEARCH_CERTIFICATE_V4_VERSION
            | SEARCH_CERTIFICATE_VERSION
    ) {
        return Err(reject("unsupported search certificate version"));
    }
    let source_sha256 = take(&mut lines, "source_sha256")?;
    if !valid_digest(&source_sha256) {
        return Err(reject("search source digest is not canonical"));
    }
    let query_horizon = number(take(&mut lines, "query_horizon")?, "query horizon")?;
    if query_horizon > MAX_SEARCH_HORIZON {
        return Err(reject("search query horizon exceeds limit"));
    }
    let bad_property = number(take(&mut lines, "bad_property")?, "bad property")?;
    let (input, inputs) = if uses_packed_valuations(version) {
        let input_count: usize = number(take(&mut lines, "input_count")?, "input count")?;
        let minimum_inputs = if version == SEARCH_CERTIFICATE_V3_VERSION {
            2
        } else {
            1
        };
        if !(minimum_inputs..=MAX_SEARCH_INPUTS).contains(&input_count) {
            return Err(reject("search input count is outside limits"));
        }
        let text = take(&mut lines, "inputs")?;
        let values = text
            .split(',')
            .map(|value| number(value.to_string(), "input identifier"))
            .collect::<Result<Vec<NodeId>, _>>()?;
        if values.len() != input_count || !values.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(reject("search inputs are not canonical"));
        }
        (values[0], values)
    } else {
        (number(take(&mut lines, "input")?, "input")?, Vec::new())
    };
    let input_widths = if version == SEARCH_CERTIFICATE_VERSION {
        let values = take(&mut lines, "input_widths")?
            .split(',')
            .map(|value| number(value.to_string(), "input width"))
            .collect::<Result<Vec<u32>, _>>()?;
        if values.len() != inputs.len()
            || values.iter().all(|width| *width == 1)
            || values
                .iter()
                .any(|width| *width == 0 || *width as usize > MAX_SEARCH_INPUT_BITS)
            || total_input_bits(&values) > MAX_SEARCH_INPUT_BITS
        {
            return Err(reject("search input widths are outside limits"));
        }
        let declared_total: usize =
            number(take(&mut lines, "total_input_bits")?, "total input bits")?;
        if declared_total != total_input_bits(&values) {
            return Err(reject("search total input width is noncanonical"));
        }
        values
    } else {
        Vec::new()
    };
    let constraints = if uses_constraints(version) {
        let constraint_count: usize =
            number(take(&mut lines, "constraint_count")?, "constraint count")?;
        if (constraint_count == 0 && version == SEARCH_CERTIFICATE_V4_VERSION)
            || constraint_count > btor2::MAX_BTOR2_NODES
        {
            return Err(reject("search constraint count is outside limits"));
        }
        let encoded = take(&mut lines, "constraints")?;
        let values = if encoded.is_empty() {
            Vec::new()
        } else {
            encoded
                .split(',')
                .map(|value| number(value.to_string(), "constraint identifier"))
                .collect::<Result<Vec<NodeId>, _>>()?
        };
        if values.len() != constraint_count || !values.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(reject("search constraints are not canonical"));
        }
        values
    } else {
        Vec::new()
    };
    let valuation_widths = if version == SEARCH_CERTIFICATE_VERSION {
        input_widths.clone()
    } else {
        vec![1; inputs.len()]
    };
    let result = match take(&mut lines, "result")?.as_str() {
        "SAFE" => SearchResult::Safe,
        "UNSAFE" => SearchResult::Unsafe,
        _ => return Err(reject("search result must be SAFE or UNSAFE")),
    };
    let bad_frame = match take(&mut lines, "bad_frame")?.as_str() {
        "none" => None,
        value => Some(number(value.to_string(), "bad frame")?),
    };
    let (witness_inputs, witness_valuations, terminal_valuation) =
        if uses_packed_valuations(version) {
            let text = take(&mut lines, "witness_valuations")?;
            let values = if text.is_empty() {
                Vec::new()
            } else {
                text.split(',')
                    .map(|value| number(value.to_string(), "witness valuation"))
                    .collect::<Result<Vec<u16>, _>>()?
            };
            if values.len() > MAX_SEARCH_HORIZON as usize
                || values
                    .iter()
                    .any(|value| !valuation_is_canonical(&valuation_widths, *value))
            {
                return Err(reject("search witness valuations are outside limits"));
            }
            let terminal = match take(&mut lines, "terminal_valuation")?.as_str() {
                "none" => None,
                value => Some(number(value.to_string(), "terminal valuation")?),
            };
            if terminal.is_some_and(|value| !valuation_is_canonical(&valuation_widths, value)) {
                return Err(reject("search terminal valuation is noncanonical"));
            }
            (Vec::new(), values, terminal)
        } else {
            let witness_text = take(&mut lines, "witness")?;
            if witness_text.len() > MAX_SEARCH_HORIZON as usize
                || !witness_text.bytes().all(|byte| matches!(byte, b'0' | b'1'))
            {
                return Err(reject("search witness is not a bit string"));
            }
            (
                witness_text.bytes().map(|byte| byte == b'1').collect(),
                Vec::new(),
                None,
            )
        };
    let terminal_input = if version == SEARCH_CERTIFICATE_V2_VERSION {
        match take(&mut lines, "terminal_input")?.as_str() {
            "none" => None,
            "0" => Some(false),
            "1" => Some(true),
            _ => return Err(reject("search terminal input must be none, 0, or 1")),
        }
    } else {
        None
    };
    let layer_count: usize = number(take(&mut lines, "layer_count")?, "layer count")?;
    if layer_count > MAX_SEARCH_HORIZON as usize + 1 {
        return Err(reject("search layer count exceeds limit"));
    }
    let mut layers = Vec::with_capacity(layer_count);
    let mut total_states = 0usize;
    for frame in 0..layer_count {
        let count: usize = number(
            take(&mut lines, &format!("layer_{frame}_count"))?,
            "layer state count",
        )?;
        if (count == 0 && !uses_constraints(version)) || count > MAX_STATES_PER_LAYER {
            return Err(reject("search layer state count is outside limits"));
        }
        total_states = total_states
            .checked_add(count)
            .filter(|total| *total <= MAX_TOTAL_STATES)
            .ok_or_else(|| reject("search certificate exceeds total state limit"))?;
        let mut layer = Vec::with_capacity(count);
        for index in 0..count {
            let encoded = take(&mut lines, &format!("layer_{frame}_state_{index}"))?;
            if encoded.is_empty() {
                return Err(reject("search state is empty"));
            }
            let mut values = Vec::new();
            for field in encoded.split(',') {
                let (id, value) = field
                    .split_once(':')
                    .ok_or_else(|| reject("invalid search state field"))?;
                values.push((
                    number(id.to_string(), "state identifier")?,
                    number(value.to_string(), "state value")?,
                ));
                if values.len() > btor2::MAX_BTOR2_NODES {
                    return Err(reject("search state vector exceeds node limit"));
                }
            }
            if !values.windows(2).all(|pair| pair[0].0 < pair[1].0) {
                return Err(reject("search state identifiers are not ordered"));
            }
            layer.push(SearchState(values));
        }
        layers.push(layer);
    }
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "search certificate is incomplete or has trailing fields",
        ));
    }
    Ok(SearchCertificate {
        certificate_version: version,
        source_sha256,
        query_horizon,
        bad_property,
        input,
        inputs,
        input_widths,
        constraints,
        result,
        bad_frame,
        witness_inputs,
        terminal_input,
        witness_valuations,
        terminal_valuation,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATCHDOG: &[u8] = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
    const SATURATING: &[u8] =
        include_bytes!("../examples/btor2/saturating-timer-rejected-v1.btor2");
    const INPUT_DEPENDENT_BAD: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 3\n3 input 1 reset\n4 zero 2\n5 state 2 count\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n11 ite 2 3 4 5\n12 constd 2 2\n13 eq 1 11 12\n14 bad 13 reset_guarded\n";
    const TWO_INPUTS: &[u8] = b"1 sort bitvec 1\n2 input 1 a\n3 input 1 b\n4 state 1 state\n5 zero 1\n6 init 1 4 5\n7 xor 1 2 3\n8 next 1 4 7\n9 and 1 4 2\n10 bad 9 state_and_a\n";
    const CONSTRAINED_TWO_INPUTS: &[u8] = b"1 sort bitvec 1\n2 input 1 a\n3 input 1 b\n4 state 1 state\n5 zero 1\n6 init 1 4 5\n7 xor 1 2 3\n8 next 1 4 7\n9 and 1 2 3\n10 not 1 9\n11 constraint 10\n12 bad 4 reached_one\n";
    const CONSTRAINED_UNREACHABLE_BAD: &[u8] = b"1 sort bitvec 1\n2 input 1 a\n3 input 1 b\n4 state 1 state\n5 zero 1\n6 init 1 4 5\n7 xor 1 2 3\n8 next 1 4 7\n9 and 1 2 3\n10 not 1 9\n11 constraint 10\n12 bad 9 forbidden_pair\n";
    const CONSTRAINED_FORBIDDEN_TRANSITION: &[u8] = b"1 sort bitvec 1\n2 input 1 advance\n3 state 1 state\n4 zero 1\n5 init 1 3 4\n6 next 1 3 2\n7 not 1 2\n8 constraint 7\n9 bad 3 reached_one\n";
    const CONSTRAINT_DEAD_END: &[u8] = b"1 sort bitvec 1\n2 input 1 advance\n3 state 1 state\n4 zero 1\n5 init 1 3 4\n6 one 1\n7 next 1 3 6\n8 not 1 3\n9 eq 1 2 2\n10 and 1 8 9\n11 constraint 10\n12 bad 4 never_bad\n";
    const ONE_INPUT_CONSTRAINT: &[u8] = b"1 sort bitvec 1\n2 input 1 enabled\n3 state 1 state\n4 zero 1\n5 init 1 3 4\n6 not 1 3\n7 next 1 3 6\n8 constraint 2\n9 bad 3 toggled\n";
    const MIXED_WORD_INPUTS: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 1 enable\n4 input 2 command\n5 state 2 state\n6 zero 2\n7 init 2 5 6\n8 next 2 5 4\n9 constd 2 2\n10 eq 1 5 9\n11 and 1 10 3\n12 bad 11 state_two_and_enable\n";
    const MULTI_WORD_INPUTS: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 left\n4 input 2 right\n5 state 2 state\n6 zero 2\n7 init 2 5 6\n8 xor 2 3 4\n9 next 2 5 8\n10 constd 2 3\n11 eq 1 5 10\n12 bad 11 reached_three\n";
    const WORD_INPUT_DEPENDENT_BAD: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 next 2 4 3\n8 constd 2 3\n9 eq 1 3 8\n10 bad 9 command_three\n";
    const WORD_CONSTRAINT_DEAD_END: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 one 2\n8 next 2 4 7\n9 eq 1 4 5\n10 eq 1 3 3\n11 and 1 9 10\n12 constraint 11\n13 not 1 10\n14 bad 13 never_bad\n";

    #[test]
    fn proves_both_bounded_answers_and_round_trips() {
        let safe = produce(WATCHDOG, 13, 2).unwrap();
        assert_eq!(safe.certificate_version, SEARCH_CERTIFICATE_V1_VERSION);
        assert_eq!(safe.terminal_input, None);
        assert_eq!(safe.result, SearchResult::Safe);
        let safe = decode(encode(&safe).unwrap().as_bytes()).unwrap();
        let summary = verify(WATCHDOG, &safe).unwrap();
        assert_eq!(summary.result, SearchResult::Safe);
        assert_eq!(summary.reachable_states, 6);

        let unsafe_certificate = produce(WATCHDOG, 13, 3).unwrap();
        assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
        assert_eq!(unsafe_certificate.bad_frame, Some(3));
        assert_eq!(unsafe_certificate.witness_inputs, vec![false; 3]);
        let unsafe_certificate = decode(encode(&unsafe_certificate).unwrap().as_bytes()).unwrap();
        assert_eq!(
            verify(WATCHDOG, &unsafe_certificate).unwrap().result,
            SearchResult::Unsafe
        );
    }

    #[test]
    fn v2_proves_input_dependent_bad_properties_without_reinterpreting_v1() {
        let safe = produce(INPUT_DEPENDENT_BAD, 14, 1).unwrap();
        assert_eq!(safe.certificate_version, SEARCH_CERTIFICATE_V2_VERSION);
        assert_eq!(safe.result, SearchResult::Safe);
        assert_eq!(safe.terminal_input, None);
        let safe_text = encode(&safe).unwrap();
        assert!(safe_text.starts_with("search_certificate_version=2\n"));
        assert!(safe_text.contains("terminal_input=none\n"));
        assert!(verify(INPUT_DEPENDENT_BAD, &decode(safe_text.as_bytes()).unwrap()).is_ok());

        let unsafe_certificate = produce(INPUT_DEPENDENT_BAD, 14, 2).unwrap();
        assert_eq!(
            unsafe_certificate.certificate_version,
            SEARCH_CERTIFICATE_V2_VERSION
        );
        assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
        assert_eq!(unsafe_certificate.bad_frame, Some(2));
        assert_eq!(unsafe_certificate.witness_inputs, vec![false; 2]);
        assert_eq!(unsafe_certificate.terminal_input, Some(false));
        let encoded = encode(&unsafe_certificate).unwrap();
        let decoded = decode(encoded.as_bytes()).unwrap();
        assert_eq!(
            verify(INPUT_DEPENDENT_BAD, &decoded).unwrap().bad_frame,
            Some(2)
        );

        let mut wrong_terminal = decoded;
        wrong_terminal.terminal_input = Some(true);
        assert!(verify(INPUT_DEPENDENT_BAD, &wrong_terminal).is_err());

        let mut missing_terminal = unsafe_certificate.clone();
        missing_terminal.terminal_input = None;
        assert!(verify(INPUT_DEPENDENT_BAD, &missing_terminal).is_err());
        let mut downgraded = unsafe_certificate.clone();
        downgraded.certificate_version = SEARCH_CERTIFICATE_V1_VERSION;
        downgraded.terminal_input = None;
        assert!(verify(INPUT_DEPENDENT_BAD, &downgraded).is_err());
        assert!(decode(encoded.replace("terminal_input=0\n", "").as_bytes()).is_err());
        assert!(
            decode(
                encoded
                    .replace(
                        "search_certificate_version=2",
                        "search_certificate_version=1"
                    )
                    .as_bytes()
            )
            .is_err()
        );

        let mut invalid_safe = safe;
        invalid_safe.terminal_input = Some(false);
        assert!(verify(INPUT_DEPENDENT_BAD, &invalid_safe).is_err());

        let v1 = produce(WATCHDOG, 13, 2).unwrap();
        let v1_text = encode(&v1).unwrap();
        assert!(v1_text.starts_with("search_certificate_version=1\n"));
        assert!(!v1_text.contains("terminal_input="));
        assert!(verify(WATCHDOG, &decode(v1_text.as_bytes()).unwrap()).is_ok());
        let mut forced_v2 = v1;
        forced_v2.certificate_version = SEARCH_CERTIFICATE_V2_VERSION;
        assert!(verify(WATCHDOG, &forced_v2).is_err());
    }

    #[test]
    fn v3_preserves_complete_multi_input_transition_and_terminal_valuations() {
        let safe = produce(TWO_INPUTS, 10, 0).unwrap();
        assert_eq!(safe.certificate_version, SEARCH_CERTIFICATE_V3_VERSION);
        assert_eq!(safe.inputs, vec![2, 3]);
        assert_eq!(safe.result, SearchResult::Safe);
        let safe_text = encode(&safe).unwrap();
        assert!(safe_text.starts_with("search_certificate_version=3\n"));
        assert!(safe_text.contains("input_count=2\ninputs=2,3\n"));
        assert!(verify(TWO_INPUTS, &decode(safe_text.as_bytes()).unwrap()).is_ok());

        let unsafe_certificate = produce(TWO_INPUTS, 10, 1).unwrap();
        assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
        assert_eq!(unsafe_certificate.bad_frame, Some(1));
        assert_eq!(unsafe_certificate.witness_valuations, vec![1]);
        assert_eq!(unsafe_certificate.terminal_valuation, Some(1));
        let encoded = encode(&unsafe_certificate).unwrap();
        let decoded = decode(encoded.as_bytes()).unwrap();
        assert!(verify(TWO_INPUTS, &decoded).is_ok());

        let mut reordered = decoded.clone();
        reordered.inputs.swap(0, 1);
        assert!(verify(TWO_INPUTS, &reordered).is_err());
        let mut high_transition_bit = decoded.clone();
        high_transition_bit.witness_valuations[0] = 4;
        assert!(verify(TWO_INPUTS, &high_transition_bit).is_err());
        let mut wrong_terminal = decoded.clone();
        wrong_terminal.terminal_valuation = Some(0);
        assert!(verify(TWO_INPUTS, &wrong_terminal).is_err());
        let mut downgraded = decoded;
        downgraded.certificate_version = SEARCH_CERTIFICATE_V2_VERSION;
        downgraded.inputs.clear();
        downgraded.witness_inputs = vec![true];
        downgraded.witness_valuations.clear();
        downgraded.terminal_input = Some(true);
        downgraded.terminal_valuation = None;
        assert!(verify(TWO_INPUTS, &downgraded).is_err());
        assert!(decode(encoded.replace("inputs=2,3\n", "inputs=3,2\n").as_bytes()).is_err());
        assert!(decode(encoded.replace("terminal_valuation=1\n", "").as_bytes()).is_err());
    }

    #[test]
    fn v4_preserves_constraints_admissible_witnesses_and_dead_ends() {
        let unsafe_certificate = produce(CONSTRAINED_TWO_INPUTS, 12, 1).unwrap();
        assert_eq!(
            unsafe_certificate.certificate_version,
            SEARCH_CERTIFICATE_V4_VERSION
        );
        assert_eq!(unsafe_certificate.inputs, vec![2, 3]);
        assert_eq!(unsafe_certificate.constraints, vec![11]);
        assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
        assert_eq!(unsafe_certificate.bad_frame, Some(1));
        assert_eq!(unsafe_certificate.witness_valuations, vec![1]);
        assert_eq!(unsafe_certificate.terminal_valuation, Some(0));
        let encoded = encode(&unsafe_certificate).unwrap();
        assert!(encoded.contains("constraint_count=1\nconstraints=11\n"));
        let decoded = decode(encoded.as_bytes()).unwrap();
        assert!(verify(CONSTRAINED_TWO_INPUTS, &decoded).is_ok());

        let mut inadmissible_transition = decoded.clone();
        inadmissible_transition.witness_valuations[0] = 3;
        assert!(verify(CONSTRAINED_TWO_INPUTS, &inadmissible_transition).is_err());
        let mut omitted = decoded.clone();
        omitted.constraints.clear();
        assert!(verify(CONSTRAINED_TWO_INPUTS, &omitted).is_err());
        let mut downgraded = decoded;
        downgraded.certificate_version = SEARCH_CERTIFICATE_V3_VERSION;
        downgraded.constraints.clear();
        assert!(verify(CONSTRAINED_TWO_INPUTS, &downgraded).is_err());
        assert!(
            decode(
                encoded
                    .replace("constraints=11\n", "constraints=10\n")
                    .as_bytes()
            )
            .is_ok()
        );
        let rebound = decode(
            encoded
                .replace("constraints=11\n", "constraints=10\n")
                .as_bytes(),
        )
        .unwrap();
        assert!(verify(CONSTRAINED_TWO_INPUTS, &rebound).is_err());

        let mut forbidden_terminal = produce(CONSTRAINED_UNREACHABLE_BAD, 12, 0).unwrap();
        forbidden_terminal.result = SearchResult::Unsafe;
        forbidden_terminal.bad_frame = Some(0);
        forbidden_terminal.terminal_valuation = Some(3);
        forbidden_terminal.layers.clear();
        assert!(
            verify(CONSTRAINED_UNREACHABLE_BAD, &forbidden_terminal)
                .unwrap_err()
                .0
                .contains("inadmissible terminal valuation")
        );

        let mut forbidden_transition = produce(CONSTRAINED_FORBIDDEN_TRANSITION, 9, 1).unwrap();
        forbidden_transition.result = SearchResult::Unsafe;
        forbidden_transition.bad_frame = Some(1);
        forbidden_transition.witness_valuations = vec![1];
        forbidden_transition.terminal_valuation = Some(0);
        forbidden_transition.layers.clear();
        assert!(
            verify(CONSTRAINED_FORBIDDEN_TRANSITION, &forbidden_transition)
                .unwrap_err()
                .0
                .contains("inadmissible transition valuation")
        );

        let dead_end = produce(CONSTRAINT_DEAD_END, 12, 3).unwrap();
        assert_eq!(dead_end.result, SearchResult::Safe);
        assert_eq!(dead_end.layers.len(), 4);
        assert_eq!(dead_end.layers[0].len(), 1);
        assert_eq!(dead_end.layers[1].len(), 1);
        assert!(dead_end.layers[2].is_empty());
        assert!(dead_end.layers[3].is_empty());
        let dead_end = decode(encode(&dead_end).unwrap().as_bytes()).unwrap();
        assert!(verify(CONSTRAINT_DEAD_END, &dead_end).is_ok());
        let mut false_successor = dead_end;
        false_successor.layers[2] = false_successor.layers[1].clone();
        assert!(verify(CONSTRAINT_DEAD_END, &false_successor).is_err());
    }

    #[test]
    fn v4_agrees_with_a_separate_exhaustive_trace_oracle() {
        fn oracle(
            source: &[u8],
            bad_property: NodeId,
            horizon: u32,
        ) -> (SearchResult, Option<u32>) {
            let model = btor2::parse_bytes(source).unwrap();
            let bad_expression = model
                .bad_properties()
                .iter()
                .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))
                .unwrap();
            let inputs = model.inputs().to_vec();
            let valuation_count = 1usize << inputs.len();
            let mut layer = BTreeSet::from([model.initial_state().unwrap()]);
            for frame in 0..=horizon {
                let mut successors = BTreeSet::new();
                for state in &layer {
                    for valuation in 0..valuation_count {
                        let values = inputs
                            .iter()
                            .enumerate()
                            .map(|(index, id)| (*id, ((valuation >> index) & 1) as u64))
                            .collect::<WordValues>();
                        let admissible = model.constraints().iter().all(|(_, expression)| {
                            model.evaluate(*expression, state, &values).unwrap() != 0
                        });
                        if !admissible {
                            continue;
                        }
                        if model.evaluate(bad_expression, state, &values).unwrap() != 0 {
                            return (SearchResult::Unsafe, Some(frame));
                        }
                        if frame < horizon {
                            successors.insert(model.step(state, &values).unwrap());
                        }
                    }
                }
                layer = successors;
            }
            (SearchResult::Safe, None)
        }

        for (source, bad, horizon) in [
            (CONSTRAINED_TWO_INPUTS, 12, 0),
            (CONSTRAINED_TWO_INPUTS, 12, 1),
            (CONSTRAINED_UNREACHABLE_BAD, 12, 4),
            (CONSTRAINT_DEAD_END, 12, 3),
            (ONE_INPUT_CONSTRAINT, 9, 0),
            (ONE_INPUT_CONSTRAINT, 9, 1),
        ] {
            let expected = oracle(source, bad, horizon);
            let certificate = produce(source, bad, horizon).unwrap();
            assert_eq!(
                (certificate.result, certificate.bad_frame),
                expected,
                "oracle disagreement at property {bad}, horizon {horizon}"
            );
            assert_eq!(
                certificate.certificate_version,
                SEARCH_CERTIFICATE_V4_VERSION
            );
            assert!(
                verify(
                    source,
                    &decode(encode(&certificate).unwrap().as_bytes()).unwrap()
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn v5_preserves_word_widths_packing_constraints_and_both_answers() {
        fn oracle(
            source: &[u8],
            bad_property: NodeId,
            horizon: u32,
        ) -> (SearchResult, Option<u32>) {
            let model = btor2::parse_bytes(source).unwrap();
            let bad_expression = model
                .bad_properties()
                .iter()
                .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))
                .unwrap();
            let inputs = model.inputs().to_vec();
            let widths = inputs
                .iter()
                .map(|input| model.nodes()[input].width)
                .collect::<Vec<_>>();
            let valuation_count = 1usize << widths.iter().sum::<u32>();
            let mut layer = BTreeSet::from([model.initial_state().unwrap()]);
            for frame in 0..=horizon {
                let mut successors = BTreeSet::new();
                for state in &layer {
                    for packed in 0..valuation_count {
                        let mut offset = 0usize;
                        let values = inputs
                            .iter()
                            .zip(&widths)
                            .map(|(input, width)| {
                                let mask = (1usize << *width) - 1;
                                let value = (packed >> offset) & mask;
                                offset += *width as usize;
                                (*input, value as u64)
                            })
                            .collect::<WordValues>();
                        if !model.constraints().iter().all(|(_, expression)| {
                            model.evaluate(*expression, state, &values).unwrap() != 0
                        }) {
                            continue;
                        }
                        if model.evaluate(bad_expression, state, &values).unwrap() != 0 {
                            return (SearchResult::Unsafe, Some(frame));
                        }
                        if frame < horizon {
                            successors.insert(model.step(state, &values).unwrap());
                        }
                    }
                }
                layer = successors;
            }
            (SearchResult::Safe, None)
        }

        fn word_model(width: u32, constrained: bool) -> (Vec<u8>, NodeId) {
            let maximum = (1u64 << width) - 1;
            let mut text = format!(
                "1 sort bitvec 1\n2 sort bitvec {width}\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 next 2 4 3\n8 constd 2 {maximum}\n9 eq 1 4 8\n"
            );
            let bad = if constrained {
                text.push_str(&format!(
                    "10 constd 2 {}\n11 ulte 1 3 10\n12 constraint 11\n13 bad 9 maximum_reached\n",
                    maximum - 1
                ));
                13
            } else {
                text.push_str("10 bad 9 maximum_reached\n");
                10
            };
            (text.into_bytes(), bad)
        }

        for width in 2..=MAX_SEARCH_INPUT_BITS as u32 {
            let (source, bad) = word_model(width, false);
            let safe = produce(&source, bad, 0).unwrap();
            assert_eq!((safe.result, safe.bad_frame), oracle(&source, bad, 0));
            assert_eq!(safe.certificate_version, SEARCH_CERTIFICATE_VERSION);
            assert_eq!(safe.input_widths, vec![width]);
            assert_eq!(safe.result, SearchResult::Safe);
            let unsafe_certificate = produce(&source, bad, 1).unwrap();
            assert_eq!(
                (unsafe_certificate.result, unsafe_certificate.bad_frame),
                oracle(&source, bad, 1)
            );
            assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
            assert_eq!(
                unsafe_certificate.witness_valuations,
                vec![((1u16 << width) - 1)]
            );
            assert_eq!(unsafe_certificate.terminal_valuation, Some(0));
            assert!(
                verify(
                    &source,
                    &decode(encode(&unsafe_certificate).unwrap().as_bytes()).unwrap()
                )
                .is_ok()
            );

            let (constrained, constrained_bad) = word_model(width, true);
            let constrained_safe = produce(&constrained, constrained_bad, 2).unwrap();
            assert_eq!(
                (constrained_safe.result, constrained_safe.bad_frame),
                oracle(&constrained, constrained_bad, 2)
            );
            assert_eq!(constrained_safe.result, SearchResult::Safe);
            assert_eq!(constrained_safe.constraints, vec![12]);
            let mut omitted_constraint = constrained_safe.clone();
            omitted_constraint.constraints.clear();
            assert!(verify(&constrained, &omitted_constraint).is_err());
            assert!(
                verify(
                    &constrained,
                    &decode(encode(&constrained_safe).unwrap().as_bytes()).unwrap()
                )
                .is_ok()
            );
        }

        let mixed = produce(MIXED_WORD_INPUTS, 12, 1).unwrap();
        assert_eq!(
            (mixed.result, mixed.bad_frame),
            oracle(MIXED_WORD_INPUTS, 12, 1)
        );
        assert_eq!(mixed.inputs, vec![3, 4]);
        assert_eq!(mixed.input_widths, vec![1, 2]);
        assert_eq!(mixed.witness_valuations, vec![4]);
        assert_eq!(mixed.terminal_valuation, Some(1));
        let encoded = encode(&mixed).unwrap();
        assert!(encoded.contains("input_widths=1,2\ntotal_input_bits=3\n"));
        assert!(encoded.contains("constraint_count=0\nconstraints=\n"));
        let decoded = decode(encoded.as_bytes()).unwrap();
        assert!(verify(MIXED_WORD_INPUTS, &decoded).is_ok());
        for end in 0..encoded.len() {
            assert!(decode(&encoded.as_bytes()[..end]).is_err());
        }
        let mut width_drift = decoded.clone();
        width_drift.input_widths = vec![2, 1];
        assert!(verify(MIXED_WORD_INPUTS, &width_drift).is_err());
        let mut high_bit = decoded.clone();
        high_bit.terminal_valuation = Some(8);
        assert!(verify(MIXED_WORD_INPUTS, &high_bit).is_err());
        let mut downgraded = decoded;
        downgraded.certificate_version = SEARCH_CERTIFICATE_V4_VERSION;
        downgraded.input_widths.clear();
        assert!(verify(MIXED_WORD_INPUTS, &downgraded).is_err());
        assert!(
            decode(
                encoded
                    .replace("total_input_bits=3", "total_input_bits=2")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            decode(
                encoded
                    .replace("input_widths=1,2", "input_widths=2,1")
                    .as_bytes()
            )
            .is_ok()
        );
        let rebound = decode(
            encoded
                .replace("input_widths=1,2", "input_widths=2,1")
                .as_bytes(),
        )
        .unwrap();
        assert!(verify(MIXED_WORD_INPUTS, &rebound).is_err());

        let multi_word = produce(MULTI_WORD_INPUTS, 12, 1).unwrap();
        assert_eq!(multi_word.input_widths, vec![2, 2]);
        assert_eq!(multi_word.witness_valuations, vec![3]);
        assert_eq!(
            (multi_word.result, multi_word.bad_frame),
            oracle(MULTI_WORD_INPUTS, 12, 1)
        );
        assert!(verify(MULTI_WORD_INPUTS, &multi_word).is_ok());

        let input_bad = produce(WORD_INPUT_DEPENDENT_BAD, 10, 0).unwrap();
        assert_eq!(input_bad.result, SearchResult::Unsafe);
        assert_eq!(input_bad.witness_valuations, Vec::<u16>::new());
        assert_eq!(input_bad.terminal_valuation, Some(3));
        assert_eq!(
            (input_bad.result, input_bad.bad_frame),
            oracle(WORD_INPUT_DEPENDENT_BAD, 10, 0)
        );

        let dead_end = produce(WORD_CONSTRAINT_DEAD_END, 14, 3).unwrap();
        assert_eq!(dead_end.result, SearchResult::Safe);
        assert_eq!(
            dead_end.layers.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![1, 1, 0, 0]
        );
        assert_eq!(
            (dead_end.result, dead_end.bad_frame),
            oracle(WORD_CONSTRAINT_DEAD_END, 14, 3)
        );
        assert!(verify(WORD_CONSTRAINT_DEAD_END, &dead_end).is_ok());

        fn wide_state_model(state_count: usize) -> Vec<u8> {
            let mut text =
                "1 sort bitvec 1\n2 sort bitvec 8\n3 input 2 command\n4 zero 2\n5 zero 1\n"
                    .to_string();
            let first_state = 6usize;
            let first_init = first_state + state_count;
            let first_next = first_init + state_count;
            for index in 0..state_count {
                text.push_str(&format!("{} state 2 state_{index}\n", first_state + index));
            }
            for index in 0..state_count {
                text.push_str(&format!(
                    "{} init 2 {} 4\n",
                    first_init + index,
                    first_state + index
                ));
            }
            for index in 0..state_count {
                text.push_str(&format!(
                    "{} next 2 {} 3\n",
                    first_next + index,
                    first_state + index
                ));
            }
            let bad = first_next + state_count;
            text.push_str(&format!("{bad} bad 5 never_bad\n"));
            text.into_bytes()
        }
        let resource_source = wide_state_model(10);
        let resource_bad = 6 + 30;
        assert!(
            produce(&resource_source, resource_bad as NodeId, 3)
                .unwrap_err()
                .0
                .contains("node-step limit")
        );

        let (too_wide, too_wide_bad) = word_model(9, false);
        assert!(
            produce(&too_wide, too_wide_bad, 1)
                .unwrap_err()
                .0
                .contains("eight total semantic input bits")
        );
    }

    #[test]
    fn v3_agrees_with_closed_form_bruteforce_across_two_to_eight_inputs() {
        fn parity_model(input_count: usize, input_dependent_bad: bool) -> (Vec<u8>, NodeId) {
            let mut text = "1 sort bitvec 1\n".to_string();
            for index in 0..input_count {
                text.push_str(&format!("{} input 1 input_{index}\n", index + 2));
            }
            let state = input_count + 2;
            let zero = state + 1;
            let init = zero + 1;
            text.push_str(&format!("{state} state 1 state\n"));
            text.push_str(&format!("{zero} zero 1\n"));
            text.push_str(&format!("{init} init 1 {state} {zero}\n"));
            let mut expression = 2;
            let mut next_id = init + 1;
            for input in 3..input_count + 2 {
                text.push_str(&format!("{next_id} xor 1 {expression} {input}\n"));
                expression = next_id;
                next_id += 1;
            }
            text.push_str(&format!("{next_id} next 1 {state} {expression}\n"));
            next_id += 1;
            let bad_expression = if input_dependent_bad {
                text.push_str(&format!("{next_id} and 1 {state} 2\n"));
                next_id
            } else {
                state
            };
            let bad = next_id + usize::from(input_dependent_bad);
            text.push_str(&format!("{bad} bad {bad_expression} parity_reachable\n"));
            (text.into_bytes(), bad as NodeId)
        }

        for input_count in 2..=MAX_SEARCH_INPUTS {
            let (source, bad) = parity_model(input_count, true);
            let safe = produce(&source, bad, 0).unwrap();
            assert_eq!(safe.result, SearchResult::Safe);
            assert_eq!(safe.inputs.len(), input_count);
            assert!(verify(&source, &decode(encode(&safe).unwrap().as_bytes()).unwrap()).is_ok());

            let unsafe_certificate = produce(&source, bad, 1).unwrap();
            assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
            assert_eq!(unsafe_certificate.bad_frame, Some(1));
            assert_eq!(unsafe_certificate.witness_valuations, vec![1]);
            assert_eq!(unsafe_certificate.terminal_valuation, Some(1));
            assert!(verify(&source, &unsafe_certificate).is_ok());
        }

        let (state_only, state_only_bad) = parity_model(2, false);
        let state_only_unsafe = produce(&state_only, state_only_bad, 1).unwrap();
        assert_eq!(state_only_unsafe.result, SearchResult::Unsafe);
        assert_eq!(state_only_unsafe.terminal_valuation, Some(0));
        assert!(verify(&state_only, &state_only_unsafe).is_ok());

        let (too_many, too_many_bad) = parity_model(MAX_SEARCH_INPUTS + 1, true);
        assert!(
            produce(&too_many, too_many_bad, 1)
                .unwrap_err()
                .0
                .contains("between one and eight")
        );
    }

    #[test]
    fn searches_the_non_affine_saturating_model_exactly() {
        assert_eq!(
            produce(SATURATING, 15, 254).unwrap().result,
            SearchResult::Safe
        );
        let unsafe_certificate = produce(SATURATING, 15, 255).unwrap();
        assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
        assert_eq!(unsafe_certificate.bad_frame, Some(255));
        assert!(verify(SATURATING, &unsafe_certificate).is_ok());
    }

    #[test]
    fn rejects_tampering_and_hostile_shapes() {
        let mut safe = produce(WATCHDOG, 13, 2).unwrap();
        safe.layers[1].pop();
        assert!(verify(WATCHDOG, &safe).is_err());

        let mut unsafe_certificate = produce(WATCHDOG, 13, 3).unwrap();
        unsafe_certificate.witness_inputs[0] = true;
        assert!(verify(WATCHDOG, &unsafe_certificate).is_err());

        assert!(produce(WATCHDOG, 13, MAX_SEARCH_HORIZON + 1).is_err());
        assert!(decode(&vec![b'x'; MAX_SEARCH_CERTIFICATE_BYTES + 1]).is_err());
        assert!(decode(b"search_certificate_version=1\r\n").is_err());

        let mut many_states =
            "1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 control\n4 zero 2\n".to_string();
        for id in 5..205 {
            many_states.push_str(&format!("{id} state 2 s{id}\n"));
        }
        let mut id = 205;
        for state in 5..205 {
            many_states.push_str(&format!("{id} init 2 {state} 4\n"));
            id += 1;
            let next = if state == 5 {
                many_states.push_str(&format!("{id} ite 2 3 4 {state}\n"));
                id += 1;
                id - 1
            } else {
                state
            };
            many_states.push_str(&format!("{id} next 2 {state} {next}\n"));
            id += 1;
        }
        many_states.push_str(&format!("{id} neq 1 5 4\n"));
        id += 1;
        many_states.push_str(&format!("{id} bad {} never\n", id - 1));
        assert!(
            produce(many_states.as_bytes(), id, MAX_SEARCH_HORIZON)
                .unwrap_err()
                .0
                .contains("node-step")
        );
    }
}
