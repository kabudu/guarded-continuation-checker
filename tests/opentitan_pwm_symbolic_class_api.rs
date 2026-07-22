use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_bitblast::{
    decode_btor2_bitblast_certificate, encode_btor2_bitblast_certificate,
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_equivalence::derive_btor2_region_equivalence;
use guarded_continuation_checker::btor2_region_equivalence::{
    MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES, admit_btor2_region_equivalence_artifact,
    decode_btor2_region_equivalence_artifact, encode_btor2_region_equivalence_artifact,
    produce_btor2_region_equivalence_artifact, verify_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelPropertyBackend, Btor2ChannelPropertyProductionPolicy,
    Btor2ChannelPropertyProofPolicy, Btor2ChannelPropertyQuery, Btor2ChannelPropertySolver,
    MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES, MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
    MAX_CHANNEL_PROPERTY_PROJECTED_WORK, MAX_CHANNEL_PROPERTY_QUERIES,
    build_btor2_channel_property_model, decode_btor2_channel_property_proof_artifact,
    encode_btor2_channel_property_proof_artifact, preflight_btor2_channel_property_proof,
    produce_btor2_channel_property_evidence, produce_btor2_channel_property_proof,
    produce_btor2_channel_property_proof_bytes,
    produce_btor2_channel_property_proof_bytes_observed,
    produce_btor2_channel_property_proof_bytes_phase_observed,
    produce_btor2_channel_property_proof_bytes_with_policy, verify_btor2_channel_property_proof,
    verify_btor2_channel_property_proof_bytes,
};
use guarded_continuation_checker::btor2_search::{self, SearchResult};
use sha2::{Digest, Sha256};

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

fn symbolic_queries(horizon: u32) -> Vec<Btor2ChannelPropertyQuery> {
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
                horizon,
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
    let queries = symbolic_queries(1);
    let artifact =
        produce_btor2_channel_property_proof(model, &structural, &queries, policy).unwrap();
    let summary = verify_btor2_channel_property_proof(model, &queries, &artifact, policy).unwrap();

    assert_eq!(summary.metrics.logical_queries, 12);
    assert_eq!(summary.metrics.proof_members, 6);
    assert_eq!(summary.metrics.representative_members, 4);
    assert_eq!(summary.metrics.direct_exact_members, 2);
    assert_eq!(summary.metrics.explicit_state_members, 6);
    assert_eq!(summary.metrics.bitblast_members, 0);
    assert_eq!(summary.metrics.reused_logical_queries, 6);
    for result in &summary.results[..6] {
        assert_eq!(result.result, SearchResult::Safe);
        assert_eq!(result.bad_frame, None);
    }
    for result in &summary.results[6..] {
        assert_eq!(result.result, SearchResult::Unsafe);
        assert_eq!(result.bad_frame, Some(0));
        assert_eq!(result.witness_valuations, vec![0]);
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
    let queries = symbolic_queries(1);
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

    let mut forced_solver = artifact.clone();
    forced_solver.members[0].solver = Btor2ChannelPropertySolver::BitblastCnf;
    assert!(verify_btor2_channel_property_proof(model, &queries, &forced_solver, policy).is_err());

    let mut source_drift = model.to_vec();
    source_drift.push(b'\n');
    assert!(
        verify_btor2_channel_property_proof(&source_drift, &queries, &artifact, policy).is_err()
    );
}

#[test]
fn static_portfolio_routes_horizon_two_to_bitblast_without_trial_solving() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let policy = Btor2RegionPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, &[9, 39], 6, policy).unwrap(),
    )
    .unwrap();
    let queries = symbolic_queries(2);
    let artifact =
        produce_btor2_channel_property_proof(model, &structural, &queries, policy).unwrap();
    assert!(
        artifact
            .members
            .iter()
            .all(|member| member.solver == Btor2ChannelPropertySolver::BitblastCnf)
    );
    let summary = verify_btor2_channel_property_proof(model, &queries, &artifact, policy).unwrap();
    assert_eq!(summary.metrics.logical_queries, 12);
    assert_eq!(summary.metrics.proof_members, 6);
    assert_eq!(summary.metrics.reused_logical_queries, 6);
    assert_eq!(summary.metrics.explicit_state_members, 0);
    assert_eq!(summary.metrics.bitblast_members, 6);
    for result in &summary.results[..6] {
        assert_eq!(result.result, SearchResult::Unsafe);
        assert_eq!(result.bad_frame, Some(2));
        assert_eq!(result.witness_valuations.len(), 3);
    }
    for result in &summary.results[6..] {
        assert_eq!(result.result, SearchResult::Unsafe);
        assert_eq!(result.bad_frame, Some(0));
        assert_eq!(result.witness_valuations.len(), 1);
    }

    let mut changed_bitblast_evidence = artifact.clone();
    changed_bitblast_evidence.members[0].evidence[80] ^= 1;
    assert!(
        verify_btor2_channel_property_proof(model, &queries, &changed_bitblast_evidence, policy,)
            .is_err()
    );

    let outside_fallback_horizon = (0..6)
        .map(|channel| Btor2ChannelPropertyQuery {
            query_id: channel as u32,
            channel_index: channel,
            property: Btor2ChannelProperty::OutputHigh,
            horizon: 65,
        })
        .collect::<Vec<_>>();
    assert!(
        produce_btor2_channel_property_proof(
            model,
            &structural,
            &outside_fallback_horizon,
            policy,
        )
        .is_err()
    );
}

