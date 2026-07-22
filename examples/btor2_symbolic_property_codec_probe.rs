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
    Btor2ChannelProperty, Btor2ChannelPropertyProofPolicy, Btor2ChannelPropertyQuery,
    build_btor2_channel_property_model, decode_btor2_channel_property_proof_artifact,
    produce_btor2_channel_property_proof_bytes, verify_btor2_channel_property_proof_bytes,
};
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_symbolic_property_codec_probe OUTPUT.csv")?;
    let model = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
    );
    let roots = &[9, 39];
    let channels = 6;
    let horizon = 2;
    let region_policy = Btor2RegionPolicy::default();
    let artifact_policy = Btor2ChannelPropertyProofPolicy::default();
    let structural = encode_btor2_region_equivalence_artifact(
        &produce_btor2_region_equivalence_artifact(model, roots, channels, region_policy)?,
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
    let bytes = produce_btor2_channel_property_proof_bytes(
        model,
        &structural,
        &queries,
        region_policy,
        artifact_policy,
    )?;
    let artifact = decode_btor2_channel_property_proof_artifact(&bytes, artifact_policy)?;
    let summary = verify_btor2_channel_property_proof_bytes(
        model,
        &queries,
        &bytes,
        region_policy,
        artifact_policy,
    )?;
    let mut direct_evidence_bytes = 0usize;
    for query in &queries {
        let (property_model, bad) = build_btor2_channel_property_model(
            model,
            roots,
            channels,
            query.channel_index,
            query.property,
            region_policy,
        )?;
        direct_evidence_bytes += encode_btor2_bitblast_certificate(
            &produce_btor2_bitblast_certificate(&property_model, bad, query.horizon)?,
        )?
        .len();
    }
    let complete_delta = 100.0 * (bytes.len() as f64 / direct_evidence_bytes as f64 - 1.0);
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let row = format!(
        "schema_version,channels,horizon,logical_queries,proof_members,structural_bytes,evidence_bytes,artifact_bytes,direct_evidence_bytes,artifact_vs_direct_pct,artifact_sha256,roundtrip,verified,status\n1,{channels},{horizon},{},{},{},{},{},{direct_evidence_bytes},{complete_delta:.6},{digest},true,true,accepted\n",
        summary.metrics.logical_queries,
        summary.metrics.proof_members,
        artifact.structural_admission.len(),
        summary.metrics.evidence_bytes,
        bytes.len(),
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(row.as_bytes())?;
    file.sync_all()?;
    println!("btor2_symbolic_property_codec_probe=PASS rows=1 output={output}");
    Ok(())
}
