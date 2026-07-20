use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerPlantArtifactInput, ControllerPlantResourceEnvelope,
    ControllerProofEvidenceResourceEnvelope, admit_controller_proof_evidence,
    admit_controller_proof_evidence_with_resources, assess_bound_plant_results_resources,
    assess_controller_proof_evidence_resources, decode_bound_plant_results_artifact,
    decode_controller_mtbdd_plant_artifact, decode_controller_proof_evidence_artifact,
    decode_controller_proof_mtbdd_plant_artifact, encode_bound_plant_results_artifact,
    encode_controller_mtbdd_plant_artifact, encode_controller_proof_evidence_artifact,
    encode_controller_proof_mtbdd_plant_artifact, produce_bound_plant_results_artifact,
    produce_bound_plant_results_with_admitted_controller, produce_controller_mtbdd_plant_artifact,
    produce_controller_proof_evidence_artifact, produce_controller_proof_mtbdd_plant_artifact,
    verify_bound_plant_results_artifact, verify_bound_plant_results_with_admitted_controller,
    verify_bound_plant_results_with_resources, verify_controller_mtbdd_plant_artifact,
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
    assert_eq!(summary.assignments_checked, 0);

    let mut corrupted = encoded;
    let middle = corrupted.len() / 2;
    corrupted[middle] ^= 1;
    assert!(decode_controller_proof_mtbdd_plant_artifact(&corrupted).is_err());
}

