use guarded_continuation_checker::btor2_bitblast::{
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelTraceBackend, Btor2ChannelTracePattern,
    Btor2ChannelTraceProductionPolicy, Btor2ChannelTraceProofPolicy, Btor2ChannelTraceQuery,
    Btor2ChannelTraceSolver, MAX_CHANNEL_TRACE_PATTERN_LENGTH, build_btor2_channel_property_model,
    build_btor2_channel_trace_model, preflight_btor2_channel_trace_proof,
    produce_btor2_channel_trace_proof, verify_btor2_channel_trace_proof,
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

fn structural_admission() -> Vec<u8> {
    encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(MODEL, ROOTS, 6, Btor2RegionPolicy::default())
            .unwrap(),
    )
    .unwrap()
}

fn composed_queries() -> Vec<Btor2ChannelTraceQuery> {
    let shapes = [
        (Btor2ChannelTracePattern::new(1, 1, 1).unwrap(), 1),
        (Btor2ChannelTracePattern::new(1, 1, 0).unwrap(), 1),
        (Btor2ChannelTracePattern::new(2, 0b11, 0b01).unwrap(), 2),
    ];
    let mut queries = Vec::new();
    for (pattern, horizon) in shapes {
        for channel_index in 0..6 {
            queries.push(Btor2ChannelTraceQuery {
                query_id: queries.len() as u32,
                channel_index,
                pattern,
                horizon,
            });
        }
    }
    queries
}

#[test]
fn verified_classes_compose_both_answer_trace_proofs() {
    let structural = structural_admission();
    let queries = composed_queries();
    let artifact = produce_btor2_channel_trace_proof(
        MODEL,
        &structural,
        &queries,
        Btor2RegionPolicy::default(),
        Btor2ChannelTraceProductionPolicy::default(),
    )
    .unwrap();
    let summary = verify_btor2_channel_trace_proof(
        MODEL,
        &queries,
        &artifact,
        Btor2RegionPolicy::default(),
        Btor2ChannelTraceProofPolicy::default(),
    )
    .unwrap();

    assert_eq!(summary.metrics.logical_queries, 18);
    assert_eq!(summary.metrics.proof_members, 9);
    assert_eq!(summary.metrics.representative_members, 6);
    assert_eq!(summary.metrics.direct_exact_members, 3);
    assert_eq!(summary.metrics.explicit_state_members, 6);
    assert_eq!(summary.metrics.bitblast_members, 3);
    assert_eq!(summary.metrics.reused_logical_queries, 9);
    for result in &summary.results[..6] {
        assert_eq!(result.result, SearchResult::Safe);
        assert_eq!(result.bad_frame, None);
    }
    for result in &summary.results[6..12] {
        assert_eq!(result.result, SearchResult::Unsafe);
        assert_eq!(result.bad_frame, Some(0));
    }
    for result in &summary.results[12..] {
        assert_eq!(result.result, SearchResult::Unsafe);
        assert_eq!(result.bad_frame, Some(2));
    }
    assert_eq!(
        summary.results[0].backend,
        Btor2ChannelTraceBackend::RepresentativeClass
    );
    assert_eq!(
        summary.results[6].solver,
        Btor2ChannelTraceSolver::ExplicitState
    );
    assert_eq!(
        summary.results[12].solver,
        Btor2ChannelTraceSolver::BitblastCnf
    );
}

#[test]
fn trace_proof_preflight_and_verifier_fail_closed() {
    let structural = structural_admission();
    let queries = composed_queries();
    let region_policy = Btor2RegionPolicy::default();
    let plan = preflight_btor2_channel_trace_proof(
        MODEL,
        &structural,
        &queries,
        region_policy,
        Btor2ChannelTraceProductionPolicy::default(),
    )
    .unwrap();
    assert_eq!(plan.logical_queries, 18);
    assert_eq!(plan.proof_members, 9);
    assert!(plan.projected_work > 1);
    let refused = Btor2ChannelTraceProductionPolicy::new(
        Btor2ChannelTraceProofPolicy::default(),
        plan.projected_work - 1,
    )
    .unwrap();
    assert!(
        produce_btor2_channel_trace_proof(MODEL, &structural, &queries, region_policy, refused)
            .is_err()
    );

    let artifact = produce_btor2_channel_trace_proof(
        MODEL,
        &structural,
        &queries,
        region_policy,
        Btor2ChannelTraceProductionPolicy::default(),
    )
    .unwrap();
    let mut query_drift = queries.clone();
    query_drift[0].pattern = Btor2ChannelTracePattern::new(1, 1, 0).unwrap();
    assert!(
        verify_btor2_channel_trace_proof(
            MODEL,
            &query_drift,
            &artifact,
            region_policy,
            Btor2ChannelTraceProofPolicy::default()
        )
        .is_err()
    );

    let mut backend_drift = artifact.clone();
    backend_drift.members[0].backend = Btor2ChannelTraceBackend::DirectExact;
    assert!(
        verify_btor2_channel_trace_proof(
            MODEL,
            &queries,
            &backend_drift,
            region_policy,
            Btor2ChannelTraceProofPolicy::default()
        )
        .is_err()
    );

    let mut evidence_drift = artifact;
    evidence_drift.members[0].evidence[0] ^= 1;
    assert!(
        verify_btor2_channel_trace_proof(
            MODEL,
            &queries,
            &evidence_drift,
            region_policy,
            Btor2ChannelTraceProofPolicy::default()
        )
        .is_err()
    );
}
