use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn query_manifest(horizon: u32) -> String {
    let mut text =
        String::from("gcc-btor2-channel-properties-v1\nchannels=6\nsemantic_roots=9,39\n");
    let mut query_id = 0;
    for property in ["output-high", "output-low"] {
        for channel in 0..6 {
            text.push_str(&format!(
                "query={query_id},{channel},{property},{horizon}\n"
            ));
            query_id += 1;
        }
    }
    text.push_str("status=complete\n");
    text
}

fn policy_text(max_projected_work: u64) -> String {
    format!(
        "channel_property_policy_version=1\nmax_queries=4096\nmax_members=4096\nmax_evidence_bytes=67108864\nmax_artifact_bytes=69206016\nmax_projected_work={max_projected_work}\nstatus=complete\n"
    )
}

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-channel-property-cli-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("model.btor2"),
        include_bytes!(
            "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
        ),
    )
    .unwrap();
    fs::write(root.join("queries.txt"), query_manifest(2)).unwrap();
    fs::write(root.join("policy.txt"), policy_text(6_189_840)).unwrap();
    root
}

fn invoke(root: &Path, command: &str, queries: &Path, policy: &Path, artifact: &Path) -> Output {
    Command::new(BINARY)
        .arg(command)
        .arg(root.join("model.btor2"))
        .arg(queries)
        .arg(policy)
        .arg(artifact)
        .output()
        .unwrap()
}

#[test]
fn channel_property_cli_is_versioned_self_service_and_fail_closed() {
    let capabilities = Command::new(BINARY)
        .arg("btor2-channel-property-cli-version")
        .output()
        .unwrap();
    assert!(capabilities.status.success());
    assert_eq!(
        String::from_utf8(capabilities.stdout).unwrap(),
        "btor2_channel_property_cli_version=1 artifact_version=1 query_manifest_version=1 policy_version=1 max_query_manifest_bytes=262144 max_policy_bytes=4096 max_model_bytes=8388608 max_channels=64 max_queries=4096 max_evidence_bytes=67108864 max_artifact_bytes=69206016 max_projected_work=100000000000 routing=static-explicit-or-bitblast fallback=exact unsupported=fail-closed verification=source-replay\n"
    );

    let root = fixture();
    let queries = root.join("queries.txt");
    let policy = root.join("policy.txt");
    let artifact = root.join("result.channel-properties");
    let created = invoke(
        &root,
        "certify-btor2-channel-properties",
        &queries,
        &policy,
        &artifact,
    );
    assert!(created.status.success(), "{:?}", created.stderr);
    let stdout = String::from_utf8(created.stdout).unwrap();
    assert!(stdout.starts_with(
        "btor2-channel-properties status=CREATED cli_version=1 artifact_version=1 channels=6 logical_queries=12 proof_members=6 reused_queries=6 explicit_members=0 bitblast_members=6 evidence_bytes=750 artifact_bytes=1568 projected_work=6189840 "
    ));
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("btor2-channel-property index="))
            .count(),
        12
    );
    assert!(fs::read(&artifact).unwrap().starts_with(b"GCCBCP01"));

    let verified = invoke(
        &root,
        "verify-btor2-channel-properties",
        &queries,
        &policy,
        &artifact,
    );
    assert!(verified.status.success(), "{:?}", verified.stderr);
    assert!(String::from_utf8(verified.stdout).unwrap().starts_with(
        "btor2-channel-properties status=VERIFIED cli_version=1 artifact_version=1 channels=6 logical_queries=12 proof_members=6 reused_queries=6 explicit_members=0 bitblast_members=6 evidence_bytes=750 artifact_bytes=1568 projected_work=not-applied "
    ));

    let collision = invoke(
        &root,
        "certify-btor2-channel-properties",
        &queries,
        &policy,
        &artifact,
    );
    assert_eq!(collision.status.code(), Some(2));

    fs::write(root.join("tight-policy.txt"), policy_text(6_189_839)).unwrap();
    let refused_artifact = root.join("refused.channel-properties");
    let refused = invoke(
        &root,
        "certify-btor2-channel-properties",
        &queries,
        &root.join("tight-policy.txt"),
        &refused_artifact,
    );
    assert_eq!(refused.status.code(), Some(3));
    assert_eq!(
        String::from_utf8(refused.stderr).unwrap(),
        "error: btor2-channel-property-resource refusal=projected-work result=none\n"
    );
    assert!(!refused_artifact.exists());

    fs::write(root.join("drifted-queries.txt"), query_manifest(1)).unwrap();
    let drift = invoke(
        &root,
        "verify-btor2-channel-properties",
        &root.join("drifted-queries.txt"),
        &policy,
        &artifact,
    );
    assert_eq!(drift.status.code(), Some(2));

    let source = fs::read(root.join("model.btor2")).unwrap();
    let mut changed_source = source.clone();
    changed_source.extend_from_slice(b"; source drift\n");
    fs::write(root.join("model.btor2"), changed_source).unwrap();
    let source_drift = invoke(
        &root,
        "verify-btor2-channel-properties",
        &queries,
        &policy,
        &artifact,
    );
    assert_eq!(source_drift.status.code(), Some(2));
    fs::write(root.join("model.btor2"), source).unwrap();

    let mut tampered = fs::read(&artifact).unwrap();
    let index = tampered.len() / 2;
    tampered[index] ^= 1;
    fs::write(root.join("tampered.channel-properties"), tampered).unwrap();
    let rejected = invoke(
        &root,
        "verify-btor2-channel-properties",
        &queries,
        &policy,
        &root.join("tampered.channel-properties"),
    );
    assert_eq!(rejected.status.code(), Some(2));

    fs::write(
        root.join("crlf-queries.txt"),
        query_manifest(2).replace('\n', "\r\n"),
    )
    .unwrap();
    let rejected = invoke(
        &root,
        "certify-btor2-channel-properties",
        &root.join("crlf-queries.txt"),
        &policy,
        &root.join("crlf.channel-properties"),
    );
    assert_eq!(rejected.status.code(), Some(2));
    assert!(!root.join("crlf.channel-properties").exists());

    fs::write(
        root.join("noncanonical-policy.txt"),
        policy_text(6_189_840).replace("max_queries=4096", "max_queries=04096"),
    )
    .unwrap();
    let rejected = invoke(
        &root,
        "certify-btor2-channel-properties",
        &queries,
        &root.join("noncanonical-policy.txt"),
        &root.join("noncanonical.channel-properties"),
    );
    assert_eq!(rejected.status.code(), Some(2));
    assert!(!root.join("noncanonical.channel-properties").exists());

    #[cfg(unix)]
    {
        let linked_model = root.join("linked-model.btor2");
        std::os::unix::fs::symlink(root.join("model.btor2"), &linked_model).unwrap();
        let output = Command::new(BINARY)
            .arg("certify-btor2-channel-properties")
            .arg(linked_model)
            .arg(&queries)
            .arg(&policy)
            .arg(root.join("linked.channel-properties"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(!root.join("linked.channel-properties").exists());
    }

    fs::remove_dir_all(root).unwrap();
}
