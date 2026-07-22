use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_region_equivalence::derive_btor2_region_equivalence;
use guarded_continuation_checker::btor2_region_equivalence::{
    MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES, admit_btor2_region_equivalence_artifact,
    decode_btor2_region_equivalence_artifact, encode_btor2_region_equivalence_artifact,
    produce_btor2_region_equivalence_artifact, verify_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelPropertyBackend, Btor2ChannelPropertyQuery,
    produce_btor2_channel_property_evidence, produce_btor2_channel_property_proof,
    verify_btor2_channel_property_proof,
};
use guarded_continuation_checker::btor2_search::{self, SearchResult};

struct Fixture {
    bytes: &'static [u8],
    roots: &'static [u64],
    channels: usize,
    states: usize,
    classes: &'static [&'static [usize]],
}

#[test]
fn symbolic_firmware_class_inputs_admit_only_exact_structural_classes() {
    let fixtures = [
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-2.btor2"
            ),
            roots: &[9, 20],
            channels: 2,
            states: 17,
            classes: &[&[0], &[1]],
        },
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-4.btor2"
            ),
            roots: &[9, 29],
            channels: 4,
            states: 25,
            classes: &[&[0, 2], &[1], &[3]],
        },
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
            ),
            roots: &[9, 39],
            channels: 6,
            states: 33,
            classes: &[&[0, 2, 4], &[1], &[3, 5]],
        },
    ];
    for fixture in fixtures {
        let model = btor2::parse_component_bytes(fixture.bytes, fixture.roots).unwrap();
        assert_eq!(model.inputs().len(), 3);
        assert_eq!(model.states().len(), fixture.states);
        assert!(model.constraints().is_empty());
        assert!(model.bad_properties().is_empty());
        let summary = derive_btor2_region_equivalence(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        assert_eq!(
            summary
                .classes
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>(),
            fixture.classes
        );
        let artifact = produce_btor2_region_equivalence_artifact(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        let encoded = encode_btor2_region_equivalence_artifact(&artifact).unwrap();
        let decoded = decode_btor2_region_equivalence_artifact(&encoded).unwrap();
        assert_eq!(
            verify_btor2_region_equivalence_artifact(
                fixture.bytes,
                &decoded,
                Btor2RegionPolicy::default(),
            )
            .unwrap(),
            summary
        );
        let admission = admit_btor2_region_equivalence_artifact(
            fixture.bytes,
            &decoded,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        assert_eq!(admission.classes(), fixture.classes);
        assert_eq!(
            encoded,
            encode_btor2_region_equivalence_artifact(
                &produce_btor2_region_equivalence_artifact(
                    fixture.bytes,
                    fixture.roots,
                    fixture.channels,
                    Btor2RegionPolicy::default(),
                )
                .unwrap(),
            )
            .unwrap()
        );
    }
}

#[test]
fn structural_admission_fails_closed_under_hostile_changes() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let policy = Btor2RegionPolicy::default();
    let artifact = produce_btor2_region_equivalence_artifact(model, &[9, 39], 6, policy).unwrap();
    let encoded = encode_btor2_region_equivalence_artifact(&artifact).unwrap();

    for end in 0..encoded.len() {
        assert!(decode_btor2_region_equivalence_artifact(&encoded[..end]).is_err());
    }
    for offset in 0..encoded.len() {
        let mut changed = encoded.clone();
        changed[offset] ^= 1;
        assert!(decode_btor2_region_equivalence_artifact(&changed).is_err());
    }
    assert!(
        decode_btor2_region_equivalence_artifact(&vec![
            0;
            MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES + 1
        ])
        .is_err()
    );

    let mut source_drift = model.to_vec();
    source_drift.push(b'\n');
    assert!(verify_btor2_region_equivalence_artifact(&source_drift, &artifact, policy).is_err());

    let mut class_drift = artifact.clone();
    class_drift.summary.classes = (0..6).map(|channel| vec![channel]).collect();
    assert!(verify_btor2_region_equivalence_artifact(model, &class_drift, policy).is_err());

    let mut signature_drift = artifact.clone();
    signature_drift.summary.signatures[0].sha256[0] ^= 1;
    assert!(verify_btor2_region_equivalence_artifact(model, &signature_drift, policy).is_err());
}

fn symbolic_queries() -> Vec<Btor2ChannelPropertyQuery> {
    let mut queries = Vec::new();
    for property in [
        Btor2ChannelProperty::OutputHigh,
        Btor2ChannelProperty::OutputLow,
    ] {
        for channel in 0..6 {
            queries.push(Btor2ChannelPropertyQuery {
                query_id: queries.len() as u32,
                channel_index: channel,
                property,
                horizon: 1,
            });
        }
    }
    queries
}

#[test]
fn verified_classes_reuse_both_answer_property_proofs_and_recover_inputs() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let policy = Btor2RegionPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, &[9, 39], 6, policy).unwrap(),
    )
    .unwrap();
    let queries = symbolic_queries();
    let artifact =
        produce_btor2_channel_property_proof(model, &structural, &queries, policy).unwrap();
    let summary = verify_btor2_channel_property_proof(model, &queries, &artifact, policy).unwrap();

    assert_eq!(summary.metrics.logical_queries, 12);
    assert_eq!(summary.metrics.proof_members, 6);
    assert_eq!(summary.metrics.representative_members, 4);
    assert_eq!(summary.metrics.direct_exact_members, 2);
    assert_eq!(summary.metrics.reused_logical_queries, 6);
    for result in &summary.results[..6] {
        assert_eq!(result.result, SearchResult::Safe);
        assert_eq!(result.bad_frame, None);
    }
    for result in &summary.results[6..] {
        assert_eq!(result.result, SearchResult::Unsafe);
        assert_eq!(result.bad_frame, Some(0));
        assert_eq!(result.terminal_valuation, Some(0));
    }
    assert_eq!(
        summary.results[0].backend,
        Btor2ChannelPropertyBackend::RepresentativeClass
    );
    assert_eq!(
        summary.results[1].backend,
        Btor2ChannelPropertyBackend::DirectExact
    );

    let direct_bytes = queries
        .iter()
        .map(|query| {
            let evidence =
                produce_btor2_channel_property_evidence(model, &[9, 39], 6, *query, policy)
                    .unwrap();
            btor2_search::encode(&evidence.certificate).unwrap().len()
        })
        .sum::<usize>();
    assert!(summary.metrics.evidence_bytes < direct_bytes);
}

