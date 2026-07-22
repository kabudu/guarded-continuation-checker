//! Exact structural equivalence for extracted repeated BTOR2 channel regions.
//!
//! A channel signature is independent of local node identifiers. Shared input
//! signals retain their exact model identity, while local states are renamed by
//! their canonical position. Equal signatures therefore prove the same local
//! transition and outgoing-observation functions under that state renaming.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::btor2::{self, BinaryOp, Node, NodeId, NodeKind, UnaryOp};
use crate::btor2_region_extract::{
    Btor2NodeRegionOwner, Btor2RegionError, Btor2RegionPolicy, extract_btor2_complete_regions,
};

pub const BTOR2_REGION_EQUIVALENCE_VERSION: u32 = 1;
pub const MAX_REGION_EQUIVALENCE_HORIZON: u32 = 4096;
pub const MAX_REGION_EQUIVALENCE_ARTIFACT_BYTES: usize = 1024 * 1024;
const REACHABLE_ARTIFACT_MAGIC: &[u8; 8] = b"GCCBRE01";

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

/// Derives exact bounded trace classes for a deterministic, input-free model.
///
/// Version 1 accepts exactly one observation edge per channel. It hashes every
/// local state value under the canonical state renaming and that observation at
/// every frame from zero through `horizon`. Equal signatures therefore prove
/// equal reachable bounded traces for this exact source model, not equivalent
/// transition functions for arbitrary environments.
pub fn derive_btor2_reachable_region_equivalence(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    horizon: u32,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ReachableRegionEquivalenceSummary, Btor2RegionError> {
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
    Ok(Btor2ReachableRegionEquivalenceSummary {
        version: BTOR2_REGION_EQUIVALENCE_VERSION,
        horizon,
        signatures,
        classes,
    })
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

/// Independently verifies source binding and every exact bounded trace class.
pub fn verify_btor2_reachable_region_equivalence_artifact(
    model_bytes: &[u8],
    artifact: &Btor2ReachableRegionEquivalenceArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2ReachableRegionEquivalenceSummary, Btor2RegionError> {
    let _ = encode_btor2_reachable_region_equivalence_artifact(artifact)?;
    if <[u8; 32]>::from(Sha256::digest(model_bytes)) != artifact.model_sha256 {
        return Err(reject("reachable-equivalence model digest mismatch"));
    }
    let recomputed = derive_btor2_reachable_region_equivalence(
        model_bytes,
        &artifact.semantic_roots,
        artifact.expected_channels,
        artifact.summary.horizon,
        policy,
    )?;
    if recomputed != artifact.summary {
        return Err(reject(
            "reachable-equivalence artifact disagrees with exact source traces",
        ));
    }
    Ok(recomputed)
}
