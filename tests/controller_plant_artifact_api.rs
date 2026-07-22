use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring,
};
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerMtbddPlantPortfolioArtifact, ControllerMtbddPlantPortfolioBackend,
    ControllerMtbddPlantSelectionReason, ControllerPlantArtifactInput,
    ControllerPlantResourceEnvelope, ControllerProofMtbddPlantPortfolioArtifact,
    ControllerProofMtbddPlantPortfolioBackend, ControllerProofMtbddResourceEnvelope,
    assess_controller_proof_mtbdd_plant_portfolio_resources,
    assess_controller_proof_mtbdd_plant_resources, decode_controller_direct_plant_artifact,
    decode_controller_mtbdd_plant_portfolio, decode_controller_plant_artifact,
    decode_controller_proof_mtbdd_plant_artifact, decode_controller_proof_mtbdd_plant_portfolio,
    encode_controller_direct_plant_artifact, encode_controller_mtbdd_plant_portfolio,
    encode_controller_plant_artifact, encode_controller_proof_mtbdd_plant_artifact,
    encode_controller_proof_mtbdd_plant_portfolio, produce_controller_direct_plant_artifact,
    produce_controller_mtbdd_plant_portfolio, produce_controller_plant_artifact,
    produce_controller_proof_mtbdd_plant_artifact, produce_controller_proof_mtbdd_plant_portfolio,
    verify_controller_direct_plant_artifact, verify_controller_mtbdd_plant_portfolio,
    verify_controller_plant_artifact, verify_controller_proof_mtbdd_plant_artifact,
    verify_controller_proof_mtbdd_plant_artifact_with_resources,
    verify_controller_proof_mtbdd_plant_portfolio,
    verify_controller_proof_mtbdd_plant_portfolio_with_resources,
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
fn proof_carrying_mtbdd_resources_bound_proof_and_composition_before_replay() {
    let controller = controller();
    let plant = plant();
    let controller_digest = [0xd0; 32];
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
    let composition = ControllerPlantResourceEnvelope::new(encoded.len(), 2, 8, 4, 72).unwrap();
    let permissive = ControllerProofMtbddResourceEnvelope::new(
        composition,
        guarded_continuation_checker::controller_mtbdd_proof::MAX_EQUIVALENCE_ARTIFACT_BYTES,
        guarded_continuation_checker::unsat_proof::MAX_UNSAT_PROOF_BYTES,
    )
    .unwrap();
    let assessment =
        assess_controller_proof_mtbdd_plant_resources(&controller, &inputs, &encoded, permissive)
            .unwrap();
    assert_eq!(assessment.members, 2);
    assert_eq!(assessment.maximum_product_states, 4);
    assert_eq!(assessment.transition_evaluation_bound, 72);
    let governed = verify_controller_proof_mtbdd_plant_artifact_with_resources(
        &controller,
        controller_digest,
        &inputs,
        &encoded,
        permissive,
    )
    .unwrap();
    assert_eq!(
        (
            governed.verification.safe,
            governed.verification.unsafe_count
        ),
        (1, 1)
    );
    assert_eq!(governed.verification.assignments_checked, 0);
    assert_eq!(governed.resources, assessment);

    let proof_limited = ControllerProofMtbddResourceEnvelope::new(
        composition,
        assessment.equivalence_artifact_bytes - 1,
        assessment.unsat_proof_bytes,
    )
    .unwrap();
    assert!(
        assess_controller_proof_mtbdd_plant_resources(
            &controller,
            &inputs,
            &encoded,
            proof_limited,
        )
        .unwrap_err()
        .0
        .contains("equivalence-artifact limit exceeded")
    );
    let unsat_limited = ControllerProofMtbddResourceEnvelope::new(
        composition,
        assessment.equivalence_artifact_bytes,
        assessment.unsat_proof_bytes - 1,
    )
    .unwrap();
    assert!(
        assess_controller_proof_mtbdd_plant_resources(
            &controller,
            &inputs,
            &encoded,
            unsat_limited,
        )
        .unwrap_err()
        .0
        .contains("UNSAT-proof limit exceeded")
    );
    let mut drift = inputs;
    drift[0].horizon -= 1;
    assert!(
        assess_controller_proof_mtbdd_plant_resources(&controller, &drift, &encoded, permissive,)
            .unwrap_err()
            .0
            .contains("member mismatch")
    );
}

