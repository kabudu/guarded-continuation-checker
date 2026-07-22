//! Exact structural equivalence for extracted repeated BTOR2 channel regions.
//!
//! A channel signature is independent of local node identifiers. Shared input
//! signals retain their exact model identity, while local states are renamed by
//! their canonical position. Equal signatures therefore prove the same local
//! transition and outgoing-observation functions under that state renaming.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use sha2::{Digest, Sha256};

use crate::btor2::{self, BinaryOp, Node, NodeId, NodeKind, UnaryOp};
use crate::btor2_region_extract::{
    Btor2NodeRegionOwner, Btor2RegionError, Btor2RegionPolicy, extract_btor2_complete_regions,
};

pub const BTOR2_REGION_EQUIVALENCE_VERSION: u32 = 1;
pub const MAX_REGION_EQUIVALENCE_HORIZON: u32 = 4096;
pub const MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES: usize = 1024 * 1024;
pub const MAX_REGION_EQUIVALENCE_QUERIES: usize = 65_536;
const REACHABLE_ARTIFACT_MAGIC: &[u8; 8] = b"GCCBRE01";
const STRUCTURAL_ARTIFACT_MAGIC: &[u8; 8] = b"GCCBSE01";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelEquivalenceSignature {
    pub channel_index: usize,
    pub local_states: usize,
    pub local_nodes: usize,
    pub incoming_signals: usize,
    pub outgoing_signals: usize,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2RegionEquivalenceSummary {
    pub version: u32,
    pub signatures: Vec<Btor2ChannelEquivalenceSignature>,
    pub classes: Vec<Vec<usize>>,
}

/// Canonical source-bound evidence for structural channel classes.
///
/// Verification reparses the separately supplied source, re-extracts complete
/// regions, and recomputes every signature and class. The artifact does not
/// carry authority to declare classes that the source does not reproduce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2RegionEquivalenceArtifact {
    pub version: u32,
    pub model_sha256: [u8; 32],
    pub semantic_roots: Vec<NodeId>,
    pub expected_channels: usize,
    pub summary: Btor2RegionEquivalenceSummary,
}

/// Opaque structural admission capability created only by source replay.
#[derive(Clone, Debug)]
pub struct Btor2RegionEquivalenceAdmission {
    model_sha256: [u8; 32],
    classes: Vec<Vec<usize>>,
}

impl Btor2RegionEquivalenceAdmission {
    pub fn model_sha256(&self) -> [u8; 32] {
        self.model_sha256
    }

