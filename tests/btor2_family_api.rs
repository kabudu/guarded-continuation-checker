use guarded_continuation_checker::btor2_family::{
    Btor2FamilyInstance, Btor2FamilyPolicy, FamilyInputBinding, compose_btor2_channel_family,
    decode_btor2_family_artifact, encode_btor2_family_artifact, produce_btor2_family_artifact,
    verify_btor2_family_artifact,
};
use sha2::{Digest, Sha256};

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
const PARAMETERS: &[u8] = b"phase_width=1\n";

#[test]
fn downstream_client_can_compose_a_bounded_channel_family() {
    let instances = ["channel0", "channel1"].map(|identifier| Btor2FamilyInstance {
        identifier: identifier.to_string(),
        parameter_sha256: Sha256::digest(PARAMETERS).into(),
        input_bindings: vec![
            FamilyInputBinding::CoreRoot(0),
            FamilyInputBinding::CoreInput(0),
        ],
    });
    let policy = Btor2FamilyPolicy::new(4096, 2, 1, 32, 4096).unwrap();
    let composition =
        compose_btor2_channel_family(CORE, &[3], CHANNEL, &[9], &instances, policy).unwrap();

    assert_eq!(composition.instances, 2);
    assert_eq!(composition.expanded_states, 3);
    assert_eq!(composition.expanded_bad_properties, 2);
    assert_eq!(composition.version, 1);

    let (artifact, produced) =
        produce_btor2_family_artifact(CORE, &[3], CHANNEL, &[9], PARAMETERS, &instances, policy)
            .unwrap();
    let bytes = encode_btor2_family_artifact(&artifact, policy).unwrap();
    let decoded = decode_btor2_family_artifact(&bytes, policy).unwrap();
    let verified =
        verify_btor2_family_artifact(CORE, CHANNEL, PARAMETERS, &decoded, policy).unwrap();
    assert_eq!(produced.expanded_sha256, verified.expanded_sha256);
}
