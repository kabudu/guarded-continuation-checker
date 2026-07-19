//! Proof-carrying equivalence between an AIGER controller and an exact MTBDD.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::aiger_obligation::AigerTransition;
use crate::controller_mtbdd::{
    ControllerMtbddArtifact, encode_controller_mtbdd, validate_controller_mtbdd_structure,
};
use crate::unsat_proof::{CnfClause, generate_unsat_proof, verify_unsat_proof};

pub const CONTROLLER_MTBDD_EQUIVALENCE_VERSION: u32 = 1;
pub const MAX_EQUIVALENCE_CNF_CLAUSES: usize = 100_000;
pub const MAX_EQUIVALENCE_CNF_LITERALS: usize = 400_000;
pub const MAX_EQUIVALENCE_ARTIFACT_BYTES: usize = 2 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"GCCMEP01";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddEquivalenceProof {
    pub version: u32,
    pub controller_source_sha256: [u8; 32],
    pub mtbdd_sha256: [u8; 32],
    pub cnf_variables: usize,
    pub cnf_clauses: usize,
    pub cnf_literals: usize,
    pub proof: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddEquivalenceSummary {
    pub cnf_variables: usize,
    pub cnf_clauses: usize,
    pub cnf_literals: usize,
    pub proof_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddEquivalenceError(pub String);

impl fmt::Display for ControllerMtbddEquivalenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ControllerMtbddEquivalenceError {}

fn reject(message: impl Into<String>) -> ControllerMtbddEquivalenceError {
    ControllerMtbddEquivalenceError(message.into())
}

fn narrow(value: usize, field: &str) -> Result<u32, ControllerMtbddEquivalenceError> {
    u32::try_from(value).map_err(|_| reject(format!("{field} exceeds canonical range")))
}

pub fn encode_controller_mtbdd_equivalence_proof(
    artifact: &ControllerMtbddEquivalenceProof,
) -> Result<Vec<u8>, ControllerMtbddEquivalenceError> {
    if artifact.version != CONTROLLER_MTBDD_EQUIVALENCE_VERSION
        || artifact.cnf_variables == 0
        || artifact.cnf_clauses == 0
        || artifact.cnf_clauses > MAX_EQUIVALENCE_CNF_CLAUSES
        || artifact.cnf_literals == 0
        || artifact.cnf_literals > MAX_EQUIVALENCE_CNF_LITERALS
        || artifact.proof.is_empty()
        || artifact.proof.len() > crate::unsat_proof::MAX_UNSAT_PROOF_BYTES
    {
        return Err(reject(
            "controller MTBDD equivalence artifact shape is invalid",
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.controller_source_sha256);
    bytes.extend_from_slice(&artifact.mtbdd_sha256);
    bytes.extend_from_slice(&narrow(artifact.cnf_variables, "CNF variables")?.to_le_bytes());
    bytes.extend_from_slice(&narrow(artifact.cnf_clauses, "CNF clauses")?.to_le_bytes());
    bytes.extend_from_slice(&narrow(artifact.cnf_literals, "CNF literals")?.to_le_bytes());
    bytes.extend_from_slice(&narrow(artifact.proof.len(), "proof length")?.to_le_bytes());
    bytes.extend_from_slice(&artifact.proof);
    bytes.extend_from_slice(&Sha256::digest(&bytes));
    if bytes.len() > MAX_EQUIVALENCE_ARTIFACT_BYTES {
        return Err(reject(
            "controller MTBDD equivalence artifact exceeds byte limit",
        ));
    }
    Ok(bytes)
}

pub fn decode_controller_mtbdd_equivalence_proof(
    bytes: &[u8],
) -> Result<ControllerMtbddEquivalenceProof, ControllerMtbddEquivalenceError> {
    const HEADER_BYTES: usize = 8 + 4 + 32 + 32 + 4 + 4 + 4 + 4;
    if bytes.len() < HEADER_BYTES + 1 + 32 || bytes.len() > MAX_EQUIVALENCE_ARTIFACT_BYTES {
        return Err(reject(
            "controller MTBDD equivalence artifact size is invalid",
        ));
    }
    let payload_len = bytes.len() - 32;
    let (payload, integrity) = bytes.split_at(payload_len);
    if Sha256::digest(payload).as_slice() != integrity {
        return Err(reject(
            "controller MTBDD equivalence artifact integrity mismatch",
        ));
    }
    let mut cursor = 0usize;
    let mut take = |count: usize| -> Result<&[u8], ControllerMtbddEquivalenceError> {
        let end = cursor
            .checked_add(count)
            .ok_or_else(|| reject("controller MTBDD equivalence cursor overflow"))?;
        let value = payload
            .get(cursor..end)
            .ok_or_else(|| reject("controller MTBDD equivalence artifact is truncated"))?;
        cursor = end;
        Ok(value)
    };
    if take(8)? != MAGIC {
        return Err(reject(
            "controller MTBDD equivalence artifact magic mismatch",
        ));
    }
    let read_u32 = |bytes: &[u8]| -> Result<u32, ControllerMtbddEquivalenceError> {
        Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
            reject("controller MTBDD equivalence integer decode failed")
        })?))
    };
    let version = read_u32(take(4)?)?;
    let controller_source_sha256 = take(32)?
        .try_into()
        .map_err(|_| reject("controller source digest decode failed"))?;
    let mtbdd_sha256 = take(32)?
        .try_into()
        .map_err(|_| reject("controller MTBDD digest decode failed"))?;
    let cnf_variables = read_u32(take(4)?)? as usize;
    let cnf_clauses = read_u32(take(4)?)? as usize;
    let cnf_literals = read_u32(take(4)?)? as usize;
    let proof_len = read_u32(take(4)?)? as usize;
    let proof = take(proof_len)?.to_vec();
    if cursor != payload.len() {
        return Err(reject(
            "controller MTBDD equivalence artifact has trailing bytes",
        ));
    }
    let artifact = ControllerMtbddEquivalenceProof {
        version,
        controller_source_sha256,
        mtbdd_sha256,
        cnf_variables,
        cnf_clauses,
        cnf_literals,
        proof,
    };
    encode_controller_mtbdd_equivalence_proof(&artifact)?;
    Ok(artifact)
}

