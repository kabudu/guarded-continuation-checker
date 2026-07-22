use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_region_equivalence::{
    decode_btor2_reachable_region_equivalence_artifact, derive_btor2_reachable_region_equivalence,
    derive_btor2_region_equivalence, encode_btor2_reachable_region_equivalence_artifact,
    produce_btor2_reachable_region_equivalence_artifact,
    verify_btor2_reachable_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::{
    Btor2RegionPolicy, decode_btor2_complete_region_artifact, decode_btor2_region_artifact,
    encode_btor2_complete_region_artifact, encode_btor2_region_artifact,
    extract_btor2_complete_regions, extract_btor2_repeated_state_regions,
    produce_btor2_complete_region_artifact, produce_btor2_region_artifact,
    verify_btor2_complete_region_artifact, verify_btor2_region_artifact,
};

#[test]
fn pinned_authentic_pwm_models_preserve_expected_channel_state_growth() {
    struct Fixture {
        bytes: &'static [u8],
        semantic_roots: &'static [u64],
        expected_states: usize,
        channels: usize,
        expected_local_states: &'static [usize],
    }
    let fixtures = [
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-2.btor2"
            ),
            semantic_roots: &[5, 17],
            expected_states: 16,
            channels: 2,
            expected_local_states: &[2, 6],
        },
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-4.btor2"
            ),
            semantic_roots: &[5, 26],
            expected_states: 26,
            channels: 4,
            expected_local_states: &[2, 6, 2, 6],
        },
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-6.btor2"
            ),
            semantic_roots: &[5, 36],
            expected_states: 36,
            channels: 6,
            expected_local_states: &[2, 6, 2, 6, 2, 6],
        },
    ];

    for fixture in fixtures {
        let Fixture {
            bytes,
            semantic_roots,
            expected_states,
            channels,
            expected_local_states,
        } = fixture;
        let model = btor2::parse_component_bytes(bytes, semantic_roots).unwrap();
        assert_eq!(model.states().len(), expected_states);
        assert!(model.bad_properties().is_empty());
        assert!(model.constraints().is_empty());
        let policy = Btor2RegionPolicy::default();
        let regions = extract_btor2_repeated_state_regions(
            bytes,
            semantic_roots,
            channels,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        assert_eq!(regions.total_states, expected_states);
        assert_eq!(
            regions
                .channels
                .iter()
                .map(|region| region.states.len())
                .collect::<Vec<_>>(),
            expected_local_states
        );
        assert_eq!(
            regions.shared_states.len(),
            expected_states - expected_local_states.iter().sum::<usize>()
        );
        let complete =
            extract_btor2_complete_regions(bytes, semantic_roots, channels, policy).unwrap();
        assert_eq!(complete.state_regions, regions);
        assert_eq!(complete.channel_nodes.len(), channels);
        assert!(complete.channel_nodes.iter().all(|nodes| !nodes.is_empty()));
        assert!(!complete.shared_to_channel_edges.is_empty());
        assert!(!complete.channel_to_aggregate_edges.is_empty());
        assert!(
            complete
                .shared_to_channel_edges
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        let equivalence =
            derive_btor2_region_equivalence(bytes, semantic_roots, channels, policy).unwrap();
        assert_eq!(
            equivalence.classes,
            (0..channels)
                .map(|channel| vec![channel])
                .collect::<Vec<_>>()
        );
        let reachable =
            derive_btor2_reachable_region_equivalence(bytes, semantic_roots, channels, 63, policy)
                .unwrap();
        let expected_classes = match channels {
            2 => vec![vec![0], vec![1]],
            4 => vec![vec![0], vec![1], vec![2], vec![3]],
            6 => vec![vec![0], vec![1], vec![2, 4], vec![3, 5]],
            _ => unreachable!(),
        };
        assert_eq!(reachable.classes, expected_classes);
        let reachable_artifact = produce_btor2_reachable_region_equivalence_artifact(
            bytes,
            semantic_roots,
            channels,
            63,
            policy,
        )
        .unwrap();
        let reachable_bytes =
            encode_btor2_reachable_region_equivalence_artifact(&reachable_artifact).unwrap();
        let reachable_decoded =
            decode_btor2_reachable_region_equivalence_artifact(&reachable_bytes).unwrap();
        assert_eq!(
            verify_btor2_reachable_region_equivalence_artifact(bytes, &reachable_decoded, policy,)
                .unwrap(),
            reachable
        );
        assert_eq!(
            reachable_bytes,
            encode_btor2_reachable_region_equivalence_artifact(
                &produce_btor2_reachable_region_equivalence_artifact(
                    bytes,
                    semantic_roots,
                    channels,
                    63,
                    policy,
                )
                .unwrap(),
            )
            .unwrap()
        );
        assert!(
            complete
                .channel_to_aggregate_edges
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        let complete_artifact =
            produce_btor2_complete_region_artifact(bytes, semantic_roots, channels, policy)
                .unwrap();
        let complete_bytes =
            encode_btor2_complete_region_artifact(&complete_artifact, policy).unwrap();
        let complete_decoded =
            decode_btor2_complete_region_artifact(&complete_bytes, policy).unwrap();
        assert_eq!(
            verify_btor2_complete_region_artifact(bytes, &complete_decoded, policy).unwrap(),
            complete
        );
        let artifact =
            produce_btor2_region_artifact(bytes, semantic_roots, channels, policy).unwrap();
        let encoded = encode_btor2_region_artifact(&artifact, policy).unwrap();
        let decoded = decode_btor2_region_artifact(&encoded, policy).unwrap();
        assert_eq!(
            verify_btor2_region_artifact(bytes, &decoded, policy).unwrap(),
            regions
        );
        assert_eq!(
            encoded,
            encode_btor2_region_artifact(
                &produce_btor2_region_artifact(bytes, semantic_roots, channels, policy).unwrap(),
                policy,
            )
            .unwrap()
        );
    }
}

#[test]
fn reachable_equivalence_artifact_fails_closed_under_hostile_changes() {
    let bytes =
        include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-6.btor2");
    let policy = Btor2RegionPolicy::default();
    let artifact =
        produce_btor2_reachable_region_equivalence_artifact(bytes, &[5, 36], 6, 63, policy)
            .unwrap();
    let encoded = encode_btor2_reachable_region_equivalence_artifact(&artifact).unwrap();

    for end in 0..encoded.len() {
        assert!(decode_btor2_reachable_region_equivalence_artifact(&encoded[..end]).is_err());
    }
    for offset in 0..encoded.len() {
        let mut changed = encoded.clone();
        changed[offset] ^= 1;
        assert!(decode_btor2_reachable_region_equivalence_artifact(&changed).is_err());
    }

    let mut source_drift = bytes.to_vec();
    source_drift.push(b'\n');
    assert!(
        verify_btor2_reachable_region_equivalence_artifact(&source_drift, &artifact, policy)
            .is_err()
    );
    let mut class_drift = artifact.clone();
    class_drift.summary.classes = (0..6).map(|channel| vec![channel]).collect();
    assert!(
        verify_btor2_reachable_region_equivalence_artifact(bytes, &class_drift, policy).is_err()
    );
    let mut digest_drift = artifact.clone();
    digest_drift.summary.signatures[0].sha256[0] ^= 1;
    assert!(
        verify_btor2_reachable_region_equivalence_artifact(bytes, &digest_drift, policy).is_err()
    );
}