#[test]
fn aggregate_production_preflight_refuses_the_complete_batch_before_solving() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let region_policy = Btor2RegionPolicy::default();
    let artifact_policy = Btor2ChannelPropertyProofPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, &[9, 39], 6, region_policy).unwrap(),
    )
    .unwrap();
    let queries = symbolic_queries(2);
    let plan = preflight_btor2_channel_property_proof(
        model,
        &structural,
        &queries,
        region_policy,
        Btor2ChannelPropertyProductionPolicy::default(),
    )
    .unwrap();
    assert_eq!(plan.logical_queries, 12);
    assert_eq!(plan.proof_members, 6);
    assert_eq!(plan.explicit_state_members, 0);
    assert_eq!(plan.bitblast_members, 6);
    assert!(plan.projected_work > 1);

    let query_limited = Btor2ChannelPropertyProofPolicy::new(
        11,
        MAX_CHANNEL_PROPERTY_QUERIES,
        MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
        MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
    )
    .unwrap();
    assert!(
        preflight_btor2_channel_property_proof(
            model,
            &structural,
            &queries,
            region_policy,
            Btor2ChannelPropertyProductionPolicy::new(query_limited, plan.projected_work).unwrap(),
        )
        .is_err()
    );
    let member_limited = Btor2ChannelPropertyProofPolicy::new(
        MAX_CHANNEL_PROPERTY_QUERIES,
        5,
        MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
        MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
    )
    .unwrap();
    assert!(
        preflight_btor2_channel_property_proof(
            model,
            &structural,
            &queries,
            region_policy,
            Btor2ChannelPropertyProductionPolicy::new(member_limited, plan.projected_work).unwrap(),
        )
        .is_err()
    );

    let refused =
        Btor2ChannelPropertyProductionPolicy::new(artifact_policy, plan.projected_work - 1)
            .unwrap();
    assert!(
        produce_btor2_channel_property_proof_bytes_with_policy(
            model,
            &structural,
            &queries,
            region_policy,
            refused,
        )
        .is_err()
    );
    let admitted =
        Btor2ChannelPropertyProductionPolicy::new(artifact_policy, plan.projected_work).unwrap();
    let (observed_plan, observed_bytes) = produce_btor2_channel_property_proof_bytes_observed(
        model,
        &structural,
        &queries,
        region_policy,
        admitted,
    )
    .unwrap();
    assert_eq!(observed_plan, plan);
    assert_eq!(observed_bytes.len(), 1_568);
    let (phase_plan, phase_bytes, phases) =
        produce_btor2_channel_property_proof_bytes_phase_observed(
            model,
            &structural,
            &queries,
            region_policy,
            admitted,
        )
        .unwrap();
    assert_eq!(phase_plan, observed_plan);
    assert_eq!(phase_bytes, observed_bytes);
    assert!(
        phases.preflight_micros + phases.proof_construction_micros + phases.encoding_micros
            <= phases.total_micros
    );
    assert!(Btor2ChannelPropertyProductionPolicy::new(artifact_policy, 0).is_err());
    assert!(
        Btor2ChannelPropertyProductionPolicy::new(
            artifact_policy,
            MAX_CHANNEL_PROPERTY_PROJECTED_WORK + 1,
        )
        .is_err()
    );
}

