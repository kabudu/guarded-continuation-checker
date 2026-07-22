use guarded_continuation_checker::btor2_bitblast::{
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelTracePattern, Btor2ChannelTraceQuery,
    MAX_CHANNEL_TRACE_PATTERN_LENGTH, build_btor2_channel_property_model,
    build_btor2_channel_trace_model,
};
use guarded_continuation_checker::btor2_search::{self, SearchResult};

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn trace_query(length: u8, mask: u8, value: u8, horizon: u32) -> Btor2ChannelTraceQuery {
    Btor2ChannelTraceQuery {
        query_id: 7,
        channel_index: 0,
        pattern: Btor2ChannelTracePattern::new(length, mask, value).unwrap(),
        horizon,
    }
}

fn solve_trace(query: Btor2ChannelTraceQuery) -> btor2_search::SearchSummary {
    let (model, bad) =
        build_btor2_channel_trace_model(MODEL, ROOTS, 6, query, Btor2RegionPolicy::default())
            .unwrap();
    let certificate = btor2_search::produce(&model, bad, query.horizon).unwrap();
    btor2_search::verify(&model, &certificate).unwrap()
}

#[test]
fn trace_pattern_contract_rejects_noncanonical_values() {
    assert!(Btor2ChannelTracePattern::new(0, 1, 0).is_err());
    assert!(Btor2ChannelTracePattern::new(MAX_CHANNEL_TRACE_PATTERN_LENGTH + 1, 1, 0).is_err());
    assert!(Btor2ChannelTracePattern::new(3, 0, 0).is_err());
    assert!(Btor2ChannelTracePattern::new(3, 0b1000, 0).is_err());
    assert!(Btor2ChannelTracePattern::new(3, 0b111, 0b1000).is_err());
    assert!(Btor2ChannelTracePattern::new(3, 0b101, 0b010).is_err());

    let pattern = Btor2ChannelTracePattern::new(3, 0b101, 0b001).unwrap();
    assert_eq!(pattern.length(), 3);
    assert_eq!(pattern.mask(), 0b101);
    assert_eq!(pattern.value(), 0b001);
}

#[test]
fn length_one_trace_controls_match_existing_property_semantics() {
    for (value, property) in [
        (1, Btor2ChannelProperty::OutputHigh),
        (0, Btor2ChannelProperty::OutputLow),
    ] {
        let trace = solve_trace(trace_query(1, 1, value, 1));
        let (property_model, bad) = build_btor2_channel_property_model(
            MODEL,
            ROOTS,
            6,
            0,
            property,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        let property_certificate = btor2_search::produce(&property_model, bad, 1).unwrap();
        let property_summary =
            btor2_search::verify(&property_model, &property_certificate).unwrap();
        assert_eq!(trace.result, property_summary.result);
        assert_eq!(trace.bad_frame, property_summary.bad_frame);
    }
}

#[test]
fn complete_window_gate_does_not_match_zero_padded_prefixes() {
    let summary = solve_trace(trace_query(3, 0b111, 0, 1));
    assert_eq!(summary.result, SearchResult::Safe);
    assert_eq!(summary.bad_frame, None);
}

#[test]
fn low_to_high_monitor_recovers_the_known_pwm_transition() {
    let query = trace_query(2, 0b11, 0b01, 2);
    let (model, bad) =
        build_btor2_channel_trace_model(MODEL, ROOTS, 6, query, Btor2RegionPolicy::default())
            .unwrap();
    let certificate = produce_btor2_bitblast_certificate(&model, bad, query.horizon).unwrap();
    let summary = verify_btor2_bitblast_certificate(&model, &certificate).unwrap();
    assert_eq!(summary.result, SearchResult::Unsafe);
    assert_eq!(summary.bad_frame, Some(2));
}

#[test]
fn trace_model_rejects_source_and_channel_boundary_violations() {
    let query = trace_query(2, 0b11, 0b01, 2);
    let mut out_of_range = query;
    out_of_range.channel_index = 6;
    assert!(
        build_btor2_channel_trace_model(
            MODEL,
            ROOTS,
            6,
            out_of_range,
            Btor2RegionPolicy::default()
        )
        .is_err()
    );

    let mut property_bearing = MODEL.to_vec();
    property_bearing.extend_from_slice(b"\n1000 bad 22 injected\n");
    assert!(
        build_btor2_channel_trace_model(
            &property_bearing,
            ROOTS,
            6,
            query,
            Btor2RegionPolicy::default()
        )
        .is_err()
    );
}
