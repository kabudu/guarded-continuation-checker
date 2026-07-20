use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_mtbdd::{
    encode_controller_mtbdd, produce_controller_mtbdd, verify_controller_mtbdd,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = parse_ascii_aiger_transition(MODEL)?;
    let digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let started = Instant::now();
    let artifact =
        produce_controller_mtbdd(&model, digest, &(1..12).collect::<Vec<_>>(), &[2, 6, 7, 9])?;
    let production_nanos = started.elapsed().as_nanos();
    let artifact_bytes = encode_controller_mtbdd(&artifact)?.len();
    let started = Instant::now();
    let summary = verify_controller_mtbdd(&model, digest, &artifact)?;
    let verification_nanos = started.elapsed().as_nanos();
    println!(
        "schema_version,assignments,terminals,nodes,artifact_bytes,production_nanos,verification_nanos,status"
    );
    println!(
        "1,{},{},{},{artifact_bytes},{production_nanos},{verification_nanos},ok",
        summary.assignments_checked, summary.terminals, summary.nodes
    );
    Ok(())
}
