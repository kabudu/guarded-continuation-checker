use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_family::{Btor2FamilyInstance, FamilyInputBinding};
use guarded_continuation_checker::btor2_family_proof::{
    Btor2DirectQuery, Btor2FamilyProofInput, Btor2FamilyProofPolicy, Btor2FamilyProofRoute,
    Btor2FamilyQuery, encode_btor2_family_proof_portfolio, produce_btor2_family_proof_portfolio,
    verify_btor2_family_proof_portfolio,
};
use sha2::{Digest, Sha256};

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn usage() -> &'static str {
    "usage: cargo run --release --example btor2_family_probe -- CORE.btor2 CHANNEL.btor2 PARAMETERS.txt OUTPUT.csv"
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(usage().into());
    }
    let core = fs::read(&arguments[0])?;
    let channel = fs::read(&arguments[1])?;
    let parameters = fs::read(&arguments[2])?;
    let output = Path::new(&arguments[3]);
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }
    let parameter_sha256: [u8; 32] = Sha256::digest(&parameters).into();
    let policy = Btor2FamilyProofPolicy::default();
    let mut rows = Vec::new();
    rows.push("schema_version,trial,channels,properties,safe,unsafe,family_model_artifact_bytes,expanded_model_bytes,family_portfolio_bytes,direct_portfolio_bytes,family_produce_micros,direct_produce_micros,family_verify_micros,direct_verify_micros,family_evidence_bytes,direct_evidence_bytes,family_sha256,direct_sha256,answers_equal,deterministic,process_scope,status".to_string());
    let mut expected_hashes = std::collections::BTreeMap::new();

    for channels in [2usize, 4, 6] {
        for trial in 1..=5 {
            let instances = (0..channels)
                .map(|index| Btor2FamilyInstance {
                    identifier: format!("channel{index}"),
                    parameter_sha256,
                    input_bindings: vec![
                        FamilyInputBinding::CoreRoot(0),
                        FamilyInputBinding::CoreRoot(1),
                        FamilyInputBinding::CoreRoot(2),
                        FamilyInputBinding::CoreRoot(3),
                    ],
                })
                .collect::<Vec<_>>();
            let queries = (0..channels * 5)
                .map(|property_index| Btor2FamilyQuery {
                    property_index,
                    horizon: 4,
                })
                .collect::<Vec<_>>();
            let family_start = Instant::now();
            let family = produce_btor2_family_proof_portfolio(
                Btor2FamilyProofRoute::Family(Btor2FamilyProofInput {
                    core_bytes: &core,
                    core_roots: &[1000, 1001, 1002, 1003],
                    channel_bytes: &channel,
                    channel_roots: &[1000, 1001, 1002, 1003, 1004],
                    parameter_bytes: &parameters,
                    instances: &instances,
                    queries: &queries,
                }),
                policy,
            )?;
            let family_produce_micros = family_start.elapsed().as_micros();
            let family_bytes = encode_btor2_family_proof_portfolio(&family, policy)?;

            let family_verify_start = Instant::now();
            let family_summary = verify_btor2_family_proof_portfolio(
                &core,
                &channel,
                &parameters,
                b"unused-on-family-route",
                &family,
                policy,
            )?;
            let family_verify_micros = family_verify_start.elapsed().as_micros();

            let family_proof =
                guarded_continuation_checker::btor2_family_proof::decode_btor2_family_proof(
                    &family.payload,
                    policy,
                )?;
            let family_model =
                guarded_continuation_checker::btor2_family::decode_btor2_family_artifact(
                    &family_proof.family_artifact,
                    policy.family(),
                )?;
            let composition =
                guarded_continuation_checker::btor2_family::verify_btor2_family_artifact(
                    &core,
                    &channel,
                    &parameters,
                    &family_model,
                    policy.family(),
                )?;
            let parsed = btor2::parse_bytes(&composition.expanded_model)?;
            let direct_queries = parsed
                .bad_properties()
                .iter()
                .map(|(bad_property, _, _)| Btor2DirectQuery {
                    bad_property: *bad_property,
                    horizon: 4,
                })
                .collect::<Vec<_>>();

            let direct_start = Instant::now();
            let direct = produce_btor2_family_proof_portfolio(
                Btor2FamilyProofRoute::ExactFallback {
                    model_bytes: &composition.expanded_model,
                    queries: &direct_queries,
                },
                policy,
            )?;
            let direct_produce_micros = direct_start.elapsed().as_micros();
            let direct_bytes = encode_btor2_family_proof_portfolio(&direct, policy)?;
            let direct_verify_start = Instant::now();
            let direct_summary = verify_btor2_family_proof_portfolio(
                &core,
                &channel,
                &parameters,
                &composition.expanded_model,
                &direct,
                policy,
            )?;
            let direct_verify_micros = direct_verify_start.elapsed().as_micros();
            let answers_equal = family_summary.members == direct_summary.members;
            if !answers_equal {
                return Err("family and direct summaries disagree".into());
            }
            let family_sha256 = digest_hex(&family_bytes);
            let direct_sha256 = digest_hex(&direct_bytes);
            let deterministic = expected_hashes
                .entry(channels)
                .or_insert_with(|| (family_sha256.clone(), direct_sha256.clone()))
                == &(family_sha256.clone(), direct_sha256.clone());
            if !deterministic {
                return Err("artifact bytes changed between trials".into());
            }
            rows.push(format!(
                "1,{trial},{channels},{},{},{},{},{},{},{},{family_produce_micros},{direct_produce_micros},{family_verify_micros},{direct_verify_micros},{},{},{family_sha256},{direct_sha256},{answers_equal},{deterministic},single-process-release,accepted",
                family_summary.queries,
                family_summary.safe,
                family_summary.unsafe_count,
                family_proof.family_artifact.len(),
                composition.expanded_model.len(),
                family_bytes.len(),
                direct_bytes.len(),
                family_summary.evidence_bytes,
                direct_summary.evidence_bytes,
            ));
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!(
        "btor2_family_probe_v1=PASS rows=15 output={} core_sha256={} channel_sha256={} parameter_sha256={}",
        output.display(),
        digest_hex(&core),
        digest_hex(&channel),
        digest_hex(&parameters)
    );
    Ok(())
}
