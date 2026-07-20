use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring,
};
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerMtbddPlantPortfolioBackend, ControllerMtbddPlantSelectionReason,
    ControllerPlantArtifactInput, ControllerPlantResourceEnvelope,
    assess_controller_mtbdd_plant_portfolio_resources, decode_controller_mtbdd_plant_portfolio,
    encode_controller_mtbdd_plant_portfolio, produce_controller_mtbdd_plant_portfolio,
    verify_controller_mtbdd_plant_portfolio,
    verify_controller_mtbdd_plant_portfolio_with_resources,
};

fn controller(outputs: usize) -> AigerTransition {
    AigerTransition {
        max_variable: 2,
        inputs: vec![2],
        latches: vec![AigerLatch {
            current: 4,
            next: 2,
        }],
        outputs: vec![2; outputs],
        ands: vec![],
    }
}

fn envelope(
    artifact_bytes: usize,
    members: usize,
    horizon: usize,
    product_states: usize,
    transitions: usize,
) -> ControllerPlantResourceEnvelope {
    ControllerPlantResourceEnvelope::new(
        artifact_bytes,
        members,
        horizon,
        product_states,
        transitions,
    )
    .unwrap()
}

fn plant(actions: usize, bad: usize) -> AigerTransition {
    AigerTransition {
        max_variable: actions + 1,
        inputs: (1..=actions).map(|variable| variable * 2).collect(),
        latches: vec![AigerLatch {
            current: (actions + 1) * 2,
            next: 0,
        }],
        outputs: vec![0, bad],
        ands: vec![],
    }
}

fn wiring(actions: usize) -> ControllerPlantWiring {
    ControllerPlantWiring {
        controller_sensor_inputs: vec![0],
        controller_action_outputs: (0..actions).collect(),
        plant_sensor_outputs: vec![0],
        plant_action_inputs: (0..actions).collect(),
    }
}

#[test]
fn portfolio_uses_mtbdd_when_admitted_and_exact_fallback_at_output_limit() {
    let safe = plant(9, 0);
    let unsafe_plant = plant(9, 1);
    let fallback_controller = controller(9);
    let fallback_wiring = wiring(9);
    let fallback_inputs = [
        ControllerPlantArtifactInput {
            plant: &safe,
            plant_source_sha256: [0x21; 32],
            wiring: &fallback_wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 4,
        },
        ControllerPlantArtifactInput {
            plant: &unsafe_plant,
            plant_source_sha256: [0x22; 32],
            wiring: &fallback_wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 4,
        },
    ];
    let fallback = produce_controller_mtbdd_plant_portfolio(
        &fallback_controller,
        [0x11; 32],
        &[0],
        &(0..9).collect::<Vec<_>>(),
        &fallback_inputs,
    )
    .unwrap();
    let decoded = decode_controller_mtbdd_plant_portfolio(&fallback).unwrap();
    assert_eq!(
        decoded.backend,
        ControllerMtbddPlantPortfolioBackend::DirectExact
    );
    assert_eq!(
        decoded.reason,
        ControllerMtbddPlantSelectionReason::BoundaryLimit
    );
    assert_eq!(
        encode_controller_mtbdd_plant_portfolio(&decoded).unwrap(),
        fallback
    );
    let summary = verify_controller_mtbdd_plant_portfolio(
        &fallback_controller,
        [0x11; 32],
        &[0],
        &(0..9).collect::<Vec<_>>(),
        &fallback_inputs,
        &fallback,
    )
    .unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
    assert_eq!(summary.members[0].answer, ControllerPlantAnswer::Safe);
    assert_eq!(summary.members[1].answer, ControllerPlantAnswer::Unsafe);
    assert_eq!(summary.members[1].bad_frame, Some(0));

    let admitted_controller = controller(1);
    let admitted_plant = plant(1, 0);
    let admitted_wiring = wiring(1);
    let admitted_inputs = [ControllerPlantArtifactInput {
        plant: &admitted_plant,
        plant_source_sha256: [0x31; 32],
        wiring: &admitted_wiring,
        initial_controller_state: 0,
        initial_plant_state: 0,
        bad_plant_output: 1,
        horizon: 4,
    }];
    let admitted = produce_controller_mtbdd_plant_portfolio(
        &admitted_controller,
        [0x12; 32],
        &[0],
        &[0],
        &admitted_inputs,
    )
    .unwrap();
    let admitted = decode_controller_mtbdd_plant_portfolio(&admitted).unwrap();
    assert_eq!(
        admitted.backend,
        ControllerMtbddPlantPortfolioBackend::Mtbdd
    );
    assert_eq!(
        admitted.reason,
        ControllerMtbddPlantSelectionReason::MtbddAdmitted
    );

    let mut wrong_reason = decoded;
    wrong_reason.reason = ControllerMtbddPlantSelectionReason::NodeLimit;
    let wrong_reason = encode_controller_mtbdd_plant_portfolio(&wrong_reason).unwrap();
    assert!(
        verify_controller_mtbdd_plant_portfolio(
            &fallback_controller,
            [0x11; 32],
            &[0],
            &(0..9).collect::<Vec<_>>(),
            &fallback_inputs,
            &wrong_reason,
        )
        .is_err()
    );
    assert!(
        produce_controller_mtbdd_plant_portfolio(
            &fallback_controller,
            [0x11; 32],
            &[1],
            &(0..9).collect::<Vec<_>>(),
            &fallback_inputs,
        )
        .is_err(),
        "invalid boundary was incorrectly treated as a resource-limit fallback"
    );
}

