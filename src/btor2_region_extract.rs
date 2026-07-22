//! Static repeated-state region extraction for source-attested Yosys BTOR2.
//!
//! Hierarchy-derived symbols propose ownership. The extractor independently
//! recomputes every next-state dependency and rejects feedback from a channel
//! into shared state or direct dependencies between different channels.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::btor2::{self, NodeId, NodeKind};

pub const BTOR2_REGION_EXTRACTION_VERSION: u32 = 1;
pub const MAX_REGION_CHANNELS: usize = 64;
pub const MAX_REGION_STATES: usize = 16_384;
pub const MAX_REGION_DEPENDENCY_VISITS: usize = 1_000_000;
pub const MAX_REGION_ARTIFACT_BYTES: usize = 1024 * 1024;
const ARTIFACT_MAGIC: &[u8; 8] = b"GCCBRX01";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2RegionPolicy {
    max_channels: usize,
    max_states: usize,
    max_dependency_visits: usize,
    max_artifact_bytes: usize,
}

impl Btor2RegionPolicy {
    pub fn new(
        max_channels: usize,
        max_states: usize,
        max_dependency_visits: usize,
        max_artifact_bytes: usize,
    ) -> Result<Self, Btor2RegionError> {
        if max_channels == 0
            || max_channels > MAX_REGION_CHANNELS
            || max_states == 0
            || max_states > MAX_REGION_STATES
            || max_dependency_visits == 0
            || max_dependency_visits > MAX_REGION_DEPENDENCY_VISITS
            || !(64..=MAX_REGION_ARTIFACT_BYTES).contains(&max_artifact_bytes)
        {
            return Err(reject("BTOR2 region policy is outside static limits"));
        }
        Ok(Self {
            max_channels,
            max_states,
            max_dependency_visits,
            max_artifact_bytes,
        })
    }
}

