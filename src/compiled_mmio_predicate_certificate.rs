//! Canonical bounded evidence for an exact finite-domain compiled-MMIO
//! predicate workflow.

use crate::compiled_mmio_quotient::{
    ExactCompiledMmioBehavior, PredicateMmioWorkflow, PredicateMmioWorkflowVerification,
    build_predicate_mmio_workflow, verify_predicate_mmio_workflow,
};
use crate::riscv32imc::{
    CompiledMmioEvent, MAX_RV32_IMAGE_BYTES, MAX_RV32_STEPS, Rv32SymbolLayout,
};
use crate::riscv32imc_predicate::{
    INVALID_PREDICATE_FIRST, INVALID_PREDICATE_LANES, PredicateControlStep,
    PredicateTransducerExecution,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

const MAGIC: &[u8; 8] = b"GCCPDT01";
pub const COMPILED_MMIO_PREDICATE_CERTIFICATE_VERSION: u32 = 1;
pub const COMPILED_MMIO_PREDICATE_POLICY_VERSION: u32 = 1;
pub const COMPILED_MMIO_PREDICATE_ROUTE: u32 = 1;
pub const MAX_COMPILED_MMIO_PREDICATE_CERTIFICATE_BYTES: usize = 4 * 1024 * 1024;
const VALID_BEHAVIORS: usize = 6;
const MAX_EVENTS: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicateCertificate {
    pub version: u32,
    pub policy_version: u32,
    pub route: u32,
    pub image_bytes: u32,
    pub image_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
    pub workflow: PredicateMmioWorkflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicateCertificateVerification {
    pub producer_decoded_transitions: u64,
    pub producer_lane_value_operations: u64,
    pub verifier_decoded_transitions: u64,
    pub verifier_lane_value_operations: u64,
    pub artifact_bytes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicateCertificateError(pub String);

impl fmt::Display for CompiledMmioPredicateCertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO predicate certificate: {}", self.0)
    }
}

impl Error for CompiledMmioPredicateCertificateError {}

fn reject(message: impl Into<String>) -> CompiledMmioPredicateCertificateError {
    CompiledMmioPredicateCertificateError(message.into())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn validate_behavior(
    label: &str,
    behavior: &ExactCompiledMmioBehavior,
) -> Result<(), CompiledMmioPredicateCertificateError> {
    if behavior.events.len() > MAX_EVENTS {
        return Err(reject(format!("{label} event count exceeds policy")));
    }
    Ok(())
}

fn validate_fields(
    certificate: &CompiledMmioPredicateCertificate,
) -> Result<(), CompiledMmioPredicateCertificateError> {
    if certificate.version != COMPILED_MMIO_PREDICATE_CERTIFICATE_VERSION {
        return Err(reject("unsupported certificate version"));
    }
    if certificate.policy_version != COMPILED_MMIO_PREDICATE_POLICY_VERSION {
        return Err(reject("unsupported semantic policy version"));
    }
    if certificate.route != COMPILED_MMIO_PREDICATE_ROUTE {
        return Err(reject("unsupported portfolio route"));
    }
    if certificate.image_bytes == 0
        || usize::try_from(certificate.image_bytes)
            .map_or(true, |bytes| bytes > MAX_RV32_IMAGE_BYTES)
    {
        return Err(reject("bound image size is outside policy"));
    }
    if certificate.workflow.valid_behaviors.len() != VALID_BEHAVIORS {
        return Err(reject("valid singleton count is not canonical"));
    }
    for (index, behavior) in certificate.workflow.valid_behaviors.iter().enumerate() {
        validate_behavior(&format!("valid behavior {index}"), behavior)?;
    }
    let invalid = &certificate.workflow.invalid;
    if invalid.first_input != INVALID_PREDICATE_FIRST
        || usize::from(invalid.lane_count) != INVALID_PREDICATE_LANES
        || invalid.symbolic_transitions > MAX_RV32_STEPS
        || invalid.control_trace.len() as u64 != invalid.symbolic_transitions
        || invalid.event_program_locations.len() != invalid.events.len()
        || invalid.events.len() > MAX_EVENTS
    {
        return Err(reject("invalid predicate fields are outside policy"));
    }
    let lane_work = invalid
        .symbolic_transitions
        .checked_mul(INVALID_PREDICATE_LANES as u64)
        .ok_or_else(|| reject("lane work overflow"))?;
    if invalid.lane_value_operations != lane_work
        || certificate.workflow.producer_lane_value_operations != lane_work
        || certificate.workflow.producer_decoded_transitions < invalid.symbolic_transitions
    {
        return Err(reject("workflow work counters are inconsistent"));
    }
    for (index, step) in invalid.control_trace.iter().enumerate() {
        if !matches!(step.instruction_bytes, 2 | 4) {
            return Err(reject(format!(
                "transition {index} instruction width is invalid"
            )));
        }
        if index > 0
            && invalid.control_trace[index - 1].next_program_counter != step.program_counter
        {
            return Err(reject(format!(
                "control trace is discontinuous before transition {index}"
            )));
        }
    }
    Ok(())
}

fn encode_behavior(bytes: &mut Vec<u8>, behavior: &ExactCompiledMmioBehavior) {
    push_u32(bytes, behavior.return_value);
    push_u32(bytes, behavior.events.len() as u32);
    for event in &behavior.events {
        push_u32(bytes, event.operation);
        push_u32(bytes, event.offset);
        push_u32(bytes, event.value);
    }
}

pub fn encode_compiled_mmio_predicate_certificate(
    certificate: &CompiledMmioPredicateCertificate,
) -> Result<Vec<u8>, CompiledMmioPredicateCertificateError> {
    validate_fields(certificate)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_u32(&mut bytes, certificate.version);
    push_u32(&mut bytes, certificate.policy_version);
    push_u32(&mut bytes, certificate.route);
    push_u32(&mut bytes, certificate.image_bytes);
    bytes.extend_from_slice(&certificate.image_sha256);
    push_u32(&mut bytes, certificate.symbols.entry);
    push_u32(&mut bytes, certificate.symbols.event_count);
    push_u32(&mut bytes, certificate.symbols.events);
    push_u32(
        &mut bytes,
        certificate.workflow.valid_behaviors.len() as u32,
    );
    for behavior in &certificate.workflow.valid_behaviors {
        encode_behavior(&mut bytes, behavior);
    }
    let invalid = &certificate.workflow.invalid;
    push_u8(&mut bytes, invalid.first_input);
    push_u16(&mut bytes, invalid.lane_count);
    push_u32(&mut bytes, invalid.return_value);
    push_u32(&mut bytes, invalid.events.len() as u32);
    for (event, location) in invalid.events.iter().zip(&invalid.event_program_locations) {
        push_u32(&mut bytes, event.operation);
        push_u32(&mut bytes, event.offset);
        push_u32(&mut bytes, event.value);
        push_u32(&mut bytes, *location);
    }
    push_u64(&mut bytes, invalid.symbolic_transitions);
    push_u64(&mut bytes, invalid.lane_value_operations);
    push_u32(&mut bytes, invalid.sparse_memory_bytes);
    push_u32(&mut bytes, invalid.control_trace.len() as u32);
    for step in &invalid.control_trace {
        push_u32(&mut bytes, step.program_counter);
        push_u32(&mut bytes, step.instruction_word);
        push_u8(&mut bytes, step.instruction_bytes);
        push_u32(&mut bytes, step.next_program_counter);
    }
    push_u64(
        &mut bytes,
        certificate.workflow.producer_decoded_transitions,
    );
    push_u64(
        &mut bytes,
        certificate.workflow.producer_lane_value_operations,
    );
    let checksum = digest(&bytes);
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_COMPILED_MMIO_PREDICATE_CERTIFICATE_BYTES {
        return Err(reject("encoded certificate exceeds policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], CompiledMmioPredicateCertificateError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| reject("certificate offset overflow"))?;
        let result = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| reject("truncated certificate"))?;
        self.position = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, CompiledMmioPredicateCertificateError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CompiledMmioPredicateCertificateError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("fixed width"),
        ))
    }

    fn u32(&mut self) -> Result<u32, CompiledMmioPredicateCertificateError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed width"),
        ))
    }

    fn u64(&mut self) -> Result<u64, CompiledMmioPredicateCertificateError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed width"),
        ))
    }
}

