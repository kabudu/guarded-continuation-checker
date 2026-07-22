use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc revision impact cli {} {}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
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

fn invoke(root: &Path, command: &str, queries: &Path, artifact: &Path) -> Output {
    Command::new(BINARY)
        .arg(command)
        .arg(root.join("left-old.btor2"))
        .arg(root.join("left-new.btor2"))
        .arg("7")
        .arg(root.join("right-old.btor2"))
        .arg(root.join("right-new.btor2"))
        .arg("7,10")
        .arg(root.join("interface-old.txt"))
        .arg(root.join("interface-new.txt"))
        .arg(queries)
        .arg(artifact)
        .output()
        .unwrap()
}

#[test]
fn revision_impact_cli_is_self_service_exact_and_fail_closed() {
    let capabilities = Command::new(BINARY)
        .arg("btor2-revision-impact-cli-version")
        .output()
        .unwrap();
    assert!(capabilities.status.success());
    assert_eq!(
        String::from_utf8(capabilities.stdout).unwrap(),
        "revision_impact_cli_version=1 impact_version=1 query_manifest_version=1 max_query_manifest_bytes=16384 max_input_bytes=67108864 max_evidence_bytes=16777216 max_bundle_bytes=67108864 max_atoms=8 max_combinations=256 max_queries=32 semantics=exact-counterfactual-v1 work_schema=verification-v1 query_schema=transition-v1 routing=none fallback=none unsupported=fail-closed\n"
    );
    assert_eq!(
        Command::new(BINARY)
            .args(["btor2-revision-impact-cli-version", "unexpected"])
            .status()
            .unwrap()
            .code(),
        Some(2)
    );

    let root = fixture();
    let queries = root.join("queries.txt");
    let artifact = root.join("answer.revision-impact");

    let created = invoke(&root, "check-btor2-revision-impact", &queries, &artifact);
    assert!(created.status.success(), "{:?}", created.stderr);
    let stdout = String::from_utf8(created.stdout).unwrap();
    assert!(stdout.starts_with("btor2-revision-impact status=CREATED impact_version=1 "));
    assert!(stdout.contains("atoms=3 queries=2 combinations=8"));
    assert!(stdout.contains("evidence_members=16 certificate_bytes="));
    let transitions = stdout
        .lines()
        .filter(|line| line.starts_with("btor2-revision-impact-query "))
        .collect::<Vec<_>>();
    assert_eq!(transitions.len(), 2);
    assert!(transitions[0].starts_with(
        "btor2-revision-impact-query index=0 horizon=0 bad_side=right bad_output=10 "
    ));
    assert!(transitions[1].starts_with(
        "btor2-revision-impact-query index=1 horizon=1 bad_side=right bad_output=10 "
    ));
    assert!(fs::read(&artifact).unwrap().starts_with(b"GCCRIB01"));

    let verified = invoke(&root, "verify-btor2-revision-impact", &queries, &artifact);
    assert!(verified.status.success(), "{:?}", verified.stderr);
    assert!(
        String::from_utf8(verified.stdout)
            .unwrap()
            .starts_with("btor2-revision-impact status=VERIFIED impact_version=1 ")
    );

    let no_clobber = invoke(&root, "check-btor2-revision-impact", &queries, &artifact);
    assert_eq!(no_clobber.status.code(), Some(2));

    fs::write(
        root.join("drifted-queries.txt"),
        b"gcc-btor2-revision-impact-queries-v1\n0,right,10\n2,right,10\n",
    )
    .unwrap();
    let drift = invoke(
        &root,
        "verify-btor2-revision-impact",
        &root.join("drifted-queries.txt"),
        &artifact,
    );
    assert_eq!(drift.status.code(), Some(2));

    let right_new = fs::read(root.join("right-new.btor2")).unwrap();
    fs::write(
        root.join("right-new.btor2"),
        String::from_utf8(right_new.clone())
            .unwrap()
            .replace("reached_two", "reached_two_drift"),
    )
    .unwrap();
    let source_drift = invoke(&root, "verify-btor2-revision-impact", &queries, &artifact);
    assert_eq!(source_drift.status.code(), Some(2));
    fs::write(root.join("right-new.btor2"), right_new).unwrap();

    let mut tampered = fs::read(&artifact).unwrap();
    let middle = tampered.len() / 2;
    tampered[middle] ^= 1;
    fs::write(root.join("tampered.revision-impact"), tampered).unwrap();
    let tamper = invoke(
        &root,
        "verify-btor2-revision-impact",
        &queries,
        &root.join("tampered.revision-impact"),
    );
    assert_eq!(tamper.status.code(), Some(2));

    fs::write(
        root.join("crlf-queries.txt"),
        b"gcc-btor2-revision-impact-queries-v1\r\n0,right,10\r\n",
    )
    .unwrap();
    let rejected_artifact = root.join("rejected.revision-impact");
    let crlf = invoke(
        &root,
        "check-btor2-revision-impact",
        &root.join("crlf-queries.txt"),
        &rejected_artifact,
    );
    assert_eq!(crlf.status.code(), Some(2));
    assert!(!rejected_artifact.exists());

    fs::write(
        root.join("oversized-queries.txt"),
        vec![b'a'; 16 * 1024 + 1],
    )
    .unwrap();
    let oversized_artifact = root.join("oversized.revision-impact");
    let oversized = invoke(
        &root,
        "check-btor2-revision-impact",
        &root.join("oversized-queries.txt"),
        &oversized_artifact,
    );
    assert_eq!(oversized.status.code(), Some(2));
    assert!(
        String::from_utf8(oversized.stderr)
            .unwrap()
            .contains("exceeds 16384 bytes")
    );
    assert!(!oversized_artifact.exists());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&queries, root.join("queries-link.txt")).unwrap();
        let symlink_artifact = root.join("symlink.revision-impact");
        let symlink = invoke(
            &root,
            "check-btor2-revision-impact",
            &root.join("queries-link.txt"),
            &symlink_artifact,
        );
        assert_eq!(symlink.status.code(), Some(2));
        assert!(!symlink_artifact.exists());
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn revision_impact_cli_admits_the_query_limit_and_refuses_one_beyond_it() {
    let root = fixture();
    fs::copy(root.join("left-old.btor2"), root.join("left-new.btor2")).unwrap();
    fs::copy(
        root.join("interface-old.txt"),
        root.join("interface-new.txt"),
    )
    .unwrap();
    let manifest = |count: usize| {
        let mut text = "gcc-btor2-revision-impact-queries-v1\n".to_string();
        for horizon in 0..count {
            text.push_str(&format!("{horizon},right,10\n"));
        }
        text
    };
    fs::write(root.join("queries-32.txt"), manifest(32)).unwrap();
    let boundary_artifact = root.join("boundary.revision-impact");
    let boundary = invoke(
        &root,
        "check-btor2-revision-impact",
        &root.join("queries-32.txt"),
        &boundary_artifact,
    );
    assert!(boundary.status.success(), "{:?}", boundary.stderr);
    let boundary_stdout = String::from_utf8(boundary.stdout).unwrap();
    assert!(boundary_stdout.contains("atoms=1 queries=32 combinations=2"));
    assert!(boundary_stdout.contains("evidence_members=64"));
    assert!(boundary_stdout.contains("semantic_replays=64 component_validations=128"));
    assert_eq!(
        boundary_stdout
            .lines()
            .filter(|line| line.starts_with("btor2-revision-impact-query "))
            .count(),
        32
    );

    fs::write(root.join("queries-33.txt"), manifest(33)).unwrap();
    let refused_artifact = root.join("refused.revision-impact");
    let refused = invoke(
        &root,
        "check-btor2-revision-impact",
        &root.join("queries-33.txt"),
        &refused_artifact,
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(
        String::from_utf8(refused.stderr)
            .unwrap()
            .contains("1..=32 queries")
    );
    assert!(!refused_artifact.exists());

    fs::remove_dir_all(root).unwrap();
}
