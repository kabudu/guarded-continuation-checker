use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_transducer::produce_controller_transducer;
use sha2::{Digest, Sha256};

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn pinned_public_controller_remains_source_exact_and_statically_rejected() {
    assert_eq!(
        digest(SOURCE),
        "edf0cf5a5b6371b945731fe941f43bf4ffec84b9916fe1a18830f1bcadd398b7"
    );
    assert_eq!(
        digest(MODEL),
        "f0278433e03ce0ef0774a376b17ec18fcfaadea5abf6a0bfb784b3726db06b65"
    );
    let model = parse_ascii_aiger_transition(MODEL).unwrap();
    assert_eq!(
        (
            model.inputs.len(),
            model.latches.len(),
            model.outputs.len(),
            model.ands.len(),
        ),
        (12, 6, 14, 122)
    );
    let source_digest: [u8; 32] = Sha256::digest(SOURCE).into();
    let error = produce_controller_transducer(
        &model,
        source_digest,
        &(1..12).collect::<Vec<_>>(),
        &[2, 6, 7, 9],
    )
    .unwrap_err();
    assert_eq!(error.0, "controller transducer cell count exceeds limit");
}
