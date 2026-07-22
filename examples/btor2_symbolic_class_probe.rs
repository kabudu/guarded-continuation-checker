use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_region_equivalence::derive_btor2_region_equivalence;
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use sha2::{Digest, Sha256};

struct Fixture {
    channels: usize,
    bytes: &'static [u8],
    roots: &'static [u64],
}

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
        .ok_or("usage: btor2_symbolic_class_probe OUTPUT.csv")?;
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
    let mut rows = vec!["schema_version,channels,input_nodes,states,classes,non_singleton_classes,reused_channels,model_bytes,model_sha256,deterministic,status".to_string()];
    for fixture in fixtures {
        let model = btor2::parse_component_bytes(fixture.bytes, fixture.roots)?;
        let equivalence = derive_btor2_region_equivalence(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            Btor2RegionPolicy::default(),
        )?;
        let repeated = derive_btor2_region_equivalence(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            Btor2RegionPolicy::default(),
        )?;
        let deterministic = equivalence == repeated;
        let non_singleton_classes = equivalence
            .classes
            .iter()
            .filter(|class| class.len() > 1)
            .count();
        let reused_channels = equivalence
            .classes
            .iter()
            .map(|class| class.len().saturating_sub(1))
            .sum::<usize>();
        rows.push(format!(
            "1,{},{},{},{},{non_singleton_classes},{reused_channels},{},{},{deterministic},accepted",
            fixture.channels,
            model.inputs().len(),
            model.states().len(),
            class_text(&equivalence.classes),
            fixture.bytes.len(),
            digest_hex(fixture.bytes),
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("btor2_symbolic_class_probe_v1=PASS rows=3 output={output}");
    Ok(())
}