#[test]
fn outer_property_portfolio_codec_is_canonical_bounded_and_fail_closed() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let region_policy = Btor2RegionPolicy::default();
    let artifact_policy = Btor2ChannelPropertyProofPolicy::default();
    for limits in [
        (
            0,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES + 1,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES,
            0,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES + 1,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES,
            0,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES + 1,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            0,
        ),
        (
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES + 1,
        ),
    ] {
        assert!(
            Btor2ChannelPropertyProofPolicy::new(limits.0, limits.1, limits.2, limits.3).is_err()
        );
    }
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, &[9, 39], 6, region_policy).unwrap(),
    )
    .unwrap();
    let queries = symbolic_queries(2);
    let artifact =
        produce_btor2_channel_property_proof(model, &structural, &queries, region_policy).unwrap();
    let encoded = encode_btor2_channel_property_proof_artifact(&artifact, artifact_policy).unwrap();
    assert_eq!(encoded.len(), 1568);
    assert_eq!(
        <[u8; 32]>::from(Sha256::digest(&encoded)),
        [
            0x31, 0xdb, 0x59, 0x02, 0x5d, 0x13, 0x87, 0x29, 0x59, 0xc1, 0x17, 0x83, 0xd6, 0xf1,
            0x88, 0x7f, 0xd9, 0x8f, 0x3b, 0xac, 0x9e, 0x02, 0x34, 0xf3, 0xda, 0x7f, 0xb8, 0x8e,
            0xd5, 0x2e, 0x34, 0x86,
        ]
    );

    assert_eq!(
        encoded,
        produce_btor2_channel_property_proof_bytes(
            model,
            &structural,
            &queries,
            region_policy,
            artifact_policy,
        )
        .unwrap()
    );
    assert_eq!(
        decode_btor2_channel_property_proof_artifact(&encoded, artifact_policy).unwrap(),
        artifact
    );
    let summary = verify_btor2_channel_property_proof_bytes(
        model,
        &queries,
        &encoded,
        region_policy,
        artifact_policy,
    )
    .unwrap();
    assert_eq!(summary.metrics.logical_queries, 12);
    assert_eq!(summary.metrics.proof_members, 6);
    assert_eq!(summary.metrics.bitblast_members, 6);

    for end in 0..encoded.len() {
        assert!(
            decode_btor2_channel_property_proof_artifact(&encoded[..end], artifact_policy).is_err()
        );
    }
    for offset in 0..encoded.len() {
        let mut changed = encoded.clone();
        changed[offset] ^= 1;
        assert!(decode_btor2_channel_property_proof_artifact(&changed, artifact_policy).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(decode_btor2_channel_property_proof_artifact(&trailing, artifact_policy).is_err());

    let refresh_checksum = |bytes: &mut Vec<u8>| {
        let payload_end = bytes.len() - 32;
        let checksum = Sha256::digest(&bytes[..payload_end]);
        bytes[payload_end..].copy_from_slice(&checksum);
    };
    let query_count_offset = 8 + 4 + 32 + 4 + structural.len();
    let member_count_offset = query_count_offset + 4 + queries.len() * (4 + 4 + 1 + 4);
    let first_member_offset = member_count_offset + 4;
    for (offset, replacement) in [
        (8, 2u32.to_le_bytes().to_vec()),
        (
            8 + 4 + 32,
            ((MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES as u32) + 1)
                .to_le_bytes()
                .to_vec(),
        ),
        (
            query_count_offset,
            ((MAX_CHANNEL_PROPERTY_QUERIES as u32) + 1)
                .to_le_bytes()
                .to_vec(),
        ),
        (member_count_offset, 7u32.to_le_bytes().to_vec()),
        (first_member_offset, 5u32.to_le_bytes().to_vec()),
        (first_member_offset + 14, vec![2]),
        (
            first_member_offset + 15,
            ((guarded_continuation_checker::btor2_bitblast::MAX_BITBLAST_CERTIFICATE_BYTES as u32)
                + 1)
            .to_le_bytes()
            .to_vec(),
        ),
    ] {
        let mut changed = encoded.clone();
        changed[offset..offset + replacement.len()].copy_from_slice(&replacement);
        refresh_checksum(&mut changed);
        assert!(decode_btor2_channel_property_proof_artifact(&changed, artifact_policy).is_err());
    }
    let mut nested_structural_drift = encoded.clone();
    nested_structural_drift[48] ^= 1;
    refresh_checksum(&mut nested_structural_drift);
    assert!(
        decode_btor2_channel_property_proof_artifact(&nested_structural_drift, artifact_policy)
            .is_err()
    );

    for policy in [
        Btor2ChannelPropertyProofPolicy::new(
            queries.len() - 1,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        )
        .unwrap(),
        Btor2ChannelPropertyProofPolicy::new(
            MAX_CHANNEL_PROPERTY_QUERIES,
            artifact.members.len() - 1,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        )
        .unwrap(),
        Btor2ChannelPropertyProofPolicy::new(
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES,
            artifact
                .members
                .iter()
                .map(|member| member.evidence.len())
                .sum::<usize>()
                - 1,
            MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        )
        .unwrap(),
        Btor2ChannelPropertyProofPolicy::new(
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_QUERIES,
            MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            encoded.len() - 1,
        )
        .unwrap(),
    ] {
        assert!(decode_btor2_channel_property_proof_artifact(&encoded, policy).is_err());
    }

    let mut source_drift = model.to_vec();
    source_drift.push(b'\n');
    assert!(
        verify_btor2_channel_property_proof_bytes(
            &source_drift,
            &queries,
            &encoded,
            region_policy,
            artifact_policy,
        )
        .is_err()
    );
    let mut query_drift = queries.clone();
    query_drift[0].horizon = 1;
    assert!(
        verify_btor2_channel_property_proof_bytes(
            model,
            &query_drift,
            &encoded,
            region_policy,
            artifact_policy,
        )
        .is_err()
    );
}

#[test]
fn proof_carrying_bitblast_cross_checks_small_horizons_and_closes_horizon_two() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let policy = Btor2RegionPolicy::default();
    for horizon in 0..=1 {
        for property in [
            Btor2ChannelProperty::OutputHigh,
            Btor2ChannelProperty::OutputLow,
        ] {
            let (property_model, bad) =
                build_btor2_channel_property_model(model, &[9, 39], 6, 0, property, policy)
                    .unwrap();
            let explicit = btor2_search::produce(&property_model, bad, horizon).unwrap();
            let bitblast =
                produce_btor2_bitblast_certificate(&property_model, bad, horizon).unwrap();
            let encoded = encode_btor2_bitblast_certificate(&bitblast).unwrap();
            let decoded = decode_btor2_bitblast_certificate(&encoded).unwrap();
            assert_eq!(decoded, bitblast);
            let summary = verify_btor2_bitblast_certificate(&property_model, &decoded).unwrap();
            assert_eq!(summary.result, explicit.result);
            assert_eq!(summary.bad_frame, explicit.bad_frame);
        }
    }

    let (safe_model, safe_bad) = build_btor2_channel_property_model(
        model,
        &[9, 39],
        6,
        0,
        Btor2ChannelProperty::OutputHigh,
        policy,
    )
    .unwrap();
    assert!(btor2_search::produce(&safe_model, safe_bad, 2).is_err());
    let safe = produce_btor2_bitblast_certificate(&safe_model, safe_bad, 2).unwrap();
    let safe_summary = verify_btor2_bitblast_certificate(&safe_model, &safe).unwrap();
    assert_eq!(safe_summary.result, SearchResult::Unsafe);
    assert_eq!(safe_summary.bad_frame, Some(2));
    assert_eq!(safe_summary.proof_bytes, 0);

    let (unsafe_model, unsafe_bad) = build_btor2_channel_property_model(
        model,
        &[9, 39],
        6,
        0,
        Btor2ChannelProperty::OutputLow,
        policy,
    )
    .unwrap();
    let unsafe_certificate =
        produce_btor2_bitblast_certificate(&unsafe_model, unsafe_bad, 2).unwrap();
    let unsafe_summary =
        verify_btor2_bitblast_certificate(&unsafe_model, &unsafe_certificate).unwrap();
    assert_eq!(unsafe_summary.result, SearchResult::Unsafe);
    assert_eq!(unsafe_summary.bad_frame, Some(0));
}

