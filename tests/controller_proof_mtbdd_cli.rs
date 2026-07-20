use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use guarded_continuation_checker::{
    ControllerPlantPortfolioBackend, ControllerPlantPortfolioReason,
    ControllerPlantResourceRefusalReason, ControllerProofMtbddPortfolioTool,
    ControllerProofMtbddResourceTool, ControllerProofMtbddTool, ControllerSplitEvidenceTool,
    ControllerSplitResourceTool, FailureClass, InvocationStatus, OperationKind, PredicateApiError,
};
use sha2::{Digest, Sha256};

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-controller-proof-mtbdd-cli-{}-{}",
        std::process::id(),
        FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
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

fn direct_fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-controller-proof-portfolio-direct-cli-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("controller.src"), b"seven-latch controller\n").unwrap();
    fs::write(
        root.join("controller.aag"),
        b"aag 8 1 7 1 0\n2\n4 2\n6 2\n8 2\n10 2\n12 2\n14 2\n16 2\n2\nc\nseven-latch controller\n",
    )
    .unwrap();
    for (name, bad) in [("safe", 0), ("unsafe", 1)] {
        fs::write(root.join(format!("{name}.src")), format!("{name} plant\n")).unwrap();
        fs::write(
            root.join(format!("{name}.aag")),
            format!("aag 2 1 1 2 0\n2\n4 2\n4\n{bad}\nc\n{name} plant\n"),
        )
        .unwrap();
    }
    fs::write(
        root.join("manifest.txt"),
        b"controller_mtbdd_plant_manifest_version=1\ncontroller_source_path=controller.src\ncontroller_aiger_path=controller.aag\nrelevant_inputs=0\nobserved_outputs=0\nmember_count=2\nplant_source_path=safe.src\nplant_aiger_path=safe.aag\ncontroller_sensor_inputs=0\ncontroller_action_outputs=0\nplant_sensor_outputs=0\nplant_action_inputs=0\ninitial_controller_state=0\ninitial_plant_state=0\nbad_plant_output=1\nhorizon=4\nplant_source_path=unsafe.src\nplant_aiger_path=unsafe.aag\ncontroller_sensor_inputs=0\ncontroller_action_outputs=0\nplant_sensor_outputs=0\nplant_action_inputs=0\ninitial_controller_state=0\ninitial_plant_state=0\nbad_plant_output=1\nhorizon=4\nstatus=complete\n",
    )
    .unwrap();
    fs::write(
        root.join("policy.txt"),
        b"controller_proof_mtbdd_resource_policy_version=1\nmax_artifact_bytes=16777216\nmax_equivalence_artifact_bytes=1\nmax_unsat_proof_bytes=1\nmax_members=2\nmax_member_horizon=4\nmax_product_states_per_member=256\nmax_transition_evaluations=2560\nstatus=complete\n",
    )
    .unwrap();
    root
}

