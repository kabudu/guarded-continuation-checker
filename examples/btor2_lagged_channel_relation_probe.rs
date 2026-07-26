use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_bitblast::{
    Btor2BitblastCertificate, encode_btor2_bitblast_certificate,
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelPairRelation, Btor2LaggedChannelOrientation, Btor2LaggedChannelRelationQuery,
    build_btor2_lagged_channel_relation_models,
};
use guarded_continuation_checker::btor2_search::SearchResult;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn produce_shortest_or_safe(
    model: &[u8],
    bad: u64,
    horizon: u32,
) -> Result<Btor2BitblastCertificate, Box<dyn Error>> {
    for prefix in 0..=horizon {
        let certificate = produce_btor2_bitblast_certificate(model, bad, prefix)?;
        if certificate.result == SearchResult::Unsafe || prefix == horizon {
            return Ok(certificate);
        }
    }
    unreachable!("bounded prefix loop always returns at its final horizon")
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_lagged_channel_relation_probe OUTPUT.csv")?;
    let mut rows = Vec::new();
    for horizon in [4, 8, 12, 16] {
        for orientation in [
            Btor2LaggedChannelOrientation::LeftLeads,
            Btor2LaggedChannelOrientation::RightLeads,
        ] {
            for relation in [
                Btor2ChannelPairRelation::Equal,
                Btor2ChannelPairRelation::Different,
            ] {
                let query = Btor2LaggedChannelRelationQuery {
                    left_channel_index: 0,
                    right_channel_index: 1,
                    orientation,
                    relation,
                    lag: 2,
                    horizon,
                };
                let models = build_btor2_lagged_channel_relation_models(
                    MODEL,
                    ROOTS,
                    6,
                    query,
                    Btor2RegionPolicy::default(),
                )?;
                let orientation_name = match orientation {
                    Btor2LaggedChannelOrientation::LeftLeads => "left_leads",
                    Btor2LaggedChannelOrientation::RightLeads => "right_leads",
                };
                let relation_name = match relation {
                    Btor2ChannelPairRelation::Equal => "equal",
                    Btor2ChannelPairRelation::Different => "different",
                };

                let coverage =
                    produce_shortest_or_safe(&models.coverage_model, models.coverage_bad, horizon)?;
                let coverage_summary =
                    verify_btor2_bitblast_certificate(&models.coverage_model, &coverage)?;
                let coverage_bytes = encode_btor2_bitblast_certificate(&coverage)?.len();
                let covered = coverage_summary.result == SearchResult::Unsafe
                    && coverage_summary.bad_frame == Some(2);

                match produce_shortest_or_safe(
                    &models.relation_model,
                    models.relation_bad,
                    horizon,
                ) {
                    Ok(certificate) => {
                        let summary = verify_btor2_bitblast_certificate(
                            &models.relation_model,
                            &certificate,
                        )?;
                        let certificate_bytes =
                            encode_btor2_bitblast_certificate(&certificate)?.len();
                        let result = if covered {
                            match summary.result {
                                SearchResult::Safe => "SAFE",
                                SearchResult::Unsafe => "UNSAFE",
                            }
                        } else {
                            "NONE"
                        };
                        let bad_frame = summary
                            .bad_frame
                            .map_or_else(|| "none".to_string(), |frame| frame.to_string());
                        rows.push(format!(
                            "{orientation_name},{relation_name},2,{horizon},{},{coverage_bytes},{result},{bad_frame},{certificate_bytes},true,accepted,{}",
                            coverage_summary.bad_frame.unwrap_or_default(),
                            models.history_state_bits
                        ));
                    }
                    Err(_) => rows.push(format!(
                        "{orientation_name},{relation_name},2,{horizon},{},{coverage_bytes},NONE,none,0,false,refused,{}",
                        coverage_summary.bad_frame.unwrap_or_default(),
                        models.history_state_bits
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
        b"orientation,relation,lag,horizon,coverage_frame,coverage_certificate_bytes,result,bad_frame,relation_certificate_bytes,verified,status,history_state_bits\n",
    )?;
    for row in &rows {
        writeln!(file, "{row}")?;
    }
    file.sync_all()?;
    println!(
        "btor2_lagged_channel_relation_probe=PASS rows={} output={output}",
        rows.len()
    );
    Ok(())
}
