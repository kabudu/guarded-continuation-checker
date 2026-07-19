//! Source-bound exact controller functions represented as reduced MTBDDs.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::aiger_obligation::{AigerOutcome, AigerTransition};

pub const CONTROLLER_MTBDD_VERSION: u32 = 1;
pub const MAX_MTBDD_STATE_BITS: usize = 6;
pub const MAX_MTBDD_INPUTS: usize = 12;
pub const MAX_MTBDD_OUTPUTS: usize = 8;
pub const MAX_MTBDD_NODES: usize = 512;
pub const MAX_MTBDD_TERMINALS: usize = 1_024;
pub const MAX_MTBDD_ASSIGNMENTS: usize = 131_072;
pub const MAX_MTBDD_ARTIFACT_BYTES: usize = 1024 * 1024;
const MAGIC: &[u8; 8] = b"GCCMTB01";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ControllerMtbddNode {
    pub variable: usize,
    pub low: usize,
    pub high: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddArtifact {
    pub version: u32,
    pub source_sha256: [u8; 32],
    pub relevant_inputs: Vec<usize>,
    pub observed_outputs: Vec<usize>,
    pub state_count: usize,
    pub root: usize,
    pub terminals: Vec<AigerOutcome>,
    pub nodes: Vec<ControllerMtbddNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddSummary {
    pub state_bits: usize,
    pub inputs: usize,
    pub outputs: usize,
    pub terminals: usize,
    pub nodes: usize,
    pub assignments_checked: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddError(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerMtbddAdmissionFailure {
    BoundaryLimit,
    TerminalLimit,
    NodeLimit,
}

impl ControllerMtbddError {
    /// Classify only static producer limits that an exact portfolio may route
    /// around. Malformed models and semantic failures are never admission
    /// failures and must remain errors.
    pub fn admission_failure(&self) -> Option<ControllerMtbddAdmissionFailure> {
        match self.0.as_str() {
            "controller MTBDD boundary exceeds limits" => {
                Some(ControllerMtbddAdmissionFailure::BoundaryLimit)
            }
            "controller MTBDD terminal count exceeds limit" => {
                Some(ControllerMtbddAdmissionFailure::TerminalLimit)
            }
            "controller MTBDD node count exceeds limit" => {
                Some(ControllerMtbddAdmissionFailure::NodeLimit)
            }
            _ => None,
        }
    }
}

impl fmt::Display for ControllerMtbddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ControllerMtbddError {}

fn reject(message: impl Into<String>) -> ControllerMtbddError {
    ControllerMtbddError(message.into())
}

fn state_bits(state_count: usize) -> Result<usize, ControllerMtbddError> {
    if state_count == 0 || !state_count.is_power_of_two() {
        return Err(reject("controller MTBDD state count is invalid"));
    }
    Ok(state_count.trailing_zeros() as usize)
}

fn validate_boundary(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
) -> Result<(usize, usize), ControllerMtbddError> {
    model
        .validate()
        .map_err(|error| reject(error.to_string()))?;
    let bits = state_bits(model.state_count())?;
    let assignments = model
        .state_count()
        .checked_mul(
            1usize
                .checked_shl(relevant_inputs.len() as u32)
                .ok_or_else(|| reject("controller MTBDD input space overflow"))?,
        )
        .ok_or_else(|| reject("controller MTBDD assignment count overflow"))?;
    if bits > MAX_MTBDD_STATE_BITS
        || relevant_inputs.len() > MAX_MTBDD_INPUTS
        || observed_outputs.len() > MAX_MTBDD_OUTPUTS
        || assignments > MAX_MTBDD_ASSIGNMENTS
    {
        return Err(reject("controller MTBDD boundary exceeds limits"));
    }
    if relevant_inputs
        .iter()
        .any(|&input| input >= model.inputs.len())
        || relevant_inputs.windows(2).any(|pair| pair[0] >= pair[1])
        || observed_outputs
            .iter()
            .any(|&output| output >= model.outputs.len())
        || observed_outputs.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("controller MTBDD boundary is invalid"));
    }
    Ok((bits, assignments))
}

fn declared_input(relevant_inputs: &[usize], pattern: usize) -> u64 {
    relevant_inputs
        .iter()
        .enumerate()
        .fold(0, |value, (bit, &input)| {
            value | (u64::from(pattern >> bit & 1 == 1) << input)
        })
}

fn projected_outputs(outputs: u128, observed_outputs: &[usize]) -> u128 {
    observed_outputs
        .iter()
        .enumerate()
        .fold(0, |value, (bit, &output)| {
            value | (((outputs >> output) & 1) << bit)
        })
}

fn assignment_outcome(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
    state_bits: usize,
    assignment: usize,
) -> Result<AigerOutcome, ControllerMtbddError> {
    let input_bits = relevant_inputs.len();
    let total = state_bits + input_bits;
    let mut state = 0usize;
    let mut pattern = 0usize;
    for position in 0..total {
        let value = assignment >> (total - position - 1) & 1;
        if position < state_bits {
            state |= value << position;
        } else {
            pattern |= value << (position - state_bits);
        }
    }
    let (target, outputs) = model
        .evaluate(state, declared_input(relevant_inputs, pattern))
        .map_err(|error| reject(error.to_string()))?;
    Ok(AigerOutcome {
        target,
        outputs: projected_outputs(outputs, observed_outputs),
    })
}

pub fn produce_controller_mtbdd(
    model: &AigerTransition,
    source_sha256: [u8; 32],
    relevant_inputs: &[usize],
    observed_outputs: &[usize],
) -> Result<ControllerMtbddArtifact, ControllerMtbddError> {
    let (bits, assignments) = validate_boundary(model, relevant_inputs, observed_outputs)?;
    let outcomes = (0..assignments)
        .map(|assignment| {
            assignment_outcome(model, relevant_inputs, observed_outputs, bits, assignment)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let terminals = outcomes.iter().copied().collect::<BTreeSet<_>>();
    if terminals.is_empty() || terminals.len() > MAX_MTBDD_TERMINALS {
        return Err(reject("controller MTBDD terminal count exceeds limit"));
    }
    let terminals = terminals.into_iter().collect::<Vec<_>>();
    let terminal_ids = terminals
        .iter()
        .copied()
        .enumerate()
        .map(|(index, outcome)| (outcome, index))
        .collect::<BTreeMap<_, _>>();
    let mut layer = outcomes
        .iter()
        .map(|outcome| terminal_ids[outcome])
        .collect::<Vec<_>>();
    let mut nodes = Vec::new();
    let mut unique = BTreeMap::new();
    for variable in (0..bits + relevant_inputs.len()).rev() {
        let mut parent = Vec::with_capacity(layer.len() / 2);
        for pair in layer.chunks_exact(2) {
            let reference = if pair[0] == pair[1] {
                pair[0]
            } else if let Some(&reference) = unique.get(&(variable, pair[0], pair[1])) {
                reference
            } else {
                if nodes.len() >= MAX_MTBDD_NODES {
                    return Err(reject("controller MTBDD node count exceeds limit"));
                }
                let reference = terminals.len() + nodes.len();
                nodes.push(ControllerMtbddNode {
                    variable,
                    low: pair[0],
                    high: pair[1],
                });
                unique.insert((variable, pair[0], pair[1]), reference);
                reference
            };
            parent.push(reference);
        }
        layer = parent;
    }
    let root = *layer
        .first()
        .ok_or_else(|| reject("controller MTBDD root is missing"))?;
    let artifact = ControllerMtbddArtifact {
        version: CONTROLLER_MTBDD_VERSION,
        source_sha256,
        relevant_inputs: relevant_inputs.to_vec(),
        observed_outputs: observed_outputs.to_vec(),
        state_count: model.state_count(),
        root,
        terminals,
        nodes,
    };
    validate_artifact(&artifact)?;
    Ok(artifact)
}

fn validate_artifact(artifact: &ControllerMtbddArtifact) -> Result<(), ControllerMtbddError> {
    let bits = state_bits(artifact.state_count)?;
    let total_variables = bits
        .checked_add(artifact.relevant_inputs.len())
        .ok_or_else(|| reject("controller MTBDD variable count overflow"))?;
    if artifact.version != CONTROLLER_MTBDD_VERSION
        || bits > MAX_MTBDD_STATE_BITS
        || artifact.relevant_inputs.len() > MAX_MTBDD_INPUTS
        || artifact.observed_outputs.len() > MAX_MTBDD_OUTPUTS
        || artifact.terminals.is_empty()
        || artifact.terminals.len() > MAX_MTBDD_TERMINALS
        || artifact.nodes.len() > MAX_MTBDD_NODES
        || artifact.terminals.windows(2).any(|pair| pair[0] >= pair[1])
        || artifact
            .relevant_inputs
            .iter()
            .chain(&artifact.observed_outputs)
            .any(|&value| value > u8::MAX as usize)
        || artifact
            .relevant_inputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || artifact
            .observed_outputs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("controller MTBDD artifact shape is invalid"));
    }
    let reference_limit = artifact
        .terminals
        .len()
        .checked_add(artifact.nodes.len())
        .ok_or_else(|| reject("controller MTBDD reference count overflow"))?;
    if artifact.root >= reference_limit
        || artifact.terminals.iter().any(|terminal| {
            terminal.target >= artifact.state_count
                || (artifact.observed_outputs.len() < u128::BITS as usize
                    && terminal.outputs >> artifact.observed_outputs.len() != 0)
        })
    {
        return Err(reject("controller MTBDD terminal or root is invalid"));
    }
    let mut unique = BTreeSet::new();
    for node in &artifact.nodes {
        if node.variable >= total_variables
            || node.low >= reference_limit
            || node.high >= reference_limit
            || node.low == node.high
            || !unique.insert(*node)
        {
            return Err(reject("controller MTBDD node is invalid"));
        }
        for child in [node.low, node.high] {
            if child >= artifact.terminals.len()
                && artifact.nodes[child - artifact.terminals.len()].variable <= node.variable
            {
                return Err(reject("controller MTBDD variable order is invalid"));
            }
        }
    }
    let mut pending = vec![artifact.root];
    let mut reached_nodes = BTreeSet::new();
    while let Some(reference) = pending.pop() {
        if reference >= artifact.terminals.len() {
            let index = reference - artifact.terminals.len();
            if reached_nodes.insert(index) {
                pending.push(artifact.nodes[index].low);
                pending.push(artifact.nodes[index].high);
            }
        }
    }
    if reached_nodes.len() != artifact.nodes.len() {
        return Err(reject("controller MTBDD contains unreachable nodes"));
    }
    Ok(())
}

/// Validate only the canonical MTBDD structure and return its state-bit count.
/// Semantic equivalence to a controller model requires either exhaustive
/// verification or a separately checked equivalence proof.
pub fn validate_controller_mtbdd_structure(
    artifact: &ControllerMtbddArtifact,
) -> Result<usize, ControllerMtbddError> {
    validate_artifact(artifact)?;
    state_bits(artifact.state_count)
}

pub fn evaluate_controller_mtbdd(
    artifact: &ControllerMtbddArtifact,
    state: usize,
    input_pattern: usize,
) -> Result<AigerOutcome, ControllerMtbddError> {
    validate_artifact(artifact)?;
    let bits = state_bits(artifact.state_count)?;
    if state >= artifact.state_count
        || (artifact.relevant_inputs.len() < usize::BITS as usize
            && input_pattern >> artifact.relevant_inputs.len() != 0)
    {
        return Err(reject("controller MTBDD query is outside dimensions"));
    }
    evaluate_controller_mtbdd_unchecked(artifact, bits, state, input_pattern)
}

pub(crate) fn evaluate_controller_mtbdd_unchecked(
    artifact: &ControllerMtbddArtifact,
    bits: usize,
    state: usize,
    input_pattern: usize,
) -> Result<AigerOutcome, ControllerMtbddError> {
    let mut reference = artifact.root;
    for _ in 0..=bits + artifact.relevant_inputs.len() {
        if reference < artifact.terminals.len() {
            return Ok(artifact.terminals[reference]);
        }
        let node = artifact.nodes[reference - artifact.terminals.len()];
        let value = if node.variable < bits {
            state >> node.variable & 1
        } else {
            input_pattern >> (node.variable - bits) & 1
        };
        reference = if value == 0 { node.low } else { node.high };
    }
    Err(reject("controller MTBDD evaluation did not terminate"))
}

pub fn verify_controller_mtbdd(
    model: &AigerTransition,
    expected_source_sha256: [u8; 32],
    artifact: &ControllerMtbddArtifact,
) -> Result<ControllerMtbddSummary, ControllerMtbddError> {
    validate_artifact(artifact)?;
    let (bits, assignments) =
        validate_boundary(model, &artifact.relevant_inputs, &artifact.observed_outputs)?;
    if artifact.source_sha256 != expected_source_sha256
        || artifact.state_count != model.state_count()
    {
        return Err(reject("controller MTBDD source binding mismatch"));
    }
    for state in 0..artifact.state_count {
        for pattern in 0..(1usize << artifact.relevant_inputs.len()) {
            let (target, outputs) = model
                .evaluate(state, declared_input(&artifact.relevant_inputs, pattern))
                .map_err(|error| reject(error.to_string()))?;
            let expected = AigerOutcome {
                target,
                outputs: projected_outputs(outputs, &artifact.observed_outputs),
            };
            if evaluate_controller_mtbdd_unchecked(artifact, bits, state, pattern)? != expected {
                return Err(reject("controller MTBDD outcome mismatch"));
            }
        }
    }
    Ok(ControllerMtbddSummary {
        state_bits: bits,
        inputs: artifact.relevant_inputs.len(),
        outputs: artifact.observed_outputs.len(),
        terminals: artifact.terminals.len(),
        nodes: artifact.nodes.len(),
        assignments_checked: assignments,
    })
}

fn put_u32(bytes: &mut Vec<u8>, value: usize) -> Result<(), ControllerMtbddError> {
    bytes.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| reject("controller MTBDD integer exceeds range"))?
            .to_le_bytes(),
    );
    Ok(())
}