#[test]
fn split_evidence_cli_admits_once_and_verifies_multiple_batches() {
    let discovery = Command::new(BINARY)
        .arg("controller-split-evidence-cli-version")
        .output()
        .unwrap();
    assert!(discovery.status.success());
    let discovery = String::from_utf8(discovery.stdout).unwrap();
    assert!(discovery.starts_with("controller_split_evidence_cli_version=1"));
    assert!(discovery.contains("admission=once"));
    assert!(discovery.ends_with("unsupported=fail-closed\n"));

    let root = fixture();
    let manifest = root.join("manifest.txt");
    let evidence = root.join("controller.controller-evidence");
    let evidence_second = root.join("controller-second.controller-evidence");
    for output in [&evidence, &evidence_second] {
        let created = Command::new(BINARY)
            .arg("certify-controller-proof-evidence-v1")
            .arg(&manifest)
            .arg(output)
            .output()
            .unwrap();
        assert!(created.status.success(), "{:?}", created.stderr);
        assert!(
            String::from_utf8(created.stdout)
                .unwrap()
                .contains("status=CREATED")
        );
    }
    assert_eq!(
        fs::read(&evidence).unwrap(),
        fs::read(&evidence_second).unwrap()
    );
    let collision = Command::new(BINARY)
        .arg("certify-controller-proof-evidence-v1")
        .arg(&manifest)
        .arg(&evidence)
        .output()
        .unwrap();
    assert!(!collision.status.success());

    let results = root.join("batch.plant-results");
    let results_second = root.join("batch-second.plant-results");
    for output in [&results, &results_second] {
        let created = Command::new(BINARY)
            .arg("certify-bound-plant-results-v1")
            .arg(&manifest)
            .arg(&evidence)
            .arg(output)
            .output()
            .unwrap();
        assert!(created.status.success(), "{:?}", created.stderr);
        let stdout = String::from_utf8(created.stdout).unwrap();
        assert!(stdout.contains("status=CREATED"));
        assert!(stdout.contains("members=2"));
    }
    assert_eq!(
        fs::read(&results).unwrap(),
        fs::read(&results_second).unwrap()
    );

    let verified = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-v1")
        .arg(&evidence)
        .arg(&manifest)
        .arg(&results)
        .arg(&manifest)
        .arg(&results_second)
        .output()
        .unwrap();
    assert!(verified.status.success(), "{:?}", verified.stderr);
    let verified = String::from_utf8(verified.stdout).unwrap();
    assert!(verified.contains("controller-split-batch index=0 status=VERIFIED"));
    assert!(verified.contains("controller-split-batch index=1 status=VERIFIED"));
    assert!(verified.contains(
        "controller-split-set status=VERIFIED cli_version=1 controller_admissions=1 batches=2 members=4 safe=2 unsafe=2"
    ));

    let resource_discovery = Command::new(BINARY)
        .arg("controller-split-resource-cli-version")
        .output()
        .unwrap();
    assert!(resource_discovery.status.success());
    let resource_discovery = String::from_utf8(resource_discovery.stdout).unwrap();
    assert!(resource_discovery.starts_with("controller_split_resource_cli_version=1"));
    assert!(resource_discovery.contains("accounting=conservative-static-per-batch-and-total"));
    assert!(resource_discovery.ends_with("unsupported=fail-closed\n"));

    let evidence_bytes = fs::metadata(&evidence).unwrap().len() as usize;
    let result_bytes = fs::metadata(&results).unwrap().len() as usize;
    let result_second_bytes = fs::metadata(&results_second).unwrap().len() as usize;
    let write_policy = |path: &Path,
                        controller_bytes: usize,
                        max_batches: usize,
                        total_plant_bytes: usize| {
        fs::write(
            path,
            format!(
                "controller_split_resource_policy_version=1\nmax_controller_artifact_bytes={controller_bytes}\nmax_unsat_proof_bytes=1048576\nmax_batches={max_batches}\nmax_plant_artifact_bytes_per_batch={}\nmax_members_per_batch=2\nmax_member_horizon=2\nmax_product_states_per_member=4\nmax_transition_evaluations_per_batch=24\nmax_total_plant_artifact_bytes={total_plant_bytes}\nmax_total_members=4\nmax_total_transition_evaluations=48\nstatus=complete\n",
                result_bytes.max(result_second_bytes),
            ),
        )
        .unwrap();
    };
    let resource_policy = root.join("split-resource-policy.txt");
    write_policy(
        &resource_policy,
        evidence_bytes,
        2,
        result_bytes + result_second_bytes,
    );
    let governed = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-with-resources-v1")
        .arg(&evidence)
        .arg(&resource_policy)
        .arg(&manifest)
        .arg(&results)
        .arg(&manifest)
        .arg(&results_second)
        .output()
        .unwrap();
    assert!(governed.status.success(), "{:?}", governed.stderr);
    let governed = String::from_utf8(governed.stdout).unwrap();
    assert!(governed.contains("controller-split-resource-batch index=0 status=VERIFIED"));
    assert!(governed.contains("controller-split-resource-batch index=1 status=VERIFIED"));
    assert!(governed.contains("controller-split-resource-set status=VERIFIED cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_admissions=1 batches=2 members=4 safe=2 unsafe=2"));
    let governed_value = |key: &str| -> usize {
        governed
            .split_whitespace()
            .find_map(|field| field.strip_prefix(&format!("{key}=")))
            .unwrap()
            .parse()
            .unwrap()
    };
    let unsat_proof_bytes = governed_value("unsat_proof_bytes");

    let resource_tool =
        ControllerSplitResourceTool::discover_observed(BINARY, Default::default()).unwrap();
    assert_eq!(
        resource_tool.metrics.operation,
        OperationKind::DiscoverControllerSplitResource
    );
    assert_eq!(resource_tool.metrics.status, InvocationStatus::Success);
    assert_eq!(resource_tool.value.capabilities().policy_version, 1);
    let typed_governed = resource_tool
        .value
        .verify_set_observed(
            &evidence,
            &resource_policy,
            &[(&manifest, &results), (&manifest, &results_second)],
        )
        .unwrap();
    assert_eq!(
        typed_governed.metrics.operation,
        OperationKind::VerifyBoundPlantResultSetResources
    );
    assert_eq!(typed_governed.metrics.status, InvocationStatus::Success);
    assert_eq!(typed_governed.value.controller_admissions, 1);
    assert_eq!(typed_governed.value.batches.len(), 2);
    assert_eq!(typed_governed.value.members, 4);
    assert_eq!(typed_governed.value.safe, 2);
    assert_eq!(typed_governed.value.unsafe_count, 2);
    assert_eq!(
        typed_governed.value.total_plant_artifact_bytes,
        result_bytes + result_second_bytes
    );
    assert_eq!(typed_governed.value.total_transition_evaluation_bound, 48);

    let tight_controller_policy = root.join("tight-controller-policy.txt");
    write_policy(
        &tight_controller_policy,
        evidence_bytes - 1,
        2,
        result_bytes + result_second_bytes,
    );
    let tight_controller = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-with-resources-v1")
        .arg(&evidence)
        .arg(&tight_controller_policy)
        .arg(&manifest)
        .arg(&results)
        .output()
        .unwrap();
    assert_eq!(tight_controller.status.code(), Some(3));
    assert!(tight_controller.stdout.is_empty());
    assert!(
        String::from_utf8(tight_controller.stderr)
            .unwrap()
            .contains("refusal=controller-artifact-bytes result=none")
    );
    let typed_tight_controller = resource_tool
        .value
        .verify_set_observed(
            &evidence,
            &tight_controller_policy,
            &[(&manifest, &results)],
        )
        .unwrap_err();
    assert!(matches!(
        *typed_tight_controller.error,
        PredicateApiError::ResourceRefused {
            reason: ControllerPlantResourceRefusalReason::ControllerArtifactBytes
        }
    ));
    assert_eq!(
        typed_tight_controller.metrics.status,
        InvocationStatus::Failed(FailureClass::ResourceRefusal)
    );

    let tight_total_policy = root.join("tight-total-policy.txt");
    write_policy(
        &tight_total_policy,
        evidence_bytes,
        2,
        result_bytes + result_second_bytes - 1,
    );
    let tight_total = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-with-resources-v1")
        .arg(&evidence)
        .arg(&tight_total_policy)
        .arg(&manifest)
        .arg(&results)
        .arg(&manifest)
        .arg(&results_second)
        .output()
        .unwrap();
    assert_eq!(tight_total.status.code(), Some(3));
    assert!(tight_total.stdout.is_empty());
    assert!(
        String::from_utf8(tight_total.stderr)
            .unwrap()
            .contains("refusal=total-plant-artifact-bytes result=none")
    );
    let typed_tight_total = resource_tool
        .value
        .verify_set_observed(
            &evidence,
            &tight_total_policy,
            &[(&manifest, &results), (&manifest, &results_second)],
        )
        .unwrap_err();
    assert!(matches!(
        *typed_tight_total.error,
        PredicateApiError::ResourceRefused {
            reason: ControllerPlantResourceRefusalReason::TotalPlantArtifactBytes
        }
    ));

    let base_policy = fs::read_to_string(&resource_policy).unwrap();
    let result_max = result_bytes.max(result_second_bytes);
    let refusal_cases = vec![
        (
            "unsat-proof",
            vec![(
                "max_unsat_proof_bytes=1048576\n".to_string(),
                format!("max_unsat_proof_bytes={}\n", unsat_proof_bytes - 1),
            )],
            "unsat-proof-bytes",
        ),
        (
            "plant-artifact",
            vec![
                (
                    format!("max_plant_artifact_bytes_per_batch={result_max}\n"),
                    format!("max_plant_artifact_bytes_per_batch={}\n", result_max - 1),
                ),
                (
                    format!(
                        "max_total_plant_artifact_bytes={}\n",
                        result_bytes + result_second_bytes
                    ),
                    format!("max_total_plant_artifact_bytes={}\n", (result_max - 1) * 2),
                ),
            ],
            "plant-artifact-bytes",
        ),
        (
            "members-per-batch",
            vec![
                (
                    "max_members_per_batch=2\n".to_string(),
                    "max_members_per_batch=1\n".to_string(),
                ),
                (
                    "max_total_members=4\n".to_string(),
                    "max_total_members=2\n".to_string(),
                ),
            ],
            "members-per-batch",
        ),
        (
            "horizon",
            vec![(
                "max_member_horizon=2\n".to_string(),
                "max_member_horizon=1\n".to_string(),
            )],
            "horizon",
        ),
        (
            "product-states",
            vec![(
                "max_product_states_per_member=4\n".to_string(),
                "max_product_states_per_member=3\n".to_string(),
            )],
            "product-states",
        ),
        (
            "transitions-per-batch",
            vec![
                (
                    "max_transition_evaluations_per_batch=24\n".to_string(),
                    "max_transition_evaluations_per_batch=23\n".to_string(),
                ),
                (
                    "max_total_transition_evaluations=48\n".to_string(),
                    "max_total_transition_evaluations=46\n".to_string(),
                ),
            ],
            "transitions-per-batch",
        ),
        (
            "total-members",
            vec![(
                "max_total_members=4\n".to_string(),
                "max_total_members=3\n".to_string(),
            )],
            "total-members",
        ),
        (
            "total-transitions",
            vec![(
                "max_total_transition_evaluations=48\n".to_string(),
                "max_total_transition_evaluations=47\n".to_string(),
            )],
            "total-transition-evaluations",
        ),
    ];
    for (name, replacements, reason) in refusal_cases {
        let mut body = base_policy.clone();
        for (from, to) in replacements {
            assert!(body.contains(&from), "{name}: missing {from:?}");
            body = body.replace(&from, &to);
        }
        let path = root.join(format!("tight-{name}-policy.txt"));
        fs::write(&path, body).unwrap();
        let refusal = Command::new(BINARY)
            .arg("verify-bound-plant-result-set-with-resources-v1")
            .arg(&evidence)
            .arg(&path)
            .arg(&manifest)
            .arg(&results)
            .arg(&manifest)
            .arg(&results_second)
            .output()
            .unwrap();
        assert_eq!(
            refusal.status.code(),
            Some(3),
            "{name}: {:?}",
            refusal.stderr
        );
        assert!(refusal.stdout.is_empty(), "{name}");
        assert!(
            String::from_utf8(refusal.stderr)
                .unwrap()
                .contains(&format!("refusal={reason} result=none")),
            "{name}"
        );
    }

    let tight_batch_policy = root.join("tight-batch-policy.txt");
    write_policy(
        &tight_batch_policy,
        evidence_bytes,
        1,
        result_bytes.max(result_second_bytes),
    );
    let tight_batch_body = fs::read_to_string(&tight_batch_policy)
        .unwrap()
        .replace("max_total_members=4\n", "max_total_members=2\n")
        .replace(
            "max_total_transition_evaluations=48\n",
            "max_total_transition_evaluations=24\n",
        );
    fs::write(&tight_batch_policy, tight_batch_body).unwrap();
    let tight_batch = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-with-resources-v1")
        .arg(&evidence)
        .arg(&tight_batch_policy)
        .arg(&manifest)
        .arg(&results)
        .arg(&manifest)
        .arg(&results_second)
        .output()
        .unwrap();
    assert_eq!(tight_batch.status.code(), Some(3));
    assert!(tight_batch.stdout.is_empty());
    assert!(
        String::from_utf8(tight_batch.stderr)
            .unwrap()
            .contains("refusal=batches result=none")
    );

    let malformed_policy = root.join("malformed-split-policy.txt");
    fs::write(
        &malformed_policy,
        b"controller_split_resource_policy_version=1\r\n",
    )
    .unwrap();
    let malformed = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-with-resources-v1")
        .arg(&evidence)
        .arg(&malformed_policy)
        .arg(&manifest)
        .arg(&results)
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    assert!(malformed.stdout.is_empty());

    let mut corrupt = fs::read(&results).unwrap();
    let middle = corrupt.len() / 2;
    corrupt[middle] ^= 1;
    let corrupt_path = root.join("corrupt.plant-results");
    fs::write(&corrupt_path, corrupt).unwrap();
    let rejected = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-v1")
        .arg(&evidence)
        .arg(&manifest)
        .arg(&corrupt_path)
        .output()
        .unwrap();
    assert!(!rejected.status.success());

    let incomplete = Command::new(BINARY)
        .arg("verify-bound-plant-result-set-v1")
        .arg(&evidence)
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(!incomplete.status.success());

    let discovery =
        ControllerSplitEvidenceTool::discover_observed(BINARY, Default::default()).unwrap();
    assert_eq!(
        discovery.metrics.operation,
        OperationKind::DiscoverControllerSplitEvidence
    );
    assert_eq!(discovery.metrics.status, InvocationStatus::Success);
    assert_eq!(discovery.value.capabilities().cli_version, 1);
    assert_eq!(
        discovery.value.capabilities().controller_artifact_version,
        1
    );
    assert_eq!(discovery.value.capabilities().plant_artifact_version, 1);

    let typed_evidence = root.join("typed.controller-evidence");
    let typed_evidence_summary = discovery
        .value
        .certify_controller_evidence_observed(&manifest, &typed_evidence)
        .unwrap();
    assert_eq!(
        typed_evidence_summary.metrics.operation,
        OperationKind::CertifyControllerProofEvidence
    );
    assert_eq!(
        typed_evidence_summary.metrics.status,
        InvocationStatus::Success
    );
    assert_eq!(typed_evidence_summary.value.artifact_version, 1);
    assert_eq!(typed_evidence_summary.value.members, None);
    assert_eq!(typed_evidence_summary.value.mtbdd_nodes, Some(1));
    assert_eq!(typed_evidence_summary.value.mtbdd_terminals, Some(2));
    assert_eq!(
        typed_evidence_summary.value.artifact_bytes,
        fs::metadata(&typed_evidence).unwrap().len() as usize
    );

    let typed_results = root.join("typed.plant-results");
    let typed_results_second = root.join("typed-second.plant-results");
    for output in [&typed_results, &typed_results_second] {
        let summary = discovery
            .value
            .certify_plant_results(&manifest, &typed_evidence, output)
            .unwrap();
        assert_eq!(summary.artifact_version, 1);
        assert_eq!(summary.members, Some(2));
        assert_eq!(summary.mtbdd_nodes, None);
        assert_eq!(summary.mtbdd_terminals, None);
        assert_eq!(
            summary.artifact_bytes,
            fs::metadata(output).unwrap().len() as usize
        );
    }

    let typed_verified = discovery
        .value
        .verify_set_observed(
            &typed_evidence,
            &[
                (&manifest, &typed_results),
                (&manifest, &typed_results_second),
            ],
        )
        .unwrap();
    assert_eq!(
        typed_verified.metrics.operation,
        OperationKind::VerifyBoundPlantResultSet
    );
    assert_eq!(typed_verified.metrics.status, InvocationStatus::Success);
    assert_eq!(typed_verified.value.controller_admissions, 1);
    assert_eq!(typed_verified.value.batches.len(), 2);
    assert_eq!(typed_verified.value.members, 4);
    assert_eq!(typed_verified.value.safe, 2);
    assert_eq!(typed_verified.value.unsafe_count, 2);

    let empty = discovery
        .value
        .verify_set_observed(&typed_evidence, &[])
        .unwrap_err();
    assert!(matches!(*empty.error, PredicateApiError::InvalidPolicy(_)));
    assert_eq!(
        empty.metrics.status,
        InvocationStatus::Failed(FailureClass::Policy)
    );
}

