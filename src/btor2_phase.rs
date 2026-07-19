//! Source-bound closed-form certificates for a strict BTOR2 counter subset.

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind, WordValues};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const PHASE_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_PHASES: usize = 4_096;
pub const MAX_HORIZON: u64 = 1_000_000_000_000;
pub const MAX_CERTIFICATE_BYTES: usize = 512 * 1024;
pub const REPLAY_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_REPLAY_HORIZON: u64 = 100_000;
pub const MAX_REPLAY_NODE_STEPS: u64 = 10_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Phase {
    pub input: bool,
    pub length: u64,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseCertificate {
    pub source_sha256: String,
    pub state: NodeId,
    pub input: NodeId,
    pub width: u32,
    pub initial: u64,
    pub delta: u64,
    pub reset: u64,
    pub horizon: u64,
    pub bad_property: NodeId,
    pub final_state: u64,
    pub phases: Vec<Phase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseSpec {
    pub input: bool,
    pub length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifySummary {
    pub horizon: u64,
    pub phases: usize,
    pub final_state: u64,
    pub bad_property: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayCertificate {
    pub source_sha256: String,
    pub input: NodeId,
    pub horizon: u64,
    pub bad_property: NodeId,
    pub phases: Vec<PhaseSpec>,
    pub final_states: Vec<(NodeId, u64)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificateError(pub String);

impl fmt::Display for CertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CertificateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CounterShape {
    state: NodeId,
    input: NodeId,
    width: u32,
    initial: u64,
    delta: u64,
    reset: u64,
}

fn reject(message: impl Into<String>) -> CertificateError {
    CertificateError(message.into())
}

fn mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn constant(model: &Btor2Model, id: NodeId) -> Result<u64, CertificateError> {
    match model.nodes().get(&id).map(|node| &node.kind) {
        Some(NodeKind::Constant(value)) => Ok(*value),
        _ => Err(reject(format!("node {id} is not a literal constant"))),
    }
}

fn recognise(model: &Btor2Model) -> Result<CounterShape, CertificateError> {
    if model.states().len() != 1 || model.inputs().len() != 1 {
        return Err(reject(
            "phase certificates require exactly one state and one input",
        ));
    }
    if !model.constraints().is_empty() {
        return Err(reject("phase certificates do not admit constraints"));
    }
    let state = model.states()[0];
    let input = model.inputs()[0];
    let state_node = &model.nodes()[&state];
    if model.nodes()[&input].width != 1 {
        return Err(reject("phase control input must be one bit"));
    }
    let initial = constant(
        model,
        model
            .initialiser(state)
            .ok_or_else(|| reject("missing state initialiser"))?,
    )?;
    let next = model
        .next_value(state)
        .ok_or_else(|| reject("missing state next expression"))?;
    let (condition, reset_node, advance_node) = match model.nodes()[&next].kind {
        NodeKind::Ite(condition, reset, advance) => (condition, reset, advance),
        _ => return Err(reject("next expression is not reset-or-advance ite")),
    };
    if condition != input {
        return Err(reject("ite condition is not the sole control input"));
    }
    let reset = constant(model, reset_node)?;
    let (operator, left, right) = match model.nodes()[&advance_node].kind {
        NodeKind::Binary(operator, left, right) => (operator, left, right),
        _ => {
            return Err(reject(
                "advance expression is not affine addition or subtraction",
            ));
        }
    };
    let delta = match (operator, left, right) {
        (BinaryOp::Add, operand, literal) if operand == state => constant(model, literal)?,
        (BinaryOp::Add, literal, operand) if operand == state => constant(model, literal)?,
        (BinaryOp::Sub, operand, literal) if operand == state => {
            0u64.wrapping_sub(constant(model, literal)?) & mask(state_node.width)
        }
        _ => {
            return Err(reject(
                "advance expression is not state plus a literal delta",
            ));
        }
    };
    Ok(CounterShape {
        state,
        input,
        width: state_node.width,
        initial,
        delta,
        reset,
    })
}

fn bad_expression(model: &Btor2Model, bad_property: NodeId) -> Result<NodeId, CertificateError> {
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
        return Err(reject(
            "phase certificates require a state-only bad property",
        ));
    }
    Ok(expression)
}

fn phase_end(shape: CounterShape, start: u64, input: bool, length: u64) -> u64 {
    if length == 0 {
        return start;
    }
    if input {
        shape.reset
    } else {
        start.wrapping_add(shape.delta.wrapping_mul(length)) & mask(shape.width)
    }
}

pub fn produce(
    source: &[u8],
    specs: &[PhaseSpec],
    bad_property: NodeId,
) -> Result<PhaseCertificate, CertificateError> {
    if specs.is_empty() || specs.len() > MAX_PHASES {
        return Err(reject(format!("phase count must be in 1..={MAX_PHASES}")));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let shape = recognise(&model)?;
    bad_expression(&model, bad_property)?;
    let mut horizon = 0u64;
    let mut current = shape.initial;
    let mut phases = Vec::with_capacity(specs.len());
    for (index, spec) in specs.iter().enumerate() {
        if spec.length == 0 {
            return Err(reject(format!("phase {index} has zero length")));
        }
        if index > 0 && specs[index - 1].input == spec.input {
            return Err(reject("adjacent phases must have distinct input values"));
        }
        horizon = horizon
            .checked_add(spec.length)
            .filter(|value| *value <= MAX_HORIZON)
            .ok_or_else(|| reject("certificate horizon exceeds limit"))?;
        let end = phase_end(shape, current, spec.input, spec.length);
        phases.push(Phase {
            input: spec.input,
            length: spec.length,
            start: current,
            end,
        });
        current = end;
    }
    let state_values = WordValues::from([(shape.state, current)]);
    let input_values = WordValues::from([(shape.input, 0)]);
    if !model
        .active_bad(&state_values, &input_values)
        .map_err(|error| reject(error.to_string()))?
        .contains(&bad_property)
    {
        return Err(reject(
            "final phase endpoint does not activate the claimed bad property",
        ));
    }
    Ok(PhaseCertificate {
        source_sha256: digest(source),
        state: shape.state,
        input: shape.input,
        width: shape.width,
        initial: shape.initial,
        delta: shape.delta,
        reset: shape.reset,
        horizon,
        bad_property,
        final_state: current,
        phases,
    })
}

pub fn verify(
    source: &[u8],
    certificate: &PhaseCertificate,
) -> Result<VerifySummary, CertificateError> {
    if certificate.source_sha256 != digest(source) {
        return Err(reject("source digest does not match certificate"));
    }
    if certificate.phases.is_empty() || certificate.phases.len() > MAX_PHASES {
        return Err(reject("certificate phase count is outside limits"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let shape = recognise(&model)?;
    if (certificate.state, certificate.input, certificate.width)
        != (shape.state, shape.input, shape.width)
        || (certificate.initial, certificate.delta, certificate.reset)
            != (shape.initial, shape.delta, shape.reset)
    {
        return Err(reject(
            "recognized source recurrence does not match certificate",
        ));
    }
    let mut horizon = 0u64;
    let mut current = shape.initial;
    let mut previous_input = None;
    for (index, phase) in certificate.phases.iter().enumerate() {
        if phase.length == 0 || previous_input == Some(phase.input) {
            return Err(reject(format!("phase {index} is noncanonical")));
        }
        if phase.start != current {
            return Err(reject(format!(
                "phase {index} start does not continue prior endpoint"
            )));
        }
        horizon = horizon
            .checked_add(phase.length)
            .filter(|value| *value <= MAX_HORIZON)
            .ok_or_else(|| reject("certificate horizon exceeds limit"))?;
        let expected = phase_end(shape, current, phase.input, phase.length);
        if phase.end != expected {
            return Err(reject(format!("phase {index} endpoint is invalid")));
        }
        current = expected;
        previous_input = Some(phase.input);
    }
    if horizon != certificate.horizon || current != certificate.final_state {
        return Err(reject("certificate summary does not match its phases"));
    }
    bad_expression(&model, certificate.bad_property)?;
    let state = BTreeMap::from([(shape.state, current)]);
    let inputs = BTreeMap::from([(shape.input, 0)]);
    if !model
        .active_bad(&state, &inputs)
        .map_err(|error| reject(error.to_string()))?
        .contains(&certificate.bad_property)
    {
        return Err(reject("claimed bad property is inactive at final endpoint"));
    }
    Ok(VerifySummary {
        horizon,
        phases: certificate.phases.len(),
        final_state: current,
        bad_property: certificate.bad_property,
    })
}

fn validate_specs(specs: &[PhaseSpec], horizon_limit: u64) -> Result<u64, CertificateError> {
    if specs.is_empty() || specs.len() > MAX_PHASES {
        return Err(reject(format!("phase count must be in 1..={MAX_PHASES}")));
    }
    let mut horizon = 0u64;
    for (index, phase) in specs.iter().enumerate() {
        if phase.length == 0 || (index > 0 && specs[index - 1].input == phase.input) {
            return Err(reject(format!("phase {index} is noncanonical")));
        }
        horizon = horizon
            .checked_add(phase.length)
            .filter(|value| *value <= horizon_limit)
            .ok_or_else(|| reject("certificate horizon exceeds limit"))?;
    }
    Ok(horizon)
}

fn replay(
    model: &Btor2Model,
    input: NodeId,
    specs: &[PhaseSpec],
) -> Result<WordValues, CertificateError> {
    let mut state = model
        .initial_state()
        .map_err(|error| reject(error.to_string()))?;
    for phase in specs {
        let inputs = WordValues::from([(input, u64::from(phase.input))]);
        for _ in 0..phase.length {
            state = model
                .step(&state, &inputs)
                .map_err(|error| reject(error.to_string()))?;
        }
    }
    Ok(state)
}

fn replay_work_gate(model: &Btor2Model, horizon: u64) -> Result<(), CertificateError> {
    let work = horizon
        .checked_mul(model.nodes().len() as u64)
        .and_then(|value| value.checked_mul(model.states().len().max(1) as u64))
        .ok_or_else(|| reject("replay node-step estimate overflowed"))?;
    if work > MAX_REPLAY_NODE_STEPS {
        return Err(reject(format!(
            "replay exceeds the {MAX_REPLAY_NODE_STEPS} node-step limit"
        )));
    }
    Ok(())
}

pub fn produce_replay(
    source: &[u8],
    specs: &[PhaseSpec],
    bad_property: NodeId,
) -> Result<ReplayCertificate, CertificateError> {
    let horizon = validate_specs(specs, MAX_REPLAY_HORIZON)?;
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    replay_work_gate(&model, horizon)?;
    if model.inputs().len() != 1 || model.nodes()[&model.inputs()[0]].width != 1 {
        return Err(reject(
            "replay certificates require exactly one one-bit input",
        ));
    }
    bad_expression(&model, bad_property)?;
    let input = model.inputs()[0];
    let state = replay(&model, input, specs)?;
    let final_inputs = WordValues::from([(input, 0)]);
    if !model
        .active_bad(&state, &final_inputs)
        .map_err(|error| reject(error.to_string()))?
        .contains(&bad_property)
    {
        return Err(reject(
            "replay endpoint does not activate the claimed bad property",
        ));
    }
    Ok(ReplayCertificate {
        source_sha256: digest(source),
        input,
        horizon,
        bad_property,
        phases: specs.to_vec(),
        final_states: state.into_iter().collect(),
    })
}

pub fn verify_replay(
    source: &[u8],
    certificate: &ReplayCertificate,
) -> Result<VerifySummary, CertificateError> {
    if certificate.source_sha256 != digest(source) {
        return Err(reject("source digest does not match replay certificate"));
    }
    let horizon = validate_specs(&certificate.phases, MAX_REPLAY_HORIZON)?;
    if horizon != certificate.horizon {
        return Err(reject("replay horizon does not match phases"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    replay_work_gate(&model, horizon)?;
    if model.inputs() != [certificate.input] || model.nodes()[&certificate.input].width != 1 {
        return Err(reject("replay input does not match source"));
    }
    bad_expression(&model, certificate.bad_property)?;
    let state = replay(&model, certificate.input, &certificate.phases)?;
    let expected_states = state
        .iter()
        .map(|(id, value)| (*id, *value))
        .collect::<Vec<_>>();
    if certificate.final_states != expected_states {
        return Err(reject("replay final states do not match exact execution"));
    }
    let final_inputs = WordValues::from([(certificate.input, 0)]);
    if !model
        .active_bad(&state, &final_inputs)
        .map_err(|error| reject(error.to_string()))?
        .contains(&certificate.bad_property)
    {
        return Err(reject("replay bad property is inactive at final state"));
    }
    Ok(VerifySummary {
        horizon,
        phases: certificate.phases.len(),
        final_state: certificate
            .final_states
            .first()
            .map_or(0, |(_, value)| *value),
        bad_property: certificate.bad_property,
    })
}

pub fn encode(certificate: &PhaseCertificate) -> Result<String, CertificateError> {
    verify_digest(&certificate.source_sha256)?;
    let mut lines = vec![
        format!("phase_certificate_version={PHASE_CERTIFICATE_VERSION}"),
        format!("source_sha256={}", certificate.source_sha256),
        format!("state={}", certificate.state),
        format!("input={}", certificate.input),
        format!("width={}", certificate.width),
        format!("initial={}", certificate.initial),
        format!("delta={}", certificate.delta),
        format!("reset={}", certificate.reset),
        format!("horizon={}", certificate.horizon),
        format!("bad_property={}", certificate.bad_property),
        format!("final_state={}", certificate.final_state),
        format!("phase_count={}", certificate.phases.len()),
    ];
    for (index, phase) in certificate.phases.iter().enumerate() {
        lines.push(format!(
            "phase_{index}={},{},{},{}",
            u8::from(phase.input),
            phase.length,
            phase.start,
            phase.end
        ));
    }
    lines.push("status=complete".to_string());
    let text = format!("{}\n", lines.join("\n"));
    if text.len() > MAX_CERTIFICATE_BYTES {
        return Err(reject("encoded certificate exceeds byte limit"));
    }
    Ok(text)
}

fn verify_digest(value: &str) -> Result<(), CertificateError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(reject(
            "source digest must be 64 lowercase hexadecimal characters",
        ))
    }
}

pub fn decode(bytes: &[u8]) -> Result<PhaseCertificate, CertificateError> {
    if bytes.len() > MAX_CERTIFICATE_BYTES {
        return Err(reject("certificate exceeds byte limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject("certificate must be canonical LF text without NUL"));
    }
    let mut lines = text.lines();
    fn take(lines: &mut std::str::Lines<'_>, key: &str) -> Result<String, CertificateError> {
        let line = lines
            .next()
            .ok_or_else(|| reject(format!("missing {key}")))?;
        line.strip_prefix(&format!("{key}="))
            .map(str::to_string)
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    fn number<T: std::str::FromStr>(value: String, key: &str) -> Result<T, CertificateError> {
        value.parse().map_err(|_| reject(format!("invalid {key}")))
    }
    let version: u32 = number(take(&mut lines, "phase_certificate_version")?, "version")?;
    if version != PHASE_CERTIFICATE_VERSION {
        return Err(reject("unsupported phase certificate version"));
    }
    let source_sha256 = take(&mut lines, "source_sha256")?;
    verify_digest(&source_sha256)?;
    let state = number(take(&mut lines, "state")?, "state")?;
    let input = number(take(&mut lines, "input")?, "input")?;
    let width = number(take(&mut lines, "width")?, "width")?;
    let initial = number(take(&mut lines, "initial")?, "initial")?;
    let delta = number(take(&mut lines, "delta")?, "delta")?;
    let reset = number(take(&mut lines, "reset")?, "reset")?;
    let horizon = number(take(&mut lines, "horizon")?, "horizon")?;
    let bad_property = number(take(&mut lines, "bad_property")?, "bad_property")?;
    let final_state = number(take(&mut lines, "final_state")?, "final_state")?;
    let phase_count: usize = number(take(&mut lines, "phase_count")?, "phase_count")?;
    if phase_count == 0 || phase_count > MAX_PHASES {
        return Err(reject("phase count is outside limits"));
    }
    let mut phases = Vec::with_capacity(phase_count);
    for index in 0..phase_count {
        let value = take(&mut lines, &format!("phase_{index}"))?;
        let fields = value.split(',').collect::<Vec<_>>();
        if fields.len() != 4 || !matches!(fields[0], "0" | "1") {
            return Err(reject(format!("invalid phase {index}")));
        }
        phases.push(Phase {
            input: fields[0] == "1",
            length: number(fields[1].to_string(), "phase length")?,
            start: number(fields[2].to_string(), "phase start")?,
            end: number(fields[3].to_string(), "phase end")?,
        });
    }
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject("certificate is incomplete or has trailing fields"));
    }
    Ok(PhaseCertificate {
        source_sha256,
        state,
        input,
        width,
        initial,
        delta,
        reset,
        horizon,
        bad_property,
        final_state,
        phases,
    })
}

pub fn encode_replay(certificate: &ReplayCertificate) -> Result<String, CertificateError> {
    verify_digest(&certificate.source_sha256)?;
    let mut lines = vec![
        format!("replay_certificate_version={REPLAY_CERTIFICATE_VERSION}"),
        format!("source_sha256={}", certificate.source_sha256),
        format!("input={}", certificate.input),
        format!("horizon={}", certificate.horizon),
        format!("bad_property={}", certificate.bad_property),
        format!("phase_count={}", certificate.phases.len()),
    ];
    for (index, phase) in certificate.phases.iter().enumerate() {
        lines.push(format!(
            "phase_{index}={},{}",
            u8::from(phase.input),
            phase.length
        ));
    }
    lines.push(format!("state_count={}", certificate.final_states.len()));
    for (index, (id, value)) in certificate.final_states.iter().enumerate() {
        lines.push(format!("state_{index}={id},{value}"));
    }
    lines.push("status=complete".to_string());
    let text = format!("{}\n", lines.join("\n"));
    if text.len() > MAX_CERTIFICATE_BYTES {
        return Err(reject("encoded replay certificate exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode_replay(bytes: &[u8]) -> Result<ReplayCertificate, CertificateError> {
    if bytes.len() > MAX_CERTIFICATE_BYTES {
        return Err(reject("replay certificate exceeds byte limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("replay certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(
            "replay certificate must be canonical LF text without NUL",
        ));
    }
    let mut lines = text.lines();
    fn take(lines: &mut std::str::Lines<'_>, key: &str) -> Result<String, CertificateError> {
        let line = lines
            .next()
            .ok_or_else(|| reject(format!("missing {key}")))?;
        line.strip_prefix(&format!("{key}="))
            .map(str::to_string)
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    fn number<T: std::str::FromStr>(value: String, key: &str) -> Result<T, CertificateError> {
        value.parse().map_err(|_| reject(format!("invalid {key}")))
    }
    let version: u32 = number(take(&mut lines, "replay_certificate_version")?, "version")?;
    if version != REPLAY_CERTIFICATE_VERSION {
        return Err(reject("unsupported replay certificate version"));
    }
    let source_sha256 = take(&mut lines, "source_sha256")?;
    verify_digest(&source_sha256)?;
    let input = number(take(&mut lines, "input")?, "input")?;
    let horizon = number(take(&mut lines, "horizon")?, "horizon")?;
    let bad_property = number(take(&mut lines, "bad_property")?, "bad property")?;
    let phase_count: usize = number(take(&mut lines, "phase_count")?, "phase count")?;
    if phase_count == 0 || phase_count > MAX_PHASES {
        return Err(reject("replay phase count is outside limits"));
    }
    let mut phases = Vec::with_capacity(phase_count);
    for index in 0..phase_count {
        let value = take(&mut lines, &format!("phase_{index}"))?;
        let fields = value.split(',').collect::<Vec<_>>();
        if fields.len() != 2 || !matches!(fields[0], "0" | "1") {
            return Err(reject(format!("invalid replay phase {index}")));
        }
        phases.push(PhaseSpec {
            input: fields[0] == "1",
            length: number(fields[1].to_string(), "phase length")?,
        });
    }
    let state_count: usize = number(take(&mut lines, "state_count")?, "state count")?;
    if state_count == 0 || state_count > btor2::MAX_BTOR2_NODES {
        return Err(reject("replay state count is outside limits"));
    }
    let mut final_states = Vec::with_capacity(state_count);
    for index in 0..state_count {
        let value = take(&mut lines, &format!("state_{index}"))?;
        let fields = value.split(',').collect::<Vec<_>>();
        if fields.len() != 2 {
            return Err(reject(format!("invalid replay state {index}")));
        }
        let id = number(fields[0].to_string(), "state identifier")?;
        let value = number(fields[1].to_string(), "state value")?;
        if final_states.last().is_some_and(|(prior, _)| *prior >= id) {
            return Err(reject("replay states are not strictly ordered"));
        }
        final_states.push((id, value));
    }
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "replay certificate is incomplete or has trailing fields",
        ));
    }
    Ok(ReplayCertificate {
        source_sha256,
        input,
        horizon,
        bad_property,
        phases,
        final_states,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATCHDOG: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 kick\n4 zero 2\n5 state 2 timer\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n11 constd 2 3\n12 ugte 1 5 11\n13 bad 12 expired\n";

    #[test]
    fn round_trip_verifies_a_billion_step_compressed_witness() {
        let certificate = produce(
            WATCHDOG,
            &[
                PhaseSpec {
                    input: true,
                    length: 2,
                },
                PhaseSpec {
                    input: false,
                    length: 1_000_000_003,
                },
            ],
            13,
        )
        .unwrap();
        let encoded = encode(&certificate).unwrap();
        assert!(encoded.len() < 512);
        let decoded = decode(encoded.as_bytes()).unwrap();
        let summary = verify(WATCHDOG, &decoded).unwrap();
        assert_eq!(summary.horizon, 1_000_000_005);
        assert_eq!(summary.final_state, 3);
    }

    #[test]
    fn rejects_tampering_source_drift_and_unsupported_shapes() {
        let mut certificate = produce(
            WATCHDOG,
            &[PhaseSpec {
                input: false,
                length: 3,
            }],
            13,
        )
        .unwrap();
        certificate.phases[0].end = 4;
        assert!(
            verify(WATCHDOG, &certificate)
                .unwrap_err()
                .0
                .contains("endpoint")
        );
        let original = produce(
            WATCHDOG,
            &[PhaseSpec {
                input: false,
                length: 3,
            }],
            13,
        )
        .unwrap();
        let drifted = std::str::from_utf8(WATCHDOG)
            .unwrap()
            .replace("11 constd 2 3", "11 constd 2 4");
        assert!(verify(drifted.as_bytes(), &original).is_err());
        let unsupported = WATCHDOG
            .windows(1)
            .flat_map(|byte| byte.iter().copied())
            .collect::<Vec<_>>();
        let unsupported = String::from_utf8(unsupported)
            .unwrap()
            .replace("9 ite 2 3 4 8", "9 add 2 5 7");
        assert!(
            produce(
                unsupported.as_bytes(),
                &[PhaseSpec {
                    input: false,
                    length: 3
                }],
                13
            )
            .is_err()
        );
        let input_dependent = std::str::from_utf8(WATCHDOG)
            .unwrap()
            .replace("13 bad 12 expired", "13 bad 3 input-dependent");
        assert!(
            produce(
                input_dependent.as_bytes(),
                &[PhaseSpec {
                    input: false,
                    length: 3,
                }],
                13,
            )
            .unwrap_err()
            .0
            .contains("state-only")
        );
    }

    #[test]
    fn decoder_is_bounded_and_canonical() {
        assert!(decode(&vec![b'x'; MAX_CERTIFICATE_BYTES + 1]).is_err());
        assert!(decode(b"phase_certificate_version=1\r\n").is_err());
        assert!(
            produce(
                WATCHDOG,
                &[PhaseSpec {
                    input: false,
                    length: 0
                }],
                13
            )
            .is_err()
        );
        assert!(
            produce(
                WATCHDOG,
                &[
                    PhaseSpec {
                        input: false,
                        length: MAX_HORIZON
                    },
                    PhaseSpec {
                        input: true,
                        length: 1
                    }
                ],
                13
            )
            .is_err()
        );
    }

    #[test]
    fn exact_replay_covers_the_rejected_saturating_neighbour() {
        let source = include_bytes!("../examples/btor2/saturating-timer-rejected-v1.btor2");
        let specs = [PhaseSpec {
            input: false,
            length: 255,
        }];
        assert!(produce(source, &specs, 15).is_err());
        let produced = produce_replay(source, &specs, 15).unwrap();
        let encoded = encode_replay(&produced).unwrap();
        let mut certificate = decode_replay(encoded.as_bytes()).unwrap();
        let summary = verify_replay(source, &certificate).unwrap();
        assert_eq!(summary.horizon, 255);
        assert_eq!(summary.final_state, 255);
        certificate.final_states[0].1 = 254;
        assert!(verify_replay(source, &certificate).is_err());
    }

    #[test]
    fn exact_replay_rejects_accelerated_scale_horizons() {
        let specs = [PhaseSpec {
            input: false,
            length: MAX_REPLAY_HORIZON + 1,
        }];
        assert!(produce_replay(WATCHDOG, &specs, 13).is_err());

        let mut source = "1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 kick\n4 zero 2\n5 state 2 timer\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n".to_string();
        for id in 11..=200 {
            source.push_str(&format!("{id} zero 2\n"));
        }
        source.push_str("201 eq 1 5 200\n202 bad 201 zero\n");
        let workload = [PhaseSpec {
            input: false,
            length: MAX_REPLAY_HORIZON,
        }];
        assert!(
            produce_replay(source.as_bytes(), &workload, 202)
                .unwrap_err()
                .0
                .contains("node-step")
        );
    }
}
