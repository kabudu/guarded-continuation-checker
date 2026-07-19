use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::{
    ControllerPlantBatchInput, ControllerPlantWiring, compose_controller_plant,
    compose_controller_plant_batch,
};
use guarded_continuation_checker::controller_transducer::{
    encode_controller_transducer, produce_controller_transducer,
};
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
    let controller_evidence_bytes = encode_controller_transducer(&obligation)?.len();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };

    println!(
        "schema_version,members,trials,repeated_controller_evidence_bytes,shared_controller_evidence_bytes,evidence_ratio,repeated_check_median_nanos,shared_check_median_nanos,check_ratio,answers_agree,status"
    );
    for count in [1usize, 2, 4, 8, 16, 32, 64] {
        let members = (0..count)
            .map(|index| ControllerPlantBatchInput {
                plant: &plant,
                wiring: &wiring,
                initial_controller_state: 0,
                initial_plant_state: index & 1,
                bad_plant_output: 1,
                horizon: 8 + index % 8,
            })
            .collect::<Vec<_>>();
        let mut repeated_times = Vec::with_capacity(TRIALS);
        let mut shared_times = Vec::with_capacity(TRIALS);
        let mut answers_agree = true;
        for trial in 0..TRIALS {
            let run_repeated = || -> Result<_, Box<dyn std::error::Error>> {
                members
                    .iter()
                    .map(|member| {
                        compose_controller_plant(
                            &controller,
                            digest,
                            &obligation,
                            member.plant,
                            member.wiring,
                            member.initial_controller_state,
                            member.initial_plant_state,
                            member.bad_plant_output,
                            member.horizon,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(Into::into)
            };
            if trial.is_multiple_of(2) {
                let started = Instant::now();
                let repeated = black_box(run_repeated()?);
                repeated_times.push(started.elapsed().as_nanos());
                let started = Instant::now();
                let shared = black_box(compose_controller_plant_batch(
                    &controller,
                    digest,
                    &obligation,
                    &members,
                )?);
                shared_times.push(started.elapsed().as_nanos());
                answers_agree &= repeated == shared.members;
            } else {
                let started = Instant::now();
                let shared = black_box(compose_controller_plant_batch(
                    &controller,
                    digest,
                    &obligation,
                    &members,
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
        let repeated_bytes = controller_evidence_bytes * count;
        println!(
            "1,{count},{TRIALS},{repeated_bytes},{controller_evidence_bytes},{:.3},{repeated_median},{shared_median},{:.3},{answers_agree},{}",
            controller_evidence_bytes as f64 / repeated_bytes as f64,
            shared_median as f64 / repeated_median as f64,
            if answers_agree { "ok" } else { "mismatch" },
        );
    }
    Ok(())
}
