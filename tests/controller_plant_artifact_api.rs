use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring,
};
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerMtbddPlantPortfolioArtifact, ControllerMtbddPlantPortfolioBackend,
    ControllerMtbddPlantSelectionReason, ControllerPlantArtifactInput,
    decode_controller_direct_plant_artifact, decode_controller_mtbdd_plant_portfolio,
    decode_controller_plant_artifact, decode_controller_proof_mtbdd_plant_artifact,
    encode_controller_direct_plant_artifact, encode_controller_mtbdd_plant_portfolio,
    encode_controller_plant_artifact, encode_controller_proof_mtbdd_plant_artifact,
    produce_controller_direct_plant_artifact, produce_controller_mtbdd_plant_portfolio,
    produce_controller_plant_artifact, produce_controller_proof_mtbdd_plant_artifact,
    verify_controller_direct_plant_artifact, verify_controller_mtbdd_plant_portfolio,
    verify_controller_plant_artifact, verify_controller_proof_mtbdd_plant_artifact,
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

#[test]
fn downstream_direct_artifact_is_deterministic_bound_and_independently_replayed() {
    let controller = controller();
    let plant = plant();
    let controller_digest = [0x71; 32];
    let plant_digests = [[0x81; 32], [0x82; 32]];
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
    let first =
        produce_controller_direct_plant_artifact(&controller, controller_digest, &inputs).unwrap();
    let encoded = encode_controller_direct_plant_artifact(&first).unwrap();
    assert_eq!(
        encode_controller_direct_plant_artifact(
            &decode_controller_direct_plant_artifact(&encoded).unwrap()
        )
        .unwrap(),
        encoded
    );
    assert_eq!(
        encode_controller_direct_plant_artifact(
            &produce_controller_direct_plant_artifact(&controller, controller_digest, &inputs,)
                .unwrap()
        )
        .unwrap(),
        encoded
    );
    let plants = [(&plant, plant_digests[0]), (&plant, plant_digests[1])];
    let result =
        verify_controller_direct_plant_artifact(&controller, controller_digest, &plants, &encoded)
            .unwrap();
    assert_eq!((result.safe, result.unsafe_count), (1, 1));

    for length in 0..encoded.len() {
        assert!(decode_controller_direct_plant_artifact(&encoded[..length]).is_err());
    }
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_direct_plant_artifact(&mutated).is_err());
    }
    assert!(
        verify_controller_direct_plant_artifact(&controller, [0; 32], &plants, &encoded,).is_err()
    );
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

fn wide_state_controller() -> AigerTransition {
    AigerTransition {
        max_variable: 8,
        inputs: vec![2],
        latches: (0..7)
            .map(|index| AigerLatch {
                current: 4 + index * 2,
                next: 2,
            })
            .collect(),
        outputs: vec![2],
        ands: vec![],
    }
}

fn portfolio_inputs<'a>(
    plant: &'a AigerTransition,
    wiring: &'a ControllerPlantWiring,
) -> [ControllerPlantArtifactInput<'a>; 2] {
    [
        ControllerPlantArtifactInput {
            plant,
            plant_source_sha256: [0x91; 32],
            wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 8,
        },
        ControllerPlantArtifactInput {
            plant,
            plant_source_sha256: [0x92; 32],
            wiring,
            initial_controller_state: 0,
            initial_plant_state: 1,
            bad_plant_output: 1,
            horizon: 8,
        },
    ]
}

#[test]
fn proof_carrying_mtbdd_batch_is_canonical_bound_and_independently_checked() {
    let controller = controller();
    let plant = plant();
    let controller_digest = [0xc0; 32];
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let inputs = portfolio_inputs(&plant, &wiring);
    let mtbdd = produce_controller_mtbdd(&controller, controller_digest, &[0], &[0]).unwrap();
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
            &produce_controller_proof_mtbdd_plant_artifact(
                &controller,
                controller_digest,
                &mtbdd,
                &inputs,
            )
            .unwrap(),
        )
        .unwrap(),
        encoded
    );
    assert_eq!(
        encode_controller_proof_mtbdd_plant_artifact(
            &decode_controller_proof_mtbdd_plant_artifact(&encoded).unwrap()
        )
        .unwrap(),
        encoded
    );
    let plants = [(&plant, [0x91; 32]), (&plant, [0x92; 32])];
    let summary = verify_controller_proof_mtbdd_plant_artifact(
        &controller,
        controller_digest,
        &plants,
        &encoded,
    )
    .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
    assert_eq!(summary.assignments_checked, 0);

    for length in 0..encoded.len() {
        assert!(decode_controller_proof_mtbdd_plant_artifact(&encoded[..length]).is_err());
    }
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_proof_mtbdd_plant_artifact(&mutated).is_err());
    }
    assert!(
        verify_controller_proof_mtbdd_plant_artifact(&controller, [0; 32], &plants, &encoded,)
            .is_err()
    );
}

