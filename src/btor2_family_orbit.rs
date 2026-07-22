//! Representative proof reuse for structurally identical BTOR2 channel orbits.

use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::btor2_family::{Btor2FamilyArtifact, Btor2FamilyInstance, decode_btor2_family_artifact};
use crate::btor2_family_proof::{
    Btor2FamilyProofArtifact, Btor2FamilyProofInput, Btor2FamilyProofPolicy,
    Btor2FamilyProofSummary, Btor2FamilyQuery, decode_btor2_family_proof,
    encode_btor2_family_proof, produce_btor2_family_proof, verify_btor2_family_proof,
};
use crate::btor2_search::SearchSummary;

pub const BTOR2_FAMILY_ORBIT_PROOF_VERSION: u32 = 1;
const MAGIC: &[u8; 8] = b"GCCBOP01";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyOrbitProofArtifact {
    pub version: u32,
    pub representative_proof: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Btor2FamilyOrbitInput<'a> {
    pub core_bytes: &'a [u8],
    pub core_roots: &'a [u64],
    pub channel_bytes: &'a [u8],
    pub channel_roots: &'a [u64],
    pub parameter_bytes: &'a [u8],
    pub instances: &'a [Btor2FamilyInstance],
    pub root_horizons: &'a [u32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyOrbitProofSummary {
    pub version: u32,
    pub expanded_sha256: [u8; 32],
    pub instances: usize,
    pub representative_queries: usize,
    pub logical_queries: usize,
    pub representative_safe: usize,
    pub representative_unsafe: usize,
    pub logical_safe: usize,
    pub logical_unsafe: usize,
    pub evidence_bytes: usize,
    pub representatives: Vec<SearchSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyOrbitError(pub String);

impl fmt::Display for Btor2FamilyOrbitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Btor2FamilyOrbitError {}

fn reject(message: impl Into<String>) -> Btor2FamilyOrbitError {
    Btor2FamilyOrbitError(message.into())
}

fn ensure_one_orbit(
    artifact: &Btor2FamilyArtifact,
    proof: &Btor2FamilyProofArtifact,
) -> Result<(), Btor2FamilyOrbitError> {
    if artifact.instances.len() < 2 {
        return Err(reject("orbit proof requires at least two family instances"));
    }
    let representative = &artifact.instances[0];
    if artifact.instances[1..].iter().any(|instance| {
        instance.parameter_sha256 != representative.parameter_sha256
            || instance.input_bindings != representative.input_bindings
    }) {
        return Err(reject(
            "orbit proof requires identical parameters and core bindings",
        ));
    }
    if proof.members.len() != artifact.channel_roots.len()
        || proof
            .members
            .iter()
            .enumerate()
            .any(|(root, member)| member.property_index != root)
    {
        return Err(reject(
            "orbit proof requires one ordered representative for every channel root",
        ));
    }
    Ok(())
}

fn summary(
    instances: usize,
    checked: Btor2FamilyProofSummary,
) -> Result<Btor2FamilyOrbitProofSummary, Btor2FamilyOrbitError> {
    let logical_queries = checked
        .queries
        .checked_mul(instances)
        .ok_or_else(|| reject("orbit logical query count overflow"))?;
    let logical_safe = checked
        .safe
        .checked_mul(instances)
        .ok_or_else(|| reject("orbit SAFE count overflow"))?;
    let logical_unsafe = checked
        .unsafe_count
        .checked_mul(instances)
        .ok_or_else(|| reject("orbit UNSAFE count overflow"))?;
    Ok(Btor2FamilyOrbitProofSummary {
        version: BTOR2_FAMILY_ORBIT_PROOF_VERSION,
        expanded_sha256: checked.expanded_sha256,
        instances,
        representative_queries: checked.queries,
        logical_queries,
        representative_safe: checked.safe,
        representative_unsafe: checked.unsafe_count,
        logical_safe,
        logical_unsafe,
        evidence_bytes: checked.evidence_bytes,
        representatives: checked.members,
    })
}

pub fn produce_btor2_family_orbit_proof(
    input: Btor2FamilyOrbitInput<'_>,
    policy: Btor2FamilyProofPolicy,
) -> Result<Btor2FamilyOrbitProofArtifact, Btor2FamilyOrbitError> {
    if input.root_horizons.len() != input.channel_roots.len() {
        return Err(reject("orbit horizon count does not match channel roots"));
    }
    let queries = input
        .root_horizons
        .iter()
        .enumerate()
        .map(|(property_index, horizon)| Btor2FamilyQuery {
            property_index,
            horizon: *horizon,
        })
        .collect::<Vec<_>>();
    let (proof, _) = produce_btor2_family_proof(
        Btor2FamilyProofInput {
            core_bytes: input.core_bytes,
            core_roots: input.core_roots,
            channel_bytes: input.channel_bytes,
            channel_roots: input.channel_roots,
            parameter_bytes: input.parameter_bytes,
            instances: input.instances,
            queries: &queries,
        },
        policy,
    )
    .map_err(|error| reject(error.to_string()))?;
    let family = decode_btor2_family_artifact(&proof.family_artifact, policy.family())
        .map_err(|error| reject(error.to_string()))?;
    ensure_one_orbit(&family, &proof)?;
    let artifact = Btor2FamilyOrbitProofArtifact {
        version: BTOR2_FAMILY_ORBIT_PROOF_VERSION,
        representative_proof: encode_btor2_family_proof(&proof, policy)
            .map_err(|error| reject(error.to_string()))?,
    };
    let _ = encode_btor2_family_orbit_proof(&artifact, policy)?;
    Ok(artifact)
}

pub fn verify_btor2_family_orbit_proof(
    core_bytes: &[u8],
    channel_bytes: &[u8],
    parameter_bytes: &[u8],
    artifact: &Btor2FamilyOrbitProofArtifact,
    policy: Btor2FamilyProofPolicy,
) -> Result<Btor2FamilyOrbitProofSummary, Btor2FamilyOrbitError> {
    let _ = encode_btor2_family_orbit_proof(artifact, policy)?;
    let proof = decode_btor2_family_proof(&artifact.representative_proof, policy)
        .map_err(|error| reject(error.to_string()))?;
    let family = decode_btor2_family_artifact(&proof.family_artifact, policy.family())
        .map_err(|error| reject(error.to_string()))?;
    ensure_one_orbit(&family, &proof)?;
    let checked =
        verify_btor2_family_proof(core_bytes, channel_bytes, parameter_bytes, &proof, policy)
            .map_err(|error| reject(error.to_string()))?;
    summary(family.instances.len(), checked)
}

pub fn encode_btor2_family_orbit_proof(
    artifact: &Btor2FamilyOrbitProofArtifact,
    policy: Btor2FamilyProofPolicy,
) -> Result<Vec<u8>, Btor2FamilyOrbitError> {
    if artifact.version != BTOR2_FAMILY_ORBIT_PROOF_VERSION
        || artifact.representative_proof.is_empty()
        || artifact.representative_proof.len() > policy.max_artifact_bytes()
    {
        return Err(reject("BTOR2 family orbit proof is outside policy"));
    }
    let length = u32::try_from(artifact.representative_proof.len())
        .map_err(|_| reject("orbit representative proof length exceeds range"))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&artifact.representative_proof);
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > policy.max_artifact_bytes() {
        return Err(reject("BTOR2 family orbit proof exceeds byte policy"));
    }
    Ok(bytes)
}

