//! Exact finite-domain control-flow DAG for compiled-MMIO execution.

use crate::riscv32imc::{
    MAX_RV32_IMAGE_BYTES, RV32_IMAGE_BASE, Rv32Execution, Rv32ReplayMachine, Rv32SymbolLayout,
    decompress,
};
use riscv_decode::{Instruction, decode};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const BRANCHING_DAG_VERSION: u32 = 1;
pub const BRANCHING_DAG_INPUTS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BranchingControlStep {
    pub program_counter: u32,
    pub instruction_word: u32,
    pub instruction_bytes: u8,
    pub next_program_counter: u32,
    pub next: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchingTerminal {
    pub execution: Rv32Execution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioBranchingDag {
    pub version: u32,
    pub roots: Vec<u32>,
    pub terminal_indices: Vec<u16>,
    pub nodes: Vec<BranchingControlStep>,
    pub terminals: Vec<BranchingTerminal>,
    pub scalar_path_steps: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchingDagVerification {
    pub decoded_transitions: u64,
    pub scalar_path_steps: u64,
    pub inputs_checked: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchingDagError(pub String);

impl fmt::Display for BranchingDagError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO branching DAG: {}", self.0)
    }
}

impl Error for BranchingDagError {}

fn reject(message: impl Into<String>) -> BranchingDagError {
    BranchingDagError(message.into())
}

fn fetch_step(image: &[u8], pc: u32) -> Result<(u32, u8), BranchingDagError> {
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
) -> Result<u16, BranchingDagError> {
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

pub fn build_compiled_mmio_branching_dag(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioBranchingDag, BranchingDagError> {
    if image.is_empty() || image.len() > MAX_RV32_IMAGE_BYTES {
        return Err(reject("image size is outside policy"));
    }
    let mut roots = Vec::with_capacity(BRANCHING_DAG_INPUTS);
    let mut terminal_indices = Vec::with_capacity(BRANCHING_DAG_INPUTS);
    let mut nodes = Vec::new();
    let mut interned = BTreeMap::new();
    let mut terminals = Vec::new();
    let mut scalar_path_steps = 0u64;

    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let mut trace = Vec::new();
        while !machine.is_complete() {
            let program_counter = machine.program_counter();
            let (instruction_word, instruction_bytes) = fetch_step(image, program_counter)?;
            machine
                .step()
                .map_err(|error| reject(format!("input {input}: {error}")))?;
            trace.push((
                program_counter,
                instruction_word,
                instruction_bytes,
                machine.program_counter(),
            ));
        }
        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        scalar_path_steps = scalar_path_steps
            .checked_add(execution.steps)
            .ok_or_else(|| reject("scalar path step count overflow"))?;
        terminal_indices.push(terminal_index(&mut terminals, execution)?);

        let mut next = None;
        for (program_counter, instruction_word, instruction_bytes, next_program_counter) in
            trace.into_iter().rev()
        {
            let node = BranchingControlStep {
                program_counter,
                instruction_word,
                instruction_bytes,
                next_program_counter,
                next,
            };
            let index = if let Some(index) = interned.get(&node) {
                *index
            } else {
                let index =
                    u32::try_from(nodes.len()).map_err(|_| reject("node count exceeds u32"))?;
                nodes.push(node);
                interned.insert(node, index);
                index
            };
            next = Some(index);
        }
        roots.push(next.ok_or_else(|| reject(format!("input {input} has an empty path")))?);
    }

    Ok(CompiledMmioBranchingDag {
        version: BRANCHING_DAG_VERSION,
        roots,
        terminal_indices,
        nodes,
        terminals,
        scalar_path_steps,
    })
}

pub fn verify_compiled_mmio_branching_dag(
    dag: &CompiledMmioBranchingDag,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<BranchingDagVerification, BranchingDagError> {
    if dag.version != BRANCHING_DAG_VERSION
        || dag.roots.len() != BRANCHING_DAG_INPUTS
        || dag.terminal_indices.len() != BRANCHING_DAG_INPUTS
        || dag.nodes.is_empty()
        || dag.terminals.is_empty()
    {
        return Err(reject("DAG shape is not canonical"));
    }
    let image_end = RV32_IMAGE_BASE
        .checked_add(image.len() as u32)
        .ok_or_else(|| reject("image end overflow"))?;
    let mut decoded: Vec<Instruction> = Vec::with_capacity(dag.nodes.len());
    for (index, node) in dag.nodes.iter().enumerate() {
        if !matches!(node.instruction_bytes, 2 | 4)
            || node.next.is_some_and(|next| next as usize >= index)
        {
            return Err(reject(format!("node {index} violates canonical ordering")));
        }
        decoded.push(
            decode(node.instruction_word)
                .map_err(|error| reject(format!("node {index} does not decode: {error:?}")))?,
        );
    }

    let mut reachable = BTreeSet::new();
    let mut scalar_path_steps = 0u64;
    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let mut node_index = dag.roots[usize::from(input)];
        loop {
            let index = usize::try_from(node_index).map_err(|_| reject("node index overflow"))?;
            let node = dag
                .nodes
                .get(index)
                .ok_or_else(|| reject(format!("input {input} root or edge is outside DAG")))?;
            reachable.insert(node_index);
            machine
                .step_predecoded(
                    node.program_counter,
                    node.instruction_word,
                    node.instruction_bytes,
                    &decoded[index],
                    image_end,
                )
                .map_err(|error| reject(format!("input {input}, node {index}: {error}")))?;
            if machine.program_counter() != node.next_program_counter {
                return Err(reject(format!("input {input} diverges after node {index}")));
            }
            scalar_path_steps = scalar_path_steps
                .checked_add(1)
                .ok_or_else(|| reject("scalar path step count overflow"))?;
            match node.next {
                Some(next) => node_index = next,
                None => break,
            }
        }
        if !machine.is_complete() {
            return Err(reject(format!("input {input} path terminates early")));
        }
        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let terminal = dag
            .terminals
            .get(usize::from(dag.terminal_indices[usize::from(input)]))
            .ok_or_else(|| reject(format!("input {input} terminal is outside table")))?;
        if execution != terminal.execution {
            return Err(reject(format!("input {input} terminal mismatch")));
        }
    }
    if reachable.len() != dag.nodes.len() || scalar_path_steps != dag.scalar_path_steps {
        return Err(reject("DAG reachability or work count is inconsistent"));
    }
    Ok(BranchingDagVerification {
        decoded_transitions: dag.nodes.len() as u64,
        scalar_path_steps,
        inputs_checked: BRANCHING_DAG_INPUTS as u16,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parity_image() -> (Vec<u8>, Rv32SymbolLayout) {
        let mut image = vec![0; 0x110];
        let andi_a0_one = (1u32 << 20) | (10 << 15) | (7 << 12) | (10 << 7) | 0x13;
        let return_to_ra = (1u32 << 15) | 0x67;
        image[..4].copy_from_slice(&andi_a0_one.to_le_bytes());
        image[4..8].copy_from_slice(&return_to_ra.to_le_bytes());
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
    fn shares_equal_suffixes_and_replays_every_input() {
        let (image, symbols) = parity_image();
        let dag = build_compiled_mmio_branching_dag(&image, symbols).unwrap();
        assert_eq!(dag.scalar_path_steps, 512);
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.terminals.len(), 2);
        let verified = verify_compiled_mmio_branching_dag(&dag, &image, symbols).unwrap();
        assert_eq!(verified.decoded_transitions, 2);
        assert_eq!(verified.scalar_path_steps, 512);
        assert_eq!(verified.inputs_checked, 256);
    }

    #[test]
    fn rejects_edge_terminal_and_source_drift() {
        let (image, symbols) = parity_image();
        let original = build_compiled_mmio_branching_dag(&image, symbols).unwrap();
        let mut edge = original.clone();
        edge.roots[0] = u32::MAX;
        assert!(verify_compiled_mmio_branching_dag(&edge, &image, symbols).is_err());
        let mut terminal = original.clone();
        terminal.terminals[0].execution.return_value ^= 1;
        assert!(verify_compiled_mmio_branching_dag(&terminal, &image, symbols).is_err());
        let mut changed_image = image.clone();
        changed_image[0] ^= 1;
        assert!(verify_compiled_mmio_branching_dag(&original, &changed_image, symbols).is_err());
    }
}
