use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelPairRelation, Btor2ChannelPairTraceQuery, Btor2ChannelTracePattern,
    Btor2ChannelTraceProductionPolicy, produce_btor2_channel_pair_trace_proof,
    verify_btor2_channel_pair_trace_proof,
};
use guarded_continuation_checker::btor2_search::SearchResult;

const MODEL: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2");
const ROOTS: &[u64] = &[9, 39];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut queries = Vec::new();
    for (left, right) in [(0, 2), (2, 4), (4, 0)] {
        for relation in [
            Btor2ChannelPairRelation::Equal,
            Btor2ChannelPairRelation::Different,
        ] {
            for (length, mask, value, horizon) in [(1, 1, 1, 0), (2, 3, 0, 2)] {
                queries.push(Btor2ChannelPairTraceQuery {
                    query_id: u32::try_from(queries.len())?,
                    left_channel_index: left,
                    right_channel_index: right,
                    relation,
                    pattern: Btor2ChannelTracePattern::new(length, mask, value)?,
                    horizon,
                });
            }
        }
    }
    let region_policy = Btor2RegionPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(MODEL, ROOTS, 6, region_policy)?,
    )?;
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
        Btor2ChannelTraceProductionPolicy::default().artifact(),
    )?;
    println!("model,query_id,left,right,relation,length,mask,value,horizon,result,bad_frame");
    for result in summary.results {
        let relation = match result.query.relation {
            Btor2ChannelPairRelation::Equal => "equal",
            Btor2ChannelPairRelation::Different => "different",
        };
        let answer = match result.result {
            SearchResult::Safe => "SAFE",
            SearchResult::Unsafe => "UNSAFE",
        };
        let bad_frame = result
            .bad_frame
            .map_or_else(|| "none".to_string(), |frame| frame.to_string());
        println!(
            "symbolic-class-6,{},{},{},{relation},{},{},{},{},{answer},{bad_frame}",
            result.query.query_id,
            result.query.left_channel_index,
            result.query.right_channel_index,
            result.query.pattern.length(),
            result.query.pattern.mask(),
            result.query.pattern.value(),
            result.query.horizon,
        );
    }
    Ok(())
}
