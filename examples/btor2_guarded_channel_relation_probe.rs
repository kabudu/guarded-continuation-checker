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
    Btor2BooleanBoundary, Btor2ChannelPairRelation, Btor2GuardedChannelRelationQuery,
    build_btor2_guarded_channel_relation_model,
};
use guarded_continuation_checker::btor2_search::SearchResult;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_guarded_channel_relation_probe OUTPUT.csv")?;
    let horizon = 2;
    let mut rows = Vec::new();
    for (guard_name, input_node) in [("class_write", 2), ("class_invert", 3), ("class_enable", 4)] {
        for bit_index in 0..2 {
            for relation in [
                Btor2ChannelPairRelation::Equal,
                Btor2ChannelPairRelation::Different,
            ] {
                let query = Btor2GuardedChannelRelationQuery {
                    left_channel_index: 0,
                    right_channel_index: 1,
                    relation,
                    guard: Btor2BooleanBoundary {
                        input_node,
                        bit_index,
                    },
                    horizon,
                };
                let (bytes, bad) = build_btor2_guarded_channel_relation_model(
                    MODEL,
                    ROOTS,
                    6,
                    query,
                    Btor2RegionPolicy::default(),
                )?;
                let certificate = produce_btor2_bitblast_certificate(&bytes, bad, horizon)?;
                let summary = verify_btor2_bitblast_certificate(&bytes, &certificate)?;
                let encoded = encode_btor2_bitblast_certificate(&certificate)?;
                let relation_name = match relation {
                    Btor2ChannelPairRelation::Equal => "equal",
                    Btor2ChannelPairRelation::Different => "different",
                };
                let result = match summary.result {
                    SearchResult::Safe => "SAFE",
                    SearchResult::Unsafe => "UNSAFE",
                };
                let bad_frame = summary
                    .bad_frame
                    .map_or_else(|| "none".to_string(), |frame| frame.to_string());
                rows.push(format!(
                    "symbolic-class-6,0,1,{guard_name},{input_node},{bit_index},{relation_name},{horizon},{result},{bad_frame},{},true",
                    encoded.len()
                ));
            }
        }
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(
        b"model,left,right,guard,input_node,bit_index,relation,horizon,result,bad_frame,certificate_bytes,verified\n",
    )?;
    for row in &rows {
        writeln!(file, "{row}")?;
    }
    file.sync_all()?;
    println!(
        "btor2_guarded_channel_relation_probe=PASS rows={} output={output}",
        rows.len()
    );
    Ok(())
}
