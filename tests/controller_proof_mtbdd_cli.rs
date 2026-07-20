use std::fs;
use std::path::PathBuf;
use std::process::Command;

use guarded_continuation_checker::{
    ControllerPlantResourceRefusalReason, ControllerProofMtbddResourceTool,
    ControllerProofMtbddTool, FailureClass, InvocationStatus, OperationKind, PredicateApiError,
};

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-controller-proof-mtbdd-cli-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("controller.src"), b"tiny controller v1\n").unwrap();
    fs::write(
        root.join("controller.aag"),
        b"aag 2 1 1 1 0\n2\n4 2\n2\ni0 sensor\nl0 state\no0 action\nc\ntiny controller\n",
    )
    .unwrap();
    fs::write(root.join("plant.src"), b"tiny plant v1\n").unwrap();
    fs::write(
        root.join("plant.aag"),
        b"aag 2 1 1 2 0\n2\n4 2\n4\n4\ni0 action\nl0 state\no0 sensor\no1 bad\nc\ntiny plant\n",
    )
    .unwrap();
    fs::write(
        root.join("manifest.txt"),
        b"controller_mtbdd_plant_manifest_version=1\ncontroller_source_path=controller.src\ncontroller_aiger_path=controller.aag\nrelevant_inputs=0\nobserved_outputs=0\nmember_count=2\nplant_source_path=plant.src\nplant_aiger_path=plant.aag\ncontroller_sensor_inputs=0\ncontroller_action_outputs=0\nplant_sensor_outputs=0\nplant_action_inputs=0\ninitial_controller_state=0\ninitial_plant_state=0\nbad_plant_output=1\nhorizon=2\nplant_source_path=plant.src\nplant_aiger_path=plant.aag\ncontroller_sensor_inputs=0\ncontroller_action_outputs=0\nplant_sensor_outputs=0\nplant_action_inputs=0\ninitial_controller_state=0\ninitial_plant_state=1\nbad_plant_output=1\nhorizon=2\nstatus=complete\n",
    )
    .unwrap();
    root
}

