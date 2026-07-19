//! Exact compressed SAFE certificates for a resettable braking controller.

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

pub const BRAKING_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_BRAKING_HORIZON: u32 = 1_000_000_000;
pub const MAX_BRAKING_CERTIFICATE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrakingCertificate {
    pub source_sha256: String,
    pub query_horizon: u32,
    pub bad_property: NodeId,
    pub reset_input: NodeId,
    pub velocity_state: NodeId,
    pub position_state: NodeId,
    pub braking_state: NodeId,
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
pub struct BrakingSummary {
    pub query_horizon: u32,
    pub max_velocity: u64,
    pub max_position: u64,
    pub switch_frame: u64,
    pub stop_frame: u64,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrakingError(pub String);

impl fmt::Display for BrakingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for BrakingError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Shape {
    reset: NodeId,
    velocity: NodeId,
    position: NodeId,
    braking: NodeId,
    width: u32,
    acceleration: u64,
    brake_velocity: u64,
    deceleration: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Endpoint {
    velocity: u64,
    position: u64,
    max_velocity: u64,
    switch_frame: u64,
    stop_frame: u64,
}

fn reject(message: impl Into<String>) -> BrakingError {
    BrakingError(message.into())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn mask(width: u32) -> u128 {
    if width == 64 {
        u128::from(u64::MAX)
    } else {
        (1u128 << width) - 1
    }
}

fn constant(model: &Btor2Model, id: NodeId) -> Option<u64> {
    match model.nodes().get(&id)?.kind {
        NodeKind::Constant(value) => Some(value),
        _ => None,
    }
}

fn zero_constant(model: &Btor2Model, id: NodeId) -> bool {
    constant(model, id) == Some(0)
}

fn reset_advance(model: &Btor2Model, state: NodeId, reset: NodeId) -> Option<NodeId> {
    let next = model.next_value(state)?;
    match model.nodes().get(&next)?.kind {
        NodeKind::Ite(condition, reset_value, advance)
            if condition == reset && zero_constant(model, reset_value) =>
        {
            Some(advance)
        }
        _ => None,
    }
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

fn recognise_control(
    model: &Btor2Model,
    expression: NodeId,
    braking: NodeId,
    velocity: NodeId,
) -> Option<u64> {
    let (left, right) = match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Or, left, right) => (left, right),
        _ => return None,
    };
    let guard = if left == braking {
        right
    } else if right == braking {
        left
    } else {
        return None;
    };
    match model.nodes().get(&guard)?.kind {
        NodeKind::Binary(BinaryOp::Ugte, state, threshold) if state == velocity => {
            constant(model, threshold).filter(|value| *value != 0)
        }
        _ => None,
    }
}

fn recognise_brake(model: &Btor2Model, expression: NodeId, velocity: NodeId) -> Option<u64> {
    let (guard, subtraction) = match model.nodes().get(&expression)?.kind {
        NodeKind::Ite(guard, zero, subtraction) if zero_constant(model, zero) => {
            (guard, subtraction)
        }
        _ => return None,
    };
    let literal = match model.nodes().get(&subtraction)?.kind {
        NodeKind::Binary(BinaryOp::Sub, state, literal) if state == velocity => literal,
        _ => return None,
    };
    let deceleration = constant(model, literal).filter(|value| *value != 0)?;
    match model.nodes().get(&guard)?.kind {
        NodeKind::Binary(BinaryOp::Ulte, state, bound)
            if state == velocity && constant(model, bound) == Some(deceleration) =>
        {
            Some(deceleration)
        }
        _ => None,
    }
}

fn shape_for_states(
    model: &Btor2Model,
    reset: NodeId,
    velocity: NodeId,
    position: NodeId,
    braking: NodeId,
) -> Option<Shape> {
    let width = model.nodes().get(&velocity)?.width;
    if width != model.nodes().get(&position)?.width
        || model.nodes().get(&braking)?.width != 1
        || !zero_constant(model, model.initialiser(velocity)?)
        || !zero_constant(model, model.initialiser(position)?)
        || !zero_constant(model, model.initialiser(braking)?)
    {
        return None;
    }
    let braking_advance = reset_advance(model, braking, reset)?;
    let brake_velocity = recognise_control(model, braking_advance, braking, velocity)?;
    let velocity_advance = reset_advance(model, velocity, reset)?;
    let (braking_velocity, accelerating_velocity) = match model.nodes().get(&velocity_advance)?.kind
    {
        NodeKind::Ite(control, braking_velocity, accelerating_velocity)
            if control == braking_advance =>
        {
            (braking_velocity, accelerating_velocity)
        }
        _ => return None,
    };
    let deceleration = recognise_brake(model, braking_velocity, velocity)?;
    let acceleration =
        add_literal(model, accelerating_velocity, velocity).filter(|value| *value != 0)?;
    let position_advance = reset_advance(model, position, reset)?;
    if !is_sum(model, position_advance, position, velocity) {
        return None;
    }
    Some(Shape {
        reset,
        velocity,
        position,
        braking,
        width,
        acceleration,
        brake_velocity,
        deceleration,
    })
}

fn recognise_shape(model: &Btor2Model) -> Option<Shape> {
    if model.states().len() != 3 || model.inputs().len() != 1 || !model.constraints().is_empty() {
        return None;
    }
    let reset = model.inputs()[0];
    if model.nodes().get(&reset)?.width != 1 {
        return None;
    }
    let mut candidates = Vec::new();
    for braking in model.states() {
        if model.nodes().get(braking)?.width != 1 {
            continue;
        }
        let words: Vec<_> = model
            .states()
            .iter()
            .copied()
            .filter(|state| state != braking)
            .collect();
        if words.len() != 2 {
            continue;
        }
        for (velocity, position) in [(words[0], words[1]), (words[1], words[0])] {
            if let Some(shape) = shape_for_states(model, reset, velocity, position, *braking) {
                candidates.push(shape);
            }
        }
    }
    (candidates.len() == 1).then(|| candidates[0])
}

fn position_threshold(model: &Btor2Model, bad_property: NodeId, position: NodeId) -> Option<u64> {
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))?;
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Ugte, state, threshold) if state == position => {
            constant(model, threshold)
        }
        _ => None,
    }
}

