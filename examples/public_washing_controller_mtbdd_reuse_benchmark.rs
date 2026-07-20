use guarded_continuation_checker::aiger_obligation::{
    AigerAnd, AigerLatch, AigerTransition, parse_ascii_aiger_transition,
};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, encode_controller_mtbdd_plant_artifact,
    produce_controller_mtbdd_plant_artifact, verify_controller_mtbdd_plant_artifact,
};
use sha2::{Digest, Sha256};
use std::hint::black_box;
use std::time::Instant;

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
const TRIALS: usize = 3;

fn environment(bad_right: usize) -> AigerTransition {
    AigerTransition {
        max_variable: 6,
        inputs: vec![2, 4, 6, 8],
        latches: vec![AigerLatch {
            current: 10,
            next: 0,
        }],
        outputs: vec![0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 12],
        ands: vec![AigerAnd {
            output: 12,
            left: 8,
            right: bad_right,
        }],
    }
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = parse_ascii_aiger_transition(MODEL)?;
    let controller_digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let mtbdd = produce_controller_mtbdd(
        &controller,
        controller_digest,
        &(1..12).collect::<Vec<_>>(),
        &[2, 6, 7, 9],
    )?;
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (1..12).collect(),
        plant_action_inputs: vec![0, 1, 2, 3],
    };
    println!(
        "schema_version,members,trials,repeated_artifact_bytes,shared_artifact_bytes,artifact_ratio,repeated_verify_median_nanos,shared_verify_median_nanos,verify_ratio,answers_agree,status"
    );
    for count in [1usize, 2, 4, 8, 16] {
        let plants = (0..count)
            .map(|index| environment(if index.is_multiple_of(2) { 6 } else { 3 }))
            .collect::<Vec<_>>();
        let digests = (0..count)
            .map(|index| {
                let mut digest = [0x71; 32];
                digest[0] = index as u8;
                digest
            })
            .collect::<Vec<_>>();
        let inputs = plants
            .iter()
            .zip(&digests)
            .map(
                |(plant, &plant_source_sha256)| ControllerPlantArtifactInput {
                    plant,
                    plant_source_sha256,
                    wiring: &wiring,
                    initial_controller_state: 0,
                    initial_plant_state: 0,
                    bad_plant_output: 12,
                    horizon: 32,
                },
            )
            .collect::<Vec<_>>();
        let shared =
            encode_controller_mtbdd_plant_artifact(&produce_controller_mtbdd_plant_artifact(
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
        let plant_sources = plants
            .iter()
            .zip(&digests)
            .map(|(plant, &digest)| (plant, digest))
            .collect::<Vec<_>>();
        let mut repeated_times = Vec::with_capacity(TRIALS);
        let mut shared_times = Vec::with_capacity(TRIALS);
        let mut answers_agree = true;
        for trial in 0..TRIALS {
            let run_repeated = || -> Result<_, Box<dyn std::error::Error>> {
                repeated
                    .iter()
                    .zip(&plant_sources)
                    .map(|(artifact, source)| {
                        verify_controller_mtbdd_plant_artifact(
                            &controller,
                            controller_digest,
                            std::slice::from_ref(source),
                            artifact,
                        )
                    })
                    .map(|result| result.map(|summary| summary.members[0].clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(Into::into)
            };
            if trial.is_multiple_of(2) {
                let started = Instant::now();
                let repeated_results = black_box(run_repeated()?);
                repeated_times.push(started.elapsed().as_nanos());
                let started = Instant::now();
                let shared_result = black_box(verify_controller_mtbdd_plant_artifact(
                    &controller,
                    controller_digest,
                    &plant_sources,
                    &shared,
                )?);
                shared_times.push(started.elapsed().as_nanos());
                answers_agree &= repeated_results == shared_result.members;
            } else {
                let started = Instant::now();
                let shared_result = black_box(verify_controller_mtbdd_plant_artifact(
                    &controller,
                    controller_digest,
                    &plant_sources,
                    &shared,
                )?);
                shared_times.push(started.elapsed().as_nanos());
                let started = Instant::now();
                let repeated_results = black_box(run_repeated()?);
                repeated_times.push(started.elapsed().as_nanos());
                answers_agree &= repeated_results == shared_result.members;
            }
        }
        let repeated_bytes = repeated.iter().map(Vec::len).sum::<usize>();
        let repeated_median = median(repeated_times);
        let shared_median = median(shared_times);
        println!(
            "1,{count},{TRIALS},{repeated_bytes},{},{:.3},{repeated_median},{shared_median},{:.3},{answers_agree},{}",
            shared.len(),
            shared.len() as f64 / repeated_bytes as f64,
            shared_median as f64 / repeated_median as f64,
            if answers_agree { "ok" } else { "mismatch" }
        );
    }
    Ok(())
}