pub fn encode_controller_mtbdd(
    artifact: &ControllerMtbddArtifact,
) -> Result<Vec<u8>, ControllerMtbddError> {
    validate_artifact(artifact)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&artifact.version.to_le_bytes());
    bytes.extend_from_slice(&artifact.source_sha256);
    bytes.push(artifact.relevant_inputs.len() as u8);
    bytes.extend(artifact.relevant_inputs.iter().map(|&value| value as u8));
    bytes.push(artifact.observed_outputs.len() as u8);
    bytes.extend(artifact.observed_outputs.iter().map(|&value| value as u8));
    put_u32(&mut bytes, artifact.state_count)?;
    put_u32(&mut bytes, artifact.root)?;
    put_u32(&mut bytes, artifact.terminals.len())?;
    put_u32(&mut bytes, artifact.nodes.len())?;
    for terminal in &artifact.terminals {
        put_u32(&mut bytes, terminal.target)?;
        bytes.extend_from_slice(&terminal.outputs.to_le_bytes());
    }
    for node in &artifact.nodes {
        put_u32(&mut bytes, node.variable)?;
        put_u32(&mut bytes, node.low)?;
        put_u32(&mut bytes, node.high)?;
    }
    let integrity = Sha256::digest(&bytes);
    bytes.extend_from_slice(&integrity);
    if bytes.len() > MAX_MTBDD_ARTIFACT_BYTES {
        return Err(reject("controller MTBDD artifact exceeds byte limit"));
    }
    Ok(bytes)
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    count: usize,
) -> Result<&'a [u8], ControllerMtbddError> {
    let end = cursor
        .checked_add(count)
        .ok_or_else(|| reject("controller MTBDD cursor overflow"))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| reject("controller MTBDD artifact is truncated"))?;
    *cursor = end;
    Ok(value)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<usize, ControllerMtbddError> {
    Ok(u32::from_le_bytes(
        take(bytes, cursor, 4)?
            .try_into()
            .map_err(|_| reject("controller MTBDD u32 decode failed"))?,
    ) as usize)
}

