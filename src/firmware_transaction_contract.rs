//! Proof-carrying composition of one fixed firmware transaction contract with
//! an exact two-component RTL revision-impact bundle.

use crate::revision_impact::{
    RevisionImpactError, RevisionImpactPolicy, RevisionImpactSummary,
    TwoComponentRevisionImpactBundle, TwoComponentRevisionImpactInput,
    decode_two_component_revision_impact_bundle, encode_two_component_revision_impact_bundle,
    produce_two_component_revision_impact, verify_two_component_revision_impact,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const FIRMWARE_TRANSACTION_CONTRACT_VERSION: u32 = 1;
pub const MAX_FIRMWARE_CONTRACT_BYTES: usize = 1024 * 1024;
pub const MAX_FIRMWARE_STIMULUS_MAPPING_BYTES: usize = 1024 * 1024;
pub const MAX_FIRMWARE_TRANSACTION_EVENTS: usize = 32;
pub const MAX_FIRMWARE_TRANSACTION_ENVELOPE_BYTES: usize = 66 * 1024 * 1024;

const MAGIC: &[u8; 8] = b"GCCFTC01";
const CHECKSUM_BYTES: usize = 32;

/// Version-1 OpenTitan PWM firmware events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareTransactionEvent {
    ConfigureChannel0,
    EnableChannel0,
    ConfigureChannel1,
    ObserveChannel0,
    DisableChannel0,
    ReconfigureChannel0,
}

impl FirmwareTransactionEvent {
    fn code(self) -> u8 {
        match self {
            Self::ConfigureChannel0 => 1,
            Self::EnableChannel0 => 2,
            Self::ConfigureChannel1 => 3,
            Self::ObserveChannel0 => 4,
            Self::DisableChannel0 => 5,
            Self::ReconfigureChannel0 => 6,
        }
    }

    fn from_code(code: u8) -> Result<Self, FirmwareTransactionContractError> {
        match code {
            1 => Ok(Self::ConfigureChannel0),
            2 => Ok(Self::EnableChannel0),
            3 => Ok(Self::ConfigureChannel1),
            4 => Ok(Self::ObserveChannel0),
            5 => Ok(Self::DisableChannel0),
            6 => Ok(Self::ReconfigureChannel0),
            _ => Err(reject("unknown firmware transaction event")),
        }
    }
}

/// Complete source and schedule inputs bound by one contract envelope.
pub struct FirmwareTransactionContractInput<'a> {
    pub contract_source: &'a [u8],
    pub stimulus_mapping: &'a [u8],
    pub events: &'a [FirmwareTransactionEvent],
    pub revision: TwoComponentRevisionImpactInput<'a>,
}

/// Canonical firmware-contract evidence plus the existing exact impact bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareTransactionContractEnvelope {
    pub contract_sha256: [u8; 32],
    pub stimulus_mapping_sha256: [u8; 32],
    pub events: Vec<FirmwareTransactionEvent>,
    pub impact: TwoComponentRevisionImpactBundle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareTransactionContractSummary {
    pub events: usize,
    pub observation_ready_frame: usize,
    pub impact: RevisionImpactSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareTransactionContractError(pub String);

impl fmt::Display for FirmwareTransactionContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "firmware transaction contract: {}", self.0)
    }
}

impl Error for FirmwareTransactionContractError {}

impl From<RevisionImpactError> for FirmwareTransactionContractError {
    fn from(error: RevisionImpactError) -> Self {
        Self(error.to_string())
    }
}