fn ceil_div(numerator: u128, denominator: u128) -> Option<u128> {
    numerator
        .checked_add(denominator.checked_sub(1)?)?
        .checked_div(denominator)
}

// Producer path: direct polynomial phase formulas.
fn produce_endpoint(shape: Shape, horizon: u64) -> Option<Endpoint> {
    let acceleration = u128::from(shape.acceleration);
    let deceleration = u128::from(shape.deceleration);
    let switch = ceil_div(u128::from(shape.brake_velocity), acceleration)?;
    let peak = acceleration.checked_mul(switch)?;
    let braking_steps = ceil_div(peak, deceleration)?;
    let stop = switch.checked_add(braking_steps)?;
    let h = u128::from(horizon);
    let acceleration_steps = h.min(switch);
    let acceleration_sum = acceleration
        .checked_mul(acceleration_steps.checked_mul(acceleration_steps.saturating_sub(1))?)?
        .checked_div(2)?;
    let braking_done = h.saturating_sub(switch).min(braking_steps);
    let braking_sum = braking_done.checked_mul(peak)?.checked_sub(
        deceleration
            .checked_mul(braking_done.checked_mul(braking_done.saturating_sub(1))?)?
            .checked_div(2)?,
    )?;
    let position = acceleration_sum.checked_add(braking_sum)?;
    let velocity = if h < switch {
        acceleration.checked_mul(h)?
    } else {
        peak.saturating_sub(deceleration.checked_mul(braking_done)?)
    };
    let maximum_velocity = acceleration.checked_mul(h.min(switch))?;
    if position > mask(shape.width)
        || velocity > mask(shape.width)
        || maximum_velocity > mask(shape.width)
        || switch > u128::from(u64::MAX)
        || stop > u128::from(u64::MAX)
    {
        return None;
    }
    Some(Endpoint {
        velocity: velocity as u64,
        position: position as u64,
        max_velocity: maximum_velocity as u64,
        switch_frame: switch as u64,
        stop_frame: stop as u64,
    })
}

