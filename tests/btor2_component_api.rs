use guarded_continuation_checker::btor2_component::{
    self, ComponentBackend, ComponentResult, ComponentSelectionReason,
};

const CONTROLLER: &[u8] =
    include_bytes!("../examples/btor2/components/braking-controller-v1.btor2");
const PLANT: &[u8] = include_bytes!("../examples/btor2/components/motion-plant-v1.btor2");
const CONTRACT: &[u8] =
    include_bytes!("../examples/btor2/components/braking-motion-contract-v1.txt");

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
