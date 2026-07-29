//! Exact finite-domain control-flow DAG for compiled-MMIO execution.

use crate::riscv32imc::{
    CompiledMmioEvent, MAX_RV32_IMAGE_BYTES, MAX_RV32_STEPS, RV32_IMAGE_BASE, Rv32Execution,
    Rv32ReplayMachine, Rv32SymbolLayout, decompress,
};
use riscv_decode::{Instruction, decode};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const BRANCHING_DAG_VERSION: u32 = 1;
pub const BRANCHING_DAG_INPUTS: usize = 256;
pub const MAX_BRANCHING_DAG_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_BRANCHING_DAG_NODES: usize = 1024 * 1024;
const MAGIC: &[u8; 8] = b"GCCBDG01";
const CHECKSUM_BYTES: usize = 32;
const MAX_TERMINALS: usize = BRANCHING_DAG_INPUTS;
const MAX_EVENTS: usize = 32;

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
    pub image_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TraceFamilyStep {
    pub program_counter: u32,
    pub instruction_word: u32,
    pub instruction_bytes: u8,
    pub next_program_counter: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioTraceFamily {
    pub version: u32,
    pub image_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
    pub input_trace_indices: Vec<u16>,
    pub terminal_indices: Vec<u16>,
    pub traces: Vec<Vec<TraceFamilyStep>>,
    pub terminals: Vec<BranchingTerminal>,
    pub scalar_path_steps: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceFamilyVerification {
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
        image_sha256: Sha256::digest(image).into(),
        symbols,
        roots,
        terminal_indices,
        nodes,
        terminals,
        scalar_path_steps,
    })
}

pub fn build_compiled_mmio_trace_family(
    dag: &CompiledMmioBranchingDag,
) -> Result<CompiledMmioTraceFamily, BranchingDagError> {
    let mut traces = Vec::new();
    let mut interned = BTreeMap::new();
    let mut input_trace_indices = Vec::with_capacity(BRANCHING_DAG_INPUTS);
    for (input, root) in dag.roots.iter().copied().enumerate() {
        let mut trace = Vec::new();
        let mut next = Some(root);
        while let Some(node_index) = next {
            let node = dag
                .nodes
                .get(node_index as usize)
                .ok_or_else(|| reject(format!("input {input} edge is outside DAG")))?;
            trace.push(TraceFamilyStep {
                program_counter: node.program_counter,
                instruction_word: node.instruction_word,
                instruction_bytes: node.instruction_bytes,
                next_program_counter: node.next_program_counter,
            });
            next = node.next;
        }
        let trace_index = if let Some(index) = interned.get(&trace) {
            *index
        } else {
            let index =
                u16::try_from(traces.len()).map_err(|_| reject("trace count exceeds u16"))?;
            traces.push(trace.clone());
            interned.insert(trace, index);
            index
        };
        input_trace_indices.push(trace_index);
    }
    Ok(CompiledMmioTraceFamily {
        version: BRANCHING_DAG_VERSION,
        image_sha256: dag.image_sha256,
        symbols: dag.symbols,
        input_trace_indices,
        terminal_indices: dag.terminal_indices.clone(),
        traces,
        terminals: dag.terminals.clone(),
        scalar_path_steps: dag.scalar_path_steps,
    })
}

