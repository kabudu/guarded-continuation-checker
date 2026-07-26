//! Exact all-input reference for guarded compiled-MMIO continuation research.
//!
//! This module deliberately performs every execution independently. It is the
//! fail-closed reference and work-accounting denominator for a later
//! continuation quotient, not an optimized producer.

use crate::riscv32imc::{
    CompiledMmioEvent, Rv32ReplayMachine, Rv32SymbolLayout, execute_compiled_mmio_with_a0,
};
use std::{error::Error, fmt};

pub const EXACT_COMPILED_MMIO_REFERENCE_VERSION: u32 = 1;
pub const EXACT_COMPILED_MMIO_INPUTS: usize = 256;
pub const GUARDED_MMIO_QUOTIENT_VERSION: u32 = 1;
pub const GUARDED_MMIO_VALID_CHANNELS: u8 = 6;
const MEMBERSHIP_WORDS: usize = EXACT_COMPILED_MMIO_INPUTS / 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCompiledMmioBehavior {
    pub return_value: u32,
    pub events: Vec<CompiledMmioEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCompiledMmioClass {
    pub representative: u8,
    pub members: [u64; MEMBERSHIP_WORDS],
    pub behavior: ExactCompiledMmioBehavior,
}

impl ExactCompiledMmioClass {
    pub fn contains(&self, input: u8) -> bool {
        let input = usize::from(input);
        self.members[input / 64] & (1u64 << (input % 64)) != 0
    }

    pub fn member_count(&self) -> u32 {
        self.members.iter().map(|word| word.count_ones()).sum()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCompiledMmioExecution {
    pub input: u8,
    pub class_index: u16,
    pub steps: u64,
    pub event_program_locations: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCompiledMmioReference {
    pub version: u32,
    pub classes: Vec<ExactCompiledMmioClass>,
    pub executions: Vec<ExactCompiledMmioExecution>,
    pub decoded_instruction_transitions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardedMmioQuotient {
    pub version: u32,
    pub valid_behaviors: Vec<ExactCompiledMmioBehavior>,
    pub invalid_behavior: ExactCompiledMmioBehavior,
    pub invalid_representative: u8,
    pub invalid_prefix_steps: Vec<u64>,
    pub shared_continuation_steps: u64,
    pub producer_decoded_instruction_transitions: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardedMmioQuotientVerification {
    pub decoded_instruction_transitions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardedMmioPortfolio {
    Quotient(GuardedMmioQuotient),
    Exact(ExactCompiledMmioReference),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCompiledMmioReferenceError(pub String);

impl fmt::Display for ExactCompiledMmioReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "exact compiled-MMIO reference: {}", self.0)
    }
}

impl Error for ExactCompiledMmioReferenceError {}

fn reject(message: impl Into<String>) -> ExactCompiledMmioReferenceError {
    ExactCompiledMmioReferenceError(message.into())
}

fn behavior(execution: crate::riscv32imc::Rv32Execution) -> ExactCompiledMmioBehavior {
    ExactCompiledMmioBehavior {
        return_value: execution.return_value,
        events: execution.events,
    }
}

fn add_work(total: &mut u64, work: u64) -> Result<(), ExactCompiledMmioReferenceError> {
    *total = total
        .checked_add(work)
        .ok_or_else(|| reject("decoded instruction transition count overflow"))?;
    Ok(())
}

/// Produce the frozen eight-bit guarded-MMIO quotient using exact state
/// convergence only.
///
/// Channels 0 through 5 remain explicit singleton paths. Inputs 6 through 255
/// may reuse the suffix reached by inputs 6 and 7 only after their opaque
/// replay machines become byte-for-byte equal. Every other invalid input must
/// independently reach that same state at the declared prefix length.
pub fn build_guarded_mmio_quotient(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<GuardedMmioQuotient, ExactCompiledMmioReferenceError> {
    let mut valid_behaviors = Vec::with_capacity(usize::from(GUARDED_MMIO_VALID_CHANNELS));
    let mut producer_work = 0u64;
    for input in 0..GUARDED_MMIO_VALID_CHANNELS {
        let execution = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("valid input {input}: {error}")))?
            .finish()
            .map_err(|error| reject(format!("valid input {input}: {error}")))?;
        add_work(&mut producer_work, execution.steps)?;
        valid_behaviors.push(behavior(execution));
    }

    let mut representative =
        Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(GUARDED_MMIO_VALID_CHANNELS))
            .map_err(|error| reject(format!("invalid representative: {error}")))?;
    let mut second =
        Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(GUARDED_MMIO_VALID_CHANNELS) + 1)
            .map_err(|error| reject(format!("invalid convergence witness: {error}")))?;
    while representative != second {
        if representative.is_complete() || second.is_complete() {
            let difference = representative
                .exact_difference(&second)
                .unwrap_or_else(|| "unclassified state".to_string());
            return Err(reject(format!(
                "invalid inputs completed without exact state convergence: {difference}"
            )));
        }
        representative
            .step()
            .map_err(|error| reject(format!("invalid representative: {error}")))?;
        second
            .step()
            .map_err(|error| reject(format!("invalid convergence witness: {error}")))?;
    }
    let merge_steps = representative.steps();
    add_work(&mut producer_work, merge_steps)?;
    add_work(&mut producer_work, second.steps())?;

    let mut invalid_prefix_steps = vec![merge_steps; 250];
    for input in (GUARDED_MMIO_VALID_CHANNELS + 2)..=u8::MAX {
        let mut candidate = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("invalid input {input}: {error}")))?;
        while candidate.steps() < merge_steps {
            candidate
                .step()
                .map_err(|error| reject(format!("invalid input {input}: {error}")))?;
        }
        if candidate != representative {
            return Err(reject(format!(
                "invalid input {input} does not reach the exact shared state"
            )));
        }
        add_work(&mut producer_work, candidate.steps())?;
        invalid_prefix_steps[usize::from(input - GUARDED_MMIO_VALID_CHANNELS)] = candidate.steps();
    }

    let prefix_steps = representative.steps();
    let invalid_execution = representative
        .finish()
        .map_err(|error| reject(format!("shared invalid continuation: {error}")))?;
    let shared_continuation_steps = invalid_execution
        .steps
        .checked_sub(prefix_steps)
        .ok_or_else(|| reject("shared continuation work underflow"))?;
    add_work(&mut producer_work, shared_continuation_steps)?;

    Ok(GuardedMmioQuotient {
        version: GUARDED_MMIO_QUOTIENT_VERSION,
        valid_behaviors,
        invalid_behavior: behavior(invalid_execution),
        invalid_representative: GUARDED_MMIO_VALID_CHANNELS,
        invalid_prefix_steps,
        shared_continuation_steps,
        producer_decoded_instruction_transitions: producer_work,
    })
}