fn checked_average_sum(first: u128, last: u128, count: u128) -> Option<u128> {
    let pair = first.checked_add(last)?;
    if pair % 2 == 0 {
        pair.checked_div(2)?.checked_mul(count)
    } else if count.is_multiple_of(2) {
        count.checked_div(2)?.checked_mul(pair)
    } else {
        None
    }
}

// Checker path: boundary inequalities and first/last arithmetic-series sums.
fn verify_endpoint(shape: Shape, horizon: u64) -> Option<Endpoint> {
    let a = u128::from(shape.acceleration);
    let d = u128::from(shape.deceleration);
    let threshold = u128::from(shape.brake_velocity);
    let switch = ceil_div(threshold, a)?;
    let peak = switch.checked_mul(a)?;
    if switch == 0 || (switch - 1).checked_mul(a)? >= threshold || peak < threshold {
        return None;
    }
    let braking_steps = ceil_div(peak, d)?;
    if braking_steps == 0
        || (braking_steps - 1).checked_mul(d)? >= peak
        || braking_steps.checked_mul(d)? < peak
    {
        return None;
    }
    let stop = switch.checked_add(braking_steps)?;
    let h = u128::from(horizon);
    let acceleration_steps = h.min(switch);
    let last_acceleration_velocity = acceleration_steps.saturating_sub(1).checked_mul(a)?;
    let acceleration_sum = checked_average_sum(0, last_acceleration_velocity, acceleration_steps)?;
    let braking_done = h.saturating_sub(switch).min(braking_steps);
    let last_braking_velocity = if braking_done == 0 {
        0
    } else {
        peak.checked_sub(braking_done.saturating_sub(1).checked_mul(d)?)?
    };
    let braking_sum = if braking_done == 0 {
        0
    } else {
        checked_average_sum(peak, last_braking_velocity, braking_done)?
    };
    let position = acceleration_sum.checked_add(braking_sum)?;
    let velocity = if h < switch {
        h.checked_mul(a)?
    } else if braking_done == braking_steps {
        0
    } else {
        peak.checked_sub(braking_done.checked_mul(d)?)?
    };
    let maximum_velocity = h.min(switch).checked_mul(a)?;
    if position > mask(shape.width)
        || velocity > mask(shape.width)
        || maximum_velocity > mask(shape.width)
        || switch > u128::from(u64::MAX)
        || stop > u128::from(u64::MAX)
    {
        return None;
    }
    Some(Endpoint {
        velocity: velocity as u64,
        position: position as u64,
        max_velocity: maximum_velocity as u64,
        switch_frame: switch as u64,
        stop_frame: stop as u64,
    })
}

fn logical_state_count(horizon: u32, stop_frame: u64) -> Option<u64> {
    let h = u64::from(horizon);
    let capped = h.min(stop_frame);
    let prefix = capped
        .checked_add(1)?
        .checked_mul(capped.checked_add(2)?)?
        .checked_div(2)?;
    prefix.checked_add(
        h.saturating_sub(stop_frame)
            .checked_mul(stop_frame.checked_add(1)?)?,
    )
}

