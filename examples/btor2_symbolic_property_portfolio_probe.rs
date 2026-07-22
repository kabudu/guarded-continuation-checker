use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelPropertyQuery, produce_btor2_channel_property_evidence,
    produce_btor2_channel_property_proof, verify_btor2_channel_property_evidence,
    verify_btor2_channel_property_proof,
};
use guarded_continuation_checker::btor2_search;

struct Fixture {
    channels: usize,
    bytes: &'static [u8],
    roots: &'static [u64],
}

fn queries(channels: usize, horizon: u32) -> Vec<Btor2ChannelPropertyQuery> {
    let mut queries = Vec::new();
    for property in [
        Btor2ChannelProperty::OutputHigh,
        Btor2ChannelProperty::OutputLow,
    ] {
        for channel in 0..channels {
            queries.push(Btor2ChannelPropertyQuery {
                query_id: queries.len() as u32,
                channel_index: channel,
                property,
                horizon,
            });
        }
    }
    queries
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_symbolic_property_portfolio_probe OUTPUT.csv")?;
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
    let mut rows = vec!["schema_version,channels,logical_queries,proof_members,reused_queries,direct_evidence_bytes,retained_evidence_bytes,evidence_reduction_pct,answers_agree,unsafe_assignments_replayed,horizon2_safe_refused,status".to_string()];
    for fixture in fixtures {
        let structural =
            encode_btor2_region_equivalence_artifact(&produce_btor2_region_equivalence_artifact(
                fixture.bytes,
                fixture.roots,
                fixture.channels,
                policy,
            )?)?;
        let queries = queries(fixture.channels, 1);
        let artifact =
            produce_btor2_channel_property_proof(fixture.bytes, &structural, &queries, policy)?;
        let summary =
            verify_btor2_channel_property_proof(fixture.bytes, &queries, &artifact, policy)?;
        let mut direct_results = Vec::new();
        let mut direct_evidence_bytes = 0usize;
        for query in &queries {
            let evidence = produce_btor2_channel_property_evidence(
                fixture.bytes,
                fixture.roots,
                fixture.channels,
                *query,
                policy,
            )?;
            direct_evidence_bytes += btor2_search::encode(&evidence.certificate)?.len();
            direct_results.push(verify_btor2_channel_property_evidence(
                fixture.bytes,
                fixture.roots,
                fixture.channels,
                &evidence,
                policy,
            )?);
        }
        let answers_agree =
            summary
                .results
                .iter()
                .zip(&direct_results)
                .all(|(portfolio, direct)| {
                    portfolio.result == direct.result && portfolio.bad_frame == direct.bad_frame
                });
        let unsafe_assignments_replayed = summary
            .results
            .iter()
            .filter(|result| result.result == btor2_search::SearchResult::Unsafe)
            .count();
        let retained_evidence_bytes = structural.len() + summary.metrics.evidence_bytes;
        let reduction =
            100.0 * (1.0 - retained_evidence_bytes as f64 / direct_evidence_bytes as f64);
        let horizon2_safe_refused = produce_btor2_channel_property_evidence(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            Btor2ChannelPropertyQuery {
                query_id: 0,
                channel_index: 0,
                property: Btor2ChannelProperty::OutputHigh,
                horizon: 2,
            },
            policy,
        )
        .is_err();
        rows.push(format!(
            "1,{},{},{},{},{direct_evidence_bytes},{retained_evidence_bytes},{reduction:.6},{answers_agree},{unsafe_assignments_replayed},{horizon2_safe_refused},accepted",
            fixture.channels,
            summary.metrics.logical_queries,
            summary.metrics.proof_members,
            summary.metrics.reused_logical_queries,
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("btor2_symbolic_property_portfolio_probe_v1=PASS rows=3 output={output}");
    Ok(())
}