#[derive(Clone, Copy)]
enum Boolean {
    Constant(bool),
    Variable(usize, bool),
}

impl Boolean {
    fn negate(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Variable(variable, positive) => Self::Variable(variable, !positive),
        }
    }
}

fn aiger_literal(literal: usize) -> Boolean {
    match literal {
        0 => Boolean::Constant(false),
        1 => Boolean::Constant(true),
        _ => Boolean::Variable(literal / 2 - 1, literal.is_multiple_of(2)),
    }
}

fn push_clause(clauses: &mut Vec<CnfClause>, literals: &[Boolean]) {
    let mut clause = Vec::new();
    for literal in literals {
        match *literal {
            Boolean::Constant(true) => return,
            Boolean::Constant(false) => {}
            Boolean::Variable(variable, positive) => clause.push((variable, positive)),
        }
    }
    clause.sort_unstable();
    clause.dedup();
    if clause
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && pair[0].1 != pair[1].1)
    {
        return;
    }
    clauses.push(CnfClause(clause));
}

fn append_aiger(clauses: &mut Vec<CnfClause>, model: &AigerTransition) {
    for gate in &model.ands {
        let output = aiger_literal(gate.output);
        let left = aiger_literal(gate.left);
        let right = aiger_literal(gate.right);
        push_clause(clauses, &[output.negate(), left]);
        push_clause(clauses, &[output.negate(), right]);
        push_clause(clauses, &[output, left.negate(), right.negate()]);
    }
}

fn append_mux(
    clauses: &mut Vec<CnfClause>,
    output: Boolean,
    select: Boolean,
    low: Boolean,
    high: Boolean,
) {
    push_clause(clauses, &[select, low.negate(), output]);
    push_clause(clauses, &[select, low, output.negate()]);
    push_clause(clauses, &[select.negate(), high.negate(), output]);
    push_clause(clauses, &[select.negate(), high, output.negate()]);
}

fn append_xor(clauses: &mut Vec<CnfClause>, output: Boolean, left: Boolean, right: Boolean) {
    push_clause(clauses, &[left.negate(), right.negate(), output.negate()]);
    push_clause(clauses, &[left, right, output.negate()]);
    push_clause(clauses, &[left, right.negate(), output]);
    push_clause(clauses, &[left.negate(), right, output]);
}

