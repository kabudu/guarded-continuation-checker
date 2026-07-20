//! Resource-bounded generation and independent checking of Varisat UNSAT proofs.

use std::error::Error;
use std::fmt;

use varisat::{ExtendFormula, Lit, ProofFormat, Solver, Var};
use varisat_checker::Checker as VarisatProofChecker;

pub const UNSAT_PROOF_VERSION: u32 = 1;
pub const MAX_UNSAT_PROOF_BYTES: usize = 1024 * 1024;
pub const MAX_UNSAT_PROOF_STEPS: usize = 100_000;
pub const MAX_CNF_CLAUSES: usize = 1_000_000;
pub const MAX_CNF_LITERALS: usize = 4_000_000;
pub const MAX_CNF_VARIABLES: usize = 16_777_216;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CnfClause(pub Vec<(usize, bool)>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsatProofError(pub String);

impl fmt::Display for UnsatProofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for UnsatProofError {}

fn reject(message: impl Into<String>) -> UnsatProofError {
    UnsatProofError(message.into())
}

fn validate_cnf(clauses: &[CnfClause]) -> Result<usize, UnsatProofError> {
    if clauses.is_empty() || clauses.len() > MAX_CNF_CLAUSES {
        return Err(reject("UNSAT proof CNF clause count is outside limit"));
    }
    let mut literals = 0usize;
    let mut max_variable = 0usize;
    for clause in clauses {
        literals = literals
            .checked_add(clause.0.len())
            .ok_or_else(|| reject("UNSAT proof CNF literal count overflow"))?;
        if literals > MAX_CNF_LITERALS {
            return Err(reject("UNSAT proof CNF literal count exceeds limit"));
        }
        for &(variable, _) in &clause.0 {
            if variable >= MAX_CNF_VARIABLES {
                return Err(reject("UNSAT proof CNF variable exceeds limit"));
            }
            max_variable = max_variable.max(variable);
        }
    }
    Ok(max_variable)
}

fn literals(clause: &CnfClause) -> Vec<Lit> {
    clause
        .0
        .iter()
        .map(|&(variable, positive)| Lit::from_var(Var::from_index(variable), positive))
        .collect()
}

pub fn generate_unsat_proof(clauses: &[CnfClause]) -> Result<Vec<u8>, UnsatProofError> {
    validate_cnf(clauses)?;
    let mut proof = Vec::new();
    {
        let mut solver = Solver::new();
        solver.write_proof(&mut proof, ProofFormat::Varisat);
        for clause in clauses {
            solver.add_clause(&literals(clause));
        }
        if solver
            .solve()
            .map_err(|error| reject(format!("solve UNSAT proof obligation: {error}")))?
        {
            return Err(reject("UNSAT proof obligation is satisfiable"));
        }
        solver
            .close_proof()
            .map_err(|error| reject(format!("close UNSAT proof obligation: {error}")))?;
    }
    if proof.len() > MAX_UNSAT_PROOF_BYTES {
        return Err(reject("generated UNSAT proof exceeds byte limit"));
    }
    Ok(proof)
}

fn read_vli(proof: &[u8], offset: &mut usize) -> Result<u64, UnsatProofError> {
    let start = *offset;
    let mut marker = None;
    for byte_index in 0..10 {
        let byte = *proof
            .get(start + byte_index)
            .ok_or_else(|| reject("UNSAT proof has a truncated integer"))?;
        if byte != 0 {
            marker = Some(byte_index * 8 + byte.trailing_zeros() as usize);
            break;
        }
    }
    let marker = marker.ok_or_else(|| reject("UNSAT proof integer is invalid"))?;
    let encoded_len = marker + 1;
    if encoded_len > 10 || start + encoded_len > proof.len() {
        return Err(reject("UNSAT proof integer is out of bounds"));
    }
    let mut raw = 0u128;
    for (index, &byte) in proof[start..start + encoded_len].iter().enumerate() {
        raw |= u128::from(byte) << (index * 8);
    }
    let value = raw >> encoded_len;
    if value > u128::from(u64::MAX) {
        return Err(reject("UNSAT proof integer exceeds u64"));
    }
    *offset += encoded_len;
    Ok(value as u64)
}

fn preflight(clauses: &[CnfClause], proof: &[u8]) -> Result<(), UnsatProofError> {
    const CODE_END: u64 = 0x9ac3_391f_4294_c211;
    if proof.is_empty() || proof.len() > MAX_UNSAT_PROOF_BYTES {
        return Err(reject("UNSAT proof byte count is outside limit"));
    }
    let max_variable = validate_cnf(clauses)?;
    let max_literal_code = max_variable
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| reject("UNSAT proof variable bound overflow"))?
        as u64;
    let mut offset = 0usize;
    let mut steps = 0usize;
    let read_variable = |offset: &mut usize| -> Result<(), UnsatProofError> {
        if read_vli(proof, offset)? > max_variable as u64 {
            return Err(reject("UNSAT proof variable exceeds obligation"));
        }
        Ok(())
    };
    let read_list = |offset: &mut usize, values_are_literals: bool| {
        let count = read_vli(proof, offset)?;
        if count > (proof.len() - *offset) as u64 {
            return Err(reject("UNSAT proof list exceeds remaining input"));
        }
        for _ in 0..count {
            let value = read_vli(proof, offset)?;
            if values_are_literals && value > max_literal_code {
                return Err(reject("UNSAT proof literal exceeds obligation"));
            }
        }
        Ok(())
    };
    while offset < proof.len() {
        steps += 1;
        if steps > MAX_UNSAT_PROOF_STEPS {
            return Err(reject("UNSAT proof step count exceeds limit"));
        }
        match read_vli(proof, &mut offset)? {
            0 | 2 => {
                read_variable(&mut offset)?;
                read_variable(&mut offset)?;
            }
            1 | 3 | 4 | 5 | 6 => read_variable(&mut offset)?,
            7 | 8 | 17 => {
                read_list(&mut offset, true)?;
                read_list(&mut offset, false)?;
            }
            9 => {
                let count = read_vli(proof, &mut offset)?;
                if count > ((proof.len() - offset) / 2) as u64 {
                    return Err(reject("UNSAT proof units exceed remaining input"));
                }
                for _ in 0..count {
                    if read_vli(proof, &mut offset)? > max_literal_code {
                        return Err(reject("UNSAT proof literal exceeds obligation"));
                    }
                    let _ = read_vli(proof, &mut offset)?;
                }
            }
            10 | 11 | 12 | 14 | 15 | 16 => read_list(&mut offset, true)?,
            13 => {
                if read_vli(proof, &mut offset)? > 64 {
                    return Err(reject("UNSAT proof hash width exceeds 64"));
                }
            }
            CODE_END => {
                if offset != proof.len() {
                    return Err(reject("UNSAT proof has trailing bytes"));
                }
                return Ok(());
            }
            _ => return Err(reject("UNSAT proof step code is invalid")),
        }
    }
    Err(reject("UNSAT proof has no end step"))
}

