//! Source-bound multi-successor decode graph for compiled-MMIO execution.

use crate::{
    compiled_mmio_branching_dag::BranchingTerminal,
    riscv32imc::{
        CompiledMmioEvent, MAX_RV32_IMAGE_BYTES, MAX_RV32_STEPS, RV32_IMAGE_BASE, Rv32Execution,
        Rv32ReplayMachine, Rv32SymbolLayout, decompress,
    },
};
use riscv_decode::{Instruction, decode};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const DECODE_GRAPH_VERSION: u32 = 2;
pub const DECODE_GRAPH_INPUTS: usize = 256;
pub const MAX_DECODE_GRAPH_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DECODE_GRAPH_NODES: usize = 1024 * 1024;
pub const MAX_DECODE_GRAPH_EDGES: usize = 4 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"GCCMDG02";
const CHECKSUM_BYTES: usize = 32;
const MAX_TERMINALS: usize = DECODE_GRAPH_INPUTS;
const MAX_EVENTS: usize = 32;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DecodeGraphNode {
    pub program_counter: u32,
    pub instruction_word: u32,
    pub instruction_bytes: u8,
    pub next_program_counters: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioDecodeGraph {
    pub version: u32,
    pub image_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
    pub terminal_indices: Vec<u16>,
    pub nodes: Vec<DecodeGraphNode>,
    pub terminals: Vec<BranchingTerminal>,
    pub scalar_path_steps: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeGraphVerification {
    pub unique_instruction_decodes: u64,
    pub graph_edges: u64,
    pub scalar_path_steps: u64,
    pub inputs_checked: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodeGraphError(pub String);

impl fmt::Display for DecodeGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO decode graph: {}", self.0)
    }
}

impl Error for DecodeGraphError {}

fn reject(message: impl Into<String>) -> DecodeGraphError {
    DecodeGraphError(message.into())
}

fn fetch_step(image: &[u8], pc: u32) -> Result<(u32, u8), DecodeGraphError> {
    let offset = usize::try_from(
        pc.checked_sub(RV32_IMAGE_BASE)
            .ok_or_else(|| reject("program counter is below image"))?,
    )
    .map_err(|_| reject("program-counter offset exceeds platform range"))?;
    let low = u16::from_le_bytes(
        image
            .get(offset..offset + 2)
            .ok_or_else(|| reject("instruction fetch is outside image"))?
            .try_into()
            .map_err(|_| reject("invalid instruction prefix"))?,
    );
    if low & 3 == 3 {
        let word = u32::from_le_bytes(
            image
                .get(offset..offset + 4)
                .ok_or_else(|| reject("instruction fetch is outside image"))?
                .try_into()
                .map_err(|_| reject("invalid instruction word"))?,
        );
        Ok((word, 4))
    } else {
        Ok((
            decompress(low).map_err(|error| reject(error.to_string()))?,
            2,
        ))
    }
}

fn terminal_index(
    terminals: &mut Vec<BranchingTerminal>,
    execution: Rv32Execution,
) -> Result<u16, DecodeGraphError> {
    if let Some(index) = terminals
        .iter()
        .position(|terminal| terminal.execution == execution)
    {
        return u16::try_from(index).map_err(|_| reject("terminal index exceeds u16"));
    }
    let index = u16::try_from(terminals.len()).map_err(|_| reject("terminal count exceeds u16"))?;
    terminals.push(BranchingTerminal { execution });
    Ok(index)
}

type NodeKey = (u32, u32, u8);

pub fn build_compiled_mmio_decode_graph(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioDecodeGraph, DecodeGraphError> {
    if image.is_empty() || image.len() > MAX_RV32_IMAGE_BYTES {
        return Err(reject("image size is outside policy"));
    }

    let mut observed: BTreeMap<NodeKey, BTreeSet<u32>> = BTreeMap::new();
    let mut terminal_indices = Vec::with_capacity(DECODE_GRAPH_INPUTS);
    let mut terminals = Vec::new();
    let mut scalar_path_steps = 0u64;

    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        while !machine.is_complete() {
            let program_counter = machine.program_counter();
            let (instruction_word, instruction_bytes) = fetch_step(image, program_counter)?;
            machine
                .step()
                .map_err(|error| reject(format!("input {input}: {error}")))?;
            observed
                .entry((program_counter, instruction_word, instruction_bytes))
                .or_default()
                .insert(machine.program_counter());
            scalar_path_steps = scalar_path_steps
                .checked_add(1)
                .ok_or_else(|| reject("scalar path step count overflow"))?;
        }
        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        terminal_indices.push(terminal_index(&mut terminals, execution)?);
    }

    let nodes = observed
        .into_iter()
        .map(
            |((program_counter, instruction_word, instruction_bytes), next_program_counters)| {
                DecodeGraphNode {
                    program_counter,
                    instruction_word,
                    instruction_bytes,
                    next_program_counters: next_program_counters.into_iter().collect(),
                }
            },
        )
        .collect();

    Ok(CompiledMmioDecodeGraph {
        version: DECODE_GRAPH_VERSION,
        image_sha256: Sha256::digest(image).into(),
        symbols,
        terminal_indices,
        nodes,
        terminals,
        scalar_path_steps,
    })
}

pub fn verify_compiled_mmio_decode_graph_btree_baseline(
    graph: &CompiledMmioDecodeGraph,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<DecodeGraphVerification, DecodeGraphError> {
    let image_sha256: [u8; 32] = Sha256::digest(image).into();
    if graph.version != DECODE_GRAPH_VERSION
        || graph.image_sha256 != image_sha256
        || graph.symbols != symbols
        || graph.terminal_indices.len() != DECODE_GRAPH_INPUTS
        || graph.nodes.is_empty()
        || graph.terminals.is_empty()
    {
        return Err(reject("graph shape or identity is not canonical"));
    }

    let image_end = RV32_IMAGE_BASE
        .checked_add(image.len() as u32)
        .ok_or_else(|| reject("image end overflow"))?;
    let mut decoded = Vec::<Instruction>::with_capacity(graph.nodes.len());
    let mut node_by_pc = BTreeMap::new();
    let mut previous_key = None;
    let mut graph_edges = 0u64;

    for (index, node) in graph.nodes.iter().enumerate() {
        let key = (
            node.program_counter,
            node.instruction_word,
            node.instruction_bytes,
        );
        if !matches!(node.instruction_bytes, 2 | 4)
            || node.next_program_counters.is_empty()
            || previous_key.is_some_and(|previous| previous >= key)
            || node
                .next_program_counters
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || node_by_pc.insert(node.program_counter, index).is_some()
        {
            return Err(reject(format!("node {index} is not canonical")));
        }
        let fetched = fetch_step(image, node.program_counter)?;
        if fetched != (node.instruction_word, node.instruction_bytes) {
            return Err(reject(format!("node {index} differs from source image")));
        }
        decoded.push(
            decode(node.instruction_word)
                .map_err(|error| reject(format!("node {index} does not decode: {error:?}")))?,
        );
        graph_edges = graph_edges
            .checked_add(node.next_program_counters.len() as u64)
            .ok_or_else(|| reject("graph edge count overflow"))?;
        previous_key = Some(key);
    }

    let mut observed: BTreeMap<NodeKey, BTreeSet<u32>> = BTreeMap::new();
    let mut canonical_terminals = Vec::new();
    let mut scalar_path_steps = 0u64;

    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        while !machine.is_complete() {
            let program_counter = machine.program_counter();
            let index = *node_by_pc.get(&program_counter).ok_or_else(|| {
                reject(format!("input {input} has no node at {program_counter:#x}"))
            })?;
            let node = &graph.nodes[index];
            machine
                .step_predecoded(
                    node.program_counter,
                    node.instruction_word,
                    node.instruction_bytes,
                    &decoded[index],
                    image_end,
                )
                .map_err(|error| reject(format!("input {input}, node {index}: {error}")))?;
            let next_program_counter = machine.program_counter();
            if node
                .next_program_counters
                .binary_search(&next_program_counter)
                .is_err()
            {
                return Err(reject(format!(
                    "input {input}, node {index} takes an undeclared edge"
                )));
            }
            observed
                .entry((
                    node.program_counter,
                    node.instruction_word,
                    node.instruction_bytes,
                ))
                .or_default()
                .insert(next_program_counter);
            scalar_path_steps = scalar_path_steps
                .checked_add(1)
                .ok_or_else(|| reject("scalar path step count overflow"))?;
        }

        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let claimed_terminal = graph.terminal_indices[usize::from(input)];
        if graph
            .terminals
            .get(usize::from(claimed_terminal))
            .is_none_or(|terminal| terminal.execution != execution)
        {
            return Err(reject(format!("input {input} terminal mismatch")));
        }
        let canonical_terminal = terminal_index(&mut canonical_terminals, execution)?;
        if claimed_terminal != canonical_terminal {
            return Err(reject(format!(
                "input {input} terminal index is not canonical"
            )));
        }
    }

    let canonical_nodes: Vec<_> = observed
        .into_iter()
        .map(
            |((program_counter, instruction_word, instruction_bytes), next_program_counters)| {
                DecodeGraphNode {
                    program_counter,
                    instruction_word,
                    instruction_bytes,
                    next_program_counters: next_program_counters.into_iter().collect(),
                }
            },
        )
        .collect();
    if canonical_nodes != graph.nodes
        || canonical_terminals != graph.terminals
        || scalar_path_steps != graph.scalar_path_steps
    {
        return Err(reject(
            "graph coverage, terminal table or scalar work is inconsistent",
        ));
    }

    Ok(DecodeGraphVerification {
        unique_instruction_decodes: graph.nodes.len() as u64,
        graph_edges,
        scalar_path_steps,
        inputs_checked: DECODE_GRAPH_INPUTS as u16,
    })
}

fn verify_compiled_mmio_decode_graph_strategy<const CARRY_SUCCESSOR_INDEX: bool>(
    graph: &CompiledMmioDecodeGraph,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<DecodeGraphVerification, DecodeGraphError> {
    let image_sha256: [u8; 32] = Sha256::digest(image).into();
    if graph.version != DECODE_GRAPH_VERSION
        || graph.image_sha256 != image_sha256
        || graph.symbols != symbols
        || graph.terminal_indices.len() != DECODE_GRAPH_INPUTS
        || graph.nodes.is_empty()
        || graph.nodes.len() > MAX_DECODE_GRAPH_NODES
        || graph.terminals.is_empty()
        || graph.terminals.len() > MAX_TERMINALS
        || image.is_empty()
        || image.len() > MAX_RV32_IMAGE_BYTES
    {
        return Err(reject("graph shape or identity is not canonical"));
    }

    let image_end = RV32_IMAGE_BASE
        .checked_add(image.len() as u32)
        .ok_or_else(|| reject("image end overflow"))?;
    let slot_count = image
        .len()
        .checked_add(1)
        .map(|bytes| bytes / 2)
        .ok_or_else(|| reject("dense source index size overflow"))?;
    let mut node_by_halfword = vec![u32::MAX; slot_count];
    let mut decoded = Vec::<Instruction>::with_capacity(graph.nodes.len());
    let mut edge_offsets = Vec::with_capacity(graph.nodes.len() + 1);
    edge_offsets.push(0usize);
    let mut previous_key = None;

    for (index, node) in graph.nodes.iter().enumerate() {
        let key = (
            node.program_counter,
            node.instruction_word,
            node.instruction_bytes,
        );
        let source_offset = node
            .program_counter
            .checked_sub(RV32_IMAGE_BASE)
            .ok_or_else(|| reject(format!("node {index} is below the source image")))?;
        if source_offset % 2 != 0 {
            return Err(reject(format!("node {index} is not halfword aligned")));
        }
        let slot = usize::try_from(source_offset / 2)
            .map_err(|_| reject(format!("node {index} source offset exceeds platform")))?;
        let encoded_index =
            u32::try_from(index).map_err(|_| reject("node index exceeds u32 policy"))?;
        if !matches!(node.instruction_bytes, 2 | 4)
            || node.next_program_counters.is_empty()
            || previous_key.is_some_and(|previous| previous >= key)
            || node
                .next_program_counters
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || node_by_halfword
                .get_mut(slot)
                .is_none_or(|entry| std::mem::replace(entry, encoded_index) != u32::MAX)
        {
            return Err(reject(format!("node {index} is not canonical")));
        }
        let fetched = fetch_step(image, node.program_counter)?;
        if fetched != (node.instruction_word, node.instruction_bytes) {
            return Err(reject(format!("node {index} differs from source image")));
        }
        decoded.push(
            decode(node.instruction_word)
                .map_err(|error| reject(format!("node {index} does not decode: {error:?}")))?,
        );
        let next_offset = edge_offsets[index]
            .checked_add(node.next_program_counters.len())
            .ok_or_else(|| reject("graph edge count overflow"))?;
        if next_offset > MAX_DECODE_GRAPH_EDGES {
            return Err(reject("graph edge count exceeds policy"));
        }
        edge_offsets.push(next_offset);
        previous_key = Some(key);
    }

    let graph_edges = *edge_offsets
        .last()
        .ok_or_else(|| reject("graph edge offsets are empty"))?;
    let successor_indices: Vec<Vec<u32>> = if CARRY_SUCCESSOR_INDEX {
        graph
            .nodes
            .iter()
            .map(|node| {
                node.next_program_counters
                    .iter()
                    .map(|program_counter| {
                        program_counter
                            .checked_sub(RV32_IMAGE_BASE)
                            .filter(|offset| offset % 2 == 0)
                            .and_then(|offset| usize::try_from(offset / 2).ok())
                            .and_then(|slot| node_by_halfword.get(slot).copied())
                            .unwrap_or(u32::MAX)
                    })
                    .collect()
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut covered_edges = vec![false; graph_edges];
    let mut canonical_terminals = Vec::new();
    let mut scalar_path_steps = 0u64;

    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let mut carried_index = u32::MAX;
        while !machine.is_complete() {
            let program_counter = machine.program_counter();
            let encoded_index = if CARRY_SUCCESSOR_INDEX && carried_index != u32::MAX {
                carried_index
            } else {
                let source_offset =
                    program_counter
                        .checked_sub(RV32_IMAGE_BASE)
                        .ok_or_else(|| {
                            reject(format!("input {input} program counter is below image"))
                        })?;
                if source_offset % 2 != 0 {
                    return Err(reject(format!(
                        "input {input} program counter is not halfword aligned"
                    )));
                }
                let slot = usize::try_from(source_offset / 2)
                    .map_err(|_| reject(format!("input {input} source offset exceeds platform")))?;
                *node_by_halfword.get(slot).ok_or_else(|| {
                    reject(format!("input {input} program counter is outside image"))
                })?
            };
            let index = usize::try_from(encoded_index)
                .ok()
                .filter(|_| encoded_index != u32::MAX)
                .ok_or_else(|| {
                    reject(format!("input {input} has no node at {program_counter:#x}"))
                })?;
            let node = &graph.nodes[index];
            machine
                .step_predecoded(
                    node.program_counter,
                    node.instruction_word,
                    node.instruction_bytes,
                    &decoded[index],
                    image_end,
                )
                .map_err(|error| reject(format!("input {input}, node {index}: {error}")))?;
            let next_program_counter = machine.program_counter();
            let edge_index = node
                .next_program_counters
                .binary_search(&next_program_counter)
                .map_err(|_| {
                    reject(format!(
                        "input {input}, node {index} takes an undeclared edge"
                    ))
                })?;
            covered_edges[edge_offsets[index] + edge_index] = true;
            if CARRY_SUCCESSOR_INDEX {
                carried_index = successor_indices[index][edge_index];
                if machine.is_complete() {
                    if carried_index != u32::MAX {
                        return Err(reject(format!(
                            "input {input}, node {index} completes on a nonterminal edge"
                        )));
                    }
                } else if carried_index == u32::MAX {
                    return Err(reject(format!(
                        "input {input}, node {index} takes an unresolved nonterminal edge"
                    )));
                }
            }
            scalar_path_steps = scalar_path_steps
                .checked_add(1)
                .ok_or_else(|| reject("scalar path step count overflow"))?;
        }

        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let claimed_terminal = graph.terminal_indices[usize::from(input)];
        if graph
            .terminals
            .get(usize::from(claimed_terminal))
            .is_none_or(|terminal| terminal.execution != execution)
        {
            return Err(reject(format!("input {input} terminal mismatch")));
        }
        let canonical_terminal = terminal_index(&mut canonical_terminals, execution)?;
        if claimed_terminal != canonical_terminal {
            return Err(reject(format!(
                "input {input} terminal index is not canonical"
            )));
        }
    }

    if covered_edges.iter().any(|covered| !covered)
        || canonical_terminals != graph.terminals
        || scalar_path_steps != graph.scalar_path_steps
    {
        return Err(reject(
            "graph coverage, terminal table or scalar work is inconsistent",
        ));
    }

    Ok(DecodeGraphVerification {
        unique_instruction_decodes: graph.nodes.len() as u64,
        graph_edges: graph_edges as u64,
        scalar_path_steps,
        inputs_checked: DECODE_GRAPH_INPUTS as u16,
    })
}

pub fn verify_compiled_mmio_decode_graph(
    graph: &CompiledMmioDecodeGraph,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<DecodeGraphVerification, DecodeGraphError> {
    verify_compiled_mmio_decode_graph_strategy::<false>(graph, image, symbols)
}

pub fn verify_compiled_mmio_decode_graph_successor_index(
    graph: &CompiledMmioDecodeGraph,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<DecodeGraphVerification, DecodeGraphError> {
    verify_compiled_mmio_decode_graph_strategy::<true>(graph, image, symbols)
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

pub fn encode_compiled_mmio_decode_graph(
    graph: &CompiledMmioDecodeGraph,
) -> Result<Vec<u8>, DecodeGraphError> {
    if graph.version != DECODE_GRAPH_VERSION
        || graph.terminal_indices.len() != DECODE_GRAPH_INPUTS
        || graph.nodes.is_empty()
        || graph.nodes.len() > MAX_DECODE_GRAPH_NODES
        || graph.terminals.is_empty()
        || graph.terminals.len() > MAX_TERMINALS
    {
        return Err(reject("graph fields are outside encoding policy"));
    }

    let mut total_edges = 0usize;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_u32(&mut bytes, graph.version);
    bytes.extend_from_slice(&graph.image_sha256);
    push_u32(&mut bytes, graph.symbols.entry);
    push_u32(&mut bytes, graph.symbols.event_count);
    push_u32(&mut bytes, graph.symbols.events);
    push_u64(&mut bytes, graph.scalar_path_steps);
    push_u32(&mut bytes, graph.nodes.len() as u32);
    push_u32(&mut bytes, graph.terminals.len() as u32);
    for terminal in &graph.terminal_indices {
        push_u16(&mut bytes, *terminal);
    }
    for node in &graph.nodes {
        total_edges = total_edges
            .checked_add(node.next_program_counters.len())
            .ok_or_else(|| reject("graph edge count overflow"))?;
        if node.next_program_counters.is_empty()
            || total_edges > MAX_DECODE_GRAPH_EDGES
            || node.next_program_counters.len() > u32::MAX as usize
        {
            return Err(reject("graph edges exceed encoding policy"));
        }
        push_u32(&mut bytes, node.program_counter);
        push_u32(&mut bytes, node.instruction_word);
        bytes.push(node.instruction_bytes);
        push_u32(&mut bytes, node.next_program_counters.len() as u32);
        for next in &node.next_program_counters {
            push_u32(&mut bytes, *next);
        }
    }
    for terminal in &graph.terminals {
        let execution = &terminal.execution;
        if execution.steps > MAX_RV32_STEPS
            || execution.events.len() > MAX_EVENTS
            || execution.events.len() != execution.event_program_locations.len()
        {
            return Err(reject("terminal execution exceeds encoding policy"));
        }
        push_u32(&mut bytes, execution.return_value);
        push_u64(&mut bytes, execution.steps);
        push_u32(&mut bytes, execution.events.len() as u32);
        for event in &execution.events {
            push_u32(&mut bytes, event.operation);
            push_u32(&mut bytes, event.offset);
            push_u32(&mut bytes, event.value);
        }
        for location in &execution.event_program_locations {
            push_u32(&mut bytes, *location);
        }
    }
    bytes.extend_from_slice(&Sha256::digest(&bytes));
    if bytes.len() > MAX_DECODE_GRAPH_BYTES {
        return Err(reject("encoded graph exceeds byte policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], DecodeGraphError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("graph offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("graph artifact is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, DecodeGraphError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeGraphError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| reject("invalid u16 field"))?,
        ))
    }

    fn u32(&mut self) -> Result<u32, DecodeGraphError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| reject("invalid u32 field"))?,
        ))
    }

    fn u64(&mut self) -> Result<u64, DecodeGraphError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| reject("invalid u64 field"))?,
        ))
    }
}

