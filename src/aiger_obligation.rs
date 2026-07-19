//! Exact, resource-bounded AIGER transition evaluation and CNF obligations.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::unsat_proof::CnfClause;

pub const AIGER_OBLIGATION_VERSION: u32 = 1;
pub const MAX_AIGER_VARIABLES: usize = 1_000_000;
pub const MAX_AIGER_INPUTS: usize = 64;
pub const MAX_AIGER_LATCHES: usize = 8;
pub const MAX_AIGER_OUTPUTS: usize = 128;
pub const MAX_PREDICATE_CLAUSES: usize = 64;
pub const MAX_PREDICATE_LITERALS: usize = 1_024;
pub const MAX_ASCII_AIGER_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AigerLatch {
    pub current: usize,
    pub next: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AigerAnd {
    pub output: usize,
    pub left: usize,
    pub right: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AigerTransition {
    pub max_variable: usize,
    pub inputs: Vec<usize>,
    pub latches: Vec<AigerLatch>,
    pub outputs: Vec<usize>,
    pub ands: Vec<AigerAnd>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AigerInputPredicate {
    pub clauses: Vec<Vec<(usize, bool)>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AigerOutcome {
    pub target: usize,
    pub outputs: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AigerObligationError(pub String);

impl fmt::Display for AigerObligationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for AigerObligationError {}

fn reject(message: impl Into<String>) -> AigerObligationError {
    AigerObligationError(message.into())
}

#[derive(Clone, Copy)]
enum CnfLiteral {
    Constant(bool),
    Variable((usize, bool)),
}

impl CnfLiteral {
    fn negate(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Variable((variable, positive)) => Self::Variable((variable, !positive)),
        }
    }
}

impl AigerTransition {
    pub fn validate(&self) -> Result<(), AigerObligationError> {
        if self.max_variable == 0 || self.max_variable > MAX_AIGER_VARIABLES {
            return Err(reject("AIGER obligation variable count is outside limit"));
        }
        if self.inputs.len() > MAX_AIGER_INPUTS
            || self.latches.is_empty()
            || self.latches.len() > MAX_AIGER_LATCHES
            || self.outputs.len() > MAX_AIGER_OUTPUTS
        {
            return Err(reject("AIGER obligation dimensions are outside limits"));
        }
        let expected = self
            .inputs
            .len()
            .checked_add(self.latches.len())
            .and_then(|count| count.checked_add(self.ands.len()))
            .ok_or_else(|| reject("AIGER obligation definition count overflow"))?;
        if expected != self.max_variable {
            return Err(reject("AIGER obligation definition count mismatch"));
        }
        let literal_limit = self
            .max_variable
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| reject("AIGER obligation literal range overflow"))?;
        let mut definitions = BTreeSet::new();
        for &literal in &self.inputs {
            if literal == 0
                || !literal.is_multiple_of(2)
                || literal > literal_limit
                || !definitions.insert(literal / 2)
            {
                return Err(reject("invalid AIGER obligation input definition"));
            }
        }
        for latch in &self.latches {
            if latch.current == 0
                || !latch.current.is_multiple_of(2)
                || latch.current > literal_limit
                || latch.next > literal_limit
                || !definitions.insert(latch.current / 2)
            {
                return Err(reject("invalid AIGER obligation latch definition"));
            }
        }
        for gate in &self.ands {
            if gate.output == 0
                || !gate.output.is_multiple_of(2)
                || gate.output > literal_limit
                || gate.left > literal_limit
                || gate.right > literal_limit
                || gate.left / 2 >= gate.output / 2
                || gate.right / 2 >= gate.output / 2
                || !definitions.insert(gate.output / 2)
            {
                return Err(reject("invalid AIGER obligation AND definition"));
            }
        }
        if definitions.len() != self.max_variable
            || self
                .latches
                .iter()
                .map(|latch| latch.next)
                .chain(self.outputs.iter().copied())
                .chain(self.ands.iter().flat_map(|gate| [gate.left, gate.right]))
                .any(|literal| {
                    literal > literal_limit
                        || (literal >= 2 && !definitions.contains(&(literal / 2)))
                })
        {
            return Err(reject("AIGER obligation references an undefined variable"));
        }
        Ok(())
    }

    pub fn state_count(&self) -> usize {
        1usize << self.latches.len()
    }

    pub fn evaluate(
        &self,
        state: usize,
        declared_input: u64,
    ) -> Result<(usize, u128), AigerObligationError> {
        self.validate()?;
        if state >= self.state_count()
            || (self.inputs.len() < u64::BITS as usize && declared_input >> self.inputs.len() != 0)
        {
            return Err(reject("AIGER evaluation assignment is outside dimensions"));
        }
        let mut values = vec![false; self.max_variable + 1];
        for (bit, latch) in self.latches.iter().enumerate() {
            values[latch.current / 2] = state >> bit & 1 == 1;
        }
        for (bit, literal) in self.inputs.iter().enumerate() {
            values[literal / 2] = declared_input >> bit & 1 == 1;
        }
        for gate in &self.ands {
            values[gate.output / 2] =
                evaluate_literal(gate.left, &values) && evaluate_literal(gate.right, &values);
        }
        let next = self
            .latches
            .iter()
            .enumerate()
            .fold(0usize, |next, (bit, latch)| {
                next | (usize::from(evaluate_literal(latch.next, &values)) << bit)
            });
        let outputs = self
            .outputs
            .iter()
            .enumerate()
            .fold(0u128, |outputs, (bit, &literal)| {
                outputs | (u128::from(evaluate_literal(literal, &values)) << bit)
            });
        Ok((next, outputs))
    }
}

fn parse_usize(token: Option<&str>, field: &str) -> Result<usize, AigerObligationError> {
    token
        .ok_or_else(|| reject(format!("missing ASCII AIGER {field}")))?
        .parse()
        .map_err(|_| reject(format!("invalid ASCII AIGER {field}")))
}

/// Parses the five-field ASCII AIGER transition subset used by proof APIs.
///
/// Optional latch initializers are syntax-checked but are not part of the
/// transition relation. Callers supply initial states to bounded composition.
pub fn parse_ascii_aiger_transition(bytes: &[u8]) -> Result<AigerTransition, AigerObligationError> {
    if bytes.len() > MAX_ASCII_AIGER_BYTES || !bytes.is_ascii() {
        return Err(reject("ASCII AIGER input size or encoding is invalid"));
    }
    let source = std::str::from_utf8(bytes).map_err(|_| reject("ASCII AIGER is not UTF-8"))?;
    let mut lines = source.lines();
    let mut header = lines
        .next()
        .ok_or_else(|| reject("ASCII AIGER input is empty"))?
        .split_whitespace();
    if header.next() != Some("aag") {
        return Err(reject("ASCII AIGER magic mismatch"));
    }
    let max_variable = parse_usize(header.next(), "maximum variable")?;
    let input_count = parse_usize(header.next(), "input count")?;
    let latch_count = parse_usize(header.next(), "latch count")?;
    let output_count = parse_usize(header.next(), "output count")?;
    let and_count = parse_usize(header.next(), "AND count")?;
    if header.next().is_some()
        || input_count > MAX_AIGER_INPUTS
        || latch_count == 0
        || latch_count > MAX_AIGER_LATCHES
        || output_count > MAX_AIGER_OUTPUTS
        || input_count
            .checked_add(latch_count)
            .and_then(|count| count.checked_add(and_count))
            != Some(max_variable)
    {
        return Err(reject("ASCII AIGER header dimensions are invalid"));
    }

    let mut inputs = Vec::with_capacity(input_count);
    for index in 0..input_count {
        let mut fields = lines
            .next()
            .ok_or_else(|| reject(format!("truncated ASCII AIGER input {index}")))?
            .split_whitespace();
        let literal = parse_usize(fields.next(), "input literal")?;
        if fields.next().is_some() {
            return Err(reject("ASCII AIGER input line has trailing fields"));
        }
        inputs.push(literal);
    }
    let mut latches = Vec::with_capacity(latch_count);
    for index in 0..latch_count {
        let fields = lines
            .next()
            .ok_or_else(|| reject(format!("truncated ASCII AIGER latch {index}")))?
            .split_whitespace()
            .collect::<Vec<_>>();
        if !(2..=3).contains(&fields.len()) {
            return Err(reject("ASCII AIGER latch line shape is invalid"));
        }
        let current = parse_usize(fields.first().copied(), "latch current literal")?;
        let next = parse_usize(fields.get(1).copied(), "latch next literal")?;
        if let Some(initial) = fields.get(2).copied()
            && initial != "0"
            && initial != "1"
            && initial.parse::<usize>().ok() != Some(current)
        {
            return Err(reject("ASCII AIGER latch initializer is invalid"));
        }
        latches.push(AigerLatch { current, next });
    }
    let mut outputs = Vec::with_capacity(output_count);
    for index in 0..output_count {
        let mut fields = lines
            .next()
            .ok_or_else(|| reject(format!("truncated ASCII AIGER output {index}")))?
            .split_whitespace();
        let literal = parse_usize(fields.next(), "output literal")?;
        if fields.next().is_some() {
            return Err(reject("ASCII AIGER output line has trailing fields"));
        }
        outputs.push(literal);
    }
    let mut ands = Vec::with_capacity(and_count);
    for index in 0..and_count {
        let mut fields = lines
            .next()
            .ok_or_else(|| reject(format!("truncated ASCII AIGER AND {index}")))?
            .split_whitespace();
        let output = parse_usize(fields.next(), "AND output literal")?;
        let left = parse_usize(fields.next(), "AND left literal")?;
        let right = parse_usize(fields.next(), "AND right literal")?;
        if fields.next().is_some() {
            return Err(reject("ASCII AIGER AND line has trailing fields"));
        }
        ands.push(AigerAnd {
            output,
            left,
            right,
        });
    }
    let transition = AigerTransition {
        max_variable,
        inputs,
        latches,
        outputs,
        ands,
    };
    transition.validate()?;
    Ok(transition)
}

fn evaluate_literal(literal: usize, values: &[bool]) -> bool {
    if literal < 2 {
        return literal == 1;
    }
    let value = values[literal / 2];
    if literal & 1 == 1 { !value } else { value }
}

fn cnf_literal(literal: usize) -> CnfLiteral {
    if literal < 2 {
        CnfLiteral::Constant(literal == 1)
    } else {
        CnfLiteral::Variable((literal / 2 - 1, literal & 1 == 0))
    }
}

fn push_simplified(clauses: &mut Vec<CnfClause>, literals: &[CnfLiteral]) {
    let mut clause = Vec::with_capacity(literals.len());
    for &literal in literals {
        match literal {
            CnfLiteral::Constant(true) => return,
            CnfLiteral::Constant(false) => {}
            CnfLiteral::Variable(literal) => clause.push(literal),
        }
    }
    clause.sort_unstable();
    clause.dedup();
    if clause
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && pair[0].1 != pair[1].1)
    {
        return;
    }
    clauses.push(CnfClause(clause));
}

fn validate_boundary(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    predicate: &AigerInputPredicate,
) -> Result<(), AigerObligationError> {
    model.validate()?;
    if relevant_inputs.len() > MAX_AIGER_INPUTS
        || relevant_inputs
            .iter()
            .any(|&input| input >= model.inputs.len())
        || relevant_inputs.windows(2).any(|pair| pair[0] >= pair[1])
        || predicate.clauses.len() > MAX_PREDICATE_CLAUSES
    {
        return Err(reject("AIGER obligation boundary is invalid"));
    }
    let literal_count = predicate
        .clauses
        .iter()
        .try_fold(0usize, |count, clause| count.checked_add(clause.len()))
        .ok_or_else(|| reject("AIGER predicate literal count overflow"))?;
    if literal_count > MAX_PREDICATE_LITERALS
        || predicate
            .clauses
            .iter()
            .flatten()
            .any(|&(input, _)| input >= relevant_inputs.len())
    {
        return Err(reject("AIGER predicate is outside limits"));
    }
    Ok(())
}

fn append_predicate(
    clauses: &mut Vec<CnfClause>,
    model: &AigerTransition,
    relevant_inputs: &[usize],
    predicate: &AigerInputPredicate,
) {
    for clause in &predicate.clauses {
        let mapped = clause
            .iter()
            .map(|&(projected, positive)| {
                CnfLiteral::Variable((model.inputs[relevant_inputs[projected]] / 2 - 1, positive))
            })
            .collect::<Vec<_>>();
        push_simplified(clauses, &mapped);
    }
}

fn append_ands(clauses: &mut Vec<CnfClause>, model: &AigerTransition) {
    for gate in &model.ands {
        let output = cnf_literal(gate.output);
        let left = cnf_literal(gate.left);
        let right = cnf_literal(gate.right);
        push_simplified(clauses, &[output.negate(), left]);
        push_simplified(clauses, &[output.negate(), right]);
        push_simplified(clauses, &[output, left.negate(), right.negate()]);
    }
}

pub fn relation_row_completeness_cnf(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    source: usize,
    predicate: &AigerInputPredicate,
    claimed_targets: &[usize],
) -> Result<Vec<CnfClause>, AigerObligationError> {
    validate_boundary(model, relevant_inputs, predicate)?;
    if source >= model.state_count()
        || claimed_targets
            .iter()
            .any(|&target| target >= model.state_count())
        || claimed_targets.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("AIGER relation-row claim is invalid"));
    }
    let mut clauses = Vec::with_capacity(
        model.ands.len() * 3
            + model.latches.len()
            + predicate.clauses.len()
            + claimed_targets.len(),
    );
    for (bit, latch) in model.latches.iter().enumerate() {
        push_simplified(
            &mut clauses,
            &[CnfLiteral::Variable((
                latch.current / 2 - 1,
                source >> bit & 1 == 1,
            ))],
        );
    }
    append_predicate(&mut clauses, model, relevant_inputs, predicate);
    append_ands(&mut clauses, model);
    for &target in claimed_targets {
        let differs = model
            .latches
            .iter()
            .enumerate()
            .map(|(bit, latch)| {
                let next = cnf_literal(latch.next);
                if target >> bit & 1 == 1 {
                    next.negate()
                } else {
                    next
                }
            })
            .collect::<Vec<_>>();
        push_simplified(&mut clauses, &differs);
    }
    Ok(clauses)
}

