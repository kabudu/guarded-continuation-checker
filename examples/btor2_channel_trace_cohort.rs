use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelTracePattern, Btor2ChannelTraceProductionPolicy, Btor2ChannelTraceQuery,
    produce_btor2_channel_trace_proof_bytes, verify_btor2_channel_trace_proof_bytes,
};
use guarded_continuation_checker::btor2_search::SearchResult;

struct Fixture {
    name: &'static str,
    model: &'static [u8],
    roots: &'static [u64],
    channels: usize,
}

fn shapes() -> [(Btor2ChannelTracePattern, u32); 7] {
    [
        (Btor2ChannelTracePattern::new(1, 0b1, 0b1).unwrap(), 1),
        (Btor2ChannelTracePattern::new(1, 0b1, 0b0).unwrap(), 1),
        (Btor2ChannelTracePattern::new(2, 0b11, 0b01).unwrap(), 8),
        (Btor2ChannelTracePattern::new(2, 0b11, 0b10).unwrap(), 8),
        (Btor2ChannelTracePattern::new(3, 0b111, 0b010).unwrap(), 8),
        (Btor2ChannelTracePattern::new(3, 0b111, 0b101).unwrap(), 8),
        (Btor2ChannelTracePattern::new(3, 0b101, 0b001).unwrap(), 8),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = [
        Fixture {
            name: "symbolic-class-2",
            model: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-2.btor2"
            ),
            roots: &[9, 20],
            channels: 2,
        },
        Fixture {
            name: "symbolic-class-4",
            model: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-4.btor2"
            ),
            roots: &[9, 29],
            channels: 4,
        },
        Fixture {
            name: "symbolic-class-6",
            model: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
            ),
            roots: &[9, 39],
            channels: 6,
        },
    ];
    let region_policy = Btor2RegionPolicy::default();
    let production_policy = Btor2ChannelTraceProductionPolicy::default();
    println!("model,query_id,channel,length,mask,value,horizon,result,bad_frame");
    for fixture in fixtures {
        let structural =
            encode_btor2_region_equivalence_artifact(&produce_btor2_region_equivalence_artifact(
                fixture.model,
                fixture.roots,
                fixture.channels,
                region_policy,
            )?)?;
        let mut queries = Vec::new();
        for (pattern, horizon) in shapes() {
            for channel_index in 0..fixture.channels {
                queries.push(Btor2ChannelTraceQuery {
                    query_id: u32::try_from(queries.len())?,
                    channel_index,
                    pattern,
                    horizon,
                });
            }
        }
        let (_, bytes) = produce_btor2_channel_trace_proof_bytes(
            fixture.model,
            &structural,
            &queries,
            region_policy,
            production_policy,
        )?;
        let summary = verify_btor2_channel_trace_proof_bytes(
            fixture.model,
            &queries,
            &bytes,
            region_policy,
            production_policy.artifact(),
        )?;
        for result in summary.results {
            let query = result.query;
            let answer = match result.result {
                SearchResult::Safe => "SAFE",
                SearchResult::Unsafe => "UNSAFE",
            };
            println!(
                "{},{},{},{},{},{},{},{},{}",
                fixture.name,
                query.query_id,
                query.channel_index,
                query.pattern.length(),
                query.pattern.mask(),
                query.pattern.value(),
                query.horizon,
                answer,
                result
                    .bad_frame
                    .map_or_else(|| "none".to_owned(), |frame| frame.to_string()),
            );
        }
    }
    Ok(())
}
