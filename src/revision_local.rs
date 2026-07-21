//! Canonical envelope primitives for revision-local component evidence.

use crate::btor2::{self, Btor2Model, NodeId, WordValues};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const REVISION_LOCAL_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_LOCAL_SECTION_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_INTERFACE_SECTION_BYTES: usize = 1024 * 1024;
pub const MAX_FINAL_SECTION_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_REVISION_LOCAL_CERTIFICATE_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_LOCAL_STATE_BITS: usize = 8;
pub const MAX_LOCAL_INPUT_BITS: usize = 8;
pub const MAX_LOCAL_OUTPUT_BITS: usize = 8;
pub const MAX_LOCAL_VALUATIONS: usize = 65_536;
pub const MAX_LOCAL_NODE_STEPS: usize = 30_000_000;
pub const LOCAL_RELATION_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_LOCAL_CONSTRAINTS: usize = 4096;
pub const MAX_INTERFACE_WIRES: usize = 8;
pub const MAX_COMPOSED_PAIR_CHECKS: usize = 4_000_000;
pub const MAX_COMPOSED_PAIRS: usize = 65_536;
pub const WORD_INTERFACE_CONTRACT_VERSION: u32 = 1;
pub const MAX_WORD_INTERFACE_CONTRACT_BYTES: usize = 4096;
pub const MAX_FINAL_HORIZON: u32 = 32;
pub const MAX_FINAL_STATES_PER_LAYER: usize = 65_536;
pub const MAX_FINAL_TOTAL_STATES: usize = 262_144;
pub const MAX_FINAL_TRANSITION_CHECKS: usize = 4_000_000;
pub const MAX_DIRECT_INPUT_BITS: usize = 12;
pub const MAX_DIRECT_INPUT_VALUATIONS: usize = 4096;
pub const MAX_DIRECT_STATE_NODES: usize = 64;

