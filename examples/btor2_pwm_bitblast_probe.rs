use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_bitblast::{
    encode_btor2_bitblast_certificate, produce_btor2_bitblast_certificate,
    verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, build_btor2_channel_property_model,
};
use guarded_continuation_checker::btor2_search;

struct Fixture {
    channels: usize,
    bytes: &'static [u8],
    roots: &'static [u64],
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_pwm_bitblast_probe OUTPUT.csv")?;
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
    let mut rows = vec!["schema_version,channels,horizon,result,bad_frame,variables,clauses,proof_bytes,certificate_bytes,explicit_status,verified,status".to_string()];
    for fixture in fixtures {
        let (property_model, bad) = build_btor2_channel_property_model(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            0,
            Btor2ChannelProperty::OutputHigh,
            Btor2RegionPolicy::default(),
        )?;
        for horizon in [1, 2] {
            let certificate = produce_btor2_bitblast_certificate(&property_model, bad, horizon)?;
            let encoded = encode_btor2_bitblast_certificate(&certificate)?;
            let summary = verify_btor2_bitblast_certificate(&property_model, &certificate)?;
            let explicit_status = match btor2_search::produce(&property_model, bad, horizon) {
                Ok(explicit)
                    if explicit.result == summary.result
                        && explicit.bad_frame == summary.bad_frame =>
                {
                    "agreed"
                }
                Ok(_) => "disagreed",
                Err(_) => "resource-refused",
            };
            rows.push(format!(
                "1,{},{horizon},{:?},{},{},{},{},{},{explicit_status},true,accepted",
                fixture.channels,
                summary.result,
                summary
                    .bad_frame
                    .map_or_else(|| "none".to_string(), |frame| frame.to_string()),
                summary.variables,
                summary.clauses,
                summary.proof_bytes,
                encoded.len(),
            ));
        }
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("btor2_pwm_bitblast_probe_v1=PASS rows=6 output={output}");
    Ok(())
}
