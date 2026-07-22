use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelPropertyProductionPolicy, Btor2ChannelPropertyProofPolicy,
    Btor2ChannelPropertyQuery, preflight_btor2_channel_property_proof,
};

struct Fixture {
    channels: usize,
    bytes: &'static [u8],
    roots: &'static [u64],
}

fn queries(channels: usize, horizon: u32) -> Vec<Btor2ChannelPropertyQuery> {
    let mut queries = Vec::with_capacity(channels * 2);
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
        .ok_or("usage: btor2_symbolic_property_preflight_probe OUTPUT.csv")?;
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
    let region_policy = Btor2RegionPolicy::default();
    let artifact_policy = Btor2ChannelPropertyProofPolicy::default();
    let mut rows = vec!["schema_version,channels,horizon,logical_queries,proof_members,explicit_members,bitblast_members,projected_work,tighter_limit_refused,exact_limit_admitted,status".to_string()];
    for fixture in fixtures {
        let structural =
            encode_btor2_region_equivalence_artifact(&produce_btor2_region_equivalence_artifact(
                fixture.bytes,
                fixture.roots,
                fixture.channels,
                region_policy,
            )?)?;
        for horizon in [1, 2] {
            let queries = queries(fixture.channels, horizon);
            let plan = preflight_btor2_channel_property_proof(
                fixture.bytes,
                &structural,
                &queries,
                region_policy,
                Btor2ChannelPropertyProductionPolicy::default(),
            )?;
            let tighter_limit = plan
                .projected_work
                .checked_sub(1)
                .ok_or("preflight projected no work")?;
            let policy = Btor2ChannelPropertyProductionPolicy::new(artifact_policy, tighter_limit)?;
            let tighter_limit_refused = preflight_btor2_channel_property_proof(
                fixture.bytes,
                &structural,
                &queries,
                region_policy,
                policy,
            )
            .is_err();
            let exact_policy =
                Btor2ChannelPropertyProductionPolicy::new(artifact_policy, plan.projected_work)?;
            let exact_limit_admitted = preflight_btor2_channel_property_proof(
                fixture.bytes,
                &structural,
                &queries,
                region_policy,
                exact_policy,
            )? == plan;
            rows.push(format!(
                "1,{},{},{},{},{},{},{},{tighter_limit_refused},{exact_limit_admitted},accepted",
                fixture.channels,
                horizon,
                plan.logical_queries,
                plan.proof_members,
                plan.explicit_state_members,
                plan.bitblast_members,
                plan.projected_work,
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
    println!(
        "btor2_symbolic_property_preflight_probe=PASS rows={} output={output}",
        rows.len() - 1
    );
    Ok(())
}