const MAGIC: &[u8; 8] = b"GCCRLCP1";
const LOCAL_RELATION_MAGIC: &[u8; 8] = b"GCCLRL01";
const BOUNDED_ANSWER_MAGIC: &[u8; 8] = b"GCCBA001";
const DIRECT_ANSWER_MAGIC: &[u8; 8] = b"GCCDA001";
pub const BOUNDED_ANSWER_CERTIFICATE_VERSION: u32 = 1;
pub const DIRECT_ANSWER_CERTIFICATE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceSection {
    Left,
    Right,
    Interface,
    Final,
    Envelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEvidence {
    pub source_sha256: [u8; 32],
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundEvidence {
    pub source_sha256: [u8; 32],
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionLocalCertificate {
    pub left: LocalEvidence,
    pub right: LocalEvidence,
    pub interface: BoundEvidence,
    pub final_evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalRelationRow {
    pub state: Vec<u64>,
    pub input: Vec<u64>,
    pub next_state: Vec<u64>,
    pub output: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalRelationCertificate {
    pub source_sha256: [u8; 32],
    pub states: Vec<NodeId>,
    pub state_widths: Vec<u32>,
    pub inputs: Vec<NodeId>,
    pub input_widths: Vec<u32>,
    pub outputs: Vec<NodeId>,
    pub output_widths: Vec<u32>,
    pub constraints: Vec<NodeId>,
    pub rows: Vec<LocalRelationRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalRelationSummary {
    pub state_bits: usize,
    pub input_bits: usize,
    pub output_bits: usize,
    pub candidate_valuations: usize,
    pub admissible_rows: usize,
    pub initial_state: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ComponentSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InterfaceWire {
    pub from: ComponentSide,
    pub output: NodeId,
    pub to_input: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WordInterfaceContract {
    pub wires: Vec<InterfaceWire>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComposedPair {
    pub left_row: u32,
    pub right_row: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposedRelation {
    pub interface_sha256: [u8; 32],
    pub pairs: Vec<ComposedPair>,
    pub pair_checks: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CombinedState {
    pub left: Vec<u64>,
    pub right: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedResult {
    Safe,
    Unsafe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedQuery {
    pub horizon: u32,
    pub bad_side: ComponentSide,
    pub bad_output: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedAnswerCertificate {
    pub left_sha256: [u8; 32],
    pub right_sha256: [u8; 32],
    pub interface_sha256: [u8; 32],
    pub query: BoundedQuery,
    pub result: BoundedResult,
    pub bad_frame: Option<u32>,
    pub witness_pairs: Vec<u32>,
    pub layers: Vec<Vec<CombinedState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedAnswerSummary {
    pub result: BoundedResult,
    pub horizon: u32,
    pub bad_frame: Option<u32>,
    pub reachable_states: usize,
    pub transition_checks: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionLocalSummary {
    pub left: LocalRelationSummary,
    pub right: LocalRelationSummary,
    pub answer: BoundedAnswerSummary,
    pub certificate_bytes: usize,
}

pub struct ValidatedLocalArtifact {
    certificate: LocalRelationCertificate,
    summary: LocalRelationSummary,
    encoded: Vec<u8>,
}

impl ValidatedLocalArtifact {
    pub fn summary(&self) -> &LocalRelationSummary {
        &self.summary
    }

    pub fn source_sha256(&self) -> &[u8; 32] {
        &self.certificate.source_sha256
    }

    pub fn encoded(&self) -> &[u8] {
        &self.encoded
    }

    fn verified(&self) -> VerifiedLocalRelation<'_> {
        VerifiedLocalRelation {
            certificate: &self.certificate,
            summary: self.summary.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionWorkObservation {
    pub decoded_local_sections: usize,
    pub semantically_verified_local_sections: usize,
    pub reused_local_sections: usize,
    pub composed_pair_checks: usize,
    pub final_transition_checks: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectAnswerCertificate {
    pub left_sha256: [u8; 32],
    pub right_sha256: [u8; 32],
    pub interface_sha256: [u8; 32],
    pub query: BoundedQuery,
    pub result: BoundedResult,
    pub bad_frame: Option<u32>,
    pub witness_valuations: Vec<u16>,
    pub layers: Vec<Vec<CombinedState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectAnswerSummary {
    pub result: BoundedResult,
    pub horizon: u32,
    pub bad_frame: Option<u32>,
    pub reachable_states: usize,
    pub valuation_checks: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionPortfolioBackend {
    RevisionLocal,
    DirectExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionSelectionReason {
    ExactLocalRelationAdmitted,
    LeftStateWidth,
    LeftInputWidth,
    LeftOutputWidth,
    LeftNodeSteps,
    RightStateWidth,
    RightInputWidth,
    RightOutputWidth,
    RightNodeSteps,
    PairCheckBound,
}

impl RevisionSelectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactLocalRelationAdmitted => "exact-local-relation-admitted",
            Self::LeftStateWidth => "left-state-width",
            Self::LeftInputWidth => "left-input-width",
            Self::LeftOutputWidth => "left-output-width",
            Self::LeftNodeSteps => "left-node-steps",
            Self::RightStateWidth => "right-state-width",
            Self::RightInputWidth => "right-input-width",
            Self::RightOutputWidth => "right-output-width",
            Self::RightNodeSteps => "right-node-steps",
            Self::PairCheckBound => "pair-check-bound",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionPortfolioCertificate {
    RevisionLocal(RevisionLocalCertificate),
    DirectExact(DirectAnswerCertificate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionPortfolioProduction {
    pub certificate: RevisionPortfolioCertificate,
    pub backend: RevisionPortfolioBackend,
    pub reason: RevisionSelectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionPortfolioSummary {
    pub result: BoundedResult,
    pub bad_frame: Option<u32>,
    pub backend: RevisionPortfolioBackend,
    pub reason: RevisionSelectionReason,
    pub certificate_bytes: usize,
}

pub struct VerifiedLocalRelation<'a> {
    certificate: &'a LocalRelationCertificate,
    summary: LocalRelationSummary,
}

impl VerifiedLocalRelation<'_> {
    pub fn summary(&self) -> &LocalRelationSummary {
        &self.summary
    }

    pub fn source_sha256(&self) -> &[u8; 32] {
        &self.certificate.source_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionLocalError {
    pub section: EvidenceSection,
    pub message: String,
}

impl fmt::Display for RevisionLocalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} evidence: {}", self.section, self.message)
    }
}

impl Error for RevisionLocalError {}

fn reject(section: EvidenceSection, message: impl Into<String>) -> RevisionLocalError {
    RevisionLocalError {
        section,
        message: message.into(),
    }
}

pub fn source_digest(source: &[u8]) -> [u8; 32] {
    Sha256::digest(source).into()
}

pub fn evidence_digest(evidence: &[u8]) -> [u8; 32] {
    Sha256::digest(evidence).into()
}

struct RelationShape {
    states: Vec<NodeId>,
    state_widths: Vec<u32>,
    inputs: Vec<NodeId>,
    input_widths: Vec<u32>,
    outputs: Vec<NodeId>,
    output_widths: Vec<u32>,
    constraints: Vec<NodeId>,
    state_bits: usize,
    input_bits: usize,
    output_bits: usize,
    candidate_valuations: usize,
}

fn widths(
    model: &Btor2Model,
    ids: &[NodeId],
    section: EvidenceSection,
    label: &str,
) -> Result<Vec<u32>, RevisionLocalError> {
    ids.iter()
        .map(|id| {
            model
                .nodes()
                .get(id)
                .map(|node| node.width)
                .ok_or_else(|| reject(section, format!("unknown {label} node {id}")))
        })
        .collect()
}

fn checked_bits(
    widths: &[u32],
    limit: usize,
    section: EvidenceSection,
    label: &str,
) -> Result<usize, RevisionLocalError> {
    let bits = widths
        .iter()
        .try_fold(0usize, |total, width| total.checked_add(*width as usize));
    let bits = bits.ok_or_else(|| reject(section, format!("{label} width overflowed")))?;
    if bits > limit {
        return Err(reject(
            section,
            format!("{label} width exceeds {limit}-bit limit"),
        ));
    }
    Ok(bits)
}

fn relation_shape(
    model: &Btor2Model,
    outputs: &[NodeId],
    section: EvidenceSection,
) -> Result<RelationShape, RevisionLocalError> {
    if outputs.is_empty() {
        return Err(reject(section, "at least one output is required"));
    }
    let states = model.states().to_vec();
    let inputs = model.inputs().to_vec();
    if states.is_empty() || inputs.is_empty() {
        return Err(reject(
            section,
            "local relation requires state and semantic input nodes",
        ));
    }
    if outputs.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(reject(
            section,
            "output nodes must be unique and strictly increasing",
        ));
    }
    let state_widths = widths(model, &states, section, "state")?;
    let input_widths = widths(model, &inputs, section, "input")?;
    let output_widths = widths(model, outputs, section, "output")?;
    let state_bits = checked_bits(&state_widths, MAX_LOCAL_STATE_BITS, section, "state")?;
    let input_bits = checked_bits(&input_widths, MAX_LOCAL_INPUT_BITS, section, "input")?;
    let output_bits = checked_bits(&output_widths, MAX_LOCAL_OUTPUT_BITS, section, "output")?;
    let total_bits = state_bits
        .checked_add(input_bits)
        .ok_or_else(|| reject(section, "local valuation width overflowed"))?;
    let candidate_valuations = 1usize
        .checked_shl(total_bits as u32)
        .filter(|count| *count <= MAX_LOCAL_VALUATIONS)
        .ok_or_else(|| reject(section, "local valuation count exceeds limit"))?;
    let node_steps = candidate_valuations
        .checked_mul(model.nodes().len())
        .ok_or_else(|| reject(section, "local node-step estimate overflowed"))?;
    if node_steps > MAX_LOCAL_NODE_STEPS {
        return Err(reject(section, "local node-step estimate exceeds limit"));
    }
    Ok(RelationShape {
        states,
        state_widths,
        inputs,
        input_widths,
        outputs: outputs.to_vec(),
        output_widths,
        constraints: model.constraints().iter().map(|(id, _)| *id).collect(),
        state_bits,
        input_bits,
        output_bits,
        candidate_valuations,
    })
}

fn unpack(ids: &[NodeId], widths: &[u32], mut packed: usize) -> (WordValues, Vec<u64>) {
    let mut map = BTreeMap::new();
    let mut values = Vec::with_capacity(ids.len());
    for (id, width) in ids.iter().zip(widths) {
        let mask = (1usize << *width) - 1;
        let value = (packed & mask) as u64;
        packed >>= *width;
        map.insert(*id, value);
        values.push(value);
    }
    (map, values)
}

fn admissible(
    model: &Btor2Model,
    state: &WordValues,
    input: &WordValues,
    section: EvidenceSection,
) -> Result<bool, RevisionLocalError> {
    for (_, expression) in model.constraints() {
        if model
            .evaluate(*expression, state, input)
            .map_err(|error| reject(section, error.to_string()))?
            == 0
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn relation_row(
    model: &Btor2Model,
    shape: &RelationShape,
    state_packed: usize,
    input_packed: usize,
    section: EvidenceSection,
) -> Result<Option<LocalRelationRow>, RevisionLocalError> {
    let (state, state_values) = unpack(&shape.states, &shape.state_widths, state_packed);
    let (input, input_values) = unpack(&shape.inputs, &shape.input_widths, input_packed);
    if !admissible(model, &state, &input, section)? {
        return Ok(None);
    }
    let next = model
        .step(&state, &input)
        .map_err(|error| reject(section, error.to_string()))?;
    let next_state = shape.states.iter().map(|id| next[id]).collect();
    let output = shape
        .outputs
        .iter()
        .map(|id| {
            model
                .evaluate(*id, &state, &input)
                .map_err(|error| reject(section, error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(LocalRelationRow {
        state: state_values,
        input: input_values,
        next_state,
        output,
    }))
}

pub fn produce_local_relation(
    source: &[u8],
    outputs: &[NodeId],
) -> Result<LocalRelationCertificate, RevisionLocalError> {
    let model = btor2::parse_bytes(source)
        .map_err(|error| reject(EvidenceSection::Envelope, error.to_string()))?;
    let shape = relation_shape(&model, outputs, EvidenceSection::Envelope)?;
    let mut rows = Vec::new();
    for state in 0..(1usize << shape.state_bits) {
        for input in 0..(1usize << shape.input_bits) {
            if let Some(row) =
                relation_row(&model, &shape, state, input, EvidenceSection::Envelope)?
            {
                rows.push(row);
            }
        }
    }
    Ok(LocalRelationCertificate {
        source_sha256: source_digest(source),
        states: shape.states,
        state_widths: shape.state_widths,
        inputs: shape.inputs,
        input_widths: shape.input_widths,
        outputs: shape.outputs,
        output_widths: shape.output_widths,
        constraints: shape.constraints,
        rows,
    })
}

pub fn verify_local_relation(
    source: &[u8],
    certificate: &LocalRelationCertificate,
    section: EvidenceSection,
) -> Result<LocalRelationSummary, RevisionLocalError> {
    if !matches!(section, EvidenceSection::Left | EvidenceSection::Right) {
        return Err(reject(
            EvidenceSection::Envelope,
            "local relation verifier requires left or right attribution",
        ));
    }
    if source_digest(source) != certificate.source_sha256 {
        return Err(reject(section, "local relation source binding is invalid"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(section, error.to_string()))?;
    let shape = relation_shape(&model, &certificate.outputs, section)?;
    if certificate.states != shape.states
        || certificate.state_widths != shape.state_widths
        || certificate.inputs != shape.inputs
        || certificate.input_widths != shape.input_widths
        || certificate.output_widths != shape.output_widths
        || certificate.constraints != shape.constraints
    {
        return Err(reject(
            section,
            "local relation shape does not match source",
        ));
    }
    let mut claimed = certificate.rows.iter();
    let mut admissible_rows = 0usize;
    for state in 0..(1usize << shape.state_bits) {
        for input in 0..(1usize << shape.input_bits) {
            if let Some(expected) = relation_row(&model, &shape, state, input, section)? {
                let actual = claimed
                    .next()
                    .ok_or_else(|| reject(section, "local relation omits an admissible row"))?;
                if actual != &expected {
                    return Err(reject(section, "local relation row does not match source"));
                }
                admissible_rows += 1;
            }
        }
    }
    if claimed.next().is_some() {
        return Err(reject(section, "local relation has extra rows"));
    }
    let source_initial = model
        .initial_state()
        .map_err(|error| reject(section, error.to_string()))?;
    let initial_state = shape.states.iter().map(|id| source_initial[id]).collect();
    Ok(LocalRelationSummary {
        state_bits: shape.state_bits,
        input_bits: shape.input_bits,
        output_bits: shape.output_bits,
        candidate_valuations: shape.candidate_valuations,
        admissible_rows,
        initial_state,
    })
}

pub fn verify_local_relation_for_composition<'a>(
    source: &[u8],
    certificate: &'a LocalRelationCertificate,
    section: EvidenceSection,
) -> Result<VerifiedLocalRelation<'a>, RevisionLocalError> {
    let summary = verify_local_relation(source, certificate, section)?;
    Ok(VerifiedLocalRelation {
        certificate,
        summary,
    })
}

enum ValidatedWire {
    LeftToRight {
        output_index: usize,
        input_index: usize,
    },
    RightToLeft {
        output_index: usize,
        input_index: usize,
    },
}

fn validate_interface(
    left: &LocalRelationCertificate,
    right: &LocalRelationCertificate,
    contract: &WordInterfaceContract,
) -> Result<Vec<ValidatedWire>, RevisionLocalError> {
    if contract.wires.is_empty() || contract.wires.len() > MAX_INTERFACE_WIRES {
        return Err(reject(
            EvidenceSection::Interface,
            "interface requires between one and eight wires",
        ));
    }
    if !contract.wires.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(reject(
            EvidenceSection::Interface,
            "interface wires must be unique and strictly ordered",
        ));
    }
    let mut destinations = Vec::new();
    let mut result = Vec::with_capacity(contract.wires.len());
    for wire in &contract.wires {
        let (source, destination, destination_side) = match wire.from {
            ComponentSide::Left => (left, right, ComponentSide::Right),
            ComponentSide::Right => (right, left, ComponentSide::Left),
        };
        let output_index = source
            .outputs
            .binary_search(&wire.output)
            .map_err(|_| reject(EvidenceSection::Interface, "wire output is not projected"))?;
        let input_index = destination
            .inputs
            .binary_search(&wire.to_input)
            .map_err(|_| reject(EvidenceSection::Interface, "wire input is not semantic"))?;
        if source.output_widths[output_index] != destination.input_widths[input_index] {
            return Err(reject(
                EvidenceSection::Interface,
                "wire output and input widths differ",
            ));
        }
        let destination_key = (destination_side, wire.to_input);
        if destinations.contains(&destination_key) {
            return Err(reject(
                EvidenceSection::Interface,
                "more than one wire drives an input",
            ));
        }
        destinations.push(destination_key);
        result.push(match wire.from {
            ComponentSide::Left => ValidatedWire::LeftToRight {
                output_index,
                input_index,
            },
            ComponentSide::Right => ValidatedWire::RightToLeft {
                output_index,
                input_index,
            },
        });
    }
    Ok(result)
}

fn pair_satisfies(
    left: &LocalRelationRow,
    right: &LocalRelationRow,
    wires: &[ValidatedWire],
) -> bool {
    wires.iter().all(|wire| match *wire {
        ValidatedWire::LeftToRight {
            output_index,
            input_index,
        } => left.output[output_index] == right.input[input_index],
        ValidatedWire::RightToLeft {
            output_index,
            input_index,
        } => right.output[output_index] == left.input[input_index],
    })
}

pub fn encode_word_interface_contract(
    contract: &WordInterfaceContract,
) -> Result<String, RevisionLocalError> {
    if contract.wires.is_empty() || contract.wires.len() > MAX_INTERFACE_WIRES {
        return Err(reject(
            EvidenceSection::Interface,
            "interface requires between one and eight wires",
        ));
    }
    if !contract.wires.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(reject(
            EvidenceSection::Interface,
            "interface wires must be unique and strictly ordered",
        ));
    }
    let mut text = format!(
        "word_interface_version={WORD_INTERFACE_CONTRACT_VERSION}\nwire_count={}\n",
        contract.wires.len()
    );
    for wire in &contract.wires {
        let side = match wire.from {
            ComponentSide::Left => "left",
            ComponentSide::Right => "right",
        };
        text.push_str(&format!("wire={side},{},{}\n", wire.output, wire.to_input));
    }
    text.push_str("status=complete\n");
    if text.len() > MAX_WORD_INTERFACE_CONTRACT_BYTES {
        return Err(reject(
            EvidenceSection::Interface,
            "interface contract exceeds byte limit",
        ));
    }
    Ok(text)
}

pub fn decode_word_interface_contract(
    bytes: &[u8],
) -> Result<WordInterfaceContract, RevisionLocalError> {
    if bytes.len() > MAX_WORD_INTERFACE_CONTRACT_BYTES
        || bytes.contains(&0)
        || bytes.contains(&b'\r')
        || !bytes.ends_with(b"\n")
    {
        return Err(reject(
            EvidenceSection::Interface,
            "interface contract is not bounded canonical LF text",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        reject(
            EvidenceSection::Interface,
            "interface contract is not UTF-8",
        )
    })?;
    let mut lines = text.lines();
    let version = lines
        .next()
        .and_then(|line| line.strip_prefix("word_interface_version="))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| reject(EvidenceSection::Interface, "invalid interface version"))?;
    if version != WORD_INTERFACE_CONTRACT_VERSION {
        return Err(reject(
            EvidenceSection::Interface,
            "unsupported interface version",
        ));
    }
    let wire_count = lines
        .next()
        .and_then(|line| line.strip_prefix("wire_count="))
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| (1..=MAX_INTERFACE_WIRES).contains(count))
        .ok_or_else(|| reject(EvidenceSection::Interface, "invalid interface wire count"))?;
    let mut wires = Vec::with_capacity(wire_count);
    for _ in 0..wire_count {
        let value = lines
            .next()
            .and_then(|line| line.strip_prefix("wire="))
            .ok_or_else(|| reject(EvidenceSection::Interface, "missing interface wire"))?;
        let mut fields = value.split(',');
        let from = match fields.next() {
            Some("left") => ComponentSide::Left,
            Some("right") => ComponentSide::Right,
            _ => return Err(reject(EvidenceSection::Interface, "invalid wire side")),
        };
        let output = fields
            .next()
            .and_then(|field| field.parse::<NodeId>().ok())
            .filter(|id| *id != 0)
            .ok_or_else(|| reject(EvidenceSection::Interface, "invalid wire output"))?;
        let to_input = fields
            .next()
            .and_then(|field| field.parse::<NodeId>().ok())
            .filter(|id| *id != 0)
            .ok_or_else(|| reject(EvidenceSection::Interface, "invalid wire input"))?;
        if fields.next().is_some() {
            return Err(reject(
                EvidenceSection::Interface,
                "wire has trailing fields",
            ));
        }
        wires.push(InterfaceWire {
            from,
            output,
            to_input,
        });
    }
    if lines.next() != Some("status=complete") || lines.next().is_some() {
        return Err(reject(
            EvidenceSection::Interface,
            "interface contract is incomplete or has trailing fields",
        ));
    }
    let contract = WordInterfaceContract { wires };
    if encode_word_interface_contract(&contract)? != text {
        return Err(reject(
            EvidenceSection::Interface,
            "interface contract is not canonical",
        ));
    }
    Ok(contract)
}

fn compose_validated_local_relations(
    left: &LocalRelationCertificate,
    right: &LocalRelationCertificate,
    contract: &WordInterfaceContract,
) -> Result<ComposedRelation, RevisionLocalError> {
    validate_local_relation_structure(left, EvidenceSection::Left)?;
    validate_local_relation_structure(right, EvidenceSection::Right)?;
    let wires = validate_interface(left, right, contract)?;
    let pair_checks = left
        .rows
        .len()
        .checked_mul(right.rows.len())
        .filter(|count| *count <= MAX_COMPOSED_PAIR_CHECKS)
        .ok_or_else(|| {
            reject(
                EvidenceSection::Interface,
                "interface pair-check count exceeds limit",
            )
        })?;
    let mut pairs = Vec::new();
    for (left_index, left_row) in left.rows.iter().enumerate() {
        for (right_index, right_row) in right.rows.iter().enumerate() {
            if pair_satisfies(left_row, right_row, &wires) {
                if pairs.len() == MAX_COMPOSED_PAIRS {
                    return Err(reject(
                        EvidenceSection::Interface,
                        "composed relation pair count exceeds limit",
                    ));
                }
                pairs.push(ComposedPair {
                    left_row: u32::try_from(left_index).expect("bounded local rows fit u32"),
                    right_row: u32::try_from(right_index).expect("bounded local rows fit u32"),
                });
            }
        }
    }
    let interface = encode_word_interface_contract(contract)?;
    Ok(ComposedRelation {
        interface_sha256: evidence_digest(interface.as_bytes()),
        pairs,
        pair_checks,
    })
}

pub fn compose_verified_local_relations(
    left: &VerifiedLocalRelation<'_>,
    right: &VerifiedLocalRelation<'_>,
    contract: &WordInterfaceContract,
) -> Result<ComposedRelation, RevisionLocalError> {
    compose_validated_local_relations(left.certificate, right.certificate, contract)
}

pub fn compose_local_relations(
    left_source: &[u8],
    left: &LocalRelationCertificate,
    right_source: &[u8],
    right: &LocalRelationCertificate,
    contract: &WordInterfaceContract,
) -> Result<ComposedRelation, RevisionLocalError> {
    let left = verify_local_relation_for_composition(left_source, left, EvidenceSection::Left)?;
    let right = verify_local_relation_for_composition(right_source, right, EvidenceSection::Right)?;
    compose_verified_local_relations(&left, &right, contract)
}

#[derive(Clone)]
struct IndexedTransition {
    pair: u32,
    next: CombinedState,
    bad: bool,
}

fn bad_output_index(
    relation: &LocalRelationCertificate,
    output: NodeId,
) -> Result<usize, RevisionLocalError> {
    let index = relation
        .outputs
        .binary_search(&output)
        .map_err(|_| reject(EvidenceSection::Final, "bad output is not projected"))?;
    if relation.output_widths[index] != 1 {
        return Err(reject(EvidenceSection::Final, "bad output must be one bit"));
    }
    Ok(index)
}

fn transition_index(
    left: &LocalRelationCertificate,
    right: &LocalRelationCertificate,
    composed: &ComposedRelation,
    query: &BoundedQuery,
) -> Result<BTreeMap<CombinedState, Vec<IndexedTransition>>, RevisionLocalError> {
    if query.horizon > MAX_FINAL_HORIZON {
        return Err(reject(
            EvidenceSection::Final,
            "bounded answer horizon exceeds limit",
        ));
    }
    let bad_index = match query.bad_side {
        ComponentSide::Left => bad_output_index(left, query.bad_output)?,
        ComponentSide::Right => bad_output_index(right, query.bad_output)?,
    };
    let mut index = BTreeMap::<CombinedState, Vec<IndexedTransition>>::new();
    for (pair_index, pair) in composed.pairs.iter().enumerate() {
        let left_row = left
            .rows
            .get(pair.left_row as usize)
            .ok_or_else(|| reject(EvidenceSection::Final, "left row index is invalid"))?;
        let right_row = right
            .rows
            .get(pair.right_row as usize)
            .ok_or_else(|| reject(EvidenceSection::Final, "right row index is invalid"))?;
        let current = CombinedState {
            left: left_row.state.clone(),
            right: right_row.state.clone(),
        };
        let next = CombinedState {
            left: left_row.next_state.clone(),
            right: right_row.next_state.clone(),
        };
        let bad = match query.bad_side {
            ComponentSide::Left => left_row.output[bad_index] != 0,
            ComponentSide::Right => right_row.output[bad_index] != 0,
        };
        index.entry(current).or_default().push(IndexedTransition {
            pair: u32::try_from(pair_index).expect("bounded composed pairs fit u32"),
            next,
            bad,
        });
    }
    Ok(index)
}

fn initial_combined_state(
    left: &VerifiedLocalRelation<'_>,
    right: &VerifiedLocalRelation<'_>,
) -> CombinedState {
    CombinedState {
        left: left.summary.initial_state.clone(),
        right: right.summary.initial_state.clone(),
    }
}

fn reconstruct_witness(
    predecessors: &[BTreeMap<CombinedState, (CombinedState, u32)>],
    mut state: CombinedState,
    terminal_pair: u32,
) -> Vec<u32> {
    let mut reversed = Vec::with_capacity(predecessors.len() + 1);
    for layer in predecessors.iter().rev() {
        let (previous, pair) = &layer[&state];
        reversed.push(*pair);
        state = previous.clone();
    }
    reversed.reverse();
    reversed.push(terminal_pair);
    reversed
}

fn bounded_certificate(
    left: &VerifiedLocalRelation<'_>,
    right: &VerifiedLocalRelation<'_>,
    contract: &WordInterfaceContract,
    query: &BoundedQuery,
) -> Result<(BoundedAnswerCertificate, BoundedAnswerSummary), RevisionLocalError> {
    let composed = compose_verified_local_relations(left, right, contract)?;
    let index = transition_index(left.certificate, right.certificate, &composed, query)?;
    let mut current = BTreeSet::from([initial_combined_state(left, right)]);
    let mut layers = vec![current.iter().cloned().collect::<Vec<_>>()];
    let mut predecessors = Vec::<BTreeMap<CombinedState, (CombinedState, u32)>>::new();
    let mut reachable_states = 1usize;
    let mut transition_checks = 0usize;
    for frame in 0..=query.horizon {
        let mut next = BTreeSet::new();
        let mut next_predecessors = BTreeMap::new();
        for state in &current {
            for transition in index.get(state).into_iter().flatten() {
                transition_checks = transition_checks
                    .checked_add(1)
                    .filter(|count| *count <= MAX_FINAL_TRANSITION_CHECKS)
                    .ok_or_else(|| {
                        reject(
                            EvidenceSection::Final,
                            "bounded answer transition checks exceed limit",
                        )
                    })?;
                if transition.bad {
                    let bad_frame = Some(frame);
                    let certificate = BoundedAnswerCertificate {
                        left_sha256: *left.source_sha256(),
                        right_sha256: *right.source_sha256(),
                        interface_sha256: composed.interface_sha256,
                        query: query.clone(),
                        result: BoundedResult::Unsafe,
                        bad_frame,
                        witness_pairs: reconstruct_witness(
                            &predecessors,
                            state.clone(),
                            transition.pair,
                        ),
                        layers: Vec::new(),
                    };
                    return Ok((
                        certificate,
                        BoundedAnswerSummary {
                            result: BoundedResult::Unsafe,
                            horizon: query.horizon,
                            bad_frame,
                            reachable_states,
                            transition_checks,
                        },
                    ));
                }
                if frame < query.horizon && next.insert(transition.next.clone()) {
                    next_predecessors
                        .insert(transition.next.clone(), (state.clone(), transition.pair));
                }
            }
        }
        if frame < query.horizon {
            if next.len() > MAX_FINAL_STATES_PER_LAYER {
                return Err(reject(
                    EvidenceSection::Final,
                    "bounded answer layer exceeds state limit",
                ));
            }
            reachable_states = reachable_states
                .checked_add(next.len())
                .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
                .ok_or_else(|| {
                    reject(
                        EvidenceSection::Final,
                        "bounded answer total states exceed limit",
                    )
                })?;
            layers.push(next.iter().cloned().collect());
            predecessors.push(next_predecessors);
            current = next;
        }
    }
    let certificate = BoundedAnswerCertificate {
        left_sha256: *left.source_sha256(),
        right_sha256: *right.source_sha256(),
        interface_sha256: composed.interface_sha256,
        query: query.clone(),
        result: BoundedResult::Safe,
        bad_frame: None,
        witness_pairs: Vec::new(),
        layers,
    };
    Ok((
        certificate,
        BoundedAnswerSummary {
            result: BoundedResult::Safe,
            horizon: query.horizon,
            bad_frame: None,
            reachable_states,
            transition_checks,
        },
    ))
}

pub fn produce_bounded_answer(
    left: &VerifiedLocalRelation<'_>,
    right: &VerifiedLocalRelation<'_>,
    contract: &WordInterfaceContract,
    query: &BoundedQuery,
) -> Result<BoundedAnswerCertificate, RevisionLocalError> {
    bounded_certificate(left, right, contract, query).map(|(certificate, _)| certificate)
}

pub fn verify_bounded_answer(
    left: &VerifiedLocalRelation<'_>,
    right: &VerifiedLocalRelation<'_>,
    contract: &WordInterfaceContract,
    certificate: &BoundedAnswerCertificate,
) -> Result<BoundedAnswerSummary, RevisionLocalError> {
    if certificate.left_sha256 != *left.source_sha256() {
        return Err(reject(
            EvidenceSection::Left,
            "final source binding is invalid",
        ));
    }
    if certificate.right_sha256 != *right.source_sha256() {
        return Err(reject(
            EvidenceSection::Right,
            "final source binding is invalid",
        ));
    }
    let composed = compose_verified_local_relations(left, right, contract)?;
    if certificate.interface_sha256 != composed.interface_sha256 {
        return Err(reject(
            EvidenceSection::Interface,
            "final interface binding is invalid",
        ));
    }
    let index = transition_index(
        left.certificate,
        right.certificate,
        &composed,
        &certificate.query,
    )?;
    let initial = initial_combined_state(left, right);
    let mut current = BTreeSet::from([initial.clone()]);
    let mut reachable_states = 1usize;
    let mut transition_checks = 0usize;
    let advance_count = |count: &mut usize| -> Result<(), RevisionLocalError> {
        *count = count
            .checked_add(1)
            .filter(|value| *value <= MAX_FINAL_TRANSITION_CHECKS)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "bounded answer transition checks exceed limit",
                )
            })?;
        Ok(())
    };
    match certificate.result {
        BoundedResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_pairs.is_empty()
                || certificate.layers.len() != certificate.query.horizon as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "SAFE bounded answer shape is invalid",
                ));
            }
            for frame in 0..=certificate.query.horizon {
                let claimed = &certificate.layers[frame as usize];
                if claimed.len() > MAX_FINAL_STATES_PER_LAYER
                    || claimed.windows(2).any(|pair| pair[0] >= pair[1])
                    || claimed.iter().cloned().collect::<BTreeSet<_>>() != current
                {
                    return Err(reject(
                        EvidenceSection::Final,
                        "SAFE bounded answer layer is incomplete or noncanonical",
                    ));
                }
                let mut next = BTreeSet::new();
                for state in &current {
                    for transition in index.get(state).into_iter().flatten() {
                        advance_count(&mut transition_checks)?;
                        if transition.bad {
                            return Err(reject(
                                EvidenceSection::Final,
                                "SAFE bounded answer contains a bad transition",
                            ));
                        }
                        if frame < certificate.query.horizon {
                            next.insert(transition.next.clone());
                        }
                    }
                }
                if frame < certificate.query.horizon {
                    if next.len() > MAX_FINAL_STATES_PER_LAYER {
                        return Err(reject(
                            EvidenceSection::Final,
                            "bounded answer layer exceeds state limit",
                        ));
                    }
                    reachable_states = reachable_states
                        .checked_add(next.len())
                        .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
                        .ok_or_else(|| {
                            reject(
                                EvidenceSection::Final,
                                "bounded answer total states exceed limit",
                            )
                        })?;
                    current = next;
                }
            }
            Ok(BoundedAnswerSummary {
                result: BoundedResult::Safe,
                horizon: certificate.query.horizon,
                bad_frame: None,
                reachable_states,
                transition_checks,
            })
        }
        BoundedResult::Unsafe => {
            let bad_frame = certificate.bad_frame.ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "UNSAFE bounded answer has no bad frame",
                )
            })?;
            if bad_frame > certificate.query.horizon
                || !certificate.layers.is_empty()
                || certificate.witness_pairs.len() != bad_frame as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "UNSAFE bounded answer shape is invalid",
                ));
            }
            for frame in 0..=bad_frame {
                let mut next = BTreeSet::new();
                let mut found_bad = false;
                for state in &current {
                    for transition in index.get(state).into_iter().flatten() {
                        advance_count(&mut transition_checks)?;
                        found_bad |= transition.bad;
                        if frame < bad_frame {
                            next.insert(transition.next.clone());
                        }
                    }
                }
                if frame < bad_frame && found_bad {
                    return Err(reject(
                        EvidenceSection::Final,
                        "UNSAFE bounded answer does not claim the earliest bad frame",
                    ));
                }
                if frame == bad_frame && !found_bad {
                    return Err(reject(
                        EvidenceSection::Final,
                        "UNSAFE bounded answer frame has no bad transition",
                    ));
                }
                if frame < bad_frame {
                    if next.len() > MAX_FINAL_STATES_PER_LAYER {
                        return Err(reject(
                            EvidenceSection::Final,
                            "bounded answer layer exceeds state limit",
                        ));
                    }
                    reachable_states = reachable_states
                        .checked_add(next.len())
                        .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
                        .ok_or_else(|| {
                            reject(
                                EvidenceSection::Final,
                                "bounded answer total states exceed limit",
                            )
                        })?;
                    current = next;
                }
            }
            let mut witness_state = initial;
            for (frame, pair) in certificate.witness_pairs.iter().enumerate() {
                let transition = index
                    .get(&witness_state)
                    .and_then(|transitions| transitions.iter().find(|item| item.pair == *pair))
                    .ok_or_else(|| {
                        reject(EvidenceSection::Final, "UNSAFE witness pair is not enabled")
                    })?;
                if frame == bad_frame as usize {
                    if !transition.bad {
                        return Err(reject(
                            EvidenceSection::Final,
                            "UNSAFE terminal witness is not bad",
                        ));
                    }
                } else {
                    witness_state = transition.next.clone();
                }
            }
            Ok(BoundedAnswerSummary {
                result: BoundedResult::Unsafe,
                horizon: certificate.query.horizon,
                bad_frame: Some(bad_frame),
                reachable_states,
                transition_checks,
            })
        }
    }
}

fn validate_combined_state_shape(state: &CombinedState) -> bool {
    !state.left.is_empty()
        && state.left.len() <= MAX_LOCAL_STATE_BITS
        && !state.right.is_empty()
        && state.right.len() <= MAX_LOCAL_STATE_BITS
}

fn validate_bounded_answer_structure(
    certificate: &BoundedAnswerCertificate,
) -> Result<(), RevisionLocalError> {
    if certificate.query.horizon > MAX_FINAL_HORIZON || certificate.query.bad_output == 0 {
        return Err(reject(
            EvidenceSection::Final,
            "bounded answer query is invalid",
        ));
    }
    match certificate.result {
        BoundedResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_pairs.is_empty()
                || certificate.layers.len() != certificate.query.horizon as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "SAFE bounded answer shape is invalid",
                ));
            }
        }
        BoundedResult::Unsafe => {
            let bad_frame = certificate.bad_frame.ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "UNSAFE bounded answer has no bad frame",
                )
            })?;
            if bad_frame > certificate.query.horizon
                || !certificate.layers.is_empty()
                || certificate.witness_pairs.len() != bad_frame as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "UNSAFE bounded answer shape is invalid",
                ));
            }
        }
    }
    let mut total_states = 0usize;
    for layer in &certificate.layers {
        if layer.len() > MAX_FINAL_STATES_PER_LAYER
            || layer.windows(2).any(|pair| pair[0] >= pair[1])
            || !layer.iter().all(validate_combined_state_shape)
        {
            return Err(reject(
                EvidenceSection::Final,
                "bounded answer layer shape is invalid",
            ));
        }
        total_states = total_states
            .checked_add(layer.len())
            .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "bounded answer total states exceed limit",
                )
            })?;
    }
    Ok(())
}

