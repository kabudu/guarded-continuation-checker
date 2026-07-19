use guarded_continuation_checker::{btor2_bounded, btor2_search};

const WATCHDOG: &[u8] = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");

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
