//! Exact compressed SAFE certificates for a coupled velocity-position recurrence.

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

pub const MOTION_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_MOTION_HORIZON: u32 = 1_000_000_000;
pub const MAX_MOTION_CERTIFICATE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionCertificate {
    pub source_sha256: String,
    pub query_horizon: u32,
    pub bad_property: NodeId,
    pub input: NodeId,
    pub velocity_state: NodeId,
    pub position_state: NodeId,
    pub width: u32,
    pub initial_velocity: u64,
    pub initial_position: u64,
    pub acceleration: u64,
    pub velocity_threshold: u64,
    pub position_threshold: u64,
    pub max_velocity: u64,
    pub max_position: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionSummary {
    pub query_horizon: u32,
    pub max_velocity: u64,
    pub max_position: u64,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionError(pub String);

impl fmt::Display for MotionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for MotionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Shape {
    input: NodeId,
    velocity: NodeId,
    position: NodeId,
    width: u32,
    initial_velocity: u64,
    initial_position: u64,
    acceleration: u64,
}

fn reject(message: impl Into<String>) -> MotionError {
    MotionError(message.into())
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

fn reset_advance(model: &Btor2Model, state: NodeId, input: NodeId) -> Option<(u64, NodeId)> {
    let next = model.next_value(state)?;
    match model.nodes().get(&next)?.kind {
        NodeKind::Ite(condition, reset, advance) if condition == input => {
            Some((constant(model, reset)?, advance))
        }
        _ => None,
    }
}

fn self_add_literal(model: &Btor2Model, expression: NodeId, state: NodeId) -> Option<u64> {
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

fn recognise_shape(model: &Btor2Model) -> Option<Shape> {
    if model.states().len() != 2 || model.inputs().len() != 1 || !model.constraints().is_empty() {
        return None;
    }
    let input = model.inputs()[0];
    if model.nodes().get(&input)?.width != 1 {
        return None;
    }
    let mut candidates = Vec::new();
    for velocity in model.states() {
        let position = *model.states().iter().find(|state| *state != velocity)?;
        let width = model.nodes().get(velocity)?.width;
        if width != model.nodes().get(&position)?.width {
            continue;
        }
        let initial_velocity = constant(model, model.initialiser(*velocity)?)?;
        let initial_position = constant(model, model.initialiser(position)?)?;
        let (velocity_reset, velocity_advance) = reset_advance(model, *velocity, input)?;
        let (position_reset, position_advance) = reset_advance(model, position, input)?;
        if (initial_velocity, initial_position) != (velocity_reset, position_reset) {
            continue;
        }
        let Some(acceleration) =
            self_add_literal(model, velocity_advance, *velocity).filter(|value| *value != 0)
        else {
            continue;
        };
        if !is_sum(model, position_advance, position, *velocity) {
            continue;
        }
        candidates.push(Shape {
            input,
            velocity: *velocity,
            position,
            width,
            initial_velocity,
            initial_position,
            acceleration,
        });
    }
    (candidates.len() == 1).then(|| candidates[0])
}

fn unsigned_threshold(model: &Btor2Model, expression: NodeId, state: NodeId) -> Option<u64> {
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Ugte, left, right) if left == state => constant(model, right),
        _ => None,
    }
}

fn recognise_thresholds(
    model: &Btor2Model,
    bad_property: NodeId,
    shape: Shape,
) -> Option<(u64, u64)> {
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))?;
    let (left, right) = match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::And, left, right) => (left, right),
        _ => return None,
    };
    unsigned_threshold(model, left, shape.velocity)
        .zip(unsigned_threshold(model, right, shape.position))
        .or_else(|| {
            unsigned_threshold(model, right, shape.velocity).zip(unsigned_threshold(
                model,
                left,
                shape.position,
            ))
        })
}

fn values_at(shape: Shape, index: u64) -> Option<(u64, u64)> {
    let k = u128::from(index);
    let acceleration = u128::from(shape.acceleration);
    let initial_velocity = u128::from(shape.initial_velocity);
    let velocity = initial_velocity.checked_add(acceleration.checked_mul(k)?)?;
    let triangular = k.checked_mul(k.saturating_sub(1))?.checked_div(2)?;
    let position = u128::from(shape.initial_position)
        .checked_add(initial_velocity.checked_mul(k)?)?
        .checked_add(acceleration.checked_mul(triangular)?)?;
    let word_mask = u128::from(mask(shape.width));
    if velocity > word_mask || position > word_mask {
        return None;
    }
    Some((velocity as u64, position as u64))
}

