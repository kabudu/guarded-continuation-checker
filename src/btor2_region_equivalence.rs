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
                hashes[channel].update(
                    state
                        .get(local_state)
                        .ok_or_else(|| reject(format!("state {local_state} is unavailable")))?
                        .to_le_bytes(),
                );
            }
            hashes[channel].update(
                model
                    .evaluate(outgoing[channel], &state, &inputs)
                    .map_err(|error| {
                        reject(format!("reachable equivalence observation failed: {error}"))
                    })?
                    .to_le_bytes(),
            );
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
    let mut by_signature = BTreeMap::<[u8; 32], Vec<usize>>::new();
    for signature in &signatures {
        by_signature
            .entry(signature.sha256)
            .or_default()
            .push(signature.channel_index);
    }
    let mut classes = by_signature.into_values().collect::<Vec<_>>();
    classes.sort_by_key(|class| class[0]);
    Ok(Btor2ReachableRegionEquivalenceSummary {
        version: BTOR2_REGION_EQUIVALENCE_VERSION,
        horizon,
        signatures,
        classes,
    })
}
