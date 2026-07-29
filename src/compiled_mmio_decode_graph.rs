//! Source-bound multi-successor decode graph for compiled-MMIO execution.

use crate::{
    compiled_mmio_branching_dag::BranchingTerminal,
    riscv32imc::{
        MAX_RV32_IMAGE_BYTES, RV32_IMAGE_BASE, Rv32Execution, Rv32ReplayMachine, Rv32SymbolLayout,
        decompress,
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

pub fn verify_compiled_mmio_decode_graph(
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
        assert_eq!(verified.unique_instruction_decodes, 6);
        assert_eq!(verified.graph_edges, 7);
        assert_eq!(verified.scalar_path_steps, 1_152);
        assert_eq!(verified.inputs_checked, 256);
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

        let mut additional_edge = original.clone();
        additional_edge.nodes[branch_index]
            .next_program_counters
            .push(RV32_IMAGE_BASE + 24);
        assert!(verify_compiled_mmio_decode_graph(&additional_edge, &image, symbols).is_err());

        let mut terminal = original.clone();
        terminal.terminals[0].execution.return_value ^= 1;
        assert!(verify_compiled_mmio_decode_graph(&terminal, &image, symbols).is_err());

        let mut changed_image = image.clone();
        changed_image[0] ^= 1;
        assert!(verify_compiled_mmio_decode_graph(&original, &changed_image, symbols).is_err());
    }
}
