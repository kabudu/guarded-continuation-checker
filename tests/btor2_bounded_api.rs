use guarded_continuation_checker::{btor2_bounded, btor2_search};

const WATCHDOG: &[u8] = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
const MOTION: &[u8] = include_bytes!("../examples/btor2/motion-envelope-v1.btor2");
const BRAKING: &[u8] = include_bytes!("../examples/btor2/braking-controller-v1.btor2");

#[test]
fn public_bounded_api_preserves_both_exact_answers() {
    let safe = btor2_bounded::produce(WATCHDOG, 13, 2).unwrap();
    let safe_bytes = btor2_bounded::encode(&safe).unwrap();
    let safe = btor2_bounded::decode(safe_bytes.as_bytes()).unwrap();
    let safe = btor2_bounded::verify(WATCHDOG, &safe).unwrap();
    assert_eq!(safe.result, btor2_search::SearchResult::Safe);
    assert_eq!(safe.logical_reachable_states, 6);

    let unsafe_certificate = btor2_bounded::produce(WATCHDOG, 13, 3).unwrap();
    let unsafe_bytes = btor2_bounded::encode(&unsafe_certificate).unwrap();
    let unsafe_certificate = btor2_bounded::decode(unsafe_bytes.as_bytes()).unwrap();
    let unsafe_summary = btor2_bounded::verify(WATCHDOG, &unsafe_certificate).unwrap();
    assert_eq!(unsafe_summary.result, btor2_search::SearchResult::Unsafe);
    assert_eq!(unsafe_summary.bad_frame, Some(3));
}

#[test]
fn public_bounded_api_exposes_exact_coupled_motion_and_fallback() {
    let production = btor2_bounded::produce_with_observation(MOTION, 21, 200).unwrap();
    assert_eq!(
        production.selection_reason.as_str(),
        "motion-curve-exact-safe"
    );
    let safe = production.certificate;
    let safe = btor2_bounded::decode(btor2_bounded::encode(&safe).unwrap().as_bytes()).unwrap();
    let summary = btor2_bounded::verify(MOTION, &safe).unwrap();
    assert_eq!(summary.backend, btor2_bounded::BoundedBackend::MotionCurve);
    assert_eq!(summary.result, btor2_search::SearchResult::Safe);
    assert_eq!(summary.logical_reachable_states, 20_301);

    let unsafe_certificate = btor2_bounded::produce(MOTION, 21, 201).unwrap();
    let summary = btor2_bounded::verify(MOTION, &unsafe_certificate).unwrap();
    assert_eq!(
        summary.backend,
        btor2_bounded::BoundedBackend::ExplicitSearch
    );
    assert_eq!(summary.result, btor2_search::SearchResult::Unsafe);
    assert_eq!(summary.bad_frame, Some(201));
}

#[test]
fn public_bounded_api_exposes_phase_composed_braking_and_fallback() {
    let production = btor2_bounded::produce_with_observation(BRAKING, 31, 255).unwrap();
    assert_eq!(
        production.selection_reason.as_str(),
        "braking-phases-exact-safe"
    );
    let encoded = btor2_bounded::encode(&production.certificate).unwrap();
    let certificate = btor2_bounded::decode(encoded.as_bytes()).unwrap();
    let summary = btor2_bounded::verify(BRAKING, &certificate).unwrap();
    assert_eq!(
        summary.backend,
        btor2_bounded::BoundedBackend::BrakingPhases
    );
    assert_eq!(summary.result, btor2_search::SearchResult::Safe);
    assert_eq!(summary.logical_reachable_states, 32_896);

    let unsafe_certificate = btor2_bounded::produce(BRAKING, 31, 256).unwrap();
    let summary = btor2_bounded::verify(BRAKING, &unsafe_certificate).unwrap();
    assert_eq!(
        summary.backend,
        btor2_bounded::BoundedBackend::ExplicitSearch
    );
    assert_eq!(summary.result, btor2_search::SearchResult::Unsafe);
    assert_eq!(summary.bad_frame, Some(256));
}
