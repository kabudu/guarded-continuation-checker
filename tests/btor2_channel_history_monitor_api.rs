use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_bitblast::{
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2BooleanBoundary, Btor2ChannelHistoryClass, Btor2ChannelHistoryGuard,
    Btor2ChannelPairRelation, Btor2GuardedChannelHistoryQuery,
    build_btor2_guarded_channel_history_model,
};

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn boundary(input_node: u64, bit_index: u32) -> Btor2BooleanBoundary {
    Btor2BooleanBoundary {
        input_node,
        bit_index,
    }
}

fn query() -> Btor2GuardedChannelHistoryQuery {
    Btor2GuardedChannelHistoryQuery {
        left_channel_index: 0,
        right_channel_index: 1,
        relation: Btor2ChannelPairRelation::Equal,
        left_class: Btor2ChannelHistoryClass {
            write: boundary(2, 0),
            enable: boundary(4, 0),
            invert: boundary(3, 0),
        },
        right_class: Btor2ChannelHistoryClass {
            write: boundary(2, 1),
            enable: boundary(4, 1),
            invert: boundary(3, 1),
        },
        guard: Btor2ChannelHistoryGuard::SameTrackedConfig,
        horizon: 2,
    }
}

#[test]
fn history_monitor_is_canonical_state_bearing_btor2() {
    let (first, first_bad) = build_btor2_guarded_channel_history_model(
        MODEL,
        ROOTS,
        6,
        query(),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    let (second, second_bad) = build_btor2_guarded_channel_history_model(
        MODEL,
        ROOTS,
        6,
        query(),
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first_bad, second_bad);
    let product = btor2::parse_bytes(&first).unwrap();
    let source = btor2::parse_component_bytes(MODEL, ROOTS).unwrap();
    assert_eq!(product.states().len(), source.states().len() + 6);
    assert_eq!(product.inputs(), source.inputs());
    assert_eq!(product.bad_properties().len(), 1);
}

#[test]
fn history_monitor_rejects_hostile_boundary_or_endpoint_drift() {
    let mut cases = Vec::new();
    let mut identical = query();
    identical.right_channel_index = identical.left_channel_index;
    cases.push(identical);

    let mut alias = query();
    alias.right_class.write = alias.left_class.write;
    cases.push(alias);

    let mut state_guard = query();
    state_guard.left_class.write = boundary(9, 0);
    cases.push(state_guard);

    let mut out_of_range = query();
    out_of_range.left_class.write = boundary(2, 2);
    cases.push(out_of_range);

    for hostile in cases {
        assert!(
            build_btor2_guarded_channel_history_model(
                MODEL,
                ROOTS,
                6,
                hostile,
                Btor2RegionPolicy::default(),
            )
            .is_err()
        );
    }
}

#[test]
fn history_monitor_certificate_rejects_boundary_substitution() {
    let original = query();
    let (original_bytes, original_bad) = build_btor2_guarded_channel_history_model(
        MODEL,
        ROOTS,
        6,
        original,
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    let certificate = produce_btor2_bitblast_certificate(&original_bytes, original_bad, 1).unwrap();
    verify_btor2_bitblast_certificate(&original_bytes, &certificate).unwrap();

    let mut changed = original;
    std::mem::swap(
        &mut changed.left_class.enable,
        &mut changed.left_class.invert,
    );
    let (changed_bytes, _) = build_btor2_guarded_channel_history_model(
        MODEL,
        ROOTS,
        6,
        changed,
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    assert_ne!(original_bytes, changed_bytes);
    assert!(verify_btor2_bitblast_certificate(&changed_bytes, &certificate).is_err());
}

#[test]
fn endpoint_swap_preserves_the_symmetric_relation_answer() {
    let original = query();
    let mut swapped = original;
    std::mem::swap(
        &mut swapped.left_channel_index,
        &mut swapped.right_channel_index,
    );
    std::mem::swap(&mut swapped.left_class, &mut swapped.right_class);
    for candidate in [original, swapped] {
        let (bytes, bad) = build_btor2_guarded_channel_history_model(
            MODEL,
            ROOTS,
            6,
            candidate,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        let certificate = produce_btor2_bitblast_certificate(&bytes, bad, 2).unwrap();
        assert_eq!(
            certificate.result,
            guarded_continuation_checker::btor2_search::SearchResult::Unsafe
        );
        assert_eq!(certificate.bad_frame, Some(2));
        verify_btor2_bitblast_certificate(&bytes, &certificate).unwrap();
    }
}