#[test]
fn controller_evidence_is_reused_across_a_plant_replacement() {
    let controller = controller();
    let first_plant = plant();
    let mut replacement_plant = plant();
    replacement_plant.outputs[1] = 0;
    let controller_digest = [0x91; 32];
    let first_digest = [0xa1; 32];
    let replacement_digest = [0xa2; 32];
    let mtbdd = produce_controller_mtbdd(&controller, controller_digest, &[0], &[0]).unwrap();
    let evidence =
        produce_controller_proof_evidence_artifact(&controller, controller_digest, &mtbdd).unwrap();
    let evidence_bytes = encode_controller_proof_evidence_artifact(&evidence).unwrap();
    assert_eq!(
        encode_controller_proof_evidence_artifact(
            &decode_controller_proof_evidence_artifact(&evidence_bytes).unwrap()
        )
        .unwrap(),
        evidence_bytes
    );
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: vec![0],
        plant_sensor_outputs: vec![0],
        plant_action_inputs: vec![0],
    };
    let make_input = |plant_ref, digest| ControllerPlantArtifactInput {
        plant: plant_ref,
        plant_source_sha256: digest,
        wiring: &wiring,
        initial_controller_state: 0,
        initial_plant_state: 0,
        bad_plant_output: 1,
        horizon: 8,
    };
    let first = produce_bound_plant_results_artifact(
        &controller,
        controller_digest,
        &evidence_bytes,
        &[make_input(&first_plant, first_digest)],
    )
    .unwrap();
    let replacement = produce_bound_plant_results_artifact(
        &controller,
        controller_digest,
        &evidence_bytes,
        &[make_input(&replacement_plant, replacement_digest)],
    )
    .unwrap();
    let first_bytes = encode_bound_plant_results_artifact(&first).unwrap();
    let replacement_bytes = encode_bound_plant_results_artifact(&replacement).unwrap();
    assert_ne!(first_bytes, replacement_bytes);
    assert_eq!(
        encode_bound_plant_results_artifact(
            &decode_bound_plant_results_artifact(&replacement_bytes).unwrap()
        )
        .unwrap(),
        replacement_bytes
    );
    let admitted =
        admit_controller_proof_evidence(&controller, controller_digest, &evidence_bytes).unwrap();
    assert_eq!(admitted.summary().assignments_checked, 0);
    let proof_limits = ControllerProofEvidenceResourceEnvelope::new(
        evidence_bytes.len(),
        guarded_continuation_checker::unsat_proof::MAX_UNSAT_PROOF_BYTES,
    )
    .unwrap();
    let proof_resources =
        assess_controller_proof_evidence_resources(&evidence_bytes, proof_limits).unwrap();
    let exact_proof_limits = ControllerProofEvidenceResourceEnvelope::new(
        proof_resources.artifact_bytes,
        proof_resources.unsat_proof_bytes,
    )
    .unwrap();
    let governed_admission = admit_controller_proof_evidence_with_resources(
        &controller,
        controller_digest,
        &evidence_bytes,
        exact_proof_limits,
    )
    .unwrap();
    assert_eq!(governed_admission.resources, proof_resources);
    assert!(
        assess_controller_proof_evidence_resources(
            &evidence_bytes,
            ControllerProofEvidenceResourceEnvelope::new(
                evidence_bytes.len() - 1,
                proof_resources.unsat_proof_bytes,
            )
            .unwrap(),
        )
        .is_err()
    );
    assert!(
        assess_controller_proof_evidence_resources(
            &evidence_bytes,
            ControllerProofEvidenceResourceEnvelope::new(
                evidence_bytes.len(),
                proof_resources.unsat_proof_bytes - 1,
            )
            .unwrap(),
        )
        .is_err()
    );
    let replacement_input = [make_input(&replacement_plant, replacement_digest)];
    let plant_resources = assess_bound_plant_results_resources(
        &controller,
        &replacement_input,
        &replacement_bytes,
        ControllerPlantResourceEnvelope::default(),
    )
    .unwrap();
    let exact_plant_limits = ControllerPlantResourceEnvelope::new(
        plant_resources.artifact_bytes,
        plant_resources.members,
        plant_resources.maximum_member_horizon,
        plant_resources.maximum_product_states,
        plant_resources.transition_evaluation_bound,
    )
    .unwrap();
    let governed = verify_bound_plant_results_with_resources(
        &governed_admission.admitted,
        &controller,
        &replacement_input,
        &replacement_bytes,
        exact_plant_limits,
    )
    .unwrap();
    assert_eq!(governed.resources, plant_resources);
    assert_eq!(governed.verification.safe, 1);
    for tight in [
        ControllerPlantResourceEnvelope::new(
            plant_resources.artifact_bytes - 1,
            1,
            plant_resources.maximum_member_horizon,
            plant_resources.maximum_product_states,
            plant_resources.transition_evaluation_bound,
        )
        .unwrap(),
        ControllerPlantResourceEnvelope::new(
            plant_resources.artifact_bytes,
            1,
            plant_resources.maximum_member_horizon - 1,
            plant_resources.maximum_product_states,
            plant_resources.transition_evaluation_bound,
        )
        .unwrap(),
        ControllerPlantResourceEnvelope::new(
            plant_resources.artifact_bytes,
            1,
            plant_resources.maximum_member_horizon,
            plant_resources.maximum_product_states - 1,
            plant_resources.transition_evaluation_bound,
        )
        .unwrap(),
        ControllerPlantResourceEnvelope::new(
            plant_resources.artifact_bytes,
            1,
            plant_resources.maximum_member_horizon,
            plant_resources.maximum_product_states,
            plant_resources.transition_evaluation_bound - 1,
        )
        .unwrap(),
    ] {
        assert!(
            assess_bound_plant_results_resources(
                &controller,
                &replacement_input,
                &replacement_bytes,
                tight,
            )
            .is_err()
        );
    }
    assert_eq!(
        verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &[make_input(&first_plant, first_digest)],
            &first_bytes,
        )
        .unwrap()
        .safe,
        1
    );
    assert_eq!(
        verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &[make_input(&replacement_plant, replacement_digest)],
            &replacement_bytes,
        )
        .unwrap()
        .safe,
        1
    );
    assert!(
        verify_bound_plant_results_artifact(
            &controller,
            controller_digest,
            &evidence_bytes,
            &[make_input(&replacement_plant, first_digest)],
            &replacement_bytes,
        )
        .is_err()
    );
    assert!(
        verify_bound_plant_results_artifact(
            &controller,
            controller_digest,
            &evidence_bytes,
            &[make_input(&first_plant, first_digest)],
            &replacement_bytes,
        )
        .is_err()
    );
    let mut obligation_drift = make_input(&replacement_plant, replacement_digest);
    obligation_drift.bad_plant_output = 0;
    assert!(
        verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &[obligation_drift],
            &replacement_bytes,
        )
        .is_err()
    );

    let other_digest = [0x92; 32];
    let other_mtbdd = produce_controller_mtbdd(&controller, other_digest, &[0], &[0]).unwrap();
    let other_evidence = encode_controller_proof_evidence_artifact(
        &produce_controller_proof_evidence_artifact(&controller, other_digest, &other_mtbdd)
            .unwrap(),
    )
    .unwrap();
    let other_admitted =
        admit_controller_proof_evidence(&controller, other_digest, &other_evidence).unwrap();
    assert!(
        verify_bound_plant_results_with_admitted_controller(
            &other_admitted,
            &[make_input(&replacement_plant, replacement_digest)],
            &replacement_bytes,
        )
        .is_err()
    );
    let first_obligation = make_input(&replacement_plant, replacement_digest);
    let mut second_obligation = first_obligation;
    second_obligation.bad_plant_output = 0;
    let ordered = [first_obligation, second_obligation];
    let ordered_bytes = encode_bound_plant_results_artifact(
        &produce_bound_plant_results_with_admitted_controller(&admitted, &ordered).unwrap(),
    )
    .unwrap();
    assert!(
        verify_bound_plant_results_with_admitted_controller(&admitted, &ordered, &ordered_bytes,)
            .is_ok()
    );
    assert!(
        verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &[second_obligation, first_obligation],
            &ordered_bytes,
        )
        .is_err()
    );
    assert!(
        verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &[first_obligation, first_obligation],
            &ordered_bytes,
        )
        .is_err()
    );
    assert!(
        verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &[first_obligation],
            &ordered_bytes,
        )
        .is_err()
    );

    let mut wrong_evidence = evidence_bytes.clone();
    let middle = wrong_evidence.len() / 2;
    wrong_evidence[middle] ^= 1;
    assert!(
        verify_bound_plant_results_artifact(
            &controller,
            controller_digest,
            &wrong_evidence,
            &[make_input(&replacement_plant, replacement_digest)],
            &replacement_bytes,
        )
        .is_err()
    );
    for length in 0..evidence_bytes.len() {
        assert!(decode_controller_proof_evidence_artifact(&evidence_bytes[..length]).is_err());
    }
    for index in 0..evidence_bytes.len() {
        let mut mutated = evidence_bytes.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_proof_evidence_artifact(&mutated).is_err());
    }
    for length in 0..replacement_bytes.len() {
        assert!(decode_bound_plant_results_artifact(&replacement_bytes[..length]).is_err());
    }
    for index in 0..replacement_bytes.len() {
        let mut mutated = replacement_bytes.clone();
        mutated[index] ^= 1;
        assert!(decode_bound_plant_results_artifact(&mutated).is_err());
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