pub fn decode_btor2_family_orbit_proof(
    bytes: &[u8],
    policy: Btor2FamilyProofPolicy,
) -> Result<Btor2FamilyOrbitProofArtifact, Btor2FamilyOrbitError> {
    if bytes.len() < 8 + 4 + 4 + 32 || bytes.len() > policy.max_artifact_bytes() {
        return Err(reject("BTOR2 family orbit proof size is outside policy"));
    }
    let payload_end = bytes.len() - 32;
    let expected: [u8; 32] = bytes[payload_end..].try_into().expect("fixed suffix");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_end])) != expected {
        return Err(reject("BTOR2 family orbit proof checksum mismatch"));
    }
    if &bytes[..8] != MAGIC {
        return Err(reject("BTOR2 family orbit proof magic mismatch"));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().expect("fixed length"));
    let length = usize::try_from(u32::from_le_bytes(
        bytes[12..16].try_into().expect("fixed length"),
    ))
    .map_err(|_| reject("orbit representative proof length exceeds range"))?;
    if length == 0
        || length > policy.max_artifact_bytes()
        || 16usize.checked_add(length) != Some(payload_end)
    {
        return Err(reject("orbit representative proof length mismatch"));
    }
    let artifact = Btor2FamilyOrbitProofArtifact {
        version,
        representative_proof: bytes[16..payload_end].to_vec(),
    };
    if encode_btor2_family_orbit_proof(&artifact, policy)? != bytes {
        return Err(reject("BTOR2 family orbit proof is not canonical"));
    }
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btor2_family::FamilyInputBinding;

    const CORE: &[u8] = br#"1 sort bitvec 1
