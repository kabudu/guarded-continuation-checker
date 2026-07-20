use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_plant::{
    ControllerPlantAnswer, ControllerPlantWiring,
};
use guarded_continuation_checker::controller_plant_artifact::{
    ControllerMtbddPlantPortfolioBackend, ControllerMtbddPlantSelectionReason,
    ControllerPlantArtifactInput, decode_controller_mtbdd_plant_portfolio,
    encode_controller_mtbdd_plant_portfolio, produce_controller_mtbdd_plant_portfolio,
    verify_controller_mtbdd_plant_portfolio,
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
