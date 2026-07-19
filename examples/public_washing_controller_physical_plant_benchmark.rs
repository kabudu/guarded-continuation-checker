use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantWiring, compose_verified_mtbdd_plant, verify_mtbdd_for_composition,
};
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, encode_controller_mtbdd_plant_artifact,
    produce_controller_mtbdd_plant_artifact, verify_controller_mtbdd_plant_artifact,
};
use sha2::{Digest, Sha256};
use std::hint::black_box;
use std::time::Instant;

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
    let shared = encode_controller_mtbdd_plant_artifact(&produce_controller_mtbdd_plant_artifact(
        &controller,
        controller_digest,
        &mtbdd,
        &inputs,
    )?)?;
    let repeated = inputs
        .iter()
        .map(|input| {
            encode_controller_mtbdd_plant_artifact(&produce_controller_mtbdd_plant_artifact(
                &controller,
                controller_digest,
                &mtbdd,
                std::slice::from_ref(input),
            )?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let sources = vec![(&plant, plant_digest); inputs.len()];

    let mut shared_times = Vec::with_capacity(TRIALS);
    let mut repeated_times = Vec::with_capacity(TRIALS);
    let mut in_process_times = Vec::with_capacity(TRIALS);
    let mut answers_agree = true;
    let mut safe = 0usize;
    let mut unsafe_count = 0usize;
    for trial in 0..TRIALS {
        let run_shared = || {
            verify_controller_mtbdd_plant_artifact(
                &controller,
                controller_digest,
                &sources,
                &shared,
            )
        };
        let run_repeated = || {
            repeated
                .iter()
                .map(|bytes| {
                    verify_controller_mtbdd_plant_artifact(
                        &controller,
                        controller_digest,
                        std::slice::from_ref(&sources[0]),
                        bytes,
                    )
                    .map(|summary| summary.members[0].clone())
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let run_in_process = || -> Result<_, Box<dyn std::error::Error>> {
            let verified = verify_mtbdd_for_composition(&controller, controller_digest, &mtbdd)?;
            (11..17)
                .map(|property| {
                    compose_verified_mtbdd_plant(&verified, &plant, &wiring, 0, 0, property, 32)
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(Into::into)
        };

        let started = Instant::now();
        let shared_result = black_box(run_shared()?);
        shared_times.push(started.elapsed().as_nanos());
        let started = Instant::now();
        let in_process_result = black_box(run_in_process()?);
        in_process_times.push(started.elapsed().as_nanos());
        let started = Instant::now();
        let repeated_result = black_box(run_repeated()?);
        repeated_times.push(started.elapsed().as_nanos());

        answers_agree &= shared_result.members == in_process_result;
        answers_agree &= shared_result.members == repeated_result;
        if trial == 0 {
            safe = shared_result.safe;
            unsafe_count = shared_result.unsafe_count;
        }
    }

    let repeated_bytes = repeated.iter().map(Vec::len).sum::<usize>();
    let shared_time = median(shared_times);
    let repeated_time = median(repeated_times);
    let in_process_time = median(in_process_times);
    println!(
        "schema_version,members,trials,safe,unsafe,repeated_artifact_bytes,shared_artifact_bytes,artifact_ratio,repeated_check_median_nanos,shared_check_median_nanos,shared_check_ratio,in_process_checked_reuse_median_nanos,shared_vs_in_process_ratio,answers_agree,status"
    );
    println!(
        "1,{},{TRIALS},{safe},{unsafe_count},{repeated_bytes},{},{:.3},{repeated_time},{shared_time},{:.3},{in_process_time},{:.3},{answers_agree},{}",
        inputs.len(),
        shared.len(),
        shared.len() as f64 / repeated_bytes as f64,
        shared_time as f64 / repeated_time as f64,
        shared_time as f64 / in_process_time as f64,
        if answers_agree { "ok" } else { "mismatch" },
    );
    Ok(())
}
