//! Exact all-input reference for guarded compiled-MMIO continuation research.
//!
//! This module deliberately performs every execution independently. It is the
//! fail-closed reference and work-accounting denominator for a later
//! continuation quotient, not an optimized producer.

use crate::riscv32imc::{CompiledMmioEvent, Rv32SymbolLayout, execute_compiled_mmio_with_a0};
use std::{error::Error, fmt};

pub const EXACT_COMPILED_MMIO_REFERENCE_VERSION: u32 = 1;
pub const EXACT_COMPILED_MMIO_INPUTS: usize = 256;
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
}