#[test]
fn mtbdd_plant_portfolio_uses_mtbdd_when_admitted() {
    let controller = controller();
    let plant = plant();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let inputs = portfolio_inputs(&plant, &wiring);
    let encoded =
        produce_controller_mtbdd_plant_portfolio(&controller, [0x90; 32], &[0], &[0], &inputs)
            .unwrap();
    let decoded = decode_controller_mtbdd_plant_portfolio(&encoded).unwrap();
    assert_eq!(decoded.backend, ControllerMtbddPlantPortfolioBackend::Mtbdd);
    assert_eq!(
        decoded.reason,
        ControllerMtbddPlantSelectionReason::MtbddAdmitted
    );
    let summary = verify_controller_mtbdd_plant_portfolio(
        &controller,
        [0x90; 32],
        &[0],
        &[0],
        &inputs,
        &encoded,
    )
    .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));

    let direct = produce_controller_direct_plant_artifact(&controller, [0x90; 32], &inputs)
        .and_then(|artifact| encode_controller_direct_plant_artifact(&artifact))
        .unwrap();
    let downgrade =
        encode_controller_mtbdd_plant_portfolio(&ControllerMtbddPlantPortfolioArtifact {
            version: 1,
            backend: ControllerMtbddPlantPortfolioBackend::DirectExact,
            reason: ControllerMtbddPlantSelectionReason::BoundaryLimit,
            relevant_inputs: vec![0],
            observed_outputs: vec![0],
            payload: direct,
        })
        .unwrap();
    assert!(
        verify_controller_mtbdd_plant_portfolio(
            &controller,
            [0x90; 32],
            &[0],
            &[0],
            &inputs,
            &downgrade,
        )
        .is_err()
    );
}

#[test]
fn mtbdd_plant_portfolio_falls_back_only_for_an_exact_static_rejection() {
    let controller = wide_state_controller();
    let plant = plant();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let inputs = portfolio_inputs(&plant, &wiring);
    let encoded =
        produce_controller_mtbdd_plant_portfolio(&controller, [0xa0; 32], &[0], &[0], &inputs)
            .unwrap();
    let decoded = decode_controller_mtbdd_plant_portfolio(&encoded).unwrap();
    assert_eq!(
        decoded.backend,
        ControllerMtbddPlantPortfolioBackend::DirectExact
    );
    assert_eq!(
        decoded.reason,
        ControllerMtbddPlantSelectionReason::BoundaryLimit
    );
    let summary = verify_controller_mtbdd_plant_portfolio(
        &controller,
        [0xa0; 32],
        &[0],
        &[0],
        &inputs,
        &encoded,
    )
    .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));

    assert!(
        verify_controller_mtbdd_plant_portfolio(
            &controller,
            [0xa0; 32],
            &[],
            &[0],
            &inputs,
            &encoded,
        )
        .is_err()
    );
    assert!(
        produce_controller_mtbdd_plant_portfolio(&controller, [0xa0; 32], &[0], &[], &inputs,)
            .is_err()
    );
    for length in 0..encoded.len() {
        assert!(decode_controller_mtbdd_plant_portfolio(&encoded[..length]).is_err());
    }
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_mtbdd_plant_portfolio(&mutated).is_err());
    }

    let malformed = AigerTransition {
        max_variable: 0,
        inputs: vec![2],
        latches: vec![],
        outputs: vec![2],
        ands: vec![],
    };
    let error =
        produce_controller_mtbdd_plant_portfolio(&malformed, [0xb0; 32], &[0], &[0], &inputs)
            .unwrap_err();
    assert!(!error.to_string().contains("direct controller"));
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