fn append_values(output: &mut Vec<u8>, values: &[u64]) -> Result<(), RevisionLocalError> {
    append_u32(output, values.len(), EvidenceSection::Final)?;
    for value in values {
        output.extend_from_slice(&value.to_be_bytes());
    }
    Ok(())
}

pub fn encode_bounded_answer_certificate(
    certificate: &BoundedAnswerCertificate,
) -> Result<Vec<u8>, RevisionLocalError> {
    validate_bounded_answer_structure(certificate)?;
    let mut output = Vec::new();
    output.extend_from_slice(BOUNDED_ANSWER_MAGIC);
    output.extend_from_slice(&BOUNDED_ANSWER_CERTIFICATE_VERSION.to_be_bytes());
    output.extend_from_slice(&certificate.left_sha256);
    output.extend_from_slice(&certificate.right_sha256);
    output.extend_from_slice(&certificate.interface_sha256);
    output.extend_from_slice(&certificate.query.horizon.to_be_bytes());
    output.push(match certificate.query.bad_side {
        ComponentSide::Left => 0,
        ComponentSide::Right => 1,
    });
    output.extend_from_slice(&certificate.query.bad_output.to_be_bytes());
    output.push(match certificate.result {
        BoundedResult::Safe => 0,
        BoundedResult::Unsafe => 1,
    });
    output.extend_from_slice(&certificate.bad_frame.unwrap_or(u32::MAX).to_be_bytes());
    append_u32(
        &mut output,
        certificate.witness_pairs.len(),
        EvidenceSection::Final,
    )?;
    for pair in &certificate.witness_pairs {
        output.extend_from_slice(&pair.to_be_bytes());
    }
    append_u32(
        &mut output,
        certificate.layers.len(),
        EvidenceSection::Final,
    )?;
    for layer in &certificate.layers {
        append_u32(&mut output, layer.len(), EvidenceSection::Final)?;
        for state in layer {
            append_values(&mut output, &state.left)?;
            append_values(&mut output, &state.right)?;
        }
    }
    if output.len() > MAX_FINAL_SECTION_BYTES {
        return Err(reject(
            EvidenceSection::Final,
            "bounded answer encoding exceeds byte limit",
        ));
    }
    Ok(output)
}

pub fn decode_bounded_answer_certificate(
    bytes: &[u8],
) -> Result<BoundedAnswerCertificate, RevisionLocalError> {
    if bytes.len() > MAX_FINAL_SECTION_BYTES {
        return Err(reject(
            EvidenceSection::Final,
            "bounded answer encoding exceeds byte limit",
        ));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(BOUNDED_ANSWER_MAGIC.len(), EvidenceSection::Final)? != BOUNDED_ANSWER_MAGIC {
        return Err(reject(
            EvidenceSection::Final,
            "invalid bounded answer magic",
        ));
    }
    if decoder.u32(EvidenceSection::Final)? != BOUNDED_ANSWER_CERTIFICATE_VERSION {
        return Err(reject(
            EvidenceSection::Final,
            "unsupported bounded answer version",
        ));
    }
    let left_sha256 = decoder.digest(EvidenceSection::Final)?;
    let right_sha256 = decoder.digest(EvidenceSection::Final)?;
    let interface_sha256 = decoder.digest(EvidenceSection::Final)?;
    let horizon = decoder.u32(EvidenceSection::Final)?;
    let bad_side = match decoder.byte(EvidenceSection::Final)? {
        0 => ComponentSide::Left,
        1 => ComponentSide::Right,
        _ => return Err(reject(EvidenceSection::Final, "invalid bad side")),
    };
    let bad_output = decoder.u64(EvidenceSection::Final)?;
    let result = match decoder.byte(EvidenceSection::Final)? {
        0 => BoundedResult::Safe,
        1 => BoundedResult::Unsafe,
        _ => return Err(reject(EvidenceSection::Final, "invalid bounded result")),
    };
    let encoded_bad_frame = decoder.u32(EvidenceSection::Final)?;
    let bad_frame = (encoded_bad_frame != u32::MAX).then_some(encoded_bad_frame);
    let witness_count = decoder.bounded_count(
        MAX_FINAL_HORIZON as usize + 1,
        EvidenceSection::Final,
        "witness",
    )?;
    let mut witness_pairs = Vec::with_capacity(witness_count);
    for _ in 0..witness_count {
        witness_pairs.push(decoder.u32(EvidenceSection::Final)?);
    }
    let layer_count = decoder.bounded_count(
        MAX_FINAL_HORIZON as usize + 1,
        EvidenceSection::Final,
        "layer",
    )?;
    let mut layers = Vec::with_capacity(layer_count);
    let mut total_states = 0usize;
    for _ in 0..layer_count {
        let state_count =
            decoder.bounded_count(MAX_FINAL_STATES_PER_LAYER, EvidenceSection::Final, "state")?;
        total_states = total_states
            .checked_add(state_count)
            .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "bounded answer total states exceed limit",
                )
            })?;
        let mut layer = Vec::with_capacity(state_count);
        for _ in 0..state_count {
            let left_count = decoder.bounded_count(
                MAX_LOCAL_STATE_BITS,
                EvidenceSection::Final,
                "left-state-value",
            )?;
            let left = decoder.values(left_count, EvidenceSection::Final)?;
            let right_count = decoder.bounded_count(
                MAX_LOCAL_STATE_BITS,
                EvidenceSection::Final,
                "right-state-value",
            )?;
            let right = decoder.values(right_count, EvidenceSection::Final)?;
            layer.push(CombinedState { left, right });
        }
        layers.push(layer);
    }
    if decoder.offset != bytes.len() {
        return Err(reject(
            EvidenceSection::Final,
            "bounded answer has trailing bytes",
        ));
    }
    let certificate = BoundedAnswerCertificate {
        left_sha256,
        right_sha256,
        interface_sha256,
        query: BoundedQuery {
            horizon,
            bad_side,
            bad_output,
        },
        result,
        bad_frame,
        witness_pairs,
        layers,
    };
    validate_bounded_answer_structure(&certificate)?;
    if encode_bounded_answer_certificate(&certificate)? != bytes {
        return Err(reject(
            EvidenceSection::Final,
            "bounded answer encoding is not canonical",
        ));
    }
    Ok(certificate)
}