#[test]
fn proof_mtbdd_cli_is_versioned_deterministic_and_fail_closed() {
    let discovery = Command::new(BINARY)
        .arg("controller-proof-mtbdd-cli-version")
        .output()
        .unwrap();
    assert!(discovery.status.success());
    assert_eq!(
        String::from_utf8(discovery.stdout).unwrap(),
        "controller_proof_mtbdd_cli_version=1 mtbdd_version=1 equivalence_proof_version=1 plant_artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=16777216 max_equivalence_artifact_bytes=2097152 max_unsat_proof_bytes=1048576 max_members=64 max_state_bits=6 max_inputs=12 max_outputs=8 max_nodes=512 max_terminals=1024 max_horizon=1024 verification=unsat-miter exhaustive_replay=no unsupported=fail-closed\n"
    );

    let root = fixture();
    let manifest = root.join("manifest.txt");
    let artifact = root.join("batch.proof-mtbdd-plant");
    let created = Command::new(BINARY)
        .arg("certify-controller-proof-mtbdd-plant-batch")
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(created.status.success(), "{:?}", created.stderr);
    let created = String::from_utf8(created.stdout).unwrap();
    assert!(created.contains("status=CREATED"));
    assert!(created.contains("members=2 safe=1 unsafe=1"));
    assert!(created.contains("assignments_checked=0"));

    let verified = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-plant-batch")
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(verified.status.success(), "{:?}", verified.stderr);
    let verified = String::from_utf8(verified.stdout).unwrap();
    assert!(verified.contains("status=VERIFIED"));
    assert!(verified.contains("assignments_checked=0"));

    let resource_discovery = Command::new(BINARY)
        .arg("controller-proof-mtbdd-resource-cli-version")
        .output()
        .unwrap();
    assert!(resource_discovery.status.success());
    let resource_discovery = String::from_utf8(resource_discovery.stdout).unwrap();
    assert!(resource_discovery.starts_with(
        "controller_proof_mtbdd_resource_cli_version=1 policy_version=1 envelope_version=1"
    ));
    assert!(
        resource_discovery.contains(
            "verification=unsat-miter exhaustive_replay=no accounting=conservative-static"
        )
    );
    assert!(resource_discovery.ends_with(
        "result_on_refusal=none refusal_schema=proof-reason-v1 unsupported=fail-closed\n"
    ));
    let policy = root.join("proof-resource.policy");
    fs::write(
        &policy,
        b"controller_proof_mtbdd_resource_policy_version=1\nmax_artifact_bytes=16777216\nmax_equivalence_artifact_bytes=2097152\nmax_unsat_proof_bytes=1048576\nmax_members=64\nmax_member_horizon=1024\nmax_product_states_per_member=4096\nmax_transition_evaluations=18446744073709551615\nstatus=complete\n",
    )
    .unwrap();
    let portfolio_discovery = Command::new(BINARY)
        .arg("controller-proof-mtbdd-portfolio-cli-version")
        .output()
        .unwrap();
    assert!(portfolio_discovery.status.success());
    let portfolio_discovery = String::from_utf8(portfolio_discovery.stdout).unwrap();
    assert!(
        portfolio_discovery
            .starts_with("controller_proof_mtbdd_portfolio_cli_version=1 artifact_version=1")
    );
    assert!(portfolio_discovery.ends_with(
        "routing=static fallback=exact proof_failure=fail-closed unsupported=fail-closed\n"
    ));
    let portfolio = root.join("batch.proof-mtbdd-portfolio");
    let portfolio_created = Command::new(BINARY)
        .arg("certify-controller-proof-mtbdd-portfolio")
        .arg(&manifest)
        .arg(&portfolio)
        .output()
        .unwrap();
    assert!(
        portfolio_created.status.success(),
        "{:?}",
        portfolio_created.stderr
    );
    let portfolio_created = String::from_utf8(portfolio_created.stdout).unwrap();
    assert!(portfolio_created.contains("backend=PROOF_MTBDD reason=MTBDD_ADMITTED"));
    assert!(portfolio_created.contains("safe=1 unsafe=1"));
    assert!(portfolio_created.contains("assignments_checked=0"));
    let portfolio_verified = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-portfolio")
        .arg(&manifest)
        .arg(&portfolio)
        .output()
        .unwrap();
    assert!(portfolio_verified.status.success());
    assert!(
        String::from_utf8(portfolio_verified.stdout)
            .unwrap()
            .contains("backend=PROOF_MTBDD reason=MTBDD_ADMITTED")
    );
    let governed_portfolio = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-portfolio-resources")
        .arg(&manifest)
        .arg(&policy)
        .arg(&portfolio)
        .output()
        .unwrap();
    assert!(
        governed_portfolio.status.success(),
        "{:?}",
        governed_portfolio.stderr
    );
    let governed_portfolio = String::from_utf8(governed_portfolio.stdout).unwrap();
    assert!(governed_portfolio.contains("backend=PROOF_MTBDD reason=MTBDD_ADMITTED"));
    assert!(governed_portfolio.contains("assignments_checked=0"));
    let governed = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-plant-resources")
        .arg(&manifest)
        .arg(&policy)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(governed.status.success(), "{:?}", governed.stderr);
    let governed = String::from_utf8(governed.stdout).unwrap();
    assert!(governed.contains("status=VERIFIED"));
    assert!(governed.contains("members=2"));
    assert!(governed.contains("safe=1 unsafe=1"));
    assert!(governed.contains("assignments_checked=0"));

    let tight = root.join("tight-proof-resource.policy");
    fs::write(
        &tight,
        fs::read_to_string(&policy)
            .unwrap()
            .replace("max_unsat_proof_bytes=1048576", "max_unsat_proof_bytes=1"),
    )
    .unwrap();
    let refused = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-plant-resources")
        .arg(&manifest)
        .arg(&tight)
        .arg(&artifact)
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(3));
    assert_eq!(
        String::from_utf8(refused.stderr).unwrap(),
        "error: controller-proof-mtbdd-resource refusal=unsat-proof-bytes result=none\n"
    );
    let refused_portfolio = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-portfolio-resources")
        .arg(&manifest)
        .arg(&tight)
        .arg(&portfolio)
        .output()
        .unwrap();
    assert_eq!(refused_portfolio.status.code(), Some(3));
    assert_eq!(
        String::from_utf8(refused_portfolio.stderr).unwrap(),
        "error: controller-proof-mtbdd-resource refusal=unsat-proof-bytes result=none\n"
    );

    let malformed = root.join("malformed-proof-resource.policy");
    fs::write(
        &malformed,
        fs::read_to_string(&policy)
            .unwrap()
            .replace("max_members=64", "max_members=064"),
    )
    .unwrap();
    let malformed = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-plant-resources")
        .arg(&manifest)
        .arg(&malformed)
        .arg(&artifact)
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(malformed.stderr).unwrap(),
        "error: controller proof MTBDD resource policy members is noncanonical\n"
    );
    let canonical_policy = fs::read_to_string(&policy).unwrap();
    for (name, body) in [
        (
            "crlf-proof-resource.policy",
            canonical_policy.replace('\n', "\r\n"),
        ),
        (
            "trailing-proof-resource.policy",
            canonical_policy.replace("status=complete\n", "status=complete\nextra=1\n"),
        ),
        (
            "missing-proof-resource.policy",
            canonical_policy.replace("status=complete\n", ""),
        ),
        (
            "zero-proof-resource.policy",
            canonical_policy.replace("max_members=64", "max_members=0"),
        ),
        (
            "oversize-proof-resource.policy",
            canonical_policy.replace(
                "max_equivalence_artifact_bytes=2097152",
                "max_equivalence_artifact_bytes=2097153",
            ),
        ),
    ] {
        let path = root.join(name);
        fs::write(&path, body).unwrap();
        assert_eq!(
            Command::new(BINARY)
                .arg("verify-controller-proof-mtbdd-plant-resources")
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
    let nul_policy = root.join("nul-proof-resource.policy");
    let mut nul = canonical_policy.into_bytes();
    nul.insert(nul.len() / 2, 0);
    fs::write(&nul_policy, nul).unwrap();
    assert_eq!(
        Command::new(BINARY)
            .arg("verify-controller-proof-mtbdd-plant-resources")
            .arg(&manifest)
            .arg(&nul_policy)
            .arg(&artifact)
            .status()
            .unwrap()
            .code(),
        Some(2)
    );

    let resource_tool = ControllerProofMtbddResourceTool::discover(BINARY).unwrap();
    assert_eq!(resource_tool.capabilities().cli_version, 1);
    assert_eq!(
        resource_tool.capabilities().max_unsat_proof_bytes,
        1_048_576
    );
    let typed_governed = resource_tool.verify(&manifest, &policy, &artifact).unwrap();
    assert_eq!((typed_governed.safe, typed_governed.unsafe_count), (1, 1));
    assert_eq!(typed_governed.assignments_checked, 0);
    let typed_refusal = resource_tool
        .verify_observed(&manifest, &tight, &artifact)
        .unwrap_err();
    assert!(matches!(
        typed_refusal.error.as_ref(),
        PredicateApiError::ResourceRefused {
            reason: ControllerPlantResourceRefusalReason::UnsatProofBytes
        }
    ));
    assert_eq!(
        typed_refusal.metrics.status,
        InvocationStatus::Failed(FailureClass::ResourceRefusal)
    );

    let tool = ControllerProofMtbddTool::discover(BINARY).unwrap();
    assert_eq!(tool.capabilities().equivalence_proof_version, 1);
    let typed_artifact = root.join("typed.proof-mtbdd-plant");
    let typed = tool.certify_observed(&manifest, &typed_artifact).unwrap();
    assert_eq!(
        typed.metrics.operation,
        OperationKind::CertifyControllerProofMtbddPlantBatch
    );
    assert_eq!(typed.value.assignments_checked, 0);
    assert_eq!((typed.value.safe, typed.value.unsafe_count), (1, 1));
    let typed_verified = tool.verify(&manifest, &typed_artifact).unwrap();
    assert_eq!(typed_verified.members, typed.value.members);
    assert_eq!(typed_verified.assignments_checked, 0);

    let duplicate = root.join("duplicate.proof-mtbdd-plant");
    assert!(
        Command::new(BINARY)
            .arg("certify-controller-proof-mtbdd-plant-batch")
            .arg(&manifest)
            .arg(&duplicate)
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(fs::read(&artifact).unwrap(), fs::read(&duplicate).unwrap());
    assert_eq!(
        Command::new(BINARY)
            .arg("certify-controller-proof-mtbdd-plant-batch")
            .arg(&manifest)
            .arg(&artifact)
            .status()
            .unwrap()
            .code(),
        Some(2)
    );

    let mut mutated = fs::read(&artifact).unwrap();
    let index = mutated.len() / 2;
    mutated[index] ^= 1;
    fs::write(root.join("mutated.proof-mtbdd-plant"), mutated).unwrap();
    assert_eq!(
        Command::new(BINARY)
            .arg("verify-controller-proof-mtbdd-plant-batch")
            .arg(&manifest)
            .arg(root.join("mutated.proof-mtbdd-plant"))
            .status()
            .unwrap()
            .code(),
        Some(2)
    );

    let drift = fs::read_to_string(&manifest)
        .unwrap()
        .replacen("horizon=2", "horizon=1", 1);
    fs::write(root.join("drift.txt"), drift).unwrap();
    assert_eq!(
        Command::new(BINARY)
            .arg("verify-controller-proof-mtbdd-plant-batch")
            .arg(root.join("drift.txt"))
            .arg(&artifact)
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    fs::remove_dir_all(root).unwrap();
}
