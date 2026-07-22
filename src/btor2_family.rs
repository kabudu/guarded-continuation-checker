//! Canonical structural composition for repeated BTOR2 component families.
//!
//! The family grammar deliberately permits a channel input to reference only
//! a declared core input or core semantic root. A channel instance cannot name
//! another channel's state or expressions, so undeclared cross-instance edges
//! are unrepresentable rather than detected after composition.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::btor2::{
    self, BinaryOp, Btor2Model, MAX_BTOR2_BYTES, MAX_BTOR2_NODES, NodeId, NodeKind, UnaryOp,
};

pub const BTOR2_FAMILY_COMPOSITION_VERSION: u32 = 1;
pub const BTOR2_FAMILY_ARTIFACT_VERSION: u32 = 1;
pub const MAX_FAMILY_INSTANCES: usize = 64;
pub const MAX_FAMILY_ROOTS: usize = 256;
pub const MAX_FAMILY_BINDINGS: usize = 1024;
pub const MAX_FAMILY_ARTIFACT_BYTES: usize = 1024 * 1024;
const ARTIFACT_MAGIC: &[u8; 8] = b"GCCBTF01";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FamilyInputBinding {
    CoreInput(usize),
    CoreRoot(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyInstance {
    pub identifier: String,
    pub parameter_sha256: [u8; 32],
    pub input_bindings: Vec<FamilyInputBinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2FamilyPolicy {
    max_artifact_bytes: usize,
    max_instances: usize,
    max_roots_per_component: usize,
    max_expanded_nodes: usize,
    max_expanded_bytes: usize,
}

impl Btor2FamilyPolicy {
    pub fn new(
        max_artifact_bytes: usize,
        max_instances: usize,
        max_roots_per_component: usize,
        max_expanded_nodes: usize,
        max_expanded_bytes: usize,
    ) -> Result<Self, Btor2FamilyError> {
        if max_artifact_bytes == 0
            || max_artifact_bytes > MAX_FAMILY_ARTIFACT_BYTES
            || max_instances == 0
            || max_instances > MAX_FAMILY_INSTANCES
            || max_roots_per_component == 0
            || max_roots_per_component > MAX_FAMILY_ROOTS
            || max_expanded_nodes == 0
            || max_expanded_nodes > MAX_BTOR2_NODES
            || max_expanded_bytes == 0
            || max_expanded_bytes > MAX_BTOR2_BYTES
        {
            return Err(reject("BTOR2 family policy is outside static limits"));
        }
        Ok(Self {
            max_artifact_bytes,
            max_instances,
            max_roots_per_component,
            max_expanded_nodes,
            max_expanded_bytes,
        })
    }

    pub fn max_artifact_bytes(self) -> usize {
        self.max_artifact_bytes
    }

    pub fn max_instances(self) -> usize {
        self.max_instances
    }

    pub fn max_roots_per_component(self) -> usize {
        self.max_roots_per_component
    }

    pub fn max_expanded_nodes(self) -> usize {
        self.max_expanded_nodes
    }

    pub fn max_expanded_bytes(self) -> usize {
        self.max_expanded_bytes
    }
}

impl Default for Btor2FamilyPolicy {
    fn default() -> Self {
        Self {
            max_artifact_bytes: MAX_FAMILY_ARTIFACT_BYTES,
            max_instances: MAX_FAMILY_INSTANCES,
            max_roots_per_component: MAX_FAMILY_ROOTS,
            max_expanded_nodes: MAX_BTOR2_NODES,
            max_expanded_bytes: MAX_BTOR2_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyComposition {
    pub version: u32,
    pub core_sha256: [u8; 32],
    pub channel_sha256: [u8; 32],
    pub expanded_sha256: [u8; 32],
    pub instances: usize,
    pub expanded_nodes: usize,
    pub expanded_states: usize,
    pub expanded_inputs: usize,
    pub expanded_bad_properties: usize,
    pub expanded_model: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyArtifact {
    pub version: u32,
    pub core_sha256: [u8; 32],
    pub channel_sha256: [u8; 32],
    pub expanded_sha256: [u8; 32],
    pub core_roots: Vec<NodeId>,
    pub channel_roots: Vec<NodeId>,
    pub instances: Vec<Btor2FamilyInstance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2FamilyError(pub String);

impl fmt::Display for Btor2FamilyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Btor2FamilyError {}

fn reject(message: impl Into<String>) -> Btor2FamilyError {
    Btor2FamilyError(message.into())
}

fn parse_component(
    label: &str,
    bytes: &[u8],
    roots: &[NodeId],
    policy: Btor2FamilyPolicy,
) -> Result<Btor2Model, Btor2FamilyError> {
    if roots.is_empty() || roots.len() > policy.max_roots_per_component {
        return Err(reject(format!(
            "{label} semantic root count is outside policy"
        )));
    }
    btor2::parse_component_bytes(bytes, roots)
        .map_err(|error| reject(format!("invalid {label} BTOR2 component: {error}")))
}

fn valid_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.len() <= 64
        && identifier.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'_' | b'-'))
        })
}

struct CanonicalWriter {
    text: String,
    next_id: NodeId,
    sort_ids: BTreeMap<u32, NodeId>,
    node_count: usize,
    max_nodes: usize,
    max_bytes: usize,
}

impl CanonicalWriter {
    fn new(widths: BTreeSet<u32>, policy: Btor2FamilyPolicy) -> Result<Self, Btor2FamilyError> {
        let mut writer = Self {
            text: String::new(),
            next_id: 1,
            sort_ids: BTreeMap::new(),
            node_count: 0,
            max_nodes: policy.max_expanded_nodes,
            max_bytes: policy.max_expanded_bytes,
        };
        for width in widths {
            let id = writer.take_id()?;
            writer.sort_ids.insert(width, id);
            writer.line(format_args!("{id} sort bitvec {width}\n"))?;
        }
        Ok(writer)
    }

    fn take_id(&mut self) -> Result<NodeId, Btor2FamilyError> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| reject("BTOR2 family identifier overflow"))?;
        Ok(id)
    }

    fn line(&mut self, arguments: fmt::Arguments<'_>) -> Result<(), Btor2FamilyError> {
        use fmt::Write as _;
        self.text
            .write_fmt(arguments)
            .map_err(|_| reject("BTOR2 family text construction failed"))?;
        if self.text.len() > self.max_bytes {
            return Err(reject("expanded BTOR2 family exceeds byte policy"));
        }
        Ok(())
    }

    fn node_id(&mut self) -> Result<NodeId, Btor2FamilyError> {
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or_else(|| reject("BTOR2 family node count overflow"))?;
        if self.node_count > self.max_nodes {
            return Err(reject("expanded BTOR2 family exceeds node policy"));
        }
        self.take_id()
    }

    fn sort(&self, width: u32) -> Result<NodeId, Btor2FamilyError> {
        self.sort_ids
            .get(&width)
            .copied()
            .ok_or_else(|| reject("missing canonical BTOR2 sort"))
    }
}

fn collect_widths(core: &Btor2Model, channel: &Btor2Model) -> BTreeSet<u32> {
    core.nodes()
        .values()
        .chain(channel.nodes().values())
        .map(|node| node.width)
        .collect()
}

fn mapped(map: &BTreeMap<NodeId, NodeId>, id: NodeId) -> Result<NodeId, Btor2FamilyError> {
    map.get(&id)
        .copied()
        .ok_or_else(|| reject(format!("BTOR2 family references unmapped node {id}")))
}

fn emit_node(
    writer: &mut CanonicalWriter,
    map: &mut BTreeMap<NodeId, NodeId>,
    node: &btor2::Node,
    symbol: &str,
) -> Result<(), Btor2FamilyError> {
    let id = writer.node_id()?;
    let sort = writer.sort(node.width)?;
    match node.kind {
        NodeKind::Input => writer.line(format_args!("{id} input {sort} {symbol}\n"))?,
        NodeKind::State => writer.line(format_args!("{id} state {sort} {symbol}\n"))?,
        NodeKind::Constant(value) => writer.line(format_args!("{id} constd {sort} {value}\n"))?,
        NodeKind::Unary(operator, value) => {
            let operation = match operator {
                UnaryOp::Not => "not",
                UnaryOp::Inc => "inc",
                UnaryOp::Dec => "dec",
                UnaryOp::Neg => "neg",
                UnaryOp::Redor => "redor",
                UnaryOp::Redand => "redand",
            };
            let value = mapped(map, value)?;
            writer.line(format_args!("{id} {operation} {sort} {value}\n"))?;
        }
        NodeKind::Binary(operator, left, right) => {
            let operation = match operator {
                BinaryOp::And => "and",
                BinaryOp::Or => "or",
                BinaryOp::Xor => "xor",
                BinaryOp::Add => "add",
                BinaryOp::Sub => "sub",
                BinaryOp::Mul => "mul",
                BinaryOp::Eq => "eq",
                BinaryOp::Neq => "neq",
                BinaryOp::Ult => "ult",
                BinaryOp::Ulte => "ulte",
                BinaryOp::Ugt => "ugt",
                BinaryOp::Ugte => "ugte",
            };
            let left = mapped(map, left)?;
            let right = mapped(map, right)?;
            writer.line(format_args!("{id} {operation} {sort} {left} {right}\n"))?;
        }
        NodeKind::Ite(condition, then_value, else_value) => {
            let condition = mapped(map, condition)?;
            let then_value = mapped(map, then_value)?;
            let else_value = mapped(map, else_value)?;
            writer.line(format_args!(
                "{id} ite {sort} {condition} {then_value} {else_value}\n"
            ))?;
        }
        NodeKind::Slice {
            value,
            upper,
            lower,
        } => {
            let value = mapped(map, value)?;
            writer.line(format_args!("{id} slice {sort} {value} {upper} {lower}\n"))?;
        }
        NodeKind::Uext { value, amount } => {
            let value = mapped(map, value)?;
            writer.line(format_args!("{id} uext {sort} {value} {amount}\n"))?;
        }
        NodeKind::Concat { high, low } => {
            let high = mapped(map, high)?;
            let low = mapped(map, low)?;
            writer.line(format_args!("{id} concat {sort} {high} {low}\n"))?;
        }
    }
    map.insert(node.id, id);
    Ok(())
}

fn emit_state_edges(
    writer: &mut CanonicalWriter,
    model: &Btor2Model,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<(), Btor2FamilyError> {
    for &state in model.states() {
        let mapped_state = mapped(map, state)?;
        let sort = writer.sort(model.nodes()[&state].width)?;
        let initialiser = mapped(
            map,
            model
                .initialiser(state)
                .ok_or_else(|| reject("BTOR2 family state lacks initialiser"))?,
        )?;
        let next = mapped(
            map,
            model
                .next_value(state)
                .ok_or_else(|| reject("BTOR2 family state lacks next value"))?,
        )?;
        let init_id = writer.take_id()?;
        writer.line(format_args!(
            "{init_id} init {sort} {mapped_state} {initialiser}\n"
        ))?;
        let next_id = writer.take_id()?;
        writer.line(format_args!(
            "{next_id} next {sort} {mapped_state} {next}\n"
        ))?;
    }
    Ok(())
}

fn emit_constraints(
    writer: &mut CanonicalWriter,
    model: &Btor2Model,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<(), Btor2FamilyError> {
    for (_, expression) in model.constraints() {
        let id = writer.take_id()?;
        let expression = mapped(map, *expression)?;
        writer.line(format_args!("{id} constraint {expression}\n"))?;
    }
    Ok(())
}

fn channel_bindings(
    core: &Btor2Model,
    core_roots: &[NodeId],
    channel: &Btor2Model,
    instance: &Btor2FamilyInstance,
    core_map: &BTreeMap<NodeId, NodeId>,
) -> Result<BTreeMap<NodeId, NodeId>, Btor2FamilyError> {
    if instance.input_bindings.len() != channel.inputs().len() {
        return Err(reject(format!(
            "instance {} input binding count mismatch",
            instance.identifier
        )));
    }
    let mut map = BTreeMap::new();
    for (&input, binding) in channel.inputs().iter().zip(&instance.input_bindings) {
        let source = match *binding {
            FamilyInputBinding::CoreInput(index) => core
                .inputs()
                .get(index)
                .copied()
                .ok_or_else(|| reject("family binding core input index is outside range"))?,
            FamilyInputBinding::CoreRoot(index) => core_roots
                .get(index)
                .copied()
                .ok_or_else(|| reject("family binding core root index is outside range"))?,
        };
        if channel.nodes()[&input].width != core.nodes()[&source].width {
            return Err(reject(format!(
                "instance {} input binding width mismatch",
                instance.identifier
            )));
        }
        map.insert(input, mapped(core_map, source)?);
    }
    Ok(map)
}

/// Compose one core and one repeated channel relation into canonical BTOR2.
///
/// The returned bytes are parsed again with the strict BTOR2 parser. Success
/// therefore means the expanded relation is syntactically canonical, within
/// the caller's limits, and contains one bad root per channel semantic root.
pub fn compose_btor2_channel_family(
    core_bytes: &[u8],
    core_roots: &[NodeId],
    channel_bytes: &[u8],
    channel_roots: &[NodeId],
    instances: &[Btor2FamilyInstance],
    policy: Btor2FamilyPolicy,
) -> Result<Btor2FamilyComposition, Btor2FamilyError> {
    if instances.is_empty() || instances.len() > policy.max_instances {
        return Err(reject("BTOR2 family instance count is outside policy"));
    }
    let mut previous: Option<&str> = None;
    let mut parameter: Option<[u8; 32]> = None;
    for instance in instances {
        if !valid_identifier(&instance.identifier)
            || previous.is_some_and(|value| value >= instance.identifier.as_str())
        {
            return Err(reject(
                "BTOR2 family identifiers must be canonical, unique, and strictly ordered",
            ));
        }
        if parameter
            .replace(instance.parameter_sha256)
            .is_some_and(|value| value != instance.parameter_sha256)
        {
            return Err(reject("BTOR2 family parameter digests do not match"));
        }
        previous = Some(&instance.identifier);
    }

    let core = parse_component("core", core_bytes, core_roots, policy)?;
    let channel = parse_component("channel", channel_bytes, channel_roots, policy)?;
    if !core.bad_properties().is_empty() || !channel.bad_properties().is_empty() {
        return Err(reject(
            "BTOR2 family components must expose semantic roots instead of embedded bad properties",
        ));
    }
    let widths = collect_widths(&core, &channel);
    let mut writer = CanonicalWriter::new(widths, policy)?;
    let mut core_map = BTreeMap::new();

    for node in core.nodes().values() {
        let index = match node.kind {
            NodeKind::Input => core
                .inputs()
                .iter()
                .position(|id| *id == node.id)
                .unwrap_or(0),
            NodeKind::State => core
                .states()
                .iter()
                .position(|id| *id == node.id)
                .unwrap_or(0),
            _ => 0,
        };
        let symbol = match node.kind {
            NodeKind::Input => format!("core.input.{index}"),
            NodeKind::State => format!("core.state.{index}"),
            _ => format!("core.node.{}", node.id),
        };
        emit_node(&mut writer, &mut core_map, node, &symbol)?;
    }
    emit_state_edges(&mut writer, &core, &core_map)?;
    emit_constraints(&mut writer, &core, &core_map)?;

    for instance in instances {
        let mut map = channel_bindings(&core, core_roots, &channel, instance, &core_map)?;
        for node in channel.nodes().values() {
            if matches!(node.kind, NodeKind::Input) {
                continue;
            }
            let index = if matches!(node.kind, NodeKind::State) {
                channel
                    .states()
                    .iter()
                    .position(|id| *id == node.id)
                    .unwrap_or(0)
            } else {
                0
            };
            let symbol = if matches!(node.kind, NodeKind::State) {
                format!("{}.state.{index}", instance.identifier)
            } else {
                format!("{}.node.{}", instance.identifier, node.id)
            };
            emit_node(&mut writer, &mut map, node, &symbol)?;
        }
        emit_state_edges(&mut writer, &channel, &map)?;
        emit_constraints(&mut writer, &channel, &map)?;
        for (root_index, root) in channel_roots.iter().enumerate() {
            if channel.nodes()[root].width != 1 {
                return Err(reject("channel family bad root must have width one"));
            }
            let id = writer.take_id()?;
            let expression = mapped(&map, *root)?;
            writer.line(format_args!(
                "{id} bad {expression} {}.bad.{root_index}\n",
                instance.identifier
            ))?;
        }
    }

    let expanded_model = writer.text.into_bytes();
    if expanded_model.len() > policy.max_expanded_bytes {
        return Err(reject("expanded BTOR2 family exceeds byte policy"));
    }
    let checked = btor2::parse_bytes(&expanded_model)
        .map_err(|error| reject(format!("expanded BTOR2 family failed reparse: {error}")))?;
    if checked.nodes().len() != writer.node_count
        || checked.bad_properties().len()
            != instances
                .len()
                .checked_mul(channel_roots.len())
                .ok_or_else(|| reject("BTOR2 family property count overflow"))?
    {
        return Err(reject("expanded BTOR2 family structural count mismatch"));
    }

    Ok(Btor2FamilyComposition {
        version: BTOR2_FAMILY_COMPOSITION_VERSION,
        core_sha256: Sha256::digest(core_bytes).into(),
        channel_sha256: Sha256::digest(channel_bytes).into(),
        expanded_sha256: Sha256::digest(&expanded_model).into(),
        instances: instances.len(),
        expanded_nodes: checked.nodes().len(),
        expanded_states: checked.states().len(),
        expanded_inputs: checked.inputs().len(),
        expanded_bad_properties: checked.bad_properties().len(),
        expanded_model,
    })
}

fn checked_u32(value: usize, label: &str) -> Result<u32, Btor2FamilyError> {
    u32::try_from(value).map_err(|_| reject(format!("{label} exceeds canonical integer range")))
}

fn validate_artifact_shape(
    artifact: &Btor2FamilyArtifact,
    policy: Btor2FamilyPolicy,
) -> Result<(), Btor2FamilyError> {
    if artifact.version != BTOR2_FAMILY_ARTIFACT_VERSION {
        return Err(reject("BTOR2 family artifact version mismatch"));
    }
    if artifact.core_roots.is_empty()
        || artifact.core_roots.len() > policy.max_roots_per_component
        || artifact.channel_roots.is_empty()
        || artifact.channel_roots.len() > policy.max_roots_per_component
        || artifact.instances.is_empty()
        || artifact.instances.len() > policy.max_instances
    {
        return Err(reject(
            "BTOR2 family artifact dimensions are outside policy",
        ));
    }
    if artifact
        .core_roots
        .windows(2)
        .chain(artifact.channel_roots.windows(2))
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject(
            "BTOR2 family artifact roots must be unique and strictly ordered",
        ));
    }
    let mut previous: Option<&str> = None;
    let parameter = artifact.instances[0].parameter_sha256;
    for instance in &artifact.instances {
        if !valid_identifier(&instance.identifier)
            || previous.is_some_and(|value| value >= instance.identifier.as_str())
            || instance.parameter_sha256 != parameter
            || instance.input_bindings.len() > MAX_FAMILY_BINDINGS
        {
            return Err(reject(
                "BTOR2 family artifact instance table is non-canonical",
            ));
        }
        previous = Some(&instance.identifier);
    }
    Ok(())
}

/// Produce a compact family artifact and the independently reproducible model.
pub fn produce_btor2_family_artifact(
    core_bytes: &[u8],
    core_roots: &[NodeId],
    channel_bytes: &[u8],
    channel_roots: &[NodeId],
    parameter_bytes: &[u8],
    instances: &[Btor2FamilyInstance],
    policy: Btor2FamilyPolicy,
) -> Result<(Btor2FamilyArtifact, Btor2FamilyComposition), Btor2FamilyError> {
    let parameter_sha256: [u8; 32] = Sha256::digest(parameter_bytes).into();
    if instances
        .iter()
        .any(|instance| instance.parameter_sha256 != parameter_sha256)
    {
        return Err(reject("BTOR2 family parameter digest mismatch"));
    }
    let composition = compose_btor2_channel_family(
        core_bytes,
        core_roots,
        channel_bytes,
        channel_roots,
        instances,
        policy,
    )?;
    let artifact = Btor2FamilyArtifact {
        version: BTOR2_FAMILY_ARTIFACT_VERSION,
        core_sha256: composition.core_sha256,
        channel_sha256: composition.channel_sha256,
        expanded_sha256: composition.expanded_sha256,
        core_roots: core_roots.to_vec(),
        channel_roots: channel_roots.to_vec(),
        instances: instances.to_vec(),
    };
    validate_artifact_shape(&artifact, policy)?;
    let _ = encode_btor2_family_artifact(&artifact, policy)?;
    Ok((artifact, composition))
}

/// Reconstruct and authenticate a family model from separately supplied bytes.
pub fn verify_btor2_family_artifact(
    core_bytes: &[u8],
    channel_bytes: &[u8],
    parameter_bytes: &[u8],
    artifact: &Btor2FamilyArtifact,
    policy: Btor2FamilyPolicy,
) -> Result<Btor2FamilyComposition, Btor2FamilyError> {
    validate_artifact_shape(artifact, policy)?;
    let _ = encode_btor2_family_artifact(artifact, policy)?;
    if <[u8; 32]>::from(Sha256::digest(core_bytes)) != artifact.core_sha256 {
        return Err(reject("BTOR2 family core digest mismatch"));
    }
    if <[u8; 32]>::from(Sha256::digest(channel_bytes)) != artifact.channel_sha256 {
        return Err(reject("BTOR2 family channel digest mismatch"));
    }
    let parameter_sha256: [u8; 32] = Sha256::digest(parameter_bytes).into();
    if artifact
        .instances
        .iter()
        .any(|instance| instance.parameter_sha256 != parameter_sha256)
    {
        return Err(reject("BTOR2 family parameter digest mismatch"));
    }
    let composition = compose_btor2_channel_family(
        core_bytes,
        &artifact.core_roots,
        channel_bytes,
        &artifact.channel_roots,
        &artifact.instances,
        policy,
    )?;
    if composition.expanded_sha256 != artifact.expanded_sha256 {
        return Err(reject("BTOR2 family expanded model digest mismatch"));
    }
    Ok(composition)
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Encode the canonical, checksummed family artifact format.
pub fn encode_btor2_family_artifact(
    artifact: &Btor2FamilyArtifact,
    policy: Btor2FamilyPolicy,
) -> Result<Vec<u8>, Btor2FamilyError> {
    validate_artifact_shape(artifact, policy)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(ARTIFACT_MAGIC);
    push_u32(&mut bytes, artifact.version);
    bytes.extend_from_slice(&artifact.core_sha256);
    bytes.extend_from_slice(&artifact.channel_sha256);
    bytes.extend_from_slice(&artifact.expanded_sha256);
    push_u32(
        &mut bytes,
        checked_u32(artifact.core_roots.len(), "core root count")?,
    );
    for root in &artifact.core_roots {
        push_u64(&mut bytes, *root);
    }
    push_u32(
        &mut bytes,
        checked_u32(artifact.channel_roots.len(), "channel root count")?,
    );
    for root in &artifact.channel_roots {
        push_u64(&mut bytes, *root);
    }
    push_u32(
        &mut bytes,
        checked_u32(artifact.instances.len(), "family instance count")?,
    );
    for instance in &artifact.instances {
        let identifier_len = u16::try_from(instance.identifier.len())
            .map_err(|_| reject("family instance identifier is too long"))?;
        push_u16(&mut bytes, identifier_len);
        bytes.extend_from_slice(instance.identifier.as_bytes());
        bytes.extend_from_slice(&instance.parameter_sha256);
        push_u32(
            &mut bytes,
            checked_u32(instance.input_bindings.len(), "family binding count")?,
        );
        for binding in &instance.input_bindings {
            let (tag, index) = match *binding {
                FamilyInputBinding::CoreInput(index) => (0u8, index),
                FamilyInputBinding::CoreRoot(index) => (1u8, index),
            };
            bytes.push(tag);
            push_u32(&mut bytes, checked_u32(index, "family binding index")?);
        }
    }
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > policy.max_artifact_bytes {
        return Err(reject("BTOR2 family artifact exceeds byte policy"));
    }
    Ok(bytes)
}

struct ArtifactCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ArtifactCursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Btor2FamilyError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject("truncated BTOR2 family artifact"))?;
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, Btor2FamilyError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, Btor2FamilyError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("fixed length"),
        ))
    }

    fn u32(&mut self) -> Result<u32, Btor2FamilyError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed length"),
        ))
    }

    fn u64(&mut self) -> Result<u64, Btor2FamilyError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed length"),
        ))
    }

    fn digest(&mut self) -> Result<[u8; 32], Btor2FamilyError> {
        Ok(self.take(32)?.try_into().expect("fixed length"))
    }
}

