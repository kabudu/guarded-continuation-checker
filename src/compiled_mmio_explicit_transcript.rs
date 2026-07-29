//! Canonical explicit execution transcripts for the compiled-MMIO closest
//! system baseline.

use crate::riscv32imc::{
    CompiledMmioEvent, MAX_RV32_STEPS, Rv32Execution, Rv32MemoryAccess, Rv32ReplayMachine,
    Rv32StepObservation, Rv32SymbolLayout,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

const MAGIC: &[u8; 8] = b"GCCXTR01";
const CHECKSUM_BYTES: usize = 32;
pub const EXPLICIT_TRANSCRIPT_VERSION: u32 = 1;
pub const EXPLICIT_TRANSCRIPT_INPUTS: usize = 256;
pub const MAX_EXPLICIT_TRANSCRIPT_BYTES: usize = 64 * 1024 * 1024;
const MAX_TOTAL_TRANSITIONS: u64 = 16 * 1024 * 1024;
const MAX_ACCESSES_PER_STEP: usize = 16;
const MAX_EVENTS_PER_EXECUTION: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitExecutionTranscript {
    pub input: u8,
    pub steps: Vec<Rv32StepObservation>,
    pub execution: Rv32Execution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitCompiledMmioTranscript {
    pub version: u32,
    pub image_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
    pub executions: Vec<ExplicitExecutionTranscript>,
    pub decoded_instruction_transitions: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExplicitCompiledMmioTranscriptVerification {
    pub artifact_bytes: u32,
    pub decoded_instruction_transitions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitCompiledMmioTranscriptError(pub String);

impl fmt::Display for ExplicitCompiledMmioTranscriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "explicit compiled-MMIO transcript: {}", self.0)
    }
}

impl Error for ExplicitCompiledMmioTranscriptError {}

fn reject(message: impl Into<String>) -> ExplicitCompiledMmioTranscriptError {
    ExplicitCompiledMmioTranscriptError(message.into())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn validate(
    transcript: &ExplicitCompiledMmioTranscript,
) -> Result<(), ExplicitCompiledMmioTranscriptError> {
    if transcript.version != EXPLICIT_TRANSCRIPT_VERSION
        || transcript.executions.len() != EXPLICIT_TRANSCRIPT_INPUTS
    {
        return Err(reject("transcript shape is not canonical"));
    }
    let mut total = 0u64;
    for (input, execution) in transcript.executions.iter().enumerate() {
        if usize::from(execution.input) != input
            || execution.execution.steps != execution.steps.len() as u64
            || execution.steps.len() as u64 > MAX_RV32_STEPS
            || execution.execution.events.len() > MAX_EVENTS_PER_EXECUTION
            || execution.execution.events.len() != execution.execution.event_program_locations.len()
        {
            return Err(reject(format!("execution {input} shape is not canonical")));
        }
        total = total
            .checked_add(execution.execution.steps)
            .ok_or_else(|| reject("transition count overflow"))?;
        if total > MAX_TOTAL_TRANSITIONS {
            return Err(reject("total transition count exceeds policy"));
        }
        for step in &execution.steps {
            if step.reads.len() > MAX_ACCESSES_PER_STEP
                || step.writes.len() > MAX_ACCESSES_PER_STEP
                || step
                    .reads
                    .iter()
                    .chain(&step.writes)
                    .any(|access| !matches!(access.width, 1 | 2 | 4))
            {
                return Err(reject(format!(
                    "execution {input} step observation exceeds policy"
                )));
            }
        }
    }
    if total != transcript.decoded_instruction_transitions {
        return Err(reject("declared transition count is inconsistent"));
    }
    Ok(())
}

pub fn build_explicit_compiled_mmio_transcript(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<ExplicitCompiledMmioTranscript, ExplicitCompiledMmioTranscriptError> {
    let mut executions = Vec::with_capacity(EXPLICIT_TRANSCRIPT_INPUTS);
    let mut decoded_instruction_transitions = 0u64;
    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let mut steps = Vec::new();
        while !machine.is_complete() {
            steps.push(
                machine
                    .step_observed()
                    .map_err(|error| reject(format!("input {input}: {error}")))?,
            );
        }
        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        decoded_instruction_transitions = decoded_instruction_transitions
            .checked_add(execution.steps)
            .ok_or_else(|| reject("transition count overflow"))?;
        executions.push(ExplicitExecutionTranscript {
            input,
            steps,
            execution,
        });
    }
    let transcript = ExplicitCompiledMmioTranscript {
        version: EXPLICIT_TRANSCRIPT_VERSION,
        image_sha256: digest(image),
        symbols,
        executions,
        decoded_instruction_transitions,
    };
    validate(&transcript)?;
    Ok(transcript)
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn encode_accesses(bytes: &mut Vec<u8>, accesses: &[Rv32MemoryAccess]) {
    bytes.push(accesses.len() as u8);
    for access in accesses {
        push_u32(bytes, access.address);
        bytes.push(access.width);
    }
}

fn encode_execution(bytes: &mut Vec<u8>, execution: &Rv32Execution) {
    push_u32(bytes, execution.return_value);
    push_u64(bytes, execution.steps);
    push_u32(bytes, execution.events.len() as u32);
    for event in &execution.events {
        push_u32(bytes, event.operation);
        push_u32(bytes, event.offset);
        push_u32(bytes, event.value);
    }
    push_u32(bytes, execution.event_program_locations.len() as u32);
    for location in &execution.event_program_locations {
        push_u32(bytes, *location);
    }
}

pub fn encode_explicit_compiled_mmio_transcript(
    transcript: &ExplicitCompiledMmioTranscript,
) -> Result<Vec<u8>, ExplicitCompiledMmioTranscriptError> {
    validate(transcript)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_u32(&mut bytes, transcript.version);
    bytes.extend_from_slice(&transcript.image_sha256);
    push_u32(&mut bytes, transcript.symbols.entry);
    push_u32(&mut bytes, transcript.symbols.event_count);
    push_u32(&mut bytes, transcript.symbols.events);
    push_u64(&mut bytes, transcript.decoded_instruction_transitions);
    push_u32(&mut bytes, transcript.executions.len() as u32);
    for execution in &transcript.executions {
        bytes.push(execution.input);
        push_u64(&mut bytes, execution.steps.len() as u64);
        for step in &execution.steps {
            push_u32(&mut bytes, step.program_counter);
            push_u32(&mut bytes, step.register_reads);
            push_u32(&mut bytes, step.register_writes);
            encode_accesses(&mut bytes, &step.reads);
            encode_accesses(&mut bytes, &step.writes);
        }
        encode_execution(&mut bytes, &execution.execution);
    }
    bytes.extend_from_slice(&digest(&bytes));
    if bytes.len() > MAX_EXPLICIT_TRANSCRIPT_BYTES {
        return Err(reject("encoded transcript exceeds policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ExplicitCompiledMmioTranscriptError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("transcript offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("transcript is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ExplicitCompiledMmioTranscriptError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, ExplicitCompiledMmioTranscriptError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| reject("invalid u32 field"))?,
        ))
    }

    fn u64(&mut self) -> Result<u64, ExplicitCompiledMmioTranscriptError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| reject("invalid u64 field"))?,
        ))
    }
}

