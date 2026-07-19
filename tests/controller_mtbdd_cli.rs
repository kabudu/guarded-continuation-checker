use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");
fn fixture() -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("gcc-controller-mtbdd-cli-{}", std::process::id()));
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
fn controller_mtbdd_cli_is_self_service_bound_and_fail_closed() {
    let discovery = Command::new(BINARY)
        .arg("controller-mtbdd-cli-version")
        .output()
        .unwrap();
    assert!(discovery.status.success());
    let discovery = String::from_utf8(discovery.stdout).unwrap();
    assert!(discovery.starts_with("controller_mtbdd_cli_version=1 "));
    assert!(discovery.contains(" manifest_version=1 "));
    assert!(discovery.contains(" max_outputs=8 "));
    assert!(discovery.ends_with(" unsupported=fail-closed\n"));

    let root = fixture();
    let manifest = root.join("manifest.txt");
    let artifact = root.join("batch.mtbdd-plant");
    let created = Command::new(BINARY)
        .args(["certify-controller-mtbdd-plant-batch"])
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(created.status.success(), "{:?}", created.stderr);
    let created = String::from_utf8(created.stdout).unwrap();
    assert!(created.contains("status=CREATED"));
    assert!(created.contains("members=2 safe=1 unsafe=1"));
    assert!(created.contains("artifact_bytes="));
    let member_lines = created
        .lines()
        .filter(|line| line.starts_with("controller-mtbdd-plant-member "))
        .collect::<Vec<_>>();
    assert_eq!(member_lines.len(), 2);
    assert!(member_lines[0].contains("index=0 answer=SAFE horizon=2 bad_frame=none "));
    assert!(member_lines[1].contains("index=1 answer=UNSAFE horizon=2 bad_frame=0 "));

    let verified = Command::new(BINARY)
        .args(["verify-controller-mtbdd-plant-batch"])
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(verified.status.success(), "{:?}", verified.stderr);
    let verified = String::from_utf8(verified.stdout).unwrap();
    assert!(verified.contains("status=VERIFIED"));
    assert!(verified.contains("members=2 safe=1 unsafe=1"));

    let mismatched_manifest = fs::read_to_string(&manifest).unwrap().replacen(
        "bad_plant_output=1",
        "bad_plant_output=0",
        1,
    );
    fs::write(root.join("mismatched.txt"), mismatched_manifest).unwrap();
    let mismatch = Command::new(BINARY)
        .args(["verify-controller-mtbdd-plant-batch"])
        .arg(root.join("mismatched.txt"))
        .arg(&artifact)
        .status()
        .unwrap();
    assert_eq!(mismatch.code(), Some(2));

    let no_clobber = Command::new(BINARY)
        .args(["certify-controller-mtbdd-plant-batch"])
        .arg(&manifest)
        .arg(&artifact)
        .status()
        .unwrap();
    assert_eq!(no_clobber.code(), Some(2));

    let mut mutated = fs::read(&artifact).unwrap();
    let mutation_index = mutated.len() / 2;
    mutated[mutation_index] ^= 1;
    fs::write(root.join("mutated.mtbdd-plant"), mutated).unwrap();
    let rejected = Command::new(BINARY)
        .args(["verify-controller-mtbdd-plant-batch"])
        .arg(&manifest)
        .arg(root.join("mutated.mtbdd-plant"))
        .status()
        .unwrap();
    assert_eq!(rejected.code(), Some(2));

    fs::write(
        root.join("plant.src"),
        [fs::read(root.join("plant.src")).unwrap(), b"\n".to_vec()].concat(),
    )
    .unwrap();
    let drift = Command::new(BINARY)
        .args(["verify-controller-mtbdd-plant-batch"])
        .arg(&manifest)
        .arg(&artifact)
        .status()
        .unwrap();
    assert_eq!(drift.code(), Some(2));

    let hostile =
        fs::read_to_string(&manifest)
            .unwrap()
            .replacen("controller.src", "../controller.src", 1);
    fs::write(root.join("hostile.txt"), hostile).unwrap();
    let traversal = Command::new(BINARY)
        .args(["verify-controller-mtbdd-plant-batch"])
        .arg(root.join("hostile.txt"))
        .arg(&artifact)
        .status()
        .unwrap();
    assert_eq!(traversal.code(), Some(2));

    let crlf = fs::read_to_string(&manifest).unwrap().replace('\n', "\r\n");
    fs::write(root.join("crlf.txt"), crlf).unwrap();
    let noncanonical = Command::new(BINARY)
        .args(["verify-controller-mtbdd-plant-batch"])
        .arg(root.join("crlf.txt"))
        .arg(&artifact)
        .status()
        .unwrap();
    assert_eq!(noncanonical.code(), Some(2));

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&manifest, root.join("manifest-link.txt")).unwrap();
        let symlink = Command::new(BINARY)
            .args(["verify-controller-mtbdd-plant-batch"])
            .arg(root.join("manifest-link.txt"))
            .arg(&artifact)
            .status()
            .unwrap();
        assert_eq!(symlink.code(), Some(2));

        fs::rename(root.join("plant.src"), root.join("real-plant.src")).unwrap();
        std::os::unix::fs::symlink(root.join("real-plant.src"), root.join("plant.src")).unwrap();
        let component_symlink = Command::new(BINARY)
            .args(["verify-controller-mtbdd-plant-batch"])
            .arg(&manifest)
            .arg(&artifact)
            .output()
            .unwrap();
        assert_eq!(component_symlink.status.code(), Some(2));
        assert!(
            String::from_utf8(component_symlink.stderr)
                .unwrap()
                .contains("path must not contain symlinks")
        );
    }

    fs::remove_dir_all(root).unwrap();
}
