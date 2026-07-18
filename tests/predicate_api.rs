use continuation_quotient_sat::{CertificateVersion, PredicateResult, PredicateTool};
use std::path::Path;

#[test]
fn downstream_api_discovers_certifies_and_verifies_both_formats() {
    let tool = PredicateTool::discover(env!("CARGO_BIN_EXE_continuation-quotient-sat")).unwrap();
    assert_eq!(tool.capabilities().cli_version, 1);
    assert_eq!(tool.capabilities().max_relevant_inputs, 16);

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model =
        root.join("examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag");
    let transcript =
        root.join("examples/predicate-certificate-cost/interrupt-h8-avoidable.transcript");
    for (version, label) in [
        (CertificateVersion::V1, "v1"),
        (CertificateVersion::V2, "v2"),
    ] {
        let certificate = std::env::temp_dir().join(format!(
            "cq-sat-downstream-api-{}-{label}.cert",
            std::process::id()
        ));
        assert_eq!(
            tool.certify(version, &model, 0, &transcript, &certificate)
                .unwrap(),
            PredicateResult::Avoidable
        );
        assert_eq!(
            tool.verify(version, &model, &certificate).unwrap(),
            PredicateResult::Avoidable
        );
        std::fs::remove_file(certificate).unwrap();
    }
}
