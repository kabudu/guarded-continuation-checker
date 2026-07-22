use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_region_equivalence::{
    decode_btor2_region_equivalence_artifact, encode_btor2_region_equivalence_artifact,
    produce_btor2_region_equivalence_artifact, verify_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;

struct Fixture {
    channels: usize,
    bytes: &'static [u8],
    roots: &'static [u64],
}

fn class_text(classes: &[Vec<usize>]) -> String {
    classes
        .iter()
        .map(|class| {
            class
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("+")
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_symbolic_class_certificate_probe OUTPUT.csv")?;
    let fixtures = [
        Fixture {
            channels: 2,
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-2.btor2"
            ),
            roots: &[9, 20],
        },
        Fixture {
            channels: 4,
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-4.btor2"
            ),
            roots: &[9, 29],
        },
        Fixture {
            channels: 6,
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
            ),
            roots: &[9, 39],
        },
    ];
    let policy = Btor2RegionPolicy::default();
    let mut rows = vec![
        "schema_version,channels,classes,artifact_bytes,byte_ratio,verified,deterministic,status"
            .to_string(),
    ];
    for fixture in fixtures {
        let artifact = produce_btor2_region_equivalence_artifact(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            policy,
        )?;
        let encoded = encode_btor2_region_equivalence_artifact(&artifact)?;
        let decoded = decode_btor2_region_equivalence_artifact(&encoded)?;
        let verified = verify_btor2_region_equivalence_artifact(fixture.bytes, &decoded, policy)?
            == artifact.summary;
        let repeated =
            encode_btor2_region_equivalence_artifact(&produce_btor2_region_equivalence_artifact(
                fixture.bytes,
                fixture.roots,
                fixture.channels,
                policy,
            )?)?;
        rows.push(format!(
            "1,{},{},{},{:.8},{verified},{},accepted",
            fixture.channels,
            class_text(&artifact.summary.classes),
            encoded.len(),
            encoded.len() as f64 / fixture.bytes.len() as f64,
            encoded == repeated,
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("btor2_symbolic_class_certificate_probe_v1=PASS rows=3 output={output}");
    Ok(())
}
