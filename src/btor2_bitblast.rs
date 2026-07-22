//! Proof-carrying bounded bit-blasting for the strict BTOR2 semantic core.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};
use varisat::{ExtendFormula, Lit as SolverLit, Solver, Var};

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind, UnaryOp, WordValues};
use crate::btor2_search::SearchResult;
use crate::unsat_proof::{self, CnfClause};

pub const BTOR2_BITBLAST_VERSION: u32 = 1;
pub const MAX_BITBLAST_HORIZON: u32 = 64;
pub const MAX_BITBLAST_INPUT_BITS: usize = 64;
pub const MAX_BITBLAST_VARIABLES: usize = 1_000_000;
pub const MAX_BITBLAST_CLAUSES: usize = 1_000_000;
pub const MAX_BITBLAST_CERTIFICATE_BYTES: usize = 2 * 1024 * 1024;
const BITBLAST_MAGIC: &[u8; 8] = b"GCCBBB01";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2BitblastCertificate {
    pub version: u32,
    pub source_sha256: [u8; 32],
    pub bad_property: NodeId,
    pub horizon: u32,
    pub result: SearchResult,
    pub bad_frame: Option<u32>,
    pub witness_valuations: Vec<u64>,
    pub unsat_proof: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Btor2BitblastSummary {
    pub version: u32,
    pub result: SearchResult,
    pub bad_frame: Option<u32>,
    pub variables: usize,
    pub clauses: usize,
    pub proof_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2BitblastError(pub String);

impl fmt::Display for Btor2BitblastError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Btor2BitblastError {}

fn reject(message: impl Into<String>) -> Btor2BitblastError {
    Btor2BitblastError(message.into())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CnfLit {
    variable: usize,
    positive: bool,
}

impl CnfLit {
    fn not(self) -> Self {
        Self {
            variable: self.variable,
            positive: !self.positive,
        }
    }
}

#[derive(Debug)]
struct Circuit {
    variables: usize,
    clauses: Vec<CnfClause>,
    truth: CnfLit,
}

impl Circuit {
    fn new() -> Self {
        let truth = CnfLit {
            variable: 0,
            positive: true,
        };
        Self {
            variables: 1,
            clauses: vec![CnfClause(vec![(0, true)])],
            truth,
        }
    }

    fn constant(&self, value: bool) -> CnfLit {
        if value { self.truth } else { self.truth.not() }
    }

    fn variable(&mut self) -> Result<CnfLit, Btor2BitblastError> {
        if self.variables >= MAX_BITBLAST_VARIABLES {
            return Err(reject("BTOR2 bitblast variable count exceeds policy"));
        }
        let result = CnfLit {
            variable: self.variables,
            positive: true,
        };
        self.variables += 1;
        Ok(result)
    }

    fn clause(&mut self, values: &[CnfLit]) -> Result<(), Btor2BitblastError> {
        if self.clauses.len() >= MAX_BITBLAST_CLAUSES {
            return Err(reject("BTOR2 bitblast clause count exceeds policy"));
        }
        self.clauses.push(CnfClause(
            values
                .iter()
                .map(|value| (value.variable, value.positive))
                .collect(),
        ));
        Ok(())
    }

    fn equal(&mut self, left: CnfLit, right: CnfLit) -> Result<(), Btor2BitblastError> {
        self.clause(&[left.not(), right])?;
        self.clause(&[left, right.not()])
    }

    fn and(&mut self, left: CnfLit, right: CnfLit) -> Result<CnfLit, Btor2BitblastError> {
        let output = self.variable()?;
        self.clause(&[output.not(), left])?;
        self.clause(&[output.not(), right])?;
        self.clause(&[output, left.not(), right.not()])?;
        Ok(output)
    }

    fn or(&mut self, left: CnfLit, right: CnfLit) -> Result<CnfLit, Btor2BitblastError> {
        let output = self.variable()?;
        self.clause(&[output, left.not()])?;
        self.clause(&[output, right.not()])?;
        self.clause(&[output.not(), left, right])?;
        Ok(output)
    }

    fn xor(&mut self, left: CnfLit, right: CnfLit) -> Result<CnfLit, Btor2BitblastError> {
        let output = self.variable()?;
        self.clause(&[output.not(), left.not(), right.not()])?;
        self.clause(&[output.not(), left, right])?;
        self.clause(&[output, left.not(), right])?;
        self.clause(&[output, left, right.not()])?;
        Ok(output)
    }

    fn ite(
        &mut self,
        condition: CnfLit,
        when_true: CnfLit,
        when_false: CnfLit,
    ) -> Result<CnfLit, Btor2BitblastError> {
        let selected_true = self.and(condition, when_true)?;
        let selected_false = self.and(condition.not(), when_false)?;
        self.or(selected_true, selected_false)
    }

    fn and_all(&mut self, values: &[CnfLit]) -> Result<CnfLit, Btor2BitblastError> {
        values
            .iter()
            .try_fold(self.constant(true), |result, value| {
                self.and(result, *value)
            })
    }

    fn or_all(&mut self, values: &[CnfLit]) -> Result<CnfLit, Btor2BitblastError> {
        values
            .iter()
            .try_fold(self.constant(false), |result, value| {
                self.or(result, *value)
            })
    }

    fn add_bits(
        &mut self,
        left: &[CnfLit],
        right: &[CnfLit],
    ) -> Result<Vec<CnfLit>, Btor2BitblastError> {
        if left.len() != right.len() {
            return Err(reject("BTOR2 bitblast add width mismatch"));
        }
        let mut carry = self.constant(false);
        let mut output = Vec::with_capacity(left.len());
        for (left, right) in left.iter().zip(right) {
            let pair = self.xor(*left, *right)?;
            output.push(self.xor(pair, carry)?);
            let both = self.and(*left, *right)?;
            let carried = self.and(pair, carry)?;
            carry = self.or(both, carried)?;
        }
        Ok(output)
    }

    fn subtract_bits(
        &mut self,
        left: &[CnfLit],
        right: &[CnfLit],
    ) -> Result<Vec<CnfLit>, Btor2BitblastError> {
        let mut negated = right.iter().map(|value| value.not()).collect::<Vec<_>>();
        let mut one = vec![self.constant(false); right.len()];
        if let Some(low) = one.first_mut() {
            *low = self.constant(true);
        }
        negated = self.add_bits(&negated, &one)?;
        self.add_bits(left, &negated)
    }

    fn multiply_bits(
        &mut self,
        left: &[CnfLit],
        right: &[CnfLit],
    ) -> Result<Vec<CnfLit>, Btor2BitblastError> {
        if left.len() != right.len() {
            return Err(reject("BTOR2 bitblast multiply width mismatch"));
        }
        let width = left.len();
        let mut result = vec![self.constant(false); width];
        for (shift, multiplier) in right.iter().enumerate() {
            let mut partial = vec![self.constant(false); width];
            for index in shift..width {
                partial[index] = self.and(left[index - shift], *multiplier)?;
            }
            result = self.add_bits(&result, &partial)?;
        }
        Ok(result)
    }

    fn shift_bits(
        &mut self,
        value: &[CnfLit],
        amount: &[CnfLit],
        left: bool,
    ) -> Result<Vec<CnfLit>, Btor2BitblastError> {
        let mut result = value.to_vec();
        for (bit, selector) in amount.iter().enumerate() {
            let distance = 1usize.checked_shl(bit as u32);
            let previous = result.clone();
            for index in 0..result.len() {
                let shifted = if left {
                    distance
                        .and_then(|distance| index.checked_sub(distance))
                        .map(|source| previous[source])
                        .unwrap_or_else(|| self.constant(false))
                } else {
                    distance
                        .and_then(|distance| index.checked_add(distance))
                        .and_then(|source| previous.get(source))
                        .copied()
                        .unwrap_or_else(|| self.constant(false))
                };
                result[index] = self.ite(*selector, shifted, previous[index])?;
            }
        }
        Ok(result)
    }

    fn unsigned_compare(
        &mut self,
        left: &[CnfLit],
        right: &[CnfLit],
    ) -> Result<(CnfLit, CnfLit), Btor2BitblastError> {
        if left.len() != right.len() {
            return Err(reject("BTOR2 bitblast comparison width mismatch"));
        }
        let mut equal = self.constant(true);
        let mut less = self.constant(false);
        for (left, right) in left.iter().zip(right).rev() {
            let less_here = self.and(left.not(), *right)?;
            let first_less = self.and(equal, less_here)?;
            less = self.or(less, first_less)?;
            let differs = self.xor(*left, *right)?;
            equal = self.and(equal, differs.not())?;
        }
        Ok((less, equal))
    }
}

struct Encoding {
    clauses: Vec<CnfClause>,
    variables: usize,
    input_bits: Vec<Vec<CnfLit>>,
}

fn require_width(bits: &[CnfLit], width: usize, label: &str) -> Result<(), Btor2BitblastError> {
    if bits.len() != width {
        return Err(reject(format!("BTOR2 bitblast {label} width mismatch")));
    }
    Ok(())
}

fn encode_node(
    circuit: &mut Circuit,
    model: &Btor2Model,
    values: &BTreeMap<NodeId, Vec<CnfLit>>,
    id: NodeId,
) -> Result<Vec<CnfLit>, Btor2BitblastError> {
    let node = &model.nodes()[&id];
    let width = node.width as usize;
    let get = |dependency: NodeId| {
        values.get(&dependency).cloned().ok_or_else(|| {
            reject(format!(
                "BTOR2 bitblast dependency {dependency} is unavailable"
            ))
        })
    };
    let result = match node.kind {
        NodeKind::Input | NodeKind::State => unreachable!("allocated before expression encoding"),
        NodeKind::Constant(value) => (0..width)
            .map(|bit| circuit.constant((value >> bit) & 1 != 0))
            .collect(),
        NodeKind::Unary(operation, value) => {
            let value = get(value)?;
            match operation {
                UnaryOp::Not => value.into_iter().map(CnfLit::not).collect(),
                UnaryOp::Inc => {
                    let mut one = vec![circuit.constant(false); width];
                    one[0] = circuit.constant(true);
                    circuit.add_bits(&value, &one)?
                }
                UnaryOp::Dec => {
                    let mut one = vec![circuit.constant(false); width];
                    one[0] = circuit.constant(true);
                    circuit.subtract_bits(&value, &one)?
                }
                UnaryOp::Neg => {
                    let zero = vec![circuit.constant(false); width];
                    circuit.subtract_bits(&zero, &value)?
                }
                UnaryOp::Redor => vec![circuit.or_all(&value)?],
                UnaryOp::Redand => vec![circuit.and_all(&value)?],
            }
        }
        NodeKind::Binary(operation, left, right) => {
            let left = get(left)?;
            let right = get(right)?;
            match operation {
                BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => {
                    require_width(&left, width, "binary left")?;
                    require_width(&right, width, "binary right")?;
                    left.iter()
                        .zip(&right)
                        .map(|(left, right)| match operation {
                            BinaryOp::And => circuit.and(*left, *right),
                            BinaryOp::Or => circuit.or(*left, *right),
                            BinaryOp::Xor => circuit.xor(*left, *right),
                            _ => unreachable!(),
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
                BinaryOp::Add => circuit.add_bits(&left, &right)?,
                BinaryOp::Sub => circuit.subtract_bits(&left, &right)?,
                BinaryOp::Mul => circuit.multiply_bits(&left, &right)?,
                BinaryOp::Sll => circuit.shift_bits(&left, &right, true)?,
                BinaryOp::Srl => circuit.shift_bits(&left, &right, false)?,
                BinaryOp::Eq | BinaryOp::Neq => {
                    let equals = left
                        .iter()
                        .zip(&right)
                        .map(|(left, right)| circuit.xor(*left, *right).map(CnfLit::not))
                        .collect::<Result<Vec<_>, _>>()?;
                    let equals = circuit.and_all(&equals)?;
                    vec![if operation == BinaryOp::Eq {
                        equals
                    } else {
                        equals.not()
                    }]
                }
                BinaryOp::Ult | BinaryOp::Ulte | BinaryOp::Ugt | BinaryOp::Ugte => {
                    let (less, equal) = circuit.unsigned_compare(&left, &right)?;
                    vec![match operation {
                        BinaryOp::Ult => less,
                        BinaryOp::Ulte => circuit.or(less, equal)?,
                        BinaryOp::Ugt => circuit.or(less, equal)?.not(),
                        BinaryOp::Ugte => less.not(),
                        _ => unreachable!(),
                    }]
                }
            }
        }
        NodeKind::Ite(condition, when_true, when_false) => {
            let condition = get(condition)?;
            let when_true = get(when_true)?;
            let when_false = get(when_false)?;
            if condition.len() != 1 || when_true.len() != width || when_false.len() != width {
                return Err(reject("BTOR2 bitblast ite width mismatch"));
            }
            when_true
                .iter()
                .zip(&when_false)
                .map(|(when_true, when_false)| circuit.ite(condition[0], *when_true, *when_false))
                .collect::<Result<Vec<_>, _>>()?
        }
        NodeKind::Slice {
            value,
            upper,
            lower,
        } => get(value)?[lower as usize..=upper as usize].to_vec(),
        NodeKind::Uext { value, amount } => {
            let mut value = get(value)?;
            value.extend((0..amount).map(|_| circuit.constant(false)));
            value
        }
        NodeKind::Concat { high, low } => {
            let mut result = get(low)?;
            result.extend(get(high)?);
            result
        }
    };
    require_width(&result, width, "result")?;
    Ok(result)
}

fn encode(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<Encoding, Btor2BitblastError> {
    if horizon > MAX_BITBLAST_HORIZON {
        return Err(reject("BTOR2 bitblast horizon exceeds policy"));
    }
    let model = btor2::parse_bytes(source)
        .map_err(|error| reject(format!("invalid BTOR2 bitblast source: {error}")))?;
    let bad_expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))
        .ok_or_else(|| reject("BTOR2 bitblast bad property is unavailable"))?;
    let input_bits = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].width as usize)
        .sum::<usize>();
    if input_bits > MAX_BITBLAST_INPUT_BITS {
        return Err(reject("BTOR2 bitblast input width exceeds policy"));
    }
    let mut circuit = Circuit::new();
    let mut frames = Vec::<BTreeMap<NodeId, Vec<CnfLit>>>::new();
    let mut frame_inputs = Vec::new();
    for _frame in 0..=horizon {
        let mut values = BTreeMap::new();
        let mut packed_inputs = Vec::with_capacity(input_bits);
        for (&id, node) in model.nodes() {
            match node.kind {
                NodeKind::Input => {
                    let bits = (0..node.width)
                        .map(|_| circuit.variable())
                        .collect::<Result<Vec<_>, _>>()?;
                    packed_inputs.extend_from_slice(&bits);
                    values.insert(id, bits);
                }
                NodeKind::State => {
                    values.insert(
                        id,
                        (0..node.width)
                            .map(|_| circuit.variable())
                            .collect::<Result<Vec<_>, _>>()?,
                    );
                }
                _ => {
                    let bits = encode_node(&mut circuit, &model, &values, id)?;
                    values.insert(id, bits);
                }
            }
        }
        for (_, constraint) in model.constraints() {
            let expression = &values[constraint];
            if expression.len() != 1 {
                return Err(reject("BTOR2 bitblast constraint is not Boolean"));
            }
            circuit.clause(expression)?;
        }
        frames.push(values);
        frame_inputs.push(packed_inputs);
    }
    for state in model.states() {
        let initialiser = model
            .initialiser(*state)
            .ok_or_else(|| reject(format!("BTOR2 bitblast state {state} lacks initialiser")))?;
        for (state_bit, value_bit) in frames[0][state].iter().zip(&frames[0][&initialiser]) {
            circuit.equal(*state_bit, *value_bit)?;
        }
    }
    for frame in 0..horizon as usize {
        for state in model.states() {
            let next = model
                .next_value(*state)
                .ok_or_else(|| reject(format!("BTOR2 bitblast state {state} lacks next value")))?;
            for (state_bit, value_bit) in frames[frame + 1][state].iter().zip(&frames[frame][&next])
            {
                circuit.equal(*state_bit, *value_bit)?;
            }
        }
    }
    let bad = frames
        .iter()
        .map(|frame| {
            let expression = &frame[&bad_expression];
            if expression.len() != 1 {
                return Err(reject("BTOR2 bitblast bad property is not Boolean"));
            }
            Ok(expression[0])
        })
        .collect::<Result<Vec<_>, _>>()?;
    circuit.clause(&bad)?;
    Ok(Encoding {
        clauses: circuit.clauses,
        variables: circuit.variables,
        input_bits: frame_inputs,
    })
}

fn solve(encoding: &Encoding) -> Result<Option<Vec<bool>>, Btor2BitblastError> {
    let mut solver = Solver::new();
    for clause in &encoding.clauses {
        let literals = clause
            .0
            .iter()
            .map(|(variable, positive)| SolverLit::from_var(Var::from_index(*variable), *positive))
            .collect::<Vec<_>>();
        solver.add_clause(&literals);
    }
    if !solver
        .solve()
        .map_err(|error| reject(format!("BTOR2 bitblast solve failed: {error}")))?
    {
        return Ok(None);
    }
    let mut assignment = vec![false; encoding.variables];
    for literal in solver
        .model()
        .ok_or_else(|| reject("BTOR2 bitblast SAT result lacks a model"))?
    {
        assignment[literal.var().index()] = literal.is_positive();
    }
    Ok(Some(assignment))
}

fn extract_valuations(encoding: &Encoding, assignment: &[bool]) -> Vec<u64> {
    encoding
        .input_bits
        .iter()
        .map(|bits| {
            bits.iter().enumerate().fold(0u64, |value, (bit, input)| {
                value | (u64::from(assignment[input.variable] == input.positive) << bit)
            })
        })
        .collect()
}

fn input_values(model: &Btor2Model, valuation: u64) -> Result<WordValues, Btor2BitblastError> {
    let mut offset = 0usize;
    let mut result = WordValues::new();
    for input in model.inputs() {
        let width = model.nodes()[input].width as usize;
        if offset + width > MAX_BITBLAST_INPUT_BITS {
            return Err(reject("BTOR2 bitblast witness input width exceeds policy"));
        }
        let mask = if width == 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        result.insert(*input, (valuation >> offset) & mask);
        offset += width;
    }
    if offset < 64 && valuation >> offset != 0 {
        return Err(reject("BTOR2 bitblast witness valuation is noncanonical"));
    }
    Ok(result)
}

fn replay_witness(
    source: &[u8],
    bad_property: NodeId,
    valuations: &[u64],
    horizon: u32,
) -> Result<u32, Btor2BitblastError> {
    if valuations.len() != horizon as usize + 1 {
        return Err(reject("BTOR2 bitblast witness frame count mismatch"));
    }
    let model = btor2::parse_bytes(source)
        .map_err(|error| reject(format!("invalid BTOR2 bitblast witness source: {error}")))?;
    let mut state = model
        .initial_state()
        .map_err(|error| reject(format!("BTOR2 bitblast initial state failed: {error}")))?;
    for (frame, valuation) in valuations.iter().enumerate() {
        let inputs = input_values(&model, *valuation)?;
        for (_, constraint) in model.constraints() {
            if model
                .evaluate(*constraint, &state, &inputs)
                .map_err(|error| reject(format!("BTOR2 bitblast constraint failed: {error}")))?
                == 0
            {
                return Err(reject("BTOR2 bitblast witness violates a constraint"));
            }
        }
        if model
            .active_bad(&state, &inputs)
            .map_err(|error| reject(format!("BTOR2 bitblast bad replay failed: {error}")))?
            .contains(&bad_property)
        {
            return u32::try_from(frame)
                .map_err(|_| reject("BTOR2 bitblast bad frame exceeds range"));
        }
        if frame < horizon as usize {
            state = model
                .step(&state, &inputs)
                .map_err(|error| reject(format!("BTOR2 bitblast witness step failed: {error}")))?;
        }
    }
    Err(reject(
        "BTOR2 bitblast witness does not reach the bad property",
    ))
}

pub fn produce_btor2_bitblast_certificate(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<Btor2BitblastCertificate, Btor2BitblastError> {
    let encoding = encode(source, bad_property, horizon)?;
    if let Some(assignment) = solve(&encoding)? {
        let valuations = extract_valuations(&encoding, &assignment);
        let bad_frame = replay_witness(source, bad_property, &valuations, horizon)?;
        return Ok(Btor2BitblastCertificate {
            version: BTOR2_BITBLAST_VERSION,
            source_sha256: Sha256::digest(source).into(),
            bad_property,
            horizon,
            result: SearchResult::Unsafe,
            bad_frame: Some(bad_frame),
            witness_valuations: valuations,
            unsat_proof: Vec::new(),
        });
    }
    let proof = unsat_proof::generate_unsat_proof(&encoding.clauses)
        .map_err(|error| reject(format!("BTOR2 bitblast proof generation failed: {error}")))?;
    Ok(Btor2BitblastCertificate {
        version: BTOR2_BITBLAST_VERSION,
        source_sha256: Sha256::digest(source).into(),
        bad_property,
        horizon,
        result: SearchResult::Safe,
        bad_frame: None,
        witness_valuations: Vec::new(),
        unsat_proof: proof,
    })
}

pub fn verify_btor2_bitblast_certificate(
    source: &[u8],
    certificate: &Btor2BitblastCertificate,
) -> Result<Btor2BitblastSummary, Btor2BitblastError> {
    if certificate.version != BTOR2_BITBLAST_VERSION
        || certificate.source_sha256 != <[u8; 32]>::from(Sha256::digest(source))
        || certificate.horizon > MAX_BITBLAST_HORIZON
    {
        return Err(reject("BTOR2 bitblast certificate binding mismatch"));
    }
    let encoding = encode(source, certificate.bad_property, certificate.horizon)?;
    match certificate.result {
        SearchResult::Safe => {
            if certificate.bad_frame.is_some()
                || !certificate.witness_valuations.is_empty()
                || certificate.unsat_proof.is_empty()
            {
                return Err(reject("BTOR2 bitblast SAFE evidence is not canonical"));
            }
            unsat_proof::verify_unsat_proof(&encoding.clauses, &certificate.unsat_proof)
                .map_err(|error| reject(format!("BTOR2 bitblast proof failed: {error}")))?;
        }
        SearchResult::Unsafe => {
            if !certificate.unsat_proof.is_empty() {
                return Err(reject("BTOR2 bitblast UNSAFE evidence contains a proof"));
            }
            let bad_frame = replay_witness(
                source,
                certificate.bad_property,
                &certificate.witness_valuations,
                certificate.horizon,
            )?;
            if certificate.bad_frame != Some(bad_frame) {
                return Err(reject("BTOR2 bitblast bad frame mismatch"));
            }
        }
    }
    Ok(Btor2BitblastSummary {
        version: BTOR2_BITBLAST_VERSION,
        result: certificate.result,
        bad_frame: certificate.bad_frame,
        variables: encoding.variables,
        clauses: encoding.clauses.len(),
        proof_bytes: certificate.unsat_proof.len(),
    })
}

pub fn encode_btor2_bitblast_certificate(
    certificate: &Btor2BitblastCertificate,
) -> Result<Vec<u8>, Btor2BitblastError> {
    if certificate.version != BTOR2_BITBLAST_VERSION
        || certificate.horizon > MAX_BITBLAST_HORIZON
        || certificate.witness_valuations.len() > MAX_BITBLAST_HORIZON as usize + 1
        || certificate.unsat_proof.len() > unsat_proof::MAX_UNSAT_PROOF_BYTES
    {
        return Err(reject("BTOR2 bitblast certificate is outside policy"));
    }
    match certificate.result {
        SearchResult::Safe
            if certificate.bad_frame.is_some()
                || !certificate.witness_valuations.is_empty()
                || certificate.unsat_proof.is_empty() =>
        {
            return Err(reject("BTOR2 bitblast SAFE certificate is not canonical"));
        }
        SearchResult::Unsafe
            if certificate.bad_frame.is_none()
                || certificate.bad_frame > Some(certificate.horizon)
                || certificate.witness_valuations.len() != certificate.horizon as usize + 1
                || !certificate.unsat_proof.is_empty() =>
        {
            return Err(reject("BTOR2 bitblast UNSAFE certificate is not canonical"));
        }
        _ => {}
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(BITBLAST_MAGIC);
    bytes.extend_from_slice(&certificate.version.to_le_bytes());
    bytes.extend_from_slice(&certificate.source_sha256);
    bytes.extend_from_slice(&certificate.bad_property.to_le_bytes());
    bytes.extend_from_slice(&certificate.horizon.to_le_bytes());
    bytes.push(match certificate.result {
        SearchResult::Safe => 0,
        SearchResult::Unsafe => 1,
    });
    bytes.extend_from_slice(&certificate.bad_frame.unwrap_or(u32::MAX).to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(certificate.witness_valuations.len())
            .map_err(|_| reject("BTOR2 bitblast witness count exceeds range"))?
            .to_le_bytes(),
    );
    for valuation in &certificate.witness_valuations {
        bytes.extend_from_slice(&valuation.to_le_bytes());
    }
    bytes.extend_from_slice(
        &u32::try_from(certificate.unsat_proof.len())
            .map_err(|_| reject("BTOR2 bitblast proof length exceeds range"))?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&certificate.unsat_proof);
    let checksum: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_BITBLAST_CERTIFICATE_BYTES {
        return Err(reject("BTOR2 bitblast certificate exceeds byte policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Btor2BitblastError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("BTOR2 bitblast certificate offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("truncated BTOR2 bitblast certificate"))?;
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, Btor2BitblastError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed u32"),
        ))
    }

    fn u64(&mut self) -> Result<u64, Btor2BitblastError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed u64"),
        ))
    }
}

pub fn decode_btor2_bitblast_certificate(
    bytes: &[u8],
) -> Result<Btor2BitblastCertificate, Btor2BitblastError> {
    if bytes.len() < 8 + 4 * 5 + 8 + 32 * 2 + 1 || bytes.len() > MAX_BITBLAST_CERTIFICATE_BYTES {
        return Err(reject("BTOR2 bitblast certificate size is outside policy"));
    }
    let payload_end = bytes.len() - 32;
    let checksum: [u8; 32] = bytes[payload_end..].try_into().expect("fixed checksum");
    if <[u8; 32]>::from(Sha256::digest(&bytes[..payload_end])) != checksum {
        return Err(reject("BTOR2 bitblast certificate checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..payload_end],
        offset: 0,
    };
    if cursor.take(8)? != BITBLAST_MAGIC {
        return Err(reject("BTOR2 bitblast certificate magic mismatch"));
    }
    let version = cursor.u32()?;
    let source_sha256 = cursor.take(32)?.try_into().expect("fixed digest");
    let bad_property = cursor.u64()?;
    let horizon = cursor.u32()?;
    let result = match cursor.take(1)?[0] {
        0 => SearchResult::Safe,
        1 => SearchResult::Unsafe,
        _ => return Err(reject("BTOR2 bitblast result tag is invalid")),
    };
    let bad_frame = match cursor.u32()? {
        u32::MAX => None,
        frame => Some(frame),
    };
    let valuation_count = usize::try_from(cursor.u32()?)
        .map_err(|_| reject("BTOR2 bitblast witness count exceeds platform"))?;
    if valuation_count > MAX_BITBLAST_HORIZON as usize + 1 {
        return Err(reject("BTOR2 bitblast witness count exceeds policy"));
    }
    let mut witness_valuations = Vec::with_capacity(valuation_count);
    for _ in 0..valuation_count {
        witness_valuations.push(cursor.u64()?);
    }
    let proof_length = usize::try_from(cursor.u32()?)
        .map_err(|_| reject("BTOR2 bitblast proof length exceeds platform"))?;
    if proof_length > unsat_proof::MAX_UNSAT_PROOF_BYTES {
        return Err(reject("BTOR2 bitblast proof length exceeds policy"));
    }
    let unsat_proof = cursor.take(proof_length)?.to_vec();
    if cursor.offset != payload_end {
        return Err(reject("trailing BTOR2 bitblast certificate bytes"));
    }
    let certificate = Btor2BitblastCertificate {
        version,
        source_sha256,
        bad_property,
        horizon,
        result,
        bad_frame,
        witness_valuations,
        unsat_proof,
    };
    if encode_btor2_bitblast_certificate(&certificate)? != bytes {
        return Err(reject("BTOR2 bitblast certificate is not canonical"));
    }
    Ok(certificate)
}