#[cfg(unix)]
#[test]
fn typed_split_evidence_client_rejects_overflowing_helper_totals() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "gcc-hostile-split-client-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("hostile-helper");
    let maximum = usize::MAX;
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\ncontroller-split-evidence-cli-version)\nprintf '%s\\n' 'controller_split_evidence_cli_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=16777216 max_batches=64 admission=once verification=unsat-miter exhaustive_replay=no source_binding=sha256 obligation_binding=complete-ordered unsupported=fail-closed'\n;;\ncontroller-split-resource-cli-version)\nprintf '%s\\n' 'controller_split_resource_cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_policy_bytes=4096 max_controller_artifact_bytes=16777216 max_unsat_proof_bytes=1048576 max_plant_artifact_bytes=16777216 max_batches=64 max_members_per_batch=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 admission=once verification=unsat-miter exhaustive_replay=no accounting=conservative-static-per-batch-and-total timing_calibration=none result_on_refusal=none refusal_schema=split-reason-v1 unsupported=fail-closed'\n;;\nverify-bound-plant-result-set-v1)\nprintf '%s\\n' 'controller-split-batch index=0 status=VERIFIED members={maximum} safe={maximum} unsafe={maximum} reachable_product_states={maximum} explored_transitions={maximum} artifact_bytes=1 verification_micros=1'\nprintf '%s\\n' 'controller-split-set status=VERIFIED cli_version=1 controller_admissions=1 batches=1 members={maximum} safe={maximum} unsafe={maximum} reachable_product_states={maximum} explored_transitions={maximum} controller_evidence_bytes=1 admission_micros=1 elapsed_micros=1'\n;;\nverify-bound-plant-result-set-with-resources-v1)\nprintf '%s\\n' 'controller-split-resource-batch index=0 status=VERIFIED policy_version=1 envelope_version=1 artifact_version=1 members={maximum} maximum_member_horizon=1 maximum_product_states=1 transition_evaluation_bound={maximum} safe={maximum} unsafe={maximum} reachable_product_states={maximum} explored_transitions={maximum} artifact_bytes=1 verification_micros=1'\nprintf '%s\\n' 'controller-split-resource-set status=VERIFIED cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_admissions=1 batches=1 members={maximum} safe={maximum} unsafe={maximum} reachable_product_states={maximum} explored_transitions={maximum} controller_evidence_bytes=10 controller_mtbdd_bytes=1 equivalence_artifact_bytes=1 unsat_proof_bytes=1 total_plant_artifact_bytes=1 total_transition_evaluation_bound={maximum} admission_micros=1 elapsed_micros=1'\n;;\n*) exit 2;;\nesac\n"
    );
    fs::write(&executable, script).unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();

    let tool = ControllerSplitEvidenceTool::discover(&executable).unwrap();
    let failure = tool
        .verify_set_observed(
            Path::new("evidence"),
            &[(Path::new("manifest"), Path::new("results"))],
        )
        .unwrap_err();
    assert!(matches!(
        *failure.error,
        PredicateApiError::InvalidResponse(_)
    ));
    assert_eq!(
        failure.metrics.status,
        InvocationStatus::Failed(FailureClass::Response)
    );

    let resource_tool = ControllerSplitResourceTool::discover(&executable).unwrap();
    let resource_failure = resource_tool
        .verify_set_observed(
            Path::new("evidence"),
            Path::new("policy"),
            &[(Path::new("manifest"), Path::new("results"))],
        )
        .unwrap_err();
    assert!(matches!(
        *resource_failure.error,
        PredicateApiError::InvalidResponse(_)
    ));
    assert_eq!(
        resource_failure.metrics.status,
        InvocationStatus::Failed(FailureClass::Response)
    );

    fs::remove_dir_all(root).unwrap();
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
    assert!(portfolio_discovery.starts_with(
        "controller_proof_mtbdd_portfolio_cli_version=1 policy_version=1 envelope_version=1 artifact_version=1"
    ));
    assert!(portfolio_discovery.ends_with(
        "routing=static fallback=exact proof_failure=fail-closed attested_verification=required accounting=conservative-static timing_calibration=none result_on_refusal=none refusal_schema=proof-reason-v1 unsupported=fail-closed\n"
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
    fs::write(root.join("controller.ys"), b"read controller\n").unwrap();
    fs::write(root.join("plant.ys"), b"read plant\n").unwrap();
    let revision = "0123456789abcdef0123456789abcdef01234567";
    fs::write(
        root.join("provenance.txt"),
        format!(
            "source_model_provenance_manifest_version=1\ntool=yosys\ntool_revision={revision}\nmember_count=2\nworkdir=.\nsource_path=controller.src\nrecipe_path=controller.ys\nmodel_path=controller.aag\nworkdir=.\nsource_path=plant.src\nrecipe_path=plant.ys\nmodel_path=plant.aag\nstatus=complete\n"
        ),
    )
    .unwrap();
    let attested_subjects = [
        (
            fs::read(root.join("controller.src")).unwrap(),
            fs::read(root.join("controller.ys")).unwrap(),
            fs::read(root.join("controller.aag")).unwrap(),
        ),
        (
            fs::read(root.join("plant.src")).unwrap(),
            fs::read(root.join("plant.ys")).unwrap(),
            fs::read(root.join("plant.aag")).unwrap(),
        ),
    ];
    let mut evidence = "schema_version,member,tool,tool_revision,source_sha256,recipe_sha256,model_sha256,regenerated_sha256,byte_match,status\n".to_string();
    for (member, (source, recipe, model)) in attested_subjects.iter().enumerate() {
        evidence.push_str(&format!(
            "1,{member},yosys,{revision},{},{},{},{},true,attested\n",
            sha256(source),
            sha256(recipe),
            sha256(model),
            sha256(model),
        ));
    }
    fs::write(root.join("attestation.csv"), &evidence).unwrap();
    let attested = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-portfolio-resources-attested")
        .arg(&manifest)
        .arg(&policy)
        .arg(&portfolio)
        .arg(root.join("provenance.txt"))
        .arg(root.join("attestation.csv"))
        .output()
        .unwrap();
    assert!(attested.status.success(), "{:?}", attested.stderr);
    let attested = String::from_utf8(attested.stdout).unwrap();
    assert!(attested.contains("provenance=BOUND"));
    assert!(attested.contains("source_model_members=2"));
    assert!(attested.contains("safe=1 unsafe=1"));

    fs::write(root.join("controller.src"), b"substituted controller\n").unwrap();
    let refused_attestation = Command::new(BINARY)
        .arg("verify-controller-proof-mtbdd-portfolio-resources-attested")
        .arg(&manifest)
        .arg(&policy)
        .arg(&portfolio)
        .arg(root.join("provenance.txt"))
        .arg(root.join("attestation.csv"))
        .output()
        .unwrap();
    assert!(!refused_attestation.status.success());
    assert_eq!(refused_attestation.status.code(), Some(2));
    let refusal = String::from_utf8(refused_attestation.stderr).unwrap();
    assert!(refusal.contains("source digest does not match"));
    assert!(!refusal.contains(" SAFE"));
    assert!(!refusal.contains(" UNSAFE"));
    fs::write(root.join("controller.src"), b"tiny controller v1\n").unwrap();
    let portfolio_tool = ControllerProofMtbddPortfolioTool::discover(BINARY).unwrap();
    assert_eq!(portfolio_tool.capabilities().cli_version, 1);
    assert_eq!(portfolio_tool.capabilities().refusal_exit_code, 3);
    let typed_portfolio = portfolio_tool
        .verify(&manifest, &policy, &portfolio)
        .unwrap();
    assert_eq!(
        typed_portfolio.backend,
        ControllerPlantPortfolioBackend::Mtbdd
    );
    assert_eq!(
        typed_portfolio.reason,
        ControllerPlantPortfolioReason::MtbddAdmitted
    );
    assert_eq!((typed_portfolio.safe, typed_portfolio.unsafe_count), (1, 1));
    assert_eq!(typed_portfolio.assignments_checked, 0);
    let typed_attested = portfolio_tool
        .verify_attested(
            &manifest,
            &policy,
            &portfolio,
            &root.join("provenance.txt"),
            &root.join("attestation.csv"),
        )
        .unwrap();
    assert_eq!((typed_attested.safe, typed_attested.unsafe_count), (1, 1));
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
    let typed_portfolio_refusal = portfolio_tool
        .verify_observed(&manifest, &tight, &portfolio)
        .unwrap_err();
    assert!(matches!(
        typed_portfolio_refusal.error.as_ref(),
        PredicateApiError::ResourceRefused {
            reason: ControllerPlantResourceRefusalReason::UnsatProofBytes
        }
    ));
    assert_eq!(
        typed_portfolio_refusal.metrics.operation,
        OperationKind::VerifyControllerProofMtbddPortfolioResources
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

#[test]
fn proof_mtbdd_portfolio_typed_client_accepts_exact_static_fallback() {
    let root = direct_fixture();
    let manifest = root.join("manifest.txt");
    let policy = root.join("policy.txt");
    let artifact = root.join("direct.proof-mtbdd-portfolio");
    let created = Command::new(BINARY)
        .arg("certify-controller-proof-mtbdd-portfolio")
        .arg(&manifest)
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(created.status.success(), "{:?}", created.stderr);
    let created = String::from_utf8(created.stdout).unwrap();
    assert!(created.contains("backend=DIRECT_EXACT reason=BOUNDARY_LIMIT"));
    assert!(created.contains("safe=1 unsafe=1"));

    let tool = ControllerProofMtbddPortfolioTool::discover(BINARY).unwrap();
    let governed = tool.verify(&manifest, &policy, &artifact).unwrap();
    assert_eq!(
        governed.backend,
        ControllerPlantPortfolioBackend::DirectExact
    );
    assert_eq!(
        governed.reason,
        ControllerPlantPortfolioReason::BoundaryLimit
    );
    assert_eq!(governed.equivalence_artifact_bytes, 0);
    assert_eq!(governed.unsat_proof_bytes, 0);
    assert_eq!((governed.safe, governed.unsafe_count), (1, 1));
    assert_eq!(governed.transition_evaluation_bound, 2560);
    assert_eq!(governed.assignments_checked, 0);
    fs::remove_dir_all(root).unwrap();
}
