use guarded_continuation_checker::{
    ControllerMtbddAnswer, ControllerMtbddTool, InvocationStatus, OperationKind,
};
use std::fs;
use std::path::PathBuf;

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc controller mtbdd api {} {:?}",
        std::process::id(),
        std::thread::current().id()
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
fn typed_controller_mtbdd_api_discovers_certifies_and_verifies() {
    let discovery = ControllerMtbddTool::discover_observed(BINARY, Default::default()).unwrap();
    assert_eq!(
        discovery.metrics.operation,
        OperationKind::DiscoverControllerMtbdd
    );
    assert_eq!(discovery.metrics.status, InvocationStatus::Success);
    assert_eq!(discovery.value.capabilities().cli_version, 1);
    assert_eq!(discovery.value.capabilities().max_outputs, 8);

    let root = fixture();
    let manifest = root.join("manifest.txt");
    let artifact = root.join("batch.mtbdd-plant");
    let created = discovery
        .value
        .certify_observed(&manifest, &artifact)
        .unwrap();
    assert_eq!(
        created.metrics.operation,
        OperationKind::CertifyControllerMtbddPlantBatch
    );
    assert_eq!(created.metrics.status, InvocationStatus::Success);
    assert_eq!((created.value.safe, created.value.unsafe_count), (1, 1));
    assert_eq!(created.value.members.len(), 2);
    assert_eq!(created.value.members[0].answer, ControllerMtbddAnswer::Safe);
    assert_eq!(created.value.members[0].bad_frame, None);
    assert_eq!(
        created.value.members[1].answer,
        ControllerMtbddAnswer::Unsafe
    );
    assert_eq!(created.value.members[1].bad_frame, Some(0));
    assert_eq!(
        created.value.artifact_bytes,
        fs::metadata(&artifact).unwrap().len() as usize
    );

    let verified = discovery
        .value
        .verify_observed(&manifest, &artifact)
        .unwrap();
    assert_eq!(
        verified.metrics.operation,
        OperationKind::VerifyControllerMtbddPlantBatch
    );
    assert_eq!(verified.metrics.status, InvocationStatus::Success);
    assert_eq!(verified.value.members, created.value.members);
    assert_eq!(
        verified.value.assignments_checked,
        created.value.assignments_checked
    );

    fs::write(root.join("plant.src"), b"drifted plant\n").unwrap();
    let drift = discovery
        .value
        .verify_observed(&manifest, &artifact)
        .unwrap_err();
    assert_eq!(
        drift.metrics.operation,
        OperationKind::VerifyControllerMtbddPlantBatch
    );
    assert!(matches!(drift.metrics.status, InvocationStatus::Failed(_)));

    fs::remove_dir_all(root).unwrap();
}
