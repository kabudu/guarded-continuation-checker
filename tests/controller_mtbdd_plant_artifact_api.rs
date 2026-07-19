use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, decode_controller_mtbdd_plant_artifact,
    decode_controller_proof_mtbdd_plant_artifact, encode_controller_mtbdd_plant_artifact,
    encode_controller_proof_mtbdd_plant_artifact, produce_controller_mtbdd_plant_artifact,
    produce_controller_proof_mtbdd_plant_artifact, verify_controller_mtbdd_plant_artifact,
    verify_controller_proof_mtbdd_plant_artifact,
};

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

#[test]
fn proof_carrying_api_checks_equivalence_without_assignment_replay() {
    let controller = controller();
    let plant = plant();
    let controller_digest = [0x71; 32];
    let plant_digest = [0x81; 32];
    let mtbdd = produce_controller_mtbdd(&controller, controller_digest, &[0], &[0]).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let inputs = [ControllerPlantArtifactInput {
        plant: &plant,
        plant_source_sha256: plant_digest,
        wiring: &wiring,
        initial_controller_state: 0,
        initial_plant_state: 0,
        bad_plant_output: 1,
        horizon: 8,
    }];
    let artifact = produce_controller_proof_mtbdd_plant_artifact(
        &controller,
        controller_digest,
        &mtbdd,
        &inputs,
    )
    .unwrap();
    let encoded = encode_controller_proof_mtbdd_plant_artifact(&artifact).unwrap();
    assert_eq!(
        encode_controller_proof_mtbdd_plant_artifact(
            &decode_controller_proof_mtbdd_plant_artifact(&encoded).unwrap()
        )
        .unwrap(),
        encoded
    );
    let summary = verify_controller_proof_mtbdd_plant_artifact(
        &controller,
        controller_digest,
        &[(&plant, plant_digest)],
        &encoded,
    )
    .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 0));

    let mut corrupted = encoded;
    let middle = corrupted.len() / 2;
    corrupted[middle] ^= 1;
    assert!(decode_controller_proof_mtbdd_plant_artifact(&corrupted).is_err());
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

#[test]
fn downstream_api_checks_shared_mtbdd_and_every_bound_member() {
    let controller = controller();
    let plant = plant();
    let controller_digest = [0x51; 32];
    let plant_digests = [[0x61; 32], [0x62; 32]];
    let mtbdd = produce_controller_mtbdd(&controller, controller_digest, &[0], &[0]).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let inputs = [
        ControllerPlantArtifactInput {
            plant: &plant,
            plant_source_sha256: plant_digests[0],
            wiring: &wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 8,
        },
        ControllerPlantArtifactInput {
            plant: &plant,
            plant_source_sha256: plant_digests[1],
            wiring: &wiring,
            initial_controller_state: 0,
            initial_plant_state: 1,
            bad_plant_output: 1,
            horizon: 8,
        },
    ];
    let artifact =
        produce_controller_mtbdd_plant_artifact(&controller, controller_digest, &mtbdd, &inputs)
            .unwrap();
    let encoded = encode_controller_mtbdd_plant_artifact(&artifact).unwrap();
    assert_eq!(
        encode_controller_mtbdd_plant_artifact(
            &decode_controller_mtbdd_plant_artifact(&encoded).unwrap()
        )
        .unwrap(),
        encoded
    );
    let plants = [(&plant, plant_digests[0]), (&plant, plant_digests[1])];
    let summary =
        verify_controller_mtbdd_plant_artifact(&controller, controller_digest, &plants, &encoded)
            .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));

    for length in 0..encoded.len() {
        assert!(decode_controller_mtbdd_plant_artifact(&encoded[..length]).is_err());
    }
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_mtbdd_plant_artifact(&mutated).is_err());
    }
    let wrong_plants = [(&plant, plant_digests[1]), (&plant, plant_digests[0])];
    assert!(
        verify_controller_mtbdd_plant_artifact(
            &controller,
            controller_digest,
            &wrong_plants,
            &encoded,
        )
        .is_err()
    );
}
