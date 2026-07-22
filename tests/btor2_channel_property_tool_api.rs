use guarded_continuation_checker::{
    Btor2ChannelPropertyAnswer, Btor2ChannelPropertyFiles,
    Btor2ChannelPropertyResourceRefusalReason, Btor2ChannelPropertyTool, FailureClass,
    InvocationStatus, OperationKind, PredicateApiError,
};
use std::fs;
use std::path::PathBuf;

const BINARY: &str = env!("CARGO_BIN_EXE_guarded-continuation-checker");

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gcc-channel-property-tool-api-{}",
        std::process::id()
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
    let mut queries =
        String::from("gcc-btor2-channel-properties-v1\nchannels=6\nsemantic_roots=9,39\n");
    let mut query_id = 0;
    for property in ["output-high", "output-low"] {
        for channel in 0..6 {
            queries.push_str(&format!("query={query_id},{channel},{property},2\n"));
            query_id += 1;
        }
    }
    queries.push_str("status=complete\n");
    fs::write(root.join("queries.txt"), queries).unwrap();
    fs::write(
        root.join("policy.txt"),
        b"channel_property_policy_version=1\nmax_queries=4096\nmax_members=4096\nmax_evidence_bytes=67108864\nmax_artifact_bytes=69206016\nmax_projected_work=6189840\nstatus=complete\n",
    )
    .unwrap();
    fs::write(
        root.join("tight-policy.txt"),
        b"channel_property_policy_version=1\nmax_queries=4096\nmax_members=4096\nmax_evidence_bytes=67108864\nmax_artifact_bytes=69206016\nmax_projected_work=6189839\nstatus=complete\n",
    )
    .unwrap();
    root
}

#[test]
fn typed_channel_property_tool_governs_and_parses_the_complete_workflow() {
    let root = fixture();
    let discovery = Btor2ChannelPropertyTool::discover_observed(
        BINARY,
        guarded_continuation_checker::ExecutionPolicy::default()
            .with_file_limit(69_206_016)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        discovery.metrics.operation,
        OperationKind::DiscoverBtor2ChannelProperty
    );
    assert_eq!(discovery.metrics.status, InvocationStatus::Success);
    let tool = discovery.value;
    let capabilities = tool.capabilities();
    assert_eq!(capabilities.cli_version, 1);
    assert_eq!(capabilities.artifact_version, 1);
    assert_eq!(capabilities.max_artifact_bytes, 69_206_016);
    assert_eq!(capabilities.max_projected_work, 100_000_000_000);
    assert_eq!(capabilities.refusal_exit_code, 3);

    let files = Btor2ChannelPropertyFiles {
        model: &root.join("model.btor2"),
        queries: &root.join("queries.txt"),
        policy: &root.join("policy.txt"),
    };
    let artifact = root.join("result.channel-properties");
    let created = tool.certify_observed(&files, &artifact).unwrap();
    assert_eq!(
        created.metrics.operation,
        OperationKind::CertifyBtor2ChannelProperty
    );
    assert_eq!(created.metrics.status, InvocationStatus::Success);
    assert_eq!(created.value.channels, 6);
    assert_eq!(created.value.logical_queries, 12);
    assert_eq!(created.value.proof_members, 6);
    assert_eq!(created.value.reused_queries, 6);
    assert_eq!(created.value.explicit_members, 0);
    assert_eq!(created.value.bitblast_members, 6);
    assert_eq!(created.value.projected_work, Some(6_189_840));
    assert_eq!(created.value.results.len(), 12);
    assert!(
        created
            .value
            .results
            .iter()
            .all(|result| result.answer == Btor2ChannelPropertyAnswer::Unsafe)
    );
    assert_eq!(
        created.value.artifact_bytes,
        fs::metadata(&artifact).unwrap().len() as usize
    );

    let verified = tool.verify_observed(&files, &artifact).unwrap();
    assert_eq!(
        verified.metrics.operation,
        OperationKind::VerifyBtor2ChannelProperty
    );
    assert_eq!(verified.metrics.status, InvocationStatus::Success);
    assert_eq!(verified.value.projected_work, None);
    assert_eq!(verified.value.results, created.value.results);
    assert_eq!(verified.value.artifact_bytes, created.value.artifact_bytes);

    let tight_files = Btor2ChannelPropertyFiles {
        policy: &root.join("tight-policy.txt"),
        ..files.clone()
    };
    let refused = tool
        .certify_observed(&tight_files, &root.join("refused.channel-properties"))
        .unwrap_err();
    assert!(matches!(
        refused.error.as_ref(),
        PredicateApiError::Btor2ChannelPropertyResourceRefused {
            reason: Btor2ChannelPropertyResourceRefusalReason::ProjectedWork
        }
    ));
    assert_eq!(
        refused.metrics.status,
        InvocationStatus::Failed(FailureClass::ResourceRefusal)
    );
    assert_eq!(refused.metrics.exit_code, Some(3));
    assert!(!root.join("refused.channel-properties").exists());

    fs::remove_dir_all(root).unwrap();
}
