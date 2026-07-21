//! Exact compressed SAFE certificates for recognised one-word BTOR2 recurrences.

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

pub const REGION_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_REGION_HORIZON: u32 = 1_000_000_000;
pub const MAX_REGION_CERTIFICATE_BYTES: usize = 64 * 1024;
pub const MAX_REGION_SET_MEMBERS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionFamily {
    ResetAdd,
    ResetSaturatingAdd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionPredicate {
    Equal,
    UnsignedGreaterEqual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionCertificate {
    pub source_sha256: String,
    pub query_horizon: u32,
    pub bad_property: NodeId,
    pub input: NodeId,
    pub state: NodeId,
    pub width: u32,
    pub family: RegionFamily,
    pub initial: u64,
    pub reset: u64,
    pub delta: u64,
    pub saturation: Option<u64>,
    pub predicate: RegionPredicate,
    pub predicate_literal: u64,
    pub max_index: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionSummary {
    pub query_horizon: u32,
    pub max_index: u64,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionError(pub String);

impl fmt::Display for RegionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for RegionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Shape {
    input: NodeId,
    state: NodeId,
    width: u32,
    family: RegionFamily,
    initial: u64,
    reset: u64,
    delta: u64,
    saturation: Option<u64>,
}

fn reject(message: impl Into<String>) -> RegionError {
    RegionError(message.into())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn constant(model: &Btor2Model, id: NodeId) -> Option<u64> {
    match model.nodes().get(&id)?.kind {
        NodeKind::Constant(value) => Some(value),
        _ => None,
    }
}

fn boolean_identity_root(model: &Btor2Model, mut id: NodeId) -> NodeId {
    loop {
        let replacement = match model.nodes().get(&id).map(|node| &node.kind) {
            Some(NodeKind::Uext { value, amount: 0 }) => Some(*value),
            Some(NodeKind::Unary(btor2::UnaryOp::Not, inner)) => {
                match model.nodes().get(inner).map(|node| &node.kind) {
                    Some(NodeKind::Unary(btor2::UnaryOp::Not, value)) => Some(*value),
                    _ => None,
                }
            }
            Some(NodeKind::Binary(BinaryOp::And, left, right)) => {
                if constant(model, *left) == Some(1) {
                    Some(*right)
                } else if constant(model, *right) == Some(1) {
                    Some(*left)
                } else {
                    None
                }
            }
            Some(NodeKind::Binary(BinaryOp::Or, left, right)) => {
                if constant(model, *left) == Some(0) {
                    Some(*right)
                } else if constant(model, *right) == Some(0) {
                    Some(*left)
                } else {
                    None
                }
            }
            _ => None,
        };
        let Some(replacement) = replacement else {
            return id;
        };
        id = replacement;
    }
}

fn add_delta(model: &Btor2Model, expression: NodeId, state: NodeId) -> Option<u64> {
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Add, left, right) if left == state => constant(model, right),
        NodeKind::Binary(BinaryOp::Add, left, right) if right == state => constant(model, left),
        _ => None,
    }
}

fn recognise_shape(model: &Btor2Model) -> Option<Shape> {
    if model.states().len() != 1 || model.inputs().len() != 1 || !model.constraints().is_empty() {
        return None;
    }
    let state = model.states()[0];
    let input = model.inputs()[0];
    let width = model.nodes().get(&state)?.width;
    if model.nodes().get(&input)?.width != 1 {
        return None;
    }
    let initial = constant(model, model.initialiser(state)?)?;
    let next = model.next_value(state)?;
    let (condition, reset_expression, advance) = match model.nodes().get(&next)?.kind {
        NodeKind::Ite(condition, reset, advance) => (condition, reset, advance),
        _ => return None,
    };
    if condition != input {
        return None;
    }
    let reset = constant(model, reset_expression)?;
    if initial != reset {
        return None;
    }
    if let Some(delta) = add_delta(model, advance, state).filter(|delta| *delta != 0) {
        return Some(Shape {
            input,
            state,
            width,
            family: RegionFamily::ResetAdd,
            initial,
            reset,
            delta,
            saturation: None,
        });
    }
    let (guard, hold, increment) = match model.nodes().get(&advance)?.kind {
        NodeKind::Ite(guard, hold, increment) => (guard, hold, increment),
        _ => return None,
    };
    if hold != state {
        return None;
    }
    let saturation = match model.nodes().get(&guard)?.kind {
        NodeKind::Binary(BinaryOp::Ugte, left, right) if left == state => constant(model, right)?,
        _ => return None,
    };
    let delta = add_delta(model, increment, state).filter(|delta| *delta != 0)?;
    if reset > saturation || !(saturation - reset).is_multiple_of(delta) {
        return None;
    }
    Some(Shape {
        input,
        state,
        width,
        family: RegionFamily::ResetSaturatingAdd,
        initial,
        reset,
        delta,
        saturation: Some(saturation),
    })
}

fn recognise_predicate(
    model: &Btor2Model,
    bad_property: NodeId,
    state: NodeId,
) -> Option<(RegionPredicate, u64)> {
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))?;
    let expression = boolean_identity_root(model, expression);
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Eq, left, right) if left == state => {
            Some((RegionPredicate::Equal, constant(model, right)?))
        }
        NodeKind::Binary(BinaryOp::Eq, left, right) if right == state => {
            Some((RegionPredicate::Equal, constant(model, left)?))
        }
        NodeKind::Binary(BinaryOp::Ugte, left, right) if left == state => Some((
            RegionPredicate::UnsignedGreaterEqual,
            constant(model, right)?,
        )),
        _ => None,
    }
}

