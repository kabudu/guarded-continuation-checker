use guarded_continuation_checker::btor2_predicate_set::{
    self, PredicateSetRoute, PredicateSetSelectionReason,
};

const OPENTITAN_WATCHDOG: &[u8] = include_bytes!(
    "../corpus/rtl/opentitan-aon-timer/generated/watchdog-predicate-set-small.btor2"
);
const OPENTITAN_DUAL_TIMER: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-aon-timer/generated/dual-timer-predicate-set.btor2");

#[test]
fn downstream_client_can_share_and_independently_verify_open_titan_predicates() {
    let production = btor2_predicate_set::produce(OPENTITAN_WATCHDOG, &[18, 22], 4).unwrap();
    assert_eq!(
        production.selection_reason,
        PredicateSetSelectionReason::SharedExactRecurrence
    );
    let encoded = btor2_predicate_set::encode(&production.certificate).unwrap();
    let decoded = btor2_predicate_set::decode(encoded.as_bytes()).unwrap();
    let summary = btor2_predicate_set::verify(OPENTITAN_WATCHDOG, &[18, 22], 4, &decoded).unwrap();
    assert_eq!(summary.route, PredicateSetRoute::SharedExactRegion);
    assert_eq!((summary.safe, summary.unsafe_count), (2, 0));

    let mixed = btor2_predicate_set::produce(OPENTITAN_WATCHDOG, &[18, 22], 5).unwrap();
    let summary =
        btor2_predicate_set::verify(OPENTITAN_WATCHDOG, &[18, 22], 5, &mixed.certificate).unwrap();
    assert_eq!(summary.route, PredicateSetRoute::SharedExactRegion);
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
    assert_eq!(summary.members[0].bad_frame, Some(5));
}

#[test]
fn downstream_client_can_verify_invariant_chained_open_titan_timers() {
    let properties = [33, 37, 41];
    let production = btor2_predicate_set::produce(OPENTITAN_DUAL_TIMER, &properties, 9).unwrap();
    assert_eq!(
        production.selection_reason,
        PredicateSetSelectionReason::InvariantChainedRecurrences
    );
    assert_eq!(
        btor2_predicate_set::certificate_version(&production.certificate),
        3
    );
    let encoded = btor2_predicate_set::encode(&production.certificate).unwrap();
    let decoded = btor2_predicate_set::decode(encoded.as_bytes()).unwrap();
    let summary =
        btor2_predicate_set::verify(OPENTITAN_DUAL_TIMER, &properties, 9, &decoded).unwrap();
    assert_eq!(summary.route, PredicateSetRoute::InvariantChainedRegions);
    assert_eq!((summary.safe, summary.unsafe_count), (0, 3));
    assert_eq!(
        summary
            .members
            .iter()
            .map(|member| member.bad_frame)
            .collect::<Vec<_>>(),
        vec![Some(9), Some(5), Some(7)]
    );
    assert_eq!(summary.logical_reachable_states, 55);
}