fn bounded_count(value: u32, maximum: usize, label: &str) -> Result<usize, Btor2FamilyError> {
    let value = usize::try_from(value).map_err(|_| reject(format!("invalid {label}")))?;
    if value == 0 || value > maximum {
        return Err(reject(format!("{label} is outside policy")));
    }
    Ok(value)
}

fn bounded_count_allow_zero(
    value: u32,
    maximum: usize,
    label: &str,
) -> Result<usize, Btor2FamilyError> {
    let value = usize::try_from(value).map_err(|_| reject(format!("invalid {label}")))?;
    if value > maximum {
        return Err(reject(format!("{label} is outside policy")));
    }
    Ok(value)
}

/// Decode only the canonical family artifact format under caller limits.
pub fn decode_btor2_family_artifact(
    bytes: &[u8],
    policy: Btor2FamilyPolicy,
) -> Result<Btor2FamilyArtifact, Btor2FamilyError> {
    const MINIMUM_BYTES: usize = 8 + 4 + 32 + 32 + 32 + 4 + 8 + 4 + 8 + 4 + 32;
    if bytes.len() < MINIMUM_BYTES || bytes.len() > policy.max_artifact_bytes {
        return Err(reject("BTOR2 family artifact size is outside policy"));
    }
    let payload_len = bytes
        .len()
        .checked_sub(32)
        .ok_or_else(|| reject("truncated BTOR2 family artifact"))?;
    let expected: [u8; 32] = bytes[payload_len..].try_into().expect("fixed suffix");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_len])) != expected {
        return Err(reject("BTOR2 family artifact checksum mismatch"));
    }
    let mut cursor = ArtifactCursor {
        bytes: &bytes[..payload_len],
        offset: 0,
    };
    if cursor.take(8)? != ARTIFACT_MAGIC {
        return Err(reject("BTOR2 family artifact magic mismatch"));
    }
    let version = cursor.u32()?;
    let core_sha256 = cursor.digest()?;
    let channel_sha256 = cursor.digest()?;
    let expanded_sha256 = cursor.digest()?;
    let core_count = bounded_count(
        cursor.u32()?,
        policy.max_roots_per_component,
        "core root count",
    )?;
    let mut core_roots = Vec::with_capacity(core_count);
    for _ in 0..core_count {
        core_roots.push(cursor.u64()?);
    }
    let channel_count = bounded_count(
        cursor.u32()?,
        policy.max_roots_per_component,
        "channel root count",
    )?;
    let mut channel_roots = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        channel_roots.push(cursor.u64()?);
    }
    let instance_count =
        bounded_count(cursor.u32()?, policy.max_instances, "family instance count")?;
    let mut instances = Vec::with_capacity(instance_count);
    for _ in 0..instance_count {
        let identifier_len = usize::from(cursor.u16()?);
        if identifier_len == 0 || identifier_len > 64 {
            return Err(reject("family instance identifier length is outside limit"));
        }
        let identifier = std::str::from_utf8(cursor.take(identifier_len)?)
            .map_err(|_| reject("family instance identifier is not UTF-8"))?
            .to_string();
        let parameter_sha256 = cursor.digest()?;
        let binding_count =
            bounded_count_allow_zero(cursor.u32()?, MAX_FAMILY_BINDINGS, "family binding count")?;
        let mut input_bindings = Vec::with_capacity(binding_count);
        for _ in 0..binding_count {
            let tag = cursor.u8()?;
            let index = usize::try_from(cursor.u32()?)
                .map_err(|_| reject("family binding index is outside range"))?;
            input_bindings.push(match tag {
                0 => FamilyInputBinding::CoreInput(index),
                1 => FamilyInputBinding::CoreRoot(index),
                _ => return Err(reject("unknown family input binding tag")),
            });
        }
        instances.push(Btor2FamilyInstance {
            identifier,
            parameter_sha256,
            input_bindings,
        });
    }
    if cursor.offset != cursor.bytes.len() {
        return Err(reject("trailing BTOR2 family artifact bytes"));
    }
    let artifact = Btor2FamilyArtifact {
        version,
        core_sha256,
        channel_sha256,
        expanded_sha256,
        core_roots,
        channel_roots,
        instances,
    };
    validate_artifact_shape(&artifact, policy)?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

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
10 output 9 mismatch
"#;
    const PARAMETERS: &[u8] = b"phase_width=1\n";

    const OPENTITAN_CORE: &[u8] =
        include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-after.btor2");
    const OPENTITAN_CHANNEL: &[u8] = include_bytes!(
        "../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-after.btor2"
    );

    fn instance(identifier: &str) -> Btor2FamilyInstance {
        Btor2FamilyInstance {
            identifier: identifier.to_string(),
            parameter_sha256: Sha256::digest(PARAMETERS).into(),
            input_bindings: vec![
                FamilyInputBinding::CoreRoot(0),
                FamilyInputBinding::CoreInput(0),
            ],
        }
    }

    #[test]
    fn composes_disjoint_channel_state_and_reparses_exactly() {
        let instances = [instance("channel0"), instance("channel1")];
        let first = compose_btor2_channel_family(
            CORE,
            &[3],
            CHANNEL,
            &[9],
            &instances,
            Btor2FamilyPolicy::default(),
        )
        .unwrap();
        let second = compose_btor2_channel_family(
            CORE,
            &[3],
            CHANNEL,
            &[9],
            &instances,
            Btor2FamilyPolicy::default(),
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.instances, 2);
        assert_eq!(first.expanded_states, 3);
        assert_eq!(first.expanded_inputs, 1);
        assert_eq!(first.expanded_bad_properties, 2);
        assert_eq!(
            Sha256::digest(&first.expanded_model).as_slice(),
            first.expanded_sha256
        );
        let text = std::str::from_utf8(&first.expanded_model).unwrap();
        assert!(text.contains("channel0.state.0"));
        assert!(text.contains("channel1.state.0"));
    }

    #[test]
    fn rejects_noncanonical_or_duplicate_instance_identifiers() {
        for instances in [
            vec![instance("channel1"), instance("channel0")],
            vec![instance("channel0"), instance("channel0")],
            vec![instance("Channel0")],
        ] {
            assert!(
                compose_btor2_channel_family(
                    CORE,
                    &[3],
                    CHANNEL,
                    &[9],
                    &instances,
                    Btor2FamilyPolicy::default(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_changed_parameter_or_incomplete_bindings() {
        let mut changed = instance("channel1");
        changed.parameter_sha256 = [8; 32];
        assert!(
            compose_btor2_channel_family(
                CORE,
                &[3],
                CHANNEL,
                &[9],
                &[instance("channel0"), changed],
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );

        let mut incomplete = instance("channel0");
        incomplete.input_bindings.pop();
        assert!(
            compose_btor2_channel_family(
                CORE,
                &[3],
                CHANNEL,
                &[9],
                &[incomplete],
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_binding_indices_widths_and_resource_exhaustion() {
        let mut outside = instance("channel0");
        outside.input_bindings[0] = FamilyInputBinding::CoreRoot(1);
        assert!(
            compose_btor2_channel_family(
                CORE,
                &[3],
                CHANNEL,
                &[9],
                &[outside],
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );

        let policy = Btor2FamilyPolicy::new(1024, 1, 1, 2, 1024).unwrap();
        assert!(
            compose_btor2_channel_family(
                CORE,
                &[3],
                CHANNEL,
                &[9],
                &[instance("channel0")],
                policy,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_non_boolean_bad_root() {
        let channel = br#"1 sort bitvec 1
2 sort bitvec 2
3 input 1 phase
4 input 1 enable
5 state 2 pulse
6 zero 2
7 init 2 5 6
8 next 2 5 5
9 output 5 wide
"#;
        assert!(
            compose_btor2_channel_family(
                CORE,
                &[3],
                channel,
                &[5],
                &[instance("channel0")],
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn composes_retained_opentitan_pwm_models_at_predeclared_sizes() {
        for count in [2, 4, 6] {
            let instances = (0..count)
                .map(|index| Btor2FamilyInstance {
                    identifier: format!("channel{index}"),
                    parameter_sha256: [0x5a; 32],
                    input_bindings: vec![
                        FamilyInputBinding::CoreRoot(0),
                        FamilyInputBinding::CoreRoot(1),
                        FamilyInputBinding::CoreRoot(2),
                        FamilyInputBinding::CoreRoot(3),
                    ],
                })
                .collect::<Vec<_>>();
            let composition = compose_btor2_channel_family(
                OPENTITAN_CORE,
                &[1000, 1001, 1002, 1003],
                OPENTITAN_CHANNEL,
                &[1000, 1001, 1002, 1003, 1004],
                &instances,
                Btor2FamilyPolicy::default(),
            )
            .unwrap();

            assert_eq!(composition.instances, count);
            assert_eq!(composition.expanded_states, 2 + 2 * count);
            assert_eq!(composition.expanded_inputs, 1);
            assert_eq!(composition.expanded_bad_properties, 5 * count);
            assert_eq!(
                btor2::parse_bytes(&composition.expanded_model)
                    .unwrap()
                    .states()
                    .len(),
                2 + 2 * count
            );
        }
    }

    #[test]
    fn rejects_components_with_embedded_properties() {
        let core = br#"1 sort bitvec 1
2 input 1 enable
3 state 1 phase
4 zero 1
5 init 1 3 4
6 next 1 3 3
7 bad 3 embedded
"#;
        assert!(
            compose_btor2_channel_family(
                core,
                &[3],
                CHANNEL,
                &[9],
                &[instance("channel0")],
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );
    }

    fn artifact_fixture() -> (Btor2FamilyArtifact, Btor2FamilyComposition) {
        produce_btor2_family_artifact(
            CORE,
            &[3],
            CHANNEL,
            &[9],
            PARAMETERS,
            &[instance("channel0"), instance("channel1")],
            Btor2FamilyPolicy::default(),
        )
        .unwrap()
    }

    #[test]
    fn artifact_round_trip_is_canonical_and_independently_reconstructed() {
        let (artifact, produced) = artifact_fixture();
        let bytes = encode_btor2_family_artifact(&artifact, Btor2FamilyPolicy::default()).unwrap();
        let decoded = decode_btor2_family_artifact(&bytes, Btor2FamilyPolicy::default()).unwrap();
        let verified = verify_btor2_family_artifact(
            CORE,
            CHANNEL,
            PARAMETERS,
            &decoded,
            Btor2FamilyPolicy::default(),
        )
        .unwrap();

        assert_eq!(artifact, decoded);
        assert_eq!(produced, verified);
        assert_eq!(
            bytes,
            encode_btor2_family_artifact(&decoded, Btor2FamilyPolicy::default()).unwrap()
        );
        assert!(bytes.len() < produced.expanded_model.len());
    }

    #[test]
    fn every_artifact_byte_mutation_and_truncation_fails_closed() {
        let (artifact, _) = artifact_fixture();
        let bytes = encode_btor2_family_artifact(&artifact, Btor2FamilyPolicy::default()).unwrap();
        for end in 0..bytes.len() {
            assert!(
                decode_btor2_family_artifact(&bytes[..end], Btor2FamilyPolicy::default()).is_err(),
                "accepted truncation at {end}"
            );
        }
        for offset in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[offset] ^= 1;
            assert!(
                decode_btor2_family_artifact(&changed, Btor2FamilyPolicy::default()).is_err(),
                "accepted mutation at {offset}"
            );
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode_btor2_family_artifact(&trailing, Btor2FamilyPolicy::default()).is_err());
    }

    #[test]
    fn verifier_rejects_source_and_wiring_drift_with_fresh_checksums() {
        let (artifact, _) = artifact_fixture();
        assert!(
            verify_btor2_family_artifact(
                b"changed core",
                CHANNEL,
                PARAMETERS,
                &artifact,
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );
        assert!(
            verify_btor2_family_artifact(
                CORE,
                CHANNEL,
                b"phase_width=2\n",
                &artifact,
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );

        let mut rewired = artifact;
        rewired.instances[0].input_bindings.swap(0, 1);
        let encoded = encode_btor2_family_artifact(&rewired, Btor2FamilyPolicy::default()).unwrap();
        let decoded = decode_btor2_family_artifact(&encoded, Btor2FamilyPolicy::default()).unwrap();
        assert!(
            verify_btor2_family_artifact(
                CORE,
                CHANNEL,
                PARAMETERS,
                &decoded,
                Btor2FamilyPolicy::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn artifact_decoder_applies_caller_byte_and_instance_limits() {
        let (artifact, _) = artifact_fixture();
        let bytes = encode_btor2_family_artifact(&artifact, Btor2FamilyPolicy::default()).unwrap();
        let byte_limited = Btor2FamilyPolicy::new(bytes.len() - 1, 2, 1, 32, 4096).unwrap();
        assert!(decode_btor2_family_artifact(&bytes, byte_limited).is_err());

        let instance_limited = Btor2FamilyPolicy::new(bytes.len(), 1, 1, 32, 4096).unwrap();
        assert!(decode_btor2_family_artifact(&bytes, instance_limited).is_err());

        assert!(
            produce_btor2_family_artifact(
                CORE,
                &[3],
                CHANNEL,
                &[9],
                PARAMETERS,
                &[instance("channel0"), instance("channel1")],
                byte_limited,
            )
            .is_err()
        );
    }
}
