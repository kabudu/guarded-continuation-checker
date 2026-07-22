use guarded_continuation_checker::revision_impact::{
    MinimalSemanticChangeSet, TwoComponentRevisionImpactInput, derive_minimal_semantic_change_sets,
    produce_two_component_revision_impact, verify_two_component_revision_impact,
};
use guarded_continuation_checker::revision_local::{BoundedQuery, BoundedResult, ComponentSide};

const CORE_BEFORE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-before.btor2");
const CORE_AFTER: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-after.btor2");
const CHANNEL_BEFORE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-before.btor2");
const CHANNEL_AFTER: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-after.btor2");
const INTERFACE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/interface.txt");

fn queries() -> [BoundedQuery; 5] {
    [
        BoundedQuery {
            horizon: 0,
            bad_side: ComponentSide::Right,
            bad_output: 1004,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1000,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1001,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1002,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1003,
        },
    ]
}

fn make_input<'a>(
    left_old: &'a [u8],
    left_new: &'a [u8],
    interface_old: &'a [u8],
    interface_new: &'a [u8],
    queries: &'a [BoundedQuery],
) -> TwoComponentRevisionImpactInput<'a> {
    TwoComponentRevisionImpactInput {
        left_old,
        left_new,
        left_outputs: &[1000, 1001, 1002, 1003],
        right_old: CHANNEL_BEFORE,
        right_new: CHANNEL_AFTER,
        right_outputs: &[1000, 1001, 1002, 1003, 1004],
        interface_old,
        interface_new,
        queries,
    }
}

#[test]
fn authentic_connected_changes_have_distinct_and_joint_semantic_sets() {
    let queries = queries();
    let input = TwoComponentRevisionImpactInput {
        left_old: CORE_BEFORE,
        left_new: CORE_AFTER,
        left_outputs: &[1000, 1001, 1002, 1003],
        right_old: CHANNEL_BEFORE,
        right_new: CHANNEL_AFTER,
        right_outputs: &[1000, 1001, 1002, 1003, 1004],
        interface_old: INTERFACE,
        interface_new: INTERFACE,
        queries: &queries,
    };
    let bundle = produce_two_component_revision_impact(&input).unwrap();
    let summary = verify_two_component_revision_impact(&input, &bundle).unwrap();
    assert_eq!(
        (summary.atoms, summary.queries, summary.combinations),
        (2, 5, 4)
    );

    let results = bundle
        .impact
        .observations
        .iter()
        .map(|observation| observation.result)
        .collect::<Vec<_>>();
    use BoundedResult::{Safe, Unsafe};
    assert_eq!(
        results,
        vec![
            Unsafe, Unsafe, Unsafe, Unsafe, Safe, Unsafe, Safe, Unsafe, Unsafe, Safe, Unsafe,
            Unsafe, Safe, Unsafe, Safe, Unsafe, Safe, Safe, Safe, Safe,
        ]
    );

    assert_eq!(
        derive_minimal_semantic_change_sets(&bundle.impact).unwrap(),
        vec![
            MinimalSemanticChangeSet {
                query_index: 1,
                changed_mask: 1,
                baseline_result: Unsafe,
                changed_result: Safe,
            },
            MinimalSemanticChangeSet {
                query_index: 2,
                changed_mask: 2,
                baseline_result: Unsafe,
                changed_result: Safe,
            },
            MinimalSemanticChangeSet {
                query_index: 3,
                changed_mask: 3,
                baseline_result: Unsafe,
                changed_result: Safe,
            },
        ]
    );
    assert_eq!(summary.minimal_semantic_change_sets, 3);
}

#[test]
fn authentic_connected_change_bundle_fails_closed_on_bound_drift() {
    let queries = queries();
    let input = make_input(CORE_BEFORE, CORE_AFTER, INTERFACE, INTERFACE, &queries);
    let bundle = produce_two_component_revision_impact(&input).unwrap();

    let mut source_drift = CORE_BEFORE.to_vec();
    source_drift.push(b'\n');
    let source_drift_input = make_input(&source_drift, CORE_AFTER, INTERFACE, INTERFACE, &queries);
    assert!(verify_two_component_revision_impact(&source_drift_input, &bundle).is_err());

    let interface_drift = String::from_utf8(INTERFACE.to_vec())
        .unwrap()
        .replace("wire=left,1000,2", "wire=left,1000,3")
        .into_bytes();
    let interface_drift_input = make_input(
        CORE_BEFORE,
        CORE_AFTER,
        &interface_drift,
        &interface_drift,
        &queries,
    );
    assert!(verify_two_component_revision_impact(&interface_drift_input, &bundle).is_err());

    let mut query_drift = queries.clone();
    query_drift[3].horizon += 1;
    let query_drift_input = make_input(CORE_BEFORE, CORE_AFTER, INTERFACE, INTERFACE, &query_drift);
    assert!(verify_two_component_revision_impact(&query_drift_input, &bundle).is_err());

    let reversed_revision_input =
        make_input(CORE_AFTER, CORE_BEFORE, INTERFACE, INTERFACE, &queries);
    assert!(verify_two_component_revision_impact(&reversed_revision_input, &bundle).is_err());

    let mut atom_order_drift = bundle.clone();
    atom_order_drift.impact.atoms.swap(0, 1);
    assert!(verify_two_component_revision_impact(&input, &atom_order_drift).is_err());

    let mut evidence_drift = bundle.clone();
    evidence_drift.revision_evidence[7][16] ^= 1;
    assert!(verify_two_component_revision_impact(&input, &evidence_drift).is_err());

    let mut digest_drift = bundle.clone();
    digest_drift.impact.observations[7].evidence_sha256[0] ^= 1;
    assert!(verify_two_component_revision_impact(&input, &digest_drift).is_err());

    let mut result_drift = bundle.clone();
    result_drift.impact.observations[0].result = BoundedResult::Safe;
    assert!(verify_two_component_revision_impact(&input, &result_drift).is_err());

    let mut minimal_set_drift = bundle;
    minimal_set_drift.impact.minimal_invalidating_sets.pop();
    assert!(verify_two_component_revision_impact(&input, &minimal_set_drift).is_err());
}