#[test]
fn property_proof_rejects_invalid_admission_query_and_member_drift() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let policy = Btor2RegionPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, &[9, 39], 6, policy).unwrap(),
    )
    .unwrap();
    let queries = symbolic_queries();
    let artifact =
        produce_btor2_channel_property_proof(model, &structural, &queries, policy).unwrap();

    let mut invalid_admission = structural.clone();
    invalid_admission[0] ^= 1;
    assert!(
        produce_btor2_channel_property_proof(model, &invalid_admission, &queries, policy).is_err()
    );

    let mut omitted = queries.clone();
    omitted.pop();
    assert!(verify_btor2_channel_property_proof(model, &omitted, &artifact, policy).is_err());

    let mut forced_backend = artifact.clone();
    forced_backend.members[0].backend = Btor2ChannelPropertyBackend::DirectExact;
    assert!(verify_btor2_channel_property_proof(model, &queries, &forced_backend, policy).is_err());

    let mut changed_evidence = artifact.clone();
    changed_evidence.members[0].evidence[0] ^= 1;
    assert!(
        verify_btor2_channel_property_proof(model, &queries, &changed_evidence, policy).is_err()
    );

    let mut source_drift = model.to_vec();
    source_drift.push(b'\n');
    assert!(
        verify_btor2_channel_property_proof(&source_drift, &queries, &artifact, policy).is_err()
    );
}
