use guarded_continuation_checker::aiger_obligation::{
    AigerAnd, AigerLatch, AigerTransition, parse_ascii_aiger_transition,
};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring, compose_controller_plant_direct,
    compose_verified_mtbdd_plant, verify_mtbdd_for_composition,
};
use sha2::{Digest, Sha256};

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
const PHYSICAL_PLANT: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.aag");

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

#[test]
fn public_controller_mtbdd_composes_with_stateful_physical_plant_family() {
    let controller = parse_ascii_aiger_transition(MODEL).unwrap();
    let plant = parse_ascii_aiger_transition(PHYSICAL_PLANT).unwrap();
    let digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let artifact = produce_controller_mtbdd(
        &controller,
        digest,
        &(1..12).collect::<Vec<_>>(),
        &[2, 6, 7, 9],
    )
    .unwrap();
    let verified = verify_mtbdd_for_composition(&controller, digest, &artifact).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let expected = [
        (ControllerPlantAnswer::Unsafe, Some(4)),
        (ControllerPlantAnswer::Unsafe, Some(7)),
        (ControllerPlantAnswer::Unsafe, Some(15)),
        (ControllerPlantAnswer::Unsafe, Some(15)),
        (ControllerPlantAnswer::Safe, None),
        (ControllerPlantAnswer::Safe, None),
    ];
    for (property, (expected_answer, expected_frame)) in (11..17).zip(expected) {
        let mtbdd =
            compose_verified_mtbdd_plant(&verified, &plant, &wiring, 0, 0, property, 32).unwrap();
        let direct =
            compose_controller_plant_direct(&controller, &plant, &wiring, 0, 0, property, 32)
                .unwrap();
        assert_eq!(mtbdd.answer, expected_answer, "property output {property}");
        assert_eq!(
            mtbdd.bad_frame, expected_frame,
            "property output {property}"
        );
        assert_eq!(mtbdd, direct, "property output {property}");
    }
}

#[test]
fn public_controller_mtbdd_composes_exactly_with_safe_and_unsafe_appliance_monitors() {
    let controller = parse_ascii_aiger_transition(MODEL).unwrap();
    let digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let artifact = produce_controller_mtbdd(
        &controller,
        digest,
        &(1..12).collect::<Vec<_>>(),
        &[2, 6, 7, 9],
    )
    .unwrap();
    let verified = verify_mtbdd_for_composition(&controller, digest, &artifact).unwrap();
    let mtbdd_wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (1..12).collect(),
        plant_action_inputs: vec![0, 1, 2, 3],
    };
    let cases = [
        (environment(6), ControllerPlantAnswer::Safe),
        (environment(3), ControllerPlantAnswer::Unsafe),
    ];
    for (plant, expected) in cases {
        let mtbdd =
            compose_verified_mtbdd_plant(&verified, &plant, &mtbdd_wiring, 0, 0, 12, 32).unwrap();
        let direct =
            compose_controller_plant_direct(&controller, &plant, &mtbdd_wiring, 0, 0, 12, 32)
                .unwrap();
        assert_eq!(mtbdd.answer, expected);
        assert_eq!(mtbdd, direct);
    }
}
