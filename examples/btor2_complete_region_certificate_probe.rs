use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_region_extract::{
    Btor2RegionPolicy, decode_btor2_complete_region_artifact,
    encode_btor2_complete_region_artifact, produce_btor2_complete_region_artifact,
    verify_btor2_complete_region_artifact,
};
use sha2::{Digest, Sha256};

struct Fixture {
    channels: usize,
    bytes: &'static [u8],
    semantic_roots: &'static [u64],
}

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_complete_region_certificate_probe OUTPUT.csv")?;
    let fixtures = [
        Fixture {
            channels: 2,
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-2.btor2"
            ),
            semantic_roots: &[5, 17],
        },
        Fixture {
            channels: 4,
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-4.btor2"
            ),
            semantic_roots: &[5, 26],
        },
        Fixture {
            channels: 6,
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-6.btor2"
            ),
            semantic_roots: &[5, 36],
        },
    ];
    let policy = Btor2RegionPolicy::default();
    let mut rows = vec!["schema_version,channels,model_bytes,total_nodes,boundary_edges,state_artifact_bytes,complete_artifact_bytes,complete_over_model_percent,complete_artifact_sha256,deterministic,replayed,status".to_string()];
    for fixture in fixtures {
        let artifact = produce_btor2_complete_region_artifact(
            fixture.bytes,
            fixture.semantic_roots,
            fixture.channels,
            policy,
        )?;
        let encoded = encode_btor2_complete_region_artifact(&artifact, policy)?;
        let repeated = encode_btor2_complete_region_artifact(
            &produce_btor2_complete_region_artifact(
                fixture.bytes,
                fixture.semantic_roots,
                fixture.channels,
                policy,
            )?,
            policy,
        )?;
        let decoded = decode_btor2_complete_region_artifact(&encoded, policy)?;
        let verified = verify_btor2_complete_region_artifact(fixture.bytes, &decoded, policy)?;
        let total_nodes = verified.shared_nodes.len()
            + verified.channel_nodes.iter().map(Vec::len).sum::<usize>()
            + verified.aggregate_nodes.len();
        let boundary_edges =
            verified.shared_to_channel_edges.len() + verified.channel_to_aggregate_edges.len();
        let deterministic = encoded == repeated;
        if !deterministic {
            return Err("complete region artifact is not deterministic".into());
        }
        rows.push(format!(
            "1,{},{},{total_nodes},{boundary_edges},{},{},{:.2},{},{deterministic},true,accepted",
            fixture.channels,
            fixture.bytes.len(),
            artifact.state_artifact.len(),
            encoded.len(),
            encoded.len() as f64 * 100.0 / fixture.bytes.len() as f64,
            digest_hex(&encoded),
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("btor2_complete_region_certificate_probe_v1=PASS rows=3 output={output}");
    Ok(())
}
