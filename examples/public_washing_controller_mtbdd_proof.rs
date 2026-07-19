use std::time::Instant;

use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_mtbdd::{
    produce_controller_mtbdd, verify_controller_mtbdd,
};
use guarded_continuation_checker::controller_mtbdd_proof::{
    produce_controller_mtbdd_equivalence_proof, verify_controller_mtbdd_equivalence_proof,
};
use sha2::{Digest, Sha256};

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = parse_ascii_aiger_transition(MODEL)?;
    let digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let mtbdd =
        produce_controller_mtbdd(&model, digest, &(1..12).collect::<Vec<_>>(), &[2, 6, 7, 9])?;
    let started = Instant::now();
    let proof = produce_controller_mtbdd_equivalence_proof(&model, digest, &mtbdd)?;
    let proof_production_nanos = started.elapsed().as_nanos();
    let mut exhaustive = Vec::new();
    let mut checked = Vec::new();
    let mut proof_summary = None;
    for _ in 0..3 {
        let started = Instant::now();
        verify_controller_mtbdd(&model, digest, &mtbdd)?;
        exhaustive.push(started.elapsed().as_nanos());
        let started = Instant::now();
        proof_summary = Some(verify_controller_mtbdd_equivalence_proof(
            &model, digest, &mtbdd, &proof,
        )?);
        checked.push(started.elapsed().as_nanos());
    }
    let summary = proof_summary.expect("at least one proof trial");
    let exhaustive_nanos = median(exhaustive);
    let proof_check_nanos = median(checked);
    println!(
        "schema_version,assignments,cnf_variables,cnf_clauses,cnf_literals,proof_bytes,proof_production_nanos,exhaustive_verification_nanos,proof_verification_nanos,verification_ratio,status"
    );
    println!(
        "1,131072,{},{},{},{},{proof_production_nanos},{exhaustive_nanos},{proof_check_nanos},{:.6},ok",
        summary.cnf_variables,
        summary.cnf_clauses,
        summary.cnf_literals,
        summary.proof_bytes,
        proof_check_nanos as f64 / exhaustive_nanos as f64,
    );
    Ok(())
}
