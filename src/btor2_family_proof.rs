//! Exact both-answer evidence over source-bound BTOR2 channel families.

use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::btor2;
use crate::btor2_family::{
    Btor2FamilyArtifact, Btor2FamilyComposition, Btor2FamilyError, Btor2FamilyInstance,
    Btor2FamilyPolicy, decode_btor2_family_artifact, encode_btor2_family_artifact,
    produce_btor2_family_artifact, verify_btor2_family_artifact,
};
use crate::btor2_search::{
    self, MAX_SEARCH_CERTIFICATE_BYTES, MAX_SEARCH_HORIZON, SearchResult, SearchSummary,
};

pub const BTOR2_FAMILY_PROOF_VERSION: u32 = 1;
pub const MAX_FAMILY_PROOF_QUERIES: usize = 256;
pub const MAX_FAMILY_PROOF_EVIDENCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_FAMILY_PROOF_ARTIFACT_BYTES: usize = 65 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"GCCBFP01";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2FamilyQuery {
    pub property_index: usize,
    pub horizon: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Btor2FamilyProofInput<'a> {
    pub core_bytes: &'a [u8],
    pub core_roots: &'a [u64],
    pub channel_bytes: &'a [u8],
    pub channel_roots: &'a [u64],
    pub parameter_bytes: &'a [u8],
    pub instances: &'a [Btor2FamilyInstance],
    pub queries: &'a [Btor2FamilyQuery],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyProofMember {
    pub property_index: usize,
    pub horizon: u32,
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyProofArtifact {
    pub version: u32,
    pub family_artifact: Vec<u8>,
    pub members: Vec<Btor2FamilyProofMember>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2FamilyProofPolicy {
    family: Btor2FamilyPolicy,
    max_queries: usize,
    max_evidence_bytes: usize,
    max_artifact_bytes: usize,
}

impl Btor2FamilyProofPolicy {
    pub fn new(
        family: Btor2FamilyPolicy,
        max_queries: usize,
        max_evidence_bytes: usize,
        max_artifact_bytes: usize,
    ) -> Result<Self, Btor2FamilyProofError> {
        if max_queries == 0
            || max_queries > MAX_FAMILY_PROOF_QUERIES
            || max_evidence_bytes == 0
            || max_evidence_bytes > MAX_FAMILY_PROOF_EVIDENCE_BYTES
            || max_artifact_bytes == 0
            || max_artifact_bytes > MAX_FAMILY_PROOF_ARTIFACT_BYTES
        {
            return Err(reject("BTOR2 family proof policy is outside static limits"));
        }
        Ok(Self {
            family,
            max_queries,
            max_evidence_bytes,
            max_artifact_bytes,
        })
    }

    pub fn family(self) -> Btor2FamilyPolicy {
        self.family
    }

    pub fn max_queries(self) -> usize {
        self.max_queries
    }

    pub fn max_evidence_bytes(self) -> usize {
        self.max_evidence_bytes
    }

    pub fn max_artifact_bytes(self) -> usize {
        self.max_artifact_bytes
    }
}

impl Default for Btor2FamilyProofPolicy {
    fn default() -> Self {
        Self {
            family: Btor2FamilyPolicy::default(),
            max_queries: MAX_FAMILY_PROOF_QUERIES,
            max_evidence_bytes: MAX_FAMILY_PROOF_EVIDENCE_BYTES,
            max_artifact_bytes: MAX_FAMILY_PROOF_ARTIFACT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyProofSummary {
    pub version: u32,
    pub expanded_sha256: [u8; 32],
    pub queries: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub evidence_bytes: usize,
    pub members: Vec<SearchSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyProofError(pub String);

impl fmt::Display for Btor2FamilyProofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Btor2FamilyProofError {}

impl From<Btor2FamilyError> for Btor2FamilyProofError {
    fn from(error: Btor2FamilyError) -> Self {
        Self(error.to_string())
    }
}

fn reject(message: impl Into<String>) -> Btor2FamilyProofError {
    Btor2FamilyProofError(message.into())
}

fn validate_queries(
    queries: &[Btor2FamilyQuery],
    property_count: usize,
    policy: Btor2FamilyProofPolicy,
) -> Result<(), Btor2FamilyProofError> {
    if queries.is_empty() || queries.len() > policy.max_queries {
        return Err(reject("BTOR2 family proof query count is outside policy"));
    }
    if queries
        .iter()
        .any(|query| query.property_index >= property_count || query.horizon > MAX_SEARCH_HORIZON)
        || queries
            .windows(2)
            .any(|pair| pair[0].property_index >= pair[1].property_index)
    {
        return Err(reject(
            "BTOR2 family proof queries must be valid and strictly property ordered",
        ));
    }
    Ok(())
}

fn checked_evidence_bytes(
    members: &[Btor2FamilyProofMember],
    policy: Btor2FamilyProofPolicy,
) -> Result<usize, Btor2FamilyProofError> {
    members.iter().try_fold(0usize, |total, member| {
        if member.evidence.is_empty()
            || member.evidence.len() > MAX_SEARCH_CERTIFICATE_BYTES
            || member.horizon > MAX_SEARCH_HORIZON
        {
            return Err(reject("BTOR2 family proof member is outside static limits"));
        }
        total
            .checked_add(member.evidence.len())
            .filter(|value| *value <= policy.max_evidence_bytes)
            .ok_or_else(|| reject("BTOR2 family proof evidence exceeds policy"))
    })
}

fn property_ids(composition: &Btor2FamilyComposition) -> Result<Vec<u64>, Btor2FamilyProofError> {
    let model = btor2::parse_bytes(&composition.expanded_model)
        .map_err(|error| reject(format!("invalid reconstructed family model: {error}")))?;
    Ok(model
        .bad_properties()
        .iter()
        .map(|(identifier, _, _)| *identifier)
        .collect())
}

pub fn produce_btor2_family_proof(
    input: Btor2FamilyProofInput<'_>,
    policy: Btor2FamilyProofPolicy,
) -> Result<(Btor2FamilyProofArtifact, Btor2FamilyComposition), Btor2FamilyProofError> {
    let (family, composition) = produce_btor2_family_artifact(
        input.core_bytes,
        input.core_roots,
        input.channel_bytes,
        input.channel_roots,
        input.parameter_bytes,
        input.instances,
        policy.family,
    )?;
    let properties = property_ids(&composition)?;
    validate_queries(input.queries, properties.len(), policy)?;
    let mut members = Vec::with_capacity(input.queries.len());
    for query in input.queries {
        let certificate = btor2_search::produce(
            &composition.expanded_model,
            properties[query.property_index],
            query.horizon,
        )
        .map_err(|error| reject(format!("family proof production failed: {error}")))?;
        let evidence = btor2_search::encode(&certificate)
            .map_err(|error| reject(format!("family proof encoding failed: {error}")))?
            .into_bytes();
        members.push(Btor2FamilyProofMember {
            property_index: query.property_index,
            horizon: query.horizon,
            evidence,
        });
    }
    checked_evidence_bytes(&members, policy)?;
    let artifact = Btor2FamilyProofArtifact {
        version: BTOR2_FAMILY_PROOF_VERSION,
        family_artifact: encode_btor2_family_artifact(&family, policy.family)?,
        members,
    };
    let _ = encode_btor2_family_proof(&artifact, policy)?;
    Ok((artifact, composition))
}

fn verify_members(
    composition: &Btor2FamilyComposition,
    artifact: &Btor2FamilyProofArtifact,
    policy: Btor2FamilyProofPolicy,
) -> Result<Btor2FamilyProofSummary, Btor2FamilyProofError> {
    if artifact.version != BTOR2_FAMILY_PROOF_VERSION
        || artifact.members.is_empty()
        || artifact.members.len() > policy.max_queries
        || artifact
            .members
            .windows(2)
            .any(|pair| pair[0].property_index >= pair[1].property_index)
    {
        return Err(reject("BTOR2 family proof member table is non-canonical"));
    }
    let evidence_bytes = checked_evidence_bytes(&artifact.members, policy)?;
    let properties = property_ids(composition)?;
    let queries = artifact
        .members
        .iter()
        .map(|member| Btor2FamilyQuery {
            property_index: member.property_index,
            horizon: member.horizon,
        })
        .collect::<Vec<_>>();
    validate_queries(&queries, properties.len(), policy)?;

    let mut summaries = Vec::with_capacity(artifact.members.len());
    for member in &artifact.members {
        let certificate = btor2_search::decode(&member.evidence)
            .map_err(|error| reject(format!("invalid family proof member: {error}")))?;
        if certificate.bad_property != properties[member.property_index]
            || certificate.query_horizon != member.horizon
        {
            return Err(reject("BTOR2 family proof query binding mismatch"));
        }
        summaries.push(
            btor2_search::verify(&composition.expanded_model, &certificate)
                .map_err(|error| reject(format!("family proof verification failed: {error}")))?,
        );
    }
    let safe = summaries
        .iter()
        .filter(|summary| summary.result == SearchResult::Safe)
        .count();
    Ok(Btor2FamilyProofSummary {
        version: BTOR2_FAMILY_PROOF_VERSION,
        expanded_sha256: composition.expanded_sha256,
        queries: summaries.len(),
        safe,
        unsafe_count: summaries.len() - safe,
        evidence_bytes,
        members: summaries,
    })
}

pub fn verify_btor2_family_proof(
    core_bytes: &[u8],
    channel_bytes: &[u8],
    parameter_bytes: &[u8],
    artifact: &Btor2FamilyProofArtifact,
    policy: Btor2FamilyProofPolicy,
) -> Result<Btor2FamilyProofSummary, Btor2FamilyProofError> {
    let _ = encode_btor2_family_proof(artifact, policy)?;
    let family: Btor2FamilyArtifact =
        decode_btor2_family_artifact(&artifact.family_artifact, policy.family)?;
    let composition = verify_btor2_family_artifact(
        core_bytes,
        channel_bytes,
        parameter_bytes,
        &family,
        policy.family,
    )?;
    verify_members(&composition, artifact, policy)
}

fn push_u32(bytes: &mut Vec<u8>, value: usize, label: &str) -> Result<(), Btor2FamilyProofError> {
    let value = u32::try_from(value).map_err(|_| reject(format!("{label} exceeds range")))?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

pub fn encode_btor2_family_proof(
    artifact: &Btor2FamilyProofArtifact,
    policy: Btor2FamilyProofPolicy,
) -> Result<Vec<u8>, Btor2FamilyProofError> {
    if artifact.version != BTOR2_FAMILY_PROOF_VERSION
        || artifact.family_artifact.is_empty()
        || artifact.family_artifact.len() > policy.family.max_artifact_bytes()
        || artifact.members.is_empty()
        || artifact.members.len() > policy.max_queries
        || artifact
            .members
            .windows(2)
            .any(|pair| pair[0].property_index >= pair[1].property_index)
    {
        return Err(reject("BTOR2 family proof artifact is non-canonical"));
    }
    checked_evidence_bytes(&artifact.members, policy)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    push_u32(
        &mut bytes,
        artifact.family_artifact.len(),
        "family artifact length",
    )?;
    bytes.extend_from_slice(&artifact.family_artifact);
    push_u32(
        &mut bytes,
        artifact.members.len(),
        "family proof member count",
    )?;
    for member in &artifact.members {
        push_u32(&mut bytes, member.property_index, "property index")?;
        bytes.extend_from_slice(&member.horizon.to_le_bytes());
        push_u32(&mut bytes, member.evidence.len(), "proof member length")?;
        bytes.extend_from_slice(&member.evidence);
    }
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > policy.max_artifact_bytes {
        return Err(reject("BTOR2 family proof artifact exceeds byte policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Btor2FamilyProofError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject("truncated BTOR2 family proof"))?;
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn u32(&mut self) -> Result<u32, Btor2FamilyProofError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed length"),
        ))
    }
}

fn count(value: u32, maximum: usize, label: &str) -> Result<usize, Btor2FamilyProofError> {
    let value = usize::try_from(value).map_err(|_| reject(format!("invalid {label}")))?;
    if value == 0 || value > maximum {
        return Err(reject(format!("{label} is outside policy")));
    }
    Ok(value)
}

pub fn decode_btor2_family_proof(
    bytes: &[u8],
    policy: Btor2FamilyProofPolicy,
) -> Result<Btor2FamilyProofArtifact, Btor2FamilyProofError> {
    if bytes.len() < 8 + 4 + 4 + 4 + 32 || bytes.len() > policy.max_artifact_bytes {
        return Err(reject("BTOR2 family proof size is outside policy"));
    }
    let payload_len = bytes.len() - 32;
    let expected: [u8; 32] = bytes[payload_len..].try_into().expect("fixed suffix");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_len])) != expected {
        return Err(reject("BTOR2 family proof checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..payload_len],
        offset: 0,
    };
    if cursor.take(8)? != MAGIC {
        return Err(reject("BTOR2 family proof magic mismatch"));
    }
    let version = cursor.u32()?;
    let family_len = count(
        cursor.u32()?,
        policy.family.max_artifact_bytes(),
        "family artifact length",
    )?;
    let family_artifact = cursor.take(family_len)?.to_vec();
    let member_count = count(
        cursor.u32()?,
        policy.max_queries,
        "family proof member count",
    )?;
    let mut members = Vec::with_capacity(member_count);
    let mut evidence_bytes = 0usize;
    for _ in 0..member_count {
        let property_index = usize::try_from(cursor.u32()?)
            .map_err(|_| reject("family property index is outside range"))?;
        let horizon = cursor.u32()?;
        if horizon > MAX_SEARCH_HORIZON {
            return Err(reject("family proof horizon exceeds limit"));
        }
        let evidence_len = count(
            cursor.u32()?,
            MAX_SEARCH_CERTIFICATE_BYTES,
            "family proof member length",
        )?;
        evidence_bytes = evidence_bytes
            .checked_add(evidence_len)
            .filter(|value| *value <= policy.max_evidence_bytes)
            .ok_or_else(|| reject("BTOR2 family proof evidence exceeds policy"))?;
        members.push(Btor2FamilyProofMember {
            property_index,
            horizon,
            evidence: cursor.take(evidence_len)?.to_vec(),
        });
    }
    if cursor.offset != cursor.bytes.len() {
        return Err(reject("trailing BTOR2 family proof bytes"));
    }
    let artifact = Btor2FamilyProofArtifact {
        version,
        family_artifact,
        members,
    };
    let canonical = encode_btor2_family_proof(&artifact, policy)?;
    if canonical != bytes {
        return Err(reject("BTOR2 family proof is not canonically encoded"));
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
10 not 1 9
11 output 9 mismatch
12 output 10 match
"#;
    const PARAMETERS: &[u8] = b"width=1\n";

    fn instances() -> Vec<Btor2FamilyInstance> {
        ["channel0", "channel1"]
            .into_iter()
            .map(|identifier| Btor2FamilyInstance {
                identifier: identifier.to_string(),
                parameter_sha256: Sha256::digest(PARAMETERS).into(),
                input_bindings: vec![
                    FamilyInputBinding::CoreRoot(0),
                    FamilyInputBinding::CoreInput(0),
                ],
            })
            .collect()
    }

    fn proof() -> Btor2FamilyProofArtifact {
        produce_btor2_family_proof(
            Btor2FamilyProofInput {
                core_bytes: CORE,
                core_roots: &[3],
                channel_bytes: CHANNEL,
                channel_roots: &[5, 9],
                parameter_bytes: PARAMETERS,
                instances: &instances(),
                queries: &[
                    Btor2FamilyQuery {
                        property_index: 0,
                        horizon: 2,
                    },
                    Btor2FamilyQuery {
                        property_index: 1,
                        horizon: 2,
                    },
                ],
            },
            Btor2FamilyProofPolicy::default(),
        )
        .unwrap()
        .0
    }

    #[test]
    fn preserves_safe_and_unsafe_answers_through_independent_replay() {
        let artifact = proof();
        let bytes =
            encode_btor2_family_proof(&artifact, Btor2FamilyProofPolicy::default()).unwrap();
        let decoded = decode_btor2_family_proof(&bytes, Btor2FamilyProofPolicy::default()).unwrap();
        let summary = verify_btor2_family_proof(
            CORE,
            CHANNEL,
            PARAMETERS,
            &decoded,
            Btor2FamilyProofPolicy::default(),
        )
        .unwrap();
        assert_eq!(summary.queries, 2);
        assert!(summary.safe > 0);
        assert!(summary.unsafe_count > 0);
        assert_eq!(artifact, decoded);
    }

    #[test]
    fn truncation_mutation_query_and_source_drift_fail_closed() {
        let artifact = proof();
        let bytes =
            encode_btor2_family_proof(&artifact, Btor2FamilyProofPolicy::default()).unwrap();
        for end in 0..bytes.len() {
            assert!(
                decode_btor2_family_proof(&bytes[..end], Btor2FamilyProofPolicy::default())
                    .is_err()
            );
        }
        for offset in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[offset] ^= 1;
            assert!(
                decode_btor2_family_proof(&changed, Btor2FamilyProofPolicy::default()).is_err()
            );
        }

        let mut rebound = artifact.clone();
        rebound.members[0].property_index = 1;
        assert!(
            verify_btor2_family_proof(
                CORE,
                CHANNEL,
                PARAMETERS,
                &rebound,
                Btor2FamilyProofPolicy::default(),
            )
            .is_err()
        );
        assert!(
            verify_btor2_family_proof(
                CORE,
                CHANNEL,
                b"width=2\n",
                &artifact,
                Btor2FamilyProofPolicy::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn retained_opentitan_family_produces_all_predeclared_property_members() {
        const OT_CORE: &[u8] = include_bytes!(
            "../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-after.btor2"
        );
        const OT_CHANNEL: &[u8] = include_bytes!(
            "../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-after.btor2"
        );
        const OT_PARAMETERS: &[u8] = b"source_revision=child;phase_width=4\n";
        for count in [2, 4, 6] {
            let instances = (0..count)
                .map(|index| Btor2FamilyInstance {
                    identifier: format!("channel{index}"),
                    parameter_sha256: Sha256::digest(OT_PARAMETERS).into(),
                    input_bindings: vec![
                        FamilyInputBinding::CoreRoot(0),
                        FamilyInputBinding::CoreRoot(1),
                        FamilyInputBinding::CoreRoot(2),
                        FamilyInputBinding::CoreRoot(3),
                    ],
                })
                .collect::<Vec<_>>();
            let queries = (0..count * 5)
                .map(|property_index| Btor2FamilyQuery {
                    property_index,
                    horizon: 4,
                })
                .collect::<Vec<_>>();
            let (artifact, _) = produce_btor2_family_proof(
                Btor2FamilyProofInput {
                    core_bytes: OT_CORE,
                    core_roots: &[1000, 1001, 1002, 1003],
                    channel_bytes: OT_CHANNEL,
                    channel_roots: &[1000, 1001, 1002, 1003, 1004],
                    parameter_bytes: OT_PARAMETERS,
                    instances: &instances,
                    queries: &queries,
                },
                Btor2FamilyProofPolicy::default(),
            )
            .unwrap();
            let summary = verify_btor2_family_proof(
                OT_CORE,
                OT_CHANNEL,
                OT_PARAMETERS,
                &artifact,
                Btor2FamilyProofPolicy::default(),
            )
            .unwrap();
            assert_eq!(summary.queries, count * 5);
            assert_eq!(summary.safe + summary.unsafe_count, count * 5);
        }
    }
}
