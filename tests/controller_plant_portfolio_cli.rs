use std::fs;
use std::path::PathBuf;
use std::process::Command;

use guarded_continuation_checker::{
    ControllerPlantPortfolioBackend, ControllerPlantPortfolioReason, ControllerPlantPortfolioTool,
    ControllerPlantResourceTool, InvocationStatus, OperationKind,
};

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-controller-plant-portfolio-cli-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("controller.src"), b"nine-action controller\n").unwrap();
    fs::write(
        root.join("controller.aag"),
        b"aag 2 1 1 9 0\n2\n4 2\n2\n2\n2\n2\n2\n2\n2\n2\n2\nc\nnine-action controller\n",
    )
    .unwrap();
    for (name, bad) in [("safe", 0), ("unsafe", 1)] {
        fs::write(root.join(format!("{name}.src")), format!("{name} plant\n")).unwrap();
        fs::write(
            root.join(format!("{name}.aag")),
            format!(
                "aag 10 9 1 2 0\n2\n4\n6\n8\n10\n12\n14\n16\n18\n20 0\n0\n{bad}\nc\n{name} plant\n"
            ),
        )
        .unwrap();
    }
    let actions = "0,1,2,3,4,5,6,7,8";
    fs::write(
        root.join("manifest.txt"),
        format!(
            "controller_mtbdd_plant_manifest_version=1\ncontroller_source_path=controller.src\ncontroller_aiger_path=controller.aag\nrelevant_inputs=0\nobserved_outputs={actions}\nmember_count=2\nplant_source_path=safe.src\nplant_aiger_path=safe.aag\ncontroller_sensor_inputs=0\ncontroller_action_outputs={actions}\nplant_sensor_outputs=0\nplant_action_inputs={actions}\ninitial_controller_state=0\ninitial_plant_state=0\nbad_plant_output=1\nhorizon=4\nplant_source_path=unsafe.src\nplant_aiger_path=unsafe.aag\ncontroller_sensor_inputs=0\ncontroller_action_outputs={actions}\nplant_sensor_outputs=0\nplant_action_inputs={actions}\ninitial_controller_state=0\ninitial_plant_state=0\nbad_plant_output=1\nhorizon=4\nstatus=complete\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("resource-policy.txt"),
        b"controller_plant_resource_policy_version=1\nmax_artifact_bytes=16777216\nmax_members=2\nmax_member_horizon=4\nmax_product_states_per_member=4\nmax_transition_evaluations=40\nstatus=complete\n",
    )
    .unwrap();
    root
}