#[test]
fn bitblast_wire_evidence_rejects_every_mutation_truncation_and_source_drift() {
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let (property_model, bad) = build_btor2_channel_property_model(
        model,
        &[9, 39],
        6,
        0,
        Btor2ChannelProperty::OutputHigh,
        Btor2RegionPolicy::default(),
    )
    .unwrap();
    let certificate = produce_btor2_bitblast_certificate(&property_model, bad, 1).unwrap();
    assert_eq!(certificate.result, SearchResult::Safe);
    let encoded = encode_btor2_bitblast_certificate(&certificate).unwrap();
    for end in [0, 1, 32, encoded.len() / 2, encoded.len() - 1] {
        assert!(decode_btor2_bitblast_certificate(&encoded[..end]).is_err());
    }
    for offset in [0, 8, 44, encoded.len() / 2, encoded.len() - 1] {
        let mut changed = encoded.clone();
        changed[offset] ^= 1;
        assert!(decode_btor2_bitblast_certificate(&changed).is_err());
    }
    let mut source_drift = property_model.clone();
    source_drift.push(b'\n');
    assert!(verify_btor2_bitblast_certificate(&source_drift, &certificate).is_err());

    let unsafe_certificate = produce_btor2_bitblast_certificate(&property_model, bad, 2).unwrap();
    assert_eq!(unsafe_certificate.result, SearchResult::Unsafe);
    let unsafe_encoded = encode_btor2_bitblast_certificate(&unsafe_certificate).unwrap();
    for end in 0..unsafe_encoded.len() {
        assert!(decode_btor2_bitblast_certificate(&unsafe_encoded[..end]).is_err());
    }
    for offset in 0..unsafe_encoded.len() {
        let mut changed = unsafe_encoded.clone();
        changed[offset] ^= 1;
        assert!(decode_btor2_bitblast_certificate(&changed).is_err());
    }
}
