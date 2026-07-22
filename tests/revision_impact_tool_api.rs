use guarded_continuation_checker::{
    FailureClass, InvocationStatus, OperationKind, RevisionImpactFiles, RevisionImpactTool,
};
use std::fs;
use std::path::PathBuf;

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc revision impact tool api {}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("left-old.btor2"),
        b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n",
    )
    .unwrap();
    fs::write(
        root.join("left-new.btor2"),
        b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 xor 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n",
    )
    .unwrap();
    fs::write(
        root.join("right-old.btor2"),
        b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n",
    )
    .unwrap();
    fs::write(
        root.join("right-new.btor2"),
        b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 xor 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n",
    )
    .unwrap();
    fs::write(
        root.join("interface-old.txt"),
        b"word_interface_version=1\nwire_count=1\nwire=left,7,3\nstatus=complete\n",
    )
    .unwrap();
    fs::write(
        root.join("interface-new.txt"),
        b"word_interface_version=2\nwire_count=1\nwire=left,7,3\nexternal_count=1\nexternal=left,3\nstatus=complete\n",
    )
    .unwrap();
    fs::write(
        root.join("queries.txt"),
        b"gcc-btor2-revision-impact-queries-v1\n0,right,10\n1,right,10\n",
    )
    .unwrap();
    root
}

#[test]
fn typed_revision_impact_tool_discovers_certifies_verifies_and_observes_failures() {
    let root = fixture();
    let discovery = RevisionImpactTool::discover(BINARY).unwrap();
    let capabilities = discovery.capabilities();
    assert_eq!(capabilities.cli_version, 1);
    assert_eq!(capabilities.impact_version, 1);
    assert_eq!(capabilities.query_manifest_version, 1);
    assert_eq!(capabilities.max_atoms, 8);
    assert_eq!(capabilities.max_combinations, 256);
    assert_eq!(capabilities.max_queries, 32);
    assert_eq!(capabilities.max_bundle_bytes, 64 * 1024 * 1024);
    assert_eq!(
        discovery.execution_policy().file_limit_bytes(),
        capabilities.max_bundle_bytes as u64
    );

    let files = RevisionImpactFiles {
        left_old: &root.join("left-old.btor2"),
        left_new: &root.join("left-new.btor2"),
        left_outputs: &[7],
        right_old: &root.join("right-old.btor2"),
        right_new: &root.join("right-new.btor2"),
        right_outputs: &[7, 10],
        interface_old: &root.join("interface-old.txt"),
        interface_new: &root.join("interface-new.txt"),
        queries: &root.join("queries.txt"),
    };
    let artifact = root.join("answer.revision-impact");
    let created = discovery.certify_observed(&files, &artifact).unwrap();
    assert_eq!(
        created.metrics.operation,
        OperationKind::CertifyRevisionImpact
    );
    assert_eq!(created.metrics.status, InvocationStatus::Success);
    assert_eq!(created.value.atoms, 3);
    assert_eq!(created.value.queries, 2);
    assert_eq!(created.value.combinations, 8);
    assert_eq!(created.value.evidence_members, 16);
    assert_eq!(created.value.semantic_replays, 16);
    assert_eq!(created.value.component_validations, 32);
    assert_eq!(created.value.result_comparisons, 16);
    assert!(created.value.parsed_evidence_bytes > 0);
    assert!(created.value.composed_pair_checks > 0);
    assert!(created.value.final_transition_checks > 0);
    assert_eq!(
        created.value.certificate_bytes,
        fs::metadata(&artifact).unwrap().len() as usize
    );

    let verified = discovery.verify_observed(&files, &artifact).unwrap();
    assert_eq!(
        verified.metrics.operation,
        OperationKind::VerifyRevisionImpact
    );
    assert_eq!(verified.metrics.status, InvocationStatus::Success);
    assert_eq!(verified.value.impact_version, created.value.impact_version);
    assert_eq!(verified.value.atoms, created.value.atoms);
    assert_eq!(verified.value.queries, created.value.queries);
    assert_eq!(verified.value.combinations, created.value.combinations);
    assert_eq!(
        verified.value.reusable_observations,
        created.value.reusable_observations
    );
    assert_eq!(
        verified.value.invalidated_observations,
        created.value.invalidated_observations
    );
    assert_eq!(
        verified.value.minimal_invalidating_sets,
        created.value.minimal_invalidating_sets
    );
    assert_eq!(
        verified.value.evidence_members,
        created.value.evidence_members
    );
    assert_eq!(
        verified.value.certificate_bytes,
        created.value.certificate_bytes
    );
    assert_eq!(
        verified.value.parsed_evidence_bytes,
        created.value.parsed_evidence_bytes
    );
    assert_eq!(
        verified.value.semantic_replays,
        created.value.semantic_replays
    );
    assert_eq!(
        verified.value.component_validations,
        created.value.component_validations
    );
    assert_eq!(
        verified.value.composed_pair_checks,
        created.value.composed_pair_checks
    );
    assert_eq!(
        verified.value.final_transition_checks,
        created.value.final_transition_checks
    );
    assert_eq!(
        verified.value.result_comparisons,
        created.value.result_comparisons
    );

    let invalid_files = RevisionImpactFiles {
        left_outputs: &[0],
        ..files.clone()
    };
    let invalid = discovery
        .certify_observed(&invalid_files, &root.join("invalid.revision-impact"))
        .unwrap_err();
    assert_eq!(
        invalid.metrics.operation,
        OperationKind::CertifyRevisionImpact
    );
    assert_eq!(
        invalid.metrics.status,
        InvocationStatus::Failed(FailureClass::Policy)
    );
    assert!(!root.join("invalid.revision-impact").exists());

    fs::write(
        root.join("queries.txt"),
        b"gcc-btor2-revision-impact-queries-v1\n0,right,10\n2,right,10\n",
    )
    .unwrap();
    let drift = discovery.verify_observed(&files, &artifact).unwrap_err();
    assert_eq!(drift.metrics.operation, OperationKind::VerifyRevisionImpact);
    assert_eq!(
        drift.metrics.status,
        InvocationStatus::Failed(FailureClass::ExitStatus)
    );

    fs::remove_dir_all(root).unwrap();
}
