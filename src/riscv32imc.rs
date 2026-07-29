//! Bounded RV32IMC execution for source-bound firmware contract extraction.
//!
//! This is deliberately not a general-purpose emulator. It accepts one flat
//! little-endian image, executes only the integer instruction subset emitted by
//! the frozen GCC firmware build, and refuses every unsupported or out-of-bounds
//! operation.

use riscv_decode::{
    Instruction, decode,
    types::{BType, IType, RType, SType, ShiftType},
};
use std::{error::Error, fmt};

pub const RV32_IMAGE_BASE: u32 = 0x8000_0000;
pub const MAX_RV32_IMAGE_BYTES: usize = 1024 * 1024;
pub const MAX_RV32_MEMORY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_RV32_STEPS: u64 = 1_000_000;
pub const RV32_MEMORY_BITMAP_WORDS: usize = MAX_RV32_MEMORY_BYTES / 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rv32SymbolLayout {
    pub entry: u32,
    pub event_count: u32,
    pub events: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMmioEvent {
    pub operation: u32,
    pub offset: u32,
    pub value: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rv32Execution {
    pub return_value: u32,
    pub steps: u64,
    pub events: Vec<CompiledMmioEvent>,
    pub event_program_locations: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rv32Error(pub String);

impl fmt::Display for Rv32Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "bounded RV32IMC extraction: {}", self.0)
    }
}

impl Error for Rv32Error {}

fn reject(message: impl Into<String>) -> Rv32Error {
    Rv32Error(message.into())
}

pub(crate) fn sign_extend(value: u32, bits: u32) -> u32 {
    (((value << (32 - bits)) as i32) >> (32 - bits)) as u32
}

pub(crate) fn rv32_div(dividend: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        u32::MAX
    } else if dividend == i32::MIN as u32 && divisor == (-1i32) as u32 {
        dividend
    } else {
        ((dividend as i32) / (divisor as i32)) as u32
    }
}

pub(crate) fn rv32_divu(dividend: u32, divisor: u32) -> u32 {
    dividend.checked_div(divisor).unwrap_or(u32::MAX)
}

pub(crate) fn rv32_rem(dividend: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        dividend
    } else if dividend == i32::MIN as u32 && divisor == (-1i32) as u32 {
        0
    } else {
        ((dividend as i32) % (divisor as i32)) as u32
    }
}

pub(crate) fn rv32_remu(dividend: u32, divisor: u32) -> u32 {
    dividend.checked_rem(divisor).unwrap_or(dividend)
}

