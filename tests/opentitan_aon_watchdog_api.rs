use guarded_continuation_checker::{btor2, btor2_bounded, btor2_search};
use std::collections::BTreeMap;

const SMALL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-aon-timer/generated/watchdog-small.btor2");
const SCALE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-aon-timer/generated/watchdog-scale.btor2");
const DUAL_TIMER: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-aon-timer/generated/dual-timer-predicate-set.btor2");

#[test]
fn public_opentitan_dual_timer_model_preserves_three_exact_boundaries() {
    let model = btor2::parse_bytes(DUAL_TIMER).unwrap();
    assert_eq!(model.inputs(), &[2]);
    assert_eq!(model.states(), &[6, 16, 23]);
    assert_eq!(
        model
            .bad_properties()
            .iter()
            .map(|(property, _, _)| *property)
            .collect::<Vec<_>>(),
        vec![33, 37, 41]
    );

    let input = BTreeMap::from([(2, 0)]);
    let mut state = model.initial_state().unwrap();
    assert!(model.active_bad(&state, &input).unwrap().is_empty());
    for frame in 1..=9 {
        state = model.step(&state, &input).unwrap();
        let active = model.active_bad(&state, &input).unwrap();
        match frame {
            5 | 6 => assert_eq!(active, vec![37]),
            7 | 8 => assert_eq!(active, vec![37, 41]),
            9 => assert_eq!(active, vec![33, 37, 41]),
            _ => assert!(active.is_empty()),
        }
    }
}

#[test]
fn public_opentitan_watchdog_path_preserves_exact_boundary_answers() {
    let model = btor2::parse(std::str::from_utf8(SMALL).unwrap()).unwrap();
    assert_eq!(model.inputs(), &[3]);
    assert_eq!(model.states(), &[6]);
    assert_eq!(model.bad_properties()[0].0, 15);

    let safe = btor2_bounded::produce_with_observation(SMALL, 15, 8).unwrap();
    assert_eq!(safe.selection_reason.as_str(), "word-region-exact-safe");
    let summary = btor2_bounded::verify(SMALL, &safe.certificate).unwrap();
    assert_eq!(summary.backend, btor2_bounded::BoundedBackend::WordRegion);
    assert_eq!(summary.result, btor2_search::SearchResult::Safe);
    assert_eq!(summary.logical_reachable_states, 45);

    let unsafe_result = btor2_bounded::produce_with_observation(SMALL, 15, 9).unwrap();
    let summary = btor2_bounded::verify(SMALL, &unsafe_result.certificate).unwrap();
    assert_eq!(
        summary.backend,
        btor2_bounded::BoundedBackend::ExplicitSearch
    );
    assert_eq!(summary.result, btor2_search::SearchResult::Unsafe);
    assert_eq!(summary.bad_frame, Some(9));

    let scale = btor2_bounded::produce_with_observation(SCALE, 15, 1_000_000_000).unwrap();
    let summary = btor2_bounded::verify(SCALE, &scale.certificate).unwrap();
    assert_eq!(summary.backend, btor2_bounded::BoundedBackend::WordRegion);
    assert_eq!(summary.result, btor2_search::SearchResult::Safe);
    assert_eq!(summary.logical_reachable_states, 500_000_001_500_000_001);
}

#[test]
fn public_opentitan_watchdog_near_neighbour_is_not_specialised() {
    let hostile = String::from_utf8(SMALL.to_vec())
        .unwrap()
        .replace("14 and 1 12 13", "14 xor 1 12 13");
    let production = btor2_bounded::produce_with_observation(hostile.as_bytes(), 15, 8).unwrap();
    let summary = btor2_bounded::verify(hostile.as_bytes(), &production.certificate).unwrap();
    assert_eq!(
        summary.backend,
        btor2_bounded::BoundedBackend::ExplicitSearch
    );
}