fn bounded_count(
    cursor: &mut Cursor<'_>,
    maximum: usize,
    label: &str,
) -> Result<usize, CompiledMmioPredicateCertificateError> {
    let count = usize::try_from(cursor.u32()?).map_err(|_| reject(format!("{label} overflow")))?;
    if count > maximum {
        return Err(reject(format!("{label} exceeds policy")));
    }
    Ok(count)
}

fn decode_behavior(
    cursor: &mut Cursor<'_>,
    label: &str,
) -> Result<ExactCompiledMmioBehavior, CompiledMmioPredicateCertificateError> {
    let return_value = cursor.u32()?;
    let event_count = bounded_count(cursor, MAX_EVENTS, &format!("{label} event count"))?;
    let mut events = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        events.push(CompiledMmioEvent {
            operation: cursor.u32()?,
            offset: cursor.u32()?,
            value: cursor.u32()?,
        });
    }
    Ok(ExactCompiledMmioBehavior {
        return_value,
        events,
    })
}

pub fn decode_compiled_mmio_predicate_certificate(
    bytes: &[u8],
) -> Result<CompiledMmioPredicateCertificate, CompiledMmioPredicateCertificateError> {
    if bytes.len() < 128 || bytes.len() > MAX_COMPILED_MMIO_PREDICATE_CERTIFICATE_BYTES {
        return Err(reject("certificate size is outside policy"));
    }
    let content_len = bytes
        .len()
        .checked_sub(32)
        .ok_or_else(|| reject("certificate is truncated"))?;
    if digest(&bytes[..content_len]) != bytes[content_len..] {
        return Err(reject("certificate checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..content_len],
        position: 0,
    };
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(reject("certificate magic mismatch"));
    }
    let version = cursor.u32()?;
    let policy_version = cursor.u32()?;
    let route = cursor.u32()?;
    let image_bytes = cursor.u32()?;
    let image_sha256 = cursor.take(32)?.try_into().expect("fixed width");
    let symbols = Rv32SymbolLayout {
        entry: cursor.u32()?,
        event_count: cursor.u32()?,
        events: cursor.u32()?,
    };
    let valid_count = bounded_count(&mut cursor, VALID_BEHAVIORS, "valid singleton count")?;
    if valid_count != VALID_BEHAVIORS {
        return Err(reject("valid singleton count is not canonical"));
    }
    let mut valid_behaviors = Vec::with_capacity(valid_count);
    for index in 0..valid_count {
        valid_behaviors.push(decode_behavior(
            &mut cursor,
            &format!("valid behavior {index}"),
        )?);
    }
    let first_input = cursor.u8()?;
    let lane_count = cursor.u16()?;
    let return_value = cursor.u32()?;
    let event_count = bounded_count(&mut cursor, MAX_EVENTS, "invalid event count")?;
    let mut events = Vec::with_capacity(event_count);
    let mut event_program_locations = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        events.push(CompiledMmioEvent {
            operation: cursor.u32()?,
            offset: cursor.u32()?,
            value: cursor.u32()?,
        });
        event_program_locations.push(cursor.u32()?);
    }
    let symbolic_transitions = cursor.u64()?;
    let lane_value_operations = cursor.u64()?;
    let sparse_memory_bytes = cursor.u32()?;
    let transition_count = bounded_count(&mut cursor, MAX_RV32_STEPS as usize, "transition count")?;
    let transition_bytes = transition_count
        .checked_mul(13)
        .ok_or_else(|| reject("transition byte count overflow"))?;
    if transition_bytes > cursor.bytes.len().saturating_sub(cursor.position) {
        return Err(reject("transition table is truncated"));
    }
    let mut control_trace = Vec::with_capacity(transition_count);
    for _ in 0..transition_count {
        control_trace.push(PredicateControlStep {
            program_counter: cursor.u32()?,
            instruction_word: cursor.u32()?,
            instruction_bytes: cursor.u8()?,
            next_program_counter: cursor.u32()?,
        });
    }
    let producer_decoded_transitions = cursor.u64()?;
    let producer_lane_value_operations = cursor.u64()?;
    if cursor.position != content_len {
        return Err(reject("certificate has trailing content"));
    }
    let certificate = CompiledMmioPredicateCertificate {
        version,
        policy_version,
        route,
        image_bytes,
        image_sha256,
        symbols,
        workflow: PredicateMmioWorkflow {
            valid_behaviors,
            invalid: PredicateTransducerExecution {
                first_input,
                lane_count,
                return_value,
                events,
                event_program_locations,
                symbolic_transitions,
                lane_value_operations,
                sparse_memory_bytes,
                control_trace,
            },
            producer_decoded_transitions,
            producer_lane_value_operations,
        },
    };
    validate_fields(&certificate)?;
    if encode_compiled_mmio_predicate_certificate(&certificate)? != bytes {
        return Err(reject("certificate encoding is not canonical"));
    }
    Ok(certificate)
}

