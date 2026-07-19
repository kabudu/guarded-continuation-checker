//! Canonical, source-bound artifacts for proof-carrying controller/plant batches.

use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::aiger_obligation::AigerTransition;
use crate::controller_mtbdd::{
    ControllerMtbddAdmissionFailure, ControllerMtbddArtifact, decode_controller_mtbdd,
    encode_controller_mtbdd, produce_controller_mtbdd,
};
use crate::controller_plant::{
    ControllerPlantAnswer, ControllerPlantBatchInput, ControllerPlantBatchResult,
    ControllerPlantResult, ControllerPlantTraceStep, ControllerPlantWiring,
    MAX_COMPOSITION_HORIZON, compose_controller_plant_batch, compose_controller_plant_direct_batch,
    compose_verified_mtbdd_plant, verify_mtbdd_for_composition,
};
use crate::controller_transducer::{
    ControllerTransducerObligation, decode_controller_transducer, encode_controller_transducer,
};

pub const CONTROLLER_PLANT_ARTIFACT_VERSION: u32 = 1;
pub const MAX_CONTROLLER_PLANT_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS: usize = 64;
const MAGIC: &[u8; 8] = b"GCCCPA01";
pub const MTBDD_PLANT_ARTIFACT_VERSION: u32 = 1;
const MTBDD_MAGIC: &[u8; 8] = b"GCCMPA01";
pub const DIRECT_PLANT_ARTIFACT_VERSION: u32 = 1;
const DIRECT_MAGIC: &[u8; 8] = b"GCCDPA01";
pub const MTBDD_PLANT_PORTFOLIO_VERSION: u32 = 1;
const PORTFOLIO_MAGIC: &[u8; 8] = b"GCCMPP01";

