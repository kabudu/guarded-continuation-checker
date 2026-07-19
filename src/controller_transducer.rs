//! Source-bound, proof-carrying symbolic controller transducers.

use std::error::Error;
use std::fmt;

use crate::aiger_obligation::{
    AigerInputPredicate, AigerOutcome, AigerTransition, transducer_row_completeness_cnf,
};
use crate::unsat_proof::{MAX_UNSAT_PROOF_BYTES, generate_unsat_proof, verify_unsat_proof};

pub const CONTROLLER_TRANSDUCER_VERSION: u32 = 1;
pub const MAX_TRANSDUCER_INPUTS: usize = 16;
pub const MAX_TRANSDUCER_LATCHES: usize = 6;
pub const MAX_TRANSDUCER_OUTPUTS: usize = 4;
pub const MAX_TRANSDUCER_CELLS: usize = 256;
pub const MAX_TRANSDUCER_PROOFS: usize = 4_096;
pub const MAX_TRANSDUCER_PROOF_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_TRANSDUCER_ARTIFACT_BYTES: usize = 9 * 1024 * 1024;
const CONTROLLER_TRANSDUCER_MAGIC: &[u8; 8] = b"GCCTRN01";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerTransducerRow {
    pub outcome: AigerOutcome,
    pub witness_input: u64,
    pub proof: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerTransducerCell {
    pub cube: Vec<Option<bool>>,
    pub rows: Vec<ControllerTransducerRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerTransducerObligation {
    pub version: u32,
    pub source_sha256: [u8; 32],
    pub relevant_inputs: Vec<usize>,
    pub observed_outputs: Vec<usize>,
    pub state_count: usize,
    pub cells: Vec<ControllerTransducerCell>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerTransducerSummary {
    pub cells: usize,
    pub rows: usize,
    pub proof_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerTransducerError(pub String);

impl fmt::Display for ControllerTransducerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ControllerTransducerError {}

fn reject(message: impl Into<String>) -> ControllerTransducerError {
    ControllerTransducerError(message.into())
}

fn validate_boundary(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
) -> Result<(), ControllerTransducerError> {
    model
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    if relevant_inputs.len() > MAX_TRANSDUCER_INPUTS
        || model.latches.len() > MAX_TRANSDUCER_LATCHES
        || observed_outputs.len() > MAX_TRANSDUCER_OUTPUTS
        || relevant_inputs
            .iter()
            .any(|&input| input >= model.inputs.len())
        || relevant_inputs.windows(2).any(|pair| pair[0] >= pair[1])
        || observed_outputs
            .iter()
            .any(|&output| output >= model.outputs.len())
        || observed_outputs.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("controller transducer boundary is invalid"));
    }
    Ok(())
}

fn declared_input(relevant_inputs: &[usize], pattern: usize) -> u64 {
    relevant_inputs
        .iter()
        .enumerate()
        .fold(0u64, |declared, (bit, &input)| {
            declared | (u64::from(pattern >> bit & 1 == 1) << input)
        })
}

fn projected_outputs(outputs: u128, observed_outputs: &[usize]) -> u128 {
    observed_outputs
        .iter()
        .enumerate()
        .fold(0u128, |projected, (bit, &output)| {
            projected | (((outputs >> output) & 1) << bit)
        })
}

fn outcome_vector(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    pattern: usize,
) -> Result<Vec<AigerOutcome>, ControllerTransducerError> {
    let input = declared_input(relevant_inputs, pattern);
    (0..model.state_count())
        .map(|source| {
            let (target, outputs) = model
                .evaluate(source, input)
                .map_err(|error| reject(error.to_string()))?;
            Ok(AigerOutcome {
                target,
                outputs: projected_outputs(outputs, observed_outputs),
            })
        })
        .collect()
}

#[derive(Clone)]
struct PartitionCell {
    cube: Vec<Option<bool>>,
    representative: usize,
    outcomes: Vec<AigerOutcome>,
}

fn build_partition(
    bit: usize,
    cube: &mut [Option<bool>],
    patterns: &[usize],
    signatures: &[Vec<AigerOutcome>],
    cells: &mut Vec<PartitionCell>,
) -> Result<(), ControllerTransducerError> {
    let representative = *patterns
        .first()
        .ok_or_else(|| reject("controller transducer partition is empty"))?;
    if patterns
        .iter()
        .all(|&pattern| signatures[pattern] == signatures[representative])
    {
        if cells.len() >= MAX_TRANSDUCER_CELLS {
            return Err(reject("controller transducer cell count exceeds limit"));
        }
        cells.push(PartitionCell {
            cube: cube.to_vec(),
            representative,
            outcomes: signatures[representative].clone(),
        });
        return Ok(());
    }
    if bit >= cube.len() {
        return Err(reject(
            "controller transducer partition failed to separate outcomes",
        ));
    }
    for value in [false, true] {
        cube[bit] = Some(value);
        let branch = patterns
            .iter()
            .copied()
            .filter(|pattern| (pattern >> bit & 1 == 1) == value)
            .collect::<Vec<_>>();
        build_partition(bit + 1, cube, &branch, signatures, cells)?;
    }
    cube[bit] = None;
    Ok(())
}

fn predicate(cube: &[Option<bool>]) -> AigerInputPredicate {
    AigerInputPredicate {
        clauses: cube
            .iter()
            .enumerate()
            .filter_map(|(bit, value)| value.map(|value| vec![(bit, value)]))
            .collect(),
    }
}

pub fn produce_controller_transducer(
    model: &AigerTransition,
    source_sha256: [u8; 32],
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
) -> Result<ControllerTransducerObligation, ControllerTransducerError> {
    validate_boundary(model, relevant_inputs, observed_outputs)?;
    let pattern_count = 1usize << relevant_inputs.len();
    let signatures = (0..pattern_count)
        .map(|pattern| outcome_vector(model, relevant_inputs, observed_outputs, pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let patterns = (0..pattern_count).collect::<Vec<_>>();
    let mut partition = Vec::new();
    build_partition(
        0,
        &mut vec![None; relevant_inputs.len()],
        &patterns,
        &signatures,
        &mut partition,
    )?;
    let proof_count = partition
        .len()
        .checked_mul(model.state_count())
        .ok_or_else(|| reject("controller transducer proof count overflow"))?;
    if proof_count > MAX_TRANSDUCER_PROOFS {
        return Err(reject("controller transducer proof count exceeds limit"));
    }
    let mut total_proof_bytes = 0usize;
    let mut cells = Vec::with_capacity(partition.len());
    for cell in partition {
        let input = declared_input(relevant_inputs, cell.representative);
        let cell_predicate = predicate(&cell.cube);
        let mut rows = Vec::with_capacity(model.state_count());
        for (source, &outcome) in cell.outcomes.iter().enumerate() {
            let cnf = transducer_row_completeness_cnf(
                model,
                relevant_inputs,
                source,
                &cell_predicate,
                observed_outputs,
                &[outcome],
            )
            .map_err(|error| reject(error.to_string()))?;
            let proof = generate_unsat_proof(&cnf).map_err(|error| reject(error.to_string()))?;
            total_proof_bytes = total_proof_bytes
                .checked_add(proof.len())
                .ok_or_else(|| reject("controller transducer proof bytes overflow"))?;
            if total_proof_bytes > MAX_TRANSDUCER_PROOF_BYTES {
                return Err(reject("controller transducer proof bytes exceed limit"));
            }
            rows.push(ControllerTransducerRow {
                outcome,
                witness_input: input,
                proof,
            });
        }
        cells.push(ControllerTransducerCell {
            cube: cell.cube,
            rows,
        });
    }
    Ok(ControllerTransducerObligation {
        version: CONTROLLER_TRANSDUCER_VERSION,
        source_sha256,
        relevant_inputs: relevant_inputs.to_vec(),
        observed_outputs: observed_outputs.to_vec(),
        state_count: model.state_count(),
        cells,
    })
}

fn cube_allows(cube: &[Option<bool>], pattern: usize) -> bool {
    cube.iter()
        .enumerate()
        .all(|(bit, required)| required.is_none_or(|value| (pattern >> bit & 1 == 1) == value))
}

fn witness_respects_boundary(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    cube: &[Option<bool>],
    witness: u64,
) -> bool {
    if model.inputs.len() < u64::BITS as usize && witness >> model.inputs.len() != 0 {
        return false;
    }
    let relevant_mask = relevant_inputs
        .iter()
        .fold(0u64, |mask, &input| mask | (1u64 << input));
    if witness & !relevant_mask != 0 {
        return false;
    }
    cube.iter().enumerate().all(|(bit, required)| {
        required.is_none_or(|value| (witness >> relevant_inputs[bit] & 1 == 1) == value)
    })
}

pub fn verify_controller_transducer(
    model: &AigerTransition,
    expected_source_sha256: [u8; 32],
    obligation: &ControllerTransducerObligation,
) -> Result<ControllerTransducerSummary, ControllerTransducerError> {
    if obligation.version != CONTROLLER_TRANSDUCER_VERSION
        || obligation.source_sha256 != expected_source_sha256
    {
        return Err(reject(
            "controller transducer version or source binding mismatch",
        ));
    }
    validate_boundary(
        model,
        &obligation.relevant_inputs,
        &obligation.observed_outputs,
    )?;
    if obligation.state_count != model.state_count()
        || obligation.cells.is_empty()
        || obligation.cells.len() > MAX_TRANSDUCER_CELLS
    {
        return Err(reject("controller transducer dimensions are invalid"));
    }
    let pattern_count = 1usize << obligation.relevant_inputs.len();
    let mut owner = vec![None; pattern_count];
    let mut previous_representative = None;
    let mut total_proof_bytes = 0usize;
    for (cell_index, cell) in obligation.cells.iter().enumerate() {
        if cell.cube.len() != obligation.relevant_inputs.len()
            || cell.rows.len() != obligation.state_count
        {
            return Err(reject("controller transducer cell dimensions are invalid"));
        }
        let mut wildcard_seen = false;
        for required in &cell.cube {
            if required.is_none() {
                wildcard_seen = true;
            } else if wildcard_seen {
                return Err(reject(
                    "controller transducer cube is not canonical prefix form",
                ));
            }
        }
        let representative = (0..pattern_count)
            .find(|&pattern| cube_allows(&cell.cube, pattern))
            .ok_or_else(|| reject("controller transducer cube is empty"))?;
        if previous_representative.is_some_and(|previous| representative <= previous) {
            return Err(reject(
                "controller transducer cells are not in canonical order",
            ));
        }
        previous_representative = Some(representative);
        for (pattern, pattern_owner) in owner.iter_mut().enumerate() {
            if cube_allows(&cell.cube, pattern) && pattern_owner.replace(cell_index).is_some() {
                return Err(reject("controller transducer cubes overlap"));
            }
        }
        let cell_predicate = predicate(&cell.cube);
        for (source, row) in cell.rows.iter().enumerate() {
            if row.proof.is_empty() || row.proof.len() > MAX_UNSAT_PROOF_BYTES {
                return Err(reject("controller transducer proof size is invalid"));
            }
            total_proof_bytes = total_proof_bytes
                .checked_add(row.proof.len())
                .ok_or_else(|| reject("controller transducer proof bytes overflow"))?;
            if total_proof_bytes > MAX_TRANSDUCER_PROOF_BYTES
                || !witness_respects_boundary(
                    model,
                    &obligation.relevant_inputs,
                    &cell.cube,
                    row.witness_input,
                )
            {
                return Err(reject(
                    "controller transducer witness or proof exceeds boundary",
                ));
            }
            let (target, outputs) = model
                .evaluate(source, row.witness_input)
                .map_err(|error| reject(error.to_string()))?;
            if row.outcome
                != (AigerOutcome {
                    target,
                    outputs: projected_outputs(outputs, &obligation.observed_outputs),
                })
            {
                return Err(reject("controller transducer witness outcome mismatch"));
            }
            let cnf = transducer_row_completeness_cnf(
                model,
                &obligation.relevant_inputs,
                source,
                &cell_predicate,
                &obligation.observed_outputs,
                &[row.outcome],
            )
            .map_err(|error| reject(error.to_string()))?;
            verify_unsat_proof(&cnf, &row.proof).map_err(|error| reject(error.to_string()))?;
        }
    }
    if owner.iter().any(Option::is_none) {
        return Err(reject(
            "controller transducer cubes do not cover input space",
        ));
    }
    let rows = obligation
        .cells
        .len()
        .checked_mul(obligation.state_count)
        .ok_or_else(|| reject("controller transducer row count overflow"))?;
    if rows > MAX_TRANSDUCER_PROOFS {
        return Err(reject("controller transducer row count exceeds limit"));
    }
    Ok(ControllerTransducerSummary {
        cells: obligation.cells.len(),
        rows,
        proof_bytes: total_proof_bytes,
    })
}

fn validate_artifact_shape(
    obligation: &ControllerTransducerObligation,
) -> Result<(), ControllerTransducerError> {
    if obligation.version != CONTROLLER_TRANSDUCER_VERSION
        || obligation.relevant_inputs.len() > MAX_TRANSDUCER_INPUTS
        || obligation.observed_outputs.len() > MAX_TRANSDUCER_OUTPUTS
        || obligation.state_count == 0
        || obligation.state_count > (1usize << MAX_TRANSDUCER_LATCHES)
        || obligation.cells.is_empty()
        || obligation.cells.len() > MAX_TRANSDUCER_CELLS
        || obligation
            .relevant_inputs
            .iter()
            .any(|&input| input > u8::MAX as usize)
        || obligation
            .relevant_inputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || obligation
            .observed_outputs
            .iter()
            .any(|&output| output > u8::MAX as usize)
        || obligation
            .observed_outputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("controller transducer artifact shape is invalid"));
    }
    let mut total_proof_bytes = 0usize;
    for cell in &obligation.cells {
        if cell.cube.len() != obligation.relevant_inputs.len()
            || cell.rows.len() != obligation.state_count
        {
            return Err(reject(
                "controller transducer artifact cell shape is invalid",
            ));
        }
        for row in &cell.rows {
            if row.outcome.target >= obligation.state_count
                || row.outcome.outputs > u8::MAX as u128
                || (obligation.observed_outputs.len() < u8::BITS as usize
                    && row.outcome.outputs >> obligation.observed_outputs.len() != 0)
                || row.proof.is_empty()
                || row.proof.len() > MAX_UNSAT_PROOF_BYTES
            {
                return Err(reject("controller transducer artifact row is invalid"));
            }
            total_proof_bytes = total_proof_bytes
                .checked_add(row.proof.len())
                .ok_or_else(|| reject("controller transducer artifact proof bytes overflow"))?;
            if total_proof_bytes > MAX_TRANSDUCER_PROOF_BYTES {
                return Err(reject(
                    "controller transducer artifact proof bytes exceed limit",
                ));
            }
        }
    }
    Ok(())
}

pub fn encode_controller_transducer(
    obligation: &ControllerTransducerObligation,
) -> Result<Vec<u8>, ControllerTransducerError> {
    validate_artifact_shape(obligation)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(CONTROLLER_TRANSDUCER_MAGIC);
    bytes.extend_from_slice(&obligation.version.to_le_bytes());
    bytes.extend_from_slice(&obligation.source_sha256);
    bytes.push(obligation.relevant_inputs.len() as u8);
    bytes.extend(obligation.relevant_inputs.iter().map(|&input| input as u8));
    bytes.push(obligation.observed_outputs.len() as u8);
    bytes.extend(
        obligation
            .observed_outputs
            .iter()
            .map(|&output| output as u8),
    );
    bytes.push(obligation.state_count as u8);
    bytes.extend_from_slice(&(obligation.cells.len() as u16).to_le_bytes());
    for cell in &obligation.cells {
        bytes.extend(cell.cube.iter().map(|required| match required {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        }));
        for row in &cell.rows {
            bytes.push(row.outcome.target as u8);
            bytes.push(row.outcome.outputs as u8);
            bytes.extend_from_slice(&row.witness_input.to_le_bytes());
            bytes.extend_from_slice(&(row.proof.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&row.proof);
        }
    }
    if bytes.len() > MAX_TRANSDUCER_ARTIFACT_BYTES {
        return Err(reject("controller transducer artifact exceeds byte limit"));
    }
    Ok(bytes)
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    count: usize,
) -> Result<&'a [u8], ControllerTransducerError> {
    let end = cursor
        .checked_add(count)
        .ok_or_else(|| reject("controller transducer artifact cursor overflow"))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| reject("controller transducer artifact is truncated"))?;
    *cursor = end;
    Ok(value)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, ControllerTransducerError> {
    Ok(take(bytes, cursor, 1)?[0])
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, ControllerTransducerError> {
    Ok(u16::from_le_bytes(
        take(bytes, cursor, 2)?
            .try_into()
            .map_err(|_| reject("controller transducer u16 decode failed"))?,
    ))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, ControllerTransducerError> {
    Ok(u32::from_le_bytes(
        take(bytes, cursor, 4)?
            .try_into()
            .map_err(|_| reject("controller transducer u32 decode failed"))?,
    ))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, ControllerTransducerError> {
    Ok(u64::from_le_bytes(
        take(bytes, cursor, 8)?
            .try_into()
            .map_err(|_| reject("controller transducer u64 decode failed"))?,
    ))
}

pub fn decode_controller_transducer(
    bytes: &[u8],
) -> Result<ControllerTransducerObligation, ControllerTransducerError> {
    if bytes.len() > MAX_TRANSDUCER_ARTIFACT_BYTES {
        return Err(reject("controller transducer artifact exceeds byte limit"));
    }
    let mut cursor = 0usize;
    if take(bytes, &mut cursor, CONTROLLER_TRANSDUCER_MAGIC.len())? != CONTROLLER_TRANSDUCER_MAGIC {
        return Err(reject("controller transducer artifact magic mismatch"));
    }
    let version = read_u32(bytes, &mut cursor)?;
    if version != CONTROLLER_TRANSDUCER_VERSION {
        return Err(reject("controller transducer artifact version mismatch"));
    }
    let source_sha256 = take(bytes, &mut cursor, 32)?
        .try_into()
        .map_err(|_| reject("controller transducer source digest decode failed"))?;
    let input_count = read_u8(bytes, &mut cursor)? as usize;
    if input_count > MAX_TRANSDUCER_INPUTS {
        return Err(reject("controller transducer input count exceeds limit"));
    }
    let relevant_inputs = take(bytes, &mut cursor, input_count)?
        .iter()
        .map(|&input| input as usize)
        .collect::<Vec<_>>();
    let output_count = read_u8(bytes, &mut cursor)? as usize;
    if output_count > MAX_TRANSDUCER_OUTPUTS {
        return Err(reject("controller transducer output count exceeds limit"));
    }
    let observed_outputs = take(bytes, &mut cursor, output_count)?
        .iter()
        .map(|&output| output as usize)
        .collect::<Vec<_>>();
    let state_count = read_u8(bytes, &mut cursor)? as usize;
    let cell_count = read_u16(bytes, &mut cursor)? as usize;
    if state_count == 0
        || state_count > (1usize << MAX_TRANSDUCER_LATCHES)
        || cell_count == 0
        || cell_count > MAX_TRANSDUCER_CELLS
        || cell_count
            .checked_mul(state_count)
            .is_none_or(|proofs| proofs > MAX_TRANSDUCER_PROOFS)
    {
        return Err(reject(
            "controller transducer artifact dimensions exceed limits",
        ));
    }
    let mut total_proof_bytes = 0usize;
    let mut cells = Vec::with_capacity(cell_count);
    for _ in 0..cell_count {
        let cube = take(bytes, &mut cursor, input_count)?
            .iter()
            .map(|symbol| match symbol {
                0 => Ok(None),
                1 => Ok(Some(false)),
                2 => Ok(Some(true)),
                _ => Err(reject("controller transducer cube symbol is invalid")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut rows = Vec::with_capacity(state_count);
        for _ in 0..state_count {
            let target = read_u8(bytes, &mut cursor)? as usize;
            let outputs = read_u8(bytes, &mut cursor)? as u128;
            let witness_input = read_u64(bytes, &mut cursor)?;
            let proof_len = read_u32(bytes, &mut cursor)? as usize;
            if proof_len == 0 || proof_len > MAX_UNSAT_PROOF_BYTES {
                return Err(reject("controller transducer proof length is invalid"));
            }
            total_proof_bytes = total_proof_bytes
                .checked_add(proof_len)
                .ok_or_else(|| reject("controller transducer proof bytes overflow"))?;
            if total_proof_bytes > MAX_TRANSDUCER_PROOF_BYTES {
                return Err(reject("controller transducer proof bytes exceed limit"));
            }
            let proof = take(bytes, &mut cursor, proof_len)?.to_vec();
            rows.push(ControllerTransducerRow {
                outcome: AigerOutcome { target, outputs },
                witness_input,
                proof,
            });
        }
        cells.push(ControllerTransducerCell { cube, rows });
    }
    if cursor != bytes.len() {
        return Err(reject("controller transducer artifact has trailing bytes"));
    }
    let obligation = ControllerTransducerObligation {
        version,
        source_sha256,
        relevant_inputs,
        observed_outputs,
        state_count,
        cells,
    };
    validate_artifact_shape(&obligation)?;
    Ok(obligation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiger_obligation::AigerLatch;

    fn input_driven() -> AigerTransition {
        AigerTransition {
            max_variable: 2,
            inputs: vec![2],
            latches: vec![AigerLatch {
                current: 4,
                next: 2,
            }],
            outputs: vec![2],
            ands: vec![],
        }
    }

    #[test]
    fn produces_two_exact_cells_and_verifies_independently() {
        let model = input_driven();
        let digest = [7; 32];
        let first = produce_controller_transducer(&model, digest, &[0], &[0]).unwrap();
        let second = produce_controller_transducer(&model, digest, &[0], &[0]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.cells.len(), 2);
        assert_eq!(first.cells[0].cube, [Some(false)]);
        assert_eq!(first.cells[1].cube, [Some(true)]);
        assert_eq!(
            verify_controller_transducer(&model, digest, &first).unwrap(),
            ControllerTransducerSummary {
                cells: 2,
                rows: 4,
                proof_bytes: first
                    .cells
                    .iter()
                    .flat_map(|cell| &cell.rows)
                    .map(|row| row.proof.len())
                    .sum(),
            }
        );
    }

    #[test]
    fn source_boundary_witness_outcome_and_proof_tampering_fail_closed() {
        let model = input_driven();
        let digest = [9; 32];
        let obligation = produce_controller_transducer(&model, digest, &[0], &[0]).unwrap();
        assert!(verify_controller_transducer(&model, [8; 32], &obligation).is_err());

        let mut tampered = obligation.clone();
        tampered.relevant_inputs.clear();
        assert!(verify_controller_transducer(&model, digest, &tampered).is_err());

        let mut tampered = obligation.clone();
        tampered.cells[0].rows[0].witness_input = 1;
        assert!(verify_controller_transducer(&model, digest, &tampered).is_err());

        let mut tampered = obligation.clone();
        tampered.cells[0].rows[0].outcome.outputs ^= 1;
        assert!(verify_controller_transducer(&model, digest, &tampered).is_err());

        let mut tampered = obligation;
        tampered.cells[0].rows[0].proof.pop();
        assert!(verify_controller_transducer(&model, digest, &tampered).is_err());
    }

    #[test]
    fn incomplete_sensed_input_boundary_is_rejected_by_completeness_proof() {
        let model = input_driven();
        assert!(produce_controller_transducer(&model, [1; 32], &[], &[0]).is_err());
    }

    #[test]
    fn codec_is_canonical_bounded_and_hostile_mutations_fail_closed() {
        let model = input_driven();
        let digest = [3; 32];
        let obligation = produce_controller_transducer(&model, digest, &[0], &[0]).unwrap();
        let encoded = encode_controller_transducer(&obligation).unwrap();
        let decoded = decode_controller_transducer(&encoded).unwrap();
        assert_eq!(decoded, obligation);
        assert_eq!(encode_controller_transducer(&decoded).unwrap(), encoded);

        for length in 0..encoded.len() {
            assert!(decode_controller_transducer(&encoded[..length]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] ^= 1;
            let rejected = decode_controller_transducer(&mutated)
                .and_then(|artifact| verify_controller_transducer(&model, digest, &artifact))
                .is_err();
            assert!(rejected, "mutation at byte {index} was accepted");
        }
    }
}
