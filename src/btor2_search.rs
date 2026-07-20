//! Exact bounded reachability certificates for the strict BTOR2 semantic core.

use crate::btor2::{self, Btor2Model, NodeId, NodeKind, WordValues};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const SEARCH_CERTIFICATE_VERSION: u32 = 1;
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
    pub source_sha256: String,
    pub query_horizon: u32,
    pub bad_property: NodeId,
    pub input: NodeId,
    pub result: SearchResult,
    pub bad_frame: Option<u32>,
    pub witness_inputs: Vec<bool>,
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
) -> Result<(NodeId, NodeId), SearchError> {
    if model.inputs().len() != 1 || model.nodes()[&model.inputs()[0]].width != 1 {
        return Err(reject("bounded search requires exactly one one-bit input"));
    }
    if !model.constraints().is_empty() {
        return Err(reject("bounded search v1 does not admit constraints"));
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
            NodeKind::Ite(condition, then_value, else_value) => {
                depends_on_input(model, condition, memo)
                    || depends_on_input(model, then_value, memo)
                    || depends_on_input(model, else_value, memo)
            }
        };
        memo.insert(id, result);
        result
    }
    if depends_on_input(model, expression, &mut BTreeMap::new()) {
        return Err(reject("bounded search requires a state-only bad property"));
    }
    Ok((model.inputs()[0], expression))
}

fn bad_active(
    model: &Btor2Model,
    state: &SearchState,
    input: NodeId,
    bad_property: NodeId,
) -> Result<bool, SearchError> {
    Ok(model
        .active_bad(&state_values(state), &WordValues::from([(input, 0)]))
        .map_err(|error| reject(error.to_string()))?
        .contains(&bad_property))
}

#[derive(Default)]
struct SearchBudget {
    total_states: usize,
    node_steps: u64,
}

