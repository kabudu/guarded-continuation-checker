use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_bitblast::{
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2BooleanBoundary, Btor2ChannelHistoryClass, Btor2ChannelHistoryGuard,
    Btor2ChannelPairRelation, Btor2ChannelPhaseAbstractionQuery, Btor2GuardedChannelHistoryQuery,
    build_btor2_channel_phase_abstraction_models,
};
use guarded_continuation_checker::btor2_search::SearchResult;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn boundary(input_node: u64, bit_index: u32) -> Btor2BooleanBoundary {
    Btor2BooleanBoundary {
        input_node,
        bit_index,
    }
}

fn class(bit_index: u32) -> Btor2ChannelHistoryClass {
    Btor2ChannelHistoryClass {
        write: boundary(2, bit_index),
        enable: boundary(4, bit_index),
        invert: boundary(3, bit_index),
    }
}

fn query(phase_value: u64) -> Btor2ChannelPhaseAbstractionQuery {
    Btor2ChannelPhaseAbstractionQuery {
        history: Btor2GuardedChannelHistoryQuery {
            left_channel_index: 0,
            right_channel_index: 1,
            relation: Btor2ChannelPairRelation::Equal,
            left_class: class(0),
            right_class: class(1),
            guard: Btor2ChannelHistoryGuard::BothUnwritten,
            horizon: 8,
        },
        phase_root: 9,
        phase_value,
    }
}

#[test]
fn phase_abstraction_separates_non_vacuity_from_relation_proof() {
    let models = build_btor2_channel_phase_abstraction_models(
        MODEL,
        ROOTS,
        6,
        query(0),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert_eq!(models.abstract_state_bits, 10);
    assert_eq!(models.concrete_state_nodes, 33);
    assert_eq!(models.concrete_state_bits, 69);
    assert!(models.abstract_state_bits < models.concrete_state_bits);
    assert_eq!(
        btor2::parse_bytes(&models.reachability_model)
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

    let reachable =
        produce_btor2_bitblast_certificate(&models.reachability_model, models.reachability_bad, 8)
            .unwrap();
    assert_eq!(reachable.result, SearchResult::Unsafe);
    assert_eq!(reachable.bad_frame, Some(0));
    verify_btor2_bitblast_certificate(&models.reachability_model, &reachable).unwrap();

    let relation =
        produce_btor2_bitblast_certificate(&models.relation_model, models.relation_bad, 0).unwrap();
    verify_btor2_bitblast_certificate(&models.relation_model, &relation).unwrap();
}

#[test]
fn phase_abstraction_rejects_root_width_value_and_certificate_drift() {
    for hostile in [
        Btor2ChannelPhaseAbstractionQuery {
            phase_root: 45,
            ..query(0)
        },
        Btor2ChannelPhaseAbstractionQuery {
            phase_root: 39,
            ..query(0)
        },
        query(16),
    ] {
        assert!(
            build_btor2_channel_phase_abstraction_models(
                MODEL,
                ROOTS,
                6,
                hostile,
                Btor2RegionPolicy::default(),
            )
            .is_err()
        );
    }

    let phase_zero = build_btor2_channel_phase_abstraction_models(
        MODEL,
        ROOTS,
        6,
        query(0),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    let certificate =
        produce_btor2_bitblast_certificate(&phase_zero.relation_model, phase_zero.relation_bad, 0)
            .unwrap();
    let phase_one = build_btor2_channel_phase_abstraction_models(
        MODEL,
        ROOTS,
        6,
        query(1),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert!(verify_btor2_bitblast_certificate(&phase_one.relation_model, &certificate).is_err());
}
