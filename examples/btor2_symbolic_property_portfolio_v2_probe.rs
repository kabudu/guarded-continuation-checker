use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::btor2_bitblast::{
    encode_btor2_bitblast_certificate, produce_btor2_bitblast_certificate,
};
use guarded_continuation_checker::btor2_region_equivalence::{
    encode_btor2_region_equivalence_artifact, produce_btor2_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;
use guarded_continuation_checker::btor2_region_property::{
    Btor2ChannelProperty, Btor2ChannelPropertyQuery, Btor2ChannelPropertySolver,
    build_btor2_channel_property_model, produce_btor2_channel_property_proof,
    verify_btor2_channel_property_proof,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_symbolic_property_portfolio_v2_probe OUTPUT.csv")?;
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let roots = &[9, 39];
    let channels = 6;
    let horizon = 2;
    let policy = Btor2RegionPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, roots, channels, policy)?,
    )?;
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
    let artifact = produce_btor2_channel_property_proof(model, &structural, &queries, policy)?;
    let summary = verify_btor2_channel_property_proof(model, &queries, &artifact, policy)?;
    if !artifact
        .members
        .iter()
        .all(|member| member.solver == Btor2ChannelPropertySolver::BitblastCnf)
    {
        return Err("portfolio did not statically select bitblast".into());
    }
    let mut direct_evidence_bytes = 0usize;
    for query in &queries {
        let (property_model, bad) = build_btor2_channel_property_model(
            model,
            roots,
            channels,
            query.channel_index,
            query.property,
            policy,
        )?;
        direct_evidence_bytes += encode_btor2_bitblast_certificate(
            &produce_btor2_bitblast_certificate(&property_model, bad, horizon)?,
        )?
        .len();
    }
    let retained_evidence_bytes = structural.len() + summary.metrics.evidence_bytes;
    let reduction = 100.0 * (1.0 - retained_evidence_bytes as f64 / direct_evidence_bytes as f64);
    let high_frame_two = summary.results[..channels]
        .iter()
        .all(|result| result.bad_frame == Some(2) && result.witness_valuations.len() == 3);
    let low_frame_zero = summary.results[channels..]
        .iter()
        .all(|result| result.bad_frame == Some(0) && result.witness_valuations.len() == 1);
    let row = format!(
        "schema_version,channels,horizon,logical_queries,proof_members,reused_queries,explicit_members,bitblast_members,direct_evidence_bytes,retained_evidence_bytes,evidence_reduction_pct,high_frame_two,low_frame_zero,verified,status\n1,{channels},{horizon},{},{},{},{},{},{direct_evidence_bytes},{retained_evidence_bytes},{reduction:.6},{high_frame_two},{low_frame_zero},true,accepted\n",
        summary.metrics.logical_queries,
        summary.metrics.proof_members,
        summary.metrics.reused_logical_queries,
        summary.metrics.explicit_state_members,
        summary.metrics.bitblast_members,
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(row.as_bytes())?;
    file.sync_all()?;
    println!("btor2_symbolic_property_portfolio_v2_probe=PASS rows=1 output={output}");
    Ok(())
}