pub fn transducer_row_completeness_cnf(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    source: usize,
    predicate: &AigerInputPredicate,
    observed_outputs: &[usize],
    claimed_outcomes: &[AigerOutcome],
) -> Result<Vec<CnfClause>, AigerObligationError> {
    validate_boundary(model, relevant_inputs, predicate)?;
    if source >= model.state_count()
        || observed_outputs.len() > u128::BITS as usize
        || observed_outputs
            .iter()
            .any(|&output| output >= model.outputs.len())
        || observed_outputs.windows(2).any(|pair| pair[0] >= pair[1])
        || claimed_outcomes
            .iter()
            .any(|outcome| outcome.target >= model.state_count())
        || claimed_outcomes.windows(2).any(|pair| pair[0] >= pair[1])
        || (observed_outputs.len() < u128::BITS as usize
            && claimed_outcomes
                .iter()
                .any(|outcome| outcome.outputs >> observed_outputs.len() != 0))
    {
        return Err(reject("AIGER transducer-row claim is invalid"));
    }
    let mut clauses = Vec::with_capacity(
        model.ands.len() * 3
            + model.latches.len()
            + predicate.clauses.len()
            + claimed_outcomes.len(),
    );
    for (bit, latch) in model.latches.iter().enumerate() {
        push_simplified(
            &mut clauses,
            &[CnfLiteral::Variable((
                latch.current / 2 - 1,
                source >> bit & 1 == 1,
            ))],
        );
    }
    append_predicate(&mut clauses, model, relevant_inputs, predicate);
    append_ands(&mut clauses, model);
    for outcome in claimed_outcomes {
        let mut differs = model
            .latches
            .iter()
            .enumerate()
            .map(|(bit, latch)| {
                let next = cnf_literal(latch.next);
                if outcome.target >> bit & 1 == 1 {
                    next.negate()
                } else {
                    next
                }
            })
            .collect::<Vec<_>>();
        differs.extend(observed_outputs.iter().enumerate().map(|(bit, &output)| {
            let value = cnf_literal(model.outputs[output]);
            if outcome.outputs >> bit & 1 == 1 {
                value.negate()
            } else {
                value
            }
        }));
        push_simplified(&mut clauses, &differs);
    }
    Ok(clauses)
}