fn attribute(mut error: RevisionLocalError, section: EvidenceSection) -> RevisionLocalError {
    if error.section == EvidenceSection::Envelope {
        error.section = section;
    }
    error
}

pub fn produce_revision_local_certificate(
    left_source: &[u8],
    left_outputs: &[NodeId],
    right_source: &[u8],
    right_outputs: &[NodeId],
    interface_source: &[u8],
    query: &BoundedQuery,
) -> Result<(RevisionLocalCertificate, RevisionLocalSummary), RevisionLocalError> {
    let left_relation = produce_local_relation(left_source, left_outputs)
        .map_err(|error| attribute(error, EvidenceSection::Left))?;
    let right_relation = produce_local_relation(right_source, right_outputs)
        .map_err(|error| attribute(error, EvidenceSection::Right))?;
    let left =
        verify_local_relation_for_composition(left_source, &left_relation, EvidenceSection::Left)?;
    let right = verify_local_relation_for_composition(
        right_source,
        &right_relation,
        EvidenceSection::Right,
    )?;
    let contract = decode_word_interface_contract(interface_source)?;
    let (answer, answer_summary) = bounded_certificate(&left, &right, &contract, query)?;
    let certificate = RevisionLocalCertificate {
        left: LocalEvidence {
            source_sha256: source_digest(left_source),
            evidence: encode_local_relation_certificate(&left_relation)
                .map_err(|error| attribute(error, EvidenceSection::Left))?,
        },
        right: LocalEvidence {
            source_sha256: source_digest(right_source),
            evidence: encode_local_relation_certificate(&right_relation)
                .map_err(|error| attribute(error, EvidenceSection::Right))?,
        },
        interface: BoundEvidence {
            source_sha256: source_digest(interface_source),
            evidence: interface_source.to_vec(),
        },
        final_evidence: encode_bounded_answer_certificate(&answer)?,
    };
    let certificate_bytes = encode_revision_local_certificate(&certificate)?.len();
    Ok((
        certificate,
        RevisionLocalSummary {
            left: left.summary.clone(),
            right: right.summary.clone(),
            answer: answer_summary,
            certificate_bytes,
        },
    ))
}

pub fn verify_revision_local_certificate(
    left_source: &[u8],
    right_source: &[u8],
    interface_source: &[u8],
    certificate: &RevisionLocalCertificate,
) -> Result<RevisionLocalSummary, RevisionLocalError> {
    verify_source_bindings(left_source, right_source, interface_source, certificate)?;
    if certificate.interface.evidence != interface_source {
        return Err(reject(
            EvidenceSection::Interface,
            "embedded interface contract differs from supplied source",
        ));
    }
    let left_relation = decode_local_relation_certificate(&certificate.left.evidence)
        .map_err(|error| attribute(error, EvidenceSection::Left))?;
    let right_relation = decode_local_relation_certificate(&certificate.right.evidence)
        .map_err(|error| attribute(error, EvidenceSection::Right))?;
    let left =
        verify_local_relation_for_composition(left_source, &left_relation, EvidenceSection::Left)?;
    let right = verify_local_relation_for_composition(
        right_source,
        &right_relation,
        EvidenceSection::Right,
    )?;
    let contract = decode_word_interface_contract(interface_source)?;
    let answer = decode_bounded_answer_certificate(&certificate.final_evidence)?;
    let answer_summary = verify_bounded_answer(&left, &right, &contract, &answer)?;
    let certificate_bytes = encode_revision_local_certificate(certificate)?.len();
    Ok(RevisionLocalSummary {
        left: left.summary.clone(),
        right: right.summary.clone(),
        answer: answer_summary,
        certificate_bytes,
    })
}

pub fn validate_local_artifact(
    source: &[u8],
    encoded: &[u8],
    section: EvidenceSection,
) -> Result<ValidatedLocalArtifact, RevisionLocalError> {
    if !matches!(section, EvidenceSection::Left | EvidenceSection::Right) {
        return Err(reject(
            EvidenceSection::Envelope,
            "local artifact validation requires left or right attribution",
        ));
    }
    let certificate =
        decode_local_relation_certificate(encoded).map_err(|error| attribute(error, section))?;
    let summary = verify_local_relation(source, &certificate, section)?;
    Ok(ValidatedLocalArtifact {
        certificate,
        summary,
        encoded: encoded.to_vec(),
    })
}

pub fn verify_revision_with_retained_left(
    retained_left: &ValidatedLocalArtifact,
    right_source: &[u8],
    interface_source: &[u8],
    certificate: &RevisionLocalCertificate,
) -> Result<(RevisionLocalSummary, RevisionWorkObservation), RevisionLocalError> {
    if certificate.left.source_sha256 != *retained_left.source_sha256()
        || certificate.left.evidence != retained_left.encoded()
    {
        return Err(reject(
            EvidenceSection::Left,
            "retained left evidence is not byte-identical",
        ));
    }
    if certificate.right.source_sha256 != source_digest(right_source) {
        return Err(reject(
            EvidenceSection::Right,
            "right source binding is invalid",
        ));
    }
    if certificate.interface.source_sha256 != source_digest(interface_source)
        || certificate.interface.evidence != interface_source
    {
        return Err(reject(
            EvidenceSection::Interface,
            "interface source binding is invalid",
        ));
    }
    let right_artifact = validate_local_artifact(
        right_source,
        &certificate.right.evidence,
        EvidenceSection::Right,
    )?;
    let left = retained_left.verified();
    let right = right_artifact.verified();
    let contract = decode_word_interface_contract(interface_source)?;
    let composed = compose_verified_local_relations(&left, &right, &contract)?;
    let answer = decode_bounded_answer_certificate(&certificate.final_evidence)?;
    let answer_summary = verify_bounded_answer(&left, &right, &contract, &answer)?;
    let certificate_bytes = encode_revision_local_certificate(certificate)?.len();
    Ok((
        RevisionLocalSummary {
            left: retained_left.summary.clone(),
            right: right_artifact.summary.clone(),
            answer: answer_summary.clone(),
            certificate_bytes,
        },
        RevisionWorkObservation {
            decoded_local_sections: 1,
            semantically_verified_local_sections: 1,
            reused_local_sections: 1,
            composed_pair_checks: composed.pair_checks,
            final_transition_checks: answer_summary.transition_checks,
        },
    ))
}

struct DirectContext {
    left: Btor2Model,
    right: Btor2Model,
    left_inputs: Vec<NodeId>,
    right_inputs: Vec<NodeId>,
    left_input_widths: Vec<u32>,
    right_input_widths: Vec<u32>,
    left_input_bits: usize,
    input_valuations: usize,
    wires: Vec<InterfaceWire>,
    bad_side: ComponentSide,
    bad_output: NodeId,
}

fn direct_context(
    left_source: &[u8],
    right_source: &[u8],
    contract: &WordInterfaceContract,
    query: &BoundedQuery,
) -> Result<DirectContext, RevisionLocalError> {
    if query.horizon > MAX_FINAL_HORIZON {
        return Err(reject(
            EvidenceSection::Final,
            "direct fallback horizon exceeds limit",
        ));
    }
    encode_word_interface_contract(contract)?;
    let left = btor2::parse_bytes(left_source)
        .map_err(|error| reject(EvidenceSection::Left, error.to_string()))?;
    let right = btor2::parse_bytes(right_source)
        .map_err(|error| reject(EvidenceSection::Right, error.to_string()))?;
    if left.states().is_empty()
        || right.states().is_empty()
        || left.states().len() > MAX_DIRECT_STATE_NODES
        || right.states().len() > MAX_DIRECT_STATE_NODES
    {
        return Err(reject(
            EvidenceSection::Final,
            "direct fallback state-node count exceeds limit",
        ));
    }
    let left_inputs = left.inputs().to_vec();
    let right_inputs = right.inputs().to_vec();
    let left_input_widths = widths(&left, &left_inputs, EvidenceSection::Left, "input")?;
    let right_input_widths = widths(&right, &right_inputs, EvidenceSection::Right, "input")?;
    let left_input_bits = checked_bits(
        &left_input_widths,
        MAX_DIRECT_INPUT_BITS,
        EvidenceSection::Left,
        "direct input",
    )?;
    let right_input_bits = checked_bits(
        &right_input_widths,
        MAX_DIRECT_INPUT_BITS,
        EvidenceSection::Right,
        "direct input",
    )?;
    let input_bits = left_input_bits
        .checked_add(right_input_bits)
        .filter(|bits| *bits <= MAX_DIRECT_INPUT_BITS)
        .ok_or_else(|| {
            reject(
                EvidenceSection::Final,
                "direct fallback joint input width exceeds limit",
            )
        })?;
    let input_valuations = 1usize
        .checked_shl(input_bits as u32)
        .filter(|count| *count <= MAX_DIRECT_INPUT_VALUATIONS)
        .ok_or_else(|| {
            reject(
                EvidenceSection::Final,
                "direct fallback valuation count exceeds limit",
            )
        })?;
    let bad_model = match query.bad_side {
        ComponentSide::Left => &left,
        ComponentSide::Right => &right,
    };
    let bad_node = bad_model
        .nodes()
        .get(&query.bad_output)
        .ok_or_else(|| reject(EvidenceSection::Final, "direct bad output is unknown"))?;
    if bad_node.width != 1 {
        return Err(reject(
            EvidenceSection::Final,
            "direct bad output must be one bit",
        ));
    }
    let mut destinations = Vec::new();
    for wire in &contract.wires {
        let (source, destination, destination_side) = match wire.from {
            ComponentSide::Left => (&left, &right, ComponentSide::Right),
            ComponentSide::Right => (&right, &left, ComponentSide::Left),
        };
        let source_width = source
            .nodes()
            .get(&wire.output)
            .map(|node| node.width)
            .ok_or_else(|| reject(EvidenceSection::Interface, "wire output is unknown"))?;
        let input_position = destination
            .inputs()
            .binary_search(&wire.to_input)
            .map_err(|_| reject(EvidenceSection::Interface, "wire input is not semantic"))?;
        if source_width != destination.nodes()[&wire.to_input].width {
            return Err(reject(
                EvidenceSection::Interface,
                "wire output and input widths differ",
            ));
        }
        let destination_key = (destination_side, destination.inputs()[input_position]);
        if destinations.contains(&destination_key) {
            return Err(reject(
                EvidenceSection::Interface,
                "more than one wire drives an input",
            ));
        }
        destinations.push(destination_key);
    }
    Ok(DirectContext {
        left,
        right,
        left_inputs,
        right_inputs,
        left_input_widths,
        right_input_widths,
        left_input_bits,
        input_valuations,
        wires: contract.wires.clone(),
        bad_side: query.bad_side,
        bad_output: query.bad_output,
    })
}

fn state_map(
    model: &Btor2Model,
    values: &[u64],
    section: EvidenceSection,
) -> Result<WordValues, RevisionLocalError> {
    if values.len() != model.states().len() {
        return Err(reject(section, "direct state vector length is invalid"));
    }
    let mut state = BTreeMap::new();
    for (id, value) in model.states().iter().zip(values) {
        let width = model.nodes()[id].width;
        if width < 64 && (*value >> width) != 0 {
            return Err(reject(section, "direct state value exceeds width"));
        }
        state.insert(*id, *value);
    }
    Ok(state)
}

fn direct_transition(
    context: &DirectContext,
    current: &CombinedState,
    packed: u16,
) -> Result<Option<(CombinedState, bool)>, RevisionLocalError> {
    let left_state = state_map(&context.left, &current.left, EvidenceSection::Left)?;
    let right_state = state_map(&context.right, &current.right, EvidenceSection::Right)?;
    let left_mask = (1usize << context.left_input_bits) - 1;
    let (left_input, _) = unpack(
        &context.left_inputs,
        &context.left_input_widths,
        usize::from(packed) & left_mask,
    );
    let (right_input, _) = unpack(
        &context.right_inputs,
        &context.right_input_widths,
        usize::from(packed) >> context.left_input_bits,
    );
    if !admissible(
        &context.left,
        &left_state,
        &left_input,
        EvidenceSection::Left,
    )? || !admissible(
        &context.right,
        &right_state,
        &right_input,
        EvidenceSection::Right,
    )? {
        return Ok(None);
    }
    for wire in &context.wires {
        let (source, source_state, source_input, destination_input) = match wire.from {
            ComponentSide::Left => (&context.left, &left_state, &left_input, &right_input),
            ComponentSide::Right => (&context.right, &right_state, &right_input, &left_input),
        };
        let output = source
            .evaluate(wire.output, source_state, source_input)
            .map_err(|error| reject(EvidenceSection::Interface, error.to_string()))?;
        if destination_input[&wire.to_input] != output {
            return Ok(None);
        }
    }
    let (bad_model, bad_state, bad_input) = match context.bad_side {
        ComponentSide::Left => (&context.left, &left_state, &left_input),
        ComponentSide::Right => (&context.right, &right_state, &right_input),
    };
    let bad = bad_model
        .evaluate(context.bad_output, bad_state, bad_input)
        .map_err(|error| reject(EvidenceSection::Final, error.to_string()))?
        != 0;
    let left_next = context
        .left
        .step(&left_state, &left_input)
        .map_err(|error| reject(EvidenceSection::Left, error.to_string()))?;
    let right_next = context
        .right
        .step(&right_state, &right_input)
        .map_err(|error| reject(EvidenceSection::Right, error.to_string()))?;
    Ok(Some((
        CombinedState {
            left: context
                .left
                .states()
                .iter()
                .map(|id| left_next[id])
                .collect(),
            right: context
                .right
                .states()
                .iter()
                .map(|id| right_next[id])
                .collect(),
        },
        bad,
    )))
}

fn direct_initial(context: &DirectContext) -> Result<CombinedState, RevisionLocalError> {
    let left = context
        .left
        .initial_state()
        .map_err(|error| reject(EvidenceSection::Left, error.to_string()))?;
    let right = context
        .right
        .initial_state()
        .map_err(|error| reject(EvidenceSection::Right, error.to_string()))?;
    Ok(CombinedState {
        left: context.left.states().iter().map(|id| left[id]).collect(),
        right: context.right.states().iter().map(|id| right[id]).collect(),
    })
}

fn reconstruct_direct_witness(
    predecessors: &[BTreeMap<CombinedState, (CombinedState, u16)>],
    mut state: CombinedState,
    terminal_valuation: u16,
) -> Vec<u16> {
    let mut reversed = Vec::with_capacity(predecessors.len() + 1);
    for layer in predecessors.iter().rev() {
        let (previous, valuation) = &layer[&state];
        reversed.push(*valuation);
        state = previous.clone();
    }
    reversed.reverse();
    reversed.push(terminal_valuation);
    reversed
}