pub fn certify_compiled_mmio_predicate(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioPredicateCertificate, CompiledMmioPredicateCertificateError> {
    let image_bytes =
        u32::try_from(image.len()).map_err(|_| reject("image byte count overflow"))?;
    let workflow =
        build_predicate_mmio_workflow(image, symbols).map_err(|error| reject(error.to_string()))?;
    let certificate = CompiledMmioPredicateCertificate {
        version: COMPILED_MMIO_PREDICATE_CERTIFICATE_VERSION,
        policy_version: COMPILED_MMIO_PREDICATE_POLICY_VERSION,
        route: COMPILED_MMIO_PREDICATE_ROUTE,
        image_bytes,
        image_sha256: digest(image),
        symbols,
        workflow,
    };
    validate_fields(&certificate)?;
    Ok(certificate)
}

pub fn verify_compiled_mmio_predicate_bytes(
    bytes: &[u8],
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioPredicateCertificateVerification, CompiledMmioPredicateCertificateError> {
    let certificate = decode_compiled_mmio_predicate_certificate(bytes)?;
    if certificate.image_bytes as usize != image.len()
        || certificate.image_sha256 != digest(image)
        || certificate.symbols != symbols
    {
        return Err(reject("certificate source identity mismatch"));
    }
    let PredicateMmioWorkflowVerification {
        decoded_transitions,
        lane_value_operations,
    } = verify_predicate_mmio_workflow(&certificate.workflow, image, symbols)
        .map_err(|error| reject(error.to_string()))?;
    Ok(CompiledMmioPredicateCertificateVerification {
        producer_decoded_transitions: certificate.workflow.producer_decoded_transitions,
        producer_lane_value_operations: certificate.workflow.producer_lane_value_operations,
        verifier_decoded_transitions: decoded_transitions,
        verifier_lane_value_operations: lane_value_operations,
        artifact_bytes: bytes
            .len()
            .try_into()
            .map_err(|_| reject("artifact byte count overflow"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riscv32imc::RV32_IMAGE_BASE;

    fn guarded_image() -> (Vec<u8>, Rv32SymbolLayout) {
        let mut image = vec![0; 0x110];
        let sltiu_a0_six = (6u32 << 20) | (10 << 15) | (3 << 12) | (10 << 7) | 0x13;
        let return_to_ra = (1u32 << 15) | 0x67;
        image[..4].copy_from_slice(&sltiu_a0_six.to_le_bytes());
        image[4..8].copy_from_slice(&return_to_ra.to_le_bytes());
        (
            image,
            Rv32SymbolLayout {
                entry: RV32_IMAGE_BASE,
                event_count: RV32_IMAGE_BASE + 0x100,
                events: RV32_IMAGE_BASE + 0x104,
            },
        )
    }

    #[test]
    fn bytes_are_deterministic_and_independently_verified() {
        let (image, symbols) = guarded_image();
        let first = certify_compiled_mmio_predicate(&image, symbols).unwrap();
        let second = certify_compiled_mmio_predicate(&image, symbols).unwrap();
        let first_bytes = encode_compiled_mmio_predicate_certificate(&first).unwrap();
        let second_bytes = encode_compiled_mmio_predicate_certificate(&second).unwrap();
        assert_eq!(first_bytes, second_bytes);
        assert_eq!(
            decode_compiled_mmio_predicate_certificate(&first_bytes).unwrap(),
            first
        );
        let verification =
            verify_compiled_mmio_predicate_bytes(&first_bytes, &image, symbols).unwrap();
        assert_eq!(verification.producer_decoded_transitions, 14);
        assert_eq!(verification.producer_lane_value_operations, 500);
        assert_eq!(verification.verifier_decoded_transitions, 14);
        assert_eq!(verification.verifier_lane_value_operations, 500);
        assert_eq!(verification.artifact_bytes as usize, first_bytes.len());
    }

    #[test]
    fn every_byte_mutation_and_truncation_fails_closed() {
        let (image, symbols) = guarded_image();
        let certificate = certify_compiled_mmio_predicate(&image, symbols).unwrap();
        let bytes = encode_compiled_mmio_predicate_certificate(&certificate).unwrap();
        for index in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[index] ^= 1;
            assert!(
                verify_compiled_mmio_predicate_bytes(&changed, &image, symbols).is_err(),
                "accepted mutation at byte {index}"
            );
        }
        for length in 0..bytes.len() {
            assert!(
                verify_compiled_mmio_predicate_bytes(&bytes[..length], &image, symbols).is_err(),
                "accepted truncation at byte {length}"
            );
        }
    }

    #[test]
    fn source_and_symbol_substitution_fail_closed() {
        let (mut image, symbols) = guarded_image();
        let certificate = certify_compiled_mmio_predicate(&image, symbols).unwrap();
        let bytes = encode_compiled_mmio_predicate_certificate(&certificate).unwrap();
        image[0] ^= 1;
        assert!(verify_compiled_mmio_predicate_bytes(&bytes, &image, symbols).is_err());
        let changed_symbols = Rv32SymbolLayout {
            entry: symbols.entry + 4,
            ..symbols
        };
        assert!(verify_compiled_mmio_predicate_bytes(&bytes, &image, changed_symbols).is_err());
    }
}
