use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_family::{
    Btor2FamilyInstance, FamilyInputBinding, decode_btor2_family_artifact,
    verify_btor2_family_artifact,
};
use guarded_continuation_checker::btor2_family_orbit::{
    Btor2FamilyOrbitInput, encode_btor2_family_orbit_proof, produce_btor2_family_orbit_proof,
    verify_btor2_family_orbit_proof,
};
use guarded_continuation_checker::btor2_family_proof::{
    Btor2DirectQuery, Btor2FamilyProofPolicy, Btor2FamilyProofRoute, decode_btor2_family_proof,
    encode_btor2_family_proof_portfolio, produce_btor2_family_proof_portfolio,
    verify_btor2_family_proof_portfolio,
};
use sha2::{Digest, Sha256};

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(
            "usage: btor2_family_orbit_probe CORE.btor2 CHANNEL.btor2 PARAMETERS.txt OUTPUT.csv"
                .into(),
        );
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
    let mut rows = vec!["schema_version,trial,channels,logical_properties,representative_properties,logical_safe,logical_unsafe,orbit_artifact_bytes,direct_artifact_bytes,orbit_evidence_bytes,direct_evidence_bytes,orbit_produce_micros,direct_produce_micros,orbit_verify_micros,direct_verify_micros,orbit_sha256,direct_sha256,answers_equal,deterministic,process_scope,status".to_string()];
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
            let orbit_start = Instant::now();
            let orbit = produce_btor2_family_orbit_proof(
                Btor2FamilyOrbitInput {
                    core_bytes: &core,
                    core_roots: &[1000, 1001, 1002, 1003],
                    channel_bytes: &channel,
                    channel_roots: &[1000, 1001, 1002, 1003, 1004],
                    parameter_bytes: &parameters,
                    instances: &instances,
                    root_horizons: &[4, 4, 4, 4, 4],
                },
                policy,
            )?;
            let orbit_produce_micros = orbit_start.elapsed().as_micros();
            let orbit_bytes = encode_btor2_family_orbit_proof(&orbit, policy)?;
            let orbit_verify_start = Instant::now();
            let orbit_summary =
                verify_btor2_family_orbit_proof(&core, &channel, &parameters, &orbit, policy)?;
            let orbit_verify_micros = orbit_verify_start.elapsed().as_micros();

            let representative = decode_btor2_family_proof(&orbit.representative_proof, policy)?;
            let family =
                decode_btor2_family_artifact(&representative.family_artifact, policy.family())?;
            let composition = verify_btor2_family_artifact(
                &core,
                &channel,
                &parameters,
                &family,
                policy.family(),
            )?;
            let model = btor2::parse_bytes(&composition.expanded_model)?;
            let direct_queries = model
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
            let answers_equal = direct_summary
                .members
                .iter()
                .enumerate()
                .all(|(index, member)| {
                    let representative = &orbit_summary.representatives[index % 5];
                    member.result == representative.result
                        && member.bad_frame == representative.bad_frame
                        && member.query_horizon == representative.query_horizon
                });
            if !answers_equal
                || direct_summary.safe != orbit_summary.logical_safe
                || direct_summary.unsafe_count != orbit_summary.logical_unsafe
            {
                return Err("representative and direct answers disagree".into());
            }
            let orbit_sha256 = digest_hex(&orbit_bytes);
            let direct_sha256 = digest_hex(&direct_bytes);
            let deterministic = expected_hashes
                .entry(channels)
                .or_insert_with(|| (orbit_sha256.clone(), direct_sha256.clone()))
                == &(orbit_sha256.clone(), direct_sha256.clone());
            if !deterministic {
                return Err("orbit or direct artifact bytes changed between trials".into());
            }
            rows.push(format!(
                "1,{trial},{channels},{},{},{},{},{},{},{},{},{orbit_produce_micros},{direct_produce_micros},{orbit_verify_micros},{direct_verify_micros},{orbit_sha256},{direct_sha256},{answers_equal},{deterministic},single-process-release,accepted",
                orbit_summary.logical_queries,
                orbit_summary.representative_queries,
                orbit_summary.logical_safe,
                orbit_summary.logical_unsafe,
                orbit_bytes.len(),
                direct_bytes.len(),
                orbit_summary.evidence_bytes,
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
        "btor2_family_orbit_probe_v1=PASS rows=15 output={} core_sha256={} channel_sha256={} parameter_sha256={}",
        output.display(),
        digest_hex(&core),
        digest_hex(&channel),
        digest_hex(&parameters)
    );
    Ok(())
}
