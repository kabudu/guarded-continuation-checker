use std::fs;
use std::path::PathBuf;
use std::process::Command;

use guarded_continuation_checker::{ControllerProofMtbddTool, OperationKind};

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