impl Default for Btor2RegionPolicy {
    fn default() -> Self {
        Self {
            max_channels: MAX_REGION_CHANNELS,
            max_states: MAX_REGION_STATES,
            max_dependency_visits: MAX_REGION_DEPENDENCY_VISITS,
            max_artifact_bytes: MAX_REGION_ARTIFACT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2ChannelStateRegion {
    pub channel_index: usize,
    pub states: Vec<NodeId>,
    pub shared_state_dependencies: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2RepeatedStateRegions {
    pub version: u32,
    pub total_states: usize,
    pub shared_states: Vec<NodeId>,
    pub channels: Vec<Btor2ChannelStateRegion>,
    pub dependency_visits: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Btor2NodeRegionOwner {
    Shared,
    Channel(usize),
    Aggregate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2RegionBoundaryEdge {
    pub source: NodeId,
    pub target: NodeId,
    pub channel_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2CompleteRegionSummary {
    pub version: u32,
    pub state_regions: Btor2RepeatedStateRegions,
    pub shared_nodes: Vec<NodeId>,
    pub channel_nodes: Vec<Vec<NodeId>>,
    pub aggregate_nodes: Vec<NodeId>,
    pub shared_to_channel_edges: Vec<Btor2RegionBoundaryEdge>,
    pub channel_to_aggregate_edges: Vec<Btor2RegionBoundaryEdge>,
    pub dependency_visits: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2RegionArtifact {
    pub version: u32,
    pub model_sha256: [u8; 32],
    pub semantic_roots: Vec<NodeId>,
    pub expected_channels: usize,
    pub regions: Btor2RepeatedStateRegions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2RegionError(pub String);

impl fmt::Display for Btor2RegionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Btor2RegionError {}

fn reject(message: impl Into<String>) -> Btor2RegionError {
    Btor2RegionError(message.into())
}

fn channel_from_symbol(symbol: &str) -> Result<Option<usize>, Btor2RegionError> {
    const MARKER: &str = "gen_chan_insts[";
    let Some(start) = symbol.find(MARKER) else {
        if symbol.contains("gen_chan_insts") {
            return Err(reject("malformed Yosys channel hierarchy symbol"));
        }
        return Ok(None);
    };
    if start == 0
        || !matches!(symbol.as_bytes()[start - 1], b'.' | b'\\')
        || symbol[start + MARKER.len()..].contains(MARKER)
    {
        return Err(reject("ambiguous Yosys channel hierarchy symbol"));
    }
    let digits = &symbol[start + MARKER.len()..];
    let end = digits
        .find(']')
        .ok_or_else(|| reject("unterminated Yosys channel hierarchy index"))?;
    let index_text = &digits[..end];
    if index_text.is_empty() || !index_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(reject("invalid Yosys channel hierarchy index"));
    }
    let suffix = &digits[end + 1..];
    if !suffix.starts_with(".u_chan.") {
        return Err(reject("ambiguous Yosys channel hierarchy symbol"));
    }
    index_text
        .parse::<usize>()
        .map(Some)
        .map_err(|_| reject("Yosys channel hierarchy index exceeds range"))
}

fn state_support(
    model: &btor2::Btor2Model,
    root: NodeId,
    visits: &mut usize,
    policy: Btor2RegionPolicy,
) -> Result<BTreeSet<NodeId>, Btor2RegionError> {
    let mut stack = vec![root];
    let mut visited = BTreeSet::new();
    let mut states = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        *visits = visits
            .checked_add(1)
            .ok_or_else(|| reject("BTOR2 region dependency work overflow"))?;
        if *visits > policy.max_dependency_visits {
            return Err(reject("BTOR2 region dependency work exceeds policy"));
        }
        let node = model
            .nodes()
            .get(&id)
            .ok_or_else(|| reject("BTOR2 region references an unknown node"))?;
        match node.kind {
            NodeKind::Input | NodeKind::Constant(_) => {}
            NodeKind::State => {
                states.insert(id);
            }
            NodeKind::Unary(_, value)
            | NodeKind::Slice { value, .. }
            | NodeKind::Uext { value, .. } => stack.push(value),
            NodeKind::Binary(_, left, right) => {
                stack.push(left);
                stack.push(right);
            }
            NodeKind::Ite(condition, then_value, else_value) => {
                stack.push(condition);
                stack.push(then_value);
                stack.push(else_value);
            }
            NodeKind::Concat { high, low } => {
                stack.push(high);
                stack.push(low);
            }
        }
    }
    Ok(states)
}

fn dependencies(kind: &NodeKind) -> ([NodeId; 3], usize) {
    match *kind {
        NodeKind::Input | NodeKind::State | NodeKind::Constant(_) => ([0; 3], 0),
        NodeKind::Unary(_, value)
        | NodeKind::Slice { value, .. }
        | NodeKind::Uext { value, .. } => ([value, 0, 0], 1),
        NodeKind::Binary(_, left, right) => ([left, right, 0], 2),
        NodeKind::Ite(condition, then_value, else_value) => {
            ([condition, then_value, else_value], 3)
        }
        NodeKind::Concat { high, low } => ([high, low, 0], 2),
    }
}

fn reachable_nodes(
    model: &btor2::Btor2Model,
    roots: impl IntoIterator<Item = NodeId>,
    visits: &mut usize,
    policy: Btor2RegionPolicy,
) -> Result<BTreeSet<NodeId>, Btor2RegionError> {
    let mut stack = roots.into_iter().collect::<Vec<_>>();
    let mut reached = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !reached.insert(id) {
            continue;
        }
        *visits = visits
            .checked_add(1)
            .ok_or_else(|| reject("BTOR2 complete-region work overflow"))?;
        if *visits > policy.max_dependency_visits {
            return Err(reject("BTOR2 complete-region work exceeds policy"));
        }
        let node = model
            .nodes()
            .get(&id)
            .ok_or_else(|| reject("BTOR2 complete region references an unknown node"))?;
        let (sources, count) = dependencies(&node.kind);
        stack.extend_from_slice(&sources[..count]);
    }
    Ok(reached)
}

/// Extract and verify state ownership for repeated `gen_chan_insts` regions.
///
/// This establishes only a state-dependency cut. It is not a complete
/// combinational region proof and is therefore insufficient by itself for
/// representative proof reuse.
pub fn extract_btor2_repeated_state_regions(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RepeatedStateRegions, Btor2RegionError> {
    if expected_channels == 0 || expected_channels > policy.max_channels {
        return Err(reject("expected channel count is outside policy"));
    }
    let model = btor2::parse_component_bytes(model_bytes, semantic_roots)
        .map_err(|error| reject(format!("invalid BTOR2 region model: {error}")))?;
    if model.states().len() > policy.max_states {
        return Err(reject("BTOR2 region state count exceeds policy"));
    }

    let mut owners = BTreeMap::<NodeId, Option<usize>>::new();
    let mut groups = vec![Vec::new(); expected_channels];
    let mut shared_states = Vec::new();
    for &state in model.states() {
        let owner = match model.next_symbol(state) {
            Some(symbol) => channel_from_symbol(symbol)?,
            None => None,
        };
        if let Some(index) = owner {
            if index >= expected_channels {
                return Err(reject("channel hierarchy index is outside expected range"));
            }
            groups[index].push(state);
        } else {
            shared_states.push(state);
        }
        owners.insert(state, owner);
    }
    if groups.iter().any(Vec::is_empty) {
        return Err(reject("one or more expected channel regions has no state"));
    }

    let mut visits = 0usize;
    let mut channel_shared = vec![BTreeSet::new(); expected_channels];
    for &state in model.states() {
        let root = model
            .next_value(state)
            .ok_or_else(|| reject("BTOR2 region state lacks next value"))?;
        for dependency in state_support(&model, root, &mut visits, policy)? {
            match (owners[&state], owners[&dependency]) {
                (None, Some(_)) => {
                    return Err(reject("shared state depends on channel-local state"));
                }
                (Some(owner), Some(dependency_owner)) if owner != dependency_owner => {
                    return Err(reject("channel state depends on a different channel"));
                }
                (Some(owner), None) => {
                    channel_shared[owner].insert(dependency);
                }
                _ => {}
            }
        }
    }

    let channels = groups
        .into_iter()
        .enumerate()
        .map(|(channel_index, states)| Btor2ChannelStateRegion {
            channel_index,
            states,
            shared_state_dependencies: channel_shared[channel_index].iter().copied().collect(),
        })
        .collect();
    Ok(Btor2RepeatedStateRegions {
        version: BTOR2_REGION_EXTRACTION_VERSION,
        total_states: model.states().len(),
        shared_states,
        channels,
        dependency_visits: visits,
    })
}

/// Classify every semantic node and verify the complete dependency-edge cut.
///
/// Multi-channel nodes are admitted only as top-level aggregation reachable
/// from declared semantic roots and absent from every next-state cone.
pub fn extract_btor2_complete_regions(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    policy: Btor2RegionPolicy,
) -> Result<Btor2CompleteRegionSummary, Btor2RegionError> {
    let state_regions = extract_btor2_repeated_state_regions(
        model_bytes,
        semantic_roots,
        expected_channels,
        policy,
    )?;
    let model = btor2::parse_component_bytes(model_bytes, semantic_roots)
        .map_err(|error| reject(format!("invalid BTOR2 complete-region model: {error}")))?;
    let mut state_owners = BTreeMap::new();
    for state in &state_regions.shared_states {
        state_owners.insert(*state, Btor2NodeRegionOwner::Shared);
    }
    for channel in &state_regions.channels {
        for state in &channel.states {
            state_owners.insert(*state, Btor2NodeRegionOwner::Channel(channel.channel_index));
        }
    }

    let mut visits = state_regions.dependency_visits;
    let transition_roots = model
        .states()
        .iter()
        .map(|state| {
            model
                .next_value(*state)
                .expect("parsed states have next values")
        })
        .collect::<Vec<_>>();
    let transition_nodes = reachable_nodes(&model, transition_roots, &mut visits, policy)?;
    let observation_nodes =
        reachable_nodes(&model, semantic_roots.iter().copied(), &mut visits, policy)?;

    let mut owners = BTreeMap::<NodeId, Btor2NodeRegionOwner>::new();
    let mut shared_nodes = Vec::new();
    let mut channel_nodes = vec![Vec::new(); expected_channels];
    let mut aggregate_nodes = Vec::new();
    for &id in model.nodes().keys() {
        let owner = if let Some(owner) = state_owners.get(&id) {
            *owner
        } else {
            let support = state_support(&model, id, &mut visits, policy)?;
            let local_owners = support
                .iter()
                .filter_map(|state| match state_owners[state] {
                    Btor2NodeRegionOwner::Channel(index) => Some(index),
                    Btor2NodeRegionOwner::Shared => None,
                    Btor2NodeRegionOwner::Aggregate => unreachable!("states cannot aggregate"),
                })
                .collect::<BTreeSet<_>>();
            match local_owners.len() {
                0 => Btor2NodeRegionOwner::Shared,
                1 => Btor2NodeRegionOwner::Channel(*local_owners.first().expect("one owner")),
                _ => Btor2NodeRegionOwner::Aggregate,
            }
        };
        match owner {
            Btor2NodeRegionOwner::Shared => shared_nodes.push(id),
            Btor2NodeRegionOwner::Channel(index) => channel_nodes[index].push(id),
            Btor2NodeRegionOwner::Aggregate => aggregate_nodes.push(id),
        }
        owners.insert(id, owner);
    }

    let mut shared_to_channel_edges = Vec::new();
    let mut channel_to_aggregate_edges = Vec::new();
    for (&target, node) in model.nodes() {
        let target_owner = owners[&target];
        let (sources, count) = dependencies(&node.kind);
        for source in &sources[..count] {
            let source_owner = owners[source];
            match (target_owner, source_owner) {
                (Btor2NodeRegionOwner::Shared, Btor2NodeRegionOwner::Shared) => {}
                (
                    Btor2NodeRegionOwner::Channel(target_channel),
                    Btor2NodeRegionOwner::Channel(source_channel),
                ) if target_channel == source_channel => {}
                (Btor2NodeRegionOwner::Channel(channel_index), Btor2NodeRegionOwner::Shared) => {
                    shared_to_channel_edges.push(Btor2RegionBoundaryEdge {
                        source: *source,
                        target,
                        channel_index,
                    });
                }
                (Btor2NodeRegionOwner::Aggregate, Btor2NodeRegionOwner::Channel(channel_index)) => {
                    channel_to_aggregate_edges.push(Btor2RegionBoundaryEdge {
                        source: *source,
                        target,
                        channel_index,
                    });
                }
                (Btor2NodeRegionOwner::Aggregate, Btor2NodeRegionOwner::Shared)
                | (Btor2NodeRegionOwner::Aggregate, Btor2NodeRegionOwner::Aggregate) => {}
                _ => {
                    return Err(reject(format!(
                        "BTOR2 dependency edge {source}->{target} crosses invalid owners {source_owner:?}->{target_owner:?}"
                    )));
                }
            }
        }
    }
    if aggregate_nodes
        .iter()
        .any(|node| transition_nodes.contains(node) || !observation_nodes.contains(node))
    {
        return Err(reject(
            "multi-channel aggregation is not confined to semantic observations",
        ));
    }

    Ok(Btor2CompleteRegionSummary {
        version: BTOR2_REGION_EXTRACTION_VERSION,
        state_regions,
        shared_nodes,
        channel_nodes,
        aggregate_nodes,
        shared_to_channel_edges,
        channel_to_aggregate_edges,
        dependency_visits: visits,
    })
}

fn checked_u32(value: usize, label: &str) -> Result<u32, Btor2RegionError> {
    u32::try_from(value).map_err(|_| reject(format!("{label} exceeds encoding range")))
}

fn push_u32(bytes: &mut Vec<u8>, value: usize, label: &str) -> Result<(), Btor2RegionError> {
    bytes.extend_from_slice(&checked_u32(value, label)?.to_le_bytes());
    Ok(())
}

fn encode_regions(
    artifact: &Btor2RegionArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Vec<u8>, Btor2RegionError> {
    if artifact.version != BTOR2_REGION_EXTRACTION_VERSION
        || artifact.expected_channels == 0
        || artifact.expected_channels > policy.max_channels
        || artifact.semantic_roots.is_empty()
        || artifact.semantic_roots.contains(&0)
        || artifact
            .semantic_roots
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || artifact.regions.version != BTOR2_REGION_EXTRACTION_VERSION
        || artifact.regions.total_states > policy.max_states
        || artifact.regions.channels.len() != artifact.expected_channels
        || artifact.regions.dependency_visits > policy.max_dependency_visits
    {
        return Err(reject("BTOR2 region artifact is outside policy"));
    }
    if artifact.regions.shared_states.contains(&0)
        || artifact
            .regions
            .shared_states
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("BTOR2 region shared states are not canonical"));
    }
    let shared = artifact
        .regions
        .shared_states
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut all_states = shared.clone();
    for (expected_index, channel) in artifact.regions.channels.iter().enumerate() {
        if channel.channel_index != expected_index
            || channel.states.is_empty()
            || channel.states.contains(&0)
            || channel.states.windows(2).any(|pair| pair[0] >= pair[1])
            || channel
                .shared_state_dependencies
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || channel
                .shared_state_dependencies
                .iter()
                .any(|state| !shared.contains(state))
            || channel
                .states
                .iter()
                .any(|state| !all_states.insert(*state))
        {
            return Err(reject("BTOR2 region artifact state partition is invalid"));
        }
    }
    if all_states.len() != artifact.regions.total_states {
        return Err(reject("BTOR2 region artifact state total is inconsistent"));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(ARTIFACT_MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.model_sha256);
    push_u32(&mut bytes, artifact.expected_channels, "channel count")?;
    push_u32(
        &mut bytes,
        artifact.semantic_roots.len(),
        "semantic root count",
    )?;
    for root in &artifact.semantic_roots {
        bytes.extend_from_slice(&root.to_le_bytes());
    }
    push_u32(&mut bytes, artifact.regions.total_states, "state count")?;
    push_u32(
        &mut bytes,
        artifact.regions.dependency_visits,
        "dependency visits",
    )?;
    push_u32(
        &mut bytes,
        artifact.regions.shared_states.len(),
        "shared state count",
    )?;
    for state in &artifact.regions.shared_states {
        bytes.extend_from_slice(&state.to_le_bytes());
    }
    push_u32(
        &mut bytes,
        artifact.regions.channels.len(),
        "encoded channel count",
    )?;
    for (expected_index, channel) in artifact.regions.channels.iter().enumerate() {
        debug_assert_eq!(channel.channel_index, expected_index);
        push_u32(&mut bytes, channel.channel_index, "channel index")?;
        push_u32(&mut bytes, channel.states.len(), "channel state count")?;
        for state in &channel.states {
            bytes.extend_from_slice(&state.to_le_bytes());
        }
        push_u32(
            &mut bytes,
            channel.shared_state_dependencies.len(),
            "shared dependency count",
        )?;
        for state in &channel.shared_state_dependencies {
            bytes.extend_from_slice(&state.to_le_bytes());
        }
    }
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > policy.max_artifact_bytes {
        return Err(reject("BTOR2 region artifact exceeds byte policy"));
    }
    Ok(bytes)
}

pub fn encode_btor2_region_artifact(
    artifact: &Btor2RegionArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Vec<u8>, Btor2RegionError> {
    encode_regions(artifact, policy)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Btor2RegionError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("BTOR2 region artifact offset overflow"))?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("truncated BTOR2 region artifact"))?;
        self.offset = end;
        Ok(result)
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
}

fn decode_count(
    cursor: &mut Cursor<'_>,
    maximum: usize,
    label: &str,
) -> Result<usize, Btor2RegionError> {
    let value =
        usize::try_from(cursor.u32()?).map_err(|_| reject(format!("{label} exceeds range")))?;
    if value > maximum {
        return Err(reject(format!("{label} exceeds policy")));
    }
    Ok(value)
}

pub fn decode_btor2_region_artifact(
    bytes: &[u8],
    policy: Btor2RegionPolicy,
) -> Result<Btor2RegionArtifact, Btor2RegionError> {
    if bytes.len() < 8 + 4 + 32 + 4 * 6 + 32 || bytes.len() > policy.max_artifact_bytes {
        return Err(reject("BTOR2 region artifact size is outside policy"));
    }
    let payload_end = bytes.len() - 32;
    let expected: [u8; 32] = bytes[payload_end..].try_into().expect("fixed checksum");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_end])) != expected {
        return Err(reject("BTOR2 region artifact checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..payload_end],
        offset: 0,
    };
    if cursor.take(8)? != ARTIFACT_MAGIC {
        return Err(reject("BTOR2 region artifact magic mismatch"));
    }
    let version = cursor.u32()?;
    let model_sha256 = cursor.take(32)?.try_into().expect("fixed digest");
    let expected_channels = decode_count(&mut cursor, policy.max_channels, "channel count")?;
    let root_count = decode_count(&mut cursor, policy.max_states, "semantic root count")?;
    if root_count == 0 {
        return Err(reject("BTOR2 region artifact has no semantic roots"));
    }
    let mut semantic_roots = Vec::with_capacity(root_count);
    for _ in 0..root_count {
        semantic_roots.push(cursor.u64()?);
    }
    let total_states = decode_count(&mut cursor, policy.max_states, "state count")?;
    let dependency_visits = decode_count(
        &mut cursor,
        policy.max_dependency_visits,
        "dependency visits",
    )?;
    let shared_count = decode_count(&mut cursor, policy.max_states, "shared state count")?;
    let mut shared_states = Vec::with_capacity(shared_count);
    for _ in 0..shared_count {
        shared_states.push(cursor.u64()?);
    }
    let channel_count = decode_count(&mut cursor, policy.max_channels, "encoded channel count")?;
    let mut channels = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        let channel_index =
            usize::try_from(cursor.u32()?).map_err(|_| reject("channel index exceeds range"))?;
        let state_count = decode_count(&mut cursor, policy.max_states, "channel state count")?;
        let mut states = Vec::with_capacity(state_count);
        for _ in 0..state_count {
            states.push(cursor.u64()?);
        }
        let dependency_count =
            decode_count(&mut cursor, policy.max_states, "shared dependency count")?;
        let mut shared_state_dependencies = Vec::with_capacity(dependency_count);
        for _ in 0..dependency_count {
            shared_state_dependencies.push(cursor.u64()?);
        }
        channels.push(Btor2ChannelStateRegion {
            channel_index,
            states,
            shared_state_dependencies,
        });
    }
    if cursor.offset != payload_end {
        return Err(reject("trailing BTOR2 region artifact bytes"));
    }
    let artifact = Btor2RegionArtifact {
        version,
        model_sha256,
        semantic_roots,
        expected_channels,
        regions: Btor2RepeatedStateRegions {
            version,
            total_states,
            shared_states,
            channels,
            dependency_visits,
        },
    };
    if encode_regions(&artifact, policy)? != bytes {
        return Err(reject("BTOR2 region artifact is not canonical"));
    }
    Ok(artifact)
}

pub fn produce_btor2_region_artifact(
    model_bytes: &[u8],
    semantic_roots: &[NodeId],
    expected_channels: usize,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RegionArtifact, Btor2RegionError> {
    let artifact = Btor2RegionArtifact {
        version: BTOR2_REGION_EXTRACTION_VERSION,
        model_sha256: Sha256::digest(model_bytes).into(),
        semantic_roots: semantic_roots.to_vec(),
        expected_channels,
        regions: extract_btor2_repeated_state_regions(
            model_bytes,
            semantic_roots,
            expected_channels,
            policy,
        )?,
    };
    let _ = encode_regions(&artifact, policy)?;
    Ok(artifact)
}

pub fn verify_btor2_region_artifact(
    model_bytes: &[u8],
    artifact: &Btor2RegionArtifact,
    policy: Btor2RegionPolicy,
) -> Result<Btor2RepeatedStateRegions, Btor2RegionError> {
    let _ = encode_regions(artifact, policy)?;
    if <[u8; 32]>::from(Sha256::digest(model_bytes)) != artifact.model_sha256 {
        return Err(reject("BTOR2 region artifact model digest mismatch"));
    }
    let recomputed = extract_btor2_repeated_state_regions(
        model_bytes,
        &artifact.semantic_roots,
        artifact.expected_channels,
        policy,
    )?;
    if recomputed != artifact.regions {
        return Err(reject(
            "BTOR2 region artifact disagrees with model dependencies",
        ));
    }
    Ok(recomputed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN: &[u8] = br#"1 sort bitvec 1
2 state 1 shared
3 state 1 local0
4 state 1 local1
5 zero 1
6 init 1 2 5
7 init 1 3 5
8 init 1 4 5
9 xor 1 3 2
10 xor 1 4 2
11 next 1 2 2 $shared
12 next 1 3 9 $flatten\u_pwm_core.\gen_chan_insts[0].u_chan.$state
13 next 1 4 10 $flatten\u_pwm_core.\gen_chan_insts[1].u_chan.$state
14 output 2 shared_out
"#;

    #[test]
    fn extracts_disjoint_channels_and_shared_dependencies() {
        let regions =
            extract_btor2_repeated_state_regions(CLEAN, &[2], 2, Btor2RegionPolicy::default())
                .unwrap();
        assert_eq!(regions.shared_states, vec![2]);
        assert_eq!(regions.channels[0].states, vec![3]);
        assert_eq!(regions.channels[1].states, vec![4]);
        assert_eq!(regions.channels[0].shared_state_dependencies, vec![2]);
        assert_eq!(regions.channels[1].shared_state_dependencies, vec![2]);
    }

    #[test]
    fn refuses_cross_channel_feedback_and_hostile_symbols() {
        let crossed = std::str::from_utf8(CLEAN)
            .unwrap()
            .replace("10 xor 1 4 2", "10 xor 1 4 3");
        assert!(
            extract_btor2_repeated_state_regions(
                crossed.as_bytes(),
                &[2],
                2,
                Btor2RegionPolicy::default(),
            )
            .is_err()
        );
        let feedback = std::str::from_utf8(CLEAN)
            .unwrap()
            .replace("11 next 1 2 2", "11 next 1 2 3");
        assert!(
            extract_btor2_repeated_state_regions(
                feedback.as_bytes(),
                &[2],
                2,
                Btor2RegionPolicy::default(),
            )
            .is_err()
        );
        for hostile in [
            CLEAN.replace(b"[0]", b"[x]"),
            CLEAN.replace(b".u_chan.", b".other."),
        ] {
            assert!(
                extract_btor2_repeated_state_regions(
                    &hostile,
                    &[2],
                    2,
                    Btor2RegionPolicy::default(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn canonical_artifact_recomputes_regions_and_fails_closed() {
        let policy = Btor2RegionPolicy::default();
        let artifact = produce_btor2_region_artifact(CLEAN, &[2], 2, policy).unwrap();
        let bytes = encode_btor2_region_artifact(&artifact, policy).unwrap();
        let decoded = decode_btor2_region_artifact(&bytes, policy).unwrap();
        assert_eq!(decoded, artifact);
        assert_eq!(
            verify_btor2_region_artifact(CLEAN, &decoded, policy).unwrap(),
            artifact.regions
        );
        assert!(verify_btor2_region_artifact(&CLEAN[..CLEAN.len() - 1], &decoded, policy).is_err());
        for end in 0..bytes.len() {
            assert!(decode_btor2_region_artifact(&bytes[..end], policy).is_err());
        }
        for offset in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[offset] ^= 1;
            assert!(decode_btor2_region_artifact(&changed, policy).is_err());
        }
    }

    trait ReplaceBytes {
        fn replace(&self, from: &[u8], to: &[u8]) -> Vec<u8>;
    }

    impl ReplaceBytes for [u8] {
        fn replace(&self, from: &[u8], to: &[u8]) -> Vec<u8> {
            let position = self
                .windows(from.len())
                .position(|window| window == from)
                .unwrap();
            let mut result = self.to_vec();
            result.splice(position..position + from.len(), to.iter().copied());
            result
        }
    }
}