fn equivalence_cnf(
    model: &AigerTransition,
    artifact: &ControllerMtbddArtifact,
) -> Result<(Vec<CnfClause>, usize, usize), ControllerMtbddEquivalenceError> {
    model
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    let state_bits =
        validate_controller_mtbdd_structure(artifact).map_err(|error| reject(error.to_string()))?;
    if artifact.state_count != model.state_count()
        || artifact
            .relevant_inputs
            .iter()
            .any(|&input| input >= model.inputs.len())
        || artifact
            .observed_outputs
            .iter()
            .any(|&output| output >= model.outputs.len())
    {
        return Err(reject("controller MTBDD equivalence boundary is invalid"));
    }
    let output_bits = state_bits
        .checked_add(artifact.observed_outputs.len())
        .ok_or_else(|| reject("controller MTBDD equivalence output width overflow"))?;
    let estimated_clauses = model
        .ands
        .len()
        .checked_mul(3)
        .and_then(|value| {
            value.checked_add(
                artifact
                    .nodes
                    .len()
                    .saturating_mul(output_bits)
                    .saturating_mul(4),
            )
        })
        .and_then(|value| value.checked_add(output_bits.saturating_mul(4) + 1))
        .ok_or_else(|| reject("controller MTBDD equivalence clause count overflow"))?;
    if estimated_clauses > MAX_EQUIVALENCE_CNF_CLAUSES {
        return Err(reject(
            "controller MTBDD equivalence CNF exceeds clause limit",
        ));
    }
    let mut clauses = Vec::with_capacity(estimated_clauses);
    append_aiger(&mut clauses, model);

    let relevant = artifact
        .relevant_inputs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for (index, &literal) in model.inputs.iter().enumerate() {
        if !relevant.contains(&index) {
            push_clause(&mut clauses, &[aiger_literal(literal).negate()]);
        }
    }

    let selectors = model
        .latches
        .iter()
        .map(|latch| aiger_literal(latch.current))
        .chain(
            artifact
                .relevant_inputs
                .iter()
                .map(|&input| aiger_literal(model.inputs[input])),
        )
        .collect::<Vec<_>>();
    let mut next_variable = model.max_variable;
    let mut references = artifact
        .terminals
        .iter()
        .map(|terminal| {
            (0..output_bits)
                .map(|bit| {
                    let value = if bit < state_bits {
                        terminal.target >> bit & 1 == 1
                    } else {
                        terminal.outputs >> (bit - state_bits) & 1 == 1
                    };
                    Boolean::Constant(value)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for node in &artifact.nodes {
        let low = references
            .get(node.low)
            .ok_or_else(|| reject("controller MTBDD equivalence low reference is invalid"))?;
        let high = references
            .get(node.high)
            .ok_or_else(|| reject("controller MTBDD equivalence high reference is invalid"))?;
        let select = *selectors
            .get(node.variable)
            .ok_or_else(|| reject("controller MTBDD equivalence selector is invalid"))?;
        let mut outputs = Vec::with_capacity(output_bits);
        for bit in 0..output_bits {
            let output = Boolean::Variable(next_variable, true);
            next_variable += 1;
            append_mux(&mut clauses, output, select, low[bit], high[bit]);
            outputs.push(output);
        }
        references.push(outputs);
    }
    let root = references
        .get(artifact.root)
        .ok_or_else(|| reject("controller MTBDD equivalence root is invalid"))?;
    let expected = model
        .latches
        .iter()
        .map(|latch| aiger_literal(latch.next))
        .chain(
            artifact
                .observed_outputs
                .iter()
                .map(|&output| aiger_literal(model.outputs[output])),
        )
        .collect::<Vec<_>>();
    let mut differences = Vec::with_capacity(output_bits);
    for bit in 0..output_bits {
        let difference = Boolean::Variable(next_variable, true);
        next_variable += 1;
        append_xor(&mut clauses, difference, expected[bit], root[bit]);
        differences.push(difference);
    }
    push_clause(&mut clauses, &differences);
    let literal_count = clauses
        .iter()
        .try_fold(0usize, |total, clause| total.checked_add(clause.0.len()))
        .ok_or_else(|| reject("controller MTBDD equivalence literal count overflow"))?;
    if clauses.len() > MAX_EQUIVALENCE_CNF_CLAUSES || literal_count > MAX_EQUIVALENCE_CNF_LITERALS {
        return Err(reject("controller MTBDD equivalence CNF exceeds limits"));
    }
    Ok((clauses, next_variable, literal_count))
}

pub fn produce_controller_mtbdd_equivalence_proof(
    model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    artifact: &ControllerMtbddArtifact,
) -> Result<ControllerMtbddEquivalenceProof, ControllerMtbddEquivalenceError> {
    if artifact.source_sha256 != controller_source_sha256 {
        return Err(reject(
            "controller MTBDD equivalence source binding mismatch",
        ));
    }
    let encoded = encode_controller_mtbdd(artifact).map_err(|error| reject(error.to_string()))?;
    let (cnf, variables, literals) = equivalence_cnf(model, artifact)?;
    let proof = generate_unsat_proof(&cnf).map_err(|error| reject(error.to_string()))?;
    Ok(ControllerMtbddEquivalenceProof {
        version: CONTROLLER_MTBDD_EQUIVALENCE_VERSION,
        controller_source_sha256,
        mtbdd_sha256: Sha256::digest(encoded).into(),
        cnf_variables: variables,
        cnf_clauses: cnf.len(),
        cnf_literals: literals,
        proof,
    })
}

pub fn verify_controller_mtbdd_equivalence_proof(
    model: &AigerTransition,
    controller_source_sha256: [u8; 32],
    artifact: &ControllerMtbddArtifact,
    proof: &ControllerMtbddEquivalenceProof,
) -> Result<ControllerMtbddEquivalenceSummary, ControllerMtbddEquivalenceError> {
    let encoded = encode_controller_mtbdd(artifact).map_err(|error| reject(error.to_string()))?;
    if proof.version != CONTROLLER_MTBDD_EQUIVALENCE_VERSION
        || proof.controller_source_sha256 != controller_source_sha256
        || artifact.source_sha256 != controller_source_sha256
        || proof.mtbdd_sha256 != <[u8; 32]>::from(Sha256::digest(encoded))
    {
        return Err(reject("controller MTBDD equivalence binding mismatch"));
    }
    let (cnf, variables, literals) = equivalence_cnf(model, artifact)?;
    if proof.cnf_variables != variables
        || proof.cnf_clauses != cnf.len()
        || proof.cnf_literals != literals
    {
        return Err(reject("controller MTBDD equivalence CNF metadata mismatch"));
    }
    verify_unsat_proof(&cnf, &proof.proof).map_err(|error| reject(error.to_string()))?;
    Ok(ControllerMtbddEquivalenceSummary {
        cnf_variables: variables,
        cnf_clauses: cnf.len(),
        cnf_literals: literals,
        proof_bytes: proof.proof.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiger_obligation::AigerLatch;
    use crate::controller_mtbdd::produce_controller_mtbdd;

    fn controller() -> AigerTransition {
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
    fn equivalence_proof_replaces_assignment_replay_and_rejects_drift() {
        let model = controller();
        let digest = [0x51; 32];
        let mtbdd = produce_controller_mtbdd(&model, digest, &[0], &[0]).unwrap();
        let proof = produce_controller_mtbdd_equivalence_proof(&model, digest, &mtbdd).unwrap();
        let encoded = encode_controller_mtbdd_equivalence_proof(&proof).unwrap();
        assert_eq!(
            decode_controller_mtbdd_equivalence_proof(&encoded).unwrap(),
            proof
        );
        for end in 0..encoded.len() {
            assert!(decode_controller_mtbdd_equivalence_proof(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut corrupted = encoded.clone();
            corrupted[index] ^= 1;
            assert!(decode_controller_mtbdd_equivalence_proof(&corrupted).is_err());
        }
        let summary =
            verify_controller_mtbdd_equivalence_proof(&model, digest, &mtbdd, &proof).unwrap();
        assert!(summary.cnf_clauses > 0);
        assert!(summary.proof_bytes > 0);

        let mut wrong_digest = proof.clone();
        wrong_digest.mtbdd_sha256[0] ^= 1;
        assert!(
            verify_controller_mtbdd_equivalence_proof(&model, digest, &mtbdd, &wrong_digest,)
                .is_err()
        );
        let mut wrong_proof = proof;
        wrong_proof.proof[0] ^= 1;
        assert!(
            verify_controller_mtbdd_equivalence_proof(&model, digest, &mtbdd, &wrong_proof,)
                .is_err()
        );

        let mut wrong_mtbdd = mtbdd;
        wrong_mtbdd.terminals[0].outputs ^= 1;
        assert!(produce_controller_mtbdd_equivalence_proof(&model, digest, &wrong_mtbdd).is_err());
    }
}