/// Reconstruct and verify every quotient route without invoking the producer.
pub fn verify_guarded_mmio_quotient(
    quotient: &GuardedMmioQuotient,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<GuardedMmioQuotientVerification, ExactCompiledMmioReferenceError> {
    if quotient.version != GUARDED_MMIO_QUOTIENT_VERSION
        || quotient.invalid_representative != GUARDED_MMIO_VALID_CHANNELS
        || quotient.valid_behaviors.len() != usize::from(GUARDED_MMIO_VALID_CHANNELS)
        || quotient.invalid_prefix_steps.len()
            != EXACT_COMPILED_MMIO_INPUTS - usize::from(GUARDED_MMIO_VALID_CHANNELS)
    {
        return Err(reject("quotient shape is not canonical"));
    }

    let mut verifier_work = 0u64;
    for input in 0..GUARDED_MMIO_VALID_CHANNELS {
        let execution = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("verify valid input {input}: {error}")))?
            .finish()
            .map_err(|error| reject(format!("verify valid input {input}: {error}")))?;
        add_work(&mut verifier_work, execution.steps)?;
        if behavior(execution) != quotient.valid_behaviors[usize::from(input)] {
            return Err(reject(format!("valid input {input} behavior mismatch")));
        }
    }

    let representative_steps = quotient.invalid_prefix_steps[0];
    if representative_steps == 0 {
        return Err(reject("invalid representative prefix is empty"));
    }
    let mut representative =
        Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(GUARDED_MMIO_VALID_CHANNELS))
            .map_err(|error| reject(format!("verify invalid representative: {error}")))?;
    while representative.steps() < representative_steps {
        representative
            .step()
            .map_err(|error| reject(format!("verify invalid representative: {error}")))?;
    }
    add_work(&mut verifier_work, representative.steps())?;

    for input in (GUARDED_MMIO_VALID_CHANNELS + 1)..=u8::MAX {
        let declared =
            quotient.invalid_prefix_steps[usize::from(input - GUARDED_MMIO_VALID_CHANNELS)];
        if declared != representative_steps {
            return Err(reject(format!(
                "invalid input {input} has a noncanonical prefix length"
            )));
        }
        let mut candidate = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("verify invalid input {input}: {error}")))?;
        while candidate.steps() < declared {
            candidate
                .step()
                .map_err(|error| reject(format!("verify invalid input {input}: {error}")))?;
        }
        if candidate != representative {
            return Err(reject(format!(
                "invalid input {input} exact state mismatch"
            )));
        }
        add_work(&mut verifier_work, candidate.steps())?;
    }

    let prefix_steps = representative.steps();
    let invalid_execution = representative
        .finish()
        .map_err(|error| reject(format!("verify shared continuation: {error}")))?;
    let shared_work = invalid_execution
        .steps
        .checked_sub(prefix_steps)
        .ok_or_else(|| reject("verified shared continuation work underflow"))?;
    if shared_work != quotient.shared_continuation_steps {
        return Err(reject("shared continuation work mismatch"));
    }
    add_work(&mut verifier_work, shared_work)?;
    if behavior(invalid_execution) != quotient.invalid_behavior {
        return Err(reject("invalid class behavior mismatch"));
    }

    Ok(GuardedMmioQuotientVerification {
        decoded_instruction_transitions: verifier_work,
    })
}