pub fn produce_direct_answer(
    left_source: &[u8],
    right_source: &[u8],
    interface_source: &[u8],
    query: &BoundedQuery,
) -> Result<(DirectAnswerCertificate, DirectAnswerSummary), RevisionLocalError> {
    let contract = decode_word_interface_contract(interface_source)?;
    let context = direct_context(left_source, right_source, &contract, query)?;
    let mut current = BTreeSet::from([direct_initial(&context)?]);
    let mut layers = vec![current.iter().cloned().collect::<Vec<_>>()];
    let mut predecessors = Vec::<BTreeMap<CombinedState, (CombinedState, u16)>>::new();
    let mut reachable_states = 1usize;
    let mut valuation_checks = 0usize;
    for frame in 0..=query.horizon {
        let mut next = BTreeSet::new();
        let mut next_predecessors = BTreeMap::new();
        for state in &current {
            for packed in 0..context.input_valuations {
                valuation_checks = valuation_checks
                    .checked_add(1)
                    .filter(|count| *count <= MAX_FINAL_TRANSITION_CHECKS)
                    .ok_or_else(|| {
                        reject(
                            EvidenceSection::Final,
                            "direct fallback valuation checks exceed limit",
                        )
                    })?;
                let packed = u16::try_from(packed).expect("bounded valuations fit u16");
                if let Some((successor, bad)) = direct_transition(&context, state, packed)? {
                    if bad {
                        let bad_frame = Some(frame);
                        return Ok((
                            DirectAnswerCertificate {
                                left_sha256: source_digest(left_source),
                                right_sha256: source_digest(right_source),
                                interface_sha256: source_digest(interface_source),
                                query: query.clone(),
                                result: BoundedResult::Unsafe,
                                bad_frame,
                                witness_valuations: reconstruct_direct_witness(
                                    &predecessors,
                                    state.clone(),
                                    packed,
                                ),
                                layers: Vec::new(),
                            },
                            DirectAnswerSummary {
                                result: BoundedResult::Unsafe,
                                horizon: query.horizon,
                                bad_frame,
                                reachable_states,
                                valuation_checks,
                            },
                        ));
                    }
                    if frame < query.horizon && next.insert(successor.clone()) {
                        next_predecessors.insert(successor, (state.clone(), packed));
                    }
                }
            }
        }
        if frame < query.horizon {
            if next.len() > MAX_FINAL_STATES_PER_LAYER {
                return Err(reject(
                    EvidenceSection::Final,
                    "direct fallback layer exceeds state limit",
                ));
            }
            reachable_states = reachable_states
                .checked_add(next.len())
                .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
                .ok_or_else(|| {
                    reject(
                        EvidenceSection::Final,
                        "direct fallback total states exceed limit",
                    )
                })?;
            layers.push(next.iter().cloned().collect());
            predecessors.push(next_predecessors);
            current = next;
        }
    }
    Ok((
        DirectAnswerCertificate {
            left_sha256: source_digest(left_source),
            right_sha256: source_digest(right_source),
            interface_sha256: source_digest(interface_source),
            query: query.clone(),
            result: BoundedResult::Safe,
            bad_frame: None,
            witness_valuations: Vec::new(),
            layers,
        },
        DirectAnswerSummary {
            result: BoundedResult::Safe,
            horizon: query.horizon,
            bad_frame: None,
            reachable_states,
            valuation_checks,
        },
    ))
}

pub fn verify_direct_answer(
    left_source: &[u8],
    right_source: &[u8],
    interface_source: &[u8],
    certificate: &DirectAnswerCertificate,
) -> Result<DirectAnswerSummary, RevisionLocalError> {
    if certificate.left_sha256 != source_digest(left_source) {
        return Err(reject(
            EvidenceSection::Left,
            "direct source binding is invalid",
        ));
    }
    if certificate.right_sha256 != source_digest(right_source) {
        return Err(reject(
            EvidenceSection::Right,
            "direct source binding is invalid",
        ));
    }
    if certificate.interface_sha256 != source_digest(interface_source) {
        return Err(reject(
            EvidenceSection::Interface,
            "direct interface binding is invalid",
        ));
    }
    let contract = decode_word_interface_contract(interface_source)?;
    let context = direct_context(left_source, right_source, &contract, &certificate.query)?;
    let initial = direct_initial(&context)?;
    let mut current = BTreeSet::from([initial.clone()]);
    let mut reachable_states = 1usize;
    let mut valuation_checks = 0usize;
    let advance = |checks: &mut usize| -> Result<(), RevisionLocalError> {
        *checks = checks
            .checked_add(1)
            .filter(|count| *count <= MAX_FINAL_TRANSITION_CHECKS)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "direct fallback valuation checks exceed limit",
                )
            })?;
        Ok(())
    };
    match certificate.result {
        BoundedResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_valuations.is_empty()
                || certificate.layers.len() != certificate.query.horizon as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "direct SAFE evidence shape is invalid",
                ));
            }
            for frame in 0..=certificate.query.horizon {
                let claimed = &certificate.layers[frame as usize];
                if claimed.len() > MAX_FINAL_STATES_PER_LAYER
                    || claimed.windows(2).any(|pair| pair[0] >= pair[1])
                    || claimed.iter().cloned().collect::<BTreeSet<_>>() != current
                {
                    return Err(reject(
                        EvidenceSection::Final,
                        "direct SAFE layer is incomplete or noncanonical",
                    ));
                }
                let mut next = BTreeSet::new();
                for state in &current {
                    for packed in 0..context.input_valuations {
                        advance(&mut valuation_checks)?;
                        let packed = u16::try_from(packed).expect("bounded valuations fit u16");
                        if let Some((successor, bad)) = direct_transition(&context, state, packed)?
                        {
                            if bad {
                                return Err(reject(
                                    EvidenceSection::Final,
                                    "direct SAFE evidence contains a bad valuation",
                                ));
                            }
                            if frame < certificate.query.horizon {
                                next.insert(successor);
                            }
                        }
                    }
                }
                if frame < certificate.query.horizon {
                    if next.len() > MAX_FINAL_STATES_PER_LAYER {
                        return Err(reject(
                            EvidenceSection::Final,
                            "direct fallback layer exceeds state limit",
                        ));
                    }
                    reachable_states = reachable_states
                        .checked_add(next.len())
                        .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
                        .ok_or_else(|| {
                            reject(
                                EvidenceSection::Final,
                                "direct fallback total states exceed limit",
                            )
                        })?;
                    current = next;
                }
            }
            Ok(DirectAnswerSummary {
                result: BoundedResult::Safe,
                horizon: certificate.query.horizon,
                bad_frame: None,
                reachable_states,
                valuation_checks,
            })
        }
        BoundedResult::Unsafe => {
            let bad_frame = certificate.bad_frame.ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "direct UNSAFE evidence has no bad frame",
                )
            })?;
            if bad_frame > certificate.query.horizon
                || !certificate.layers.is_empty()
                || certificate.witness_valuations.len() != bad_frame as usize + 1
                || certificate
                    .witness_valuations
                    .iter()
                    .any(|value| usize::from(*value) >= context.input_valuations)
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "direct UNSAFE evidence shape is invalid",
                ));
            }
            for frame in 0..=bad_frame {
                let mut next = BTreeSet::new();
                let mut found_bad = false;
                for state in &current {
                    for packed in 0..context.input_valuations {
                        advance(&mut valuation_checks)?;
                        let packed = u16::try_from(packed).expect("bounded valuations fit u16");
                        if let Some((successor, bad)) = direct_transition(&context, state, packed)?
                        {
                            found_bad |= bad;
                            if frame < bad_frame {
                                next.insert(successor);
                            }
                        }
                    }
                }
                if frame < bad_frame && found_bad {
                    return Err(reject(
                        EvidenceSection::Final,
                        "direct UNSAFE evidence does not claim the earliest bad frame",
                    ));
                }
                if frame == bad_frame && !found_bad {
                    return Err(reject(
                        EvidenceSection::Final,
                        "direct UNSAFE frame has no bad valuation",
                    ));
                }
                if frame < bad_frame {
                    reachable_states = reachable_states
                        .checked_add(next.len())
                        .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
                        .ok_or_else(|| {
                            reject(
                                EvidenceSection::Final,
                                "direct fallback total states exceed limit",
                            )
                        })?;
                    current = next;
                }
            }
            let mut witness_state = initial;
            for (frame, packed) in certificate.witness_valuations.iter().enumerate() {
                let (successor, bad) = direct_transition(&context, &witness_state, *packed)?
                    .ok_or_else(|| {
                        reject(
                            EvidenceSection::Final,
                            "direct UNSAFE witness valuation is inadmissible",
                        )
                    })?;
                if frame == bad_frame as usize {
                    if !bad {
                        return Err(reject(
                            EvidenceSection::Final,
                            "direct UNSAFE terminal valuation is not bad",
                        ));
                    }
                } else {
                    witness_state = successor;
                }
            }
            Ok(DirectAnswerSummary {
                result: BoundedResult::Unsafe,
                horizon: certificate.query.horizon,
                bad_frame: Some(bad_frame),
                reachable_states,
                valuation_checks,
            })
        }
    }
}

fn validate_direct_answer_structure(
    certificate: &DirectAnswerCertificate,
) -> Result<(), RevisionLocalError> {
    if certificate.query.horizon > MAX_FINAL_HORIZON || certificate.query.bad_output == 0 {
        return Err(reject(
            EvidenceSection::Final,
            "direct answer query is invalid",
        ));
    }
    match certificate.result {
        BoundedResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_valuations.is_empty()
                || certificate.layers.len() != certificate.query.horizon as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "direct SAFE evidence shape is invalid",
                ));
            }
        }
        BoundedResult::Unsafe => {
            let bad_frame = certificate.bad_frame.ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "direct UNSAFE evidence has no bad frame",
                )
            })?;
            if bad_frame > certificate.query.horizon
                || !certificate.layers.is_empty()
                || certificate.witness_valuations.len() != bad_frame as usize + 1
            {
                return Err(reject(
                    EvidenceSection::Final,
                    "direct UNSAFE evidence shape is invalid",
                ));
            }
        }
    }
    let mut total_states = 0usize;
    for layer in &certificate.layers {
        if layer.len() > MAX_FINAL_STATES_PER_LAYER
            || layer.windows(2).any(|pair| pair[0] >= pair[1])
            || !layer.iter().all(|state| {
                !state.left.is_empty()
                    && state.left.len() <= MAX_DIRECT_STATE_NODES
                    && !state.right.is_empty()
                    && state.right.len() <= MAX_DIRECT_STATE_NODES
            })
        {
            return Err(reject(
                EvidenceSection::Final,
                "direct answer layer shape is invalid",
            ));
        }
        total_states = total_states
            .checked_add(layer.len())
            .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "direct answer total states exceed limit",
                )
            })?;
    }
    Ok(())
}

pub fn encode_direct_answer_certificate(
    certificate: &DirectAnswerCertificate,
) -> Result<Vec<u8>, RevisionLocalError> {
    validate_direct_answer_structure(certificate)?;
    let mut output = Vec::new();
    output.extend_from_slice(DIRECT_ANSWER_MAGIC);
    output.extend_from_slice(&DIRECT_ANSWER_CERTIFICATE_VERSION.to_be_bytes());
    output.extend_from_slice(&certificate.left_sha256);
    output.extend_from_slice(&certificate.right_sha256);
    output.extend_from_slice(&certificate.interface_sha256);
    output.extend_from_slice(&certificate.query.horizon.to_be_bytes());
    output.push(match certificate.query.bad_side {
        ComponentSide::Left => 0,
        ComponentSide::Right => 1,
    });
    output.extend_from_slice(&certificate.query.bad_output.to_be_bytes());
    output.push(match certificate.result {
        BoundedResult::Safe => 0,
        BoundedResult::Unsafe => 1,
    });
    output.extend_from_slice(&certificate.bad_frame.unwrap_or(u32::MAX).to_be_bytes());
    append_u32(
        &mut output,
        certificate.witness_valuations.len(),
        EvidenceSection::Final,
    )?;
    for valuation in &certificate.witness_valuations {
        output.extend_from_slice(&valuation.to_be_bytes());
    }
    append_u32(
        &mut output,
        certificate.layers.len(),
        EvidenceSection::Final,
    )?;
    for layer in &certificate.layers {
        append_u32(&mut output, layer.len(), EvidenceSection::Final)?;
        for state in layer {
            append_values(&mut output, &state.left)?;
            append_values(&mut output, &state.right)?;
        }
    }
    if output.len() > MAX_FINAL_SECTION_BYTES {
        return Err(reject(
            EvidenceSection::Final,
            "direct answer encoding exceeds byte limit",
        ));
    }
    Ok(output)
}

pub fn decode_direct_answer_certificate(
    bytes: &[u8],
) -> Result<DirectAnswerCertificate, RevisionLocalError> {
    if bytes.len() > MAX_FINAL_SECTION_BYTES {
        return Err(reject(
            EvidenceSection::Final,
            "direct answer encoding exceeds byte limit",
        ));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(DIRECT_ANSWER_MAGIC.len(), EvidenceSection::Final)? != DIRECT_ANSWER_MAGIC {
        return Err(reject(
            EvidenceSection::Final,
            "invalid direct answer magic",
        ));
    }
    if decoder.u32(EvidenceSection::Final)? != DIRECT_ANSWER_CERTIFICATE_VERSION {
        return Err(reject(
            EvidenceSection::Final,
            "unsupported direct answer version",
        ));
    }
    let left_sha256 = decoder.digest(EvidenceSection::Final)?;
    let right_sha256 = decoder.digest(EvidenceSection::Final)?;
    let interface_sha256 = decoder.digest(EvidenceSection::Final)?;
    let horizon = decoder.u32(EvidenceSection::Final)?;
    let bad_side = match decoder.byte(EvidenceSection::Final)? {
        0 => ComponentSide::Left,
        1 => ComponentSide::Right,
        _ => return Err(reject(EvidenceSection::Final, "invalid direct bad side")),
    };
    let bad_output = decoder.u64(EvidenceSection::Final)?;
    let result = match decoder.byte(EvidenceSection::Final)? {
        0 => BoundedResult::Safe,
        1 => BoundedResult::Unsafe,
        _ => return Err(reject(EvidenceSection::Final, "invalid direct result")),
    };
    let encoded_bad_frame = decoder.u32(EvidenceSection::Final)?;
    let bad_frame = (encoded_bad_frame != u32::MAX).then_some(encoded_bad_frame);
    let witness_count = decoder.bounded_count(
        MAX_FINAL_HORIZON as usize + 1,
        EvidenceSection::Final,
        "direct-witness",
    )?;
    let mut witness_valuations = Vec::with_capacity(witness_count);
    for _ in 0..witness_count {
        witness_valuations.push(decoder.u16(EvidenceSection::Final)?);
    }
    let layer_count = decoder.bounded_count(
        MAX_FINAL_HORIZON as usize + 1,
        EvidenceSection::Final,
        "direct-layer",
    )?;
    let mut layers = Vec::with_capacity(layer_count);
    let mut total_states = 0usize;
    for _ in 0..layer_count {
        let state_count = decoder.bounded_count(
            MAX_FINAL_STATES_PER_LAYER,
            EvidenceSection::Final,
            "direct-state",
        )?;
        total_states = total_states
            .checked_add(state_count)
            .filter(|count| *count <= MAX_FINAL_TOTAL_STATES)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Final,
                    "direct answer total states exceed limit",
                )
            })?;
        let mut layer = Vec::with_capacity(state_count);
        for _ in 0..state_count {
            let left_count = decoder.bounded_count(
                MAX_DIRECT_STATE_NODES,
                EvidenceSection::Final,
                "direct-left-state-value",
            )?;
            let left = decoder.values(left_count, EvidenceSection::Final)?;
            let right_count = decoder.bounded_count(
                MAX_DIRECT_STATE_NODES,
                EvidenceSection::Final,
                "direct-right-state-value",
            )?;
            let right = decoder.values(right_count, EvidenceSection::Final)?;
            layer.push(CombinedState { left, right });
        }
        layers.push(layer);
    }
    if decoder.offset != bytes.len() {
        return Err(reject(
            EvidenceSection::Final,
            "direct answer has trailing bytes",
        ));
    }
    let certificate = DirectAnswerCertificate {
        left_sha256,
        right_sha256,
        interface_sha256,
        query: BoundedQuery {
            horizon,
            bad_side,
            bad_output,
        },
        result,
        bad_frame,
        witness_valuations,
        layers,
    };
    validate_direct_answer_structure(&certificate)?;
    if encode_direct_answer_certificate(&certificate)? != bytes {
        return Err(reject(
            EvidenceSection::Final,
            "direct answer encoding is not canonical",
        ));
    }
    Ok(certificate)
}

