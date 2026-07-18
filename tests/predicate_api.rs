use continuation_quotient_sat::{
    CertificateVersion, INVOCATION_METRICS_SCHEMA_VERSION, InvocationStatus, OperationKind,
    PredicateResult, PredicateTool,
};
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
        let observed = tool.verify_observed(version, &model, &certificate).unwrap();
        assert_eq!(observed.value, PredicateResult::Avoidable);
        assert_eq!(
            observed.metrics.schema_version,
            INVOCATION_METRICS_SCHEMA_VERSION
        );
        assert_eq!(observed.metrics.status, InvocationStatus::Success);
        assert!(observed.metrics.stdout_bytes > 0);
        assert_eq!(observed.metrics.process_group_containment, cfg!(unix));
        assert_eq!(observed.metrics.file_limit_bytes, 32 * 1024 * 1024);
        #[cfg(target_os = "macos")]
        assert_eq!(observed.metrics.memory_limit_bytes, None);
        #[cfg(all(unix, not(target_os = "macos")))]
        assert_eq!(
            observed.metrics.memory_limit_bytes,
            Some(2 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            observed.metrics.operation,
            match version {
                CertificateVersion::V1 => OperationKind::VerifyV1,
                CertificateVersion::V2 => OperationKind::VerifyV2,
            }
        );
        std::fs::remove_file(certificate).unwrap();
    }
}