/// Select the exact quotient when its invariant is established, otherwise run
/// the complete per-input reference without returning a partial quotient.
pub fn build_guarded_mmio_portfolio(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<GuardedMmioPortfolio, ExactCompiledMmioReferenceError> {
    match build_guarded_mmio_quotient(image, symbols) {
        Ok(quotient) => Ok(GuardedMmioPortfolio::Quotient(quotient)),
        Err(_) => {
            build_exact_compiled_mmio_reference(image, symbols).map(GuardedMmioPortfolio::Exact)
        }
    }
}

pub fn verify_guarded_mmio_portfolio(
    portfolio: &GuardedMmioPortfolio,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<(), ExactCompiledMmioReferenceError> {
    match portfolio {
        GuardedMmioPortfolio::Quotient(quotient) => {
            verify_guarded_mmio_quotient(quotient, image, symbols)?;
            Ok(())
        }
        GuardedMmioPortfolio::Exact(reference) => {
            verify_exact_compiled_mmio_reference(reference, image, symbols)
        }
    }
}

/// Execute every value in the complete eight-bit `a0` domain independently.
///
/// Classes are canonical: inputs are visited in ascending order, the first
/// input for a behavior is its representative, and class membership depends
/// only on the return value and complete ordered MMIO stream. Program
/// locations remain per-execution evidence and are never normalized away.
pub fn build_exact_compiled_mmio_reference(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<ExactCompiledMmioReference, ExactCompiledMmioReferenceError> {
    let mut classes: Vec<ExactCompiledMmioClass> = Vec::new();
    let mut executions = Vec::with_capacity(EXACT_COMPILED_MMIO_INPUTS);
    let mut decoded_instruction_transitions = 0u64;

    for input in 0u8..=u8::MAX {
        let execution = execute_compiled_mmio_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        decoded_instruction_transitions = decoded_instruction_transitions
            .checked_add(execution.steps)
            .ok_or_else(|| reject("decoded instruction transition count overflow"))?;
        let behavior = ExactCompiledMmioBehavior {
            return_value: execution.return_value,
            events: execution.events,
        };
        let class_index =
            if let Some(index) = classes.iter().position(|class| class.behavior == behavior) {
                index
            } else {
                classes.push(ExactCompiledMmioClass {
                    representative: input,
                    members: [0; MEMBERSHIP_WORDS],
                    behavior,
                });
                classes.len() - 1
            };
        classes[class_index].members[usize::from(input) / 64] |= 1u64 << (usize::from(input) % 64);
        executions.push(ExactCompiledMmioExecution {
            input,
            class_index: u16::try_from(class_index)
                .map_err(|_| reject("behavior class index exceeds policy"))?,
            steps: execution.steps,
            event_program_locations: execution.event_program_locations,
        });
    }

    if executions.len() != EXACT_COMPILED_MMIO_INPUTS {
        return Err(reject("complete eight-bit input domain was not executed"));
    }
    let members: u32 = classes
        .iter()
        .map(ExactCompiledMmioClass::member_count)
        .sum();
    if members != EXACT_COMPILED_MMIO_INPUTS as u32 {
        return Err(reject("behavior classes are not exhaustive"));
    }

    Ok(ExactCompiledMmioReference {
        version: EXACT_COMPILED_MMIO_REFERENCE_VERSION,
        classes,
        executions,
        decoded_instruction_transitions,
    })
}

/// Deterministically rebuild and compare the complete exact reference.
///
/// This is an integrity check implemented through the same execution engine,
/// not the independent quotient verifier required by the later experiment.
pub fn verify_exact_compiled_mmio_reference(
    reference: &ExactCompiledMmioReference,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<(), ExactCompiledMmioReferenceError> {
    if reference.version != EXACT_COMPILED_MMIO_REFERENCE_VERSION {
        return Err(reject("unsupported exact reference version"));
    }
    let rebuilt = build_exact_compiled_mmio_reference(image, symbols)?;
    if rebuilt != *reference {
        return Err(reject("exact reference does not match rebuilt executions"));
    }
    Ok(())
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
    fn exact_reference_partitions_the_complete_input_domain() {
        let (image, symbols) = parity_image();
        let reference = build_exact_compiled_mmio_reference(&image, symbols).unwrap();
        assert_eq!(reference.version, EXACT_COMPILED_MMIO_REFERENCE_VERSION);
        assert_eq!(reference.executions.len(), EXACT_COMPILED_MMIO_INPUTS);
        assert_eq!(reference.classes.len(), 2);
        assert_eq!(reference.decoded_instruction_transitions, 512);
        assert_eq!(reference.classes[0].representative, 0);
        assert_eq!(reference.classes[0].behavior.return_value, 0);
        assert_eq!(reference.classes[0].member_count(), 128);
        assert!(reference.classes[0].contains(254));
        assert!(!reference.classes[0].contains(255));
        assert_eq!(reference.classes[1].representative, 1);
        assert_eq!(reference.classes[1].behavior.return_value, 1);
        assert_eq!(reference.classes[1].member_count(), 128);
        assert!(reference.classes[1].contains(255));
        verify_exact_compiled_mmio_reference(&reference, &image, symbols).unwrap();
    }

    #[test]
    fn verifier_rejects_changed_membership() {
        let (image, symbols) = parity_image();
        let mut reference = build_exact_compiled_mmio_reference(&image, symbols).unwrap();
        reference.classes[0].members[0] ^= 1;
        assert!(verify_exact_compiled_mmio_reference(&reference, &image, symbols).is_err());
    }

    #[test]
    fn exact_guarded_quotient_reuses_only_a_byte_equal_state() {
        let (image, symbols) = guarded_image();
        let quotient = build_guarded_mmio_quotient(&image, symbols).unwrap();
        assert_eq!(quotient.valid_behaviors.len(), 6);
        assert_eq!(quotient.invalid_prefix_steps, vec![1; 250]);
        assert_eq!(quotient.shared_continuation_steps, 1);
        assert_eq!(quotient.producer_decoded_instruction_transitions, 263);
        let verified = verify_guarded_mmio_quotient(&quotient, &image, symbols).unwrap();
        assert_eq!(verified.decoded_instruction_transitions, 263);
        assert_eq!(quotient.invalid_behavior.return_value, 0);
    }

    #[test]
    fn guarded_quotient_verifier_rejects_route_and_behavior_tampering() {
        let (image, symbols) = guarded_image();
        let quotient = build_guarded_mmio_quotient(&image, symbols).unwrap();

        let mut changed_route = quotient.clone();
        changed_route.invalid_prefix_steps[249] = 2;
        assert!(verify_guarded_mmio_quotient(&changed_route, &image, symbols).is_err());

        let mut changed_behavior = quotient;
        changed_behavior.invalid_behavior.return_value = 7;
        assert!(verify_guarded_mmio_quotient(&changed_behavior, &image, symbols).is_err());
    }

    #[test]
    fn guarded_portfolio_falls_back_to_the_complete_exact_reference() {
        let (image, symbols) = parity_image();
        let portfolio = build_guarded_mmio_portfolio(&image, symbols).unwrap();
        let GuardedMmioPortfolio::Exact(reference) = &portfolio else {
            panic!("nonconvergent inputs must use exact fallback");
        };
        assert_eq!(reference.executions.len(), EXACT_COMPILED_MMIO_INPUTS);
        verify_guarded_mmio_portfolio(&portfolio, &image, symbols).unwrap();
    }
}