    pub fn classes(&self) -> &[Vec<usize>] {
        &self.classes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ReachableChannelSignature {
    pub channel_index: usize,
    pub local_states: usize,
    pub frames: u32,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ReachableRegionEquivalenceSummary {
    pub version: u32,
    pub horizon: u32,
    pub signatures: Vec<Btor2ReachableChannelSignature>,
    pub classes: Vec<Vec<usize>>,
}

/// Canonical, source-bound evidence for bounded reachable channel classes.
///
/// Verification reparses the supplied source and independently recomputes the
/// complete bounded traces. Digests identify traces in the artifact, but class
/// membership is decided from exact trace vectors rather than digest equality.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ReachableRegionEquivalenceArtifact {
    pub version: u32,
    pub model_sha256: [u8; 32],
    pub semantic_roots: Vec<NodeId>,
    pub expected_channels: usize,
    pub summary: Btor2ReachableRegionEquivalenceSummary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelTraceQuery {
    pub query_id: u32,
    pub channel_index: usize,
    pub start_frame: u32,
    pub end_frame: u32,
    pub mask: u64,
    pub value: u64,
}

impl Btor2ChannelTraceQuery {
    pub fn new(
        query_id: u32,
        channel_index: usize,
        start_frame: u32,
        end_frame: u32,
        mask: u64,
        value: u64,
    ) -> Result<Self, Btor2RegionError> {
        if start_frame > end_frame || mask == 0 || value & !mask != 0 {
            return Err(reject(
                "BTOR2 channel trace query window, mask and value are not canonical",
            ));
        }
        Ok(Self {
            query_id,
            channel_index,
            start_frame,
            end_frame,
            mask,
            value,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Btor2TraceQueryBackend {
    RepresentativeClass,
    DirectExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2ChannelTraceResult {
    pub query_id: u32,
    pub channel_index: usize,
    pub matched: bool,
    pub earliest_frame: Option<u32>,
    pub backend: Btor2TraceQueryBackend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2TracePortfolioMetrics {
    pub logical_queries: usize,
    pub representative_classes: usize,
    pub representative_predicate_evaluations: usize,
    pub exact_singleton_predicate_evaluations: usize,
    pub reused_logical_queries: usize,
    pub direct_predicate_evaluation_bound: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2TracePortfolioSummary {
    pub results: Vec<Btor2ChannelTraceResult>,
    pub metrics: Btor2TracePortfolioMetrics,
}

/// Opaque capability created only by independent source replay.
#[derive(Clone, Debug)]
pub struct Btor2ReachableEquivalenceAdmission {
    model_sha256: [u8; 32],
    horizon: u32,
    classes: Vec<Vec<usize>>,
    channel_to_class: Vec<usize>,
    representative_observations: Vec<Vec<u64>>,
}

impl Btor2ReachableEquivalenceAdmission {
    pub fn model_sha256(&self) -> [u8; 32] {
        self.model_sha256
    }

    pub fn horizon(&self) -> u32 {
        self.horizon
    }

    pub fn classes(&self) -> &[Vec<usize>] {
        &self.classes
    }
}

fn reject(message: impl Into<String>) -> Btor2RegionError {
    Btor2RegionError(message.into())
}

fn unary_tag(op: UnaryOp) -> u8 {
    match op {
        UnaryOp::Not => 1,
        UnaryOp::Inc => 2,
        UnaryOp::Dec => 3,
        UnaryOp::Neg => 4,
        UnaryOp::Redor => 5,
        UnaryOp::Redand => 6,
    }
}

fn binary_tag(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::And => 1,
        BinaryOp::Or => 2,
        BinaryOp::Xor => 3,
        BinaryOp::Add => 4,
        BinaryOp::Sub => 5,
        BinaryOp::Mul => 6,
        BinaryOp::Sll => 7,
        BinaryOp::Srl => 8,
        BinaryOp::Eq => 9,
        BinaryOp::Neq => 10,
        BinaryOp::Ult => 11,
        BinaryOp::Ulte => 12,
        BinaryOp::Ugt => 13,
        BinaryOp::Ugte => 14,
    }
}

fn dependency_hash(
    id: NodeId,
    shared: &BTreeMap<NodeId, [u8; 32]>,
    local: &BTreeMap<NodeId, [u8; 32]>,
) -> Result<[u8; 32], Btor2RegionError> {
    local
        .get(&id)
        .or_else(|| shared.get(&id))
        .copied()
        .ok_or_else(|| reject(format!("BTOR2 equivalence dependency {id} is unavailable")))
}

fn canonical_node_hash(
    node: &Node,
    local_state_ordinals: &BTreeMap<NodeId, usize>,
    shared: &BTreeMap<NodeId, [u8; 32]>,
    local: &BTreeMap<NodeId, [u8; 32]>,
) -> Result<[u8; 32], Btor2RegionError> {
    let mut hash = Sha256::new();
    hash.update(node.width.to_le_bytes());
    match node.kind {
        NodeKind::Input => {
            hash.update([1]);
            hash.update(node.id.to_le_bytes());
        }
        NodeKind::State => {
            if let Some(ordinal) = local_state_ordinals.get(&node.id) {
                hash.update([2]);
                hash.update(
                    u32::try_from(*ordinal)
                        .map_err(|_| reject("local state ordinal exceeds range"))?
                        .to_le_bytes(),
                );
            } else {
                hash.update([3]);
                hash.update(node.id.to_le_bytes());
            }
        }
        NodeKind::Constant(value) => {
            hash.update([4]);
            hash.update(value.to_le_bytes());
        }
        NodeKind::Unary(op, value) => {
            hash.update([5, unary_tag(op)]);
            hash.update(dependency_hash(value, shared, local)?);
        }
        NodeKind::Binary(op, left, right) => {
            hash.update([6, binary_tag(op)]);
            hash.update(dependency_hash(left, shared, local)?);
            hash.update(dependency_hash(right, shared, local)?);
        }
        NodeKind::Ite(condition, when_true, when_false) => {
            hash.update([7]);
            hash.update(dependency_hash(condition, shared, local)?);
            hash.update(dependency_hash(when_true, shared, local)?);
            hash.update(dependency_hash(when_false, shared, local)?);
        }
        NodeKind::Slice {
            value,
            upper,
            lower,
        } => {
            hash.update([8]);
            hash.update(upper.to_le_bytes());
            hash.update(lower.to_le_bytes());
            hash.update(dependency_hash(value, shared, local)?);
        }
        NodeKind::Uext { value, amount } => {
            hash.update([9]);
            hash.update(amount.to_le_bytes());
            hash.update(dependency_hash(value, shared, local)?);
        }
        NodeKind::Concat { high, low } => {
            hash.update([10]);
            hash.update(dependency_hash(high, shared, local)?);
            hash.update(dependency_hash(low, shared, local)?);
        }
    }
    Ok(hash.finalize().into())
}

fn push_hashes(hash: &mut Sha256, tag: u8, values: &[[u8; 32]]) {
    hash.update([tag]);
    hash.update((values.len() as u64).to_le_bytes());
    for value in values {
        hash.update(value);
    }
}

/// Derives exact structural equivalence classes for every extracted channel.
///
/// The complete-region extractor first proves ownership and the dependency
/// cut. This function then recomputes local DAG hashes, canonicalising only
/// local state identifiers. Shared state and input leaves retain exact identity,
/// so unequal boundary signals cannot enter the same class.
pub fn derive_btor2_region_equivalence(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RegionEquivalenceSummary, Btor2RegionError> {
    let complete =
        extract_btor2_complete_regions(model_bytes, semantic_roots, expected_channels, policy)?;
    let model = btor2::parse_component_bytes(model_bytes, semantic_roots)
        .map_err(|error| reject(format!("invalid BTOR2 equivalence model: {error}")))?;
    let mut owners = BTreeMap::new();
    for node in &complete.shared_nodes {
        owners.insert(*node, Btor2NodeRegionOwner::Shared);
    }
    for (channel, nodes) in complete.channel_nodes.iter().enumerate() {
        for node in nodes {
            owners.insert(*node, Btor2NodeRegionOwner::Channel(channel));
        }
    }
    for node in &complete.aggregate_nodes {
        owners.insert(*node, Btor2NodeRegionOwner::Aggregate);
    }
    if owners.len() != model.nodes().len() {
        return Err(reject("BTOR2 equivalence ownership is incomplete"));
    }

    let empty_ordinals = BTreeMap::new();
    let mut shared_hashes = BTreeMap::new();
    for (&id, node) in model.nodes() {
        if owners[&id] == Btor2NodeRegionOwner::Shared {
            let value =
                canonical_node_hash(node, &empty_ordinals, &shared_hashes, &BTreeMap::new())?;
            shared_hashes.insert(id, value);
        }
    }

    let mut signatures = Vec::with_capacity(expected_channels);
    for channel in 0..expected_channels {
        let states = &complete.state_regions.channels[channel].states;
        let state_ordinals = states
            .iter()
            .enumerate()
            .map(|(ordinal, state)| (*state, ordinal))
            .collect::<BTreeMap<_, _>>();
        let mut local_hashes = BTreeMap::new();
        for (&id, node) in model.nodes() {
            if owners[&id] == Btor2NodeRegionOwner::Channel(channel) {
                let value =
                    canonical_node_hash(node, &state_ordinals, &shared_hashes, &local_hashes)?;
                local_hashes.insert(id, value);
            }
        }

        let mut transitions = Vec::with_capacity(states.len() * 3);
        for state in states {
            transitions.push(local_hashes[state]);
            transitions.push(dependency_hash(
                model
                    .initialiser(*state)
                    .ok_or_else(|| reject(format!("state {state} has no initialiser")))?,
                &shared_hashes,
                &local_hashes,
            )?);
            transitions.push(dependency_hash(
                model
                    .next_value(*state)
                    .ok_or_else(|| reject(format!("state {state} has no next value")))?,
                &shared_hashes,
                &local_hashes,
            )?);
        }
        let mut incoming = complete
            .shared_to_channel_edges
            .iter()
            .filter(|edge| edge.channel_index == channel)
            .map(|edge| shared_hashes[&edge.source])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        incoming.sort_unstable();
        let mut outgoing = complete
            .channel_to_aggregate_edges
            .iter()
            .filter(|edge| edge.channel_index == channel)
            .map(|edge| local_hashes[&edge.source])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        outgoing.sort_unstable();
        let mut local_nodes = local_hashes.values().copied().collect::<Vec<_>>();
        local_nodes.sort_unstable();

        let mut signature = Sha256::new();
        signature.update(BTOR2_REGION_EQUIVALENCE_VERSION.to_le_bytes());
        push_hashes(&mut signature, 1, &transitions);
        push_hashes(&mut signature, 2, &incoming);
        push_hashes(&mut signature, 3, &outgoing);
        push_hashes(&mut signature, 4, &local_nodes);
        signatures.push(Btor2ChannelEquivalenceSignature {
            channel_index: channel,
            local_states: states.len(),
            local_nodes: local_hashes.len(),
            incoming_signals: incoming.len(),
            outgoing_signals: outgoing.len(),
            sha256: signature.finalize().into(),
        });
    }

    let mut by_signature = BTreeMap::<[u8; 32], Vec<usize>>::new();
    for signature in &signatures {
        by_signature
            .entry(signature.sha256)
            .or_default()
            .push(signature.channel_index);
    }
    let mut classes = by_signature.into_values().collect::<Vec<_>>();
    classes.sort_by_key(|class| class[0]);
    Ok(Btor2RegionEquivalenceSummary {
        version: BTOR2_REGION_EQUIVALENCE_VERSION,
        signatures,
        classes,
    })
}

fn validate_structural_artifact(
    artifact: &Btor2RegionEquivalenceArtifact,
) -> Result<(), Btor2RegionError> {
    let summary = &artifact.summary;
    if artifact.version != BTOR2_REGION_EQUIVALENCE_VERSION
        || summary.version != BTOR2_REGION_EQUIVALENCE_VERSION
        || artifact.semantic_roots.is_empty()
        || artifact.semantic_roots.len() > 1024
        || artifact.semantic_roots.contains(&0)
        || artifact
            .semantic_roots
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || artifact.expected_channels == 0
        || artifact.expected_channels > crate::btor2_region_extract::MAX_REGION_CHANNELS
        || summary.signatures.len() != artifact.expected_channels
        || summary.classes.is_empty()
    {
        return Err(reject(
            "structural-equivalence artifact is outside static policy",
        ));
    }
    if summary
        .signatures
        .iter()
        .enumerate()
        .any(|(channel, signature)| {
            signature.channel_index != channel
                || signature.local_states == 0
                || signature.local_nodes == 0
                || signature.outgoing_signals == 0
        })
    {
        return Err(reject(
            "structural-equivalence signatures are not canonical",
        ));
    }
    let mut members = Vec::with_capacity(artifact.expected_channels);
    for class in &summary.classes {
        if class.is_empty()
            || class.windows(2).any(|pair| pair[0] >= pair[1])
            || class
                .iter()
                .any(|channel| *channel >= artifact.expected_channels)
        {
            return Err(reject("structural-equivalence classes are not canonical"));
        }
        members.extend_from_slice(class);
    }
    members.sort_unstable();
    if members != (0..artifact.expected_channels).collect::<Vec<_>>()
        || summary
            .classes
            .windows(2)
            .any(|pair| pair[0][0] >= pair[1][0])
    {
        return Err(reject(
            "structural-equivalence classes do not form a canonical partition",
        ));
    }
    Ok(())
}

/// Produces deterministic source-bound evidence for structural classes.
pub fn produce_btor2_region_equivalence_artifact(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RegionEquivalenceArtifact, Btor2RegionError> {
    let artifact = Btor2RegionEquivalenceArtifact {
        version: BTOR2_REGION_EQUIVALENCE_VERSION,
        model_sha256: Sha256::digest(model_bytes).into(),
        semantic_roots: semantic_roots.to_vec(),
        expected_channels,
        summary: derive_btor2_region_equivalence(
            model_bytes,
            semantic_roots,
            expected_channels,
            policy,
        )?,
    };
    let _ = encode_btor2_region_equivalence_artifact(&artifact)?;
    Ok(artifact)
}

/// Encodes a structural-equivalence artifact in canonical v1 form.
pub fn encode_btor2_region_equivalence_artifact(
    artifact: &Btor2RegionEquivalenceArtifact,
) -> Result<Vec<u8>, Btor2RegionError> {
    validate_structural_artifact(artifact)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(STRUCTURAL_ARTIFACT_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.model_sha256);
    push_u32(
        &mut bytes,
        artifact.semantic_roots.len(),
        "semantic root count",
    )?;
    for root in &artifact.semantic_roots {
        bytes.extend_from_slice(&root.to_le_bytes());
    }
    push_u32(&mut bytes, artifact.expected_channels, "channel count")?;
    bytes.extend_from_slice(&artifact.summary.version.to_le_bytes());
    push_u32(
        &mut bytes,
        artifact.summary.signatures.len(),
        "signature count",
    )?;
    for signature in &artifact.summary.signatures {
        push_u32(&mut bytes, signature.channel_index, "signature channel")?;
        push_u32(&mut bytes, signature.local_states, "local state count")?;
        push_u32(&mut bytes, signature.local_nodes, "local node count")?;
        push_u32(
            &mut bytes,
            signature.incoming_signals,
            "incoming signal count",
        )?;
        push_u32(
            &mut bytes,
            signature.outgoing_signals,
            "outgoing signal count",
        )?;
        bytes.extend_from_slice(&signature.sha256);
    }
    push_u32(&mut bytes, artifact.summary.classes.len(), "class count")?;
    for class in &artifact.summary.classes {
        push_u32(&mut bytes, class.len(), "class member count")?;
        for channel in class {
            push_u32(&mut bytes, *channel, "class member")?;
        }
    }
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES {
        return Err(reject(
            "structural-equivalence artifact exceeds byte policy",
        ));
    }
    Ok(bytes)
}

/// Derives exact bounded trace classes for a deterministic, input-free model.
///
/// Version 1 accepts exactly one observation edge per channel. It hashes every
/// local state value under the canonical state renaming and that observation at
/// every frame from zero through `horizon`. Exact trace vectors decide class
/// membership; signatures are identifiers rather than the equality oracle.
fn derive_btor2_reachable_region_equivalence_with_observations(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    horizon: u32,
    policy: Btor2RegionPolicy,
) -> Result<(Btor2ReachableRegionEquivalenceSummary, Vec<Vec<u64>>), Btor2RegionError> {
    if horizon > MAX_REGION_EQUIVALENCE_HORIZON {
        return Err(reject("BTOR2 reachable equivalence horizon exceeds policy"));
    }
    let complete =
        extract_btor2_complete_regions(model_bytes, semantic_roots, expected_channels, policy)?;
    let model = btor2::parse_component_bytes(model_bytes, semantic_roots).map_err(|error| {
        reject(format!(
            "invalid BTOR2 reachable-equivalence model: {error}"
        ))
    })?;
    if !model.inputs().is_empty() || !model.constraints().is_empty() {
        return Err(reject(
            "reachable equivalence v1 requires an input-free unconstrained model",
        ));
    }
    let outgoing = (0..expected_channels)
        .map(|channel| {
            let edges = complete
                .channel_to_aggregate_edges
                .iter()
                .filter(|edge| edge.channel_index == channel)
                .collect::<Vec<_>>();
            if edges.len() != 1 {
                return Err(reject(
                    "reachable equivalence v1 requires one observation per channel",
                ));
            }
            Ok(edges[0].source)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut traces = (0..expected_channels)
        .map(|_| Vec::<u64>::new())
        .collect::<Vec<_>>();
    let mut observations = (0..expected_channels)
        .map(|_| Vec::<u64>::new())
        .collect::<Vec<_>>();
    let mut hashes = (0..expected_channels)
        .map(|channel| {
            let mut hash = Sha256::new();
            hash.update(BTOR2_REGION_EQUIVALENCE_VERSION.to_le_bytes());
            hash.update(horizon.to_le_bytes());
            hash.update(
                (complete.state_regions.channels[channel].states.len() as u64).to_le_bytes(),
            );
            hash
        })
        .collect::<Vec<_>>();
    let mut state = model.initial_state().map_err(|error| {
        reject(format!(
            "reachable equivalence initial state failed: {error}"
        ))
    })?;
    let inputs = BTreeMap::new();
    for frame in 0..=horizon {
        for channel in 0..expected_channels {
            hashes[channel].update(frame.to_le_bytes());
            for local_state in &complete.state_regions.channels[channel].states {
                let value = *state
                    .get(local_state)
                    .ok_or_else(|| reject(format!("state {local_state} is unavailable")))?;
                traces[channel].push(value);
                hashes[channel].update(value.to_le_bytes());
            }
            let observation =
                model
                    .evaluate(outgoing[channel], &state, &inputs)
                    .map_err(|error| {
                        reject(format!("reachable equivalence observation failed: {error}"))
                    })?;
            traces[channel].push(observation);
            observations[channel].push(observation);
            hashes[channel].update(observation.to_le_bytes());
        }
        if frame != horizon {
            state = model.step(&state, &inputs).map_err(|error| {
                reject(format!("reachable equivalence transition failed: {error}"))
            })?;
        }
    }
    let frames = horizon
        .checked_add(1)
        .ok_or_else(|| reject("reachable equivalence frame count overflow"))?;
    let signatures = hashes
        .into_iter()
        .enumerate()
        .map(|(channel_index, hash)| Btor2ReachableChannelSignature {
            channel_index,
            local_states: complete.state_regions.channels[channel_index].states.len(),
            frames,
            sha256: hash.finalize().into(),
        })
        .collect::<Vec<_>>();
    let mut by_trace = BTreeMap::<Vec<u64>, Vec<usize>>::new();
    for (channel, trace) in traces.into_iter().enumerate() {
        by_trace.entry(trace).or_default().push(channel);
    }
    let mut classes = by_trace.into_values().collect::<Vec<_>>();
    classes.sort_by_key(|class| class[0]);
    Ok((
        Btor2ReachableRegionEquivalenceSummary {
            version: BTOR2_REGION_EQUIVALENCE_VERSION,
            horizon,
            signatures,
            classes,
        },
        observations,
    ))
}

/// Derives exact bounded trace classes for a deterministic, input-free model.
pub fn derive_btor2_reachable_region_equivalence(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    horizon: u32,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ReachableRegionEquivalenceSummary, Btor2RegionError> {
    Ok(derive_btor2_reachable_region_equivalence_with_observations(
        model_bytes,
        semantic_roots,
        expected_channels,
        horizon,
        policy,
    )?
    .0)
}

fn push_u32(bytes: &mut Vec<u8>, value: usize, label: &str) -> Result<(), Btor2RegionError> {
    bytes.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| reject(format!("{label} exceeds encoding range")))?
            .to_le_bytes(),
    );
    Ok(())
}

fn validate_reachable_artifact(
    artifact: &Btor2ReachableRegionEquivalenceArtifact,
) -> Result<(), Btor2RegionError> {
    let summary = &artifact.summary;
    if artifact.version != BTOR2_REGION_EQUIVALENCE_VERSION
        || summary.version != BTOR2_REGION_EQUIVALENCE_VERSION
        || summary.horizon > MAX_REGION_EQUIVALENCE_HORIZON
        || artifact.semantic_roots.is_empty()
        || artifact.semantic_roots.contains(&0)
        || artifact
            .semantic_roots
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || artifact.expected_channels == 0
        || artifact.expected_channels > crate::btor2_region_extract::MAX_REGION_CHANNELS
        || summary.signatures.len() != artifact.expected_channels
        || summary.classes.is_empty()
    {
        return Err(reject(
            "reachable-equivalence artifact is outside static policy",
        ));
    }
    let frames = summary
        .horizon
        .checked_add(1)
        .ok_or_else(|| reject("reachable-equivalence frame count overflow"))?;
    if summary
        .signatures
        .iter()
        .enumerate()
        .any(|(channel, signature)| {
            signature.channel_index != channel
                || signature.local_states == 0
                || signature.frames != frames
        })
    {
        return Err(reject("reachable-equivalence signatures are not canonical"));
    }
    let mut members = Vec::with_capacity(artifact.expected_channels);
    for class in &summary.classes {
        if class.is_empty()
            || class.windows(2).any(|pair| pair[0] >= pair[1])
            || class
                .iter()
                .any(|channel| *channel >= artifact.expected_channels)
        {
            return Err(reject("reachable-equivalence classes are not canonical"));
        }
        members.extend_from_slice(class);
    }
    members.sort_unstable();
    if members != (0..artifact.expected_channels).collect::<Vec<_>>()
        || summary
            .classes
            .windows(2)
            .any(|pair| pair[0][0] >= pair[1][0])
    {
        return Err(reject(
            "reachable-equivalence classes do not form a canonical partition",
        ));
    }
    Ok(())
}

/// Produces deterministic, source-bound evidence for bounded reachable classes.
pub fn produce_btor2_reachable_region_equivalence_artifact(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    horizon: u32,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ReachableRegionEquivalenceArtifact, Btor2RegionError> {
    let artifact = Btor2ReachableRegionEquivalenceArtifact {
        version: BTOR2_REGION_EQUIVALENCE_VERSION,
        model_sha256: Sha256::digest(model_bytes).into(),
        semantic_roots: semantic_roots.to_vec(),
        expected_channels,
        summary: derive_btor2_reachable_region_equivalence(
            model_bytes,
            semantic_roots,
            expected_channels,
            horizon,
            policy,
        )?,
    };
    let _ = encode_btor2_reachable_region_equivalence_artifact(&artifact)?;
    Ok(artifact)
}

/// Encodes a reachable-equivalence artifact in its canonical v1 wire format.
pub fn encode_btor2_reachable_region_equivalence_artifact(
    artifact: &Btor2ReachableRegionEquivalenceArtifact,
) -> Result<Vec<u8>, Btor2RegionError> {
    validate_reachable_artifact(artifact)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(REACHABLE_ARTIFACT_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.model_sha256);
    push_u32(
        &mut bytes,
        artifact.semantic_roots.len(),
        "semantic root count",
    )?;
    for root in &artifact.semantic_roots {
        bytes.extend_from_slice(&root.to_le_bytes());
    }
    push_u32(&mut bytes, artifact.expected_channels, "channel count")?;
    bytes.extend_from_slice(&artifact.summary.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.summary.horizon.to_le_bytes());
    push_u32(
        &mut bytes,
        artifact.summary.signatures.len(),
        "signature count",
    )?;
    for signature in &artifact.summary.signatures {
        push_u32(&mut bytes, signature.channel_index, "signature channel")?;
        push_u32(&mut bytes, signature.local_states, "local state count")?;
        bytes.extend_from_slice(&signature.frames.to_le_bytes());
        bytes.extend_from_slice(&signature.sha256);
    }
    push_u32(&mut bytes, artifact.summary.classes.len(), "class count")?;
    for class in &artifact.summary.classes {
        push_u32(&mut bytes, class.len(), "class member count")?;
        for channel in class {
            push_u32(&mut bytes, *channel, "class member")?;
        }
    }
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES {
        return Err(reject("reachable-equivalence artifact exceeds byte policy"));
    }
    Ok(bytes)
}

struct ArtifactCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ArtifactCursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Btor2RegionError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("reachable-equivalence artifact offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("truncated reachable-equivalence artifact"))?;
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, Btor2RegionError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed u32"),
        ))
    }

    fn u64(&mut self) -> Result<u64, Btor2RegionError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed u64"),
        ))
    }

    fn count(&mut self, maximum: usize, label: &str) -> Result<usize, Btor2RegionError> {
        let count = usize::try_from(self.u32()?)
            .map_err(|_| reject(format!("{label} exceeds platform range")))?;
        if count > maximum {
            return Err(reject(format!("{label} exceeds static policy")));
        }
        Ok(count)
    }
}

/// Decodes and re-encodes a canonical structural-equivalence artifact.
pub fn decode_btor2_region_equivalence_artifact(
    bytes: &[u8],
) -> Result<Btor2RegionEquivalenceArtifact, Btor2RegionError> {
    if bytes.len() < 8 + 4 * 6 + 32 * 2 || bytes.len() > MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES {
        return Err(reject(
            "structural-equivalence artifact size is outside policy",
        ));
    }
    let payload_end = bytes.len() - 32;
    let expected: [u8; 32] = bytes[payload_end..].try_into().expect("fixed checksum");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_end])) != expected {
        return Err(reject("structural-equivalence artifact checksum mismatch"));
    }
    let mut cursor = ArtifactCursor {
        bytes: &bytes[..payload_end],
        offset: 0,
    };
    if cursor.take(8)? != STRUCTURAL_ARTIFACT_MAGIC {
        return Err(reject("structural-equivalence artifact magic mismatch"));
    }
    let version = cursor.u32()?;
    let model_sha256 = cursor.take(32)?.try_into().expect("fixed digest");
    let root_count = cursor.count(1024, "semantic root count")?;
    let mut semantic_roots = Vec::with_capacity(root_count);
    for _ in 0..root_count {
        semantic_roots.push(cursor.u64()?);
    }
    let expected_channels = cursor.count(
        crate::btor2_region_extract::MAX_REGION_CHANNELS,
        "channel count",
    )?;
    let summary_version = cursor.u32()?;
    let signature_count = cursor.count(expected_channels, "signature count")?;
    let mut signatures = Vec::with_capacity(signature_count);
    for _ in 0..signature_count {
        signatures.push(Btor2ChannelEquivalenceSignature {
            channel_index: cursor.count(expected_channels, "signature channel")?,
            local_states: cursor.count(
                crate::btor2_region_extract::MAX_REGION_STATES,
                "local state count",
            )?,
            local_nodes: cursor.count(
                crate::btor2_region_extract::MAX_REGION_DEPENDENCY_VISITS,
                "local node count",
            )?,
            incoming_signals: cursor.count(
                crate::btor2_region_extract::MAX_REGION_DEPENDENCY_VISITS,
                "incoming signal count",
            )?,
            outgoing_signals: cursor.count(
                crate::btor2_region_extract::MAX_REGION_DEPENDENCY_VISITS,
                "outgoing signal count",
            )?,
            sha256: cursor.take(32)?.try_into().expect("fixed digest"),
        });
    }
    let class_count = cursor.count(expected_channels, "class count")?;
    let mut classes = Vec::with_capacity(class_count);
    let mut total_members = 0usize;
    for _ in 0..class_count {
        let count = cursor.count(expected_channels, "class member count")?;
        total_members = total_members
            .checked_add(count)
            .ok_or_else(|| reject("class member count overflow"))?;
        if total_members > expected_channels {
            return Err(reject("class members exceed channel count"));
        }
        let mut class = Vec::with_capacity(count);
        for _ in 0..count {
            class.push(cursor.count(expected_channels, "class member")?);
        }
        classes.push(class);
    }
    if cursor.offset != payload_end {
        return Err(reject("trailing structural-equivalence artifact bytes"));
    }
    let artifact = Btor2RegionEquivalenceArtifact {
        version,
        model_sha256,
        semantic_roots,
        expected_channels,
        summary: Btor2RegionEquivalenceSummary {
            version: summary_version,
            signatures,
            classes,
        },
    };
    if encode_btor2_region_equivalence_artifact(&artifact)? != bytes {
        return Err(reject("structural-equivalence artifact is not canonical"));
    }
    Ok(artifact)
}

/// Independently verifies source binding and recomputes every structural class.
pub fn verify_btor2_region_equivalence_artifact(
    model_bytes: &[u8],
    artifact: &Btor2RegionEquivalenceArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RegionEquivalenceSummary, Btor2RegionError> {
    let _ = encode_btor2_region_equivalence_artifact(artifact)?;
    if <[u8; 32]>::from(Sha256::digest(model_bytes)) != artifact.model_sha256 {
        return Err(reject("structural-equivalence model digest mismatch"));
    }
    let recomputed = derive_btor2_region_equivalence(
        model_bytes,
        &artifact.semantic_roots,
        artifact.expected_channels,
        policy,
    )?;
    if recomputed != artifact.summary {
        return Err(reject(
            "structural-equivalence artifact disagrees with source",
        ));
    }
    Ok(recomputed)
}

/// Verifies an artifact once and returns an opaque admission capability.
pub fn admit_btor2_region_equivalence_artifact(
    model_bytes: &[u8],
    artifact: &Btor2RegionEquivalenceArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RegionEquivalenceAdmission, Btor2RegionError> {
    let summary = verify_btor2_region_equivalence_artifact(model_bytes, artifact, policy)?;
    Ok(Btor2RegionEquivalenceAdmission {
        model_sha256: artifact.model_sha256,
        classes: summary.classes,
    })
}

/// Decodes and re-encodes the canonical v1 artifact before returning it.
pub fn decode_btor2_reachable_region_equivalence_artifact(
    bytes: &[u8],
) -> Result<Btor2ReachableRegionEquivalenceArtifact, Btor2RegionError> {
    if bytes.len() < 8 + 4 * 7 + 32 * 2 || bytes.len() > MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES {
        return Err(reject(
            "reachable-equivalence artifact size is outside policy",
        ));
    }
    let payload_end = bytes.len() - 32;
    let expected: [u8; 32] = bytes[payload_end..].try_into().expect("fixed checksum");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_end])) != expected {
        return Err(reject("reachable-equivalence artifact checksum mismatch"));
    }
    let mut cursor = ArtifactCursor {
        bytes: &bytes[..payload_end],
        offset: 0,
    };
    if cursor.take(8)? != REACHABLE_ARTIFACT_MAGIC {
        return Err(reject("reachable-equivalence artifact magic mismatch"));
    }
    let version = cursor.u32()?;
    let model_sha256 = cursor.take(32)?.try_into().expect("fixed digest");
    let root_count = cursor.count(1024, "semantic root count")?;
    let mut semantic_roots = Vec::with_capacity(root_count);
    for _ in 0..root_count {
        semantic_roots.push(cursor.u64()?);
    }
    let expected_channels = cursor.count(
        crate::btor2_region_extract::MAX_REGION_CHANNELS,
        "channel count",
    )?;
    let summary_version = cursor.u32()?;
    let horizon = cursor.u32()?;
    let signature_count = cursor.count(expected_channels, "signature count")?;
    let mut signatures = Vec::with_capacity(signature_count);
    for _ in 0..signature_count {
        signatures.push(Btor2ReachableChannelSignature {
            channel_index: cursor.count(expected_channels, "signature channel")?,
            local_states: cursor.count(
                crate::btor2_region_extract::MAX_REGION_STATES,
                "local state count",
            )?,
            frames: cursor.u32()?,
            sha256: cursor.take(32)?.try_into().expect("fixed digest"),
        });
    }
    let class_count = cursor.count(expected_channels, "class count")?;
    let mut classes = Vec::with_capacity(class_count);
    let mut total_members = 0usize;
    for _ in 0..class_count {
        let count = cursor.count(expected_channels, "class member count")?;
        total_members = total_members
            .checked_add(count)
            .ok_or_else(|| reject("class member count overflow"))?;
        if total_members > expected_channels {
            return Err(reject("class members exceed channel count"));
        }
        let mut class = Vec::with_capacity(count);
        for _ in 0..count {
            class.push(cursor.count(expected_channels, "class member")?);
        }
        classes.push(class);
    }
    if cursor.offset != payload_end {
        return Err(reject("trailing reachable-equivalence artifact bytes"));
    }
    let artifact = Btor2ReachableRegionEquivalenceArtifact {
        version,
        model_sha256,
        semantic_roots,
        expected_channels,
        summary: Btor2ReachableRegionEquivalenceSummary {
            version: summary_version,
            horizon,
            signatures,
            classes,
        },
    };
    if encode_btor2_reachable_region_equivalence_artifact(&artifact)? != bytes {
        return Err(reject("reachable-equivalence artifact is not canonical"));
    }
    Ok(artifact)
}

fn replay_btor2_reachable_region_equivalence_artifact(
    model_bytes: &[u8],
    artifact: &Btor2ReachableRegionEquivalenceArtifact,
    policy: Btor2RegionPolicy,
) -> Result<(Btor2ReachableRegionEquivalenceSummary, Vec<Vec<u64>>), Btor2RegionError> {
    let _ = encode_btor2_reachable_region_equivalence_artifact(artifact)?;
    if <[u8; 32]>::from(Sha256::digest(model_bytes)) != artifact.model_sha256 {
        return Err(reject("reachable-equivalence model digest mismatch"));
    }
    let recomputed = derive_btor2_reachable_region_equivalence_with_observations(
        model_bytes,
        &artifact.semantic_roots,
        artifact.expected_channels,
        artifact.summary.horizon,
        policy,
    )?;
    if recomputed.0 != artifact.summary {
        return Err(reject(
            "reachable-equivalence artifact disagrees with exact source traces",
        ));
    }
    Ok(recomputed)
}

/// Independently verifies source binding and every exact bounded trace class.
pub fn verify_btor2_reachable_region_equivalence_artifact(
    model_bytes: &[u8],
    artifact: &Btor2ReachableRegionEquivalenceArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ReachableRegionEquivalenceSummary, Btor2RegionError> {
    Ok(replay_btor2_reachable_region_equivalence_artifact(model_bytes, artifact, policy)?.0)
}

/// Verifies an artifact once and retains only one observation trace per class.
pub fn admit_btor2_reachable_region_equivalence_artifact(
    model_bytes: &[u8],
    artifact: &Btor2ReachableRegionEquivalenceArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ReachableEquivalenceAdmission, Btor2RegionError> {
    let (summary, observations) =
        replay_btor2_reachable_region_equivalence_artifact(model_bytes, artifact, policy)?;
    let mut channel_to_class = vec![usize::MAX; artifact.expected_channels];
    let mut representative_observations = Vec::with_capacity(summary.classes.len());
    for (class_index, class) in summary.classes.iter().enumerate() {
        let representative = observations[class[0]].clone();
        for channel in class {
            if observations[*channel] != representative {
                return Err(reject(
                    "reachable-equivalence class observations disagree after replay",
                ));
            }
            channel_to_class[*channel] = class_index;
        }
        representative_observations.push(representative);
    }
    if channel_to_class.contains(&usize::MAX) {
        return Err(reject("reachable-equivalence admission is incomplete"));
    }
    Ok(Btor2ReachableEquivalenceAdmission {
        model_sha256: artifact.model_sha256,
        horizon: summary.horizon,
        classes: summary.classes,
        channel_to_class,
        representative_observations,
    })
}

fn validate_trace_queries(
    queries: &[Btor2ChannelTraceQuery],
    channels: usize,
    horizon: u32,
) -> Result<(), Btor2RegionError> {
    if queries.is_empty()
        || queries.len() > MAX_REGION_EQUIVALENCE_QUERIES
        || queries
            .windows(2)
            .any(|pair| pair[0].query_id >= pair[1].query_id)
        || queries.iter().any(|query| {
            query.channel_index >= channels
                || query.start_frame > query.end_frame
                || query.end_frame > horizon
                || query.mask == 0
                || query.value & !query.mask != 0
        })
    {
        return Err(reject("BTOR2 channel trace query batch is outside policy"));
    }
    Ok(())
}

fn evaluate_trace_query(
    observations: &[u64],
    query: Btor2ChannelTraceQuery,
) -> Result<(bool, Option<u32>), Btor2RegionError> {
    let start = usize::try_from(query.start_frame)
        .map_err(|_| reject("trace query start frame exceeds platform range"))?;
    let end = usize::try_from(query.end_frame)
        .map_err(|_| reject("trace query end frame exceeds platform range"))?;
    let window = observations
        .get(start..=end)
        .ok_or_else(|| reject("trace query window exceeds admitted observations"))?;
    let earliest = window
        .iter()
        .position(|observation| observation & query.mask == query.value)
        .map(|frame| {
            let absolute = start
                .checked_add(frame)
                .ok_or_else(|| reject("trace query frame overflow"))?;
            u32::try_from(absolute).map_err(|_| reject("trace query frame exceeds encoding range"))
        })
        .transpose()?;
    Ok((earliest.is_some(), earliest))
}

/// Evaluates every query independently from the source, providing the exact
/// fallback and comparison path for the representative portfolio.
pub fn evaluate_btor2_channel_trace_queries_exact(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    horizon: u32,
    queries: &[Btor2ChannelTraceQuery],
    policy: Btor2RegionPolicy,
) -> Result<Btor2TracePortfolioSummary, Btor2RegionError> {
    validate_trace_queries(queries, expected_channels, horizon)?;
    let (_, observations) = derive_btor2_reachable_region_equivalence_with_observations(
        model_bytes,
        semantic_roots,
        expected_channels,
        horizon,
        policy,
    )?;
    let results = queries
        .iter()
        .map(|query| {
            let (matched, earliest_frame) =
                evaluate_trace_query(&observations[query.channel_index], *query)?;
            Ok(Btor2ChannelTraceResult {
                query_id: query.query_id,
                channel_index: query.channel_index,
                matched,
                earliest_frame,
                backend: Btor2TraceQueryBackend::DirectExact,
            })
        })
        .collect::<Result<Vec<_>, Btor2RegionError>>()?;
    Ok(Btor2TracePortfolioSummary {
        results,
        metrics: Btor2TracePortfolioMetrics {
            logical_queries: queries.len(),
            representative_classes: 0,
            representative_predicate_evaluations: 0,
            exact_singleton_predicate_evaluations: queries.len(),
            reused_logical_queries: 0,
            direct_predicate_evaluation_bound: queries.len(),
        },
    })
}

/// Evaluates one trace predicate per equivalence class and exact predicates for
/// singleton classes. Invalid admission evidence propagates and never falls
/// back; only statically non-reusable queries take the direct exact route.
pub fn evaluate_btor2_channel_trace_queries_portfolio(
    admission: &Btor2ReachableEquivalenceAdmission,
    queries: &[Btor2ChannelTraceQuery],
) -> Result<Btor2TracePortfolioSummary, Btor2RegionError> {
    validate_trace_queries(queries, admission.channel_to_class.len(), admission.horizon)?;
    let mut representative_cache =
        HashMap::<(usize, u32, u32, u64, u64), (bool, Option<u32>)>::with_capacity(queries.len());
    let mut representative_predicate_evaluations = 0usize;
    let mut exact_singleton_predicate_evaluations = 0usize;
    let mut reused_logical_queries = 0usize;
    let mut results = Vec::with_capacity(queries.len());
    for query in queries {
        let class_index = admission.channel_to_class[query.channel_index];
        let class = &admission.classes[class_index];
        let (matched, earliest_frame, backend) = if class.len() == 1 {
            exact_singleton_predicate_evaluations += 1;
            let (matched, frame) =
                evaluate_trace_query(&admission.representative_observations[class_index], *query)?;
            (matched, frame, Btor2TraceQueryBackend::DirectExact)
        } else {
            let key = (
                class_index,
                query.start_frame,
                query.end_frame,
                query.mask,
                query.value,
            );
            let cached = representative_cache.get(&key).copied();
            let (matched, frame) = if let Some(result) = cached {
                reused_logical_queries += 1;
                result
            } else {
                representative_predicate_evaluations += 1;
                let result = evaluate_trace_query(
                    &admission.representative_observations[class_index],
                    *query,
                )?;
                representative_cache.insert(key, result);
                result
            };
            (matched, frame, Btor2TraceQueryBackend::RepresentativeClass)
        };
        results.push(Btor2ChannelTraceResult {
            query_id: query.query_id,
            channel_index: query.channel_index,
            matched,
            earliest_frame,
            backend,
        });
    }
    Ok(Btor2TracePortfolioSummary {
        results,
        metrics: Btor2TracePortfolioMetrics {
            logical_queries: queries.len(),
            representative_classes: admission
                .classes
                .iter()
                .filter(|class| class.len() > 1)
                .count(),
            representative_predicate_evaluations,
            exact_singleton_predicate_evaluations,
            reused_logical_queries,
            direct_predicate_evaluation_bound: queries.len(),
        },
    })
}