#[test]
fn proof_carrying_portfolio_uses_proof_and_falls_back_only_on_static_rejection() {
    let controller = controller();
    let plant = plant();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let inputs = portfolio_inputs(&plant, &wiring);
    let encoded = produce_controller_proof_mtbdd_plant_portfolio(
        &controller,
        [0xe0; 32],
        &[0],
        &[0],
        &inputs,
    )
    .unwrap();
    let decoded = decode_controller_proof_mtbdd_plant_portfolio(&encoded).unwrap();
    assert_eq!(
        decoded.backend,
        ControllerProofMtbddPlantPortfolioBackend::ProofMtbdd
    );
    assert_eq!(
        decoded.reason,
        ControllerMtbddPlantSelectionReason::MtbddAdmitted
    );
    assert_eq!(
        encode_controller_proof_mtbdd_plant_portfolio(&decoded).unwrap(),
        encoded
    );
    let summary = verify_controller_proof_mtbdd_plant_portfolio(
        &controller,
        [0xe0; 32],
        &[0],
        &[0],
        &inputs,
        &encoded,
    )
    .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
    assert_eq!(summary.assignments_checked, 0);
    let proof_composition =
        ControllerPlantResourceEnvelope::new(encoded.len(), 2, 8, 4, 72).unwrap();
    let proof_envelope = ControllerProofMtbddResourceEnvelope::new(
        proof_composition,
        guarded_continuation_checker::controller_mtbdd_proof::MAX_EQUIVALENCE_ARTIFACT_BYTES,
        guarded_continuation_checker::unsat_proof::MAX_UNSAT_PROOF_BYTES,
    )
    .unwrap();
    let governed = verify_controller_proof_mtbdd_plant_portfolio_with_resources(
        &controller,
        [0xe0; 32],
        &[0],
        &[0],
        &inputs,
        &encoded,
        proof_envelope,
    )
    .unwrap();
    assert_eq!(
        governed.resources.backend,
        ControllerProofMtbddPlantPortfolioBackend::ProofMtbdd
    );
    assert!(governed.resources.unsat_proof_bytes > 1);
    assert_eq!(governed.verification.assignments_checked, 0);
    let tight_proof = ControllerProofMtbddResourceEnvelope::new(
        proof_composition,
        governed.resources.equivalence_artifact_bytes,
        governed.resources.unsat_proof_bytes - 1,
    )
    .unwrap();
    assert!(
        assess_controller_proof_mtbdd_plant_portfolio_resources(
            &controller,
            &[0],
            &[0],
            &inputs,
            &encoded,
            tight_proof,
        )
        .unwrap_err()
        .0
        .contains("UNSAT-proof limit exceeded")
    );
    assert!(
        assess_controller_proof_mtbdd_plant_portfolio_resources(
            &controller,
            &[],
            &[0],
            &inputs,
            &encoded,
            proof_envelope,
        )
        .unwrap_err()
        .0
        .contains("resource boundary mismatch")
    );
    let mismatched_wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![],
        ..wiring.clone()
    };
    let mismatched_inputs = portfolio_inputs(&plant, &mismatched_wiring);
    assert!(
        assess_controller_proof_mtbdd_plant_portfolio_resources(
            &controller,
            &[0],
            &[0],
            &mismatched_inputs,
            &encoded,
            proof_envelope,
        )
        .unwrap_err()
        .0
        .contains("member boundary mismatch")
    );

    let wide = wide_state_controller();
    let fallback =
        produce_controller_proof_mtbdd_plant_portfolio(&wide, [0xe1; 32], &[0], &[0], &inputs)
            .unwrap();
    let decoded_fallback = decode_controller_proof_mtbdd_plant_portfolio(&fallback).unwrap();
    assert_eq!(
        decoded_fallback.backend,
        ControllerProofMtbddPlantPortfolioBackend::DirectExact
    );
    assert_eq!(
        decoded_fallback.reason,
        ControllerMtbddPlantSelectionReason::BoundaryLimit
    );
    let fallback_summary = verify_controller_proof_mtbdd_plant_portfolio(
        &wide,
        [0xe1; 32],
        &[0],
        &[0],
        &inputs,
        &fallback,
    )
    .unwrap();
    assert_eq!(
        (fallback_summary.safe, fallback_summary.unsafe_count),
        (1, 1)
    );
    let fallback_composition =
        ControllerPlantResourceEnvelope::new(fallback.len(), 2, 8, 256, 4608).unwrap();
    let fallback_envelope =
        ControllerProofMtbddResourceEnvelope::new(fallback_composition, 1, 1).unwrap();
    let governed_fallback = verify_controller_proof_mtbdd_plant_portfolio_with_resources(
        &wide,
        [0xe1; 32],
        &[0],
        &[0],
        &inputs,
        &fallback,
        fallback_envelope,
    )
    .unwrap();
    assert_eq!(
        governed_fallback.resources.backend,
        ControllerProofMtbddPlantPortfolioBackend::DirectExact
    );
    assert_eq!(governed_fallback.resources.equivalence_artifact_bytes, 0);
    assert_eq!(governed_fallback.resources.unsat_proof_bytes, 0);
    assert_eq!(
        governed_fallback.resources.transition_evaluation_bound,
        4608
    );

    let forced = encode_controller_proof_mtbdd_plant_portfolio(
        &ControllerProofMtbddPlantPortfolioArtifact {
            version: decoded.version,
            backend: ControllerProofMtbddPlantPortfolioBackend::DirectExact,
            reason: ControllerMtbddPlantSelectionReason::BoundaryLimit,
            relevant_inputs: vec![0],
            observed_outputs: vec![0],
            payload: decoded_fallback.payload,
        },
    )
    .unwrap();
    assert!(
        verify_controller_proof_mtbdd_plant_portfolio(
            &controller,
            [0xe0; 32],
            &[0],
            &[0],
            &inputs,
            &forced,
        )
        .unwrap_err()
        .0
        .contains("downgrade detected")
    );
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_proof_mtbdd_plant_portfolio(&mutated).is_err());
    }
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
