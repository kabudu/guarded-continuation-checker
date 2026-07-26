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
    Btor2BooleanBoundary, Btor2ChannelHistoryClass, Btor2ChannelHistoryGuard,
    Btor2ChannelPairRelation, Btor2GuardedChannelHistoryQuery,
    build_btor2_guarded_channel_history_model,
};
use guarded_continuation_checker::btor2_search::SearchResult;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn boundary(input_node: u64, bit_index: u32) -> Btor2BooleanBoundary {
    Btor2BooleanBoundary {
        input_node,
        bit_index,
    }
}

fn class(bit_index: u32) -> Btor2ChannelHistoryClass {
    Btor2ChannelHistoryClass {
        write: boundary(2, bit_index),
        enable: boundary(4, bit_index),
        invert: boundary(3, bit_index),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_channel_history_monitor_probe OUTPUT.csv")?;
    let mut rows = Vec::new();
    for horizon in [0, 1, 2, 4, 8] {
        for guard in [
            Btor2ChannelHistoryGuard::BothUnwritten,
            Btor2ChannelHistoryGuard::SameTrackedConfig,
            Btor2ChannelHistoryGuard::OppositeTrackedInvert,
        ] {
            for relation in [
                Btor2ChannelPairRelation::Equal,
                Btor2ChannelPairRelation::Different,
            ] {
                let query = Btor2GuardedChannelHistoryQuery {
                    left_channel_index: 0,
                    right_channel_index: 1,
                    relation,
                    left_class: class(0),
                    right_class: class(1),
                    guard,
                    horizon,
                };
                let (bytes, bad) = build_btor2_guarded_channel_history_model(
                    MODEL,
                    ROOTS,
                    6,
                    query,
                    Btor2RegionPolicy::default(),
                )?;
                let guard_name = match guard {
                    Btor2ChannelHistoryGuard::BothUnwritten => "both_unwritten",
                    Btor2ChannelHistoryGuard::SameTrackedConfig => "same_tracked_config",
                    Btor2ChannelHistoryGuard::OppositeTrackedInvert => "opposite_tracked_invert",
                };
                let relation_name = match relation {
                    Btor2ChannelPairRelation::Equal => "equal",
                    Btor2ChannelPairRelation::Different => "different",
                };
                match produce_btor2_bitblast_certificate(&bytes, bad, horizon) {
                    Ok(certificate) => {
                        let summary = verify_btor2_bitblast_certificate(&bytes, &certificate)?;
                        let encoded = encode_btor2_bitblast_certificate(&certificate)?;
                        let result = match summary.result {
                            SearchResult::Safe => "SAFE",
                            SearchResult::Unsafe => "UNSAFE",
                        };
                        let bad_frame = summary
                            .bad_frame
                            .map_or_else(|| "none".to_string(), |frame| frame.to_string());
                        rows.push(format!(
                            "symbolic-class-6,0,1,{guard_name},{relation_name},{horizon},{result},{bad_frame},{},true,accepted",
                            encoded.len()
                        ));
                    }
                    Err(_) => rows.push(format!(
                        "symbolic-class-6,0,1,{guard_name},{relation_name},{horizon},NONE,none,0,false,refused"
                    )),
                }
            }
        }
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(
        b"model,left,right,guard,relation,horizon,result,bad_frame,certificate_bytes,verified,status\n",
    )?;
    for row in &rows {
        writeln!(file, "{row}")?;
    }
    file.sync_all()?;
    println!(
        "btor2_channel_history_monitor_probe=PASS rows={} output={output}",
        rows.len()
    );
    Ok(())
}