#[derive(Clone, Copy, Debug)]
pub struct ControllerPlantArtifactInput<'a> {
    pub plant: &'a AigerTransition,
    pub plant_source_sha256: [u8; 32],
    pub wiring: &'a ControllerPlantWiring,
    pub initial_controller_state: usize,
    pub initial_plant_state: usize,
    pub bad_plant_output: usize,
    pub horizon: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantArtifactMember {
    pub plant_source_sha256: [u8; 32],
    pub wiring: ControllerPlantWiring,
    pub initial_controller_state: usize,
    pub initial_plant_state: usize,
    pub bad_plant_output: usize,
    pub horizon: usize,
    pub result: ControllerPlantResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantBatchArtifact {
    pub version: u32,
    pub controller_transducer: Vec<u8>,
    pub members: Vec<ControllerPlantArtifactMember>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddPlantBatchArtifact {
    pub version: u32,
    pub controller_mtbdd: Vec<u8>,
    pub members: Vec<ControllerPlantArtifactMember>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerDirectPlantBatchArtifact {
    pub version: u32,
    pub controller_source_sha256: [u8; 32],
    pub members: Vec<ControllerPlantArtifactMember>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerMtbddPlantPortfolioBackend {
    Mtbdd,
    DirectExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerMtbddPlantSelectionReason {
    MtbddAdmitted,
    BoundaryLimit,
    TerminalLimit,
    NodeLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddPlantPortfolioArtifact {
    pub version: u32,
    pub backend: ControllerMtbddPlantPortfolioBackend,
    pub reason: ControllerMtbddPlantSelectionReason,
    pub relevant_inputs: Vec<usize>,
    pub observed_outputs: Vec<usize>,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddPlantPortfolioSummary {
    pub backend: ControllerMtbddPlantPortfolioBackend,
    pub reason: ControllerMtbddPlantSelectionReason,
    pub members: Vec<ControllerPlantResult>,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddPlantBatchSummary {
    pub members: Vec<ControllerPlantResult>,
    pub safe: usize,
    pub unsafe_count: usize,
    pub mtbdd_nodes: usize,
    pub mtbdd_terminals: usize,
    pub assignments_checked: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantArtifactError(pub String);

impl fmt::Display for ControllerPlantArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ControllerPlantArtifactError {}

fn reject(message: impl Into<String>) -> ControllerPlantArtifactError {
    ControllerPlantArtifactError(message.into())
}

fn narrow(value: usize, field: &str) -> Result<u32, ControllerPlantArtifactError> {
    u32::try_from(value).map_err(|_| reject(format!("{field} exceeds canonical range")))
}

fn put_vec(bytes: &mut Vec<u8>, values: &[usize]) -> Result<(), ControllerPlantArtifactError> {
    let count = u8::try_from(values.len()).map_err(|_| reject("wiring vector is too long"))?;
    bytes.push(count);
    for &value in values {
        bytes.push(u8::try_from(value).map_err(|_| reject("wiring index exceeds range"))?);
    }
    Ok(())
}

fn put_result(
    bytes: &mut Vec<u8>,
    result: &ControllerPlantResult,
) -> Result<(), ControllerPlantArtifactError> {
    bytes.push(match result.answer {
        ControllerPlantAnswer::Safe => 0,
        ControllerPlantAnswer::Unsafe => 1,
    });
    bytes.extend_from_slice(&narrow(result.horizon, "result horizon")?.to_le_bytes());
    bytes.extend_from_slice(
        &result
            .bad_frame
            .map(|value| narrow(value, "bad frame"))
            .transpose()?
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    bytes.extend_from_slice(
        &narrow(result.reachable_product_states, "reachable states")?.to_le_bytes(),
    );
    bytes.extend_from_slice(
        &narrow(result.explored_transitions, "explored transitions")?.to_le_bytes(),
    );
    bytes.extend_from_slice(&narrow(result.trace.len(), "trace length")?.to_le_bytes());
    for step in &result.trace {
        for (value, field) in [
            (step.frame, "trace frame"),
            (step.controller_state, "controller state"),
            (step.plant_state, "plant state"),
            (step.sensor_pattern, "sensor pattern"),
            (step.action_pattern, "action pattern"),
        ] {
            bytes.extend_from_slice(&narrow(value, field)?.to_le_bytes());
        }
        bytes.extend_from_slice(&step.controller_input.to_le_bytes());
        bytes.extend_from_slice(&step.plant_input.to_le_bytes());
        bytes.push(u8::from(step.bad));
    }
    Ok(())
}

pub fn produce_controller_plant_artifact(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    controller: &ControllerTransducerObligation,
    members: &[ControllerPlantArtifactInput<'_>],
) -> Result<ControllerPlantBatchArtifact, ControllerPlantArtifactError> {
    let batch_inputs = members
        .iter()
        .map(|member| ControllerPlantBatchInput {
            plant: member.plant,
            wiring: member.wiring,
            initial_controller_state: member.initial_controller_state,
            initial_plant_state: member.initial_plant_state,
            bad_plant_output: member.bad_plant_output,
            horizon: member.horizon,
        })
        .collect::<Vec<_>>();
    let batch = compose_controller_plant_batch(
        controller_model,
        controller_source_sha256,
        controller,
        &batch_inputs,
    )
    .map_err(|error| reject(error.to_string()))?;
    let artifact_members = members
        .iter()
        .zip(batch.members)
        .map(|(member, result)| ControllerPlantArtifactMember {
            plant_source_sha256: member.plant_source_sha256,
            wiring: member.wiring.clone(),
            initial_controller_state: member.initial_controller_state,
            initial_plant_state: member.initial_plant_state,
            bad_plant_output: member.bad_plant_output,
            horizon: member.horizon,
            result,
        })
        .collect();
    Ok(ControllerPlantBatchArtifact {
        version: CONTROLLER_PLANT_ARTIFACT_VERSION,
        controller_transducer: encode_controller_transducer(controller)
            .map_err(|error| reject(error.to_string()))?,
        members: artifact_members,
    })
}

pub fn encode_controller_plant_artifact(
    artifact: &ControllerPlantBatchArtifact,
) -> Result<Vec<u8>, ControllerPlantArtifactError> {
    if artifact.version != CONTROLLER_PLANT_ARTIFACT_VERSION
        || artifact.members.is_empty()
        || artifact.members.len() > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
    {
        return Err(reject("controller-plant artifact dimensions are invalid"));
    }
    decode_controller_transducer(&artifact.controller_transducer)
        .map_err(|error| reject(error.to_string()))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(
        &narrow(
            artifact.controller_transducer.len(),
            "controller artifact length",
        )?
        .to_le_bytes(),
    );
    bytes.extend_from_slice(&artifact.controller_transducer);
    bytes.push(artifact.members.len() as u8);
    for member in &artifact.members {
        bytes.extend_from_slice(&member.plant_source_sha256);
        put_vec(&mut bytes, &member.wiring.controller_sensor_inputs)?;
        put_vec(&mut bytes, &member.wiring.controller_action_outputs)?;
        put_vec(&mut bytes, &member.wiring.plant_sensor_outputs)?;
        put_vec(&mut bytes, &member.wiring.plant_action_inputs)?;
        for (value, field) in [
            (member.initial_controller_state, "initial controller state"),
            (member.initial_plant_state, "initial plant state"),
            (member.bad_plant_output, "bad plant output"),
            (member.horizon, "member horizon"),
        ] {
            bytes.extend_from_slice(&narrow(value, field)?.to_le_bytes());
        }
        put_result(&mut bytes, &member.result)?;
    }
    let integrity = Sha256::digest(&bytes);
    bytes.extend_from_slice(&integrity);
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES {
        return Err(reject("controller-plant artifact exceeds byte limit"));
    }
    Ok(bytes)
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    count: usize,
) -> Result<&'a [u8], ControllerPlantArtifactError> {
    let end = cursor
        .checked_add(count)
        .ok_or_else(|| reject("controller-plant artifact cursor overflow"))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| reject("controller-plant artifact is truncated"))?;
    *cursor = end;
    Ok(value)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, ControllerPlantArtifactError> {
    Ok(take(bytes, cursor, 1)?[0])
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, ControllerPlantArtifactError> {
    Ok(u32::from_le_bytes(
        take(bytes, cursor, 4)?
            .try_into()
            .map_err(|_| reject("controller-plant u32 decode failed"))?,
    ))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, ControllerPlantArtifactError> {
    Ok(u64::from_le_bytes(
        take(bytes, cursor, 8)?
            .try_into()
            .map_err(|_| reject("controller-plant u64 decode failed"))?,
    ))
}

fn read_vec(bytes: &[u8], cursor: &mut usize) -> Result<Vec<usize>, ControllerPlantArtifactError> {
    let count = read_u8(bytes, cursor)? as usize;
    Ok(take(bytes, cursor, count)?
        .iter()
        .map(|&value| value as usize)
        .collect())
}

fn read_result(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<ControllerPlantResult, ControllerPlantArtifactError> {
    let answer = match read_u8(bytes, cursor)? {
        0 => ControllerPlantAnswer::Safe,
        1 => ControllerPlantAnswer::Unsafe,
        _ => return Err(reject("controller-plant answer is invalid")),
    };
    let horizon = read_u32(bytes, cursor)? as usize;
    let bad_raw = read_u32(bytes, cursor)?;
    let bad_frame = (bad_raw != u32::MAX).then_some(bad_raw as usize);
    let reachable_product_states = read_u32(bytes, cursor)? as usize;
    let explored_transitions = read_u32(bytes, cursor)? as usize;
    let trace_count = read_u32(bytes, cursor)? as usize;
    if horizon > MAX_COMPOSITION_HORIZON || trace_count > horizon.saturating_add(1) {
        return Err(reject("controller-plant result dimensions exceed limits"));
    }
    let mut trace = Vec::with_capacity(trace_count);
    for _ in 0..trace_count {
        let frame = read_u32(bytes, cursor)? as usize;
        let controller_state = read_u32(bytes, cursor)? as usize;
        let plant_state = read_u32(bytes, cursor)? as usize;
        let sensor_pattern = read_u32(bytes, cursor)? as usize;
        let action_pattern = read_u32(bytes, cursor)? as usize;
        let controller_input = read_u64(bytes, cursor)?;
        let plant_input = read_u64(bytes, cursor)?;
        let bad = match read_u8(bytes, cursor)? {
            0 => false,
            1 => true,
            _ => return Err(reject("controller-plant trace flag is invalid")),
        };
        trace.push(ControllerPlantTraceStep {
            frame,
            controller_state,
            plant_state,
            sensor_pattern,
            action_pattern,
            controller_input,
            plant_input,
            bad,
        });
    }
    Ok(ControllerPlantResult {
        version: crate::controller_plant::CONTROLLER_PLANT_VERSION,
        answer,
        horizon,
        bad_frame,
        reachable_product_states,
        explored_transitions,
        trace,
    })
}

pub fn decode_controller_plant_artifact(
    bytes: &[u8],
) -> Result<ControllerPlantBatchArtifact, ControllerPlantArtifactError> {
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES || bytes.len() < 32 {
        return Err(reject("controller-plant artifact size is invalid"));
    }
    let payload_len = bytes.len() - 32;
    let (payload, claimed_integrity) = bytes.split_at(payload_len);
    if Sha256::digest(payload).as_slice() != claimed_integrity {
        return Err(reject("controller-plant artifact integrity mismatch"));
    }
    let mut cursor = 0usize;
    if take(payload, &mut cursor, MAGIC.len())? != MAGIC {
        return Err(reject("controller-plant artifact magic mismatch"));
    }
    let version = read_u32(payload, &mut cursor)?;
    if version != CONTROLLER_PLANT_ARTIFACT_VERSION {
        return Err(reject("controller-plant artifact version mismatch"));
    }
    let controller_len = read_u32(payload, &mut cursor)? as usize;
    let controller_transducer = take(payload, &mut cursor, controller_len)?.to_vec();
    decode_controller_transducer(&controller_transducer)
        .map_err(|error| reject(error.to_string()))?;
    let member_count = read_u8(payload, &mut cursor)? as usize;
    if member_count == 0 || member_count > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS {
        return Err(reject("controller-plant member count is outside limit"));
    }
    let mut members = Vec::with_capacity(member_count);
    for _ in 0..member_count {
        let plant_source_sha256 = take(payload, &mut cursor, 32)?
            .try_into()
            .map_err(|_| reject("plant digest decode failed"))?;
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: read_vec(payload, &mut cursor)?,
            controller_action_outputs: read_vec(payload, &mut cursor)?,
            plant_sensor_outputs: read_vec(payload, &mut cursor)?,
            plant_action_inputs: read_vec(payload, &mut cursor)?,
        };
        let initial_controller_state = read_u32(payload, &mut cursor)? as usize;
        let initial_plant_state = read_u32(payload, &mut cursor)? as usize;
        let bad_plant_output = read_u32(payload, &mut cursor)? as usize;
        let horizon = read_u32(payload, &mut cursor)? as usize;
        let result = read_result(payload, &mut cursor)?;
        members.push(ControllerPlantArtifactMember {
            plant_source_sha256,
            wiring,
            initial_controller_state,
            initial_plant_state,
            bad_plant_output,
            horizon,
            result,
        });
    }
    if cursor != payload.len() {
        return Err(reject("controller-plant artifact has trailing bytes"));
    }
    Ok(ControllerPlantBatchArtifact {
        version,
        controller_transducer,
        members,
    })
}

pub fn verify_controller_plant_artifact(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    plants: &[(&AigerTransition, [u8; 32])],
    artifact_bytes: &[u8],
) -> Result<ControllerPlantBatchResult, ControllerPlantArtifactError> {
    let artifact = decode_controller_plant_artifact(artifact_bytes)?;
    if plants.len() != artifact.members.len() {
        return Err(reject("controller-plant source count mismatch"));
    }
    let controller = decode_controller_transducer(&artifact.controller_transducer)
        .map_err(|error| reject(error.to_string()))?;
    let inputs = plants
        .iter()
        .zip(&artifact.members)
        .map(|(&(plant, digest), member)| {
            if digest != member.plant_source_sha256 {
                return Err(reject("controller-plant source digest mismatch"));
            }
            Ok(ControllerPlantBatchInput {
                plant,
                wiring: &member.wiring,
                initial_controller_state: member.initial_controller_state,
                initial_plant_state: member.initial_plant_state,
                bad_plant_output: member.bad_plant_output,
                horizon: member.horizon,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let verified = compose_controller_plant_batch(
        controller_model,
        controller_source_sha256,
        &controller,
        &inputs,
    )
    .map_err(|error| reject(error.to_string()))?;
    if verified
        .members
        .iter()
        .zip(&artifact.members)
        .any(|(actual, claimed)| actual != &claimed.result)
    {
        return Err(reject("controller-plant member result mismatch"));
    }
    Ok(verified)
}

pub fn produce_controller_mtbdd_plant_artifact(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    controller: &ControllerMtbddArtifact,
    members: &[ControllerPlantArtifactInput<'_>],
) -> Result<ControllerMtbddPlantBatchArtifact, ControllerPlantArtifactError> {
    if members.is_empty() || members.len() > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS {
        return Err(reject(
            "controller MTBDD plant member count is outside limit",
        ));
    }
    let verified =
        verify_mtbdd_for_composition(controller_model, controller_source_sha256, controller)
            .map_err(|error| reject(error.to_string()))?;
    let artifact_members = members
        .iter()
        .map(|member| {
            let result = compose_verified_mtbdd_plant(
                &verified,
                member.plant,
                member.wiring,
                member.initial_controller_state,
                member.initial_plant_state,
                member.bad_plant_output,
                member.horizon,
            )
            .map_err(|error| reject(error.to_string()))?;
            Ok(ControllerPlantArtifactMember {
                plant_source_sha256: member.plant_source_sha256,
                wiring: member.wiring.clone(),
                initial_controller_state: member.initial_controller_state,
                initial_plant_state: member.initial_plant_state,
                bad_plant_output: member.bad_plant_output,
                horizon: member.horizon,
                result,
            })
        })
        .collect::<Result<Vec<_>, ControllerPlantArtifactError>>()?;
    Ok(ControllerMtbddPlantBatchArtifact {
        version: MTBDD_PLANT_ARTIFACT_VERSION,
        controller_mtbdd: encode_controller_mtbdd(controller)
            .map_err(|error| reject(error.to_string()))?,
        members: artifact_members,
    })
}

pub fn encode_controller_mtbdd_plant_artifact(
    artifact: &ControllerMtbddPlantBatchArtifact,
) -> Result<Vec<u8>, ControllerPlantArtifactError> {
    if artifact.version != MTBDD_PLANT_ARTIFACT_VERSION
        || artifact.members.is_empty()
        || artifact.members.len() > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
    {
        return Err(reject(
            "controller MTBDD plant artifact dimensions are invalid",
        ));
    }
    decode_controller_mtbdd(&artifact.controller_mtbdd)
        .map_err(|error| reject(error.to_string()))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MTBDD_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(
        &narrow(artifact.controller_mtbdd.len(), "controller MTBDD length")?.to_le_bytes(),
    );
    bytes.extend_from_slice(&artifact.controller_mtbdd);
    bytes.push(artifact.members.len() as u8);
    for member in &artifact.members {
        bytes.extend_from_slice(&member.plant_source_sha256);
        put_vec(&mut bytes, &member.wiring.controller_sensor_inputs)?;
        put_vec(&mut bytes, &member.wiring.controller_action_outputs)?;
        put_vec(&mut bytes, &member.wiring.plant_sensor_outputs)?;
        put_vec(&mut bytes, &member.wiring.plant_action_inputs)?;
        for (value, field) in [
            (member.initial_controller_state, "initial controller state"),
            (member.initial_plant_state, "initial plant state"),
            (member.bad_plant_output, "bad plant output"),
            (member.horizon, "member horizon"),
        ] {
            bytes.extend_from_slice(&narrow(value, field)?.to_le_bytes());
        }
        put_result(&mut bytes, &member.result)?;
    }
    let integrity = Sha256::digest(&bytes);
    bytes.extend_from_slice(&integrity);
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES {
        return Err(reject("controller MTBDD plant artifact exceeds byte limit"));
    }
    Ok(bytes)
}

pub fn decode_controller_mtbdd_plant_artifact(
    bytes: &[u8],
) -> Result<ControllerMtbddPlantBatchArtifact, ControllerPlantArtifactError> {
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES || bytes.len() < 32 {
        return Err(reject("controller MTBDD plant artifact size is invalid"));
    }
    let payload_len = bytes.len() - 32;
    let (payload, claimed_integrity) = bytes.split_at(payload_len);
    if Sha256::digest(payload).as_slice() != claimed_integrity {
        return Err(reject("controller MTBDD plant artifact integrity mismatch"));
    }
    let mut cursor = 0usize;
    if take(payload, &mut cursor, MTBDD_MAGIC.len())? != MTBDD_MAGIC {
        return Err(reject("controller MTBDD plant artifact magic mismatch"));
    }
    let version = read_u32(payload, &mut cursor)?;
    if version != MTBDD_PLANT_ARTIFACT_VERSION {
        return Err(reject("controller MTBDD plant artifact version mismatch"));
    }
    let controller_len = read_u32(payload, &mut cursor)? as usize;
    let controller_mtbdd = take(payload, &mut cursor, controller_len)?.to_vec();
    decode_controller_mtbdd(&controller_mtbdd).map_err(|error| reject(error.to_string()))?;
    let member_count = read_u8(payload, &mut cursor)? as usize;
    if member_count == 0 || member_count > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS {
        return Err(reject(
            "controller MTBDD plant member count is outside limit",
        ));
    }
    let mut members = Vec::with_capacity(member_count);
    for _ in 0..member_count {
        let plant_source_sha256 = take(payload, &mut cursor, 32)?
            .try_into()
            .map_err(|_| reject("plant digest decode failed"))?;
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: read_vec(payload, &mut cursor)?,
            controller_action_outputs: read_vec(payload, &mut cursor)?,
            plant_sensor_outputs: read_vec(payload, &mut cursor)?,
            plant_action_inputs: read_vec(payload, &mut cursor)?,
        };
        let initial_controller_state = read_u32(payload, &mut cursor)? as usize;
        let initial_plant_state = read_u32(payload, &mut cursor)? as usize;
        let bad_plant_output = read_u32(payload, &mut cursor)? as usize;
        let horizon = read_u32(payload, &mut cursor)? as usize;
        let result = read_result(payload, &mut cursor)?;
        members.push(ControllerPlantArtifactMember {
            plant_source_sha256,
            wiring,
            initial_controller_state,
            initial_plant_state,
            bad_plant_output,
            horizon,
            result,
        });
    }
    if cursor != payload.len() {
        return Err(reject("controller MTBDD plant artifact has trailing bytes"));
    }
    Ok(ControllerMtbddPlantBatchArtifact {
        version,
        controller_mtbdd,
        members,
    })
}

pub fn verify_controller_mtbdd_plant_artifact(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    plants: &[(&AigerTransition, [u8; 32])],
    artifact_bytes: &[u8],
) -> Result<ControllerMtbddPlantBatchSummary, ControllerPlantArtifactError> {
    let artifact = decode_controller_mtbdd_plant_artifact(artifact_bytes)?;
    if plants.len() != artifact.members.len() {
        return Err(reject("controller MTBDD plant source count mismatch"));
    }
    let controller = decode_controller_mtbdd(&artifact.controller_mtbdd)
        .map_err(|error| reject(error.to_string()))?;
    let verified =
        verify_mtbdd_for_composition(controller_model, controller_source_sha256, &controller)
            .map_err(|error| reject(error.to_string()))?;
    let mut members = Vec::with_capacity(plants.len());
    let mut safe = 0usize;
    let mut unsafe_count = 0usize;
    let mut reachable_product_states = 0usize;
    let mut explored_transitions = 0usize;
    for ((plant, digest), claimed) in plants.iter().copied().zip(&artifact.members) {
        if digest != claimed.plant_source_sha256 {
            return Err(reject("controller MTBDD plant source digest mismatch"));
        }
        let result = compose_verified_mtbdd_plant(
            &verified,
            plant,
            &claimed.wiring,
            claimed.initial_controller_state,
            claimed.initial_plant_state,
            claimed.bad_plant_output,
            claimed.horizon,
        )
        .map_err(|error| reject(error.to_string()))?;
        if result != claimed.result {
            return Err(reject("controller MTBDD plant member result mismatch"));
        }
        match result.answer {
            ControllerPlantAnswer::Safe => safe += 1,
            ControllerPlantAnswer::Unsafe => unsafe_count += 1,
        }
        reachable_product_states = reachable_product_states
            .checked_add(result.reachable_product_states)
            .ok_or_else(|| reject("controller MTBDD plant reachable count overflow"))?;
        explored_transitions = explored_transitions
            .checked_add(result.explored_transitions)
            .ok_or_else(|| reject("controller MTBDD plant transition count overflow"))?;
        members.push(result);
    }
    Ok(ControllerMtbddPlantBatchSummary {
        members,
        safe,
        unsafe_count,
        mtbdd_nodes: verified.summary().nodes,
        mtbdd_terminals: verified.summary().terminals,
        assignments_checked: verified.summary().assignments_checked,
        reachable_product_states,
        explored_transitions,
    })
}

pub fn produce_controller_direct_plant_artifact(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    members: &[ControllerPlantArtifactInput<'_>],
) -> Result<ControllerDirectPlantBatchArtifact, ControllerPlantArtifactError> {
    let inputs = members
        .iter()
        .map(|member| ControllerPlantBatchInput {
            plant: member.plant,
            wiring: member.wiring,
            initial_controller_state: member.initial_controller_state,
            initial_plant_state: member.initial_plant_state,
            bad_plant_output: member.bad_plant_output,
            horizon: member.horizon,
        })
        .collect::<Vec<_>>();
    let batch = compose_controller_plant_direct_batch(controller_model, &inputs)
        .map_err(|error| reject(error.to_string()))?;
    let artifact_members = members
        .iter()
        .zip(batch.members)
        .map(|(member, result)| ControllerPlantArtifactMember {
            plant_source_sha256: member.plant_source_sha256,
            wiring: member.wiring.clone(),
            initial_controller_state: member.initial_controller_state,
            initial_plant_state: member.initial_plant_state,
            bad_plant_output: member.bad_plant_output,
            horizon: member.horizon,
            result,
        })
        .collect();
    Ok(ControllerDirectPlantBatchArtifact {
        version: DIRECT_PLANT_ARTIFACT_VERSION,
        controller_source_sha256,
        members: artifact_members,
    })
}

pub fn encode_controller_direct_plant_artifact(
    artifact: &ControllerDirectPlantBatchArtifact,
) -> Result<Vec<u8>, ControllerPlantArtifactError> {
    if artifact.version != DIRECT_PLANT_ARTIFACT_VERSION
        || artifact.members.is_empty()
        || artifact.members.len() > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
    {
        return Err(reject(
            "controller direct plant artifact dimensions are invalid",
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(DIRECT_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.controller_source_sha256);
    bytes.push(artifact.members.len() as u8);
    for member in &artifact.members {
        bytes.extend_from_slice(&member.plant_source_sha256);
        put_vec(&mut bytes, &member.wiring.controller_sensor_inputs)?;
        put_vec(&mut bytes, &member.wiring.controller_action_outputs)?;
        put_vec(&mut bytes, &member.wiring.plant_sensor_outputs)?;
        put_vec(&mut bytes, &member.wiring.plant_action_inputs)?;
        for (value, field) in [
            (member.initial_controller_state, "initial controller state"),
            (member.initial_plant_state, "initial plant state"),
            (member.bad_plant_output, "bad plant output"),
            (member.horizon, "member horizon"),
        ] {
            bytes.extend_from_slice(&narrow(value, field)?.to_le_bytes());
        }
        put_result(&mut bytes, &member.result)?;
    }
    let integrity = Sha256::digest(&bytes);
    bytes.extend_from_slice(&integrity);
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES {
        return Err(reject(
            "controller direct plant artifact exceeds byte limit",
        ));
    }
    Ok(bytes)
}

pub fn decode_controller_direct_plant_artifact(
    bytes: &[u8],
) -> Result<ControllerDirectPlantBatchArtifact, ControllerPlantArtifactError> {
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES || bytes.len() < 77 {
        return Err(reject("controller direct plant artifact size is invalid"));
    }
    let payload_len = bytes.len() - 32;
    let (payload, claimed_integrity) = bytes.split_at(payload_len);
    if Sha256::digest(payload).as_slice() != claimed_integrity {
        return Err(reject(
            "controller direct plant artifact integrity mismatch",
        ));
    }
    let mut cursor = 0usize;
    if take(payload, &mut cursor, DIRECT_MAGIC.len())? != DIRECT_MAGIC {
        return Err(reject("controller direct plant artifact magic mismatch"));
    }
    let version = read_u32(payload, &mut cursor)?;
    if version != DIRECT_PLANT_ARTIFACT_VERSION {
        return Err(reject("controller direct plant artifact version mismatch"));
    }
    let controller_source_sha256 = take(payload, &mut cursor, 32)?
        .try_into()
        .map_err(|_| reject("controller digest decode failed"))?;
    let member_count = read_u8(payload, &mut cursor)? as usize;
    if member_count == 0 || member_count > MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS {
        return Err(reject(
            "controller direct plant member count is outside limit",
        ));
    }
    let mut members = Vec::with_capacity(member_count);
    for _ in 0..member_count {
        let plant_source_sha256 = take(payload, &mut cursor, 32)?
            .try_into()
            .map_err(|_| reject("plant digest decode failed"))?;
        let wiring = ControllerPlantWiring {
            controller_sensor_inputs: read_vec(payload, &mut cursor)?,
            controller_action_outputs: read_vec(payload, &mut cursor)?,
            plant_sensor_outputs: read_vec(payload, &mut cursor)?,
            plant_action_inputs: read_vec(payload, &mut cursor)?,
        };
        let initial_controller_state = read_u32(payload, &mut cursor)? as usize;
        let initial_plant_state = read_u32(payload, &mut cursor)? as usize;
        let bad_plant_output = read_u32(payload, &mut cursor)? as usize;
        let horizon = read_u32(payload, &mut cursor)? as usize;
        let result = read_result(payload, &mut cursor)?;
        members.push(ControllerPlantArtifactMember {
            plant_source_sha256,
            wiring,
            initial_controller_state,
            initial_plant_state,
            bad_plant_output,
            horizon,
            result,
        });
    }
    if cursor != payload.len() {
        return Err(reject(
            "controller direct plant artifact has trailing bytes",
        ));
    }
    Ok(ControllerDirectPlantBatchArtifact {
        version,
        controller_source_sha256,
        members,
    })
}

pub fn verify_controller_direct_plant_artifact(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    plants: &[(&AigerTransition, [u8; 32])],
    artifact_bytes: &[u8],
) -> Result<ControllerPlantBatchResult, ControllerPlantArtifactError> {
    let artifact = decode_controller_direct_plant_artifact(artifact_bytes)?;
    if artifact.controller_source_sha256 != controller_source_sha256
        || plants.len() != artifact.members.len()
    {
        return Err(reject("controller direct plant source binding mismatch"));
    }
    let inputs = plants
        .iter()
        .zip(&artifact.members)
        .map(|(&(plant, digest), member)| {
            if digest != member.plant_source_sha256 {
                return Err(reject("controller direct plant source digest mismatch"));
            }
            Ok(ControllerPlantBatchInput {
                plant,
                wiring: &member.wiring,
                initial_controller_state: member.initial_controller_state,
                initial_plant_state: member.initial_plant_state,
                bad_plant_output: member.bad_plant_output,
                horizon: member.horizon,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let verified = compose_controller_plant_direct_batch(controller_model, &inputs)
        .map_err(|error| reject(error.to_string()))?;
    if verified
        .members
        .iter()
        .zip(&artifact.members)
        .any(|(actual, claimed)| actual != &claimed.result)
    {
        return Err(reject("controller direct plant member result mismatch"));
    }
    Ok(verified)
}

fn portfolio_reason_from_failure(
    failure: ControllerMtbddAdmissionFailure,
) -> ControllerMtbddPlantSelectionReason {
    match failure {
        ControllerMtbddAdmissionFailure::BoundaryLimit => {
            ControllerMtbddPlantSelectionReason::BoundaryLimit
        }
        ControllerMtbddAdmissionFailure::TerminalLimit => {
            ControllerMtbddPlantSelectionReason::TerminalLimit
        }
        ControllerMtbddAdmissionFailure::NodeLimit => {
            ControllerMtbddPlantSelectionReason::NodeLimit
        }
    }
}

fn portfolio_backend_tag(backend: ControllerMtbddPlantPortfolioBackend) -> u8 {
    match backend {
        ControllerMtbddPlantPortfolioBackend::Mtbdd => 0,
        ControllerMtbddPlantPortfolioBackend::DirectExact => 1,
    }
}

fn portfolio_reason_tag(reason: ControllerMtbddPlantSelectionReason) -> u8 {
    match reason {
        ControllerMtbddPlantSelectionReason::MtbddAdmitted => 0,
        ControllerMtbddPlantSelectionReason::BoundaryLimit => 1,
        ControllerMtbddPlantSelectionReason::TerminalLimit => 2,
        ControllerMtbddPlantSelectionReason::NodeLimit => 3,
    }
}

pub fn encode_controller_mtbdd_plant_portfolio(
    artifact: &ControllerMtbddPlantPortfolioArtifact,
) -> Result<Vec<u8>, ControllerPlantArtifactError> {
    let route_is_valid = matches!(
        (artifact.backend, artifact.reason),
        (
            ControllerMtbddPlantPortfolioBackend::Mtbdd,
            ControllerMtbddPlantSelectionReason::MtbddAdmitted
        ) | (
            ControllerMtbddPlantPortfolioBackend::DirectExact,
            ControllerMtbddPlantSelectionReason::BoundaryLimit
                | ControllerMtbddPlantSelectionReason::TerminalLimit
                | ControllerMtbddPlantSelectionReason::NodeLimit
        )
    );
    if artifact.version != MTBDD_PLANT_PORTFOLIO_VERSION
        || !route_is_valid
        || artifact.relevant_inputs.is_empty()
        || artifact.observed_outputs.is_empty()
        || artifact.payload.is_empty()
    {
        return Err(reject("controller MTBDD portfolio dimensions are invalid"));
    }
    match artifact.backend {
        ControllerMtbddPlantPortfolioBackend::Mtbdd => {
            decode_controller_mtbdd_plant_artifact(&artifact.payload)?;
        }
        ControllerMtbddPlantPortfolioBackend::DirectExact => {
            decode_controller_direct_plant_artifact(&artifact.payload)?;
        }
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PORTFOLIO_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.push(portfolio_backend_tag(artifact.backend));
    bytes.push(portfolio_reason_tag(artifact.reason));
    put_vec(&mut bytes, &artifact.relevant_inputs)?;
    put_vec(&mut bytes, &artifact.observed_outputs)?;
    bytes.extend_from_slice(
        &narrow(artifact.payload.len(), "portfolio payload length")?.to_le_bytes(),
    );
    bytes.extend_from_slice(&artifact.payload);
    let integrity = Sha256::digest(&bytes);
    bytes.extend_from_slice(&integrity);
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES {
        return Err(reject("controller MTBDD portfolio exceeds byte limit"));
    }
    Ok(bytes)
}

pub fn decode_controller_mtbdd_plant_portfolio(
    bytes: &[u8],
) -> Result<ControllerMtbddPlantPortfolioArtifact, ControllerPlantArtifactError> {
    if bytes.len() > MAX_CONTROLLER_PLANT_ARTIFACT_BYTES || bytes.len() < 52 {
        return Err(reject("controller MTBDD portfolio size is invalid"));
    }
    let payload_len = bytes.len() - 32;
    let (payload, claimed_integrity) = bytes.split_at(payload_len);
    if Sha256::digest(payload).as_slice() != claimed_integrity {
        return Err(reject("controller MTBDD portfolio integrity mismatch"));
    }
    let mut cursor = 0usize;
    if take(payload, &mut cursor, PORTFOLIO_MAGIC.len())? != PORTFOLIO_MAGIC {
        return Err(reject("controller MTBDD portfolio magic mismatch"));
    }
    let version = read_u32(payload, &mut cursor)?;
    if version != MTBDD_PLANT_PORTFOLIO_VERSION {
        return Err(reject("controller MTBDD portfolio version mismatch"));
    }
    let backend = match read_u8(payload, &mut cursor)? {
        0 => ControllerMtbddPlantPortfolioBackend::Mtbdd,
        1 => ControllerMtbddPlantPortfolioBackend::DirectExact,
        _ => return Err(reject("controller MTBDD portfolio backend is invalid")),
    };
    let reason = match read_u8(payload, &mut cursor)? {
        0 => ControllerMtbddPlantSelectionReason::MtbddAdmitted,
        1 => ControllerMtbddPlantSelectionReason::BoundaryLimit,
        2 => ControllerMtbddPlantSelectionReason::TerminalLimit,
        3 => ControllerMtbddPlantSelectionReason::NodeLimit,
        _ => return Err(reject("controller MTBDD portfolio reason is invalid")),
    };
    let relevant_inputs = read_vec(payload, &mut cursor)?;
    let observed_outputs = read_vec(payload, &mut cursor)?;
    let embedded_len = read_u32(payload, &mut cursor)? as usize;
    let embedded = take(payload, &mut cursor, embedded_len)?.to_vec();
    if cursor != payload.len() {
        return Err(reject("controller MTBDD portfolio has trailing bytes"));
    }
    let artifact = ControllerMtbddPlantPortfolioArtifact {
        version,
        backend,
        reason,
        relevant_inputs,
        observed_outputs,
        payload: embedded,
    };
    encode_controller_mtbdd_plant_portfolio(&artifact)?;
    Ok(artifact)
}

pub fn produce_controller_mtbdd_plant_portfolio(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    members: &[ControllerPlantArtifactInput<'_>],
) -> Result<Vec<u8>, ControllerPlantArtifactError> {
    if !portfolio_boundary_matches_members(relevant_inputs, observed_outputs, members) {
        return Err(reject(
            "controller MTBDD portfolio member boundary mismatch",
        ));
    }
    let (backend, reason, payload) = match produce_controller_mtbdd(
        controller_model,
        controller_source_sha256,
        relevant_inputs,
        observed_outputs,
    ) {
        Ok(mtbdd) => {
            let artifact = produce_controller_mtbdd_plant_artifact(
                controller_model,
                controller_source_sha256,
                &mtbdd,
                members,
            )?;
            (
                ControllerMtbddPlantPortfolioBackend::Mtbdd,
                ControllerMtbddPlantSelectionReason::MtbddAdmitted,
                encode_controller_mtbdd_plant_artifact(&artifact)?,
            )
        }
        Err(error) => {
            let failure = error
                .admission_failure()
                .ok_or_else(|| reject(error.to_string()))?;
            let artifact = produce_controller_direct_plant_artifact(
                controller_model,
                controller_source_sha256,
                members,
            )?;
            (
                ControllerMtbddPlantPortfolioBackend::DirectExact,
                portfolio_reason_from_failure(failure),
                encode_controller_direct_plant_artifact(&artifact)?,
            )
        }
    };
    encode_controller_mtbdd_plant_portfolio(&ControllerMtbddPlantPortfolioArtifact {
        version: MTBDD_PLANT_PORTFOLIO_VERSION,
        backend,
        reason,
        relevant_inputs: relevant_inputs.to_vec(),
        observed_outputs: observed_outputs.to_vec(),
        payload,
    })
}

fn portfolio_boundary_matches_members(
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    members: &[ControllerPlantArtifactInput<'_>],
) -> bool {
    !members.is_empty()
        && members.iter().all(|member| {
            member.wiring.controller_sensor_inputs == relevant_inputs
                && member.wiring.controller_action_outputs == observed_outputs
        })
}

fn portfolio_members_match(
    claimed: &[ControllerPlantArtifactMember],
    expected: &[ControllerPlantArtifactInput<'_>],
) -> bool {
    claimed.len() == expected.len()
        && claimed.iter().zip(expected).all(|(claimed, expected)| {
            claimed.plant_source_sha256 == expected.plant_source_sha256
                && claimed.wiring == *expected.wiring
                && claimed.initial_controller_state == expected.initial_controller_state
                && claimed.initial_plant_state == expected.initial_plant_state
                && claimed.bad_plant_output == expected.bad_plant_output
                && claimed.horizon == expected.horizon
        })
}

pub fn verify_controller_mtbdd_plant_portfolio(
    controller_model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    members: &[ControllerPlantArtifactInput<'_>],
    bytes: &[u8],
) -> Result<ControllerMtbddPlantPortfolioSummary, ControllerPlantArtifactError> {
    let artifact = decode_controller_mtbdd_plant_portfolio(bytes)?;
    if artifact.relevant_inputs != relevant_inputs || artifact.observed_outputs != observed_outputs
    {
        return Err(reject("controller MTBDD portfolio boundary mismatch"));
    }
    if !portfolio_boundary_matches_members(relevant_inputs, observed_outputs, members) {
        return Err(reject(
            "controller MTBDD portfolio member boundary mismatch",
        ));
    }
    let plants = members
        .iter()
        .map(|member| (member.plant, member.plant_source_sha256))
        .collect::<Vec<_>>();
    let (results, safe, unsafe_count, reachable_product_states, explored_transitions) =
        match artifact.backend {
            ControllerMtbddPlantPortfolioBackend::Mtbdd => {
                let decoded = decode_controller_mtbdd_plant_artifact(&artifact.payload)?;
                if !portfolio_members_match(&decoded.members, members) {
                    return Err(reject("controller MTBDD portfolio member mismatch"));
                }
                let summary = verify_controller_mtbdd_plant_artifact(
                    controller_model,
                    controller_source_sha256,
                    &plants,
                    &artifact.payload,
                )?;
                (
                    summary.members,
                    summary.safe,
                    summary.unsafe_count,
                    summary.reachable_product_states,
                    summary.explored_transitions,
                )
            }
            ControllerMtbddPlantPortfolioBackend::DirectExact => {
                let expected_reason = match produce_controller_mtbdd(
                    controller_model,
                    controller_source_sha256,
                    relevant_inputs,
                    observed_outputs,
                ) {
                    Ok(_) => return Err(reject("controller MTBDD portfolio downgrade detected")),
                    Err(error) => error
                        .admission_failure()
                        .map(portfolio_reason_from_failure)
                        .ok_or_else(|| reject(error.to_string()))?,
                };
                if artifact.reason != expected_reason {
                    return Err(reject("controller MTBDD portfolio reason mismatch"));
                }
                let decoded = decode_controller_direct_plant_artifact(&artifact.payload)?;
                if !portfolio_members_match(&decoded.members, members) {
                    return Err(reject("controller MTBDD portfolio member mismatch"));
                }
                let summary = verify_controller_direct_plant_artifact(
                    controller_model,
                    controller_source_sha256,
                    &plants,
                    &artifact.payload,
                )?;
                (
                    summary.members,
                    summary.safe,
                    summary.unsafe_count,
                    summary.reachable_product_states,
                    summary.explored_transitions,
                )
            }
        };
    Ok(ControllerMtbddPlantPortfolioSummary {
        backend: artifact.backend,
        reason: artifact.reason,
        members: results,
        safe,
        unsafe_count,
        reachable_product_states,
        explored_transitions,
    })
}