impl SearchBudget {
    fn add_layer(&mut self, model: &Btor2Model, states: usize) -> Result<(), SearchError> {
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
                    .checked_mul(2)
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
    let (input, _) = validate_model(&model, bad_property)?;
    let initial = state_key(
        &model
            .initial_state()
            .map_err(|error| reject(error.to_string()))?,
    );
    if bad_active(&model, &initial, input, bad_property)? {
        return Ok(SearchCertificate {
            source_sha256: digest(source),
            query_horizon: horizon,
            bad_property,
            input,
            result: SearchResult::Unsafe,
            bad_frame: Some(0),
            witness_inputs: Vec::new(),
            layers: Vec::new(),
        });
    }
    let mut layers = vec![vec![initial.clone()]];
    let mut predecessors = Vec::<BTreeMap<SearchState, (SearchState, bool)>>::new();
    let mut budget = SearchBudget::default();
    budget.add_layer(&model, 1)?;
    for frame in 0..horizon {
        let mut next = BTreeSet::new();
        let mut prior = BTreeMap::new();
        for state in &layers[frame as usize] {
            for input_value in [false, true] {
                let values = model
                    .step(
                        &state_values(state),
                        &WordValues::from([(input, u64::from(input_value))]),
                    )
                    .map_err(|error| reject(error.to_string()))?;
                let target = state_key(&values);
                if next.insert(target.clone()) {
                    prior.insert(target, (state.clone(), input_value));
                }
            }
        }
        budget.add_layer(&model, next.len())?;
        let next = next.into_iter().collect::<Vec<_>>();
        let mut bad_state = None;
        for state in &next {
            if bad_active(&model, state, input, bad_property)? {
                bad_state = Some(state.clone());
                break;
            }
        }
        if let Some(bad_state) = bad_state {
            predecessors.push(prior);
            let mut cursor = bad_state;
            let mut witness = Vec::with_capacity((frame + 1) as usize);
            for predecessor_layer in predecessors.iter().rev() {
                let (previous, bit) = predecessor_layer
                    .get(&cursor)
                    .ok_or_else(|| reject("internal predecessor chain is incomplete"))?;
                witness.push(*bit);
                cursor = previous.clone();
            }
            witness.reverse();
            return Ok(SearchCertificate {
                source_sha256: digest(source),
                query_horizon: horizon,
                bad_property,
                input,
                result: SearchResult::Unsafe,
                bad_frame: Some(frame + 1),
                witness_inputs: witness,
                layers: Vec::new(),
            });
        }
        predecessors.push(prior);
        layers.push(next);
    }
    Ok(SearchCertificate {
        source_sha256: digest(source),
        query_horizon: horizon,
        bad_property,
        input,
        result: SearchResult::Safe,
        bad_frame: None,
        witness_inputs: Vec::new(),
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
    let (input, _) = validate_model(&model, certificate.bad_property)?;
    if input != certificate.input {
        return Err(reject("search certificate input does not match source"));
    }
    let initial = state_key(
        &model
            .initial_state()
            .map_err(|error| reject(error.to_string()))?,
    );
    match certificate.result {
        SearchResult::Unsafe => {
            if !certificate.layers.is_empty()
                || certificate.bad_frame != Some(certificate.witness_inputs.len() as u32)
                || certificate.witness_inputs.len() > certificate.query_horizon as usize
            {
                return Err(reject("UNSAFE search certificate shape is invalid"));
            }
            let witness_work = (certificate.witness_inputs.len() as u64)
                .checked_mul(model.nodes().len() as u64)
                .and_then(|value| value.checked_mul(model.states().len().max(1) as u64))
                .ok_or_else(|| reject("UNSAFE witness node-step estimate overflowed"))?;
            if witness_work > MAX_SEARCH_NODE_STEPS {
                return Err(reject("UNSAFE witness exceeds node-step limit"));
            }
            let mut state = state_values(&initial);
            for bit in &certificate.witness_inputs {
                state = model
                    .step(&state, &WordValues::from([(input, u64::from(*bit))]))
                    .map_err(|error| reject(error.to_string()))?;
            }
            let final_state = state_key(&state);
            if !bad_active(&model, &final_state, input, certificate.bad_property)? {
                return Err(reject("UNSAFE witness does not reach the bad property"));
            }
            Ok(SearchSummary {
                result: SearchResult::Unsafe,
                query_horizon: certificate.query_horizon,
                bad_frame: certificate.bad_frame,
                reachable_states: certificate.witness_inputs.len() + 1,
            })
        }
        SearchResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_inputs.is_empty()
                || certificate.layers.len() != certificate.query_horizon as usize + 1
                || certificate.layers.first() != Some(&vec![initial])
            {
                return Err(reject("SAFE search certificate shape is invalid"));
            }
            let mut budget = SearchBudget::default();
            for (frame, layer) in certificate.layers.iter().enumerate() {
                if layer.is_empty() || !layer.windows(2).all(|pair| pair[0] < pair[1]) {
                    return Err(reject(format!("reachable layer {frame} is noncanonical")));
                }
                for state in layer {
                    validate_state_shape(&model, state)?;
                }
                budget.add_layer(&model, layer.len())?;
                for state in layer {
                    if bad_active(&model, state, input, certificate.bad_property)? {
                        return Err(reject(format!(
                            "reachable layer {frame} contains a bad state"
                        )));
                    }
                }
                if frame + 1 < certificate.layers.len() {
                    let mut expected = BTreeSet::new();
                    for state in layer {
                        for input_value in [false, true] {
                            let target = model
                                .step(
                                    &state_values(state),
                                    &WordValues::from([(input, u64::from(input_value))]),
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
    let witness = certificate
        .witness_inputs
        .iter()
        .map(|bit| if *bit { '1' } else { '0' })
        .collect::<String>();
    let mut lines = vec![
        format!("search_certificate_version={SEARCH_CERTIFICATE_VERSION}"),
        format!("source_sha256={}", certificate.source_sha256),
        format!("query_horizon={}", certificate.query_horizon),
        format!("bad_property={}", certificate.bad_property),
        format!("input={}", certificate.input),
        format!("result={result}"),
        format!("bad_frame={bad_frame}"),
        format!("witness={witness}"),
        format!("layer_count={}", certificate.layers.len()),
    ];
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
    if version != SEARCH_CERTIFICATE_VERSION {
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
    let input = number(take(&mut lines, "input")?, "input")?;
    let result = match take(&mut lines, "result")?.as_str() {
        "SAFE" => SearchResult::Safe,
        "UNSAFE" => SearchResult::Unsafe,
        _ => return Err(reject("search result must be SAFE or UNSAFE")),
    };
    let bad_frame = match take(&mut lines, "bad_frame")?.as_str() {
        "none" => None,
        value => Some(number(value.to_string(), "bad frame")?),
    };
    let witness_text = take(&mut lines, "witness")?;
    if witness_text.len() > MAX_SEARCH_HORIZON as usize
        || !witness_text.bytes().all(|byte| matches!(byte, b'0' | b'1'))
    {
        return Err(reject("search witness is not a bit string"));
    }
    let witness_inputs = witness_text.bytes().map(|byte| byte == b'1').collect();
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
        if count == 0 || count > MAX_STATES_PER_LAYER {
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
        source_sha256,
        query_horizon,
        bad_property,
        input,
        result,
        bad_frame,
        witness_inputs,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATCHDOG: &[u8] = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
    const SATURATING: &[u8] =
        include_bytes!("../examples/btor2/saturating-timer-rejected-v1.btor2");

    #[test]
    fn proves_both_bounded_answers_and_round_trips() {
        let safe = produce(WATCHDOG, 13, 2).unwrap();
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
