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
pub const MAX_FAMILY_INSTANCES: usize = 64;
pub const MAX_FAMILY_ROOTS: usize = 256;

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
    max_instances: usize,
    max_roots_per_component: usize,
    max_expanded_nodes: usize,
    max_expanded_bytes: usize,
}

impl Btor2FamilyPolicy {
    pub fn new(
        max_instances: usize,
        max_roots_per_component: usize,
        max_expanded_nodes: usize,
        max_expanded_bytes: usize,
    ) -> Result<Self, Btor2FamilyError> {
        if max_instances == 0
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
            max_instances,
            max_roots_per_component,
            max_expanded_nodes,
            max_expanded_bytes,
        })
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

    const OPENTITAN_CORE: &[u8] =
        include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-after.btor2");
    const OPENTITAN_CHANNEL: &[u8] = include_bytes!(
        "../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-after.btor2"
    );

    fn instance(identifier: &str) -> Btor2FamilyInstance {
        Btor2FamilyInstance {
            identifier: identifier.to_string(),
            parameter_sha256: [7; 32],
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

        let policy = Btor2FamilyPolicy::new(1, 1, 2, 1024).unwrap();
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
}
