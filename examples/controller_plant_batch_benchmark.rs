use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, encode_controller_plant_artifact,
    produce_controller_plant_artifact, verify_controller_plant_artifact,
};
use guarded_continuation_checker::controller_transducer::produce_controller_transducer;
use std::hint::black_box;
use std::time::Instant;

const TRIALS: usize = 101;

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

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

fn plant() -> AigerTransition {
    AigerTransition {
        max_variable: 2,
        inputs: vec![2],
        latches: vec![AigerLatch {
            current: 4,
            next: 2,
        }],
        outputs: vec![4, 4],
        ands: vec![],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = controller();
    let plant = plant();
    let digest = [19; 32];
    let obligation = produce_controller_transducer(&controller, digest, &[0], &[0])?;
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };

    println!(
        "schema_version,members,trials,repeated_complete_artifact_bytes,shared_complete_artifact_bytes,artifact_ratio,repeated_check_median_nanos,shared_check_median_nanos,check_ratio,answers_agree,status"
    );
    for count in [1usize, 2, 4, 8, 16, 32, 64] {
        let members = (0..count)
            .map(|index| {
                let mut plant_digest = [23; 32];
                plant_digest[0] = index as u8;
                ControllerPlantArtifactInput {
                    plant: &plant,
                    plant_source_sha256: plant_digest,
                    wiring: &wiring,
                    initial_controller_state: 0,
                    initial_plant_state: index & 1,
                    bad_plant_output: 1,
                    horizon: 8 + index % 8,
                }
            })
            .collect::<Vec<_>>();
        let shared_artifact = encode_controller_plant_artifact(
            &produce_controller_plant_artifact(&controller, digest, &obligation, &members)?,
        )?;
        let repeated_artifacts = members
            .iter()
            .map(|member| {
                encode_controller_plant_artifact(&produce_controller_plant_artifact(
                    &controller,
                    digest,
                    &obligation,
                    std::slice::from_ref(member),
                )?)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let plants = members
            .iter()
            .map(|member| (member.plant, member.plant_source_sha256))
            .collect::<Vec<_>>();
        let mut repeated_times = Vec::with_capacity(TRIALS);
        let mut shared_times = Vec::with_capacity(TRIALS);
        let mut answers_agree = true;
        for trial in 0..TRIALS {
            let run_repeated = || -> Result<_, Box<dyn std::error::Error>> {
                repeated_artifacts
                    .iter()
                    .zip(&plants)
                    .map(|(artifact, plant_source)| {
                        verify_controller_plant_artifact(
                            &controller,
                            digest,
                            std::slice::from_ref(plant_source),
                            artifact,
                        )
                    })
                    .map(|result| result.map(|batch| batch.members[0].clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(Into::into)
            };
            if trial.is_multiple_of(2) {
                let started = Instant::now();
                let repeated = black_box(run_repeated()?);
                repeated_times.push(started.elapsed().as_nanos());
                let started = Instant::now();
                let shared = black_box(verify_controller_plant_artifact(
                    &controller,
                    digest,
                    &plants,
                    &shared_artifact,
                )?);
                shared_times.push(started.elapsed().as_nanos());
                answers_agree &= repeated == shared.members;
            } else {
                let started = Instant::now();
                let shared = black_box(verify_controller_plant_artifact(
                    &controller,
                    digest,
                    &plants,
                    &shared_artifact,
                )?);
                shared_times.push(started.elapsed().as_nanos());
                let started = Instant::now();
                let repeated = black_box(run_repeated()?);
                repeated_times.push(started.elapsed().as_nanos());
                answers_agree &= repeated == shared.members;
            }
        }
        let repeated_median = median(repeated_times);
        let shared_median = median(shared_times);
        let repeated_bytes = repeated_artifacts.iter().map(Vec::len).sum::<usize>();
        println!(
            "2,{count},{TRIALS},{repeated_bytes},{},{:.3},{repeated_median},{shared_median},{:.3},{answers_agree},{}",
            shared_artifact.len(),
            shared_artifact.len() as f64 / repeated_bytes as f64,
            shared_median as f64 / repeated_median as f64,
            if answers_agree { "ok" } else { "mismatch" },
        );
    }
    Ok(())
}
