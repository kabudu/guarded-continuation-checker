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