fn local_side_admission(
    model: &Btor2Model,
    outputs: &[NodeId],
    state_reason: RevisionSelectionReason,
    input_reason: RevisionSelectionReason,
    output_reason: RevisionSelectionReason,
    node_reason: RevisionSelectionReason,
    section: EvidenceSection,
) -> Result<(Option<RevisionSelectionReason>, usize), RevisionLocalError> {
    if model.states().is_empty() || model.inputs().is_empty() {
        return Err(reject(
            section,
            "portfolio requires state and semantic input nodes",
        ));
    }
    if outputs.is_empty() || outputs.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(reject(
            section,
            "portfolio output projection must be nonempty and strictly ordered",
        ));
    }
    let state_widths = widths(model, model.states(), section, "state")?;
    let input_widths = widths(model, model.inputs(), section, "input")?;
    let output_widths = widths(model, outputs, section, "output")?;
    let sum = |values: &[u32], label: &str| -> Result<usize, RevisionLocalError> {
        values
            .iter()
            .try_fold(0usize, |total, width| total.checked_add(*width as usize))
            .ok_or_else(|| reject(section, format!("portfolio {label} width overflowed")))
    };
    let state_bits = sum(&state_widths, "state")?;
    let input_bits = sum(&input_widths, "input")?;
    let output_bits = sum(&output_widths, "output")?;
    if state_bits > MAX_LOCAL_STATE_BITS {
        return Ok((Some(state_reason), 0));
    }
    if input_bits > MAX_LOCAL_INPUT_BITS {
        return Ok((Some(input_reason), 0));
    }
    if output_bits > MAX_LOCAL_OUTPUT_BITS {
        return Ok((Some(output_reason), 0));
    }
    let candidate_valuations = 1usize << (state_bits + input_bits);
    let node_steps = candidate_valuations.saturating_mul(model.nodes().len());
    if node_steps > MAX_LOCAL_NODE_STEPS {
        return Ok((Some(node_reason), 0));
    }
    Ok((None, candidate_valuations))
}

pub fn assess_revision_portfolio(
    left_source: &[u8],
    left_outputs: &[NodeId],
    right_source: &[u8],
    right_outputs: &[NodeId],
    interface_source: &[u8],
    query: &BoundedQuery,
) -> Result<RevisionSelectionReason, RevisionLocalError> {
    if query.horizon > MAX_FINAL_HORIZON {
        return Err(reject(
            EvidenceSection::Final,
            "portfolio horizon exceeds limit",
        ));
    }
    decode_word_interface_contract(interface_source)?;
    let left = btor2::parse_bytes(left_source)
        .map_err(|error| reject(EvidenceSection::Left, error.to_string()))?;
    let right = btor2::parse_bytes(right_source)
        .map_err(|error| reject(EvidenceSection::Right, error.to_string()))?;
    let (left_reason, left_candidates) = local_side_admission(
        &left,
        left_outputs,
        RevisionSelectionReason::LeftStateWidth,
        RevisionSelectionReason::LeftInputWidth,
        RevisionSelectionReason::LeftOutputWidth,
        RevisionSelectionReason::LeftNodeSteps,
        EvidenceSection::Left,
    )?;
    if let Some(reason) = left_reason {
        return Ok(reason);
    }
    let (right_reason, right_candidates) = local_side_admission(
        &right,
        right_outputs,
        RevisionSelectionReason::RightStateWidth,
        RevisionSelectionReason::RightInputWidth,
        RevisionSelectionReason::RightOutputWidth,
        RevisionSelectionReason::RightNodeSteps,
        EvidenceSection::Right,
    )?;
    if let Some(reason) = right_reason {
        return Ok(reason);
    }
    if left_candidates
        .checked_mul(right_candidates)
        .filter(|count| *count <= MAX_COMPOSED_PAIR_CHECKS)
        .is_none()
    {
        return Ok(RevisionSelectionReason::PairCheckBound);
    }
    Ok(RevisionSelectionReason::ExactLocalRelationAdmitted)
}

pub fn produce_revision_portfolio(
    left_source: &[u8],
    left_outputs: &[NodeId],
    right_source: &[u8],
    right_outputs: &[NodeId],
    interface_source: &[u8],
    query: &BoundedQuery,
) -> Result<RevisionPortfolioProduction, RevisionLocalError> {
    let reason = assess_revision_portfolio(
        left_source,
        left_outputs,
        right_source,
        right_outputs,
        interface_source,
        query,
    )?;
    if reason == RevisionSelectionReason::ExactLocalRelationAdmitted {
        let (certificate, _) = produce_revision_local_certificate(
            left_source,
            left_outputs,
            right_source,
            right_outputs,
            interface_source,
            query,
        )?;
        Ok(RevisionPortfolioProduction {
            certificate: RevisionPortfolioCertificate::RevisionLocal(certificate),
            backend: RevisionPortfolioBackend::RevisionLocal,
            reason,
        })
    } else {
        let (certificate, _) =
            produce_direct_answer(left_source, right_source, interface_source, query)?;
        Ok(RevisionPortfolioProduction {
            certificate: RevisionPortfolioCertificate::DirectExact(certificate),
            backend: RevisionPortfolioBackend::DirectExact,
            reason,
        })
    }
}

pub fn verify_revision_portfolio(
    left_source: &[u8],
    left_outputs: &[NodeId],
    right_source: &[u8],
    right_outputs: &[NodeId],
    interface_source: &[u8],
    production: &RevisionPortfolioProduction,
) -> Result<RevisionPortfolioSummary, RevisionLocalError> {
    let query = match &production.certificate {
        RevisionPortfolioCertificate::RevisionLocal(certificate) => {
            decode_bounded_answer_certificate(&certificate.final_evidence)?.query
        }
        RevisionPortfolioCertificate::DirectExact(certificate) => certificate.query.clone(),
    };
    let expected_reason = assess_revision_portfolio(
        left_source,
        left_outputs,
        right_source,
        right_outputs,
        interface_source,
        &query,
    )?;
    let expected_backend = if expected_reason == RevisionSelectionReason::ExactLocalRelationAdmitted
    {
        RevisionPortfolioBackend::RevisionLocal
    } else {
        RevisionPortfolioBackend::DirectExact
    };
    if production.reason != expected_reason || production.backend != expected_backend {
        return Err(reject(
            EvidenceSection::Envelope,
            "portfolio selection does not match static assessment",
        ));
    }
    match (&production.certificate, expected_backend) {
        (
            RevisionPortfolioCertificate::RevisionLocal(certificate),
            RevisionPortfolioBackend::RevisionLocal,
        ) => {
            let summary = verify_revision_local_certificate(
                left_source,
                right_source,
                interface_source,
                certificate,
            )?;
            Ok(RevisionPortfolioSummary {
                result: summary.answer.result,
                bad_frame: summary.answer.bad_frame,
                backend: expected_backend,
                reason: expected_reason,
                certificate_bytes: summary.certificate_bytes,
            })
        }
        (
            RevisionPortfolioCertificate::DirectExact(certificate),
            RevisionPortfolioBackend::DirectExact,
        ) => {
            let summary =
                verify_direct_answer(left_source, right_source, interface_source, certificate)?;
            Ok(RevisionPortfolioSummary {
                result: summary.result,
                bad_frame: summary.bad_frame,
                backend: expected_backend,
                reason: expected_reason,
                certificate_bytes: encode_direct_answer_certificate(certificate)?.len(),
            })
        }
        _ => Err(reject(
            EvidenceSection::Envelope,
            "portfolio certificate backend does not match static assessment",
        )),
    }
}

fn valid_ids(ids: &[NodeId], maximum: usize) -> bool {
    !ids.is_empty()
        && ids.len() <= maximum
        && ids.iter().all(|id| *id != 0)
        && ids.windows(2).all(|pair| pair[0] < pair[1])
}

fn validate_local_relation_structure(
    certificate: &LocalRelationCertificate,
    section: EvidenceSection,
) -> Result<(), RevisionLocalError> {
    if !valid_ids(&certificate.states, MAX_LOCAL_STATE_BITS)
        || !valid_ids(&certificate.inputs, MAX_LOCAL_INPUT_BITS)
        || !valid_ids(&certificate.outputs, MAX_LOCAL_OUTPUT_BITS)
    {
        return Err(reject(section, "local relation node vectors are invalid"));
    }
    if certificate.states.len() != certificate.state_widths.len()
        || certificate.inputs.len() != certificate.input_widths.len()
        || certificate.outputs.len() != certificate.output_widths.len()
    {
        return Err(reject(section, "local relation width vectors are invalid"));
    }
    if certificate
        .state_widths
        .iter()
        .chain(&certificate.input_widths)
        .chain(&certificate.output_widths)
        .any(|width| !(1..=64).contains(width))
    {
        return Err(reject(section, "local relation contains an invalid width"));
    }
    checked_bits(
        &certificate.state_widths,
        MAX_LOCAL_STATE_BITS,
        section,
        "state",
    )?;
    checked_bits(
        &certificate.input_widths,
        MAX_LOCAL_INPUT_BITS,
        section,
        "input",
    )?;
    checked_bits(
        &certificate.output_widths,
        MAX_LOCAL_OUTPUT_BITS,
        section,
        "output",
    )?;
    if certificate.constraints.len() > MAX_LOCAL_CONSTRAINTS
        || certificate.constraints.contains(&0)
        || !certificate
            .constraints
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    {
        return Err(reject(
            section,
            "local relation constraint vector is invalid",
        ));
    }
    if certificate.rows.len() > MAX_LOCAL_VALUATIONS {
        return Err(reject(section, "local relation row count exceeds limit"));
    }
    for row in &certificate.rows {
        if row.state.len() != certificate.states.len()
            || row.input.len() != certificate.inputs.len()
            || row.next_state.len() != certificate.states.len()
            || row.output.len() != certificate.outputs.len()
        {
            return Err(reject(section, "local relation row shape is invalid"));
        }
        for (value, width) in row
            .state
            .iter()
            .zip(&certificate.state_widths)
            .chain(row.input.iter().zip(&certificate.input_widths))
            .chain(row.next_state.iter().zip(&certificate.state_widths))
            .chain(row.output.iter().zip(&certificate.output_widths))
        {
            if *width < 64 && (*value >> *width) != 0 {
                return Err(reject(section, "local relation row value exceeds width"));
            }
        }
    }
    Ok(())
}

fn append_u32(
    output: &mut Vec<u8>,
    value: usize,
    section: EvidenceSection,
) -> Result<(), RevisionLocalError> {
    let value = u32::try_from(value)
        .map_err(|_| reject(section, "local relation count cannot be encoded"))?;
    output.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn append_id_widths(
    output: &mut Vec<u8>,
    ids: &[NodeId],
    widths: &[u32],
    section: EvidenceSection,
) -> Result<(), RevisionLocalError> {
    append_u32(output, ids.len(), section)?;
    for (id, width) in ids.iter().zip(widths) {
        output.extend_from_slice(&id.to_be_bytes());
        output.extend_from_slice(&width.to_be_bytes());
    }
    Ok(())
}

pub fn encode_local_relation_certificate(
    certificate: &LocalRelationCertificate,
) -> Result<Vec<u8>, RevisionLocalError> {
    validate_local_relation_structure(certificate, EvidenceSection::Envelope)?;
    let mut output = Vec::new();
    output.extend_from_slice(LOCAL_RELATION_MAGIC);
    output.extend_from_slice(&LOCAL_RELATION_CERTIFICATE_VERSION.to_be_bytes());
    output.extend_from_slice(&certificate.source_sha256);
    append_id_widths(
        &mut output,
        &certificate.states,
        &certificate.state_widths,
        EvidenceSection::Envelope,
    )?;
    append_id_widths(
        &mut output,
        &certificate.inputs,
        &certificate.input_widths,
        EvidenceSection::Envelope,
    )?;
    append_id_widths(
        &mut output,
        &certificate.outputs,
        &certificate.output_widths,
        EvidenceSection::Envelope,
    )?;
    append_u32(
        &mut output,
        certificate.constraints.len(),
        EvidenceSection::Envelope,
    )?;
    for constraint in &certificate.constraints {
        output.extend_from_slice(&constraint.to_be_bytes());
    }
    append_u32(
        &mut output,
        certificate.rows.len(),
        EvidenceSection::Envelope,
    )?;
    for row in &certificate.rows {
        for value in row
            .state
            .iter()
            .chain(&row.input)
            .chain(&row.next_state)
            .chain(&row.output)
        {
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
    if output.len() > MAX_LOCAL_SECTION_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "encoded local relation exceeds byte limit",
        ));
    }
    Ok(output)
}

fn decode_id_widths(
    decoder: &mut Decoder<'_>,
    maximum: usize,
    section: EvidenceSection,
) -> Result<(Vec<NodeId>, Vec<u32>), RevisionLocalError> {
    let count = decoder.bounded_count(maximum, section, "node")?;
    if count == 0 {
        return Err(reject(section, "local relation node vector is empty"));
    }
    let mut ids = Vec::with_capacity(count);
    let mut widths = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(decoder.u64(section)?);
        widths.push(decoder.u32(section)?);
    }
    Ok((ids, widths))
}

