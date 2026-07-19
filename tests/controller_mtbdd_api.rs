use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_mtbdd::{
    decode_controller_mtbdd, encode_controller_mtbdd, evaluate_controller_mtbdd,
    produce_controller_mtbdd, verify_controller_mtbdd,
};
use sha2::{Digest, Sha256};

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");

#[test]
fn public_controller_has_a_source_bound_exact_mtbdd() {
    let model = parse_ascii_aiger_transition(MODEL).unwrap();
    let digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let artifact =
        produce_controller_mtbdd(&model, digest, &(1..12).collect::<Vec<_>>(), &[2, 6, 7, 9])
            .unwrap();
    let summary = verify_controller_mtbdd(&model, digest, &artifact).unwrap();
    assert_eq!(summary.nodes, 254);
    assert_eq!(summary.assignments_checked, 131_072);
    let encoded = encode_controller_mtbdd(&artifact).unwrap();
    assert_eq!(encode_controller_mtbdd(&artifact).unwrap(), encoded);
    assert_eq!(decode_controller_mtbdd(&encoded).unwrap(), artifact);
    assert_eq!(
        evaluate_controller_mtbdd(&artifact, 0, 0).unwrap().target,
        0
    );

    for length in 0..encoded.len() {
        assert!(decode_controller_mtbdd(&encoded[..length]).is_err());
    }
    for index in 0..encoded.len() {
        let mut mutated = encoded.clone();
        mutated[index] ^= 1;
        assert!(decode_controller_mtbdd(&mutated).is_err());
    }
    let mut wrong_boundary = artifact.clone();
    wrong_boundary.observed_outputs[3] = 10;
    let wrong_encoded = encode_controller_mtbdd(&wrong_boundary).unwrap();
    let wrong_decoded = decode_controller_mtbdd(&wrong_encoded).unwrap();
    assert!(verify_controller_mtbdd(&model, digest, &wrong_decoded).is_err());
}