type Matrix3 = [[u128; 3]; 3];

fn matrix_product(left: Matrix3, right: Matrix3) -> Option<Matrix3> {
    let mut product = [[0u128; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for inner in 0..3 {
                let term = left[row][inner].checked_mul(right[inner][column])?;
                product[row][column] = product[row][column].checked_add(term)?;
            }
        }
    }
    Some(product)
}

fn matrix_power(mut base: Matrix3, mut exponent: u64) -> Option<Matrix3> {
    let mut result = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = matrix_product(result, base)?;
        }
        exponent >>= 1;
        if exponent != 0 {
            base = matrix_product(base, base)?;
        }
    }
    Some(result)
}

fn independently_verified_values_at(shape: Shape, index: u64) -> Option<(u64, u64)> {
    let transition = [[1, 1, 0], [0, 1, u128::from(shape.acceleration)], [0, 0, 1]];
    let powered = matrix_power(transition, index)?;
    let initial = [
        u128::from(shape.initial_position),
        u128::from(shape.initial_velocity),
        1,
    ];
    let mut final_values = [0u128; 3];
    for row in 0..3 {
        for (column, value) in initial.iter().enumerate() {
            final_values[row] =
                final_values[row].checked_add(powered[row][column].checked_mul(*value)?)?;
        }
    }
    let word_mask = u128::from(mask(shape.width));
    let position = final_values[0];
    let velocity = final_values[1];
    if velocity > word_mask || position > word_mask || final_values[2] != 1 {
        return None;
    }
    Some((velocity as u64, position as u64))
}

fn logical_state_count(horizon: u32) -> Option<u64> {
    let layers = u64::from(horizon).checked_add(1)?;
    layers.checked_mul(layers.checked_add(1)?)?.checked_div(2)
}