pub fn terminal_completeness_cnf(
    model: &AigerTransition,
    relevant_inputs: &[usize],
    predicate: &AigerInputPredicate,
    bad_output: usize,
    claimed_safe_states: &[usize],
) -> Result<Vec<CnfClause>, AigerObligationError> {
    validate_boundary(model, relevant_inputs, predicate)?;
    if bad_output >= model.outputs.len()
        || claimed_safe_states
            .iter()
            .any(|&state| state >= model.state_count())
        || claimed_safe_states
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(reject("AIGER terminal claim is invalid"));
    }
    let mut clauses = Vec::with_capacity(
        model.ands.len() * 3 + predicate.clauses.len() + claimed_safe_states.len() + 1,
    );
    append_predicate(&mut clauses, model, relevant_inputs, predicate);
    append_ands(&mut clauses, model);
    for &state in claimed_safe_states {
        let differs = model
            .latches
            .iter()
            .enumerate()
            .map(|(bit, latch)| {
                let current = cnf_literal(latch.current);
                if state >> bit & 1 == 1 {
                    current.negate()
                } else {
                    current
                }
            })
            .collect::<Vec<_>>();
        push_simplified(&mut clauses, &differs);
    }
    push_simplified(
        &mut clauses,
        &[cnf_literal(model.outputs[bad_output]).negate()],
    );
    Ok(clauses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unsat_proof::{generate_unsat_proof, verify_unsat_proof};

    fn toggler() -> AigerTransition {
        AigerTransition {
            max_variable: 1,
            inputs: vec![],
            latches: vec![AigerLatch {
                current: 2,
                next: 3,
            }],
            outputs: vec![2],
            ands: vec![],
        }
    }

    #[test]
    fn evaluates_and_proves_a_complete_relation_row() {
        let model = toggler();
        assert_eq!(model.evaluate(0, 0).unwrap(), (1, 0));
        assert_eq!(model.evaluate(1, 0).unwrap(), (0, 1));
        let predicate = AigerInputPredicate { clauses: vec![] };
        let cnf = relation_row_completeness_cnf(&model, &[], 0, &predicate, &[1]).unwrap();
        let proof = generate_unsat_proof(&cnf).unwrap();
        verify_unsat_proof(&cnf, &proof).unwrap();
    }

    #[test]
    fn omitted_targets_and_invalid_models_fail_closed() {
        let model = toggler();
        let predicate = AigerInputPredicate { clauses: vec![] };
        let incomplete = relation_row_completeness_cnf(&model, &[], 0, &predicate, &[0]).unwrap();
        assert!(generate_unsat_proof(&incomplete).is_err());
        let mut invalid = model;
        invalid.latches[0].next = 8;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn transducer_obligation_preserves_observed_outputs() {
        let model = toggler();
        let predicate = AigerInputPredicate { clauses: vec![] };
        let complete = transducer_row_completeness_cnf(
            &model,
            &[],
            0,
            &predicate,
            &[0],
            &[AigerOutcome {
                target: 1,
                outputs: 0,
            }],
        )
        .unwrap();
        let proof = generate_unsat_proof(&complete).unwrap();
        verify_unsat_proof(&complete, &proof).unwrap();

        let wrong_output = transducer_row_completeness_cnf(
            &model,
            &[],
            0,
            &predicate,
            &[0],
            &[AigerOutcome {
                target: 1,
                outputs: 1,
            }],
        )
        .unwrap();
        assert!(generate_unsat_proof(&wrong_output).is_err());
    }
}
