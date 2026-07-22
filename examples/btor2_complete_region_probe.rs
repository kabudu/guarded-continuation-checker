use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_region_extract::{
    Btor2RegionPolicy, encode_btor2_region_artifact, extract_btor2_complete_regions,
    produce_btor2_region_artifact,
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

fn join_counts(values: impl IntoIterator<Item = usize>) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(":")
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_complete_region_probe OUTPUT.csv")?;
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
    let mut rows = vec!["schema_version,channels,total_states,shared_states,local_states_pattern,shared_nodes,local_nodes_pattern,aggregate_nodes,shared_to_channel_edges,channel_to_aggregate_edges,state_artifact_bytes,state_artifact_sha256,deterministic,status".to_string()];
    for fixture in fixtures {
        let complete = extract_btor2_complete_regions(
            fixture.bytes,
            fixture.semantic_roots,
            fixture.channels,
            policy,
        )?;
        let artifact = produce_btor2_region_artifact(
            fixture.bytes,
            fixture.semantic_roots,
            fixture.channels,
            policy,
        )?;
        let encoded = encode_btor2_region_artifact(&artifact, policy)?;
        let repeated = encode_btor2_region_artifact(
            &produce_btor2_region_artifact(
                fixture.bytes,
                fixture.semantic_roots,
                fixture.channels,
                policy,
            )?,
            policy,
        )?;
        let deterministic = encoded == repeated;
        if !deterministic {
            return Err("region artifact is not deterministic".into());
        }
        rows.push(format!(
            "1,{},{},{},{},{},{},{},{},{},{},{},{deterministic},accepted",
            fixture.channels,
            complete.state_regions.total_states,
            complete.state_regions.shared_states.len(),
            join_counts(
                complete
                    .state_regions
                    .channels
                    .iter()
                    .map(|channel| channel.states.len()),
            ),
            complete.shared_nodes.len(),
            join_counts(complete.channel_nodes.iter().map(Vec::len)),
            complete.aggregate_nodes.len(),
            complete.shared_to_channel_edges.len(),
            complete.channel_to_aggregate_edges.len(),
            encoded.len(),
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
    println!("btor2_complete_region_probe_v1=PASS rows=3 output={output}");
    Ok(())
}
