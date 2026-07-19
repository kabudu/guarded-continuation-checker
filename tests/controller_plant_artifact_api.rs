use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring,
};
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, decode_controller_plant_artifact,
    encode_controller_plant_artifact, produce_controller_plant_artifact,
    verify_controller_plant_artifact,
};
use guarded_continuation_checker::controller_transducer::produce_controller_transducer;

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

#[test]
fn downstream_api_round_trips_and_independently_checks_batch_artifacts() {
    let controller = controller();
    let plant = plant();
    let controller_digest = [0x31; 32];
    let plant_digests = [[0x41; 32], [0x42; 32]];
    let transducer =
        produce_controller_transducer(&controller, controller_digest, &[0], &[0]).unwrap();
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
            horizon: 16,
        },
        ControllerPlantArtifactInput {
            plant: &plant,
            plant_source_sha256: plant_digests[1],
            wiring: &wiring,
            initial_controller_state: 0,
            initial_plant_state: 1,
            bad_plant_output: 1,
            horizon: 16,
        },
    ];
    let artifact =
        produce_controller_plant_artifact(&controller, controller_digest, &transducer, &inputs)
            .unwrap();
    let encoded = encode_controller_plant_artifact(&artifact).unwrap();
    assert_eq!(
        encode_controller_plant_artifact(&decode_controller_plant_artifact(&encoded).unwrap())
            .unwrap(),
        encoded
    );
    let plants = [(&plant, plant_digests[0]), (&plant, plant_digests[1])];
    let result =
        verify_controller_plant_artifact(&controller, controller_digest, &plants, &encoded)
            .unwrap();
    assert_eq!((result.safe, result.unsafe_count), (1, 1));
    assert_eq!(result.members[0].answer, ControllerPlantAnswer::Safe);
    assert_eq!(result.members[1].answer, ControllerPlantAnswer::Unsafe);

    for length in 0..encoded.len() {
        assert!(
            verify_controller_plant_artifact(
                &controller,
                controller_digest,
                &plants,
                &encoded[..length],
            )
            .is_err(),
            "accepted truncation at byte {length}"
        );
    }
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(
            verify_controller_plant_artifact(&controller, controller_digest, &plants, &mutated,)
                .is_err(),
            "accepted mutation at byte {index}"
        );
    }
    let wrong_sources = [(&plant, plant_digests[1]), (&plant, plant_digests[0])];
    assert!(
        verify_controller_plant_artifact(&controller, controller_digest, &wrong_sources, &encoded,)
            .is_err()
    );
}