pub fn decode_local_relation_certificate(
    bytes: &[u8],
) -> Result<LocalRelationCertificate, RevisionLocalError> {
    if bytes.len() > MAX_LOCAL_SECTION_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "encoded local relation exceeds byte limit",
        ));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(LOCAL_RELATION_MAGIC.len(), EvidenceSection::Envelope)? != LOCAL_RELATION_MAGIC
    {
        return Err(reject(
            EvidenceSection::Envelope,
            "invalid local relation magic",
        ));
    }
    if decoder.u32(EvidenceSection::Envelope)? != LOCAL_RELATION_CERTIFICATE_VERSION {
        return Err(reject(
            EvidenceSection::Envelope,
            "unsupported local relation version",
        ));
    }
    let source_sha256 = decoder.digest(EvidenceSection::Envelope)?;
    let (states, state_widths) = decode_id_widths(
        &mut decoder,
        MAX_LOCAL_STATE_BITS,
        EvidenceSection::Envelope,
    )?;
    let (inputs, input_widths) = decode_id_widths(
        &mut decoder,
        MAX_LOCAL_INPUT_BITS,
        EvidenceSection::Envelope,
    )?;
    let (outputs, output_widths) = decode_id_widths(
        &mut decoder,
        MAX_LOCAL_OUTPUT_BITS,
        EvidenceSection::Envelope,
    )?;
    let constraint_count = decoder.bounded_count(
        MAX_LOCAL_CONSTRAINTS,
        EvidenceSection::Envelope,
        "constraint",
    )?;
    let mut constraints = Vec::with_capacity(constraint_count);
    for _ in 0..constraint_count {
        constraints.push(decoder.u64(EvidenceSection::Envelope)?);
    }
    let row_count =
        decoder.bounded_count(MAX_LOCAL_VALUATIONS, EvidenceSection::Envelope, "row")?;
    let mut rows = Vec::with_capacity(row_count);
    for _ in 0..row_count {
        let state = decoder.values(states.len(), EvidenceSection::Envelope)?;
        let input = decoder.values(inputs.len(), EvidenceSection::Envelope)?;
        let next_state = decoder.values(states.len(), EvidenceSection::Envelope)?;
        let output = decoder.values(outputs.len(), EvidenceSection::Envelope)?;
        rows.push(LocalRelationRow {
            state,
            input,
            next_state,
            output,
        });
    }
    if decoder.offset != bytes.len() {
        return Err(reject(
            EvidenceSection::Envelope,
            "local relation has trailing bytes",
        ));
    }
    let certificate = LocalRelationCertificate {
        source_sha256,
        states,
        state_widths,
        inputs,
        input_widths,
        outputs,
        output_widths,
        constraints,
        rows,
    };
    validate_local_relation_structure(&certificate, EvidenceSection::Envelope)?;
    if encode_local_relation_certificate(&certificate)? != bytes {
        return Err(reject(
            EvidenceSection::Envelope,
            "local relation encoding is not canonical",
        ));
    }
    Ok(certificate)
}

pub fn verify_source_bindings(
    left_source: &[u8],
    right_source: &[u8],
    interface_source: &[u8],
    certificate: &RevisionLocalCertificate,
) -> Result<(), RevisionLocalError> {
    if source_digest(left_source) != certificate.left.source_sha256 {
        return Err(reject(EvidenceSection::Left, "source binding is invalid"));
    }
    if source_digest(right_source) != certificate.right.source_sha256 {
        return Err(reject(EvidenceSection::Right, "source binding is invalid"));
    }
    if source_digest(interface_source) != certificate.interface.source_sha256 {
        return Err(reject(
            EvidenceSection::Interface,
            "source binding is invalid",
        ));
    }
    Ok(())
}

pub fn unchanged_local_evidence(
    previous: &RevisionLocalCertificate,
    next: &RevisionLocalCertificate,
    section: EvidenceSection,
) -> Result<bool, RevisionLocalError> {
    match section {
        EvidenceSection::Left => Ok(previous.left == next.left),
        EvidenceSection::Right => Ok(previous.right == next.right),
        _ => Err(reject(
            EvidenceSection::Envelope,
            "reuse comparison requires a local component section",
        )),
    }
}

fn append_section(
    output: &mut Vec<u8>,
    section: EvidenceSection,
    digest: Option<&[u8; 32]>,
    bytes: &[u8],
    limit: usize,
) -> Result<(), RevisionLocalError> {
    if bytes.is_empty() {
        return Err(reject(section, "section is empty"));
    }
    if bytes.len() > limit {
        return Err(reject(section, "section exceeds byte limit"));
    }
    if let Some(digest) = digest {
        output.extend_from_slice(digest);
    }
    let length = u32::try_from(bytes.len())
        .map_err(|_| reject(section, "section length cannot be encoded"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

pub fn encode_revision_local_certificate(
    certificate: &RevisionLocalCertificate,
) -> Result<Vec<u8>, RevisionLocalError> {
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&REVISION_LOCAL_CERTIFICATE_VERSION.to_be_bytes());
    append_section(
        &mut output,
        EvidenceSection::Left,
        Some(&certificate.left.source_sha256),
        &certificate.left.evidence,
        MAX_LOCAL_SECTION_BYTES,
    )?;
    append_section(
        &mut output,
        EvidenceSection::Right,
        Some(&certificate.right.source_sha256),
        &certificate.right.evidence,
        MAX_LOCAL_SECTION_BYTES,
    )?;
    append_section(
        &mut output,
        EvidenceSection::Interface,
        Some(&certificate.interface.source_sha256),
        &certificate.interface.evidence,
        MAX_INTERFACE_SECTION_BYTES,
    )?;
    append_section(
        &mut output,
        EvidenceSection::Final,
        None,
        &certificate.final_evidence,
        MAX_FINAL_SECTION_BYTES,
    )?;
    if output.len() > MAX_REVISION_LOCAL_CERTIFICATE_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "certificate exceeds byte limit",
        ));
    }
    Ok(output)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn take(
        &mut self,
        count: usize,
        section: EvidenceSection,
    ) -> Result<&'a [u8], RevisionLocalError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject(section, "certificate is truncated"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self, section: EvidenceSection) -> Result<u32, RevisionLocalError> {
        let bytes: [u8; 4] = self.take(4, section)?.try_into().expect("fixed size");
        Ok(u32::from_be_bytes(bytes))
    }

    fn u16(&mut self, section: EvidenceSection) -> Result<u16, RevisionLocalError> {
        let bytes: [u8; 2] = self.take(2, section)?.try_into().expect("fixed size");
        Ok(u16::from_be_bytes(bytes))
    }

    fn byte(&mut self, section: EvidenceSection) -> Result<u8, RevisionLocalError> {
        Ok(self.take(1, section)?[0])
    }

    fn u64(&mut self, section: EvidenceSection) -> Result<u64, RevisionLocalError> {
        let bytes: [u8; 8] = self.take(8, section)?.try_into().expect("fixed size");
        Ok(u64::from_be_bytes(bytes))
    }

    fn bounded_count(
        &mut self,
        maximum: usize,
        section: EvidenceSection,
        label: &str,
    ) -> Result<usize, RevisionLocalError> {
        let count = usize::try_from(self.u32(section)?).expect("u32 fits usize");
        if count > maximum {
            return Err(reject(
                section,
                format!("local relation {label} count exceeds limit"),
            ));
        }
        Ok(count)
    }

    fn values(
        &mut self,
        count: usize,
        section: EvidenceSection,
    ) -> Result<Vec<u64>, RevisionLocalError> {
        (0..count).map(|_| self.u64(section)).collect()
    }

    fn digest(&mut self, section: EvidenceSection) -> Result<[u8; 32], RevisionLocalError> {
        Ok(self.take(32, section)?.try_into().expect("fixed size"))
    }

    fn section(
        &mut self,
        section: EvidenceSection,
        limit: usize,
    ) -> Result<Vec<u8>, RevisionLocalError> {
        let length = usize::try_from(self.u32(section)?).expect("u32 fits usize");
        if length == 0 {
            return Err(reject(section, "section is empty"));
        }
        if length > limit {
            return Err(reject(section, "section exceeds byte limit"));
        }
        Ok(self.take(length, section)?.to_vec())
    }
}