/// Produces an exact compressed SAFE certificate for the admitted coupled
/// recurrence. `Ok(None)` requires the unchanged exact-search fallback.
pub fn try_produce_safe(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<Option<MotionCertificate>, MotionError> {
    if horizon > MAX_MOTION_HORIZON {
        return Err(reject("motion query horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let Some(shape) = recognise_shape(&model) else {
        return Ok(None);
    };
    let Some((velocity_threshold, position_threshold)) =
        recognise_thresholds(&model, bad_property, shape)
    else {
        return Ok(None);
    };
    let Some((max_velocity, max_position)) = values_at(shape, u64::from(horizon)) else {
        return Ok(None);
    };
    if max_velocity >= velocity_threshold && max_position >= position_threshold {
        return Ok(None);
    }
    Ok(Some(MotionCertificate {
        source_sha256: digest(source),
        query_horizon: horizon,
        bad_property,
        input: shape.input,
        velocity_state: shape.velocity,
        position_state: shape.position,
        width: shape.width,
        initial_velocity: shape.initial_velocity,
        initial_position: shape.initial_position,
        acceleration: shape.acceleration,
        velocity_threshold,
        position_threshold,
        max_velocity,
        max_position,
    }))
}

fn verify_shape(model: &Btor2Model, certificate: &MotionCertificate) -> Result<Shape, MotionError> {
    if model.states().len() != 2
        || !model.states().contains(&certificate.velocity_state)
        || !model.states().contains(&certificate.position_state)
        || certificate.velocity_state == certificate.position_state
        || model.inputs() != [certificate.input]
        || !model.constraints().is_empty()
    {
        return Err(reject(
            "source motion state vector does not match certificate",
        ));
    }
    let velocity_node = model
        .nodes()
        .get(&certificate.velocity_state)
        .ok_or_else(|| reject("motion velocity state is absent"))?;
    let position_node = model
        .nodes()
        .get(&certificate.position_state)
        .ok_or_else(|| reject("motion position state is absent"))?;
    if velocity_node.width != certificate.width
        || position_node.width != certificate.width
        || model.nodes()[&certificate.input].width != 1
    {
        return Err(reject("source motion widths do not match certificate"));
    }
    let initial_velocity = constant(
        model,
        model
            .initialiser(certificate.velocity_state)
            .ok_or_else(|| reject("velocity initialiser is absent"))?,
    )
    .ok_or_else(|| reject("velocity initialiser is not literal"))?;
    let initial_position = constant(
        model,
        model
            .initialiser(certificate.position_state)
            .ok_or_else(|| reject("position initialiser is absent"))?,
    )
    .ok_or_else(|| reject("position initialiser is not literal"))?;
    let (velocity_reset, velocity_advance) =
        reset_advance(model, certificate.velocity_state, certificate.input)
            .ok_or_else(|| reject("velocity next expression is outside motion language"))?;
    let (position_reset, position_advance) =
        reset_advance(model, certificate.position_state, certificate.input)
            .ok_or_else(|| reject("position next expression is outside motion language"))?;
    if (initial_velocity, initial_position) != (velocity_reset, position_reset)
        || (initial_velocity, initial_position)
            != (certificate.initial_velocity, certificate.initial_position)
    {
        return Err(reject("motion initial and reset values do not match"));
    }
    let acceleration = self_add_literal(model, velocity_advance, certificate.velocity_state)
        .filter(|value| *value != 0)
        .ok_or_else(|| reject("velocity advance is not nonzero literal addition"))?;
    if acceleration != certificate.acceleration
        || !is_sum(
            model,
            position_advance,
            certificate.position_state,
            certificate.velocity_state,
        )
    {
        return Err(reject(
            "source motion recurrence does not match certificate",
        ));
    }
    Ok(Shape {
        input: certificate.input,
        velocity: certificate.velocity_state,
        position: certificate.position_state,
        width: certificate.width,
        initial_velocity,
        initial_position,
        acceleration,
    })
}

fn verify_thresholds(
    model: &Btor2Model,
    certificate: &MotionCertificate,
) -> Result<(), MotionError> {
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == certificate.bad_property).then_some(*expression))
        .ok_or_else(|| reject("motion bad property is absent"))?;
    let (left, right) = match model.nodes()[&expression].kind {
        NodeKind::Binary(BinaryOp::And, left, right) => (left, right),
        _ => return Err(reject("motion bad property is not a conjunction")),
    };
    let expected = (
        certificate.velocity_threshold,
        certificate.position_threshold,
    );
    let direct = unsigned_threshold(model, left, certificate.velocity_state)
        .zip(unsigned_threshold(model, right, certificate.position_state));
    let reversed = unsigned_threshold(model, right, certificate.velocity_state)
        .zip(unsigned_threshold(model, left, certificate.position_state));
    if direct != Some(expected) && reversed != Some(expected) {
        return Err(reject("motion bad thresholds do not match certificate"));
    }
    Ok(())
}

pub fn verify(
    source: &[u8],
    certificate: &MotionCertificate,
) -> Result<MotionSummary, MotionError> {
    if certificate.source_sha256 != digest(source) {
        return Err(reject("motion certificate source digest mismatch"));
    }
    if certificate.query_horizon > MAX_MOTION_HORIZON {
        return Err(reject("motion certificate horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    let shape = verify_shape(&model, certificate)?;
    verify_thresholds(&model, certificate)?;
    let (max_velocity, max_position) =
        independently_verified_values_at(shape, u64::from(certificate.query_horizon))
            .ok_or_else(|| reject("motion recurrence wraps and is not exact"))?;
    if (max_velocity, max_position) != (certificate.max_velocity, certificate.max_position) {
        return Err(reject("motion endpoint does not match certificate"));
    }
    if max_velocity >= certificate.velocity_threshold
        && max_position >= certificate.position_threshold
    {
        return Err(reject("motion curve intersects the selected bad property"));
    }
    Ok(MotionSummary {
        query_horizon: certificate.query_horizon,
        max_velocity,
        max_position,
        logical_reachable_states: logical_state_count(certificate.query_horizon)
            .ok_or_else(|| reject("motion logical-state count overflowed"))?,
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn encode(certificate: &MotionCertificate) -> Result<String, MotionError> {
    if !valid_digest(&certificate.source_sha256) {
        return Err(reject("motion source digest is not canonical"));
    }
    let text = format!(
        "motion_certificate_version={MOTION_CERTIFICATE_VERSION}\nsource_sha256={}\nquery_horizon={}\nbad_property={}\ninput={}\nvelocity_state={}\nposition_state={}\nwidth={}\ninitial_velocity={}\ninitial_position={}\nacceleration={}\nvelocity_threshold={}\nposition_threshold={}\nmax_velocity={}\nmax_position={}\nresult=SAFE\nstatus=complete\n",
        certificate.source_sha256,
        certificate.query_horizon,
        certificate.bad_property,
        certificate.input,
        certificate.velocity_state,
        certificate.position_state,
        certificate.width,
        certificate.initial_velocity,
        certificate.initial_position,
        certificate.acceleration,
        certificate.velocity_threshold,
        certificate.position_threshold,
        certificate.max_velocity,
        certificate.max_position,
    );
    if text.len() > MAX_MOTION_CERTIFICATE_BYTES {
        return Err(reject("encoded motion certificate exceeds byte limit"));
    }
    Ok(text)
}

pub fn decode(bytes: &[u8]) -> Result<MotionCertificate, MotionError> {
    if bytes.len() > MAX_MOTION_CERTIFICATE_BYTES {
        return Err(reject("motion certificate exceeds byte limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("motion certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(
            "motion certificate must be canonical LF text without NUL",
        ));
    }
    let mut lines = text.lines();
    fn take(lines: &mut std::str::Lines<'_>, key: &str) -> Result<String, MotionError> {
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
    ) -> Result<T, MotionError> {
        let parsed = value
            .parse::<T>()
            .map_err(|_| reject(format!("invalid {key}")))?;
        if parsed.to_string() != value {
            return Err(reject(format!("noncanonical {key}")));
        }
        Ok(parsed)
    }
    let version: u32 = number(take(&mut lines, "motion_certificate_version")?, "version")?;
    if version != MOTION_CERTIFICATE_VERSION {
        return Err(reject("unsupported motion certificate version"));
    }
    let source_sha256 = take(&mut lines, "source_sha256")?;
    if !valid_digest(&source_sha256) {
        return Err(reject("motion source digest is not canonical"));
    }
    let query_horizon = number(take(&mut lines, "query_horizon")?, "query horizon")?;
    if query_horizon > MAX_MOTION_HORIZON {
        return Err(reject("motion query horizon exceeds limit"));
    }
    let bad_property = number(take(&mut lines, "bad_property")?, "bad property")?;
    let input = number(take(&mut lines, "input")?, "input")?;
    let velocity_state = number(take(&mut lines, "velocity_state")?, "velocity state")?;
    let position_state = number(take(&mut lines, "position_state")?, "position state")?;
    let width = number(take(&mut lines, "width")?, "width")?;
    let initial_velocity = number(take(&mut lines, "initial_velocity")?, "initial velocity")?;
    let initial_position = number(take(&mut lines, "initial_position")?, "initial position")?;
    let acceleration = number(take(&mut lines, "acceleration")?, "acceleration")?;
    let velocity_threshold = number(
        take(&mut lines, "velocity_threshold")?,
        "velocity threshold",
    )?;
    let position_threshold = number(
        take(&mut lines, "position_threshold")?,
        "position threshold",
    )?;
    let max_velocity = number(take(&mut lines, "max_velocity")?, "max velocity")?;
    let max_position = number(take(&mut lines, "max_position")?, "max position")?;
    if take(&mut lines, "result")? != "SAFE"
        || take(&mut lines, "status")? != "complete"
        || lines.next().is_some()
    {
        return Err(reject(
            "motion certificate is incomplete or has trailing fields",
        ));
    }
    Ok(MotionCertificate {
        source_sha256,
        query_horizon,
        bad_property,
        input,
        velocity_state,
        position_state,
        width,
        initial_velocity,
        initial_position,
        acceleration,
        velocity_threshold,
        position_threshold,
        max_velocity,
        max_position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOTION: &[u8] = include_bytes!("../examples/btor2/motion-envelope-v1.btor2");
    const SERVO: &[u8] = include_bytes!("../examples/btor2/servo-motion-envelope-v1.btor2");
    const REJECTED: &[u8] =
        include_bytes!("../examples/btor2/semi-implicit-motion-rejected-v1.btor2");

    #[test]
    fn proves_two_coupled_safe_boundaries_and_round_trips() {
        for (source, horizon, velocity, position, states) in [
            (MOTION, 200, 200, 19_900, 20_301),
            (SERVO, 128, 256, 16_256, 8_385),
        ] {
            let certificate = try_produce_safe(source, 21, horizon).unwrap().unwrap();
            assert_eq!(
                (certificate.max_velocity, certificate.max_position),
                (velocity, position)
            );
            let encoded = encode(&certificate).unwrap();
            let summary = verify(source, &decode(encoded.as_bytes()).unwrap()).unwrap();
            assert_eq!(summary.logical_reachable_states, states);
        }
    }

    #[test]
    fn producer_closed_form_matches_checker_matrix_power() {
        let model = btor2::parse_bytes(SERVO).unwrap();
        let shape = recognise_shape(&model).unwrap();
        for index in 0..=129 {
            assert_eq!(
                values_at(shape, index),
                independently_verified_values_at(shape, index)
            );
        }
        let nonzero = Shape {
            input: 1,
            velocity: 2,
            position: 3,
            width: 16,
            initial_velocity: 3,
            initial_position: 7,
            acceleration: 2,
        };
        for index in 0..=100 {
            assert_eq!(
                values_at(nonzero, index),
                independently_verified_values_at(nonzero, index)
            );
        }
    }

    #[test]
    fn defers_intersections_and_near_neighbours_to_exact_search() {
        assert!(try_produce_safe(MOTION, 21, 201).unwrap().is_none());
        assert!(try_produce_safe(SERVO, 21, 129).unwrap().is_none());
        assert!(try_produce_safe(REJECTED, 21, 3).unwrap().is_none());
    }

    #[test]
    fn rejects_claim_mutation_source_drift_and_hostile_encoding() {
        let certificate = try_produce_safe(MOTION, 21, 200).unwrap().unwrap();
        assert!(verify(SERVO, &certificate).is_err());
        let mut tampered = certificate.clone();
        tampered.max_position += 1;
        assert!(verify(MOTION, &tampered).is_err());
        let mut tampered = certificate;
        tampered.acceleration += 1;
        assert!(verify(MOTION, &tampered).is_err());
        assert!(decode(b"motion_certificate_version=1\r\n").is_err());
        assert!(decode(&vec![b'x'; MAX_MOTION_CERTIFICATE_BYTES + 1]).is_err());
    }

    #[test]
    fn every_single_byte_mutation_and_truncation_fails_closed() {
        let encoded = encode(&try_produce_safe(MOTION, 21, 200).unwrap().unwrap())
            .unwrap()
            .into_bytes();
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode(&mutated) {
                assert!(verify(MOTION, &certificate).is_err());
            }
        }
    }

    #[test]
    fn verifies_a_billion_frame_motion_curve_in_constant_artifact_space() {
        let source = b"1 sort bitvec 1\n2 sort bitvec 64\n3 input 1 brake\n4 zero 2\n5 state 2 velocity\n6 state 2 position\n7 init 2 5 4\n8 init 2 6 4\n9 one 2\n10 add 2 5 9\n11 add 2 6 5\n12 ite 2 3 4 10\n13 ite 2 3 4 11\n14 next 2 5 12\n15 next 2 6 13\n16 constd 2 2000000000\n17 ugte 1 5 16\n18 constd 2 18000000000000000000\n19 ugte 1 6 18\n20 and 1 17 19\n21 bad 20 distant_envelope\n";
        let certificate = try_produce_safe(source, 21, 1_000_000_000)
            .unwrap()
            .unwrap();
        let encoded = encode(&certificate).unwrap();
        assert!(encoded.len() < 500);
        let summary = verify(source, &decode(encoded.as_bytes()).unwrap()).unwrap();
        assert_eq!(summary.max_velocity, 1_000_000_000);
        assert_eq!(summary.max_position, 499_999_999_500_000_000);
        assert_eq!(summary.logical_reachable_states, 500_000_001_500_000_001);
        assert!(try_produce_safe(source, 21, MAX_MOTION_HORIZON + 1).is_err());
    }
}