2 input 1 enable
3 state 1 phase
4 zero 1
5 init 1 3 4
6 xor 1 3 2
7 next 1 3 6
8 output 3 phase_out
"#;
    const CHANNEL: &[u8] = br#"1 sort bitvec 1
2 input 1 phase
3 input 1 enable
4 state 1 pulse
5 zero 1
6 init 1 4 5
7 and 1 2 3
8 next 1 4 7
9 xor 1 4 2
10 output 5 safe
11 output 9 mismatch
"#;
    const PARAMETERS: &[u8] = b"width=1\n";

    fn instances(count: usize) -> Vec<Btor2FamilyInstance> {
        (0..count)
            .map(|index| Btor2FamilyInstance {
                identifier: format!("channel{index}"),
                parameter_sha256: Sha256::digest(PARAMETERS).into(),
                input_bindings: vec![
                    FamilyInputBinding::CoreRoot(0),
                    FamilyInputBinding::CoreInput(0),
                ],
            })
            .collect()
    }

    #[test]
    fn one_representative_per_root_proves_every_identical_instance() {
        let policy = Btor2FamilyProofPolicy::default();
        let artifact = produce_btor2_family_orbit_proof(
            Btor2FamilyOrbitInput {
                core_bytes: CORE,
                core_roots: &[3],
                channel_bytes: CHANNEL,
                channel_roots: &[5, 9],
                parameter_bytes: PARAMETERS,
                instances: &instances(4),
                root_horizons: &[2, 2],
            },
            policy,
        )
        .unwrap();
        let bytes = encode_btor2_family_orbit_proof(&artifact, policy).unwrap();
        let decoded = decode_btor2_family_orbit_proof(&bytes, policy).unwrap();
        let summary =
            verify_btor2_family_orbit_proof(CORE, CHANNEL, PARAMETERS, &decoded, policy).unwrap();
        assert_eq!(summary.instances, 4);
        assert_eq!(summary.representative_queries, 2);
        assert_eq!(summary.logical_queries, 8);
        assert_eq!(summary.representative_safe, 1);
        assert_eq!(summary.representative_unsafe, 1);
        assert_eq!(summary.logical_safe, 4);
        assert_eq!(summary.logical_unsafe, 4);
    }

    #[test]
    fn distinct_bindings_and_incomplete_roots_refuse_orbit_reuse() {
        let policy = Btor2FamilyProofPolicy::default();
        let mut different = instances(2);
        different[1].input_bindings.swap(0, 1);
        assert!(
            produce_btor2_family_orbit_proof(
                Btor2FamilyOrbitInput {
                    core_bytes: CORE,
                    core_roots: &[3],
                    channel_bytes: CHANNEL,
                    channel_roots: &[5, 9],
                    parameter_bytes: PARAMETERS,
                    instances: &different,
                    root_horizons: &[2, 2],
                },
                policy,
            )
            .is_err()
        );
        assert!(
            produce_btor2_family_orbit_proof(
                Btor2FamilyOrbitInput {
                    core_bytes: CORE,
                    core_roots: &[3],
                    channel_bytes: CHANNEL,
                    channel_roots: &[5, 9],
                    parameter_bytes: PARAMETERS,
                    instances: &instances(2),
                    root_horizons: &[2],
                },
                policy,
            )
            .is_err()
        );
    }

    #[test]
    fn source_drift_mutation_and_truncation_fail_closed() {
        let policy = Btor2FamilyProofPolicy::default();
        let artifact = produce_btor2_family_orbit_proof(
            Btor2FamilyOrbitInput {
                core_bytes: CORE,
                core_roots: &[3],
                channel_bytes: CHANNEL,
                channel_roots: &[5, 9],
                parameter_bytes: PARAMETERS,
                instances: &instances(2),
                root_horizons: &[2, 2],
            },
            policy,
        )
        .unwrap();
        assert!(
            verify_btor2_family_orbit_proof(CORE, CHANNEL, b"width=2\n", &artifact, policy,)
                .is_err()
        );
        let bytes = encode_btor2_family_orbit_proof(&artifact, policy).unwrap();
        for end in 0..bytes.len() {
            assert!(decode_btor2_family_orbit_proof(&bytes[..end], policy).is_err());
        }
        for offset in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[offset] ^= 1;
            assert!(decode_btor2_family_orbit_proof(&changed, policy).is_err());
        }
    }
}
