use guarded_continuation_checker::btor2_family::{Btor2FamilyInstance, FamilyInputBinding};
use guarded_continuation_checker::btor2_family_orbit::{
    Btor2FamilyOrbitInput, decode_btor2_family_orbit_proof, encode_btor2_family_orbit_proof,
    produce_btor2_family_orbit_proof, verify_btor2_family_orbit_proof,
};
use guarded_continuation_checker::btor2_family_proof::Btor2FamilyProofPolicy;
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
10 output 5 safe
11 output 9 mismatch
"#;
const PARAMETERS: &[u8] = b"width=1\n";

#[test]
fn downstream_client_can_verify_representative_orbit_evidence() {
    let instances = (0..4)
        .map(|index| Btor2FamilyInstance {
            identifier: format!("channel{index}"),
            parameter_sha256: Sha256::digest(PARAMETERS).into(),
            input_bindings: vec![
                FamilyInputBinding::CoreRoot(0),
                FamilyInputBinding::CoreInput(0),
            ],
        })
        .collect::<Vec<_>>();
    let policy = Btor2FamilyProofPolicy::default();
    let artifact = produce_btor2_family_orbit_proof(
        Btor2FamilyOrbitInput {
            core_bytes: CORE,
            core_roots: &[3],
            channel_bytes: CHANNEL,
            channel_roots: &[5, 9],
            parameter_bytes: PARAMETERS,
            instances: &instances,
            root_horizons: &[2, 2],
        },
        policy,
    )
    .unwrap();
    let bytes = encode_btor2_family_orbit_proof(&artifact, policy).unwrap();
    let decoded = decode_btor2_family_orbit_proof(&bytes, policy).unwrap();
    let summary =
        verify_btor2_family_orbit_proof(CORE, CHANNEL, PARAMETERS, &decoded, policy).unwrap();

    assert_eq!(summary.representative_queries, 2);
    assert_eq!(summary.logical_queries, 8);
    assert_eq!(summary.logical_safe, 4);
    assert_eq!(summary.logical_unsafe, 4);
}