#[test]
fn resource_envelope_preflights_both_exact_routes_without_timing_calibration() {
    let safe = plant(9, 0);
    let unsafe_plant = plant(9, 1);
    let fallback_controller = controller(9);
    let fallback_wiring = wiring(9);
    let inputs = [
        ControllerPlantArtifactInput {
            plant: &safe,
            plant_source_sha256: [0x41; 32],
            wiring: &fallback_wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 4,
        },
        ControllerPlantArtifactInput {
            plant: &unsafe_plant,
            plant_source_sha256: [0x42; 32],
            wiring: &fallback_wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output: 1,
            horizon: 4,
        },
    ];
    let artifact = produce_controller_mtbdd_plant_portfolio(
        &fallback_controller,
        [0x40; 32],
        &[0],
        &(0..9).collect::<Vec<_>>(),
        &inputs,
    )
    .unwrap();
    let policy = envelope(artifact.len(), 2, 4, 4, 40);
    let assessment = assess_controller_mtbdd_plant_portfolio_resources(
        &fallback_controller,
        &inputs,
        &artifact,
        policy,
    )
    .unwrap();
    assert_eq!(
        assessment.backend,
        ControllerMtbddPlantPortfolioBackend::DirectExact
    );
    assert_eq!(assessment.maximum_product_states, 4);
    assert_eq!(assessment.transition_evaluation_bound, 40);
    let governed = verify_controller_mtbdd_plant_portfolio_with_resources(
        &fallback_controller,
        [0x40; 32],
        &[0],
        &(0..9).collect::<Vec<_>>(),
        &inputs,
        &artifact,
        policy,
    )
    .unwrap();
    assert_eq!(
        (
            governed.verification.safe,
            governed.verification.unsafe_count
        ),
        (1, 1)
    );
    assert_eq!(governed.resources, assessment);

    let admitted_controller = controller(1);
    let admitted_plant = plant(1, 0);
    let admitted_wiring = wiring(1);
    let admitted_inputs = [ControllerPlantArtifactInput {
        plant: &admitted_plant,
        plant_source_sha256: [0x44; 32],
        wiring: &admitted_wiring,
        initial_controller_state: 0,
        initial_plant_state: 0,
        bad_plant_output: 1,
        horizon: 4,
    }];
    let admitted = produce_controller_mtbdd_plant_portfolio(
        &admitted_controller,
        [0x43; 32],
        &[0],
        &[0],
        &admitted_inputs,
    )
    .unwrap();
    let admitted_assessment = assess_controller_mtbdd_plant_portfolio_resources(
        &admitted_controller,
        &admitted_inputs,
        &admitted,
        envelope(admitted.len(), 1, 4, 4, 20),
    )
    .unwrap();
    assert_eq!(
        admitted_assessment.backend,
        ControllerMtbddPlantPortfolioBackend::Mtbdd
    );
    assert_eq!(admitted_assessment.transition_evaluation_bound, 20);
}

#[test]
fn resource_envelope_rejects_each_bound_before_semantic_replay() {
    let plant = plant(1, 0);
    let controller = controller(1);
    let wiring = wiring(1);
    let inputs = [ControllerPlantArtifactInput {
        plant: &plant,
        plant_source_sha256: [0x51; 32],
        wiring: &wiring,
        initial_controller_state: 0,
        initial_plant_state: 0,
        bad_plant_output: 1,
        horizon: 4,
    }];
    let artifact =
        produce_controller_mtbdd_plant_portfolio(&controller, [0x50; 32], &[0], &[0], &inputs)
            .unwrap();
    let cases = [
        (
            envelope(artifact.len() - 1, 1, 4, 4, 20),
            "artifact-byte limit exceeded",
        ),
        (
            envelope(artifact.len(), 1, 3, 4, 20),
            "horizon limit exceeded",
        ),
        (
            envelope(artifact.len(), 1, 4, 3, 20),
            "product-state limit exceeded",
        ),
        (
            envelope(artifact.len(), 1, 4, 4, 19),
            "transition limit exceeded",
        ),
    ];
    for (policy, expected) in cases {
        let error = assess_controller_mtbdd_plant_portfolio_resources(
            &controller,
            &inputs,
            &artifact,
            policy,
        )
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
    let member_error = assess_controller_mtbdd_plant_portfolio_resources(
        &controller,
        &[],
        &artifact,
        envelope(artifact.len(), 1, 4, 4, 20),
    )
    .unwrap_err();
    assert!(member_error.to_string().contains("member limit exceeded"));

    let mut corrupt = artifact.clone();
    corrupt[0] ^= 1;
    let error = assess_controller_mtbdd_plant_portfolio_resources(
        &controller,
        &inputs,
        &corrupt,
        envelope(corrupt.len() - 1, 1, 4, 4, 20),
    )
    .unwrap_err();
    assert!(error.to_string().contains("artifact-byte limit exceeded"));
}