#[test]
fn portfolio_cli_routes_to_exact_fallback_and_rejects_drift() {
    let discovery = Command::new(BINARY)
        .arg("controller-plant-portfolio-cli-version")
        .output()
        .unwrap();
    assert!(discovery.status.success());
    assert_eq!(
        String::from_utf8(discovery.stdout).unwrap(),
        "controller_plant_portfolio_cli_version=1 artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=16777216 max_members=64 backends=mtbdd,direct-exact routing=static fallback=exact unsupported=fail-closed\n"
    );
    let resource_discovery = Command::new(BINARY)
        .arg("controller-plant-resource-cli-version")
        .output()
        .unwrap();
    assert!(resource_discovery.status.success());
    assert_eq!(
        String::from_utf8(resource_discovery.stdout).unwrap(),
        "controller_plant_resource_cli_version=1 policy_version=1 envelope_version=1 manifest_version=1 portfolio_artifact_version=1 max_policy_bytes=4096 max_artifact_bytes=16777216 max_members=64 max_horizon=1024 max_product_states=4096 accounting=conservative-static timing_calibration=none result_on_refusal=none unsupported=fail-closed\n"
    );

    let root = fixture();
    let manifest = root.join("manifest.txt");
    let artifact = root.join("batch.controller-plant");
    let created = Command::new(BINARY)
        .arg("certify-controller-plant-portfolio")
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(created.status.success(), "{:?}", created.stderr);
    let created = String::from_utf8(created.stdout).unwrap();
    assert!(created.contains("backend=DIRECT_EXACT reason=boundary-limit"));
    assert!(created.contains("members=2 safe=1 unsafe=1"));
    assert!(created.contains("index=1 answer=UNSAFE horizon=4 bad_frame=0"));

    let verified = Command::new(BINARY)
        .arg("verify-controller-plant-portfolio")
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(verified.status.success(), "{:?}", verified.stderr);
    assert!(
        String::from_utf8(verified.stdout)
            .unwrap()
            .contains("status=VERIFIED cli_version=1 artifact_version=1 backend=DIRECT_EXACT")
    );

    let governed = Command::new(BINARY)
        .arg("verify-controller-plant-portfolio-resources")
        .arg(&manifest)
        .arg(root.join("resource-policy.txt"))
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(governed.status.success(), "{:?}", governed.stderr);
    let governed = String::from_utf8(governed.stdout).unwrap();
    assert!(governed.contains(
        "status=VERIFIED cli_version=1 policy_version=1 envelope_version=1 artifact_version=1 backend=DIRECT_EXACT members=2 maximum_member_horizon=4 maximum_product_states=4 transition_evaluation_bound=40 safe=1 unsafe=1"
    ));
    assert!(
        governed.contains(
            "controller-plant-resource-member index=1 answer=UNSAFE horizon=4 bad_frame=0"
        )
    );

    let resource_tool = ControllerPlantResourceTool::discover(BINARY).unwrap();
    assert_eq!(resource_tool.capabilities().max_product_states, 4096);
    let typed_governed = resource_tool
        .verify_observed(&manifest, &root.join("resource-policy.txt"), &artifact)
        .unwrap();
    assert_eq!(
        typed_governed.metrics.operation,
        OperationKind::VerifyControllerPlantPortfolioResources
    );
    assert_eq!(typed_governed.metrics.status, InvocationStatus::Success);
    assert_eq!(
        typed_governed.value.backend,
        ControllerPlantPortfolioBackend::DirectExact
    );
    assert_eq!(typed_governed.value.transition_evaluation_bound, 40);
    assert_eq!(
        (typed_governed.value.safe, typed_governed.value.unsafe_count),
        (1, 1)
    );
    assert_eq!(typed_governed.value.member_results[1].bad_frame, Some(0));

    let policy = fs::read_to_string(root.join("resource-policy.txt")).unwrap();
    for (name, body) in [
        (
            "tight-policy.txt",
            policy.replace(
                "max_transition_evaluations=40",
                "max_transition_evaluations=39",
            ),
        ),
        ("crlf-policy.txt", policy.replace('\n', "\r\n")),
        (
            "trailing-policy.txt",
            policy.replace("status=complete\n", "status=complete\nextra=1\n"),
        ),
        (
            "noncanonical-policy.txt",
            policy.replace("max_members=2", "max_members=02"),
        ),
    ] {
        let path = root.join(name);
        fs::write(&path, body).unwrap();
        assert_eq!(
            Command::new(BINARY)
                .arg("verify-controller-plant-portfolio-resources")
                .arg(&manifest)
                .arg(&path)
                .arg(&artifact)
                .status()
                .unwrap()
                .code(),
            Some(2),
            "hostile policy {name} was accepted"
        );
    }

    let tool = ControllerPlantPortfolioTool::discover(BINARY).unwrap();
    assert_eq!(tool.capabilities().max_members, 64);
    let typed_artifact = root.join("typed.controller-plant");
    let typed = tool.certify_observed(&manifest, &typed_artifact).unwrap();
    assert_eq!(
        typed.metrics.operation,
        OperationKind::CertifyControllerPlantPortfolio
    );
    assert_eq!(typed.metrics.status, InvocationStatus::Success);
    assert_eq!(
        typed.value.backend,
        ControllerPlantPortfolioBackend::DirectExact
    );
    assert_eq!(
        typed.value.reason,
        ControllerPlantPortfolioReason::BoundaryLimit
    );
    assert_eq!((typed.value.safe, typed.value.unsafe_count), (1, 1));
    let typed_verified = tool.verify(&manifest, &typed_artifact).unwrap();
    assert_eq!(typed_verified.members, typed.value.members);

    let mut mutated = fs::read(&artifact).unwrap();
    let index = mutated.len() / 2;
    mutated[index] ^= 1;
    fs::write(root.join("mutated.controller-plant"), mutated).unwrap();
    assert_eq!(
        Command::new(BINARY)
            .arg("verify-controller-plant-portfolio")
            .arg(&manifest)
            .arg(root.join("mutated.controller-plant"))
            .status()
            .unwrap()
            .code(),
        Some(2)
    );

    fs::write(
        root.join("drift.txt"),
        fs::read_to_string(&manifest)
            .unwrap()
            .replacen("horizon=4", "horizon=3", 1),
    )
    .unwrap();
    assert_eq!(
        Command::new(BINARY)
            .arg("verify-controller-plant-portfolio")
            .arg(root.join("drift.txt"))
            .arg(&artifact)
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    assert_eq!(
        Command::new(BINARY)
            .arg("certify-controller-plant-portfolio")
            .arg(&manifest)
            .arg(&artifact)
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    fs::remove_dir_all(root).unwrap();
}
