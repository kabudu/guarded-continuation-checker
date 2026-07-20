use std::hint::black_box;
use std::time::Instant;

use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, encode_controller_mtbdd_plant_artifact,
    encode_controller_proof_mtbdd_plant_artifact, produce_controller_mtbdd_plant_artifact,
    produce_controller_proof_mtbdd_plant_artifact, verify_controller_mtbdd_plant_artifact,
    verify_controller_proof_mtbdd_plant_artifact,
};
use sha2::{Digest, Sha256};

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
const PLANT_SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.v");
const PLANT_MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.aag");
const TRIALS: usize = 3;

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = parse_ascii_aiger_transition(MODEL)?;
    let plant = parse_ascii_aiger_transition(PLANT_MODEL)?;
    let controller_digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let plant_digest: [u8; 32] = Sha256::digest(PLANT_SOURCE).into();
    let mtbdd = produce_controller_mtbdd(
        &controller,
        controller_digest,
        &(1..12).collect::<Vec<_>>(),
        &[2, 6, 7, 9],
    )?;
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let inputs = (11..17)
        .map(|bad_plant_output| ControllerPlantArtifactInput {
            plant: &plant,
            plant_source_sha256: plant_digest,
            wiring: &wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output,
            horizon: 32,
        })
        .collect::<Vec<_>>();
    let exhaustive = encode_controller_mtbdd_plant_artifact(
        &produce_controller_mtbdd_plant_artifact(&controller, controller_digest, &mtbdd, &inputs)?,
    )?;
    let started = Instant::now();
    let proof_artifact = produce_controller_proof_mtbdd_plant_artifact(
        &controller,
        controller_digest,
        &mtbdd,
        &inputs,
    )?;
    let proof_production_nanos = started.elapsed().as_nanos();
    let proof = encode_controller_proof_mtbdd_plant_artifact(&proof_artifact)?;
    let sources = vec![(&plant, plant_digest); inputs.len()];
    let mut exhaustive_times = Vec::new();
    let mut proof_times = Vec::new();
    let mut answers_agree = true;
    for _ in 0..TRIALS {
        let started = Instant::now();
        let exhaustive_summary = black_box(verify_controller_mtbdd_plant_artifact(
            &controller,
            controller_digest,
            &sources,
            &exhaustive,
        )?);
        exhaustive_times.push(started.elapsed().as_nanos());
        let started = Instant::now();
        let proof_summary = black_box(verify_controller_proof_mtbdd_plant_artifact(
            &controller,
            controller_digest,
            &sources,
            &proof,
        )?);
        proof_times.push(started.elapsed().as_nanos());
        answers_agree &= exhaustive_summary.members == proof_summary.members;
    }
    let exhaustive_nanos = median(exhaustive_times);
    let proof_nanos = median(proof_times);
    println!(
        "schema_version,members,trials,exhaustive_artifact_bytes,proof_artifact_bytes,artifact_ratio,proof_production_nanos,exhaustive_verification_nanos,proof_verification_nanos,verification_ratio,answers_agree,status"
    );
    println!(
        "1,{},{TRIALS},{},{},{:.6},{proof_production_nanos},{exhaustive_nanos},{proof_nanos},{:.6},{answers_agree},{}",
        inputs.len(),
        exhaustive.len(),
        proof.len(),
        proof.len() as f64 / exhaustive.len() as f64,
        proof_nanos as f64 / exhaustive_nanos as f64,
        if answers_agree { "ok" } else { "mismatch" },
    );
    Ok(())
}