pub fn verify_compiled_mmio_trace_family(
    family: &CompiledMmioTraceFamily,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<TraceFamilyVerification, BranchingDagError> {
    let image_sha256: [u8; 32] = Sha256::digest(image).into();
    if family.version != BRANCHING_DAG_VERSION
        || family.image_sha256 != image_sha256
        || family.symbols != symbols
        || family.input_trace_indices.len() != BRANCHING_DAG_INPUTS
        || family.terminal_indices.len() != BRANCHING_DAG_INPUTS
        || family.traces.is_empty()
        || family.terminals.is_empty()
    {
        return Err(reject("trace-family shape or identity is not canonical"));
    }
    let image_end = RV32_IMAGE_BASE
        .checked_add(image.len() as u32)
        .ok_or_else(|| reject("image end overflow"))?;
    let mut decoded_traces = Vec::with_capacity(family.traces.len());
    let mut decoded_transitions = 0u64;
    for (trace_index, trace) in family.traces.iter().enumerate() {
        if trace.is_empty() {
            return Err(reject(format!("trace {trace_index} is empty")));
        }
        let mut decoded = Vec::with_capacity(trace.len());
        for (step_index, step) in trace.iter().enumerate() {
            if !matches!(step.instruction_bytes, 2 | 4)
                || (step_index > 0
                    && trace[step_index - 1].next_program_counter != step.program_counter)
            {
                return Err(reject(format!(
                    "trace {trace_index} step {step_index} is not canonical"
                )));
            }
            decoded.push(decode(step.instruction_word).map_err(|error| {
                reject(format!(
                    "trace {trace_index} step {step_index} does not decode: {error:?}"
                ))
            })?);
        }
        decoded_transitions = decoded_transitions
            .checked_add(trace.len() as u64)
            .ok_or_else(|| reject("trace-family decode count overflow"))?;
        decoded_traces.push(decoded);
    }

    let mut canonical_traces = Vec::new();
    let mut canonical_interned = BTreeMap::new();
    let mut canonical_terminals = Vec::new();
    let mut scalar_path_steps = 0u64;
    for input in 0u8..=u8::MAX {
        let trace_index = usize::from(family.input_trace_indices[usize::from(input)]);
        let trace = family
            .traces
            .get(trace_index)
            .ok_or_else(|| reject(format!("input {input} trace is outside table")))?;
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        for (step, instruction) in trace.iter().zip(&decoded_traces[trace_index]) {
            machine
                .step_predecoded(
                    step.program_counter,
                    step.instruction_word,
                    step.instruction_bytes,
                    instruction,
                    image_end,
                )
                .map_err(|error| reject(format!("input {input}: {error}")))?;
            if machine.program_counter() != step.next_program_counter {
                return Err(reject(format!("input {input} diverges from trace")));
            }
            scalar_path_steps = scalar_path_steps
                .checked_add(1)
                .ok_or_else(|| reject("trace-family scalar work overflow"))?;
        }
        if !machine.is_complete() {
            return Err(reject(format!("input {input} trace terminates early")));
        }
        let execution = machine
            .finish()
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let claimed_terminal = family.terminal_indices[usize::from(input)];
        if family
            .terminals
            .get(usize::from(claimed_terminal))
            .is_none_or(|terminal| terminal.execution != execution)
        {
            return Err(reject(format!("input {input} terminal mismatch")));
        }
        let canonical_terminal = terminal_index(&mut canonical_terminals, execution)?;
        if claimed_terminal != canonical_terminal {
            return Err(reject(format!("input {input} terminal is not canonical")));
        }
        let canonical_trace = if let Some(index) = canonical_interned.get(trace) {
            *index
        } else {
            let index = u16::try_from(canonical_traces.len())
                .map_err(|_| reject("canonical trace count exceeds u16"))?;
            canonical_traces.push(trace.clone());
            canonical_interned.insert(trace.clone(), index);
            index
        };
        if family.input_trace_indices[usize::from(input)] != canonical_trace {
            return Err(reject(format!(
                "input {input} trace index is not canonical"
            )));
        }
    }
    if canonical_traces != family.traces
        || canonical_terminals != family.terminals
        || scalar_path_steps != family.scalar_path_steps
    {
        return Err(reject("trace-family canonical reconstruction differs"));
    }
    Ok(TraceFamilyVerification {
        decoded_transitions,
        scalar_path_steps,
        inputs_checked: BRANCHING_DAG_INPUTS as u16,
    })
}

pub fn projected_compiled_mmio_trace_family_size(
    family: &CompiledMmioTraceFamily,
) -> Result<usize, BranchingDagError> {
    let trace_steps = family
        .traces
        .iter()
        .try_fold(0usize, |total, trace| total.checked_add(trace.len()))
        .ok_or_else(|| reject("trace step count overflow"))?;
    let terminal_bytes = family
        .terminals
        .iter()
        .try_fold(0usize, |total, terminal| {
            terminal
                .execution
                .events
                .len()
                .checked_mul(16)
                .and_then(|events| events.checked_add(16))
                .and_then(|member| total.checked_add(member))
        })
        .ok_or_else(|| reject("terminal byte count overflow"))?;
    8usize
        .checked_add(4 + 32 + 12 + 8 + 8)
        .and_then(|bytes| bytes.checked_add(BRANCHING_DAG_INPUTS * 4))
        .and_then(|bytes| bytes.checked_add(family.traces.len() * 4))
        .and_then(|bytes| bytes.checked_add(trace_steps * 13))
        .and_then(|bytes| bytes.checked_add(terminal_bytes))
        .and_then(|bytes| bytes.checked_add(CHECKSUM_BYTES))
        .ok_or_else(|| reject("trace-family encoded size overflow"))
}

pub fn verify_compiled_mmio_branching_dag(
    dag: &CompiledMmioBranchingDag,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<BranchingDagVerification, BranchingDagError> {
    let image_sha256: [u8; 32] = Sha256::digest(image).into();
    if dag.version != BRANCHING_DAG_VERSION
        || dag.image_sha256 != image_sha256
        || dag.symbols != symbols
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
    let mut canonical_nodes = Vec::new();
    let mut canonical_interned = BTreeMap::new();
    let mut canonical_terminals = Vec::new();
    let mut scalar_path_steps = 0u64;
    for input in 0u8..=u8::MAX {
        let mut machine = Rv32ReplayMachine::new_with_a0(image, symbols, u32::from(input))
            .map_err(|error| reject(format!("input {input}: {error}")))?;
        let mut node_index = dag.roots[usize::from(input)];
        let mut path = Vec::new();
        loop {
            let index = usize::try_from(node_index).map_err(|_| reject("node index overflow"))?;
            let node = dag
                .nodes
                .get(index)
                .ok_or_else(|| reject(format!("input {input} root or edge is outside DAG")))?;
            reachable.insert(node_index);
            path.push(*node);
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
        let claimed_terminal_index = dag.terminal_indices[usize::from(input)];
        let terminal = dag
            .terminals
            .get(usize::from(claimed_terminal_index))
            .ok_or_else(|| reject(format!("input {input} terminal is outside table")))?;
        if execution != terminal.execution {
            return Err(reject(format!("input {input} terminal mismatch")));
        }
        let canonical_terminal_index = terminal_index(&mut canonical_terminals, execution.clone())?;
        if claimed_terminal_index != canonical_terminal_index {
            return Err(reject(format!(
                "input {input} terminal index is not canonical"
            )));
        }

        let mut canonical_next = None;
        for observed in path.into_iter().rev() {
            let canonical = BranchingControlStep {
                next: canonical_next,
                ..observed
            };
            let canonical_index = if let Some(index) = canonical_interned.get(&canonical) {
                *index
            } else {
                let index = u32::try_from(canonical_nodes.len())
                    .map_err(|_| reject("canonical node count exceeds u32"))?;
                canonical_nodes.push(canonical);
                canonical_interned.insert(canonical, index);
                index
            };
            canonical_next = Some(canonical_index);
        }
        if canonical_next != Some(dag.roots[usize::from(input)]) {
            return Err(reject(format!("input {input} root is not canonical")));
        }
    }
    if reachable.len() != dag.nodes.len()
        || canonical_nodes != dag.nodes
        || canonical_terminals != dag.terminals
        || scalar_path_steps != dag.scalar_path_steps
    {
        return Err(reject("DAG reachability or work count is inconsistent"));
    }
    Ok(BranchingDagVerification {
        decoded_transitions: dag.nodes.len() as u64,
        scalar_path_steps,
        inputs_checked: BRANCHING_DAG_INPUTS as u16,
    })
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

pub fn encode_compiled_mmio_branching_dag(
    dag: &CompiledMmioBranchingDag,
) -> Result<Vec<u8>, BranchingDagError> {
    if dag.version != BRANCHING_DAG_VERSION
        || dag.roots.len() != BRANCHING_DAG_INPUTS
        || dag.terminal_indices.len() != BRANCHING_DAG_INPUTS
        || dag.nodes.is_empty()
        || dag.nodes.len() > MAX_BRANCHING_DAG_NODES
        || dag.terminals.is_empty()
        || dag.terminals.len() > MAX_TERMINALS
    {
        return Err(reject("DAG fields are outside encoding policy"));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_u32(&mut bytes, dag.version);
    bytes.extend_from_slice(&dag.image_sha256);
    push_u32(&mut bytes, dag.symbols.entry);
    push_u32(&mut bytes, dag.symbols.event_count);
    push_u32(&mut bytes, dag.symbols.events);
    push_u64(&mut bytes, dag.scalar_path_steps);
    push_u32(&mut bytes, dag.nodes.len() as u32);
    push_u32(&mut bytes, dag.terminals.len() as u32);
    for root in &dag.roots {
        push_u32(&mut bytes, *root);
    }
    for terminal in &dag.terminal_indices {
        push_u16(&mut bytes, *terminal);
    }
    for node in &dag.nodes {
        push_u32(&mut bytes, node.program_counter);
        push_u32(&mut bytes, node.instruction_word);
        bytes.push(node.instruction_bytes);
        push_u32(&mut bytes, node.next_program_counter);
        push_u32(&mut bytes, node.next.unwrap_or(u32::MAX));
    }
    for terminal in &dag.terminals {
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
    if bytes.len() > MAX_BRANCHING_DAG_BYTES {
        return Err(reject("encoded DAG exceeds byte policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], BranchingDagError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("DAG offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("DAG artifact is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, BranchingDagError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, BranchingDagError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| reject("invalid u16 field"))?,
        ))
    }

    fn u32(&mut self) -> Result<u32, BranchingDagError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| reject("invalid u32 field"))?,
        ))
    }

    fn u64(&mut self) -> Result<u64, BranchingDagError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| reject("invalid u64 field"))?,
        ))
    }
}

