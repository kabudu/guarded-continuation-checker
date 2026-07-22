//! Exact channel-local property models over source-bound repeated BTOR2 regions.

use crate::btor2::{self, NodeId};
use crate::btor2_bitblast::{
    MAX_BITBLAST_CERTIFICATE_BYTES, MAX_BITBLAST_HORIZON, MAX_BITBLAST_INPUT_BITS,
    decode_btor2_bitblast_certificate, encode_btor2_bitblast_certificate,
    produce_btor2_bitblast_certificate, verify_btor2_bitblast_certificate,
};
use crate::btor2_region_equivalence::{
    MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES, admit_btor2_region_equivalence_artifact,
    decode_btor2_region_equivalence_artifact,
};
use crate::btor2_region_extract::{
    Btor2RegionError, Btor2RegionPolicy, extract_btor2_complete_regions,
};
use crate::btor2_search::{self, SearchCertificate, SearchSummary};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::Instant;

pub const MAX_CHANNEL_PROPERTY_QUERIES: usize = 4096;
pub const MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES: usize = 66 * 1024 * 1024;
pub const MAX_CHANNEL_PROPERTY_PROJECTED_WORK: u64 = 100_000_000_000;
pub const BTOR2_CHANNEL_PROPERTY_PROOF_VERSION: u32 = 1;
const CHANNEL_PROPERTY_MAGIC: &[u8; 8] = b"GCCBCP01";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProofPolicy {
    max_queries: usize,
    max_members: usize,
    max_evidence_bytes: usize,
    max_artifact_bytes: usize,
}

impl Btor2ChannelPropertyProofPolicy {
    pub fn new(
        max_queries: usize,
        max_members: usize,
        max_evidence_bytes: usize,
        max_artifact_bytes: usize,
    ) -> Result<Self, Btor2RegionError> {
        if max_queries == 0
            || max_queries > MAX_CHANNEL_PROPERTY_QUERIES
            || max_members == 0
            || max_members > MAX_CHANNEL_PROPERTY_QUERIES
            || max_evidence_bytes == 0
            || max_evidence_bytes > MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES
            || max_artifact_bytes == 0
            || max_artifact_bytes > MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES
        {
            return Err(reject(
                "BTOR2 channel property proof policy is outside static limits",
            ));
        }
        Ok(Self {
            max_queries,
            max_members,
            max_evidence_bytes,
            max_artifact_bytes,
        })
    }

    pub fn max_queries(self) -> usize {
        self.max_queries
    }

    pub fn max_members(self) -> usize {
        self.max_members
    }

    pub fn max_evidence_bytes(self) -> usize {
        self.max_evidence_bytes
    }

    pub fn max_artifact_bytes(self) -> usize {
        self.max_artifact_bytes
    }
}