fn decode_accesses(
    cursor: &mut Cursor<'_>,
) -> Result<Vec<Rv32MemoryAccess>, ExplicitCompiledMmioTranscriptError> {
    let count = usize::from(cursor.u8()?);
    if count > MAX_ACCESSES_PER_STEP {
        return Err(reject("memory-access count exceeds policy"));
    }
    let mut accesses = Vec::with_capacity(count);
    for _ in 0..count {
        accesses.push(Rv32MemoryAccess {
            address: cursor.u32()?,
            width: cursor.u8()?,
        });
    }
    Ok(accesses)
}

fn decode_execution(
    cursor: &mut Cursor<'_>,
) -> Result<Rv32Execution, ExplicitCompiledMmioTranscriptError> {
    let return_value = cursor.u32()?;
    let steps = cursor.u64()?;
    let event_count =
        usize::try_from(cursor.u32()?).map_err(|_| reject("event count exceeds platform range"))?;
    if event_count > MAX_EVENTS_PER_EXECUTION {
        return Err(reject("event count exceeds policy"));
    }
    let mut events = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        events.push(CompiledMmioEvent {
            operation: cursor.u32()?,
            offset: cursor.u32()?,
            value: cursor.u32()?,
        });
    }
    let location_count = usize::try_from(cursor.u32()?)
        .map_err(|_| reject("location count exceeds platform range"))?;
    if location_count != event_count {
        return Err(reject("event and location counts differ"));
    }
    let mut event_program_locations = Vec::with_capacity(location_count);
    for _ in 0..location_count {
        event_program_locations.push(cursor.u32()?);
    }
    Ok(Rv32Execution {
        return_value,
        steps,
        events,
        event_program_locations,
    })
}