fn verify_claimed_shape(
    model: &Btor2Model,
    certificate: &RegionCertificate,
) -> Result<Shape, RegionError> {
    if model.states() != [certificate.state]
        || model.inputs() != [certificate.input]
        || !model.constraints().is_empty()
    {
        return Err(reject(
            "source state, input, or constraint shape does not match",
        ));
    }
    let state_node = model
        .nodes()
        .get(&certificate.state)
        .ok_or_else(|| reject("certificate state is absent from source"))?;
    let input_node = model
        .nodes()
        .get(&certificate.input)
        .ok_or_else(|| reject("certificate input is absent from source"))?;
    if state_node.width != certificate.width || input_node.width != 1 {
        return Err(reject("source word widths do not match certificate"));
    }
    let initial = constant(
        model,
        model
            .initialiser(certificate.state)
            .ok_or_else(|| reject("source state has no initialiser"))?,
    )
    .ok_or_else(|| reject("source initialiser is not a literal"))?;
    let next = model
        .next_value(certificate.state)
        .ok_or_else(|| reject("source state has no next expression"))?;
    let (condition, reset_expression, advance) = match model.nodes()[&next].kind {
        NodeKind::Ite(condition, reset, advance) => (condition, reset, advance),
        _ => return Err(reject("source next expression is not reset-controlled")),
    };
    if condition != certificate.input {
        return Err(reject(
            "source reset condition is not the certificate input",
        ));
    }
    let reset = constant(model, reset_expression)
        .ok_or_else(|| reject("source reset expression is not a literal"))?;
    if initial != reset || initial != certificate.initial || reset != certificate.reset {
        return Err(reject("source initial and reset literals do not match"));
    }
    let (delta, saturation) = match certificate.family {
        RegionFamily::ResetAdd => {
            if certificate.saturation.is_some() {
                return Err(reject("reset-add certificate has a saturation literal"));
            }
            let delta = add_delta(model, advance, certificate.state)
                .filter(|value| *value != 0)
                .ok_or_else(|| reject("source advance is not nonzero literal addition"))?;
            (delta, None)
        }
        RegionFamily::ResetSaturatingAdd => {
            let (guard, hold, increment) = match model.nodes()[&advance].kind {
                NodeKind::Ite(guard, hold, increment) => (guard, hold, increment),
                _ => return Err(reject("source advance is not saturating addition")),
            };
            if hold != certificate.state {
                return Err(reject("source saturation branch does not hold state"));
            }
            let saturation = match model.nodes()[&guard].kind {
                NodeKind::Binary(BinaryOp::Ugte, left, right) if left == certificate.state => {
                    constant(model, right)
                        .ok_or_else(|| reject("source saturation bound is not a literal"))?
                }
                _ => return Err(reject("source saturation guard is not state >= literal")),
            };
            let delta = add_delta(model, increment, certificate.state)
                .filter(|value| *value != 0)
                .ok_or_else(|| reject("source increment is not nonzero literal addition"))?;
            if reset > saturation || !(saturation - reset).is_multiple_of(delta) {
                return Err(reject("source saturation point is not exactly aligned"));
            }
            (delta, Some(saturation))
        }
    };
    if delta != certificate.delta || saturation != certificate.saturation {
        return Err(reject(
            "source recurrence literals do not match certificate",
        ));
    }
    Ok(Shape {
        input: certificate.input,
        state: certificate.state,
        width: certificate.width,
        family: certificate.family,
        initial,
        reset,
        delta,
        saturation,
    })
}

