use guarded_continuation_checker::btor2_component::{
    self, ComponentBackend, ComponentResult, ComponentSelectionReason,
};

const CONTROLLER: &[u8] =
    include_bytes!("../examples/btor2/components/braking-controller-v1.btor2");
const PLANT: &[u8] = include_bytes!("../examples/btor2/components/motion-plant-v1.btor2");
const CONTRACT: &[u8] =
    include_bytes!("../examples/btor2/components/braking-motion-contract-v1.txt");

#[test]
fn embedded_component_fixtures_are_canonical_lf_text() {
    for source in [CONTROLLER, PLANT, CONTRACT] {
        assert!(!source.contains(&b'\r'));
        assert!(!source.contains(&0));
        assert!(source.ends_with(b"\n"));
    }
}

#[test]
fn public_component_api_preserves_source_separation_and_both_answers() {
    let safe = btor2_component::produce(CONTROLLER, PLANT, CONTRACT, 255).unwrap();
    assert_eq!(
        safe.selection_reason,
        ComponentSelectionReason::ExactPhaseContractSafe
    );
    let encoded = btor2_component::encode(&safe.certificate).unwrap();
    let certificate = btor2_component::decode(encoded.as_bytes()).unwrap();
    let summary = btor2_component::verify(CONTROLLER, PLANT, CONTRACT, &certificate).unwrap();
    assert_eq!(summary.backend, ComponentBackend::PhaseContract);
    assert_eq!(summary.result, ComponentResult::Safe);
    assert_eq!(summary.logical_reachable_states, 32_896);

    let unsafe_result = btor2_component::produce(CONTROLLER, PLANT, CONTRACT, 256).unwrap();
    assert_eq!(
        unsafe_result.selection_reason,
        ComponentSelectionReason::SpecialisedInapplicableOrIntersecting
    );
    let encoded = btor2_component::encode(&unsafe_result.certificate).unwrap();
    let certificate = btor2_component::decode(encoded.as_bytes()).unwrap();
    let summary = btor2_component::verify(CONTROLLER, PLANT, CONTRACT, &certificate).unwrap();
    assert_eq!(summary.backend, ComponentBackend::ComposedSearch);
    assert_eq!(summary.result, ComponentResult::Unsafe);
    assert_eq!(summary.bad_frame, Some(256));
}

#[test]
fn public_controller_obligation_api_round_trips_and_verifies() {
    let obligation = btor2_component::produce_controller_obligation(CONTROLLER, CONTRACT).unwrap();
    let encoded = btor2_component::encode_controller_obligation(&obligation).unwrap();
    let decoded = btor2_component::decode_controller_obligation(encoded.as_bytes()).unwrap();
    btor2_component::verify_controller_obligation(CONTROLLER, &decoded).unwrap();
    assert_eq!(decoded.velocity_width, 16);
    assert_eq!(decoded.brake_velocity, 256);
}

#[test]
fn public_naive_batch_baseline_preserves_member_bindings() {
    let inputs = [
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 255,
        },
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 256,
        },
    ];
    let certificate = btor2_component::produce_naive_component_batch(CONTROLLER, &inputs).unwrap();
    let summary =
        btor2_component::verify_naive_component_batch(CONTROLLER, &inputs, &certificate).unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
}

#[test]
fn public_reusable_batch_api_round_trips_and_preserves_exact_answers() {
    let inputs = [
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 255,
        },
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 256,
        },
    ];
    let certificate =
        btor2_component::produce_reusable_component_batch(CONTROLLER, &inputs).unwrap();
    assert!(matches!(
        certificate.members[0],
        btor2_component::ReusableBatchMember::ReusedPhase(_)
    ));
    assert!(matches!(
        certificate.members[1],
        btor2_component::ReusableBatchMember::ExactFallback(_)
    ));
    let encoded = btor2_component::encode_reusable_component_batch(&certificate).unwrap();
    let decoded = btor2_component::decode_reusable_component_batch(encoded.as_bytes()).unwrap();
    let summary =
        btor2_component::verify_reusable_component_batch(CONTROLLER, &inputs, &decoded).unwrap();
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
}

#[test]
fn public_batch_portfolio_routes_without_timing_calibration() {
    let admitted = [
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 254,
        },
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 255,
        },
    ];
    let production =
        btor2_component::produce_component_batch_portfolio(CONTROLLER, &admitted).unwrap();
    assert_eq!(
        production.selection_reason,
        btor2_component::ComponentBatchSelectionReason::FullyAdmittedReuse
    );
    let encoded =
        btor2_component::encode_component_batch_portfolio(&production.certificate).unwrap();
    let decoded = btor2_component::decode_component_batch_portfolio(encoded.as_bytes()).unwrap();
    assert_eq!(
        btor2_component::verify_component_batch_portfolio(CONTROLLER, &admitted, &decoded)
            .unwrap()
            .safe,
        2
    );

    let mixed = [
        admitted[0],
        btor2_component::ComponentBatchInput {
            plant_source: PLANT,
            contract_source: CONTRACT,
            horizon: 256,
        },
    ];
    let production =
        btor2_component::produce_component_batch_portfolio(CONTROLLER, &mixed).unwrap();
    assert_eq!(
        production.selection_reason,
        btor2_component::ComponentBatchSelectionReason::SingletonOrExactFallback
    );
    assert!(matches!(
        production.certificate,
        btor2_component::ComponentBatchPortfolioCertificate::Ordinary(_)
    ));
}
