use std::collections::BTreeMap;
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
    Btor2ChannelPairRelation, Btor2ChannelPhaseAbstractionQuery, Btor2GuardedChannelHistoryQuery,
    build_btor2_channel_phase_abstraction_models,
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

fn guard_name(guard: Btor2ChannelHistoryGuard) -> &'static str {
    match guard {
        Btor2ChannelHistoryGuard::BothUnwritten => "both_unwritten",
        Btor2ChannelHistoryGuard::SameTrackedConfig => "same_tracked_config",
        Btor2ChannelHistoryGuard::OppositeTrackedInvert => "opposite_tracked_invert",
    }
}

fn query(
    phase_value: u64,
    guard: Btor2ChannelHistoryGuard,
    relation: Btor2ChannelPairRelation,
) -> Btor2ChannelPhaseAbstractionQuery {
    Btor2ChannelPhaseAbstractionQuery {
        history: Btor2GuardedChannelHistoryQuery {
            left_channel_index: 0,
            right_channel_index: 1,
            relation,
            left_class: class(0),
            right_class: class(1),
            guard,
            horizon: 8,
        },
        phase_root: 9,
        phase_value,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_channel_phase_abstraction_probe OUTPUT.csv")?;
    let guards = [
        Btor2ChannelHistoryGuard::BothUnwritten,
        Btor2ChannelHistoryGuard::SameTrackedConfig,
        Btor2ChannelHistoryGuard::OppositeTrackedInvert,
    ];
    let mut rows = Vec::new();
    let mut reachable = BTreeMap::new();
    for phase in 0..=8 {
        for guard in guards {
            let models = build_btor2_channel_phase_abstraction_models(
                MODEL,
                ROOTS,
                6,
                query(phase, guard, Btor2ChannelPairRelation::Equal),
                Btor2RegionPolicy::default(),
            )?;
            let key = (phase, guard);
            match produce_btor2_bitblast_certificate(
                &models.reachability_model,
                models.reachability_bad,
                8,
            ) {
                Ok(certificate) => {
                    let summary = verify_btor2_bitblast_certificate(
                        &models.reachability_model,
                        &certificate,
                    )?;
                    let encoded = encode_btor2_bitblast_certificate(&certificate)?;
                    let is_reachable = summary.result == SearchResult::Unsafe;
                    reachable.insert(key, is_reachable);
                    let result = if is_reachable {
                        "REACHABLE"
                    } else {
                        "UNREACHABLE"
                    };
                    let frame = summary
                        .bad_frame
                        .map_or_else(|| "none".to_string(), |value| value.to_string());
                    rows.push(format!(
                        "reachability,{phase},{},{},8,{result},{frame},{},true,accepted,10,33,69",
                        guard_name(guard),
                        "none",
                        encoded.len()
                    ));
                }
                Err(_) => {
                    reachable.insert(key, false);
                    rows.push(format!(
                        "reachability,{phase},{},{},8,NONE,none,0,false,refused,10,33,69",
                        guard_name(guard),
                        "none"
                    ));
                }
            }
        }
    }

    for phase in 0..=8 {
        for guard in guards {
            for relation in [
                Btor2ChannelPairRelation::Equal,
                Btor2ChannelPairRelation::Different,
            ] {
                let relation_name = match relation {
                    Btor2ChannelPairRelation::Equal => "equal",
                    Btor2ChannelPairRelation::Different => "different",
                };
                let models = build_btor2_channel_phase_abstraction_models(
                    MODEL,
                    ROOTS,
                    6,
                    query(phase, guard, relation),
                    Btor2RegionPolicy::default(),
                )?;
                match produce_btor2_bitblast_certificate(
                    &models.relation_model,
                    models.relation_bad,
                    8,
                ) {
                    Ok(certificate) => {
                        let summary = verify_btor2_bitblast_certificate(
                            &models.relation_model,
                            &certificate,
                        )?;
                        let encoded = encode_btor2_bitblast_certificate(&certificate)?;
                        let non_vacuous = reachable[&(phase, guard)];
                        let result = if non_vacuous {
                            match summary.result {
                                SearchResult::Safe => "SAFE",
                                SearchResult::Unsafe => "UNSAFE",
                            }
                        } else {
                            "NONE"
                        };
                        let frame = summary
                            .bad_frame
                            .map_or_else(|| "none".to_string(), |value| value.to_string());
                        rows.push(format!(
                            "relation,{phase},{},{relation_name},8,{result},{frame},{},true,accepted,10,33,69",
                            guard_name(guard),
                            encoded.len()
                        ));
                    }
                    Err(_) => rows.push(format!(
                        "relation,{phase},{},{relation_name},8,NONE,none,0,false,refused,10,33,69",
                        guard_name(guard)
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
        b"kind,phase,guard,relation,horizon,result,bad_frame,certificate_bytes,verified,status,abstract_state_bits,concrete_state_nodes,concrete_state_bits\n",
    )?;
    for row in &rows {
        writeln!(file, "{row}")?;
    }
    file.sync_all()?;
    println!(
        "btor2_channel_phase_abstraction_probe=PASS rows={} output={output}",
        rows.len()
    );
    Ok(())
}
