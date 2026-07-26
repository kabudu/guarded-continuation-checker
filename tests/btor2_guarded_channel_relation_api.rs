use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2BooleanBoundary, Btor2ChannelPairRelation, Btor2GuardedChannelRelationQuery,
    build_btor2_guarded_channel_relation_model,
};
use guarded_continuation_checker::btor2_search;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn query(
    left: usize,
    right: usize,
    relation: Btor2ChannelPairRelation,
    input_node: u64,
    bit_index: u32,
) -> Btor2GuardedChannelRelationQuery {
    Btor2GuardedChannelRelationQuery {
        left_channel_index: left,
        right_channel_index: right,
        relation,
        guard: Btor2BooleanBoundary {
            input_node,
            bit_index,
        },
        horizon: 0,
    }
}

#[test]
fn guarded_relation_builder_binds_a_source_input_bit() {
    let guarded = query(0, 1, Btor2ChannelPairRelation::Equal, 2, 0);
    let (bytes, bad) = build_btor2_guarded_channel_relation_model(
        MODEL,
        ROOTS,
        6,
        guarded,
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    let certificate = btor2_search::produce(&bytes, bad, guarded.horizon).unwrap();
    assert_eq!(certificate.query_horizon, guarded.horizon);
}

#[test]
fn guarded_relation_builder_rejects_untrusted_or_invalid_boundaries() {
    for invalid in [
        query(0, 0, Btor2ChannelPairRelation::Equal, 2, 0),
        query(0, 1, Btor2ChannelPairRelation::Equal, 9, 0),
        query(0, 1, Btor2ChannelPairRelation::Equal, 2, 2),
    ] {
        assert!(
            build_btor2_guarded_channel_relation_model(
                MODEL,
                ROOTS,
                6,
                invalid,
                Btor2RegionPolicy::default(),
            )
            .is_err()
        );
    }
}
