//! Independent scalar checker for finite-domain predicate transducer evidence.
//!
//! The producer uses lane-valued registers and sparse lane-valued memory. This
//! checker deliberately uses one ordinary concrete replay machine per input.
//! It shares only the certified decode sequence, then checks each lane's own
//! instruction bytes, control flow and terminal behavior.

use crate::riscv32imc::{
    MAX_RV32_IMAGE_BYTES, RV32_IMAGE_BASE, Rv32Error, Rv32ReplayMachine, Rv32SymbolLayout,
};
use crate::riscv32imc_predicate::{
    INVALID_PREDICATE_FIRST, INVALID_PREDICATE_LANES, PredicateTransducerExecution,
};
use riscv_decode::decode;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredicateReplayVerification {
    pub decoded_transitions: u64,
    pub scalar_lane_steps: u64,
    pub lanes_checked: u16,
}

fn reject(message: impl Into<String>) -> Rv32Error {
    Rv32Error(format!(
        "independent predicate replay checker: {}",
        message.into()
    ))
}

pub fn verify_invalid_channel_predicate(
    image: &[u8],
    symbols: Rv32SymbolLayout,
    claimed: &PredicateTransducerExecution,
) -> Result<PredicateReplayVerification, Rv32Error> {
    if image.is_empty() || image.len() > MAX_RV32_IMAGE_BYTES {
        return Err(reject("image size is outside policy"));
    }
    if claimed.first_input != INVALID_PREDICATE_FIRST
        || usize::from(claimed.lane_count) != INVALID_PREDICATE_LANES
    {
        return Err(reject("claimed predicate domain is not canonical"));
    }
    let expected_steps = u64::try_from(claimed.control_trace.len())
        .map_err(|_| reject("control trace length overflow"))?;
    if claimed.symbolic_transitions != expected_steps {
        return Err(reject(
            "symbolic transition count does not match control trace",
        ));
    }
    let expected_lane_operations = expected_steps
        .checked_mul(INVALID_PREDICATE_LANES as u64)
        .ok_or_else(|| reject("lane operation count overflow"))?;
    if claimed.lane_value_operations != expected_lane_operations {
        return Err(reject("lane operation count does not match trace domain"));
    }
    let image_end = RV32_IMAGE_BASE
        .checked_add(image.len() as u32)
        .ok_or_else(|| reject("image end overflow"))?;
    let mut lanes = (INVALID_PREDICATE_FIRST..=u8::MAX)
        .map(|input| Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input)))
        .collect::<Result<Vec<_>, _>>()?;
    let stop = RV32_IMAGE_BASE
        .checked_add(crate::riscv32imc::MAX_RV32_MEMORY_BYTES as u32 - 4)
        .ok_or_else(|| reject("stop address overflow"))?;
    let mut written_bytes = (stop..stop + 4).collect::<BTreeSet<_>>();

    for (index, step) in claimed.control_trace.iter().enumerate() {
        if index > 0
            && claimed.control_trace[index - 1].next_program_counter != step.program_counter
        {
            return Err(reject(format!(
                "control trace is discontinuous before transition {index}"
            )));
        }
        let instruction = decode(step.instruction_word).map_err(|error| {
            reject(format!(
                "control trace decode failed at transition {index}: {error:?}"
            ))
        })?;
        let mut shared_observation = None;
        for (lane_index, lane) in lanes.iter_mut().enumerate() {
            let observation = lane
                .step_predecoded(
                    step.program_counter,
                    step.instruction_word,
                    step.instruction_bytes,
                    &instruction,
                    image_end,
                )
                .map_err(|error| {
                    reject(format!("lane {lane_index}, transition {index}: {error}"))
                })?;
            if let Some(expected) = &shared_observation {
                if &observation != expected {
                    return Err(reject(format!(
                        "lane {lane_index} has nonuniform effects at transition {index}"
                    )));
                }
            } else {
                for write in &observation.writes {
                    for byte in 0..u32::from(write.width) {
                        written_bytes.insert(write.address + byte);
                    }
                }
                shared_observation = Some(observation);
            }
            if lane.program_counter() != step.next_program_counter {
                return Err(reject(format!(
                    "lane {lane_index} diverged after transition {index}"
                )));
            }
        }
    }
    if u32::try_from(written_bytes.len())
        .map_err(|_| reject("sparse memory byte count overflow"))?
        != claimed.sparse_memory_bytes
    {
        return Err(reject(
            "sparse memory byte count does not match independently observed stores",
        ));
    }

    for (lane_index, lane) in lanes.into_iter().enumerate() {
        if !lane.is_complete() {
            return Err(reject(format!(
                "lane {lane_index} did not terminate at the certified trace boundary"
            )));
        }
        let execution = lane.finish()?;
        if execution.return_value != claimed.return_value
            || execution.events != claimed.events
            || execution.event_program_locations != claimed.event_program_locations
        {
            return Err(reject(format!(
                "lane {lane_index} terminal behavior differs from the claim"
            )));
        }
    }

    Ok(PredicateReplayVerification {
        decoded_transitions: expected_steps,
        scalar_lane_steps: expected_lane_operations,
        lanes_checked: INVALID_PREDICATE_LANES as u16,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riscv32imc_predicate::execute_invalid_channel_predicate;

    type ClaimMutation = Box<dyn Fn(&mut PredicateTransducerExecution)>;

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
    fn independently_replays_every_invalid_input() {
        let (image, symbols) = guarded_image();
        let claim = execute_invalid_channel_predicate(&image, symbols).unwrap();
        let verification = verify_invalid_channel_predicate(&image, symbols, &claim).unwrap();
        assert_eq!(verification.decoded_transitions, 2);
        assert_eq!(verification.scalar_lane_steps, 500);
        assert_eq!(verification.lanes_checked, 250);
    }

    #[test]
    fn rejects_control_and_terminal_claim_mutations() {
        let (image, symbols) = guarded_image();
        let original = execute_invalid_channel_predicate(&image, symbols).unwrap();
        let mutations: [ClaimMutation; 11] = [
            Box::new(|claim| claim.first_input = 7),
            Box::new(|claim| claim.lane_count = 249),
            Box::new(|claim| claim.symbolic_transitions += 1),
            Box::new(|claim| claim.lane_value_operations -= 1),
            Box::new(|claim| claim.sparse_memory_bytes += 1),
            Box::new(|claim| claim.control_trace[0].program_counter += 2),
            Box::new(|claim| claim.control_trace[0].instruction_word ^= 0x1000),
            Box::new(|claim| claim.control_trace[0].instruction_bytes = 3),
            Box::new(|claim| claim.control_trace[0].next_program_counter += 2),
            Box::new(|claim| {
                claim.control_trace.pop();
                claim.symbolic_transitions -= 1;
                claim.lane_value_operations -= INVALID_PREDICATE_LANES as u64;
            }),
            Box::new(|claim| claim.return_value ^= 1),
        ];
        for mutate in mutations {
            let mut claim = original.clone();
            mutate(&mut claim);
            assert!(verify_invalid_channel_predicate(&image, symbols, &claim).is_err());
        }
    }

    #[test]
    fn rejects_claim_replayed_against_changed_code() {
        let (mut image, symbols) = guarded_image();
        let claim = execute_invalid_channel_predicate(&image, symbols).unwrap();
        image[0] ^= 0x10;
        assert!(verify_invalid_channel_predicate(&image, symbols, &claim).is_err());
    }
}
