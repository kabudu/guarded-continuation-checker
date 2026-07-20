use std::fs;
use std::process::Command;

use guarded_continuation_checker::composed_witness::{
    COMPOSED_WITNESS_BASELINE_VERSION, compose_safety_witnesses_v1,
};

const MODEL: &[u8] = b"aag 3 1 1 1 1\n2\n4 6 0\n4\n6 4 2\ni0 sensor\nl0 state\no0 bad\nc\nmodel\n";
const WITNESS: &[u8] =
    b"aag 3 1 1 1 1\n2\n4 6 0\n4\n6 4 2\ni0 = 2\nl0 = 4\no0 invariant\nc\nWITNESS o0 model.aag\n";

#[test]
fn public_api_and_cli_produce_identical_fail_closed_baseline() {
    let expected = compose_safety_witnesses_v1(MODEL, &[WITNESS, WITNESS]).unwrap();
    assert_eq!(COMPOSED_WITNESS_BASELINE_VERSION, 1);

    let scratch = std::env::temp_dir().join(format!(
        "gcc-composed-witness-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&scratch).unwrap();
    let model = scratch.join("model.aag");
    let witness = scratch.join("witness.aag");
    let output = scratch.join("composed.aag");
    fs::write(&model, MODEL).unwrap();
    fs::write(&witness, WITNESS).unwrap();
    let binary = env!("CARGO_BIN_EXE_guarded-continuation-checker");

    let version = Command::new(binary)
        .arg("composed-witness-cli-version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8(version.stdout)
            .unwrap()
            .starts_with("composed_witness_cli_version=1 ")
    );

    let created = Command::new(binary)
        .args([
            "compose-safety-witnesses-v1",
            model.to_str().unwrap(),
            output.to_str().unwrap(),
            witness.to_str().unwrap(),
            witness.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(created.status.success(), "{:?}", created.stderr);
    assert_eq!(fs::read(&output).unwrap(), expected);

    let collision = Command::new(binary)
        .args([
            "compose-safety-witnesses-v1",
            model.to_str().unwrap(),
            output.to_str().unwrap(),
            witness.to_str().unwrap(),
            witness.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!collision.success());

    #[cfg(unix)]
    {
        let witness_link = scratch.join("witness-link.aag");
        let symlink_output = scratch.join("symlink-output.aag");
        std::os::unix::fs::symlink(&witness, &witness_link).unwrap();
        let symlink = Command::new(binary)
            .args([
                "compose-safety-witnesses-v1",
                model.to_str().unwrap(),
                symlink_output.to_str().unwrap(),
                witness_link.to_str().unwrap(),
                witness.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(!symlink.success());
    }
    fs::remove_dir_all(scratch).unwrap();
}