pub fn decode_compiled_mmio_decode_graph(
    bytes: &[u8],
) -> Result<CompiledMmioDecodeGraph, DecodeGraphError> {
    if bytes.len() < 640 || bytes.len() > MAX_DECODE_GRAPH_BYTES {
        return Err(reject("graph artifact size is outside policy"));
    }
    let content_len = bytes
        .len()
        .checked_sub(CHECKSUM_BYTES)
        .ok_or_else(|| reject("graph artifact is truncated"))?;
    let checksum: [u8; 32] = Sha256::digest(&bytes[..content_len]).into();
    if checksum != bytes[content_len..] {
        return Err(reject("graph artifact checksum mismatch"));
    }

    let mut cursor = Cursor {
        bytes: &bytes[..content_len],
        offset: 0,
    };
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(reject("graph artifact magic mismatch"));
    }
    let version = cursor.u32()?;
    let image_sha256 = cursor
        .take(32)?
        .try_into()
        .map_err(|_| reject("invalid image digest"))?;
    let symbols = Rv32SymbolLayout {
        entry: cursor.u32()?,
        event_count: cursor.u32()?,
        events: cursor.u32()?,
    };
    let scalar_path_steps = cursor.u64()?;
    let node_count =
        usize::try_from(cursor.u32()?).map_err(|_| reject("node count exceeds platform"))?;
    let terminal_count =
        usize::try_from(cursor.u32()?).map_err(|_| reject("terminal count exceeds platform"))?;
    if node_count == 0
        || node_count > MAX_DECODE_GRAPH_NODES
        || terminal_count == 0
        || terminal_count > MAX_TERMINALS
    {
        return Err(reject("graph counts are outside policy"));
    }
    let minimum = DECODE_GRAPH_INPUTS
        .checked_mul(2)
        .and_then(|fixed| {
            node_count
                .checked_mul(17)
                .and_then(|nodes| fixed.checked_add(nodes))
        })
        .and_then(|bytes| {
            terminal_count
                .checked_mul(16)
                .and_then(|terminals| bytes.checked_add(terminals))
        })
        .ok_or_else(|| reject("graph minimum size overflow"))?;
    if minimum > cursor.bytes.len().saturating_sub(cursor.offset) {
        return Err(reject("graph counts exceed remaining bytes"));
    }

    let mut terminal_indices = Vec::with_capacity(DECODE_GRAPH_INPUTS);
    for _ in 0..DECODE_GRAPH_INPUTS {
        terminal_indices.push(cursor.u16()?);
    }
    let mut nodes = Vec::with_capacity(node_count);
    let mut total_edges = 0usize;
    for _ in 0..node_count {
        let program_counter = cursor.u32()?;
        let instruction_word = cursor.u32()?;
        let instruction_bytes = cursor.u8()?;
        let edge_count =
            usize::try_from(cursor.u32()?).map_err(|_| reject("edge count exceeds platform"))?;
        total_edges = total_edges
            .checked_add(edge_count)
            .ok_or_else(|| reject("graph edge count overflow"))?;
        if edge_count == 0
            || total_edges > MAX_DECODE_GRAPH_EDGES
            || edge_count > cursor.bytes.len().saturating_sub(cursor.offset) / 4
        {
            return Err(reject("graph edge count exceeds policy"));
        }
        let mut next_program_counters = Vec::with_capacity(edge_count);
        for _ in 0..edge_count {
            next_program_counters.push(cursor.u32()?);
        }
        nodes.push(DecodeGraphNode {
            program_counter,
            instruction_word,
            instruction_bytes,
            next_program_counters,
        });
    }

    let mut terminals = Vec::with_capacity(terminal_count);
    for _ in 0..terminal_count {
        let return_value = cursor.u32()?;
        let steps = cursor.u64()?;
        let event_count =
            usize::try_from(cursor.u32()?).map_err(|_| reject("event count exceeds platform"))?;
        if steps > MAX_RV32_STEPS
            || event_count > MAX_EVENTS
            || event_count > cursor.bytes.len().saturating_sub(cursor.offset) / 16
        {
            return Err(reject("terminal fields exceed policy"));
        }
        let mut events = Vec::with_capacity(event_count);
        for _ in 0..event_count {
            events.push(CompiledMmioEvent {
                operation: cursor.u32()?,
                offset: cursor.u32()?,
                value: cursor.u32()?,
            });
        }
        let mut event_program_locations = Vec::with_capacity(event_count);
        for _ in 0..event_count {
            event_program_locations.push(cursor.u32()?);
        }
        terminals.push(BranchingTerminal {
            execution: Rv32Execution {
                return_value,
                steps,
                events,
                event_program_locations,
            },
        });
    }
    if cursor.offset != cursor.bytes.len() {
        return Err(reject("graph artifact has trailing content"));
    }

    let graph = CompiledMmioDecodeGraph {
        version,
        image_sha256,
        symbols,
        terminal_indices,
        nodes,
        terminals,
        scalar_path_steps,
    };
    if encode_compiled_mmio_decode_graph(&graph)? != bytes {
        return Err(reject("graph artifact encoding is not canonical"));
    }
    Ok(graph)
}