fn verify_claimed_predicate(
    model: &Btor2Model,
    certificate: &RegionCertificate,
) -> Result<(), RegionError> {
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == certificate.bad_property).then_some(*expression))
        .ok_or_else(|| reject("certificate bad property is absent from source"))?;
    let expression = boolean_identity_root(model, expression);
    let matches = match (
        certificate.predicate,
        model.nodes()[&expression].kind.clone(),
    ) {
        (RegionPredicate::Equal, NodeKind::Binary(BinaryOp::Eq, left, right))
            if left == certificate.state =>
        {
            constant(model, right) == Some(certificate.predicate_literal)
        }
        (RegionPredicate::Equal, NodeKind::Binary(BinaryOp::Eq, left, right))
            if right == certificate.state =>
        {
            constant(model, left) == Some(certificate.predicate_literal)
        }
        (RegionPredicate::UnsignedGreaterEqual, NodeKind::Binary(BinaryOp::Ugte, left, right))
            if left == certificate.state =>
        {
            constant(model, right) == Some(certificate.predicate_literal)
        }
        _ => false,
    };
    if !matches {
        return Err(reject("source bad predicate does not match certificate"));
    }
    Ok(())
}

fn max_index(shape: Shape, horizon: u32) -> Option<u64> {
    match shape.family {
        RegionFamily::ResetAdd => {
            let distance = shape.delta.checked_mul(u64::from(horizon))?;
            (shape.reset.checked_add(distance)? <= mask(shape.width)).then_some(u64::from(horizon))
        }
        RegionFamily::ResetSaturatingAdd => {
            Some(u64::from(horizon).min((shape.saturation? - shape.reset) / shape.delta))
        }
    }
}

fn predicate_is_disjoint(
    shape: Shape,
    predicate: RegionPredicate,
    literal: u64,
    max_index: u64,
) -> bool {
    let maximum = shape.reset + shape.delta * max_index;
    match predicate {
        RegionPredicate::Equal => {
            literal < shape.reset
                || literal > maximum
                || !(literal - shape.reset).is_multiple_of(shape.delta)
        }
        RegionPredicate::UnsignedGreaterEqual => maximum < literal,
    }
}

