use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelPairRelation, Btor2ChannelPairTraceQuery, Btor2ChannelTracePattern,
    Btor2ChannelTraceProductionPolicy, Btor2ChannelTraceProofPolicy,
    produce_btor2_channel_pair_trace_proof, verify_btor2_channel_pair_trace_proof,
};
use guarded_continuation_checker::btor2_search;
use std::error::Error;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];
const CHANNELS: usize = 6;

fn main() -> Result<(), Box<dyn Error>> {
    let region_policy = Btor2RegionPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(MODEL, ROOTS, CHANNELS, region_policy)?,
    )?;
    let shapes = [
        (Btor2ChannelTracePattern::new(1, 0b1, 0b1)?, 0),
        (Btor2ChannelTracePattern::new(2, 0b11, 0b01)?, 2),
        (Btor2ChannelTracePattern::new(2, 0b11, 0b11)?, 2),
        (Btor2ChannelTracePattern::new(2, 0b11, 0b00)?, 2),
    ];
    let mut queries = Vec::new();
    for (left, right) in [(0, 2), (2, 4), (4, 0)] {
        for relation in [
            Btor2ChannelPairRelation::Equal,
            Btor2ChannelPairRelation::Different,
        ] {
            for (pattern, horizon) in shapes {
                queries.push(Btor2ChannelPairTraceQuery {
                    query_id: u32::try_from(queries.len())?,
                    left_channel_index: left,
                    right_channel_index: right,
                    relation,
                    pattern,
                    horizon,
                });
            }
        }
    }
    let artifact = produce_btor2_channel_pair_trace_proof(
        MODEL,
        &structural,
        &queries,
        region_policy,
        Btor2ChannelTraceProductionPolicy::default(),
    )?;
    let summary = verify_btor2_channel_pair_trace_proof(
        MODEL,
        &queries,
        &artifact,
        region_policy,
        Btor2ChannelTraceProofPolicy::default(),
    )?;
    println!(
        "logical_queries,proof_members,structural_constant_members,reused_logical_queries,structural_bytes,evidence_bytes,safe,unsafe"
    );
    println!(
        "{},{},{},{},{},{},{},{}",
        summary.metrics.logical_queries,
        summary.metrics.proof_members,
        summary.metrics.structural_constant_members,
        summary.metrics.reused_logical_queries,
        structural.len(),
        summary.metrics.evidence_bytes,
        summary
            .results
            .iter()
            .filter(|result| result.result == btor2_search::SearchResult::Safe)
            .count(),
        summary
            .results
            .iter()
            .filter(|result| result.result == btor2_search::SearchResult::Unsafe)
            .count(),
    );
    println!("query_id,left,right,relation,length,mask,value,horizon,result,bad_frame");
    for result in &summary.results {
        println!(
            "{},{},{},{:?},{},{},{},{},{:?},{}",
            result.query.query_id,
            result.query.left_channel_index,
            result.query.right_channel_index,
            result.query.relation,
            result.query.pattern.length(),
            result.query.pattern.mask(),
            result.query.pattern.value(),
            result.query.horizon,
            result.result,
            result
                .bad_frame
                .map_or_else(|| "none".to_string(), |frame| frame.to_string()),
        );
    }
    Ok(())
}