fn encode_i(opcode: u32, funct3: u32, rd: u32, rs1: u32, imm: u32) -> u32 {
    ((imm & 0xfff) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

fn encode_r(funct7: u32, funct3: u32, rd: u32, rs1: u32, rs2: u32) -> u32 {
    (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x33
}

fn encode_s(funct3: u32, rs1: u32, rs2: u32, imm: u32) -> u32 {
    ((imm & 0xfe0) << 20) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | ((imm & 0x1f) << 7) | 0x23
}

fn encode_b(funct3: u32, rs1: u32, rs2: u32, imm: u32) -> u32 {
    ((imm & 0x1000) << 19)
        | ((imm & 0x7e0) << 20)
        | (rs2 << 20)
        | (rs1 << 15)
        | (funct3 << 12)
        | ((imm & 0x1e) << 7)
        | ((imm & 0x800) >> 4)
        | 0x63
}

fn encode_u(opcode: u32, rd: u32, imm: u32) -> u32 {
    (imm & 0xffff_f000) | (rd << 7) | opcode
}

fn encode_j(rd: u32, imm: u32) -> u32 {
    ((imm & 0x10_0000) << 11)
        | ((imm & 0x7fe) << 20)
        | ((imm & 0x800) << 9)
        | (imm & 0xff000)
        | (rd << 7)
        | 0x6f
}

fn bit(value: u16, source: u32, destination: u32) -> u32 {
    (((value as u32) >> source) & 1) << destination
}

fn bits(value: u16, high: u32, low: u32, destination: u32) -> u32 {
    (((value as u32) >> low) & ((1 << (high - low + 1)) - 1)) << destination
}

pub(crate) fn decompress(raw: u16) -> Result<u32, Rv32Error> {
    let quadrant = raw & 0b11;
    let funct3 = raw >> 13;
    let rd = ((raw >> 7) & 0x1f) as u32;
    let rs2 = ((raw >> 2) & 0x1f) as u32;
    let prime_rd = (((raw >> 2) & 0x7) + 8) as u32;
    let prime_rs1 = (((raw >> 7) & 0x7) + 8) as u32;
    match (quadrant, funct3) {
        (0, 0) => {
            let imm = bits(raw, 10, 7, 6) | bits(raw, 12, 11, 4) | bit(raw, 5, 3) | bit(raw, 6, 2);
            if imm == 0 {
                Err(reject("reserved C.ADDI4SPN"))
            } else {
                Ok(encode_i(0x13, 0, prime_rd, 2, imm))
            }
        }
        (0, 2) => {
            let imm = bit(raw, 5, 6) | bits(raw, 12, 10, 3) | bit(raw, 6, 2);
            Ok(encode_i(0x03, 2, prime_rd, prime_rs1, imm))
        }
        (0, 6) => {
            let imm = bit(raw, 5, 6) | bits(raw, 12, 10, 3) | bit(raw, 6, 2);
            Ok(encode_s(2, prime_rs1, prime_rd, imm))
        }
        (1, 0) => {
            let imm = bit(raw, 12, 5) | bits(raw, 6, 2, 0);
            Ok(encode_i(0x13, 0, rd, rd, sign_extend(imm, 6)))
        }
        (1, 1) | (1, 5) => {
            let imm = bit(raw, 12, 11)
                | bit(raw, 11, 4)
                | bit(raw, 8, 10)
                | bits(raw, 10, 9, 8)
                | bit(raw, 6, 7)
                | bit(raw, 7, 6)
                | bits(raw, 5, 3, 1)
                | bit(raw, 2, 5);
            Ok(encode_j(
                if funct3 == 1 { 1 } else { 0 },
                sign_extend(imm, 12),
            ))
        }
        (1, 2) => {
            let imm = bit(raw, 12, 5) | bits(raw, 6, 2, 0);
            if rd == 0 {
                Err(reject("reserved C.LI"))
            } else {
                Ok(encode_i(0x13, 0, rd, 0, sign_extend(imm, 6)))
            }
        }
        (1, 3) if rd == 2 => {
            let imm = bit(raw, 12, 9)
                | bit(raw, 6, 4)
                | bit(raw, 5, 6)
                | bits(raw, 4, 3, 7)
                | bit(raw, 2, 5);
            if imm == 0 {
                Err(reject("reserved C.ADDI16SP"))
            } else {
                Ok(encode_i(0x13, 0, 2, 2, sign_extend(imm, 10)))
            }
        }
        (1, 3) => {
            let imm = bit(raw, 12, 17) | bits(raw, 6, 2, 12);
            if rd < 2 || imm == 0 {
                Err(reject("reserved C.LUI"))
            } else {
                Ok(encode_u(0x37, rd, sign_extend(imm, 18)))
            }
        }
        (1, 4) => {
            let subop = (raw >> 10) & 0x3;
            let rs1 = prime_rs1;
            match subop {
                0 | 1 => {
                    if raw & 0x1000 != 0 {
                        return Err(reject("RV64 compressed shift in RV32 image"));
                    }
                    let shamt = ((raw >> 2) & 0x1f) as u32;
                    Ok(encode_i(
                        0x13,
                        5,
                        rs1,
                        rs1,
                        shamt | if subop == 1 { 0x400 } else { 0 },
                    ))
                }
                2 => {
                    let imm = bit(raw, 12, 5) | bits(raw, 6, 2, 0);
                    Ok(encode_i(0x13, 7, rs1, rs1, sign_extend(imm, 6)))
                }
                3 if raw & 0x1000 == 0 => {
                    let rs2 = (((raw >> 2) & 0x7) + 8) as u32;
                    let (funct7, base_funct3) = match (raw >> 5) & 0x3 {
                        0 => (0x20, 0),
                        1 => (0, 4),
                        2 => (0, 6),
                        3 => (0, 7),
                        _ => unreachable!(),
                    };
                    Ok(encode_r(funct7, base_funct3, rs1, rs1, rs2))
                }
                _ => Err(reject("unsupported compressed arithmetic instruction")),
            }
        }
        (1, 6) | (1, 7) => {
            let imm = bit(raw, 12, 8)
                | bits(raw, 6, 5, 6)
                | bit(raw, 2, 5)
                | bits(raw, 11, 10, 3)
                | bits(raw, 4, 3, 1);
            Ok(encode_b(
                if funct3 == 6 { 0 } else { 1 },
                prime_rs1,
                0,
                sign_extend(imm, 9),
            ))
        }
        (2, 0) => {
            if rd == 0 || raw & 0x1000 != 0 {
                Err(reject("reserved C.SLLI"))
            } else {
                Ok(encode_i(0x13, 1, rd, rd, ((raw >> 2) & 0x1f) as u32))
            }
        }
        (2, 2) => {
            let imm = bit(raw, 12, 5) | bits(raw, 6, 4, 2) | bits(raw, 3, 2, 6);
            if rd == 0 {
                Err(reject("reserved C.LWSP"))
            } else {
                Ok(encode_i(0x03, 2, rd, 2, imm))
            }
        }
        (2, 4) => match (raw & 0x1000 != 0, rs2) {
            (false, 0) if rd != 0 => Ok(encode_i(0x67, 0, 0, rd, 0)),
            (false, _) if rd != 0 => Ok(encode_r(0, 0, rd, 0, rs2)),
            (true, 0) if rd == 0 => Ok(0x0010_0073),
            (true, 0) => Ok(encode_i(0x67, 0, 1, rd, 0)),
            (true, _) if rd != 0 => Ok(encode_r(0, 0, rd, rd, rs2)),
            _ => Err(reject("reserved compressed jump or move")),
        },
        (2, 6) => {
            let imm = bits(raw, 12, 9, 2) | bits(raw, 8, 7, 6);
            Ok(encode_s(2, 2, rs2, imm))
        }
        _ => Err(reject(format!(
            "unsupported compressed instruction 0x{raw:04x}"
        ))),
    }
}

#[derive(Clone)]
struct Machine {
    memory: Vec<u8>,
    memory_known: Vec<bool>,
    registers: [u32; 32],
    register_known: [bool; 32],
    pc: u32,
    stop: u32,
    steps: u64,
    previous_pc: u32,
    event_count_address: u32,
    event_program_locations: Vec<u32>,
    observed_reads: Vec<Rv32MemoryAccess>,
    observed_writes: Vec<Rv32MemoryAccess>,
    observed_register_reads: u32,
    observed_register_writes: u32,
}

/// Opaque bounded RV32IMC state used by exact continuation certificates.
///
/// Callers can advance and compare states, but cannot manufacture or alter
/// machine contents. Equality covers every field that can affect later
/// execution or retained evidence, and never relies on a digest.
#[derive(Clone)]
pub struct Rv32ReplayMachine {
    machine: Machine,
    symbols: Rv32SymbolLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rv32MemoryAccess {
    pub address: u32,
    pub width: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rv32StepObservation {
    pub program_counter: u32,
    pub register_reads: u32,
    pub register_writes: u32,
    pub reads: Vec<Rv32MemoryAccess>,
    pub writes: Vec<Rv32MemoryAccess>,
}

impl PartialEq for Rv32ReplayMachine {
    fn eq(&self, other: &Self) -> bool {
        self.symbols == other.symbols && self.machine.same_state(&other.machine)
    }
}

impl Eq for Rv32ReplayMachine {}

impl Rv32ReplayMachine {
    pub fn new_with_a0(
        image: &[u8],
        symbols: Rv32SymbolLayout,
        a0: u32,
    ) -> Result<Self, Rv32Error> {
        Ok(Self {
            machine: initialize_machine(image, symbols, Some(a0))?,
            symbols,
        })
    }

    pub fn is_complete(&self) -> bool {
        self.machine.pc == self.machine.stop
    }

    pub fn steps(&self) -> u64 {
        self.machine.steps
    }

    pub fn program_counter(&self) -> u32 {
        self.machine.pc
    }

    pub fn step(&mut self) -> Result<(), Rv32Error> {
        self.step_observed().map(|_| ())
    }

    pub fn step_observed(&mut self) -> Result<Rv32StepObservation, Rv32Error> {
        if self.is_complete() {
            return Err(reject("cannot step a completed replay machine"));
        }
        self.machine.step()
    }

    /// Advance one scalar replay lane using an instruction decoded once by an
    /// independent batch checker.
    ///
    /// The lane still fetches and compares its own instruction bytes before
    /// executing the supplied decode. This prevents a forged shared trace from
    /// skipping, replacing or relocating code.
    pub(crate) fn step_predecoded(
        &mut self,
        expected_pc: u32,
        expected_word: u32,
        instruction_bytes: u8,
        instruction: &Instruction,
        image_end: u32,
    ) -> Result<Rv32StepObservation, Rv32Error> {
        if self.is_complete() {
            return Err(reject("cannot step a completed replay machine"));
        }
        self.machine.step_predecoded(
            expected_pc,
            expected_word,
            instruction_bytes,
            instruction,
            image_end,
        )
    }

    pub fn finish(mut self) -> Result<Rv32Execution, Rv32Error> {
        while !self.is_complete() {
            self.step()?;
        }
        finalize_machine(&self.machine, self.symbols)
    }

    pub(crate) fn exact_difference(&self, other: &Self) -> Option<String> {
        if self.symbols != other.symbols {
            return Some("symbol layout".to_string());
        }
        let left = &self.machine;
        let right = &other.machine;
        for (index, (left_known, right_known)) in left
            .register_known
            .iter()
            .zip(&right.register_known)
            .enumerate()
        {
            if left_known != right_known {
                return Some(format!("register x{index} knownness"));
            }
            if left.registers[index] != right.registers[index] {
                return Some(format!("register x{index} value"));
            }
        }
        for (index, (left_known, right_known)) in left
            .memory_known
            .iter()
            .zip(&right.memory_known)
            .enumerate()
        {
            if left_known != right_known {
                return Some(format!(
                    "memory knownness at 0x{:08x}",
                    RV32_IMAGE_BASE + index as u32
                ));
            }
            if left.memory[index] != right.memory[index] {
                return Some(format!(
                    "memory value at 0x{:08x}",
                    RV32_IMAGE_BASE + index as u32
                ));
            }
        }
        if left.pc != right.pc {
            return Some("program counter".to_string());
        }
        if left.stop != right.stop {
            return Some("stop address".to_string());
        }
        if left.steps != right.steps {
            return Some("step count".to_string());
        }
        if left.previous_pc != right.previous_pc {
            return Some("previous program counter".to_string());
        }
        if left.event_count_address != right.event_count_address {
            return Some("event-count address".to_string());
        }
        if left.event_program_locations != right.event_program_locations {
            return Some("event program locations".to_string());
        }
        None
    }

    pub(crate) fn live_state_equal(
        &self,
        other: &Self,
        live_memory: &[u64],
    ) -> Result<bool, Rv32Error> {
        if live_memory.len() != RV32_MEMORY_BITMAP_WORDS {
            return Err(reject("live-memory bitmap length is outside policy"));
        }
        if self.symbols != other.symbols || !self.machine.same_non_memory_state(&other.machine) {
            return Ok(false);
        }
        for (word_index, word) in live_memory.iter().copied().enumerate() {
            let mut remaining = word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                let index = word_index * 64 + bit;
                if self.machine.memory[index] != other.machine.memory[index]
                    || self.machine.memory_known[index] != other.machine.memory_known[index]
                {
                    return Ok(false);
                }
                remaining &= remaining - 1;
            }
        }
        Ok(true)
    }

    pub(crate) fn non_memory_state_equal(&self, other: &Self) -> bool {
        self.symbols == other.symbols && self.machine.same_non_memory_state(&other.machine)
    }

    pub(crate) fn control_state_equal(&self, other: &Self) -> bool {
        self.symbols == other.symbols && self.machine.same_control_state(&other.machine)
    }

    pub(crate) fn live_slice_equal(
        &self,
        other: &Self,
        live_registers: u32,
        live_memory: &[u64],
    ) -> Result<bool, Rv32Error> {
        if live_memory.len() != RV32_MEMORY_BITMAP_WORDS {
            return Err(reject("live-memory bitmap length is outside policy"));
        }
        if !self.control_state_equal(other) {
            return Ok(false);
        }
        for register in 0..32 {
            if live_registers & (1u32 << register) != 0
                && (self.machine.registers[register] != other.machine.registers[register]
                    || self.machine.register_known[register]
                        != other.machine.register_known[register])
            {
                return Ok(false);
            }
        }
        for (word_index, word) in live_memory.iter().copied().enumerate() {
            let mut remaining = word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                let index = word_index * 64 + bit;
                if self.machine.memory[index] != other.machine.memory[index]
                    || self.machine.memory_known[index] != other.machine.memory_known[index]
                {
                    return Ok(false);
                }
                remaining &= remaining - 1;
            }
        }
        Ok(true)
    }
}

impl Machine {
    fn same_control_state(&self, other: &Self) -> bool {
        self.pc == other.pc
            && self.stop == other.stop
            && self.steps == other.steps
            && self.previous_pc == other.previous_pc
            && self.event_count_address == other.event_count_address
            && self.event_program_locations == other.event_program_locations
    }

    fn same_non_memory_state(&self, other: &Self) -> bool {
        self.registers == other.registers
            && self.register_known == other.register_known
            && self.same_control_state(other)
    }

    fn same_state(&self, other: &Self) -> bool {
        self.memory == other.memory
            && self.memory_known == other.memory_known
            && self.same_non_memory_state(other)
    }

    fn index(&self, address: u32, width: usize) -> Result<usize, Rv32Error> {
        let offset = address
            .checked_sub(RV32_IMAGE_BASE)
            .ok_or_else(|| reject(format!("address below image: 0x{address:08x}")))?;
        let index = usize::try_from(offset).map_err(|_| reject("address conversion overflow"))?;
        if index
            .checked_add(width)
            .is_none_or(|end| end > self.memory.len())
        {
            return Err(reject(format!(
                "memory access outside policy: 0x{address:08x}"
            )));
        }
        Ok(index)
    }

    fn load(&self, address: u32, width: usize) -> Result<u32, Rv32Error> {
        let index = self.index(address, width)?;
        let mut value = 0u32;
        for byte in 0..width {
            value |= (self.memory[index + byte] as u32) << (byte * 8);
        }
        Ok(value)
    }

    fn load_is_known(&self, address: u32, width: usize) -> Result<bool, Rv32Error> {
        let index = self.index(address, width)?;
        Ok(self.memory_known[index..index + width]
            .iter()
            .all(|known| *known))
    }

    fn store(&mut self, address: u32, width: usize, value: u32) -> Result<(), Rv32Error> {
        self.store_with_knownness(address, width, value, true)
    }

    fn store_with_knownness(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
        known: bool,
    ) -> Result<(), Rv32Error> {
        let index = self.index(address, width)?;
        for byte in 0..width {
            self.memory[index + byte] = (value >> (byte * 8)) as u8;
            self.memory_known[index + byte] = known;
        }
        Ok(())
    }

    fn reg(&self, index: u32) -> u32 {
        self.registers[index as usize]
    }

    fn set_reg(&mut self, index: u32, value: u32) {
        if index != 0 {
            self.observed_register_writes |= 1u32 << index;
            self.registers[index as usize] = value;
            self.register_known[index as usize] = true;
        }
    }

    fn set_unknown(&mut self, index: u32) {
        if index != 0 {
            self.observed_register_writes |= 1u32 << index;
            self.register_known[index as usize] = false;
        }
    }

    fn observe_register_read(&mut self, index: u32) {
        self.observed_register_reads |= 1u32 << index;
    }

    fn require_reg(&self, current: u32, index: u32, role: &str) -> Result<u32, Rv32Error> {
        if self.register_known[index as usize] {
            Ok(self.reg(index))
        } else {
            Err(reject(format!(
                "runtime-unknown x{index} used as {role} at 0x{current:08x}"
            )))
        }
    }

    fn step(&mut self) -> Result<Rv32StepObservation, Rv32Error> {
        if self.steps >= MAX_RV32_STEPS {
            return Err(reject("instruction step bound exceeded"));
        }
        self.observed_reads.clear();
        self.observed_writes.clear();
        self.observed_register_reads = 0;
        self.observed_register_writes = 0;
        let observed_pc = self.pc;
        let low = self
            .load(self.pc, 2)
            .map_err(|error| reject(format!("fetch at PC 0x{:08x}: {error}", self.pc)))?
            as u16;
        let length = if low & 3 == 3 { 4 } else { 2 };
        self.observed_reads.push(Rv32MemoryAccess {
            address: self.pc,
            width: length as u8,
        });
        let word = if length == 2 {
            decompress(low).map_err(|error| {
                reject(format!(
                    "compressed decode failed at 0x{:08x} after 0x{:08x}: {error}",
                    self.pc, self.previous_pc
                ))
            })?
        } else {
            self.load(self.pc, 4)?
        };
        let instruction = decode(word)
            .map_err(|error| reject(format!("decode failed at 0x{:08x}: {error:?}", self.pc)))?;
        let current = self.pc;
        self.previous_pc = current;
        self.pc = self.pc.wrapping_add(length);
        self.steps += 1;
        self.execute(current, instruction)?;
        Ok(self.observation(observed_pc))
    }

    fn step_predecoded(
        &mut self,
        expected_pc: u32,
        expected_word: u32,
        instruction_bytes: u8,
        instruction: &Instruction,
        image_end: u32,
    ) -> Result<Rv32StepObservation, Rv32Error> {
        if self.steps >= MAX_RV32_STEPS {
            return Err(reject("instruction step bound exceeded"));
        }
        if self.pc != expected_pc {
            return Err(reject(format!(
                "shared control trace expected PC 0x{expected_pc:08x}, lane reached 0x{:08x}",
                self.pc
            )));
        }
        if !matches!(instruction_bytes, 2 | 4) {
            return Err(reject("shared control trace instruction width is invalid"));
        }
        self.observed_reads.clear();
        self.observed_writes.clear();
        self.observed_register_reads = 0;
        self.observed_register_writes = 0;
        let low = self
            .load(self.pc, 2)
            .map_err(|error| reject(format!("fetch at PC 0x{:08x}: {error}", self.pc)))?
            as u16;
        let actual_bytes = if low & 3 == 3 { 4 } else { 2 };
        let actual_word = if actual_bytes == 2 {
            decompress(low).map_err(|error| {
                reject(format!(
                    "compressed decode failed at 0x{:08x} after 0x{:08x}: {error}",
                    self.pc, self.previous_pc
                ))
            })?
        } else {
            self.load(self.pc, 4)?
        };
        if actual_bytes != u32::from(instruction_bytes) || actual_word != expected_word {
            return Err(reject(format!(
                "shared control trace instruction mismatch at 0x{:08x}",
                self.pc
            )));
        }
        self.observed_reads.push(Rv32MemoryAccess {
            address: self.pc,
            width: instruction_bytes,
        });
        let current = self.pc;
        self.previous_pc = current;
        self.pc = self.pc.wrapping_add(actual_bytes);
        self.steps += 1;
        self.execute(current, *instruction)?;
        for write in &self.observed_writes {
            if write.address < image_end
                && write
                    .address
                    .checked_add(u32::from(write.width))
                    .is_some_and(|end| end > RV32_IMAGE_BASE)
            {
                return Err(reject(
                    "self-modifying code is unsupported by shared control replay",
                ));
            }
        }
        Ok(self.observation(current))
    }

    fn observation(&self, program_counter: u32) -> Rv32StepObservation {
        Rv32StepObservation {
            program_counter,
            register_reads: self.observed_register_reads,
            register_writes: self.observed_register_writes,
            reads: self.observed_reads.clone(),
            writes: self.observed_writes.clone(),
        }
    }

    fn execute(&mut self, current: u32, instruction: Instruction) -> Result<(), Rv32Error> {
        use Instruction::*;
        match instruction {
            Lui(value) => self.set_reg(value.rd(), value.imm()),
            Auipc(value) => self.set_reg(value.rd(), current.wrapping_add(value.imm())),
            Jal(value) => {
                self.set_reg(value.rd(), self.pc);
                self.pc = current.wrapping_add(sign_extend(value.imm(), 21));
            }
            Jalr(value) => {
                self.observe_register_read(value.rs1());
                let target = self
                    .require_reg(current, value.rs1(), "jump target")?
                    .wrapping_add(sign_extend(value.imm(), 12))
                    & !1;
                if target == 0 {
                    return Err(reject(format!(
                        "zero jump target at 0x{current:08x} through x{}",
                        value.rs1()
                    )));
                }
                self.set_reg(value.rd(), self.pc);
                self.pc = target;
            }
            Beq(value) => self.branch(current, value, |a, b| a == b)?,
            Bne(value) => self.branch(current, value, |a, b| a != b)?,
            Blt(value) => self.branch(current, value, |a, b| (a as i32) < (b as i32))?,
            Bge(value) => self.branch(current, value, |a, b| (a as i32) >= (b as i32))?,
            Bltu(value) => self.branch(current, value, |a, b| a < b)?,
            Bgeu(value) => self.branch(current, value, |a, b| a >= b)?,
            Lb(value) => self.load_i(current, value, 1, true)?,
            Lh(value) => self.load_i(current, value, 2, true)?,
            Lw(value) => self.load_i(current, value, 4, false)?,
            Lbu(value) => self.load_i(current, value, 1, false)?,
            Lhu(value) => self.load_i(current, value, 2, false)?,
            Sb(value) => self.store_s(current, value, 1)?,
            Sh(value) => self.store_s(current, value, 2)?,
            Sw(value) => self.store_s(current, value, 4)?,
            Addi(value) => self.op_i(value, |a, b| a.wrapping_add(b)),
            Slti(value) => self.op_i(value, |a, b| ((a as i32) < (b as i32)) as u32),
            Sltiu(value) => self.op_i(value, |a, b| (a < b) as u32),
            Xori(value) => self.op_i(value, |a, b| a ^ b),
            Ori(value) => self.op_i(value, |a, b| a | b),
            Andi(value) => self.op_i(value, |a, b| a & b),
            Slli(value) => self.shift_i(value, |a, b| a << b),
            Srli(value) => self.shift_i(value, |a, b| a >> b),
            Srai(value) => self.shift_i(value, |a, b| ((a as i32) >> b) as u32),
            Add(value) => self.op_r(value, |a, b| a.wrapping_add(b)),
            Sub(value) => self.op_r(value, |a, b| a.wrapping_sub(b)),
            Sll(value) => self.op_r(value, |a, b| a << (b & 31)),
            Slt(value) => self.op_r(value, |a, b| ((a as i32) < (b as i32)) as u32),
            Sltu(value) => self.op_r(value, |a, b| (a < b) as u32),
            Xor(value) => self.op_r(value, |a, b| a ^ b),
            Srl(value) => self.op_r(value, |a, b| a >> (b & 31)),
            Sra(value) => self.op_r(value, |a, b| ((a as i32) >> (b & 31)) as u32),
            Or(value) => self.op_r(value, |a, b| a | b),
            And(value) => self.op_r(value, |a, b| a & b),
            Mul(value) => self.op_r(value, |a, b| a.wrapping_mul(b)),
            Div(value) => self.op_r(value, rv32_div),
            Divu(value) => self.op_r(value, rv32_divu),
            Rem(value) => self.op_r(value, rv32_rem),
            Remu(value) => self.op_r(value, rv32_remu),
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
        predicate: impl FnOnce(u32, u32) -> bool,
    ) -> Result<(), Rv32Error> {
        self.observe_register_read(value.rs1());
        self.observe_register_read(value.rs2());
        let left = self.require_reg(current, value.rs1(), "branch operand")?;
        let right = self.require_reg(current, value.rs2(), "branch operand")?;
        if predicate(left, right) {
            self.pc = current.wrapping_add(sign_extend(value.imm(), 13));
        }
        Ok(())
    }

    fn load_i(
        &mut self,
        current: u32,
        value: IType,
        width: usize,
        signed: bool,
    ) -> Result<(), Rv32Error> {
        self.observe_register_read(value.rs1());
        let address = self
            .require_reg(current, value.rs1(), "load address")?
            .wrapping_add(sign_extend(value.imm(), 12));
        let loaded = self.load(address, width).map_err(|error| {
            reject(format!(
                "load at PC 0x{current:08x} through x{}=0x{:08x}: {error}",
                value.rs1(),
                self.reg(value.rs1())
            ))
        })?;
        self.observed_reads.push(Rv32MemoryAccess {
            address,
            width: width as u8,
        });
        let result = if signed && width < 4 {
            sign_extend(loaded, (width * 8) as u32)
        } else {
            loaded
        };
        if self.load_is_known(address, width)? {
            self.set_reg(value.rd(), result);
        } else {
            self.set_unknown(value.rd());
        }
        Ok(())
    }

    fn store_s(&mut self, current: u32, value: SType, width: usize) -> Result<(), Rv32Error> {
        self.observe_register_read(value.rs1());
        self.observe_register_read(value.rs2());
        let address = self
            .require_reg(current, value.rs1(), "store address")?
            .wrapping_add(sign_extend(value.imm(), 12));
        let stored = self.reg(value.rs2());
        let stored_known = self.register_known[value.rs2() as usize];
        self.observed_writes.push(Rv32MemoryAccess {
            address,
            width: width as u8,
        });
        self.store_with_knownness(address, width, stored, stored_known)
            .map_err(|error| {
                reject(format!(
                    "store at PC 0x{current:08x} through x{}=0x{:08x}: {error}",
                    value.rs1(),
                    self.reg(value.rs1())
                ))
            })?;
        if width == 4 && address == self.event_count_address {
            if !stored_known {
                return Err(reject(format!(
                    "runtime-unknown event count stored at 0x{current:08x}"
                )));
            }
            self.event_program_locations.push(current);
        }
        Ok(())
    }

    fn op_i(&mut self, value: IType, operation: impl FnOnce(u32, u32) -> u32) {
        self.observe_register_read(value.rs1());
        if self.register_known[value.rs1() as usize] {
            self.set_reg(
                value.rd(),
                operation(self.reg(value.rs1()), sign_extend(value.imm(), 12)),
            );
        } else {
            self.set_unknown(value.rd());
        }
    }

    fn shift_i(&mut self, value: ShiftType, operation: impl FnOnce(u32, u32) -> u32) {
        self.observe_register_read(value.rs1());
        if self.register_known[value.rs1() as usize] {
            self.set_reg(
                value.rd(),
                operation(self.reg(value.rs1()), value.shamt() & 31),
            );
        } else {
            self.set_unknown(value.rd());
        }
    }

    fn op_r(&mut self, value: RType, operation: impl FnOnce(u32, u32) -> u32) {
        self.observe_register_read(value.rs1());
        self.observe_register_read(value.rs2());
        if self.register_known[value.rs1() as usize] && self.register_known[value.rs2() as usize] {
            self.set_reg(
                value.rd(),
                operation(self.reg(value.rs1()), self.reg(value.rs2())),
            );
        } else {
            self.set_unknown(value.rd());
        }
    }
}

pub fn execute_compiled_mmio(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<Rv32Execution, Rv32Error> {
    execute_compiled_mmio_inner(image, symbols, None)
}

/// Execute the bounded firmware entry with one concrete RISC-V `a0` argument.
///
/// The remaining argument registers retain the fail-closed unknown state used
/// by [`execute_compiled_mmio`]. This narrow API prevents a caller from
/// supplying stack, return-address or ambient machine state as if it were a
/// firmware input.
pub fn execute_compiled_mmio_with_a0(
    image: &[u8],
    symbols: Rv32SymbolLayout,
    a0: u32,
) -> Result<Rv32Execution, Rv32Error> {
    execute_compiled_mmio_inner(image, symbols, Some(a0))
}

fn execute_compiled_mmio_inner(
    image: &[u8],
    symbols: Rv32SymbolLayout,
    a0: Option<u32>,
) -> Result<Rv32Execution, Rv32Error> {
    let mut machine = initialize_machine(image, symbols, a0)?;
    while machine.pc != machine.stop {
        machine.step()?;
    }
    finalize_machine(&machine, symbols)
}

fn initialize_machine(
    image: &[u8],
    symbols: Rv32SymbolLayout,
    a0: Option<u32>,
) -> Result<Machine, Rv32Error> {
    if image.is_empty() || image.len() > MAX_RV32_IMAGE_BYTES {
        return Err(reject("image size is outside policy"));
    }
    let memory_end = RV32_IMAGE_BASE
        .checked_add(MAX_RV32_MEMORY_BYTES as u32)
        .ok_or_else(|| reject("memory range overflow"))?;
    for (label, address) in [
        ("entry", symbols.entry),
        ("event count", symbols.event_count),
        ("events", symbols.events),
    ] {
        if !(RV32_IMAGE_BASE..memory_end).contains(&address) {
            return Err(reject(format!("{label} symbol is outside memory policy")));
        }
    }
    let stop = RV32_IMAGE_BASE
        .checked_add(MAX_RV32_MEMORY_BYTES as u32 - 4)
        .ok_or_else(|| reject("stop address overflow"))?;
    let mut machine = Machine {
        memory: vec![0; MAX_RV32_MEMORY_BYTES],
        memory_known: vec![true; MAX_RV32_MEMORY_BYTES],
        registers: [0; 32],
        register_known: [false; 32],
        pc: symbols.entry,
        stop,
        steps: 0,
        previous_pc: symbols.entry,
        event_count_address: symbols.event_count,
        event_program_locations: Vec::new(),
        observed_reads: Vec::new(),
        observed_writes: Vec::new(),
        observed_register_reads: 0,
        observed_register_writes: 0,
    };
    machine.register_known[0] = true;
    machine.memory[..image.len()].copy_from_slice(image);
    machine.store(stop, 4, 0x0010_0073)?;
    machine.set_reg(1, stop);
    machine.set_reg(2, stop & !0xf);
    if let Some(a0) = a0 {
        machine.set_reg(10, a0);
    }
    Ok(machine)
}

fn finalize_machine(
    machine: &Machine,
    symbols: Rv32SymbolLayout,
) -> Result<Rv32Execution, Rv32Error> {
    if machine.pc != machine.stop {
        return Err(reject("cannot finalize an incomplete replay machine"));
    }
    let return_value = machine.require_reg(machine.stop, 10, "firmware return value")?;
    if !machine.load_is_known(symbols.event_count, 4)? {
        return Err(reject("compiled event count is runtime-unknown"));
    }
    let event_count = machine.load(symbols.event_count, 4)? as usize;
    if event_count > 32 {
        return Err(reject("compiled event count exceeds policy"));
    }
    if machine.event_program_locations.len() != event_count {
        return Err(reject(
            "compiled event writes do not match the final event count",
        ));
    }
    let mut events = Vec::with_capacity(event_count);
    for index in 0..event_count {
        let address = symbols
            .events
            .checked_add((index * 12) as u32)
            .ok_or_else(|| reject("compiled event address overflow"))?;
        if !machine.load_is_known(address, 12)? {
            return Err(reject("compiled event contains a runtime-unknown field"));
        }
        events.push(CompiledMmioEvent {
            operation: machine.load(address, 4)?,
            offset: machine.load(address + 4, 4)?,
            value: machine.load(address + 8, 4)?,
        });
    }
    Ok(Rv32Execution {
        return_value,
        steps: machine.steps,
        events,
        event_program_locations: machine.event_program_locations.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_extension_is_exact() {
        assert_eq!(sign_extend(0x7ff, 12), 0x7ff);
        assert_eq!(sign_extend(0x800, 12), 0xffff_f800);
    }

    #[test]
    fn rv32m_division_edges_match_the_frozen_semantics() {
        assert_eq!(rv32_div(7, 0), u32::MAX);
        assert_eq!(rv32_divu(7, 0), u32::MAX);
        assert_eq!(rv32_rem(7, 0), 7);
        assert_eq!(rv32_remu(7, 0), 7);
        assert_eq!(rv32_div(i32::MIN as u32, (-1i32) as u32), i32::MIN as u32);
        assert_eq!(rv32_rem(i32::MIN as u32, (-1i32) as u32), 0);
        assert_eq!(rv32_div((-7i32) as u32, 3), (-2i32) as u32);
        assert_eq!(rv32_rem((-7i32) as u32, 3), (-1i32) as u32);
    }

    #[test]
    fn rv32m_division_cross_product_matches_independent_references() {
        let operands = [
            0,
            1,
            2,
            3,
            7,
            31,
            i32::MAX as u32,
            i32::MIN as u32,
            u32::MAX,
            (-2i32) as u32,
            0x1234_5678,
            0x8765_4321,
        ];
        for dividend in operands {
            for divisor in operands {
                let signed_quotient = if divisor == 0 {
                    u32::MAX
                } else if dividend == i32::MIN as u32 && divisor == (-1i32) as u32 {
                    dividend
                } else {
                    ((dividend as i32) / (divisor as i32)) as u32
                };
                let signed_remainder = if divisor == 0 {
                    dividend
                } else if dividend == i32::MIN as u32 && divisor == (-1i32) as u32 {
                    0
                } else {
                    ((dividend as i32) % (divisor as i32)) as u32
                };
                assert_eq!(rv32_div(dividend, divisor), signed_quotient);
                assert_eq!(rv32_rem(dividend, divisor), signed_remainder);
                assert_eq!(
                    rv32_divu(dividend, divisor),
                    dividend.checked_div(divisor).unwrap_or(u32::MAX)
                );
                assert_eq!(
                    rv32_remu(dividend, divisor),
                    dividend.checked_rem(divisor).unwrap_or(dividend)
                );
            }
        }
    }

    #[test]
    fn executes_all_four_rv32m_division_instructions() {
        let cases = [(4, 21, 4, 5), (5, 21, 4, 5), (6, 21, 4, 1), (7, 21, 4, 1)];
        for (funct3, dividend, divisor, expected) in cases {
            let mut image = vec![0; 0x110];
            image[..4].copy_from_slice(&encode_i(0x13, 0, 5, 0, dividend).to_le_bytes());
            image[4..8].copy_from_slice(&encode_i(0x13, 0, 6, 0, divisor).to_le_bytes());
            image[8..12].copy_from_slice(&encode_r(1, funct3, 10, 5, 6).to_le_bytes());
            image[12..16].copy_from_slice(&encode_i(0x67, 0, 0, 1, 0).to_le_bytes());
            let result = execute_compiled_mmio(
                &image,
                Rv32SymbolLayout {
                    entry: RV32_IMAGE_BASE,
                    event_count: RV32_IMAGE_BASE + 0x100,
                    events: RV32_IMAGE_BASE + 0x104,
                },
            )
            .unwrap();
            assert_eq!(result.return_value, expected);
        }
    }

    #[test]
    fn refuses_unbounded_inputs() {
        assert!(
            execute_compiled_mmio(
                &[],
                Rv32SymbolLayout {
                    entry: RV32_IMAGE_BASE,
                    event_count: RV32_IMAGE_BASE,
                    events: RV32_IMAGE_BASE,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn compressed_jump_expansion_preserves_the_target_offset() {
        let Instruction::Jal(jump) = decode(decompress(0x2039).unwrap()).unwrap() else {
            panic!("expected C.JAL to expand to JAL");
        };
        assert_eq!(jump.rd(), 1);
        assert_eq!(sign_extend(jump.imm(), 21), 14);
    }

    #[test]
    fn executes_a_small_bounded_returning_program() {
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&encode_i(0x13, 0, 10, 0, 7).to_le_bytes());
        image[4..8].copy_from_slice(&encode_i(0x67, 0, 0, 1, 0).to_le_bytes());
        let result = execute_compiled_mmio(
            &image,
            Rv32SymbolLayout {
                entry: RV32_IMAGE_BASE,
                event_count: RV32_IMAGE_BASE + 0x100,
                events: RV32_IMAGE_BASE + 0x104,
            },
        )
        .unwrap();
        assert_eq!(result.return_value, 7);
        assert_eq!(result.steps, 2);
        assert!(result.events.is_empty());
        assert!(result.event_program_locations.is_empty());
    }

    #[test]
    fn refuses_a_runtime_argument_at_its_first_control_effect() {
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&encode_b(0, 10, 0, 8).to_le_bytes());
        image[4..8].copy_from_slice(&encode_i(0x67, 0, 0, 1, 0).to_le_bytes());
        image[8..12].copy_from_slice(&encode_i(0x67, 0, 0, 1, 0).to_le_bytes());
        let error = execute_compiled_mmio(
            &image,
            Rv32SymbolLayout {
                entry: RV32_IMAGE_BASE,
                event_count: RV32_IMAGE_BASE + 0x100,
                events: RV32_IMAGE_BASE + 0x104,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("runtime-unknown x10"));
        assert!(error.to_string().contains("branch operand"));
    }

    #[test]
    fn concrete_a0_is_available_without_concretising_other_arguments() {
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&encode_i(0x13, 7, 10, 10, 1).to_le_bytes());
        image[4..8].copy_from_slice(&encode_i(0x67, 0, 0, 1, 0).to_le_bytes());
        let result = execute_compiled_mmio_with_a0(
            &image,
            Rv32SymbolLayout {
                entry: RV32_IMAGE_BASE,
                event_count: RV32_IMAGE_BASE + 0x100,
                events: RV32_IMAGE_BASE + 0x104,
            },
            42,
        )
        .unwrap();
        assert_eq!(result.return_value, 0);
        assert_eq!(result.steps, 2);
    }

    #[test]
    fn replay_state_equality_requires_complete_concrete_convergence() {
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&encode_i(0x13, 7, 10, 10, 1).to_le_bytes());
        image[4..8].copy_from_slice(&encode_i(0x67, 0, 0, 1, 0).to_le_bytes());
        let symbols = Rv32SymbolLayout {
            entry: RV32_IMAGE_BASE,
            event_count: RV32_IMAGE_BASE + 0x100,
            events: RV32_IMAGE_BASE + 0x104,
        };
        let mut even_zero = Rv32ReplayMachine::new_with_a0(&image, symbols, 0).unwrap();
        let mut even_two = Rv32ReplayMachine::new_with_a0(&image, symbols, 2).unwrap();
        assert!(even_zero != even_two);
        even_zero.step().unwrap();
        even_two.step().unwrap();
        assert!(even_zero == even_two);
        assert_eq!(even_zero.clone().finish().unwrap().return_value, 0);
    }
}