/// Produces an exact SAFE certificate for all reset schedules. `Ok(None)`
/// requires the unchanged exact-search fallback.
pub fn try_produce_safe(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<Option<BrakingCertificate>, BrakingError> {
    if horizon > MAX_BRAKING_HORIZON {
        return Err(reject("braking query horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let Some(shape) = recognise_shape(&model) else {
        return Ok(None);
    };
    let Some(threshold) = position_threshold(&model, bad_property, shape.position) else {
        return Ok(None);
    };
    let Some(endpoint) = produce_endpoint(shape, u64::from(horizon)) else {
        return Ok(None);
    };
    if endpoint.position >= threshold {
        return Ok(None);
    }
    Ok(Some(BrakingCertificate {
        source_sha256: digest(source),
        query_horizon: horizon,
        bad_property,
        reset_input: shape.reset,
        velocity_state: shape.velocity,
        position_state: shape.position,
        braking_state: shape.braking,
        width: shape.width,
        acceleration: shape.acceleration,
        brake_velocity: shape.brake_velocity,
        deceleration: shape.deceleration,
        position_threshold: threshold,
        switch_frame: endpoint.switch_frame,
        stop_frame: endpoint.stop_frame,
        max_velocity: endpoint.max_velocity,
        max_position: endpoint.position,
    }))
}

fn verify_shape(
    model: &Btor2Model,
    certificate: &BrakingCertificate,
) -> Result<Shape, BrakingError> {
    if model.states().len() != 3
        || model.inputs() != [certificate.reset_input]
        || !model.constraints().is_empty()
        || model.nodes()[&certificate.reset_input].width != 1
        || !model.states().contains(&certificate.velocity_state)
        || !model.states().contains(&certificate.position_state)
        || !model.states().contains(&certificate.braking_state)
        || certificate.velocity_state == certificate.position_state
        || certificate.velocity_state == certificate.braking_state
        || certificate.position_state == certificate.braking_state
    {
        return Err(reject(
            "source braking state vector does not match certificate",
        ));
    }
    let shape = shape_for_states(
        model,
        certificate.reset_input,
        certificate.velocity_state,
        certificate.position_state,
        certificate.braking_state,
    )
    .ok_or_else(|| reject("source braking recurrence is outside the certified language"))?;
    if shape.width != certificate.width
        || shape.acceleration != certificate.acceleration
        || shape.brake_velocity != certificate.brake_velocity
        || shape.deceleration != certificate.deceleration
    {
        return Err(reject("source braking constants do not match certificate"));
    }
    Ok(shape)
}

pub fn verify(
    source: &[u8],
    certificate: &BrakingCertificate,
) -> Result<BrakingSummary, BrakingError> {
    if certificate.source_sha256 != digest(source) {
        return Err(reject("braking certificate source digest mismatch"));
    }
    if certificate.query_horizon > MAX_BRAKING_HORIZON {
        return Err(reject("braking certificate horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let shape = verify_shape(&model, certificate)?;
    let threshold = position_threshold(&model, certificate.bad_property, shape.position)
        .ok_or_else(|| reject("braking bad property is outside the certified language"))?;
    if threshold != certificate.position_threshold {
        return Err(reject("braking bad threshold does not match certificate"));
    }
    let endpoint = verify_endpoint(shape, u64::from(certificate.query_horizon))
        .ok_or_else(|| reject("braking phase arithmetic is not exact"))?;
    if endpoint.switch_frame != certificate.switch_frame
        || endpoint.stop_frame != certificate.stop_frame
        || endpoint.max_velocity != certificate.max_velocity
        || endpoint.position != certificate.max_position
        || endpoint.position >= threshold
    {
        return Err(reject("braking certificate claim is not safe or exact"));
    }
    Ok(BrakingSummary {
        query_horizon: certificate.query_horizon,
        max_velocity: endpoint.max_velocity,
        max_position: endpoint.position,
        switch_frame: endpoint.switch_frame,
        stop_frame: endpoint.stop_frame,
        logical_reachable_states: logical_state_count(
            certificate.query_horizon,
            endpoint.stop_frame,
        )
        .ok_or_else(|| reject("braking logical-state count overflowed"))?,
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn encode(certificate: &BrakingCertificate) -> Result<String, BrakingError> {
    if !valid_digest(&certificate.source_sha256) {
        return Err(reject("braking source digest is not canonical"));
    }
    let text = format!(
        "braking_certificate_version={BRAKING_CERTIFICATE_VERSION}\nsource_sha256={}\nquery_horizon={}\nbad_property={}\nreset_input={}\nvelocity_state={}\nposition_state={}\nbraking_state={}\nwidth={}\nacceleration={}\nbrake_velocity={}\ndeceleration={}\nposition_threshold={}\nswitch_frame={}\nstop_frame={}\nmax_velocity={}\nmax_position={}\nresult=SAFE\nstatus=complete\n",
        certificate.source_sha256,
        certificate.query_horizon,
        certificate.bad_property,
        certificate.reset_input,
        certificate.velocity_state,
        certificate.position_state,
        certificate.braking_state,
        certificate.width,
        certificate.acceleration,
        certificate.brake_velocity,
        certificate.deceleration,
        certificate.position_threshold,
        certificate.switch_frame,
        certificate.stop_frame,
        certificate.max_velocity,
        certificate.max_position,
    );
    if text.len() > MAX_BRAKING_CERTIFICATE_BYTES {
        return Err(reject("encoded braking certificate exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode(bytes: &[u8]) -> Result<BrakingCertificate, BrakingError> {
    if bytes.len() > MAX_BRAKING_CERTIFICATE_BYTES {
        return Err(reject("braking certificate exceeds byte limit"));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| reject("braking certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(
            "braking certificate must be canonical LF text without NUL",
        ));
    }
    let mut lines = text.lines();
    fn take(lines: &mut std::str::Lines<'_>, key: &str) -> Result<String, BrakingError> {
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
    ) -> Result<T, BrakingError> {
        let parsed = value
            .parse::<T>()
            .map_err(|_| reject(format!("invalid {key}")))?;
        if parsed.to_string() != value {
            return Err(reject(format!("noncanonical {key}")));
        }
        Ok(parsed)
    }
    let version: u32 = number(take(&mut lines, "braking_certificate_version")?, "version")?;
    if version != BRAKING_CERTIFICATE_VERSION {
        return Err(reject("unsupported braking certificate version"));
    }
    let source_sha256 = take(&mut lines, "source_sha256")?;
    if !valid_digest(&source_sha256) {
        return Err(reject("braking source digest is not canonical"));
    }
    let certificate = BrakingCertificate {
        source_sha256,
        query_horizon: number(take(&mut lines, "query_horizon")?, "query horizon")?,
        bad_property: number(take(&mut lines, "bad_property")?, "bad property")?,
        reset_input: number(take(&mut lines, "reset_input")?, "reset input")?,
        velocity_state: number(take(&mut lines, "velocity_state")?, "velocity state")?,
        position_state: number(take(&mut lines, "position_state")?, "position state")?,
        braking_state: number(take(&mut lines, "braking_state")?, "braking state")?,
        width: number(take(&mut lines, "width")?, "width")?,
        acceleration: number(take(&mut lines, "acceleration")?, "acceleration")?,
        brake_velocity: number(take(&mut lines, "brake_velocity")?, "brake velocity")?,
        deceleration: number(take(&mut lines, "deceleration")?, "deceleration")?,
        position_threshold: number(
            take(&mut lines, "position_threshold")?,
            "position threshold",
        )?,
        switch_frame: number(take(&mut lines, "switch_frame")?, "switch frame")?,
        stop_frame: number(take(&mut lines, "stop_frame")?, "stop frame")?,
        max_velocity: number(take(&mut lines, "max_velocity")?, "max velocity")?,
        max_position: number(take(&mut lines, "max_position")?, "max position")?,
    };
    if certificate.query_horizon > MAX_BRAKING_HORIZON
        || take(&mut lines, "result")? != "SAFE"
        || take(&mut lines, "status")? != "complete"
        || lines.next().is_some()
    {
        return Err(reject(
            "braking certificate is incomplete or has trailing fields",
        ));
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btor2_search;

    const BRAKING: &[u8] = include_bytes!("../examples/btor2/braking-controller-v1.btor2");
    const MOTOR_STOP: &[u8] = include_bytes!("../examples/btor2/motor-emergency-stop-v1.btor2");
    const REJECTED: &[u8] =
        include_bytes!("../examples/btor2/semi-implicit-braking-rejected-v1.btor2");

    #[test]
    fn composes_acceleration_braking_and_stopped_regions() {
        let certificate = try_produce_safe(BRAKING, 31, 255).unwrap().unwrap();
        assert_eq!(certificate.switch_frame, 128);
        assert_eq!(certificate.stop_frame, 256);
        assert_eq!(certificate.max_velocity, 256);
        assert_eq!(certificate.max_position, 32_766);
        let encoded = encode(&certificate).unwrap();
        assert!(encoded.len() < 500);
        let summary = verify(BRAKING, &decode(encoded.as_bytes()).unwrap()).unwrap();
        assert_eq!(summary.logical_reachable_states, 32_896);
        assert!(try_produce_safe(BRAKING, 31, 256).unwrap().is_none());
    }

    #[test]
    fn producer_and_checker_phase_algorithms_agree() {
        for source in [BRAKING, MOTOR_STOP] {
            let shape = recognise_shape(&btor2::parse_bytes(source).unwrap()).unwrap();
            for horizon in 0..=300 {
                assert_eq!(
                    produce_endpoint(shape, horizon),
                    verify_endpoint(shape, horizon)
                );
            }
        }
    }

    #[test]
    fn reset_prefix_count_matches_complete_explicit_layers() {
        let source = b"1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 reset\n4 zero 2\n5 zero 1\n6 state 2 velocity\n7 state 2 position\n8 state 1 braking\n9 init 2 6 4\n10 init 2 7 4\n11 init 1 8 5\n12 constd 2 2\n13 constd 2 4\n14 ugte 1 6 13\n15 or 1 8 14\n16 constd 2 2\n17 ulte 1 6 16\n18 sub 2 6 16\n19 ite 2 17 4 18\n20 add 2 6 12\n21 ite 2 15 19 20\n22 add 2 7 6\n23 ite 2 3 4 21\n24 ite 2 3 4 22\n25 ite 1 3 5 15\n26 next 2 6 23\n27 next 2 7 24\n28 next 1 8 25\n29 constd 2 9\n30 ugte 1 7 29\n31 bad 30 stopping_envelope\n";
        let certificate = try_produce_safe(source, 31, 8).unwrap().unwrap();
        assert_eq!((certificate.switch_frame, certificate.stop_frame), (2, 4));
        let explicit = btor2_search::produce(source, 31, 8).unwrap();
        assert_eq!(
            explicit.layers.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 5, 5, 5, 5]
        );
        assert_eq!(
            explicit.layers.iter().map(Vec::len).sum::<usize>() as u64,
            logical_state_count(8, certificate.stop_frame).unwrap()
        );
    }

    #[test]
    fn rejects_semi_implicit_position_update() {
        assert!(try_produce_safe(REJECTED, 31, 127).unwrap().is_none());
    }

    #[test]
    fn rejects_tampering_source_drift_and_hostile_text() {
        let certificate = try_produce_safe(BRAKING, 31, 255).unwrap().unwrap();
        assert!(verify(REJECTED, &certificate).is_err());
        let mut tampered = certificate.clone();
        tampered.stop_frame += 1;
        assert!(verify(BRAKING, &tampered).is_err());
        let mut tampered = certificate.clone();
        tampered.position_state = tampered.velocity_state;
        assert!(verify(BRAKING, &tampered).is_err());
        let mut tampered = certificate;
        tampered.deceleration += 1;
        assert!(verify(BRAKING, &tampered).is_err());
        assert!(decode(b"braking_certificate_version=1\r\n").is_err());
        assert!(decode(&vec![b'x'; MAX_BRAKING_CERTIFICATE_BYTES + 1]).is_err());
    }

    #[test]
    fn every_single_byte_mutation_and_truncation_fails_closed() {
        let encoded = encode(&try_produce_safe(BRAKING, 31, 255).unwrap().unwrap())
            .unwrap()
            .into_bytes();
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode(&mutated) {
                assert!(verify(BRAKING, &certificate).is_err());
            }
        }
    }

    #[test]
    fn proves_a_billion_frame_stopped_controller_in_constant_space() {
        let source = String::from_utf8(BRAKING.to_vec())
            .unwrap()
            .replace("29 constd 2 32768", "29 constd 2 32769");
        let certificate = try_produce_safe(source.as_bytes(), 31, 1_000_000_000)
            .unwrap()
            .unwrap();
        assert_eq!(certificate.max_position, 32_768);
        assert!(encode(&certificate).unwrap().len() < 500);
        let summary = verify(source.as_bytes(), &certificate).unwrap();
        assert_eq!(summary.logical_reachable_states, 256_999_967_361);
    }
}