impl Default for Btor2ChannelPropertyProofPolicy {
    fn default() -> Self {
        Self {
            max_queries: MAX_CHANNEL_PROPERTY_QUERIES,
            max_members: MAX_CHANNEL_PROPERTY_QUERIES,
            max_evidence_bytes: MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES,
            max_artifact_bytes: MAX_CHANNEL_PROPERTY_ARTIFACT_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProductionPolicy {
    artifact: Btor2ChannelPropertyProofPolicy,
    max_projected_work: u64,
}

impl Btor2ChannelPropertyProductionPolicy {
    pub fn new(
        artifact: Btor2ChannelPropertyProofPolicy,
        max_projected_work: u64,
    ) -> Result<Self, Btor2RegionError> {
        if max_projected_work == 0 || max_projected_work > MAX_CHANNEL_PROPERTY_PROJECTED_WORK {
            return Err(reject(
                "BTOR2 channel property production policy is outside static limits",
            ));
        }
        Ok(Self {
            artifact,
            max_projected_work,
        })
    }

    pub fn artifact(self) -> Btor2ChannelPropertyProofPolicy {
        self.artifact
    }

    pub fn max_projected_work(self) -> u64 {
        self.max_projected_work
    }
}

impl Default for Btor2ChannelPropertyProductionPolicy {
    fn default() -> Self {
        Self {
            artifact: Btor2ChannelPropertyProofPolicy::default(),
            max_projected_work: MAX_CHANNEL_PROPERTY_PROJECTED_WORK,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Btor2ChannelProperty {
    OutputHigh,
    OutputLow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyQuery {
    pub query_id: u32,
    pub channel_index: usize,
    pub property: Btor2ChannelProperty,
    pub horizon: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyEvidence {
    pub query: Btor2ChannelPropertyQuery,
    pub property_model: Vec<u8>,
    pub certificate: SearchCertificate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Btor2ChannelPropertyBackend {
    RepresentativeClass,
    DirectExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Btor2ChannelPropertySolver {
    ExplicitState,
    BitblastCnf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProofMember {
    pub class_index: usize,
    pub representative_channel: usize,
    pub property: Btor2ChannelProperty,
    pub horizon: u32,
    pub backend: Btor2ChannelPropertyBackend,
    pub solver: Btor2ChannelPropertySolver,
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProofArtifact {
    pub version: u32,
    pub model_sha256: [u8; 32],
    pub structural_admission: Vec<u8>,
    pub queries: Vec<Btor2ChannelPropertyQuery>,
    pub members: Vec<Btor2ChannelPropertyProofMember>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyResult {
    pub query: Btor2ChannelPropertyQuery,
    pub result: btor2_search::SearchResult,
    pub bad_frame: Option<u32>,
    pub backend: Btor2ChannelPropertyBackend,
    pub solver: Btor2ChannelPropertySolver,
    pub representative_channel: usize,
    pub witness_valuations: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyMetrics {
    pub logical_queries: usize,
    pub proof_members: usize,
    pub representative_members: usize,
    pub direct_exact_members: usize,
    pub explicit_state_members: usize,
    pub bitblast_members: usize,
    pub reused_logical_queries: usize,
    pub evidence_bytes: usize,
    pub direct_proof_member_bound: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProofSummary {
    pub results: Vec<Btor2ChannelPropertyResult>,
    pub metrics: Btor2ChannelPropertyMetrics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProductionPlan {
    pub logical_queries: usize,
    pub proof_members: usize,
    pub explicit_state_members: usize,
    pub bitblast_members: usize,
    pub projected_work: u64,
}

/// Diagnostic production timings. These values never participate in
/// admission, routing, certificate bytes, or verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelPropertyProductionPhaseMetrics {
    pub preflight_micros: u128,
    pub proof_construction_micros: u128,
    pub encoding_micros: u128,
    pub total_micros: u128,
}

fn reject(message: impl Into<String>) -> Btor2RegionError {
    Btor2RegionError(message.into())
}

fn maximum_statement_id(model_bytes: &[u8]) -> Result<NodeId, Btor2RegionError> {
    let text = std::str::from_utf8(model_bytes)
        .map_err(|_| reject("BTOR2 channel property source is not UTF-8"))?;
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            (!trimmed.is_empty() && !trimmed.starts_with(';'))
                .then(|| trimmed.split_ascii_whitespace().next())
                .flatten()
        })
        .map(|token| {
            token
                .parse::<NodeId>()
                .map_err(|_| reject("BTOR2 channel property statement identifier is invalid"))
        })
        .try_fold(0, |maximum, id| id.map(|id| maximum.max(id)))
}

/// Reconstructs one canonical bad-property model from a property-free channel
/// source. This is shared by explicit and bit-blasted exact backends.
pub fn build_btor2_channel_property_model(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    channel_index: usize,
    property: Btor2ChannelProperty,
    policy: Btor2RegionPolicy,
) -> Result<(Vec<u8>, NodeId), Btor2RegionError> {
    if channel_index >= expected_channels {
        return Err(reject("BTOR2 channel property index is outside range"));
    }
    let complete =
        extract_btor2_complete_regions(model_bytes, semantic_roots, expected_channels, policy)?;
    let model = btor2::parse_component_bytes(model_bytes, semantic_roots)
        .map_err(|error| reject(format!("invalid BTOR2 channel property model: {error}")))?;
    if !model.bad_properties().is_empty() {
        return Err(reject(
            "BTOR2 channel property source must not embed bad properties",
        ));
    }
    let outgoing = complete
        .channel_to_aggregate_edges
        .iter()
        .filter(|edge| edge.channel_index == channel_index)
        .map(|edge| edge.source)
        .collect::<Vec<_>>();
    if outgoing.len() != 1 || model.nodes()[&outgoing[0]].width != 1 {
        return Err(reject(
            "BTOR2 channel property requires one Boolean channel observation",
        ));
    }
    let output = outgoing[0];
    let maximum = maximum_statement_id(model_bytes)?;
    let expression = maximum
        .checked_add(1)
        .ok_or_else(|| reject("BTOR2 channel property identifier overflow"))?;
    let bad = match property {
        Btor2ChannelProperty::OutputHigh => expression,
        Btor2ChannelProperty::OutputLow => expression
            .checked_add(1)
            .ok_or_else(|| reject("BTOR2 channel property identifier overflow"))?,
    };
    let mut bytes = model_bytes.to_vec();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    match property {
        Btor2ChannelProperty::OutputHigh => {
            bytes.extend_from_slice(
                format!("{bad} bad {output} gcc_channel_output_high\n").as_bytes(),
            );
        }
        Btor2ChannelProperty::OutputLow => {
            bytes.extend_from_slice(format!("{expression} not 5 {output}\n").as_bytes());
            bytes.extend_from_slice(
                format!("{bad} bad {expression} gcc_channel_output_low\n").as_bytes(),
            );
        }
    }
    btor2::parse_bytes(&bytes).map_err(|error| {
        reject(format!(
            "generated BTOR2 channel property is invalid: {error}"
        ))
    })?;
    Ok((bytes, bad))
}

pub fn produce_btor2_channel_property_evidence(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    query: Btor2ChannelPropertyQuery,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ChannelPropertyEvidence, Btor2RegionError> {
    let (property_model, bad) = build_btor2_channel_property_model(
        model_bytes,
        semantic_roots,
        expected_channels,
        query.channel_index,
        query.property,
        policy,
    )?;
    let certificate = btor2_search::produce(&property_model, bad, query.horizon)
        .map_err(|error| reject(format!("BTOR2 channel property search failed: {error}")))?;
    Ok(Btor2ChannelPropertyEvidence {
        query,
        property_model,
        certificate,
    })
}

pub fn verify_btor2_channel_property_evidence(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    evidence: &Btor2ChannelPropertyEvidence,
    policy: Btor2RegionPolicy,
) -> Result<SearchSummary, Btor2RegionError> {
    let (expected_model, bad) = build_btor2_channel_property_model(
        model_bytes,
        semantic_roots,
        expected_channels,
        evidence.query.channel_index,
        evidence.query.property,
        policy,
    )?;
    if expected_model != evidence.property_model || bad != evidence.certificate.bad_property {
        return Err(reject("BTOR2 channel property evidence binding mismatch"));
    }
    btor2_search::verify(&expected_model, &evidence.certificate).map_err(|error| {
        reject(format!(
            "BTOR2 channel property verification failed: {error}"
        ))
    })
}

fn validate_queries(
    queries: &[Btor2ChannelPropertyQuery],
    channels: usize,
) -> Result<(), Btor2RegionError> {
    if queries.is_empty()
        || queries.len() > MAX_CHANNEL_PROPERTY_QUERIES
        || queries
            .windows(2)
            .any(|pair| pair[0].query_id >= pair[1].query_id)
        || queries.iter().any(|query| {
            query.channel_index >= channels || query.horizon > btor2_search::MAX_SEARCH_HORIZON
        })
    {
        return Err(reject("BTOR2 channel property queries are outside policy"));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MemberKey {
    class_index: usize,
    property: Btor2ChannelProperty,
    horizon: u32,
}

struct VerifiedMember {
    result: btor2_search::SearchResult,
    bad_frame: Option<u32>,
    solver: Btor2ChannelPropertySolver,
    witness_valuations: Vec<u64>,
}

#[derive(Clone, Copy)]
struct SolverProjection {
    solver: Btor2ChannelPropertySolver,
    work: u64,
}

fn class_lookup(classes: &[Vec<usize>]) -> Result<Vec<usize>, Btor2RegionError> {
    let channels = classes.iter().map(Vec::len).sum::<usize>();
    let mut lookup = vec![usize::MAX; channels];
    for (class_index, class) in classes.iter().enumerate() {
        for channel in class {
            let slot = lookup
                .get_mut(*channel)
                .ok_or_else(|| reject("BTOR2 channel property class is outside range"))?;
            if *slot != usize::MAX {
                return Err(reject("BTOR2 channel property class overlaps"));
            }
            *slot = class_index;
        }
    }
    if lookup.contains(&usize::MAX) {
        return Err(reject(
            "BTOR2 channel property class partition is incomplete",
        ));
    }
    Ok(lookup)
}

fn expected_member_keys(
    queries: &[Btor2ChannelPropertyQuery],
    class_lookup: &[usize],
) -> BTreeMap<MemberKey, Vec<u32>> {
    let mut groups = BTreeMap::<MemberKey, Vec<u32>>::new();
    for query in queries {
        groups
            .entry(MemberKey {
                class_index: class_lookup[query.channel_index],
                property: query.property,
                horizon: query.horizon,
            })
            .or_default()
            .push(query.query_id);
    }
    groups
}

fn select_solver(
    property_model: &[u8],
    horizon: u32,
) -> Result<SolverProjection, Btor2RegionError> {
    let model = btor2::parse_bytes(property_model)
        .map_err(|error| reject(format!("invalid BTOR2 channel property model: {error}")))?;
    let input_bits = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].width as usize)
        .try_fold(0usize, |total, width| total.checked_add(width))
        .ok_or_else(|| reject("BTOR2 channel property input width overflow"))?;
    let select_bitblast = || {
        if horizon > MAX_BITBLAST_HORIZON {
            return Err(reject(
                "BTOR2 channel property projected work exceeds the explicit-state policy and its horizon exceeds the bitblast policy",
            ));
        }
        if input_bits > MAX_BITBLAST_INPUT_BITS {
            return Err(reject(
                "BTOR2 channel property input width exceeds every exact backend policy",
            ));
        }
        let frame_work = model.nodes().values().try_fold(0u64, |total, node| {
            let width = u64::from(node.width);
            width
                .checked_mul(width)
                .and_then(|value| value.checked_mul(16))
                .and_then(|value| total.checked_add(value.max(1)))
        });
        let work = frame_work
            .and_then(|value| value.checked_mul(u64::from(horizon) + 1))
            .ok_or_else(|| reject("BTOR2 channel property bitblast work projection overflow"))?;
        Ok(SolverProjection {
            solver: Btor2ChannelPropertySolver::BitblastCnf,
            work,
        })
    };
    if horizon > btor2_search::MAX_SEARCH_HORIZON
        || input_bits > btor2_search::MAX_SEARCH_INPUT_BITS
    {
        return select_bitblast();
    }
    let valuations = 1u64
        .checked_shl(input_bits as u32)
        .ok_or_else(|| reject("BTOR2 channel property valuation count overflow"))?;
    let per_state = valuations
        .checked_mul(model.nodes().len() as u64)
        .and_then(|value| value.checked_mul(model.states().len().max(1) as u64))
        .ok_or_else(|| reject("BTOR2 channel property work projection overflow"))?;
    let mut layer_states = 1u64;
    let mut work = 0u64;
    for _ in 0..horizon {
        work = work
            .checked_add(
                layer_states
                    .checked_mul(per_state)
                    .ok_or_else(|| reject("BTOR2 channel property work projection overflow"))?,
            )
            .ok_or_else(|| reject("BTOR2 channel property work projection overflow"))?;
        if work > btor2_search::MAX_SEARCH_NODE_STEPS {
            return select_bitblast();
        }
        layer_states = layer_states
            .saturating_mul(valuations)
            .min(btor2_search::MAX_STATES_PER_LAYER as u64);
    }
    Ok(SolverProjection {
        solver: Btor2ChannelPropertySolver::ExplicitState,
        work: work.max(per_state),
    })
}

fn unpack_valuation(
    model: &btor2::Btor2Model,
    valuation: u64,
) -> Result<btor2::WordValues, Btor2RegionError> {
    let mut offset = 0usize;
    let mut values = btor2::WordValues::new();
    for input in model.inputs() {
        let width = model.nodes()[input].width as usize;
        let end = offset
            .checked_add(width)
            .ok_or_else(|| reject("BTOR2 channel property witness input width overflow"))?;
        if width == 0 || width > 64 || end > MAX_BITBLAST_INPUT_BITS {
            return Err(reject(
                "BTOR2 channel property witness input width is outside policy",
            ));
        }
        let mask = if width == 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        values.insert(*input, (valuation >> offset) & mask);
        offset = end;
    }
    if offset < 64 && valuation >= (1u64 << offset) {
        return Err(reject(
            "BTOR2 channel property witness valuation is noncanonical",
        ));
    }
    Ok(values)
}

fn replay_unsafe_assignment(
    property_model: &[u8],
    bad: NodeId,
    valuations: &[u64],
    expected_bad_frame: u32,
) -> Result<(), Btor2RegionError> {
    let model = btor2::parse_bytes(property_model)
        .map_err(|error| reject(format!("invalid target property model: {error}")))?;
    let mut state = model
        .initial_state()
        .map_err(|error| reject(format!("target property initial state failed: {error}")))?;
    for (frame, valuation) in valuations.iter().enumerate() {
        let inputs = unpack_valuation(&model, *valuation)?;
        for (_, constraint) in model.constraints() {
            if model
                .evaluate(*constraint, &state, &inputs)
                .map_err(|error| reject(format!("target property constraint failed: {error}")))?
                == 0
            {
                return Err(reject(
                    "BTOR2 channel property target witness violates a constraint",
                ));
            }
        }
        if model
            .active_bad(&state, &inputs)
            .map_err(|error| reject(format!("target property witness failed: {error}")))?
            .contains(&bad)
        {
            if u32::try_from(frame).ok() == Some(expected_bad_frame) {
                return Ok(());
            }
            return Err(reject("BTOR2 channel property target bad frame mismatch"));
        }
        state = model
            .step(&state, &inputs)
            .map_err(|error| reject(format!("target property witness step failed: {error}")))?;
    }
    Err(reject(
        "BTOR2 channel property assignment does not reproduce target violation",
    ))
}

/// Plans every member and enforces the aggregate static work ceiling before a
/// property solver starts. Structural admission is replayed from source during
/// planning; an invalid admission is an error rather than a fallback signal.
pub fn preflight_btor2_channel_property_proof(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    region_policy: Btor2RegionPolicy,
    production_policy: Btor2ChannelPropertyProductionPolicy,
) -> Result<Btor2ChannelPropertyProductionPlan, Btor2RegionError> {
    let decoded = decode_btor2_region_equivalence_artifact(structural_admission)?;
    let admission = admit_btor2_region_equivalence_artifact(model_bytes, &decoded, region_policy)?;
    validate_queries(queries, decoded.expected_channels)?;
    if queries.len() > production_policy.artifact.max_queries {
        return Err(reject(
            "BTOR2 channel property production query count exceeds policy",
        ));
    }
    let lookup = class_lookup(admission.classes())?;
    let groups = expected_member_keys(queries, &lookup);
    if groups.len() > production_policy.artifact.max_members {
        return Err(reject(
            "BTOR2 channel property production member count exceeds policy",
        ));
    }
    let mut explicit_state_members = 0usize;
    let mut bitblast_members = 0usize;
    let mut projected_work = 0u64;
    for key in groups.keys() {
        let representative_channel = admission.classes()[key.class_index][0];
        let (property_model, _) = build_btor2_channel_property_model(
            model_bytes,
            &decoded.semantic_roots,
            decoded.expected_channels,
            representative_channel,
            key.property,
            region_policy,
        )?;
        let projection = select_solver(&property_model, key.horizon)?;
        match projection.solver {
            Btor2ChannelPropertySolver::ExplicitState => explicit_state_members += 1,
            Btor2ChannelPropertySolver::BitblastCnf => bitblast_members += 1,
        }
        projected_work = projected_work
            .checked_add(projection.work)
            .filter(|value| *value <= production_policy.max_projected_work)
            .ok_or_else(|| {
                reject("BTOR2 channel property aggregate projected work exceeds policy")
            })?;
    }
    Ok(Btor2ChannelPropertyProductionPlan {
        logical_queries: queries.len(),
        proof_members: groups.len(),
        explicit_state_members,
        bitblast_members,
        projected_work,
    })
}

/// Produces one exact property certificate per verified class and query shape.
/// Singleton classes remain direct exact members. Invalid admission evidence
/// propagates and is never converted into a fallback result.
pub fn produce_btor2_channel_property_proof(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    policy: Btor2RegionPolicy,
) -> Result<Btor2ChannelPropertyProofArtifact, Btor2RegionError> {
    let _ = preflight_btor2_channel_property_proof(
        model_bytes,
        structural_admission,
        queries,
        policy,
        Btor2ChannelPropertyProductionPolicy::default(),
    )?;
    produce_btor2_channel_property_proof_after_preflight(
        model_bytes,
        structural_admission,
        queries,
        policy,
    )
}

fn produce_btor2_channel_property_proof_after_preflight(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    policy: Btor2RegionPolicy,
) -> Result<Btor2ChannelPropertyProofArtifact, Btor2RegionError> {
    let decoded = decode_btor2_region_equivalence_artifact(structural_admission)?;
    let admission = admit_btor2_region_equivalence_artifact(model_bytes, &decoded, policy)?;
    validate_queries(queries, decoded.expected_channels)?;
    let lookup = class_lookup(admission.classes())?;
    let groups = expected_member_keys(queries, &lookup);
    let mut evidence_bytes = 0usize;
    let mut members = Vec::with_capacity(groups.len());
    for key in groups.keys() {
        let class = &admission.classes()[key.class_index];
        let representative_channel = class[0];
        let property_query = Btor2ChannelPropertyQuery {
            query_id: 0,
            channel_index: representative_channel,
            property: key.property,
            horizon: key.horizon,
        };
        let (property_model, bad) = build_btor2_channel_property_model(
            model_bytes,
            &decoded.semantic_roots,
            decoded.expected_channels,
            representative_channel,
            key.property,
            policy,
        )?;
        let solver = select_solver(&property_model, key.horizon)?.solver;
        let evidence = match solver {
            Btor2ChannelPropertySolver::ExplicitState => {
                let property_evidence = produce_btor2_channel_property_evidence(
                    model_bytes,
                    &decoded.semantic_roots,
                    decoded.expected_channels,
                    property_query,
                    policy,
                )?;
                btor2_search::encode(&property_evidence.certificate)
                    .map_err(|error| {
                        reject(format!("BTOR2 channel property encoding failed: {error}"))
                    })?
                    .into_bytes()
            }
            Btor2ChannelPropertySolver::BitblastCnf => encode_btor2_bitblast_certificate(
                &produce_btor2_bitblast_certificate(&property_model, bad, key.horizon)
                    .map_err(|error| reject(error.to_string()))?,
            )
            .map_err(|error| reject(error.to_string()))?,
        };
        evidence_bytes = evidence_bytes
            .checked_add(evidence.len())
            .filter(|total| *total <= MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES)
            .ok_or_else(|| reject("BTOR2 channel property evidence exceeds policy"))?;
        members.push(Btor2ChannelPropertyProofMember {
            class_index: key.class_index,
            representative_channel,
            property: key.property,
            horizon: key.horizon,
            backend: if class.len() == 1 {
                Btor2ChannelPropertyBackend::DirectExact
            } else {
                Btor2ChannelPropertyBackend::RepresentativeClass
            },
            solver,
            evidence,
        });
    }
    Ok(Btor2ChannelPropertyProofArtifact {
        version: BTOR2_CHANNEL_PROPERTY_PROOF_VERSION,
        model_sha256: Sha256::digest(model_bytes).into(),
        structural_admission: structural_admission.to_vec(),
        queries: queries.to_vec(),
        members,
    })
}

/// Replays admission from source, verifies every retained exact certificate,
/// and derives logical class members only from the verified partition.
pub fn verify_btor2_channel_property_proof(
    model_bytes: &[u8],
    expected_queries: &[Btor2ChannelPropertyQuery],
    artifact: &Btor2ChannelPropertyProofArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ChannelPropertyProofSummary, Btor2RegionError> {
    if artifact.version != BTOR2_CHANNEL_PROPERTY_PROOF_VERSION
        || artifact.model_sha256 != <[u8; 32]>::from(Sha256::digest(model_bytes))
        || artifact.queries != expected_queries
    {
        return Err(reject("BTOR2 channel property artifact binding mismatch"));
    }
    let decoded = decode_btor2_region_equivalence_artifact(&artifact.structural_admission)?;
    let admission = admit_btor2_region_equivalence_artifact(model_bytes, &decoded, policy)?;
    validate_queries(expected_queries, decoded.expected_channels)?;
    let lookup = class_lookup(admission.classes())?;
    let groups = expected_member_keys(expected_queries, &lookup);
    if artifact.members.len() != groups.len() {
        return Err(reject("BTOR2 channel property proof member count mismatch"));
    }
    let mut verified = BTreeMap::<MemberKey, VerifiedMember>::new();
    let mut evidence_bytes = 0usize;
    for (member, expected_key) in artifact.members.iter().zip(groups.keys()) {
        let class = &admission.classes()[expected_key.class_index];
        let expected_backend = if class.len() == 1 {
            Btor2ChannelPropertyBackend::DirectExact
        } else {
            Btor2ChannelPropertyBackend::RepresentativeClass
        };
        let (property_model, _bad) = build_btor2_channel_property_model(
            model_bytes,
            &decoded.semantic_roots,
            decoded.expected_channels,
            member.representative_channel,
            member.property,
            policy,
        )?;
        let expected_solver = select_solver(&property_model, member.horizon)?.solver;
        if member.class_index != expected_key.class_index
            || member.representative_channel != class[0]
            || member.property != expected_key.property
            || member.horizon != expected_key.horizon
            || member.backend != expected_backend
            || member.solver != expected_solver
            || member.evidence.is_empty()
        {
            return Err(reject("BTOR2 channel property proof member mismatch"));
        }
        evidence_bytes = evidence_bytes
            .checked_add(member.evidence.len())
            .filter(|total| *total <= MAX_CHANNEL_PROPERTY_EVIDENCE_BYTES)
            .ok_or_else(|| reject("BTOR2 channel property evidence exceeds policy"))?;
        let verified_member = match member.solver {
            Btor2ChannelPropertySolver::ExplicitState => {
                let certificate = btor2_search::decode(&member.evidence).map_err(|error| {
                    reject(format!(
                        "BTOR2 channel property evidence decode failed: {error}"
                    ))
                })?;
                if btor2_search::encode(&certificate)
                    .map_err(|error| {
                        reject(format!("BTOR2 channel property encoding failed: {error}"))
                    })?
                    .as_bytes()
                    != member.evidence
                {
                    return Err(reject("BTOR2 channel property evidence is not canonical"));
                }
                let direct = Btor2ChannelPropertyEvidence {
                    query: Btor2ChannelPropertyQuery {
                        query_id: 0,
                        channel_index: member.representative_channel,
                        property: member.property,
                        horizon: member.horizon,
                    },
                    property_model,
                    certificate: certificate.clone(),
                };
                let summary = verify_btor2_channel_property_evidence(
                    model_bytes,
                    &decoded.semantic_roots,
                    decoded.expected_channels,
                    &direct,
                    policy,
                )?;
                let mut witness_valuations = certificate
                    .witness_valuations
                    .iter()
                    .map(|valuation| u64::from(*valuation))
                    .collect::<Vec<_>>();
                if summary.result == btor2_search::SearchResult::Unsafe {
                    witness_valuations.push(u64::from(certificate.terminal_valuation.ok_or_else(
                        || reject("BTOR2 channel property UNSAFE evidence lacks terminal input"),
                    )?));
                }
                VerifiedMember {
                    result: summary.result,
                    bad_frame: summary.bad_frame,
                    solver: member.solver,
                    witness_valuations,
                }
            }
            Btor2ChannelPropertySolver::BitblastCnf => {
                let certificate = decode_btor2_bitblast_certificate(&member.evidence)
                    .map_err(|error| reject(error.to_string()))?;
                let summary = verify_btor2_bitblast_certificate(&property_model, &certificate)
                    .map_err(|error| reject(error.to_string()))?;
                let witness_valuations = if let Some(bad_frame) = summary.bad_frame {
                    certificate
                        .witness_valuations
                        .get(..=bad_frame as usize)
                        .ok_or_else(|| reject("BTOR2 bitblast witness is incomplete"))?
                        .to_vec()
                } else {
                    Vec::new()
                };
                VerifiedMember {
                    result: summary.result,
                    bad_frame: summary.bad_frame,
                    solver: member.solver,
                    witness_valuations,
                }
            }
        };
        verified.insert(*expected_key, verified_member);
    }
    let mut reused_logical_queries = 0usize;
    let mut results = Vec::with_capacity(expected_queries.len());
    for query in expected_queries {
        let key = MemberKey {
            class_index: lookup[query.channel_index],
            property: query.property,
            horizon: query.horizon,
        };
        let class = &admission.classes()[key.class_index];
        let member = &verified[&key];
        let (target_model, target_bad) = build_btor2_channel_property_model(
            model_bytes,
            &decoded.semantic_roots,
            decoded.expected_channels,
            query.channel_index,
            query.property,
            policy,
        )?;
        if member.result == btor2_search::SearchResult::Unsafe {
            replay_unsafe_assignment(
                &target_model,
                target_bad,
                &member.witness_valuations,
                member
                    .bad_frame
                    .ok_or_else(|| reject("BTOR2 channel property UNSAFE result lacks frame"))?,
            )?;
        }
        if class.len() > 1 && query.channel_index != class[0] {
            reused_logical_queries += 1;
        }
        results.push(Btor2ChannelPropertyResult {
            query: *query,
            result: member.result,
            bad_frame: member.bad_frame,
            backend: if class.len() == 1 {
                Btor2ChannelPropertyBackend::DirectExact
            } else {
                Btor2ChannelPropertyBackend::RepresentativeClass
            },
            solver: member.solver,
            representative_channel: class[0],
            witness_valuations: member.witness_valuations.clone(),
        });
    }
    let representative_members = artifact
        .members
        .iter()
        .filter(|member| member.backend == Btor2ChannelPropertyBackend::RepresentativeClass)
        .count();
    let explicit_state_members = artifact
        .members
        .iter()
        .filter(|member| member.solver == Btor2ChannelPropertySolver::ExplicitState)
        .count();
    Ok(Btor2ChannelPropertyProofSummary {
        results,
        metrics: Btor2ChannelPropertyMetrics {
            logical_queries: expected_queries.len(),
            proof_members: artifact.members.len(),
            representative_members,
            direct_exact_members: artifact.members.len() - representative_members,
            explicit_state_members,
            bitblast_members: artifact.members.len() - explicit_state_members,
            reused_logical_queries,
            evidence_bytes,
            direct_proof_member_bound: expected_queries.len(),
        },
    })
}

fn push_u32(bytes: &mut Vec<u8>, value: usize, label: &str) -> Result<(), Btor2RegionError> {
    let value = u32::try_from(value).map_err(|_| reject(format!("{label} exceeds range")))?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn validate_member_evidence(
    member: &Btor2ChannelPropertyProofMember,
) -> Result<(), Btor2RegionError> {
    if member.evidence.is_empty() {
        return Err(reject("BTOR2 channel property proof evidence is empty"));
    }
    match member.solver {
        Btor2ChannelPropertySolver::ExplicitState => {
            if member.evidence.len() > btor2_search::MAX_SEARCH_CERTIFICATE_BYTES {
                return Err(reject(
                    "BTOR2 channel property explicit evidence exceeds policy",
                ));
            }
            let certificate = btor2_search::decode(&member.evidence).map_err(|error| {
                reject(format!(
                    "BTOR2 channel property explicit evidence decode failed: {error}"
                ))
            })?;
            if btor2_search::encode(&certificate)
                .map_err(|error| {
                    reject(format!(
                        "BTOR2 channel property explicit evidence encode failed: {error}"
                    ))
                })?
                .as_bytes()
                != member.evidence
            {
                return Err(reject(
                    "BTOR2 channel property explicit evidence is not canonical",
                ));
            }
        }
        Btor2ChannelPropertySolver::BitblastCnf => {
            if member.evidence.len() > MAX_BITBLAST_CERTIFICATE_BYTES {
                return Err(reject(
                    "BTOR2 channel property bitblast evidence exceeds policy",
                ));
            }
            let certificate = decode_btor2_bitblast_certificate(&member.evidence)
                .map_err(|error| reject(error.to_string()))?;
            if encode_btor2_bitblast_certificate(&certificate)
                .map_err(|error| reject(error.to_string()))?
                != member.evidence
            {
                return Err(reject(
                    "BTOR2 channel property bitblast evidence is not canonical",
                ));
            }
        }
    }
    Ok(())
}

fn validate_property_artifact_shape(
    artifact: &Btor2ChannelPropertyProofArtifact,
    policy: Btor2ChannelPropertyProofPolicy,
) -> Result<(), Btor2RegionError> {
    if artifact.version != BTOR2_CHANNEL_PROPERTY_PROOF_VERSION
        || artifact.structural_admission.is_empty()
        || artifact.structural_admission.len() > MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES
        || artifact.queries.is_empty()
        || artifact.queries.len() > policy.max_queries
        || artifact.members.is_empty()
        || artifact.members.len() > policy.max_members
    {
        return Err(reject(
            "BTOR2 channel property proof artifact is outside policy",
        ));
    }
    let structural = decode_btor2_region_equivalence_artifact(&artifact.structural_admission)?;
    if artifact.model_sha256 != structural.model_sha256 {
        return Err(reject(
            "BTOR2 channel property proof structural binding mismatch",
        ));
    }
    validate_queries(&artifact.queries, structural.expected_channels)?;
    let lookup = class_lookup(&structural.summary.classes)?;
    let groups = expected_member_keys(&artifact.queries, &lookup);
    if artifact.members.len() != groups.len() {
        return Err(reject(
            "BTOR2 channel property proof member count is noncanonical",
        ));
    }
    let mut evidence_bytes = 0usize;
    for (member, key) in artifact.members.iter().zip(groups.keys()) {
        let class = &structural.summary.classes[key.class_index];
        let backend = if class.len() == 1 {
            Btor2ChannelPropertyBackend::DirectExact
        } else {
            Btor2ChannelPropertyBackend::RepresentativeClass
        };
        if member.class_index != key.class_index
            || member.representative_channel != class[0]
            || member.property != key.property
            || member.horizon != key.horizon
            || member.backend != backend
        {
            return Err(reject(
                "BTOR2 channel property proof member ordering is noncanonical",
            ));
        }
        validate_member_evidence(member)?;
        evidence_bytes = evidence_bytes
            .checked_add(member.evidence.len())
            .filter(|value| *value <= policy.max_evidence_bytes)
            .ok_or_else(|| reject("BTOR2 channel property proof evidence exceeds policy"))?;
    }
    Ok(())
}

/// Encodes the complete property portfolio in canonical, checksummed v1 form.
pub fn encode_btor2_channel_property_proof_artifact(
    artifact: &Btor2ChannelPropertyProofArtifact,
    policy: Btor2ChannelPropertyProofPolicy,
) -> Result<Vec<u8>, Btor2RegionError> {
    validate_property_artifact_shape(artifact, policy)?;
    let evidence_bytes = artifact.members.iter().try_fold(0usize, |total, member| {
        total.checked_add(member.evidence.len())
    });
    let encoded_bytes = (8usize + 4 + 32 + 4 + 4 + 4 + 32)
        .checked_add(artifact.structural_admission.len())
        .and_then(|total| {
            artifact
                .queries
                .len()
                .checked_mul(4 + 4 + 1 + 4)
                .and_then(|query_bytes| total.checked_add(query_bytes))
        })
        .and_then(|total| {
            artifact
                .members
                .len()
                .checked_mul(4 + 4 + 1 + 4 + 1 + 1 + 4)
                .and_then(|member_bytes| total.checked_add(member_bytes))
        })
        .and_then(|total| evidence_bytes.and_then(|value| total.checked_add(value)))
        .filter(|total| *total <= policy.max_artifact_bytes)
        .ok_or_else(|| reject("BTOR2 channel property proof artifact exceeds byte policy"))?;
    let mut bytes = Vec::with_capacity(encoded_bytes);
    bytes.extend_from_slice(CHANNEL_PROPERTY_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.model_sha256);
    push_u32(
        &mut bytes,
        artifact.structural_admission.len(),
        "structural admission length",
    )?;
    bytes.extend_from_slice(&artifact.structural_admission);
    push_u32(&mut bytes, artifact.queries.len(), "property query count")?;
    for query in &artifact.queries {
        bytes.extend_from_slice(&query.query_id.to_le_bytes());
        push_u32(&mut bytes, query.channel_index, "property query channel")?;
        bytes.push(match query.property {
            Btor2ChannelProperty::OutputHigh => 0,
            Btor2ChannelProperty::OutputLow => 1,
        });
        bytes.extend_from_slice(&query.horizon.to_le_bytes());
    }
    push_u32(&mut bytes, artifact.members.len(), "property member count")?;
    for member in &artifact.members {
        push_u32(&mut bytes, member.class_index, "property member class")?;
        push_u32(
            &mut bytes,
            member.representative_channel,
            "property representative channel",
        )?;
        bytes.push(match member.property {
            Btor2ChannelProperty::OutputHigh => 0,
            Btor2ChannelProperty::OutputLow => 1,
        });
        bytes.extend_from_slice(&member.horizon.to_le_bytes());
        bytes.push(match member.backend {
            Btor2ChannelPropertyBackend::RepresentativeClass => 0,
            Btor2ChannelPropertyBackend::DirectExact => 1,
        });
        bytes.push(match member.solver {
            Btor2ChannelPropertySolver::ExplicitState => 0,
            Btor2ChannelPropertySolver::BitblastCnf => 1,
        });
        push_u32(
            &mut bytes,
            member.evidence.len(),
            "property evidence length",
        )?;
        bytes.extend_from_slice(&member.evidence);
    }
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() != encoded_bytes {
        return Err(reject(
            "BTOR2 channel property proof artifact length calculation mismatch",
        ));
    }
    Ok(bytes)
}

struct PropertyArtifactCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PropertyArtifactCursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Btor2RegionError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject("truncated BTOR2 channel property proof artifact"))?;
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, Btor2RegionError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, Btor2RegionError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed integer"),
        ))
    }

    fn count(&mut self, maximum: usize, label: &str) -> Result<usize, Btor2RegionError> {
        let value = usize::try_from(self.u32()?)
            .map_err(|_| reject(format!("{label} exceeds platform range")))?;
        if value == 0 || value > maximum {
            return Err(reject(format!("{label} is outside policy")));
        }
        Ok(value)
    }
}

/// Decodes only canonical property portfolios after bounded allocation preflight.
pub fn decode_btor2_channel_property_proof_artifact(
    bytes: &[u8],
    policy: Btor2ChannelPropertyProofPolicy,
) -> Result<Btor2ChannelPropertyProofArtifact, Btor2RegionError> {
    const MINIMUM_BYTES: usize = 8 + 4 + 32 + 4 + 4 + 4 + 32;
    if bytes.len() < MINIMUM_BYTES || bytes.len() > policy.max_artifact_bytes {
        return Err(reject(
            "BTOR2 channel property proof artifact size is outside policy",
        ));
    }
    let payload_end = bytes.len() - 32;
    let checksum: [u8; 32] = bytes[payload_end..].try_into().expect("fixed checksum");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_end])) != checksum {
        return Err(reject(
            "BTOR2 channel property proof artifact checksum mismatch",
        ));
    }
    let mut cursor = PropertyArtifactCursor {
        bytes: &bytes[..payload_end],
        offset: 0,
    };
    if cursor.take(8)? != CHANNEL_PROPERTY_MAGIC {
        return Err(reject(
            "BTOR2 channel property proof artifact magic mismatch",
        ));
    }
    let version = cursor.u32()?;
    if version != BTOR2_CHANNEL_PROPERTY_PROOF_VERSION {
        return Err(reject(
            "unsupported BTOR2 channel property proof artifact version",
        ));
    }
    let model_sha256 = cursor.take(32)?.try_into().expect("fixed digest");
    let structural_len = cursor.count(
        MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES,
        "structural admission length",
    )?;
    let structural_slice = cursor.take(structural_len)?;
    let structural = decode_btor2_region_equivalence_artifact(structural_slice)?;
    if model_sha256 != structural.model_sha256 {
        return Err(reject(
            "BTOR2 channel property proof structural binding mismatch",
        ));
    }
    let structural_admission = structural_slice.to_vec();
    let query_count = cursor.count(policy.max_queries, "property query count")?;
    let mut queries = Vec::with_capacity(query_count);
    for _ in 0..query_count {
        let query_id = cursor.u32()?;
        let channel_index = usize::try_from(cursor.u32()?)
            .map_err(|_| reject("property query channel exceeds platform range"))?;
        let property = match cursor.u8()? {
            0 => Btor2ChannelProperty::OutputHigh,
            1 => Btor2ChannelProperty::OutputLow,
            _ => return Err(reject("unknown BTOR2 channel property query kind")),
        };
        let horizon = cursor.u32()?;
        queries.push(Btor2ChannelPropertyQuery {
            query_id,
            channel_index,
            property,
            horizon,
        });
    }
    validate_queries(&queries, structural.expected_channels)?;
    let lookup = class_lookup(&structural.summary.classes)?;
    let expected_groups = expected_member_keys(&queries, &lookup);
    let expected_keys = expected_groups.keys().copied().collect::<Vec<_>>();
    let member_count = cursor.count(policy.max_members, "property member count")?;
    if member_count != expected_groups.len() {
        return Err(reject(
            "BTOR2 channel property proof member count is noncanonical",
        ));
    }
    let mut members = Vec::with_capacity(member_count);
    let mut evidence_bytes = 0usize;
    for expected_key in expected_keys {
        let class_index = usize::try_from(cursor.u32()?)
            .map_err(|_| reject("property member class exceeds platform range"))?;
        let representative_channel = usize::try_from(cursor.u32()?)
            .map_err(|_| reject("representative channel exceeds platform range"))?;
        let property = match cursor.u8()? {
            0 => Btor2ChannelProperty::OutputHigh,
            1 => Btor2ChannelProperty::OutputLow,
            _ => return Err(reject("unknown BTOR2 channel property member kind")),
        };
        let horizon = cursor.u32()?;
        let backend = match cursor.u8()? {
            0 => Btor2ChannelPropertyBackend::RepresentativeClass,
            1 => Btor2ChannelPropertyBackend::DirectExact,
            _ => return Err(reject("unknown BTOR2 channel property proof backend")),
        };
        let solver = match cursor.u8()? {
            0 => Btor2ChannelPropertySolver::ExplicitState,
            1 => Btor2ChannelPropertySolver::BitblastCnf,
            _ => return Err(reject("unknown BTOR2 channel property proof solver")),
        };
        let class = &structural.summary.classes[expected_key.class_index];
        let expected_backend = if class.len() == 1 {
            Btor2ChannelPropertyBackend::DirectExact
        } else {
            Btor2ChannelPropertyBackend::RepresentativeClass
        };
        if class_index != expected_key.class_index
            || representative_channel != class[0]
            || property != expected_key.property
            || horizon != expected_key.horizon
            || backend != expected_backend
        {
            return Err(reject(
                "BTOR2 channel property proof member ordering is noncanonical",
            ));
        }
        let maximum_evidence = match solver {
            Btor2ChannelPropertySolver::ExplicitState => btor2_search::MAX_SEARCH_CERTIFICATE_BYTES,
            Btor2ChannelPropertySolver::BitblastCnf => MAX_BITBLAST_CERTIFICATE_BYTES,
        };
        let evidence_len = cursor.count(maximum_evidence, "property evidence length")?;
        evidence_bytes = evidence_bytes
            .checked_add(evidence_len)
            .filter(|value| *value <= policy.max_evidence_bytes)
            .ok_or_else(|| reject("BTOR2 channel property proof evidence exceeds policy"))?;
        let evidence_slice = cursor.take(evidence_len)?;
        let member = Btor2ChannelPropertyProofMember {
            class_index,
            representative_channel,
            property,
            horizon,
            backend,
            solver,
            evidence: evidence_slice.to_vec(),
        };
        members.push(member);
    }
    if cursor.offset != cursor.bytes.len() {
        return Err(reject(
            "trailing BTOR2 channel property proof artifact bytes",
        ));
    }
    let artifact = Btor2ChannelPropertyProofArtifact {
        version,
        model_sha256,
        structural_admission,
        queries,
        members,
    };
    if encode_btor2_channel_property_proof_artifact(&artifact, policy)? != bytes {
        return Err(reject(
            "BTOR2 channel property proof artifact is not canonical",
        ));
    }
    Ok(artifact)
}

/// Produces and canonically encodes one complete source-bound property portfolio.
pub fn produce_btor2_channel_property_proof_bytes(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    region_policy: Btor2RegionPolicy,
    artifact_policy: Btor2ChannelPropertyProofPolicy,
) -> Result<Vec<u8>, Btor2RegionError> {
    produce_btor2_channel_property_proof_bytes_with_policy(
        model_bytes,
        structural_admission,
        queries,
        region_policy,
        Btor2ChannelPropertyProductionPolicy::new(
            artifact_policy,
            MAX_CHANNEL_PROPERTY_PROJECTED_WORK,
        )?,
    )
}

/// Produces a canonical portfolio only after caller-governed aggregate preflight.
pub fn produce_btor2_channel_property_proof_bytes_with_policy(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    region_policy: Btor2RegionPolicy,
    production_policy: Btor2ChannelPropertyProductionPolicy,
) -> Result<Vec<u8>, Btor2RegionError> {
    produce_btor2_channel_property_proof_bytes_observed(
        model_bytes,
        structural_admission,
        queries,
        region_policy,
        production_policy,
    )
    .map(|(_, bytes)| bytes)
}

/// Produces canonical bytes and returns the exact pre-solve plan that admitted
/// them. The plan is computed once and no property solver starts on refusal.
pub fn produce_btor2_channel_property_proof_bytes_observed(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    region_policy: Btor2RegionPolicy,
    production_policy: Btor2ChannelPropertyProductionPolicy,
) -> Result<(Btor2ChannelPropertyProductionPlan, Vec<u8>), Btor2RegionError> {
    produce_btor2_channel_property_proof_bytes_phase_observed(
        model_bytes,
        structural_admission,
        queries,
        region_policy,
        production_policy,
    )
    .map(|(plan, bytes, _)| (plan, bytes))
}

/// Produces canonical bytes and reports diagnostic phase timings without
/// changing the static admission decision or the resulting artifact.
pub fn produce_btor2_channel_property_proof_bytes_phase_observed(
    model_bytes: &[u8],
    structural_admission: &[u8],
    queries: &[Btor2ChannelPropertyQuery],
    region_policy: Btor2RegionPolicy,
    production_policy: Btor2ChannelPropertyProductionPolicy,
) -> Result<
    (
        Btor2ChannelPropertyProductionPlan,
        Vec<u8>,
        Btor2ChannelPropertyProductionPhaseMetrics,
    ),
    Btor2RegionError,
> {
    let total_started = Instant::now();
    let preflight_started = Instant::now();
    let plan = preflight_btor2_channel_property_proof(
        model_bytes,
        structural_admission,
        queries,
        region_policy,
        production_policy,
    )?;
    let preflight_micros = preflight_started.elapsed().as_micros();
    let proof_started = Instant::now();
    let artifact = produce_btor2_channel_property_proof_after_preflight(
        model_bytes,
        structural_admission,
        queries,
        region_policy,
    )?;
    let proof_construction_micros = proof_started.elapsed().as_micros();
    let encoding_started = Instant::now();
    let bytes =
        encode_btor2_channel_property_proof_artifact(&artifact, production_policy.artifact)?;
    let encoding_micros = encoding_started.elapsed().as_micros();
    Ok((
        plan,
        bytes,
        Btor2ChannelPropertyProductionPhaseMetrics {
            preflight_micros,
            proof_construction_micros,
            encoding_micros,
            total_micros: total_started.elapsed().as_micros(),
        },
    ))
}

/// Decodes and independently verifies a complete source-bound property portfolio.
pub fn verify_btor2_channel_property_proof_bytes(
    model_bytes: &[u8],
    expected_queries: &[Btor2ChannelPropertyQuery],
    bytes: &[u8],
    region_policy: Btor2RegionPolicy,
    artifact_policy: Btor2ChannelPropertyProofPolicy,
) -> Result<Btor2ChannelPropertyProofSummary, Btor2RegionError> {
    let artifact = decode_btor2_channel_property_proof_artifact(bytes, artifact_policy)?;
    verify_btor2_channel_property_proof(model_bytes, expected_queries, &artifact, region_policy)
}

#[cfg(test)]
mod tests {
    use super::{replay_unsafe_assignment, unpack_valuation};
    use crate::btor2;

    #[test]
    fn witness_unpacking_preserves_the_full_bitblast_input_domain() {
        let model = btor2::parse_bytes(
            b"1 sort bitvec 64\n2 sort bitvec 1\n3 input 1 wide_input\n4 state 2 held\n5 zero 2\n6 init 2 4 5\n7 next 2 4 4\n8 redor 2 3\n9 bad 8\n",
        )
        .unwrap();
        let values = unpack_valuation(&model, u64::MAX).unwrap();
        assert_eq!(values[&3], u64::MAX);

        let narrow = btor2::parse_bytes(
            b"1 sort bitvec 6\n2 sort bitvec 1\n3 input 1 narrow_input\n4 state 2 held\n5 zero 2\n6 init 2 4 5\n7 next 2 4 4\n8 redor 2 3\n9 bad 8\n",
        )
        .unwrap();
        assert!(unpack_valuation(&narrow, 64).is_err());
    }

    #[test]
    fn target_replay_rejects_an_inadmissible_bad_valuation() {
        let source = b"1 sort bitvec 1\n2 input 1 command\n3 state 1 held\n4 zero 1\n5 init 1 3 4\n6 next 1 3 3\n7 not 1 2\n8 constraint 7\n9 bad 2\n";
        assert!(replay_unsafe_assignment(source, 9, &[1], 0).is_err());
    }
}