/// Produces a compressed exact SAFE certificate when the source is in the
/// admitted recurrence language and the selected bad set is disjoint.
/// `Ok(None)` means that the exact fallback must answer the original query.
pub fn try_produce_safe(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<Option<RegionCertificate>, RegionError> {
    try_produce_safe_set(source, &[bad_property], horizon)
        .map(|certificates| certificates.map(|mut values| values.pop().unwrap()))
}

/// Produces source-compatible region certificates with one parse and one shape
/// recognition pass. `Ok(None)` preserves the complete caller query for exact
/// fallback when any member is unsupported or intersects the reachable set.
pub fn try_produce_safe_set(
    source: &[u8],
    bad_properties: &[NodeId],
    horizon: u32,
) -> Result<Option<Vec<RegionCertificate>>, RegionError> {
    if horizon > MAX_REGION_HORIZON {
        return Err(reject("region query horizon exceeds limit"));
    }
    if bad_properties.is_empty() || bad_properties.len() > MAX_REGION_SET_MEMBERS {
        return Err(reject("region property-set member count is outside limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let Some(shape) = recognise_shape(&model) else {
        return Ok(None);
    };
    let Some(max_index) = max_index(shape, horizon) else {
        return Ok(None);
    };
    let source_sha256 = digest(source);
    let mut certificates = Vec::with_capacity(bad_properties.len());
    for bad_property in bad_properties {
        let Some((predicate, predicate_literal)) =
            recognise_predicate(&model, *bad_property, shape.state)
        else {
            return Ok(None);
        };
        if !predicate_is_disjoint(shape, predicate, predicate_literal, max_index) {
            return Ok(None);
        }
        certificates.push(RegionCertificate {
            source_sha256: source_sha256.clone(),
            query_horizon: horizon,
            bad_property: *bad_property,
            input: shape.input,
            state: shape.state,
            width: shape.width,
            family: shape.family,
            initial: shape.initial,
            reset: shape.reset,
            delta: shape.delta,
            saturation: shape.saturation,
            predicate,
            predicate_literal,
            max_index,
        });
    }
    Ok(Some(certificates))
}

pub fn verify(
    source: &[u8],
    certificate: &RegionCertificate,
) -> Result<RegionSummary, RegionError> {
    verify_set(source, std::slice::from_ref(certificate))
}

/// Independently verifies a bounded set with one source parse. Every member is
/// checked against the source graph; shared claims are not trusted by position.
pub fn verify_set(
    source: &[u8],
    certificates: &[RegionCertificate],
) -> Result<RegionSummary, RegionError> {
    if certificates.is_empty() || certificates.len() > MAX_REGION_SET_MEMBERS {
        return Err(reject("region property-set member count is outside limit"));
    }
    let source_sha256 = digest(source);
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let first = &certificates[0];
    if first.source_sha256 != source_sha256 {
        return Err(reject("region certificate source digest mismatch"));
    }
    if first.query_horizon > MAX_REGION_HORIZON {
        return Err(reject("region certificate horizon exceeds limit"));
    }
    let first_shape = verify_claimed_shape(&model, first)?;
    let expected_max = max_index(first_shape, first.query_horizon)
        .ok_or_else(|| reject("word region would wrap and is not exact"))?;
    for certificate in certificates {
        if certificate.source_sha256 != source_sha256
            || certificate.query_horizon != first.query_horizon
        {
            return Err(reject("region certificate set binding mismatch"));
        }
        let shape = verify_claimed_shape(&model, certificate)?;
        if shape != first_shape || certificate.max_index != expected_max {
            return Err(reject("region certificate claims do not match the source"));
        }
        verify_claimed_predicate(&model, certificate)?;
        if !predicate_is_disjoint(
            shape,
            certificate.predicate,
            certificate.predicate_literal,
            expected_max,
        ) {
            return Err(reject("word region intersects the selected bad property"));
        }
    }
    let layers = u64::from(first.query_horizon) + 1;
    let saturation_layer = expected_max + 1;
    let logical_reachable_states = if layers <= saturation_layer {
        layers
            .checked_mul(layers + 1)
            .and_then(|value| value.checked_div(2))
    } else {
        saturation_layer
            .checked_mul(saturation_layer + 1)
            .and_then(|value| value.checked_div(2))
            .and_then(|prefix| {
                (layers - saturation_layer)
                    .checked_mul(saturation_layer)
                    .and_then(|tail| prefix.checked_add(tail))
            })
    }
    .ok_or_else(|| reject("logical reachable-state count overflowed"))?;
    Ok(RegionSummary {
        query_horizon: first.query_horizon,
        max_index: expected_max,
        logical_reachable_states,
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn encode(certificate: &RegionCertificate) -> Result<String, RegionError> {
    if !valid_digest(&certificate.source_sha256) {
        return Err(reject("region source digest is not canonical"));
    }
    let family = match certificate.family {
        RegionFamily::ResetAdd => "reset_add",
        RegionFamily::ResetSaturatingAdd => "reset_saturating_add",
    };
    let predicate = match certificate.predicate {
        RegionPredicate::Equal => "eq",
        RegionPredicate::UnsignedGreaterEqual => "ugte",
    };
    let saturation = certificate
        .saturation
        .map_or_else(|| "none".to_string(), |value| value.to_string());
    let text = format!(
        "region_certificate_version={REGION_CERTIFICATE_VERSION}\nsource_sha256={}\nquery_horizon={}\nbad_property={}\ninput={}\nstate={}\nwidth={}\nfamily={family}\ninitial={}\nreset={}\ndelta={}\nsaturation={saturation}\npredicate={predicate}\npredicate_literal={}\nmax_index={}\nresult=SAFE\nstatus=complete\n",
        certificate.source_sha256,
        certificate.query_horizon,
        certificate.bad_property,
        certificate.input,
        certificate.state,
        certificate.width,
        certificate.initial,
        certificate.reset,
        certificate.delta,
        certificate.predicate_literal,
        certificate.max_index,
    );
    if text.len() > MAX_REGION_CERTIFICATE_BYTES {
        return Err(reject("encoded region certificate exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode(bytes: &[u8]) -> Result<RegionCertificate, RegionError> {
    if bytes.len() > MAX_REGION_CERTIFICATE_BYTES {
        return Err(reject("region certificate exceeds byte limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("region certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(
            "region certificate must be canonical LF text without NUL",
        ));
    }
    let mut lines = text.lines();
    fn take(lines: &mut std::str::Lines<'_>, key: &str) -> Result<String, RegionError> {
        let line = lines
            .next()
            .ok_or_else(|| reject(format!("missing {key}")))?;
        line.strip_prefix(&format!("{key}="))
            .map(str::to_string)
            .ok_or_else(|| reject(format!("expected {key}")))
    }
    fn number<T: std::str::FromStr + fmt::Display>(
        value: String,
        key: &str,
    ) -> Result<T, RegionError> {
        let parsed = value
            .parse::<T>()
            .map_err(|_| reject(format!("invalid {key}")))?;
        if parsed.to_string() != value {
            return Err(reject(format!("noncanonical {key}")));
        }
        Ok(parsed)
    }
    let version: u32 = number(take(&mut lines, "region_certificate_version")?, "version")?;
    if version != REGION_CERTIFICATE_VERSION {
        return Err(reject("unsupported region certificate version"));
    }
    let source_sha256 = take(&mut lines, "source_sha256")?;
    if !valid_digest(&source_sha256) {
        return Err(reject("region source digest is not canonical"));
    }
    let query_horizon = number(take(&mut lines, "query_horizon")?, "query horizon")?;
    if query_horizon > MAX_REGION_HORIZON {
        return Err(reject("region query horizon exceeds limit"));
    }
    let bad_property = number(take(&mut lines, "bad_property")?, "bad property")?;
    let input = number(take(&mut lines, "input")?, "input")?;
    let state = number(take(&mut lines, "state")?, "state")?;
    let width = number(take(&mut lines, "width")?, "width")?;
    let family = match take(&mut lines, "family")?.as_str() {
        "reset_add" => RegionFamily::ResetAdd,
        "reset_saturating_add" => RegionFamily::ResetSaturatingAdd,
        _ => return Err(reject("unknown region family")),
    };
    let initial = number(take(&mut lines, "initial")?, "initial")?;
    let reset = number(take(&mut lines, "reset")?, "reset")?;
    let delta = number(take(&mut lines, "delta")?, "delta")?;
    let saturation = match take(&mut lines, "saturation")?.as_str() {
        "none" => None,
        value => Some(number(value.to_string(), "saturation")?),
    };
    let predicate = match take(&mut lines, "predicate")?.as_str() {
        "eq" => RegionPredicate::Equal,
        "ugte" => RegionPredicate::UnsignedGreaterEqual,
        _ => return Err(reject("unknown region predicate")),
    };
    let predicate_literal = number(take(&mut lines, "predicate_literal")?, "predicate literal")?;
    let max_index = number(take(&mut lines, "max_index")?, "max index")?;
    if take(&mut lines, "result")? != "SAFE"
        || take(&mut lines, "status")? != "complete"
        || lines.next().is_some()
    {
        return Err(reject(
            "region certificate is incomplete or has trailing fields",
        ));
    }
    Ok(RegionCertificate {
        source_sha256,
        query_horizon,
        bad_property,
        input,
        state,
        width,
        family,
        initial,
        reset,
        delta,
        saturation,
        predicate,
        predicate_literal,
        max_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATCHDOG: &[u8] = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
    const ACTUATOR: &[u8] = include_bytes!("../examples/btor2/actuator-position-v1.btor2");
    const SATURATING: &[u8] =
        include_bytes!("../examples/btor2/saturating-timer-rejected-v1.btor2");

    #[test]
    fn proves_all_three_safe_boundaries_without_layers() {
        for (source, bad, horizon, expected_states) in [
            (WATCHDOG, 13, 2, 6),
            (ACTUATOR, 13, 200, 20_301),
            (SATURATING, 15, 254, 32_640),
        ] {
            let certificate = try_produce_safe(source, bad, horizon).unwrap().unwrap();
            let encoded = encode(&certificate).unwrap();
            let decoded = decode(encoded.as_bytes()).unwrap();
            assert_eq!(
                verify(source, &decoded).unwrap().logical_reachable_states,
                expected_states
            );
        }
    }

    #[test]
    fn defers_unsafe_and_wrapping_queries_to_exact_search() {
        assert!(try_produce_safe(WATCHDOG, 13, 3).unwrap().is_none());
        assert!(try_produce_safe(ACTUATOR, 13, 201).unwrap().is_none());
        assert!(try_produce_safe(SATURATING, 15, 255).unwrap().is_none());
        assert!(try_produce_safe(WATCHDOG, 13, 256).unwrap().is_none());
    }

    #[test]
    fn admits_exact_yosys_boolean_identity_wrappers_only() {
        let source = b"1 sort bitvec 1\n2 input 1 reset\n3 input 1 clk\n4 sort bitvec 32\n5 const 4 00000000000000000000000000000000\n6 state 4 count\n7 init 4 6 5\n8 const 4 00000000000000000000000000001001\n9 ugte 1 6 8\n10 output 9 bad\n11 not 1 9\n12 const 1 1\n13 not 1 11\n14 and 1 12 13\n15 bad 14 watchdog\n16 const 4 00000000000000000000000000000001\n17 add 4 6 16\n18 uext 4 17 0 count_next\n19 ite 4 2 5 17\n20 next 4 6 19\n";
        let certificate = try_produce_safe(source, 15, 8).unwrap().unwrap();
        assert_eq!(certificate.input, 2);
        assert_eq!(certificate.width, 32);
        assert_eq!(
            verify(source, &certificate)
                .unwrap()
                .logical_reachable_states,
            45
        );
        assert!(try_produce_safe(source, 15, 9).unwrap().is_none());

        let hostile = String::from_utf8(source.to_vec())
            .unwrap()
            .replace("14 and 1 12 13", "14 xor 1 12 13");
        assert!(
            try_produce_safe(hostile.as_bytes(), 15, 8)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_source_and_semantic_tampering() {
        let certificate = try_produce_safe(ACTUATOR, 13, 200).unwrap().unwrap();
        assert!(verify(WATCHDOG, &certificate).is_err());

        let mut tampered = certificate.clone();
        tampered.max_index -= 1;
        assert!(verify(ACTUATOR, &tampered).is_err());

        let mut tampered = certificate;
        tampered.predicate_literal += 5;
        assert!(verify(ACTUATOR, &tampered).is_err());

        assert!(decode(b"region_certificate_version=1\r\n").is_err());
        assert!(
            decode(
                encode(&try_produce_safe(WATCHDOG, 13, 2).unwrap().unwrap())
                    .unwrap()
                    .replacen("query_horizon=2", "query_horizon=02", 1)
                    .as_bytes()
            )
            .is_err()
        );
        assert!(decode(&vec![b'x'; MAX_REGION_CERTIFICATE_BYTES + 1]).is_err());
    }

    #[test]
    fn verifies_a_billion_frame_safe_region_in_constant_artifact_space() {
        let source = b"1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 reset\n4 zero 2\n5 state 2 timer\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 constd 2 200\n10 ugte 1 5 9\n11 ite 2 10 5 8\n12 ite 2 3 4 11\n13 next 2 5 12\n14 constd 2 255\n15 eq 1 5 14\n16 bad 15 unreachable\n";
        let certificate = try_produce_safe(source, 16, 1_000_000_000)
            .unwrap()
            .unwrap();
        let encoded = encode(&certificate).unwrap();
        assert!(encoded.len() < 400);
        let summary = verify(source, &decode(encoded.as_bytes()).unwrap()).unwrap();
        assert_eq!(summary.max_index, 200);
        assert_eq!(summary.logical_reachable_states, 200_999_980_101);
    }

    #[test]
    fn every_single_byte_mutation_and_truncation_fails_closed() {
        let encoded = encode(&try_produce_safe(ACTUATOR, 13, 200).unwrap().unwrap())
            .unwrap()
            .into_bytes();
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode(&mutated) {
                assert!(verify(ACTUATOR, &certificate).is_err());
            }
        }
    }
}