pub fn verify_unsat_proof(clauses: &[CnfClause], proof: &[u8]) -> Result<(), UnsatProofError> {
    preflight(clauses, proof)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut checker = VarisatProofChecker::new();
        for clause in clauses {
            checker
                .add_clause(&literals(clause))
                .map_err(|error| reject(format!("load UNSAT proof obligation: {error}")))?;
        }
        checker
            .check_proof(proof)
            .map_err(|error| reject(format!("check UNSAT proof obligation: {error}")))
    }));
    match result {
        Ok(result) => result,
        Err(_) => Err(reject("malformed UNSAT proof was rejected")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contradiction() -> Vec<CnfClause> {
        vec![CnfClause(vec![(0, true)]), CnfClause(vec![(0, false)])]
    }

    #[test]
    fn generated_proof_is_independently_accepted() {
        let clauses = contradiction();
        let proof = generate_unsat_proof(&clauses).unwrap();
        verify_unsat_proof(&clauses, &proof).unwrap();
    }

    #[test]
    fn satisfiable_and_hostile_inputs_fail_closed() {
        assert!(generate_unsat_proof(&[CnfClause(vec![(0, true)])]).is_err());
        assert!(verify_unsat_proof(&contradiction(), &[]).is_err());
        assert!(verify_unsat_proof(&contradiction(), &[0xff; 32]).is_err());
        assert!(generate_unsat_proof(&[]).is_err());
        assert!(generate_unsat_proof(&[CnfClause(vec![(MAX_CNF_VARIABLES, true)])]).is_err());
    }

    #[test]
    fn every_truncation_and_single_byte_mutation_fails_closed() {
        let clauses = contradiction();
        let proof = generate_unsat_proof(&clauses).unwrap();
        for length in 0..proof.len() {
            assert!(verify_unsat_proof(&clauses, &proof[..length]).is_err());
        }
        for index in 0..proof.len() {
            let mut mutated = proof.clone();
            mutated[index] ^= 1;
            assert!(verify_unsat_proof(&clauses, &mutated).is_err());
        }
    }
}
