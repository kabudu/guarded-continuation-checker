use guarded_continuation_checker::btor2_predicate_set::{
    self, PredicateSetRoute, PredicateSetSelectionReason,
};

const OPENTITAN_WATCHDOG: &[u8] = include_bytes!(
    "../corpus/rtl/opentitan-aon-timer/generated/watchdog-predicate-set-small.btor2"
);

#[test]
fn downstream_client_can_share_and_independently_verify_open_titan_predicates() {
    let production = btor2_predicate_set::produce(OPENTITAN_WATCHDOG, &[18, 22], 4).unwrap();
    assert_eq!(
        production.selection_reason,
        PredicateSetSelectionReason::SharedEvidenceSmaller
    );
    let encoded = btor2_predicate_set::encode(&production.certificate).unwrap();
    let decoded = btor2_predicate_set::decode(encoded.as_bytes()).unwrap();
    let summary = btor2_predicate_set::verify(OPENTITAN_WATCHDOG, &[18, 22], 4, &decoded).unwrap();
    assert_eq!(summary.route, PredicateSetRoute::SharedRegion);
    assert_eq!((summary.safe, summary.unsafe_count), (2, 0));

    let mixed = btor2_predicate_set::produce(OPENTITAN_WATCHDOG, &[18, 22], 5).unwrap();
    let summary =
        btor2_predicate_set::verify(OPENTITAN_WATCHDOG, &[18, 22], 5, &mixed.certificate).unwrap();
    assert_eq!(summary.route, PredicateSetRoute::OrdinaryExact);
    assert_eq!((summary.safe, summary.unsafe_count), (1, 1));
    assert_eq!(summary.members[0].bad_frame, Some(5));
}
