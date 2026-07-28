//! Exact finite-domain RV32IMC execution for one canonical input predicate.
//!
//! This is an experimental symbolic transducer, not a general emulator. One
//! state carries every value in the fixed `a0 >= 6` eight-bit domain. Control
//! and addresses must remain uniform across all lanes or execution refuses.

use crate::riscv32imc::{
    CompiledMmioEvent, MAX_RV32_IMAGE_BYTES, MAX_RV32_MEMORY_BYTES, MAX_RV32_STEPS,
    RV32_IMAGE_BASE, Rv32Error, Rv32SymbolLayout, decompress, sign_extend,
};
use riscv_decode::{
    Instruction, decode,
    types::{BType, IType, RType, SType, ShiftType},
};
use std::collections::BTreeMap;

pub const INVALID_PREDICATE_FIRST: u8 = 6;
pub const INVALID_PREDICATE_LANES: usize = 250;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateTransducerExecution {
    pub first_input: u8,
    pub lane_count: u16,
    pub return_value: u32,
    pub events: Vec<CompiledMmioEvent>,
    pub event_program_locations: Vec<u32>,
    pub symbolic_transitions: u64,
    pub lane_value_operations: u64,
    pub sparse_memory_bytes: u32,
    pub control_trace: Vec<PredicateControlStep>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredicateControlStep {
    pub program_counter: u32,
    pub instruction_word: u32,
    pub instruction_bytes: u8,
    pub next_program_counter: u32,
}

#[derive(Clone, Debug)]
struct LaneWord {
    values: Vec<u32>,
    known: Vec<bool>,
}

impl LaneWord {
    fn unknown() -> Self {
        Self {
            values: vec![0; INVALID_PREDICATE_LANES],
            known: vec![false; INVALID_PREDICATE_LANES],
        }
    }

    fn constant(value: u32) -> Self {
        Self {
            values: vec![value; INVALID_PREDICATE_LANES],
            known: vec![true; INVALID_PREDICATE_LANES],
        }
    }

    fn input_domain() -> Self {
        Self {
            values: (INVALID_PREDICATE_FIRST..=u8::MAX).map(u32::from).collect(),
            known: vec![true; INVALID_PREDICATE_LANES],
        }
    }

    fn map_unary(&self, operation: impl Fn(u32) -> u32) -> Self {
        Self {
            values: self.values.iter().copied().map(operation).collect(),
            known: self.known.clone(),
        }
    }

    fn map_binary(&self, right: &Self, operation: impl Fn(u32, u32) -> u32) -> Self {
        Self {
            values: self
                .values
                .iter()
                .copied()
                .zip(right.values.iter().copied())
                .map(|(left, right)| operation(left, right))
                .collect(),
            known: self
                .known
                .iter()
                .copied()
                .zip(right.known.iter().copied())
                .map(|(left, right)| left && right)
                .collect(),
        }
    }

    fn uniform_known(&self, role: &str) -> Result<u32, Rv32Error> {
        if !self.known.iter().all(|known| *known) {
            return Err(reject(format!("{role} contains an unknown lane")));
        }
        let first = self.values[0];
        if self.values.iter().any(|value| *value != first) {
            return Err(reject(format!("{role} is not uniform across predicate")));
        }
        Ok(first)
    }
}

#[derive(Clone, Debug)]
struct LaneByte {
    values: Vec<u8>,
    known: Vec<bool>,
}

struct PredicateMachine {
    base_memory: Vec<u8>,
    sparse_memory: BTreeMap<u32, LaneByte>,
    registers: Vec<LaneWord>,
    pc: u32,
    stop: u32,
    steps: u64,
    lane_value_operations: u64,
    previous_pc: u32,
    image_end: u32,
    event_count_address: u32,
    event_program_locations: Vec<u32>,
    control_trace: Vec<PredicateControlStep>,
}

fn reject(message: impl Into<String>) -> Rv32Error {
    Rv32Error(format!(
        "finite-domain predicate transducer: {}",
        message.into()
    ))
}

impl PredicateMachine {
    fn index(&self, address: u32, width: usize) -> Result<usize, Rv32Error> {
        let offset = address
            .checked_sub(RV32_IMAGE_BASE)
            .ok_or_else(|| reject(format!("address below image: 0x{address:08x}")))?;
        let index = usize::try_from(offset).map_err(|_| reject("address conversion overflow"))?;
        if index
            .checked_add(width)
            .is_none_or(|end| end > self.base_memory.len())
        {
            return Err(reject(format!(
                "memory access outside policy: 0x{address:08x}"
            )));
        }
        Ok(index)
    }

    fn load_byte(&self, address: u32) -> Result<LaneByte, Rv32Error> {
        if let Some(value) = self.sparse_memory.get(&address) {
            return Ok(value.clone());
        }
        let index = self.index(address, 1)?;
        Ok(LaneByte {
            values: vec![self.base_memory[index]; INVALID_PREDICATE_LANES],
            known: vec![true; INVALID_PREDICATE_LANES],
        })
    }

    fn load(&self, address: u32, width: usize) -> Result<LaneWord, Rv32Error> {
        self.index(address, width)?;
        let mut result = LaneWord::constant(0);
        for byte in 0..width {
            let loaded = self.load_byte(address + byte as u32)?;
            for lane in 0..INVALID_PREDICATE_LANES {
                result.values[lane] |= (loaded.values[lane] as u32) << (byte * 8);
                result.known[lane] &= loaded.known[lane];
            }
        }
        Ok(result)
    }

    fn store(&mut self, address: u32, width: usize, value: &LaneWord) -> Result<(), Rv32Error> {
        self.index(address, width)?;
        if address < self.image_end
            && address
                .checked_add(width as u32)
                .is_some_and(|end| end > RV32_IMAGE_BASE)
        {
            return Err(reject("self-modifying symbolic code is unsupported"));
        }
        for byte in 0..width {
            self.sparse_memory.insert(
                address + byte as u32,
                LaneByte {
                    values: value
                        .values
                        .iter()
                        .map(|value| (value >> (byte * 8)) as u8)
                        .collect(),
                    known: value.known.clone(),
                },
            );
        }
        Ok(())
    }

    fn reg(&self, index: u32) -> LaneWord {
        self.registers[index as usize].clone()
    }

    fn set_reg(&mut self, index: u32, value: LaneWord) {
        if index != 0 {
            self.registers[index as usize] = value;
        }
    }

    fn require_uniform_address(&self, value: &LaneWord, role: &str) -> Result<u32, Rv32Error> {
        value.uniform_known(role)
    }

    fn fetch(&self) -> Result<(Instruction, u32, u32), Rv32Error> {
        let low = self.load(self.pc, 2)?.uniform_known("instruction fetch")? as u16;
        let length = if low & 3 == 3 { 4 } else { 2 };
        let word = if length == 2 {
            decompress(low)?
        } else {
            self.load(self.pc, 4)?.uniform_known("instruction fetch")?
        };
        let instruction = decode(word)
            .map_err(|error| reject(format!("decode at 0x{:08x}: {error:?}", self.pc)))?;
        Ok((instruction, length, word))
    }

    fn step(&mut self) -> Result<(), Rv32Error> {
        if self.steps >= MAX_RV32_STEPS {
            return Err(reject("symbolic transition bound exceeded"));
        }
        let (instruction, length, word) = self.fetch()?;
        let current = self.pc;
        self.previous_pc = current;
        self.pc = self.pc.wrapping_add(length);
        self.steps += 1;
        self.lane_value_operations = self
            .lane_value_operations
            .checked_add(INVALID_PREDICATE_LANES as u64)
            .ok_or_else(|| reject("lane operation count overflow"))?;
        self.execute(current, instruction)?;
        self.control_trace.push(PredicateControlStep {
            program_counter: current,
            instruction_word: word,
            instruction_bytes: length as u8,
            next_program_counter: self.pc,
        });
        Ok(())
    }

    fn immediate(value: u32) -> LaneWord {
        LaneWord::constant(value)
    }

    fn execute(&mut self, current: u32, instruction: Instruction) -> Result<(), Rv32Error> {
        use Instruction::*;
        match instruction {
            Lui(value) => self.set_reg(value.rd(), Self::immediate(value.imm())),
            Auipc(value) => self.set_reg(
                value.rd(),
                Self::immediate(current.wrapping_add(value.imm())),
            ),
            Jal(value) => {
                self.set_reg(value.rd(), Self::immediate(self.pc));
                self.pc = current.wrapping_add(sign_extend(value.imm(), 21));
            }
            Jalr(value) => {
                let base = self.reg(value.rs1());
                let target =
                    base.map_unary(|base| base.wrapping_add(sign_extend(value.imm(), 12)) & !1);
                let target = self.require_uniform_address(&target, "indirect target")?;
                if target == 0 {
                    return Err(reject("zero indirect target"));
                }
                self.set_reg(value.rd(), Self::immediate(self.pc));
                self.pc = target;
            }
            Beq(value) => self.branch(current, value, |left, right| left == right)?,
            Bne(value) => self.branch(current, value, |left, right| left != right)?,
            Blt(value) => {
                self.branch(current, value, |left, right| (left as i32) < (right as i32))?
            }
            Bge(value) => self.branch(current, value, |left, right| {
                (left as i32) >= (right as i32)
            })?,
            Bltu(value) => self.branch(current, value, |left, right| left < right)?,
            Bgeu(value) => self.branch(current, value, |left, right| left >= right)?,
            Lb(value) => self.load_i(value, 1, true)?,
            Lh(value) => self.load_i(value, 2, true)?,
            Lw(value) => self.load_i(value, 4, false)?,
            Lbu(value) => self.load_i(value, 1, false)?,
            Lhu(value) => self.load_i(value, 2, false)?,
            Sb(value) => self.store_s(current, value, 1)?,
            Sh(value) => self.store_s(current, value, 2)?,
            Sw(value) => self.store_s(current, value, 4)?,
            Addi(value) => self.op_i(value, |left, right| left.wrapping_add(right)),
            Slti(value) => self.op_i(value, |left, right| ((left as i32) < (right as i32)) as u32),
            Sltiu(value) => self.op_i(value, |left, right| (left < right) as u32),
            Xori(value) => self.op_i(value, |left, right| left ^ right),
            Ori(value) => self.op_i(value, |left, right| left | right),
            Andi(value) => self.op_i(value, |left, right| left & right),
            Slli(value) => self.shift_i(value, |left, right| left << right),
            Srli(value) => self.shift_i(value, |left, right| left >> right),
            Srai(value) => self.shift_i(value, |left, right| ((left as i32) >> right) as u32),
            Add(value) => self.op_r(value, |left, right| left.wrapping_add(right)),
            Sub(value) => self.op_r(value, |left, right| left.wrapping_sub(right)),
            Sll(value) => self.op_r(value, |left, right| left << (right & 31)),
            Slt(value) => self.op_r(value, |left, right| ((left as i32) < (right as i32)) as u32),
            Sltu(value) => self.op_r(value, |left, right| (left < right) as u32),
            Xor(value) => self.op_r(value, |left, right| left ^ right),
            Srl(value) => self.op_r(value, |left, right| left >> (right & 31)),
            Sra(value) => self.op_r(value, |left, right| ((left as i32) >> (right & 31)) as u32),
            Or(value) => self.op_r(value, |left, right| left | right),
            And(value) => self.op_r(value, |left, right| left & right),
            Mul(value) => self.op_r(value, |left, right| left.wrapping_mul(right)),
            Fence(_) | FenceI => {}
            Ebreak if current == self.stop => {}
            unsupported => {
                return Err(reject(format!(
                    "unsupported instruction at 0x{current:08x}: {unsupported:?}"
                )));
            }
        }
        Ok(())
    }

    fn branch(
        &mut self,
        current: u32,
        value: BType,
        predicate: impl Fn(u32, u32) -> bool,
    ) -> Result<(), Rv32Error> {
        let left = self.reg(value.rs1());
        let right = self.reg(value.rs2());
        if !left.known.iter().all(|known| *known) || !right.known.iter().all(|known| *known) {
            return Err(reject("branch operand contains an unknown lane"));
        }
        let directions: Vec<_> = left
            .values
            .iter()
            .copied()
            .zip(right.values.iter().copied())
            .map(|(left, right)| predicate(left, right))
            .collect();
        if directions
            .iter()
            .any(|direction| *direction != directions[0])
        {
            return Err(reject("branch direction diverges across predicate"));
        }
        if directions[0] {
            self.pc = current.wrapping_add(sign_extend(value.imm(), 13));
        }
        Ok(())
    }

    fn load_i(&mut self, value: IType, width: usize, signed: bool) -> Result<(), Rv32Error> {
        let base = self.reg(value.rs1());
        let addresses = base.map_unary(|base| base.wrapping_add(sign_extend(value.imm(), 12)));
        let address = self.require_uniform_address(&addresses, "load address")?;
        let mut loaded = self.load(address, width)?;
        if signed && width < 4 {
            loaded = loaded.map_unary(|value| sign_extend(value, (width * 8) as u32));
        }
        self.set_reg(value.rd(), loaded);
        Ok(())
    }

    fn store_s(&mut self, current: u32, value: SType, width: usize) -> Result<(), Rv32Error> {
        let base = self.reg(value.rs1());
        let addresses = base.map_unary(|base| base.wrapping_add(sign_extend(value.imm(), 12)));
        let address = self.require_uniform_address(&addresses, "store address")?;
        let stored = self.reg(value.rs2());
        self.store(address, width, &stored)?;
        if width == 4 && address == self.event_count_address {
            self.event_program_locations.push(current);
        }
        Ok(())
    }

    fn op_i(&mut self, value: IType, operation: impl Fn(u32, u32) -> u32) {
        let left = self.reg(value.rs1());
        let immediate = LaneWord::constant(sign_extend(value.imm(), 12));
        self.set_reg(value.rd(), left.map_binary(&immediate, operation));
    }

    fn shift_i(&mut self, value: ShiftType, operation: impl Fn(u32, u32) -> u32) {
        let left = self.reg(value.rs1());
        self.set_reg(
            value.rd(),
            left.map_unary(|left| operation(left, value.shamt() & 31)),
        );
    }

    fn op_r(&mut self, value: RType, operation: impl Fn(u32, u32) -> u32) {
        let left = self.reg(value.rs1());
        let right = self.reg(value.rs2());
        self.set_reg(value.rd(), left.map_binary(&right, operation));
    }
}

pub fn execute_invalid_channel_predicate(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<PredicateTransducerExecution, Rv32Error> {
    if image.is_empty() || image.len() > MAX_RV32_IMAGE_BYTES {
        return Err(reject("image size is outside policy"));
    }
    let memory_end = RV32_IMAGE_BASE
        .checked_add(MAX_RV32_MEMORY_BYTES as u32)
        .ok_or_else(|| reject("memory range overflow"))?;
    for address in [symbols.entry, symbols.event_count, symbols.events] {
        if !(RV32_IMAGE_BASE..memory_end).contains(&address) {
            return Err(reject("symbol is outside memory policy"));
        }
    }
    let stop = RV32_IMAGE_BASE
        .checked_add(MAX_RV32_MEMORY_BYTES as u32 - 4)
        .ok_or_else(|| reject("stop address overflow"))?;
    let image_end = RV32_IMAGE_BASE
        .checked_add(image.len() as u32)
        .ok_or_else(|| reject("image end overflow"))?;
    let mut machine = PredicateMachine {
        base_memory: vec![0; MAX_RV32_MEMORY_BYTES],
        sparse_memory: BTreeMap::new(),
        registers: vec![LaneWord::unknown(); 32],
        pc: symbols.entry,
        stop,
        steps: 0,
        lane_value_operations: 0,
        previous_pc: symbols.entry,
        image_end,
        event_count_address: symbols.event_count,
        event_program_locations: Vec::new(),
        control_trace: Vec::new(),
    };
    machine.base_memory[..image.len()].copy_from_slice(image);
    machine.registers[0] = LaneWord::constant(0);
    machine.registers[1] = LaneWord::constant(stop);
    machine.registers[2] = LaneWord::constant(stop & !0xf);
    machine.registers[10] = LaneWord::input_domain();
    machine.store(stop, 4, &LaneWord::constant(0x0010_0073))?;

    while machine.pc != stop {
        machine.step()?;
    }
    let return_value = machine.reg(10).uniform_known("return value")?;
    let event_count = machine
        .load(symbols.event_count, 4)?
        .uniform_known("event count")? as usize;
    if event_count > 32 || machine.event_program_locations.len() != event_count {
        return Err(reject("event count is outside policy or inconsistent"));
    }
    let mut events = Vec::with_capacity(event_count);
    for index in 0..event_count {
        let address = symbols
            .events
            .checked_add((index * 12) as u32)
            .ok_or_else(|| reject("event address overflow"))?;
        events.push(CompiledMmioEvent {
            operation: machine.load(address, 4)?.uniform_known("event operation")?,
            offset: machine
                .load(address + 4, 4)?
                .uniform_known("event offset")?,
            value: machine.load(address + 8, 4)?.uniform_known("event value")?,
        });
    }
    Ok(PredicateTransducerExecution {
        first_input: INVALID_PREDICATE_FIRST,
        lane_count: INVALID_PREDICATE_LANES as u16,
        return_value,
        events,
        event_program_locations: machine.event_program_locations,
        symbolic_transitions: machine.steps,
        lane_value_operations: machine.lane_value_operations,
        sparse_memory_bytes: machine.sparse_memory.len() as u32,
        control_trace: machine.control_trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guarded_image() -> (Vec<u8>, Rv32SymbolLayout) {
        let mut image = vec![0; 0x110];
        let sltiu_a0_six = (6u32 << 20) | (10 << 15) | (3 << 12) | (10 << 7) | 0x13;
        let return_to_ra = (1u32 << 15) | 0x67;
        image[..4].copy_from_slice(&sltiu_a0_six.to_le_bytes());
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
    fn exact_predicate_executes_one_uniform_invalid_path() {
        let (image, symbols) = guarded_image();
        let execution = execute_invalid_channel_predicate(&image, symbols).unwrap();
        assert_eq!(execution.return_value, 0);
        assert_eq!(execution.lane_count, 250);
        assert_eq!(execution.symbolic_transitions, 2);
        assert_eq!(execution.lane_value_operations, 500);
    }

    #[test]
    fn divergent_invalid_branch_refuses() {
        let (mut image, symbols) = guarded_image();
        let sltiu_a0_128 = (128u32 << 20) | (10 << 15) | (3 << 12) | (10 << 7) | 0x13;
        let branch = (10u32 << 15) | (1 << 12) | (4 << 7) | 0x63;
        image[..4].copy_from_slice(&sltiu_a0_128.to_le_bytes());
        image[4..8].copy_from_slice(&branch.to_le_bytes());
        assert!(execute_invalid_channel_predicate(&image, symbols).is_err());
    }
}