pub fn decode_explicit_compiled_mmio_transcript(
    bytes: &[u8],
) -> Result<ExplicitCompiledMmioTranscript, ExplicitCompiledMmioTranscriptError> {
    if bytes.len() < 96 || bytes.len() > MAX_EXPLICIT_TRANSCRIPT_BYTES {
        return Err(reject("transcript size is outside policy"));
    }
    let content_len = bytes
        .len()
        .checked_sub(CHECKSUM_BYTES)
        .ok_or_else(|| reject("transcript is truncated"))?;
    if digest(&bytes[..content_len]) != bytes[content_len..] {
        return Err(reject("transcript checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..content_len],
        offset: 0,
    };
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(reject("transcript magic mismatch"));
    }
    let version = cursor.u32()?;
    let image_sha256 = cursor
        .take(32)?
        .try_into()
        .map_err(|_| reject("invalid image digest"))?;
    let symbols = Rv32SymbolLayout {
        entry: cursor.u32()?,
        event_count: cursor.u32()?,
        events: cursor.u32()?,
    };
    let decoded_instruction_transitions = cursor.u64()?;
    if decoded_instruction_transitions > MAX_TOTAL_TRANSITIONS {
        return Err(reject("transition count exceeds policy"));
    }
    let execution_count = usize::try_from(cursor.u32()?)
        .map_err(|_| reject("execution count exceeds platform range"))?;
    if execution_count != EXPLICIT_TRANSCRIPT_INPUTS {
        return Err(reject("execution count is not canonical"));
    }
    let mut executions = Vec::with_capacity(execution_count);
    let mut observed_total = 0u64;
    for input in 0..execution_count {
        let encoded_input = cursor.u8()?;
        if usize::from(encoded_input) != input {
            return Err(reject("execution inputs are not canonical"));
        }
        let step_count = cursor.u64()?;
        if step_count > MAX_RV32_STEPS {
            return Err(reject("execution step count exceeds policy"));
        }
        let step_count_usize =
            usize::try_from(step_count).map_err(|_| reject("step count exceeds platform range"))?;
        // Every encoded observation requires at least three u32 fields and
        // two empty access counts. Refuse impossible declarations before
        // reserving attacker-controlled capacity.
        if step_count_usize > cursor.remaining() / 14 {
            return Err(reject("declared steps exceed remaining transcript bytes"));
        }
        observed_total = observed_total
            .checked_add(step_count)
            .ok_or_else(|| reject("transition count overflow"))?;
        if observed_total > MAX_TOTAL_TRANSITIONS {
            return Err(reject("total transition count exceeds policy"));
        }
        let mut steps = Vec::with_capacity(step_count_usize);
        for _ in 0..step_count {
            steps.push(Rv32StepObservation {
                program_counter: cursor.u32()?,
                register_reads: cursor.u32()?,
                register_writes: cursor.u32()?,
                reads: decode_accesses(&mut cursor)?,
                writes: decode_accesses(&mut cursor)?,
            });
        }
        let execution = decode_execution(&mut cursor)?;
        executions.push(ExplicitExecutionTranscript {
            input: encoded_input,
            steps,
            execution,
        });
    }
    if cursor.offset != cursor.bytes.len() || observed_total != decoded_instruction_transitions {
        return Err(reject("transcript content is inconsistent"));
    }
    let transcript = ExplicitCompiledMmioTranscript {
        version,
        image_sha256,
        symbols,
        executions,
        decoded_instruction_transitions,
    };
    validate(&transcript)?;
    if encode_explicit_compiled_mmio_transcript(&transcript)? != bytes {
        return Err(reject("transcript encoding is not canonical"));
    }
    Ok(transcript)
}

pub fn verify_explicit_compiled_mmio_transcript(
    bytes: &[u8],
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<ExplicitCompiledMmioTranscriptVerification, ExplicitCompiledMmioTranscriptError> {
    let transcript = decode_explicit_compiled_mmio_transcript(bytes)?;
    if transcript.image_sha256 != digest(image) || transcript.symbols != symbols {
        return Err(reject("transcript input identity mismatch"));
    }
    let mut decoded_instruction_transitions = 0u64;
    for expected in &transcript.executions {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(expected.input))
            .map_err(|error| reject(format!("input {}: {error}", expected.input)))?;
        for (index, expected_step) in expected.steps.iter().enumerate() {
            if machine.is_complete() {
                return Err(reject(format!(
                    "input {} completed before step {index}",
                    expected.input
                )));
            }
            let actual = machine
                .step_observed()
                .map_err(|error| reject(format!("input {}: {error}", expected.input)))?;
            if actual != *expected_step {
                return Err(reject(format!(
                    "input {} observation {index} mismatch",
                    expected.input
                )));
            }
        }
        if !machine.is_complete() {
            return Err(reject(format!(
                "input {} transcript omitted steps",
                expected.input
            )));
        }
        let actual = machine
            .finish()
            .map_err(|error| reject(format!("input {}: {error}", expected.input)))?;
        if actual != expected.execution {
            return Err(reject(format!(
                "input {} terminal execution mismatch",
                expected.input
            )));
        }
        decoded_instruction_transitions = decoded_instruction_transitions
            .checked_add(actual.steps)
            .ok_or_else(|| reject("transition count overflow"))?;
    }
    if decoded_instruction_transitions != transcript.decoded_instruction_transitions {
        return Err(reject("verified transition count differs from producer"));
    }
    Ok(ExplicitCompiledMmioTranscriptVerification {
        artifact_bytes: u32::try_from(bytes.len())
            .map_err(|_| reject("artifact size exceeds u32"))?,
        decoded_instruction_transitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riscv32imc::RV32_IMAGE_BASE;

    fn parity_image() -> (Vec<u8>, Rv32SymbolLayout) {
        let mut image = vec![0; 0x110];
        let andi_a0_one = (1u32 << 20) | (10 << 15) | (7 << 12) | (10 << 7) | 0x13;
        let return_to_ra = (1u32 << 15) | 0x67;
        image[..4].copy_from_slice(&andi_a0_one.to_le_bytes());
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
    fn transcript_round_trips_and_verifies() {
        let (image, symbols) = parity_image();
        let transcript = build_explicit_compiled_mmio_transcript(&image, symbols).unwrap();
        let bytes = encode_explicit_compiled_mmio_transcript(&transcript).unwrap();
        assert_eq!(
            decode_explicit_compiled_mmio_transcript(&bytes).unwrap(),
            transcript
        );
        let verification =
            verify_explicit_compiled_mmio_transcript(&bytes, &image, symbols).unwrap();
        assert_eq!(verification.decoded_instruction_transitions, 512);
    }

    #[test]
    fn transcript_refuses_mutation_truncation_and_source_drift() {
        let (image, symbols) = parity_image();
        let transcript = build_explicit_compiled_mmio_transcript(&image, symbols).unwrap();
        let bytes = encode_explicit_compiled_mmio_transcript(&transcript).unwrap();
        let mut changed = bytes.clone();
        let middle = changed.len() / 2;
        changed[middle] ^= 1;
        assert!(verify_explicit_compiled_mmio_transcript(&changed, &image, symbols).is_err());
        assert!(
            verify_explicit_compiled_mmio_transcript(&bytes[..bytes.len() - 1], &image, symbols)
                .is_err()
        );
        let mut changed_image = image.clone();
        changed_image[0] ^= 1;
        assert!(verify_explicit_compiled_mmio_transcript(&bytes, &changed_image, symbols).is_err());
    }
}
