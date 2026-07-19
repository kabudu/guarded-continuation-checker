use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantBackend, ControllerPlantBatchInput,
    ControllerPlantWiring, compose_controller_plant, compose_controller_plant_batch,
    compose_controller_plant_portfolio,
};
use guarded_continuation_checker::controller_transducer::produce_controller_transducer;

#[test]
fn downstream_api_composes_a_verified_controller_and_sampled_plant() {
    let controller = AigerTransition {
        max_variable: 2,
        inputs: vec![2],
        latches: vec![AigerLatch {
            current: 4,
            next: 2,
        }],
        outputs: vec![2],
        ands: vec![],
    };
    let plant = AigerTransition {
        max_variable: 2,
        inputs: vec![2],
        latches: vec![AigerLatch {
            current: 4,
            next: 2,
        }],
        outputs: vec![4, 4],
        ands: vec![],
    };
    let digest = [0x71; 32];
    let obligation = produce_controller_transducer(&controller, digest, &[0], &[0]).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let result = compose_controller_plant(
        &controller,
        digest,
        &obligation,
        &plant,
        &wiring,
        0,
        0,
        1,
        16,
    )
    .unwrap();
    assert_eq!(result.answer, ControllerPlantAnswer::Safe);
    assert_eq!(result.bad_frame, None);

    let fallback =
        compose_controller_plant_portfolio(&controller, digest, None, &plant, &wiring, 0, 0, 1, 16)
            .unwrap();
    assert_eq!(fallback.backend, ControllerPlantBackend::DirectExact);
    assert_eq!(fallback.result, result);

    let batch = compose_controller_plant_batch(
        &controller,
        digest,
        &obligation,
        &[ControllerPlantBatchInput {
            plant: &plant,
            wiring: &wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 16,
        }],
    )
    .unwrap();
    assert_eq!((batch.safe, batch.unsafe_count), (1, 0));
    assert_eq!(batch.members[0], result);
}