pub fn decode_controller_mtbdd(
    bytes: &[u8],
) -> Result<ControllerMtbddArtifact, ControllerMtbddError> {
    if bytes.len() < 32 || bytes.len() > MAX_MTBDD_ARTIFACT_BYTES {
        return Err(reject("controller MTBDD artifact size is invalid"));
    }
    let payload_len = bytes.len() - 32;
    let (payload, integrity) = bytes.split_at(payload_len);
    if Sha256::digest(payload).as_slice() != integrity {
        return Err(reject("controller MTBDD artifact integrity mismatch"));
    }
    let mut cursor = 0usize;
    if take(payload, &mut cursor, MAGIC.len())? != MAGIC {
        return Err(reject("controller MTBDD artifact magic mismatch"));
    }
    let version = read_u32(payload, &mut cursor)? as u32;
    if version != CONTROLLER_MTBDD_VERSION {
        return Err(reject("controller MTBDD artifact version mismatch"));
    }
    let source_sha256 = take(payload, &mut cursor, 32)?
        .try_into()
        .map_err(|_| reject("controller MTBDD source digest decode failed"))?;
    let input_count = *take(payload, &mut cursor, 1)?
        .first()
        .ok_or_else(|| reject("controller MTBDD input count is missing"))?
        as usize;
    if input_count > MAX_MTBDD_INPUTS {
        return Err(reject("controller MTBDD input count exceeds limit"));
    }
    let relevant_inputs = take(payload, &mut cursor, input_count)?
        .iter()
        .map(|&value| value as usize)
        .collect();
    let output_count = *take(payload, &mut cursor, 1)?
        .first()
        .ok_or_else(|| reject("controller MTBDD output count is missing"))?
        as usize;
    if output_count > MAX_MTBDD_OUTPUTS {
        return Err(reject("controller MTBDD output count exceeds limit"));
    }
    let observed_outputs = take(payload, &mut cursor, output_count)?
        .iter()
        .map(|&value| value as usize)
        .collect();
    let state_count = read_u32(payload, &mut cursor)?;
    let root = read_u32(payload, &mut cursor)?;
    let terminal_count = read_u32(payload, &mut cursor)?;
    let node_count = read_u32(payload, &mut cursor)?;
    if terminal_count == 0 || terminal_count > MAX_MTBDD_TERMINALS || node_count > MAX_MTBDD_NODES {
        return Err(reject("controller MTBDD decoded dimensions exceed limits"));
    }
    let mut terminals = Vec::with_capacity(terminal_count);
    for _ in 0..terminal_count {
        let target = read_u32(payload, &mut cursor)?;
        let outputs = u128::from_le_bytes(
            take(payload, &mut cursor, 16)?
                .try_into()
                .map_err(|_| reject("controller MTBDD terminal decode failed"))?,
        );
        terminals.push(AigerOutcome { target, outputs });
    }
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        nodes.push(ControllerMtbddNode {
            variable: read_u32(payload, &mut cursor)?,
            low: read_u32(payload, &mut cursor)?,
            high: read_u32(payload, &mut cursor)?,
        });
    }
    if cursor != payload.len() {
        return Err(reject("controller MTBDD artifact has trailing bytes"));
    }
    let artifact = ControllerMtbddArtifact {
        version,
        source_sha256,
        relevant_inputs,
        observed_outputs,
        state_count,
        root,
        terminals,
        nodes,
    };
    validate_artifact(&artifact)?;
    Ok(artifact)
}