pub fn decode_revision_local_certificate(
    bytes: &[u8],
) -> Result<RevisionLocalCertificate, RevisionLocalError> {
    if bytes.len() > MAX_REVISION_LOCAL_CERTIFICATE_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "certificate exceeds byte limit",
        ));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(MAGIC.len(), EvidenceSection::Envelope)? != MAGIC {
        return Err(reject(EvidenceSection::Envelope, "invalid magic"));
    }
    if decoder.u32(EvidenceSection::Envelope)? != REVISION_LOCAL_CERTIFICATE_VERSION {
        return Err(reject(EvidenceSection::Envelope, "unsupported version"));
    }
    let left_digest = decoder.digest(EvidenceSection::Left)?;
    let left = decoder.section(EvidenceSection::Left, MAX_LOCAL_SECTION_BYTES)?;
    let right_digest = decoder.digest(EvidenceSection::Right)?;
    let right = decoder.section(EvidenceSection::Right, MAX_LOCAL_SECTION_BYTES)?;
    let interface_digest = decoder.digest(EvidenceSection::Interface)?;
    let interface = decoder.section(EvidenceSection::Interface, MAX_INTERFACE_SECTION_BYTES)?;
    let final_evidence = decoder.section(EvidenceSection::Final, MAX_FINAL_SECTION_BYTES)?;
    if decoder.offset != bytes.len() {
        return Err(reject(
            EvidenceSection::Envelope,
            "certificate has trailing bytes",
        ));
    }
    let certificate = RevisionLocalCertificate {
        left: LocalEvidence {
            source_sha256: left_digest,
            evidence: left,
        },
        right: LocalEvidence {
            source_sha256: right_digest,
            evidence: right,
        },
        interface: BoundEvidence {
            source_sha256: interface_digest,
            evidence: interface,
        },
        final_evidence,
    };
    if encode_revision_local_certificate(&certificate)? != bytes {
        return Err(reject(EvidenceSection::Envelope, "noncanonical encoding"));
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORD_COMPONENT: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 ulte 1 3 9\n11 constraint 10\n12 zero 1\n13 bad 12 never\n";
    const RIGHT_COMPONENT: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n";
    const FINAL_RIGHT_COMPONENT: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n";
    const FINAL_RIGHT_COMPONENT_V2: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 xor 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n";
    const WIDE_LEFT_COMPONENT: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 2\n3 sort bitvec 9\n4 input 2 command\n5 state 3 wide_state\n6 zero 3\n7 init 3 5 6\n8 uext 3 4 7\n9 next 3 5 8\n10 zero 1\n11 bad 10 never\n";

    fn fixture() -> RevisionLocalCertificate {
        RevisionLocalCertificate {
            left: LocalEvidence {
                source_sha256: source_digest(b"left-v1"),
                evidence: b"left-proof".to_vec(),
            },
            right: LocalEvidence {
                source_sha256: source_digest(b"right-v1"),
                evidence: b"right-proof".to_vec(),
            },
            interface: BoundEvidence {
                source_sha256: source_digest(b"word-wire"),
                evidence: b"interface-proof".to_vec(),
            },
            final_evidence: b"safe-proof".to_vec(),
        }
    }

    #[test]
    fn canonical_round_trip_and_source_binding() {
        let certificate = fixture();
        let encoded = encode_revision_local_certificate(&certificate).unwrap();
        assert_eq!(
            decode_revision_local_certificate(&encoded).unwrap(),
            certificate
        );
        verify_source_bindings(b"left-v1", b"right-v1", b"word-wire", &certificate).unwrap();
    }

    #[test]
    fn changed_right_preserves_left_bytes() {
        let previous = fixture();
        let mut next = fixture();
        next.right.source_sha256 = source_digest(b"right-v2");
        next.right.evidence = b"right-v2-proof".to_vec();
        next.final_evidence = b"unsafe-witness".to_vec();
        assert!(unchanged_local_evidence(&previous, &next, EvidenceSection::Left).unwrap());
        assert!(!unchanged_local_evidence(&previous, &next, EvidenceSection::Right).unwrap());
    }

    #[test]
    fn source_drift_is_attributed_to_smallest_section() {
        let certificate = fixture();
        let error = verify_source_bindings(b"left-v1", b"right-v2", b"word-wire", &certificate)
            .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Right);
    }

    #[test]
    fn every_truncation_and_trailing_byte_fail_closed() {
        let encoded = encode_revision_local_certificate(&fixture()).unwrap();
        for length in 0..encoded.len() {
            assert!(decode_revision_local_certificate(&encoded[..length]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        let error = decode_revision_local_certificate(&trailing).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Envelope);
    }

    #[test]
    fn section_length_attack_is_rejected_before_allocation() {
        let mut encoded = encode_revision_local_certificate(&fixture()).unwrap();
        let left_length_offset = MAGIC.len() + 4 + 32;
        encoded[left_length_offset..left_length_offset + 4]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        let error = decode_revision_local_certificate(&encoded).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
        assert_eq!(error.message, "section exceeds byte limit");
    }

    #[test]
    fn word_relation_is_complete_and_constraint_filtered() {
        let relation = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        assert_eq!(relation.state_widths, [2]);
        assert_eq!(relation.input_widths, [2]);
        assert_eq!(relation.output_widths, [2]);
        assert_eq!(relation.constraints, [11]);
        assert_eq!(relation.rows.len(), 12);
        let summary =
            verify_local_relation(WORD_COMPONENT, &relation, EvidenceSection::Left).unwrap();
        assert_eq!(summary.state_bits, 2);
        assert_eq!(summary.input_bits, 2);
        assert_eq!(summary.output_bits, 2);
        assert_eq!(summary.candidate_valuations, 16);
        assert_eq!(summary.admissible_rows, 12);
        let encoded = encode_local_relation_certificate(&relation).unwrap();
        assert_eq!(
            decode_local_relation_certificate(&encoded).unwrap(),
            relation
        );
    }

    #[test]
    fn omitted_and_mutated_rows_fail_with_local_attribution() {
        let relation = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        let mut omitted = relation.clone();
        omitted.rows.pop();
        let error =
            verify_local_relation(WORD_COMPONENT, &omitted, EvidenceSection::Right).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Right);
        assert_eq!(error.message, "local relation omits an admissible row");

        let mut changed = relation;
        changed.rows[0].output[0] ^= 1;
        let error =
            verify_local_relation(WORD_COMPONENT, &changed, EvidenceSection::Left).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
        assert_eq!(error.message, "local relation row does not match source");
    }

    #[test]
    fn state_and_input_widths_fail_closed_before_enumeration() {
        let source = b"1 sort bitvec 1\n2 sort bitvec 9\n3 input 1 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 next 2 4 4\n8 bad 3 input_bad\n";
        let error = produce_local_relation(source, &[4]).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Envelope);
        assert_eq!(error.message, "state width exceeds 8-bit limit");
    }

    #[test]
    fn local_relation_codec_rejects_truncation_trailing_and_count_attacks() {
        let relation = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        let encoded = encode_local_relation_certificate(&relation).unwrap();
        for length in 0..encoded.len() {
            assert!(decode_local_relation_certificate(&encoded[..length]).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_local_relation_certificate(&trailing).is_err());

        let mut hostile_count = encoded;
        let state_count_offset = LOCAL_RELATION_MAGIC.len() + 4 + 32;
        hostile_count[state_count_offset..state_count_offset + 4]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        let error = decode_local_relation_certificate(&hostile_count).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Envelope);
        assert_eq!(error.message, "local relation node count exceeds limit");
    }

    #[test]
    fn word_interface_is_canonical_and_composes_exact_rows() {
        let left = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        let right = produce_local_relation(RIGHT_COMPONENT, &[7]).unwrap();
        let contract = WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        };
        let encoded = encode_word_interface_contract(&contract).unwrap();
        assert_eq!(
            decode_word_interface_contract(encoded.as_bytes()).unwrap(),
            contract
        );
        let composed =
            compose_local_relations(WORD_COMPONENT, &left, RIGHT_COMPONENT, &right, &contract)
                .unwrap();
        assert_eq!(composed.pair_checks, 192);
        assert_eq!(composed.pairs.len(), 48);
        assert_eq!(
            composed.interface_sha256,
            evidence_digest(encoded.as_bytes())
        );
        for pair in composed.pairs {
            let left_row = &left.rows[pair.left_row as usize];
            let right_row = &right.rows[pair.right_row as usize];
            assert_eq!(left_row.output[0], right_row.input[0]);
        }
        let verified_left =
            verify_local_relation_for_composition(WORD_COMPONENT, &left, EvidenceSection::Left)
                .unwrap();
        let verified_right =
            verify_local_relation_for_composition(RIGHT_COMPONENT, &right, EvidenceSection::Right)
                .unwrap();
        assert_eq!(verified_left.summary().admissible_rows, 12);
        assert_eq!(
            compose_verified_local_relations(&verified_left, &verified_right, &contract)
                .unwrap()
                .pairs
                .len(),
            48
        );
    }

    #[test]
    fn interface_width_and_hidden_drive_mutations_fail_closed() {
        let left = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        let narrow_source = b"1 sort bitvec 1\n2 input 1 sensed\n3 state 1 state\n4 zero 1\n5 init 1 3 4\n6 next 1 3 2\n7 xor 1 3 2\n8 bad 4 never\n";
        let narrow = produce_local_relation(narrow_source, &[7]).unwrap();
        let contract = WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 2,
            }],
        };
        let error =
            compose_local_relations(WORD_COMPONENT, &left, narrow_source, &narrow, &contract)
                .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Interface);
        assert_eq!(error.message, "wire output and input widths differ");

        let duplicate = WordInterfaceContract {
            wires: vec![
                InterfaceWire {
                    from: ComponentSide::Left,
                    output: 7,
                    to_input: 3,
                },
                InterfaceWire {
                    from: ComponentSide::Left,
                    output: 7,
                    to_input: 3,
                },
            ],
        };
        assert!(encode_word_interface_contract(&duplicate).is_err());

        let right = produce_local_relation(RIGHT_COMPONENT, &[7]).unwrap();
        let mut false_left = left;
        false_left.rows[0].output[0] ^= 1;
        let valid_contract = WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        };
        let error = compose_local_relations(
            WORD_COMPONENT,
            &false_left,
            RIGHT_COMPONENT,
            &right,
            &valid_contract,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
        assert_eq!(error.message, "local relation row does not match source");
    }

    #[test]
    fn bounded_answer_preserves_safe_and_earliest_unsafe_results() {
        let left = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        let right = produce_local_relation(FINAL_RIGHT_COMPONENT, &[7, 10]).unwrap();
        let verified_left =
            verify_local_relation_for_composition(WORD_COMPONENT, &left, EvidenceSection::Left)
                .unwrap();
        let verified_right = verify_local_relation_for_composition(
            FINAL_RIGHT_COMPONENT,
            &right,
            EvidenceSection::Right,
        )
        .unwrap();
        let contract = WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        };
        let safe_query = BoundedQuery {
            horizon: 0,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let safe = produce_bounded_answer(&verified_left, &verified_right, &contract, &safe_query)
            .unwrap();
        assert_eq!(safe.result, BoundedResult::Safe);
        assert_eq!(safe.layers.len(), 1);
        assert_eq!(
            verify_bounded_answer(&verified_left, &verified_right, &contract, &safe)
                .unwrap()
                .bad_frame,
            None
        );

        let unsafe_query = BoundedQuery {
            horizon: 1,
            ..safe_query
        };
        let unsafe_certificate =
            produce_bounded_answer(&verified_left, &verified_right, &contract, &unsafe_query)
                .unwrap();
        assert_eq!(unsafe_certificate.result, BoundedResult::Unsafe);
        assert_eq!(unsafe_certificate.bad_frame, Some(1));
        assert_eq!(unsafe_certificate.witness_pairs.len(), 2);
        let summary = verify_bounded_answer(
            &verified_left,
            &verified_right,
            &contract,
            &unsafe_certificate,
        )
        .unwrap();
        assert_eq!(summary.bad_frame, Some(1));
        let encoded = encode_bounded_answer_certificate(&unsafe_certificate).unwrap();
        let decoded = decode_bounded_answer_certificate(&encoded).unwrap();
        assert_eq!(decoded, unsafe_certificate);
        assert_eq!(
            verify_bounded_answer(&verified_left, &verified_right, &contract, &decoded)
                .unwrap()
                .bad_frame,
            Some(1)
        );
        for length in 0..encoded.len() {
            assert!(decode_bounded_answer_certificate(&encoded[..length]).is_err());
        }
    }

    #[test]
    fn bounded_answer_tampering_is_attributed_and_rejected() {
        let left = produce_local_relation(WORD_COMPONENT, &[7]).unwrap();
        let right = produce_local_relation(FINAL_RIGHT_COMPONENT, &[7, 10]).unwrap();
        let verified_left =
            verify_local_relation_for_composition(WORD_COMPONENT, &left, EvidenceSection::Left)
                .unwrap();
        let verified_right = verify_local_relation_for_composition(
            FINAL_RIGHT_COMPONENT,
            &right,
            EvidenceSection::Right,
        )
        .unwrap();
        let contract = WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        };
        let query = BoundedQuery {
            horizon: 1,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let mut certificate =
            produce_bounded_answer(&verified_left, &verified_right, &contract, &query).unwrap();
        certificate.witness_pairs[0] ^= 1;
        let error = verify_bounded_answer(&verified_left, &verified_right, &contract, &certificate)
            .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Final);
        assert_eq!(error.message, "UNSAFE witness pair is not enabled");
    }

    #[test]
    fn complete_revision_local_envelope_preserves_both_answers() {
        let interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        })
        .unwrap();
        let mut produced = Vec::new();
        for (horizon, expected) in [(0, BoundedResult::Safe), (1, BoundedResult::Unsafe)] {
            let query = BoundedQuery {
                horizon,
                bad_side: ComponentSide::Right,
                bad_output: 10,
            };
            let (certificate, summary) = produce_revision_local_certificate(
                WORD_COMPONENT,
                &[7],
                FINAL_RIGHT_COMPONENT,
                &[7, 10],
                interface.as_bytes(),
                &query,
            )
            .unwrap();
            assert_eq!(summary.answer.result, expected);
            let bytes = encode_revision_local_certificate(&certificate).unwrap();
            let decoded = decode_revision_local_certificate(&bytes).unwrap();
            assert_eq!(
                verify_revision_local_certificate(
                    WORD_COMPONENT,
                    FINAL_RIGHT_COMPONENT,
                    interface.as_bytes(),
                    &decoded,
                )
                .unwrap()
                .answer
                .result,
                expected
            );
            produced.push(certificate);
        }
        assert_eq!(produced[0].left, produced[1].left);
        assert_eq!(produced[0].right, produced[1].right);
        assert_eq!(produced[0].interface, produced[1].interface);

        let mut hostile = produced.pop().unwrap();
        hostile.left.evidence[0] ^= 1;
        let error = verify_revision_local_certificate(
            WORD_COMPONENT,
            FINAL_RIGHT_COMPONENT,
            interface.as_bytes(),
            &hostile,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
    }

    #[test]
    fn changed_right_revision_reuses_validated_left_without_rechecking() {
        let interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        })
        .unwrap();
        let query = BoundedQuery {
            horizon: 1,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let (previous, _) = produce_revision_local_certificate(
            WORD_COMPONENT,
            &[7],
            FINAL_RIGHT_COMPONENT,
            &[7, 10],
            interface.as_bytes(),
            &query,
        )
        .unwrap();
        let retained_left = validate_local_artifact(
            WORD_COMPONENT,
            &previous.left.evidence,
            EvidenceSection::Left,
        )
        .unwrap();
        let (next, _) = produce_revision_local_certificate(
            WORD_COMPONENT,
            &[7],
            FINAL_RIGHT_COMPONENT_V2,
            &[7, 10],
            interface.as_bytes(),
            &query,
        )
        .unwrap();
        assert_eq!(previous.left, next.left);
        assert_ne!(previous.right, next.right);
        let (summary, work) = verify_revision_with_retained_left(
            &retained_left,
            FINAL_RIGHT_COMPONENT_V2,
            interface.as_bytes(),
            &next,
        )
        .unwrap();
        assert_eq!(summary.answer.result, BoundedResult::Unsafe);
        assert_eq!(work.decoded_local_sections, 1);
        assert_eq!(work.semantically_verified_local_sections, 1);
        assert_eq!(work.reused_local_sections, 1);
        assert!(work.composed_pair_checks > 0);
        assert!(work.final_transition_checks > 0);

        let mut stale = next;
        stale.left.evidence.push(0);
        let error = verify_revision_with_retained_left(
            &retained_left,
            FINAL_RIGHT_COMPONENT_V2,
            interface.as_bytes(),
            &stale,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
        assert_eq!(
            error.message,
            "retained left evidence is not byte-identical"
        );
    }

    #[test]
    fn direct_fallback_preserves_both_answers_beyond_local_state_limit() {
        assert!(produce_local_relation(WIDE_LEFT_COMPONENT, &[4]).is_err());
        let interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 4,
                to_input: 3,
            }],
        })
        .unwrap();
        for (horizon, expected, bad_frame) in [
            (0, BoundedResult::Safe, None),
            (1, BoundedResult::Unsafe, Some(1)),
        ] {
            let query = BoundedQuery {
                horizon,
                bad_side: ComponentSide::Right,
                bad_output: 10,
            };
            let (certificate, produced) = produce_direct_answer(
                WIDE_LEFT_COMPONENT,
                FINAL_RIGHT_COMPONENT,
                interface.as_bytes(),
                &query,
            )
            .unwrap();
            assert_eq!(produced.result, expected);
            assert_eq!(produced.bad_frame, bad_frame);
            let verified = verify_direct_answer(
                WIDE_LEFT_COMPONENT,
                FINAL_RIGHT_COMPONENT,
                interface.as_bytes(),
                &certificate,
            )
            .unwrap();
            assert_eq!(verified.result, expected);
            assert_eq!(verified.bad_frame, bad_frame);
            let encoded = encode_direct_answer_certificate(&certificate).unwrap();
            let decoded = decode_direct_answer_certificate(&encoded).unwrap();
            assert_eq!(decoded, certificate);
            assert_eq!(
                verify_direct_answer(
                    WIDE_LEFT_COMPONENT,
                    FINAL_RIGHT_COMPONENT,
                    interface.as_bytes(),
                    &decoded,
                )
                .unwrap()
                .result,
                expected
            );
            if horizon == 1 {
                for length in 0..encoded.len() {
                    assert!(decode_direct_answer_certificate(&encoded[..length]).is_err());
                }
            }
        }
    }

    #[test]
    fn direct_fallback_rejects_witness_and_source_drift() {
        let interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 4,
                to_input: 3,
            }],
        })
        .unwrap();
        let query = BoundedQuery {
            horizon: 1,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let (mut certificate, _) = produce_direct_answer(
            WIDE_LEFT_COMPONENT,
            FINAL_RIGHT_COMPONENT,
            interface.as_bytes(),
            &query,
        )
        .unwrap();
        certificate.witness_valuations[0] = u16::MAX;
        let error = verify_direct_answer(
            WIDE_LEFT_COMPONENT,
            FINAL_RIGHT_COMPONENT,
            interface.as_bytes(),
            &certificate,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Final);
        assert_eq!(error.message, "direct UNSAFE evidence shape is invalid");

        certificate.left_sha256[0] ^= 1;
        let error = verify_direct_answer(
            WIDE_LEFT_COMPONENT,
            FINAL_RIGHT_COMPONENT,
            interface.as_bytes(),
            &certificate,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
    }

    #[test]
    fn portfolio_routes_statically_and_preserves_both_backends() {
        let local_interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        })
        .unwrap();
        let local_query = BoundedQuery {
            horizon: 1,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let local = produce_revision_portfolio(
            WORD_COMPONENT,
            &[7],
            FINAL_RIGHT_COMPONENT,
            &[7, 10],
            local_interface.as_bytes(),
            &local_query,
        )
        .unwrap();
        assert_eq!(local.backend, RevisionPortfolioBackend::RevisionLocal);
        assert_eq!(
            local.reason,
            RevisionSelectionReason::ExactLocalRelationAdmitted
        );
        assert_eq!(
            verify_revision_portfolio(
                WORD_COMPONENT,
                &[7],
                FINAL_RIGHT_COMPONENT,
                &[7, 10],
                local_interface.as_bytes(),
                &local,
            )
            .unwrap()
            .result,
            BoundedResult::Unsafe
        );

        let wide_interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 4,
                to_input: 3,
            }],
        })
        .unwrap();
        let fallback = produce_revision_portfolio(
            WIDE_LEFT_COMPONENT,
            &[4],
            FINAL_RIGHT_COMPONENT,
            &[7, 10],
            wide_interface.as_bytes(),
            &local_query,
        )
        .unwrap();
        assert_eq!(fallback.backend, RevisionPortfolioBackend::DirectExact);
        assert_eq!(fallback.reason, RevisionSelectionReason::LeftStateWidth);
        let summary = verify_revision_portfolio(
            WIDE_LEFT_COMPONENT,
            &[4],
            FINAL_RIGHT_COMPONENT,
            &[7, 10],
            wide_interface.as_bytes(),
            &fallback,
        )
        .unwrap();
        assert_eq!(summary.result, BoundedResult::Unsafe);
        assert_eq!(summary.bad_frame, Some(1));
    }

    #[test]
    fn portfolio_rejects_forced_downgrade_and_invalid_wiring() {
        let interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 3,
            }],
        })
        .unwrap();
        let query = BoundedQuery {
            horizon: 1,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let (direct, _) = produce_direct_answer(
            WORD_COMPONENT,
            FINAL_RIGHT_COMPONENT,
            interface.as_bytes(),
            &query,
        )
        .unwrap();
        let forced = RevisionPortfolioProduction {
            certificate: RevisionPortfolioCertificate::DirectExact(direct),
            backend: RevisionPortfolioBackend::DirectExact,
            reason: RevisionSelectionReason::LeftStateWidth,
        };
        let error = verify_revision_portfolio(
            WORD_COMPONENT,
            &[7],
            FINAL_RIGHT_COMPONENT,
            &[7, 10],
            interface.as_bytes(),
            &forced,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Envelope);
        assert_eq!(
            error.message,
            "portfolio selection does not match static assessment"
        );

        let invalid_interface = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 7,
                to_input: 999,
            }],
        })
        .unwrap();
        let error = produce_revision_portfolio(
            WORD_COMPONENT,
            &[7],
            FINAL_RIGHT_COMPONENT,
            &[7, 10],
            invalid_interface.as_bytes(),
            &query,
        )
        .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Interface);
    }
}
