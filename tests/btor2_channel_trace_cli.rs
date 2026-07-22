use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn query_manifest() -> String {
    let mut text = String::from("gcc-btor2-channel-traces-v1\nchannels=6\nsemantic_roots=9,39\n");
    let mut query_id = 0;
    for (length, mask, value, horizon) in [(1, 1, 1, 1), (1, 1, 0, 1), (1, 1, 0, 2)] {
        for channel in 0..6 {
            text.push_str(&format!(
                "query={query_id},{channel},{length},{mask},{value},{horizon}\n"
            ));
            query_id += 1;
        }
    }
    text.push_str("status=complete\n");
    text
}

fn policy(max_projected_work: u64) -> String {
    format!(
        "channel_trace_policy_version=1\nmax_queries=4096\nmax_members=4096\nmax_evidence_bytes=67108864\nmax_artifact_bytes=69206016\nmax_projected_work={max_projected_work}\nstatus=complete\n"
    )
}

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-channel-trace-cli-{}-{}",
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
    fs::write(root.join("queries.txt"), query_manifest()).unwrap();
    fs::write(root.join("policy.txt"), policy(100_000_000_000)).unwrap();
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

fn projected_work(stdout: &str) -> u64 {
    stdout
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .find_map(|field| field.strip_prefix("projected_work="))
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn channel_trace_cli_is_bounded_self_service_and_fail_closed() {
    let version = Command::new(BINARY)
        .arg("btor2-channel-trace-cli-version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        "btor2_channel_trace_cli_version=1 artifact_version=1 query_manifest_version=1 policy_version=1 max_query_manifest_bytes=262144 max_policy_bytes=4096 max_model_bytes=8388608 max_channels=64 max_queries=4096 max_pattern_length=8 max_horizon=64 max_evidence_bytes=67108864 max_artifact_bytes=69206016 max_projected_work=100000000000 refusal_exit=3 routing=static-explicit-or-bitblast fallback=exact result_on_refusal=none refusal_schema=reason-v1 unsupported=fail-closed verification=source-replay-and-shortest-frame-proof publication=create-new\n"
    );

    let root = fixture();
    let queries = root.join("queries.txt");
    let policy_path = root.join("policy.txt");
    let artifact = root.join("result.channel-traces");
    let created = invoke(
        &root,
        "certify-btor2-channel-traces",
        &queries,
        &policy_path,
        &artifact,
    );
    assert!(created.status.success(), "{:?}", created.stderr);
    let stdout = String::from_utf8(created.stdout).unwrap();
    assert!(stdout.starts_with(
        "btor2-channel-traces status=CREATED cli_version=1 artifact_version=1 channels=6 logical_queries=18 proof_members=9 reused_queries=9 explicit_members=6 bitblast_members=3 "
    ));
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("btor2-channel-trace index="))
            .count(),
        18
    );
    let artifact_bytes = fs::read(&artifact).unwrap();
    assert_eq!(artifact_bytes.len(), 4_808);
    assert!(artifact_bytes.starts_with(b"GCCTRC01"));
    assert!(!fs::read_dir(&root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".gcc-certificate-")
    }));

    let verified = invoke(
        &root,
        "verify-btor2-channel-traces",
        &queries,
        &policy_path,
        &artifact,
    );
    assert!(verified.status.success(), "{:?}", verified.stderr);
    assert!(String::from_utf8(verified.stdout).unwrap().starts_with(
        "btor2-channel-traces status=VERIFIED cli_version=1 artifact_version=1 channels=6 logical_queries=18 proof_members=9 reused_queries=9 explicit_members=6 bitblast_members=3 "
    ));

    let collision = invoke(
        &root,
        "certify-btor2-channel-traces",
        &queries,
        &policy_path,
        &artifact,
    );
    assert_eq!(collision.status.code(), Some(2));
    assert_eq!(fs::read(&artifact).unwrap(), artifact_bytes);

    let work = projected_work(&stdout);
    fs::write(root.join("tight-policy.txt"), policy(work - 1)).unwrap();
    let refused_artifact = root.join("refused.channel-traces");
    let refused = invoke(
        &root,
        "certify-btor2-channel-traces",
        &queries,
        &root.join("tight-policy.txt"),
        &refused_artifact,
    );
    assert_eq!(refused.status.code(), Some(3));
    assert_eq!(
        String::from_utf8(refused.stderr).unwrap(),
        "error: btor2-channel-trace-resource refusal=projected-work result=none\n"
    );
    assert!(!refused_artifact.exists());

    let drifted = query_manifest().replacen("query=0,0,1,1,1,1", "query=0,0,1,1,0,1", 1);
    fs::write(root.join("drifted.txt"), drifted).unwrap();
    let drift = invoke(
        &root,
        "verify-btor2-channel-traces",
        &root.join("drifted.txt"),
        &policy_path,
        &artifact,
    );
    assert_eq!(drift.status.code(), Some(2));

    let mut tampered = artifact_bytes.clone();
    let offset = tampered.len() / 2;
    tampered[offset] ^= 1;
    fs::write(root.join("tampered.channel-traces"), tampered).unwrap();
    let rejected = invoke(
        &root,
        "verify-btor2-channel-traces",
        &queries,
        &policy_path,
        &root.join("tampered.channel-traces"),
    );
    assert_eq!(rejected.status.code(), Some(2));

    fs::write(
        root.join("invalid-pattern.txt"),
        query_manifest().replacen("query=0,0,1,1,1,1", "query=0,0,3,1,2,1", 1),
    )
    .unwrap();
    let rejected = invoke(
        &root,
        "certify-btor2-channel-traces",
        &root.join("invalid-pattern.txt"),
        &policy_path,
        &root.join("invalid.channel-traces"),
    );
    assert_eq!(rejected.status.code(), Some(2));
    assert!(!root.join("invalid.channel-traces").exists());

    fs::write(root.join("oversized.txt"), vec![b'x'; 256 * 1024 + 1]).unwrap();
    let rejected = invoke(
        &root,
        "certify-btor2-channel-traces",
        &root.join("oversized.txt"),
        &policy_path,
        &root.join("oversized.channel-traces"),
    );
    assert_eq!(rejected.status.code(), Some(2));
    assert!(!root.join("oversized.channel-traces").exists());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("model.btor2"), root.join("linked-model.btor2"))
            .unwrap();
        let linked = Command::new(BINARY)
            .arg("verify-btor2-channel-traces")
            .arg(root.join("linked-model.btor2"))
            .arg(&queries)
            .arg(&policy_path)
            .arg(&artifact)
            .output()
            .unwrap();
        assert_eq!(linked.status.code(), Some(2));
    }

    fs::remove_dir_all(root).unwrap();
}
