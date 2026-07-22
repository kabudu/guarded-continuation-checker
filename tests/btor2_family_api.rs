use guarded_continuation_checker::btor2_family::{
    Btor2FamilyInstance, Btor2FamilyPolicy, FamilyInputBinding, compose_btor2_channel_family,
};

const CORE: &[u8] = br#"1 sort bitvec 1
2 input 1 enable
3 state 1 phase
4 zero 1
5 init 1 3 4
6 xor 1 3 2
7 next 1 3 6
8 output 3 phase_out
"#;

const CHANNEL: &[u8] = br#"1 sort bitvec 1
2 input 1 phase
3 input 1 enable
4 state 1 pulse
5 zero 1
6 init 1 4 5
7 and 1 2 3
8 next 1 4 7
9 xor 1 4 2
10 output 9 mismatch
"#;

#[test]
fn downstream_client_can_compose_a_bounded_channel_family() {
    let instances = ["channel0", "channel1"].map(|identifier| Btor2FamilyInstance {
        identifier: identifier.to_string(),
        parameter_sha256: [0x31; 32],
        input_bindings: vec![
            FamilyInputBinding::CoreRoot(0),
            FamilyInputBinding::CoreInput(0),
        ],
    });
    let policy = Btor2FamilyPolicy::new(2, 1, 32, 4096).unwrap();
    let composition =
        compose_btor2_channel_family(CORE, &[3], CHANNEL, &[9], &instances, policy).unwrap();

    assert_eq!(composition.instances, 2);
    assert_eq!(composition.expanded_states, 3);
    assert_eq!(composition.expanded_bad_properties, 2);
    assert_eq!(composition.version, 1);
}
