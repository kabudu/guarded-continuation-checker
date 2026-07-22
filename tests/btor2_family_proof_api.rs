use guarded_continuation_checker::btor2_family::{Btor2FamilyInstance, FamilyInputBinding};
use guarded_continuation_checker::btor2_family_proof::{
    Btor2FamilyProofInput, Btor2FamilyProofPolicy, Btor2FamilyQuery, decode_btor2_family_proof,
    encode_btor2_family_proof, produce_btor2_family_proof, verify_btor2_family_proof,
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
10 output 5 safe
11 output 9 mismatch
"#;
const PARAMETERS: &[u8] = b"width=1\n";

#[test]
fn downstream_client_can_exchange_and_verify_both_answer_family_evidence() {
    let instances = ["channel0", "channel1"]
        .into_iter()
        .map(|identifier| Btor2FamilyInstance {
            identifier: identifier.to_string(),
            parameter_sha256: Sha256::digest(PARAMETERS).into(),
            input_bindings: vec![
                FamilyInputBinding::CoreRoot(0),
                FamilyInputBinding::CoreInput(0),
            ],
        })
        .collect::<Vec<_>>();
    let queries = [
        Btor2FamilyQuery {
            property_index: 0,
            horizon: 2,
        },
        Btor2FamilyQuery {
            property_index: 1,
            horizon: 2,
        },
    ];
    let policy = Btor2FamilyProofPolicy::default();
    let (artifact, _) = produce_btor2_family_proof(
        Btor2FamilyProofInput {
            core_bytes: CORE,
            core_roots: &[3],
            channel_bytes: CHANNEL,
            channel_roots: &[5, 9],
            parameter_bytes: PARAMETERS,
            instances: &instances,
            queries: &queries,
        },
        policy,
    )
    .unwrap();
    let bytes = encode_btor2_family_proof(&artifact, policy).unwrap();
    let decoded = decode_btor2_family_proof(&bytes, policy).unwrap();
    let summary = verify_btor2_family_proof(CORE, CHANNEL, PARAMETERS, &decoded, policy).unwrap();

    assert_eq!(summary.queries, 2);
    assert_eq!(summary.safe, 1);
    assert_eq!(summary.unsafe_count, 1);
}
