use guarded_continuation_checker::btor2;

#[test]
fn pinned_authentic_pwm_models_preserve_expected_channel_state_growth() {
    let fixtures: [(&[u8], &[u64], usize); 3] = [
        (
            include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-2.btor2"
            ),
            &[5, 17],
            16,
        ),
        (
            include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-4.btor2"
            ),
            &[5, 26],
            26,
        ),
        (
            include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-6.btor2"
            ),
            &[5, 36],
            36,
        ),
    ];

    for (bytes, semantic_roots, expected_states) in fixtures {
        let model = btor2::parse_component_bytes(bytes, semantic_roots).unwrap();
        assert_eq!(model.states().len(), expected_states);
        assert!(model.bad_properties().is_empty());
        assert!(model.constraints().is_empty());
    }
}
