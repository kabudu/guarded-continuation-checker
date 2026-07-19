use guarded_continuation_checker::{
    EventContractResult, EventContractTool, INVOCATION_METRICS_SCHEMA_VERSION, InvocationStatus,
    OperationKind,
};
use std::path::Path;

#[test]
fn downstream_event_contract_api_discovers_certifies_and_runs_portfolio() {
    let tool =
        EventContractTool::discover(env!("CARGO_BIN_EXE_guarded-continuation-checker")).unwrap();
    assert_eq!(tool.capabilities().cli_version, 1);
    assert_eq!(tool.capabilities().certificate_version, 3);
    assert_eq!(tool.capabilities().portfolio_version, 1);
    assert_eq!(tool.capabilities().max_relevant_inputs, 16);

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (index, (model, contract, expected)) in [
        (
            "examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag",
            "examples/event-contracts/interrupt-priority-v1.contract",
            EventContractResult::Avoidable,
        ),
        (
            "examples/products/actuator-controller/firmware/dense-actuator-interlock.aag",
            "examples/event-contracts/actuator-h1-unavoidable-v1.contract",
            EventContractResult::Unavoidable,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let model = root.join(model);
        let contract = root.join(contract);
        let stem = std::env::temp_dir().join(format!(
            "guarded-continuation-downstream-event-api-{}-{index}",
            std::process::id()
        ));
        let certificate = stem.with_extension("cert3");
        let report = stem.with_extension("report");
        assert_eq!(
            tool.certify_v3(&model, 0, &contract, &certificate).unwrap(),
            expected
        );
        let checked = tool
            .verify_v3_observed(&model, &contract, &certificate)
            .unwrap();
        assert_eq!(checked.value, expected);
        assert_eq!(
            checked.metrics.operation,
            OperationKind::VerifyEventContractV3
        );
        assert_eq!(checked.metrics.status, InvocationStatus::Success);
        assert_eq!(
            checked.metrics.schema_version,
            INVOCATION_METRICS_SCHEMA_VERSION
        );
        assert_eq!(checked.metrics.process_group_containment, cfg!(unix));
        assert_eq!(checked.metrics.file_limit_bytes, 32 * 1024 * 1024);
        #[cfg(target_os = "macos")]
        assert_eq!(checked.metrics.memory_limit_bytes, None);
        #[cfg(all(unix, not(target_os = "macos")))]
        assert_eq!(
            checked.metrics.memory_limit_bytes,
            Some(2 * 1024 * 1024 * 1024)
        );
        std::fs::remove_file(&certificate).unwrap();

        let portfolio = tool
            .verify_portfolio_observed(&model, 0, &contract, &report, &certificate)
            .unwrap();
        assert_eq!(portfolio.value, expected);
        assert_eq!(
            portfolio.metrics.operation,
            OperationKind::EventContractPortfolio
        );
        assert_eq!(
            tool.verify_portfolio_report(&model, 0, &contract, &report, &certificate)
                .unwrap(),
            expected
        );
        std::fs::remove_file(report).unwrap();
        std::fs::remove_file(certificate).unwrap();
    }

    let model = root.join("examples/products/infusion-pump/firmware/safe-controller.aag");
    let stem = std::env::temp_dir().join(format!(
        "guarded-continuation-downstream-event-fallback-{}",
        std::process::id()
    ));
    let contract = stem.with_extension("contract");
    let report = stem.with_extension("report");
    let certificate = stem.with_extension("cert3");
    std::fs::write(
        &contract,
        "event_contract_version=1\nhorizon=1\nphase_count=1\nphase_0=0,1\nphase_0_clause_count=0\nterminal_clause_count=0\n",
    )
    .unwrap();
    let fallback = tool
        .verify_portfolio(&model, 0, &contract, &report, &certificate)
        .unwrap();
    assert_eq!(
        tool.verify_portfolio_report(&model, 0, &contract, &report, &certificate)
            .unwrap(),
        fallback
    );
    assert!(!certificate.exists());
    std::fs::remove_file(contract).unwrap();
    std::fs::remove_file(report).unwrap();
}
