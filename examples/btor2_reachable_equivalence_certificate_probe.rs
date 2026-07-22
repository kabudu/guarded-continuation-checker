use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;

use guarded_continuation_checker::btor2_region_equivalence::{
    decode_btor2_reachable_region_equivalence_artifact,
    encode_btor2_reachable_region_equivalence_artifact,
    produce_btor2_reachable_region_equivalence_artifact,
    verify_btor2_reachable_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
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
        .ok_or("usage: btor2_reachable_equivalence_certificate_probe OUTPUT.csv")?;
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
    let horizon = 63;
    let mut rows = vec!["schema_version,channels,horizon,classes,representatives,reused_channels,artifact_bytes,artifact_sha256,produce_micros,verify_micros,deterministic,replayed,status".to_string()];
    for fixture in fixtures {
        let produce_start = Instant::now();
        let artifact = produce_btor2_reachable_region_equivalence_artifact(
            fixture.bytes,
            fixture.semantic_roots,
            fixture.channels,
            horizon,
            policy,
        )?;
        let encoded = encode_btor2_reachable_region_equivalence_artifact(&artifact)?;
        let produce_micros = produce_start.elapsed().as_micros();
        let repeated = encode_btor2_reachable_region_equivalence_artifact(
            &produce_btor2_reachable_region_equivalence_artifact(
                fixture.bytes,
                fixture.semantic_roots,
                fixture.channels,
                horizon,
                policy,
            )?,
        )?;
        let decoded = decode_btor2_reachable_region_equivalence_artifact(&encoded)?;
        let verify_start = Instant::now();
        let verified =
            verify_btor2_reachable_region_equivalence_artifact(fixture.bytes, &decoded, policy)?;
        let verify_micros = verify_start.elapsed().as_micros();
        let classes = verified.classes.len();
        let reused_channels = fixture.channels - classes;
        let deterministic = encoded == repeated;
        if !deterministic {
            return Err("reachable-equivalence artifact is not deterministic".into());
        }
        rows.push(format!(
            "1,{},{horizon},{classes},{classes},{reused_channels},{},{},{produce_micros},{verify_micros},{deterministic},true,accepted",
            fixture.channels,
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
    println!("btor2_reachable_equivalence_certificate_probe_v1=PASS rows=3 output={output}");
    Ok(())
}