fn reject(message: impl Into<String>) -> FirmwareTransactionContractError {
    FirmwareTransactionContractError(message.into())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn validate_input(
    input: &FirmwareTransactionContractInput<'_>,
) -> Result<(), FirmwareTransactionContractError> {
    if input.contract_source.is_empty() || input.contract_source.len() > MAX_FIRMWARE_CONTRACT_BYTES
    {
        return Err(reject("contract source size is outside policy"));
    }
    if input.stimulus_mapping.is_empty()
        || input.stimulus_mapping.len() > MAX_FIRMWARE_STIMULUS_MAPPING_BYTES
    {
        return Err(reject("stimulus mapping size is outside policy"));
    }
    validate_events(input.events)
}

fn validate_events(
    events: &[FirmwareTransactionEvent],
) -> Result<(), FirmwareTransactionContractError> {
    if events.len() > MAX_FIRMWARE_TRANSACTION_EVENTS {
        return Err(reject("firmware transaction event count exceeds policy"));
    }
    let expected = [
        FirmwareTransactionEvent::ConfigureChannel0,
        FirmwareTransactionEvent::EnableChannel0,
        FirmwareTransactionEvent::ConfigureChannel1,
        FirmwareTransactionEvent::ObserveChannel0,
    ];
    if events != expected {
        return Err(reject(
            "firmware transaction does not reach observation-ready state",
        ));
    }
    Ok(())
}

/// Produces the exact RTL impact evidence only after the firmware transaction
/// reaches the fixed observation-ready state without entering rejection.
pub fn produce_firmware_transaction_contract(
    input: &FirmwareTransactionContractInput<'_>,
) -> Result<FirmwareTransactionContractEnvelope, FirmwareTransactionContractError> {
    validate_input(input)?;
    let envelope = FirmwareTransactionContractEnvelope {
        contract_sha256: digest(input.contract_source),
        stimulus_mapping_sha256: digest(input.stimulus_mapping),
        events: input.events.to_vec(),
        impact: produce_two_component_revision_impact(&input.revision)?,
    };
    encode_firmware_transaction_contract(&envelope)?;
    Ok(envelope)
}

/// Independently checks the firmware trace and every RTL counterfactual.
pub fn verify_firmware_transaction_contract(
    input: &FirmwareTransactionContractInput<'_>,
    envelope: &FirmwareTransactionContractEnvelope,
) -> Result<FirmwareTransactionContractSummary, FirmwareTransactionContractError> {
    validate_input(input)?;
    if envelope.contract_sha256 != digest(input.contract_source) {
        return Err(reject("contract source digest mismatch"));
    }
    if envelope.stimulus_mapping_sha256 != digest(input.stimulus_mapping) {
        return Err(reject("stimulus mapping digest mismatch"));
    }
    if envelope.events != input.events {
        return Err(reject("firmware transaction trace mismatch"));
    }
    validate_events(&envelope.events)?;
    encode_firmware_transaction_contract(envelope)?;
    let impact = verify_two_component_revision_impact(&input.revision, &envelope.impact)?;
    Ok(FirmwareTransactionContractSummary {
        events: envelope.events.len(),
        observation_ready_frame: envelope.events.len(),
        impact,
    })
}

pub fn encode_firmware_transaction_contract(
    envelope: &FirmwareTransactionContractEnvelope,
) -> Result<Vec<u8>, FirmwareTransactionContractError> {
    validate_events(&envelope.events)?;
    let impact = encode_two_component_revision_impact_bundle(
        &envelope.impact,
        RevisionImpactPolicy::default(),
    )?;
    let projected = 8usize
        .checked_add(4)
        .and_then(|value| value.checked_add(32 + 32 + 4))
        .and_then(|value| value.checked_add(envelope.events.len()))
        .and_then(|value| value.checked_add(8))
        .and_then(|value| value.checked_add(impact.len()))
        .and_then(|value| value.checked_add(CHECKSUM_BYTES))
        .ok_or_else(|| reject("firmware transaction envelope size overflow"))?;
    if projected > MAX_FIRMWARE_TRANSACTION_ENVELOPE_BYTES {
        return Err(reject("firmware transaction envelope exceeds policy"));
    }
    let mut bytes = Vec::with_capacity(projected);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&FIRMWARE_TRANSACTION_CONTRACT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&envelope.contract_sha256);
    bytes.extend_from_slice(&envelope.stimulus_mapping_sha256);
    bytes.extend_from_slice(&(envelope.events.len() as u32).to_le_bytes());
    bytes.extend(envelope.events.iter().map(|event| event.code()));
    bytes.extend_from_slice(&(impact.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&impact);
    let checksum = digest(&bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

pub fn decode_firmware_transaction_contract(
    bytes: &[u8],
) -> Result<FirmwareTransactionContractEnvelope, FirmwareTransactionContractError> {
    if bytes.len() > MAX_FIRMWARE_TRANSACTION_ENVELOPE_BYTES
        || bytes.len() < 8 + 4 + 32 + 32 + 4 + 8 + CHECKSUM_BYTES
    {
        return Err(reject(
            "firmware transaction envelope size is outside policy",
        ));
    }
    let content_len = bytes.len() - CHECKSUM_BYTES;
    if digest(&bytes[..content_len]) != bytes[content_len..] {
        return Err(reject("firmware transaction envelope checksum mismatch"));
    }
    let mut cursor = Cursor::new(&bytes[..content_len]);
    if cursor.take(8)? != MAGIC {
        return Err(reject("firmware transaction envelope magic mismatch"));
    }
    if cursor.u32()? != FIRMWARE_TRANSACTION_CONTRACT_VERSION {
        return Err(reject("unsupported firmware transaction contract version"));
    }
    let contract_sha256 = cursor.array32()?;
    let stimulus_mapping_sha256 = cursor.array32()?;
    let event_count = cursor.u32()? as usize;
    if event_count > MAX_FIRMWARE_TRANSACTION_EVENTS {
        return Err(reject("firmware transaction event count exceeds policy"));
    }
    let events = cursor
        .take(event_count)?
        .iter()
        .map(|code| FirmwareTransactionEvent::from_code(*code))
        .collect::<Result<Vec<_>, _>>()?;
    validate_events(&events)?;
    let impact_len = usize::try_from(cursor.u64()?)
        .map_err(|_| reject("revision impact length is outside platform range"))?;
    let impact = decode_two_component_revision_impact_bundle(
        cursor.take(impact_len)?,
        RevisionImpactPolicy::default(),
    )?;
    if !cursor.remaining().is_empty() {
        return Err(reject("trailing firmware transaction envelope bytes"));
    }
    let envelope = FirmwareTransactionContractEnvelope {
        contract_sha256,
        stimulus_mapping_sha256,
        events,
        impact,
    };
    if encode_firmware_transaction_contract(&envelope)? != bytes {
        return Err(reject("firmware transaction envelope is not canonical"));
    }
    Ok(envelope)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], FirmwareTransactionContractError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("firmware transaction envelope offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("truncated firmware transaction envelope"))?;
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, FirmwareTransactionContractError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| reject("invalid firmware transaction u32"))?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, FirmwareTransactionContractError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| reject("invalid firmware transaction u64"))?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn array32(&mut self) -> Result<[u8; 32], FirmwareTransactionContractError> {
        self.take(32)?
            .try_into()
            .map_err(|_| reject("invalid firmware transaction digest"))
    }

    fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.offset..]
    }
}
