use guarded_continuation_checker::aiger_obligation::{AigerLatch, AigerTransition};
use guarded_continuation_checker::controller_transducer::{
    decode_controller_transducer, encode_controller_transducer, produce_controller_transducer,
    verify_controller_transducer,
};

#[test]
fn downstream_api_produces_and_verifies_source_bound_exact_cells() {
    let controller = AigerTransition {
        max_variable: 2,
        inputs: vec![2],
        latches: vec![AigerLatch {
            current: 4,
            next: 2,
        }],
        outputs: vec![2],
        ands: vec![],
    };
    let source_sha256 = [0x42; 32];
    let obligation = produce_controller_transducer(&controller, source_sha256, &[0], &[0]).unwrap();
    let encoded = encode_controller_transducer(&obligation).unwrap();
    let decoded = decode_controller_transducer(&encoded).unwrap();
    let summary = verify_controller_transducer(&controller, source_sha256, &decoded).unwrap();

    assert_eq!(summary.cells, 2);
    assert_eq!(summary.rows, 4);
    assert!(summary.proof_bytes > 0);
    assert!(verify_controller_transducer(&controller, [0x41; 32], &obligation).is_err());
}