pub fn decode_compiled_mmio_branching_dag(
    bytes: &[u8],
) -> Result<CompiledMmioBranchingDag, BranchingDagError> {
    if bytes.len() < 1600 || bytes.len() > MAX_BRANCHING_DAG_BYTES {
        return Err(reject("DAG artifact size is outside policy"));
    }
    let content_len = bytes
        .len()
        .checked_sub(CHECKSUM_BYTES)
        .ok_or_else(|| reject("DAG artifact is truncated"))?;
    let checksum: [u8; 32] = Sha256::digest(&bytes[..content_len]).into();
    if checksum != bytes[content_len..] {
        return Err(reject("DAG artifact checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..content_len],
        offset: 0,
    };
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(reject("DAG artifact magic mismatch"));
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
        || node_count > MAX_BRANCHING_DAG_NODES
        || terminal_count == 0
        || terminal_count > MAX_TERMINALS
    {
        return Err(reject("DAG counts are outside policy"));
    }
    let minimum = node_count
        .checked_mul(17)
        .and_then(|nodes| nodes.checked_add(terminal_count * 16))
        .ok_or_else(|| reject("DAG minimum size overflow"))?;
    if minimum > cursor.bytes.len().saturating_sub(cursor.offset) {
        return Err(reject("DAG counts exceed remaining bytes"));
    }
    let mut roots = Vec::with_capacity(BRANCHING_DAG_INPUTS);
    for _ in 0..BRANCHING_DAG_INPUTS {
        roots.push(cursor.u32()?);
    }
    let mut terminal_indices = Vec::with_capacity(BRANCHING_DAG_INPUTS);
    for _ in 0..BRANCHING_DAG_INPUTS {
        terminal_indices.push(cursor.u16()?);
    }
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let program_counter = cursor.u32()?;
        let instruction_word = cursor.u32()?;
        let instruction_bytes = cursor.u8()?;
        let next_program_counter = cursor.u32()?;
        let encoded_next = cursor.u32()?;
        nodes.push(BranchingControlStep {
            program_counter,
            instruction_word,
            instruction_bytes,
            next_program_counter,
            next: (encoded_next != u32::MAX).then_some(encoded_next),
        });
    }
    let mut terminals = Vec::with_capacity(terminal_count);
    for _ in 0..terminal_count {
        let return_value = cursor.u32()?;
        let steps = cursor.u64()?;
        let event_count =
            usize::try_from(cursor.u32()?).map_err(|_| reject("event count exceeds platform"))?;
        if steps > MAX_RV32_STEPS || event_count > MAX_EVENTS {
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
        return Err(reject("DAG artifact has trailing content"));
    }
    let dag = CompiledMmioBranchingDag {
        version,
        image_sha256,
        symbols,
        roots,
        terminal_indices,
        nodes,
        terminals,
        scalar_path_steps,
    };
    if encode_compiled_mmio_branching_dag(&dag)? != bytes {
        return Err(reject("DAG artifact encoding is not canonical"));
    }
    Ok(dag)
}

pub fn verify_compiled_mmio_branching_dag_bytes(
    bytes: &[u8],
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<BranchingDagVerification, BranchingDagError> {
    let dag = decode_compiled_mmio_branching_dag(bytes)?;
    verify_compiled_mmio_branching_dag(&dag, image, symbols)
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
        let bytes = encode_compiled_mmio_branching_dag(&dag).unwrap();
        assert_eq!(decode_compiled_mmio_branching_dag(&bytes).unwrap(), dag);
        assert_eq!(
            verify_compiled_mmio_branching_dag_bytes(&bytes, &image, symbols).unwrap(),
            verified
        );
        let family = build_compiled_mmio_trace_family(&dag).unwrap();
        let family_verified = verify_compiled_mmio_trace_family(&family, &image, symbols).unwrap();
        assert_eq!(family.traces.len(), 1);
        assert_eq!(family_verified.decoded_transitions, 2);
        assert!(projected_compiled_mmio_trace_family_size(&family).unwrap() > 0);
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
        let bytes = encode_compiled_mmio_branching_dag(&original).unwrap();
        let mut changed = bytes.clone();
        let middle = changed.len() / 2;
        changed[middle] ^= 1;
        assert!(decode_compiled_mmio_branching_dag(&changed).is_err());
        assert!(decode_compiled_mmio_branching_dag(&bytes[..bytes.len() - 1]).is_err());
    }
}
