use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring, compose_controller_plant,
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
}