pub fn verify_compiled_mmio_decode_graph_bytes(
    bytes: &[u8],
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<DecodeGraphVerification, DecodeGraphError> {
    let graph = decode_compiled_mmio_decode_graph(bytes)?;
    verify_compiled_mmio_decode_graph(&graph, image, symbols)
}

pub fn verify_compiled_mmio_decode_graph_bytes_btree_baseline(
    bytes: &[u8],
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<DecodeGraphVerification, DecodeGraphError> {
    let graph = decode_compiled_mmio_decode_graph(bytes)?;
    verify_compiled_mmio_decode_graph_btree_baseline(&graph, image, symbols)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branching_image() -> (Vec<u8>, Rv32SymbolLayout) {
        let mut image = vec![0; 0x110];
        let words = [
            (1u32 << 20) | (10 << 15) | (7 << 12) | (11 << 7) | 0x13,
            (11 << 15) | (6 << 8) | 0x63,
            0x0010_0513,
            0x0080_006f,
            0x0000_0513,
            0x0000_8067,
        ];
        for (index, word) in words.into_iter().enumerate() {
            image[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        (
            image,
            Rv32SymbolLayout {
                entry: RV32_IMAGE_BASE,
                event_count: RV32_IMAGE_BASE + 0x100,
                events: RV32_IMAGE_BASE + 0x104,
            },
        )
    }

    #[test]
    fn shares_decodes_while_scalar_state_selects_edges() {
        let (image, symbols) = branching_image();
        let graph = build_compiled_mmio_decode_graph(&image, symbols).unwrap();
        assert_eq!(graph.nodes.len(), 6);
        assert_eq!(graph.scalar_path_steps, 1_152);
        assert_eq!(graph.terminals.len(), 2);
        let branch = graph
            .nodes
            .iter()
            .find(|node| node.program_counter == RV32_IMAGE_BASE + 4)
            .unwrap();
        assert_eq!(
            branch.next_program_counters,
            vec![RV32_IMAGE_BASE + 8, RV32_IMAGE_BASE + 16]
        );

        let verified = verify_compiled_mmio_decode_graph(&graph, &image, symbols).unwrap();
        let successor =
            verify_compiled_mmio_decode_graph_successor_index(&graph, &image, symbols).unwrap();
        let baseline =
            verify_compiled_mmio_decode_graph_btree_baseline(&graph, &image, symbols).unwrap();
        assert_eq!(verified, baseline);
        assert_eq!(verified, successor);
        assert_eq!(verified.unique_instruction_decodes, 6);
        assert_eq!(verified.graph_edges, 7);
        assert_eq!(verified.scalar_path_steps, 1_152);
        assert_eq!(verified.inputs_checked, 256);
        let bytes = encode_compiled_mmio_decode_graph(&graph).unwrap();
        assert_eq!(decode_compiled_mmio_decode_graph(&bytes).unwrap(), graph);
        assert_eq!(
            verify_compiled_mmio_decode_graph_bytes(&bytes, &image, symbols).unwrap(),
            verified
        );
        assert_eq!(
            verify_compiled_mmio_decode_graph_bytes_btree_baseline(&bytes, &image, symbols)
                .unwrap(),
            verified
        );
    }

    #[test]
    fn rejects_missing_additional_terminal_and_source_evidence() {
        let (image, symbols) = branching_image();
        let original = build_compiled_mmio_decode_graph(&image, symbols).unwrap();
        let branch_index = original
            .nodes
            .iter()
            .position(|node| node.program_counter == RV32_IMAGE_BASE + 4)
            .unwrap();

        let mut missing_edge = original.clone();
        missing_edge.nodes[branch_index].next_program_counters.pop();
        assert!(verify_compiled_mmio_decode_graph(&missing_edge, &image, symbols).is_err());
        assert!(
            verify_compiled_mmio_decode_graph_successor_index(&missing_edge, &image, symbols)
                .is_err()
        );

        let mut additional_edge = original.clone();
        additional_edge.nodes[branch_index]
            .next_program_counters
            .push(RV32_IMAGE_BASE + 24);
        assert!(verify_compiled_mmio_decode_graph(&additional_edge, &image, symbols).is_err());
        assert!(
            verify_compiled_mmio_decode_graph_successor_index(&additional_edge, &image, symbols)
                .is_err()
        );

        let mut terminal = original.clone();
        terminal.terminals[0].execution.return_value ^= 1;
        assert!(verify_compiled_mmio_decode_graph(&terminal, &image, symbols).is_err());
        assert!(
            verify_compiled_mmio_decode_graph_successor_index(&terminal, &image, symbols).is_err()
        );

        let mut changed_image = image.clone();
        changed_image[0] ^= 1;
        assert!(verify_compiled_mmio_decode_graph(&original, &changed_image, symbols).is_err());
        assert!(
            verify_compiled_mmio_decode_graph_successor_index(&original, &changed_image, symbols)
                .is_err()
        );

        let bytes = encode_compiled_mmio_decode_graph(&original).unwrap();
        let mut mutation = bytes.clone();
        let middle = mutation.len() / 2;
        mutation[middle] ^= 1;
        assert!(decode_compiled_mmio_decode_graph(&mutation).is_err());
        assert!(decode_compiled_mmio_decode_graph(&bytes[..bytes.len() - 1]).is_err());
    }
}
