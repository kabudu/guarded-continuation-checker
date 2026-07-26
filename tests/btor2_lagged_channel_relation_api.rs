use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_bitblast::{
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelPairRelation, Btor2LaggedChannelOrientation, Btor2LaggedChannelRelationQuery,
    build_btor2_lagged_channel_relation_models,
};
use guarded_continuation_checker::btor2_search::SearchResult;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn query() -> Btor2LaggedChannelRelationQuery {
    Btor2LaggedChannelRelationQuery {
        left_channel_index: 0,
        right_channel_index: 1,
        orientation: Btor2LaggedChannelOrientation::LeftLeads,
        relation: Btor2ChannelPairRelation::Equal,
        lag: 2,
        horizon: 4,
    }
}

#[test]
fn lagged_relation_separates_prefix_coverage_from_the_relation() {
    let models = build_btor2_lagged_channel_relation_models(
        MODEL,
        ROOTS,
        6,
        query(),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert_eq!(models.history_state_bits, 4);
    assert_eq!(
        btor2::parse_bytes(&models.coverage_model)
            .unwrap()
            .bad_properties()
            .len(),
        1
    );
    assert_eq!(
        btor2::parse_bytes(&models.relation_model)
            .unwrap()
            .bad_properties()
            .len(),
        1
    );

    let coverage =
        produce_btor2_bitblast_certificate(&models.coverage_model, models.coverage_bad, 4).unwrap();
    assert_eq!(coverage.result, SearchResult::Unsafe);
    assert_eq!(coverage.bad_frame, Some(2));
    verify_btor2_bitblast_certificate(&models.coverage_model, &coverage).unwrap();

    let relation =
        produce_btor2_bitblast_certificate(&models.relation_model, models.relation_bad, 4).unwrap();
    verify_btor2_bitblast_certificate(&models.relation_model, &relation).unwrap();
}

#[test]
fn lagged_relation_rejects_endpoint_lag_horizon_and_certificate_drift() {
    for hostile in [
        Btor2LaggedChannelRelationQuery {
            right_channel_index: 0,
            ..query()
        },
        Btor2LaggedChannelRelationQuery { lag: 0, ..query() },
        Btor2LaggedChannelRelationQuery { lag: 1, ..query() },
        Btor2LaggedChannelRelationQuery { lag: 3, ..query() },
        Btor2LaggedChannelRelationQuery {
            horizon: 1,
            ..query()
        },
    ] {
        assert!(
            build_btor2_lagged_channel_relation_models(
                MODEL,
                ROOTS,
                6,
                hostile,
                Btor2RegionPolicy::default(),
            )
            .is_err()
        );
    }

    let original = build_btor2_lagged_channel_relation_models(
        MODEL,
        ROOTS,
        6,
        query(),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    let certificate =
        produce_btor2_bitblast_certificate(&original.relation_model, original.relation_bad, 4)
            .unwrap();
    let reversed = build_btor2_lagged_channel_relation_models(
        MODEL,
        ROOTS,
        6,
        Btor2LaggedChannelRelationQuery {
            orientation: Btor2LaggedChannelOrientation::RightLeads,
            ..query()
        },
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert!(verify_btor2_bitblast_certificate(&reversed.relation_model, &certificate).is_err());

    let different = build_btor2_lagged_channel_relation_models(
        MODEL,
        ROOTS,
        6,
        Btor2LaggedChannelRelationQuery {
            relation: Btor2ChannelPairRelation::Different,
            ..query()
        },
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert!(verify_btor2_bitblast_certificate(&different.relation_model, &certificate).is_err());

    let longer = build_btor2_lagged_channel_relation_models(
        MODEL,
        ROOTS,
        6,
        Btor2LaggedChannelRelationQuery {
            horizon: 8,
            ..query()
        },
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert!(verify_btor2_bitblast_certificate(&longer.relation_model, &certificate).is_err());
}
