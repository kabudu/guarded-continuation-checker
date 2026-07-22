use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_region_extract::{
    Btor2RegionPolicy, decode_btor2_region_artifact, encode_btor2_region_artifact,
    extract_btor2_complete_regions, extract_btor2_repeated_state_regions,
    produce_btor2_region_artifact, verify_btor2_region_artifact,
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
