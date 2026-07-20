use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring, compose_controller_plant_direct,
    compose_verified_mtbdd_plant, verify_mtbdd_for_composition,
};
use sha2::{Digest, Sha256};

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const CONTROLLER: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
const PLANT: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/physical-plant/washing-plant.aag");

#[test]
fn public_controller_composes_with_a_stateful_physical_process() {
    let controller = parse_ascii_aiger_transition(CONTROLLER).unwrap();
    let plant = parse_ascii_aiger_transition(PLANT).unwrap();
    let digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let sensor_inputs = (1..12).collect::<Vec<_>>();
    let action_outputs = vec![2, 3, 4, 5, 6, 7, 9];
    let artifact =
        produce_controller_mtbdd(&controller, digest, &sensor_inputs, &action_outputs).unwrap();
    let verified = verify_mtbdd_for_composition(&controller, digest, &artifact).unwrap();
    assert_eq!(verified.summary().outputs, 7);
    assert_eq!(verified.summary().nodes, 254);
    assert_eq!(verified.summary().terminals, 189);
    assert_eq!(verified.summary().assignments_checked, 131_072);
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: sensor_inputs,
        controller_action_outputs: action_outputs,
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: (1..8).collect(),
    };

    for bad_output in 11..14 {
        let mtbdd =
            compose_verified_mtbdd_plant(&verified, &plant, &wiring, 0, 0, bad_output, 64).unwrap();
        let direct =
            compose_controller_plant_direct(&controller, &plant, &wiring, 0, 0, bad_output, 64)
                .unwrap();
        assert_eq!(mtbdd, direct);
        assert_eq!(mtbdd.answer, ControllerPlantAnswer::Safe);
    }
}
