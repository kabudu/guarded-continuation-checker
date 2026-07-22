//! Deterministic safety-witness composition for the FM 2026 baseline.
//!
//! This is a closest-prior-art baseline implementation, not a GCC novelty
//! claim. Version 1 accepts bounded ASCII AIGER safety witnesses, coalesces
//! variables explicitly mapped to the same model input or latch, keeps private
//! variables disjoint, hash-conses gates, and conjoins safety and constraint
//! semantics. Liveness and comment-based mappings fail closed.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const COMPOSED_WITNESS_BASELINE_VERSION: u32 = 1;
pub const MAX_COMPOSED_WITNESSES: usize = 64;
pub const MAX_COMPOSED_AIGER_BYTES: usize = 16 * 1024 * 1024;
const MAX_VARIABLES: usize = 2_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposedWitnessError(pub String);

impl fmt::Display for ComposedWitnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ComposedWitnessError {}

fn reject(message: impl Into<String>) -> ComposedWitnessError {
    ComposedWitnessError(message.into())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum VariableKind {
    Input,
    Latch,
    Gate,
}

#[derive(Clone, Debug)]
struct Latch {
    current: usize,
    next: usize,
    reset: usize,
}

#[derive(Clone, Debug)]
struct Gate {
    output: usize,
    left: usize,
    right: usize,
}

#[derive(Clone, Debug)]
struct Aag {
    max_variable: usize,
    inputs: Vec<usize>,
    latches: Vec<Latch>,
    outputs: Vec<usize>,
    bads: Vec<usize>,
    constraints: Vec<usize>,
    gates: Vec<Gate>,
    input_names: BTreeMap<usize, String>,
    latch_names: BTreeMap<usize, String>,
}

fn parse_number(value: Option<&str>, field: &str) -> Result<usize, ComposedWitnessError> {
    let text = value.ok_or_else(|| reject(format!("missing AIGER {field}")))?;
    if text.is_empty() || (text.len() > 1 && text.starts_with('0')) {
        return Err(reject(format!("noncanonical AIGER {field}")));
    }
    text.parse()
        .map_err(|_| reject(format!("invalid AIGER {field}")))
}

fn take_literal(
    lines: &mut std::str::Lines<'_>,
    field: &str,
) -> Result<usize, ComposedWitnessError> {
    let line = lines
        .next()
        .ok_or_else(|| reject(format!("truncated AIGER {field}")))?;
    let mut fields = line.split_whitespace();
    let literal = parse_number(fields.next(), field)?;
    if fields.next().is_some() {
        return Err(reject(format!("AIGER {field} has trailing fields")));
    }
    Ok(literal)
}

fn parse_aag(bytes: &[u8], witness: bool) -> Result<Aag, ComposedWitnessError> {
    if bytes.is_empty()
        || bytes.len() > MAX_COMPOSED_AIGER_BYTES
        || !bytes.is_ascii()
        || bytes.contains(&b'\r')
        || bytes.last() != Some(&b'\n')
    {
        return Err(reject(
            "ASCII AIGER bytes are noncanonical or outside limits",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("ASCII AIGER is not UTF-8"))?;
    let mut lines = text.lines();
    let mut header = lines
        .next()
        .ok_or_else(|| reject("ASCII AIGER is empty"))?
        .split_whitespace();
    if header.next() != Some("aag") {
        return Err(reject("only ASCII AIGER is supported"));
    }
    let max_variable = parse_number(header.next(), "maximum variable")?;
    let input_count = parse_number(header.next(), "input count")?;
    let latch_count = parse_number(header.next(), "latch count")?;
    let output_count = parse_number(header.next(), "output count")?;
    let gate_count = parse_number(header.next(), "AND count")?;
    let mut extensions = Vec::new();
    for value in header {
        extensions.push(parse_number(Some(value), "extended count")?);
    }
    if extensions.len() > 4 {
        return Err(reject("ASCII AIGER header has trailing fields"));
    }
    extensions.resize(4, 0);
    let [bad_count, constraint_count, justice_count, fairness_count] =
        extensions.try_into().unwrap();
    if max_variable > MAX_VARIABLES
        || input_count
            .checked_add(latch_count)
            .and_then(|count| count.checked_add(gate_count))
            != Some(max_variable)
    {
        return Err(reject("ASCII AIGER dimensions are invalid"));
    }
    if witness && (justice_count != 0 || fairness_count != 0) {
        return Err(reject("v1 does not compose liveness witnesses"));
    }

    let mut inputs = Vec::with_capacity(input_count);
    for index in 0..input_count {
        inputs.push(take_literal(&mut lines, &format!("input {index}"))?);
    }
    let mut latches = Vec::with_capacity(latch_count);
    for index in 0..latch_count {
        let line = lines
            .next()
            .ok_or_else(|| reject(format!("truncated AIGER latch {index}")))?;
        let mut fields = line.split_whitespace();
        let current = parse_number(fields.next(), "latch current")?;
        let next = parse_number(fields.next(), "latch next")?;
        let reset = match fields.next() {
            Some(value) => parse_number(Some(value), "latch reset")?,
            None => 0,
        };
        if fields.next().is_some() {
            return Err(reject("AIGER latch has trailing fields"));
        }
        latches.push(Latch {
            current,
            next,
            reset,
        });
    }
    let mut outputs = Vec::with_capacity(output_count);
    for index in 0..output_count {
        outputs.push(take_literal(&mut lines, &format!("output {index}"))?);
    }
    let mut bads = Vec::with_capacity(bad_count);
    for index in 0..bad_count {
        bads.push(take_literal(&mut lines, &format!("bad {index}"))?);
    }
    let mut constraints = Vec::with_capacity(constraint_count);
    for index in 0..constraint_count {
        constraints.push(take_literal(&mut lines, &format!("constraint {index}"))?);
    }
    if justice_count != 0 || fairness_count != 0 {
        return Err(reject("v1 does not parse AIGER liveness sections"));
    }
    let mut gates = Vec::with_capacity(gate_count);
    for index in 0..gate_count {
        let line = lines
            .next()
            .ok_or_else(|| reject(format!("truncated AIGER AND {index}")))?;
        let mut fields = line.split_whitespace();
        let output = parse_number(fields.next(), "AND output")?;
        let left = parse_number(fields.next(), "AND left")?;
        let right = parse_number(fields.next(), "AND right")?;
        if fields.next().is_some() {
            return Err(reject("AIGER AND has trailing fields"));
        }
        gates.push(Gate {
            output,
            left,
            right,
        });
    }

    let mut input_names = BTreeMap::new();
    let mut latch_names = BTreeMap::new();
    let mut comments = Vec::new();
    let mut in_comments = false;
    for line in lines {
        if in_comments {
            comments.push(line.to_string());
            continue;
        }
        if line == "c" {
            in_comments = true;
            continue;
        }
        let Some((kind_index, name)) = line.split_once(' ') else {
            return Err(reject("invalid AIGER symbol line"));
        };
        let (kind, index) = kind_index.split_at(1);
        let index = parse_number(Some(index), "symbol index")?;
        match kind {
            "i" if index < input_count => {
                if input_names.insert(index, name.to_string()).is_some() {
                    return Err(reject("duplicate AIGER input symbol"));
                }
            }
            "l" if index < latch_count => {
                if latch_names.insert(index, name.to_string()).is_some() {
                    return Err(reject("duplicate AIGER latch symbol"));
                }
            }
            "o" if index < output_count => {}
            "b" if index < bad_count => {}
            "c" if index < constraint_count => {}
            _ => return Err(reject("unsupported or invalid AIGER symbol")),
        }
    }
    if witness
        && comments.iter().any(|line| {
            line.split_whitespace()
                .next()
                .is_some_and(|word| word == "MAPPING" || word == "INTERVENTION")
        })
    {
        return Err(reject("v1 rejects comment-based witness mappings"));
    }

    let circuit = Aag {
        max_variable,
        inputs,
        latches,
        outputs,
        bads,
        constraints,
        gates,
        input_names,
        latch_names,
    };
    validate_aag(&circuit)?;
    Ok(circuit)
}

fn validate_aag(circuit: &Aag) -> Result<(), ComposedWitnessError> {
    let mut kinds = vec![None; circuit.max_variable + 1];
    for (index, &literal) in circuit.inputs.iter().enumerate() {
        let expected = 2 * (index + 1);
        if literal != expected {
            return Err(reject("AIGER inputs are not consecutively indexed"));
        }
        kinds[literal / 2] = Some(VariableKind::Input);
    }
    for (index, latch) in circuit.latches.iter().enumerate() {
        let expected = 2 * (circuit.inputs.len() + index + 1);
        if latch.current != expected {
            return Err(reject("AIGER latches are not consecutively indexed"));
        }
        kinds[latch.current / 2] = Some(VariableKind::Latch);
    }
    let variable_limit = circuit.max_variable * 2 + 1;
    for (index, gate) in circuit.gates.iter().enumerate() {
        let expected = 2 * (circuit.inputs.len() + circuit.latches.len() + index + 1);
        if gate.output != expected
            || gate.left / 2 >= gate.output / 2
            || gate.right / 2 >= gate.output / 2
        {
            return Err(reject("AIGER AND gates are not canonical and topological"));
        }
        kinds[gate.output / 2] = Some(VariableKind::Gate);
    }
    for literal in circuit
        .latches
        .iter()
        .flat_map(|latch| [latch.next, latch.reset])
        .chain(circuit.outputs.iter().copied())
        .chain(circuit.bads.iter().copied())
        .chain(circuit.constraints.iter().copied())
        .chain(
            circuit
                .gates
                .iter()
                .flat_map(|gate| [gate.left, gate.right]),
        )
    {
        if literal > variable_limit || (literal >= 2 && kinds[literal / 2].is_none()) {
            return Err(reject("AIGER references an undefined literal"));
        }
    }
    Ok(())
}

fn mapped_literal(name: Option<&String>) -> Result<Option<usize>, ComposedWitnessError> {
    let Some(name) = name else {
        return Ok(None);
    };
    let Some(value) = name.strip_prefix("= ") else {
        return Ok(None);
    };
    let literal = parse_number(Some(value), "mapped model literal")?;
    if !literal.is_multiple_of(2) || literal == 0 {
        return Err(reject(
            "mapped model literal must be positive and unnegated",
        ));
    }
    Ok(Some(literal))
}

fn model_kinds(model: &Aag) -> Vec<Option<VariableKind>> {
    let mut kinds = vec![None; model.max_variable + 1];
    for &literal in &model.inputs {
        kinds[literal / 2] = Some(VariableKind::Input);
    }
    for latch in &model.latches {
        kinds[latch.current / 2] = Some(VariableKind::Latch);
    }
    for gate in &model.gates {
        kinds[gate.output / 2] = Some(VariableKind::Gate);
    }
    kinds
}

fn translate(literal: usize, variables: &[usize]) -> Result<usize, ComposedWitnessError> {
    if literal < 2 {
        return Ok(literal);
    }
    let mapped = variables
        .get(literal / 2)
        .copied()
        .filter(|value| *value != 0)
        .ok_or_else(|| reject("witness literal was not mapped"))?;
    Ok(mapped * 2 + literal % 2)
}

/// Compose safety witness circuits using the repeated Theorem 1 construction
/// from FM 2026. The result is canonical ASCII AIGER 1.9.
pub fn compose_safety_witnesses_v1(
    model_bytes: &[u8],
    witness_bytes: &[&[u8]],
) -> Result<Vec<u8>, ComposedWitnessError> {
    if witness_bytes.len() < 2 || witness_bytes.len() > MAX_COMPOSED_WITNESSES {
        return Err(reject("composed witness count must be in 2..=64"));
    }
    let model = parse_aag(model_bytes, false)?;
    let model_kinds = model_kinds(&model);
    let witnesses = witness_bytes
        .iter()
        .map(|bytes| parse_aag(bytes, true))
        .collect::<Result<Vec<_>, _>>()?;

    let mut variable_maps = witnesses
        .iter()
        .map(|witness| vec![0; witness.max_variable + 1])
        .collect::<Vec<_>>();
    let mut shared = BTreeMap::<(VariableKind, usize), usize>::new();
    let mut input_symbols = BTreeMap::<usize, String>::new();
    let mut latch_symbols = BTreeMap::<usize, String>::new();
    let mut next_variable = 1usize;

    for (witness_index, witness) in witnesses.iter().enumerate() {
        let has_mapping = witness
            .input_names
            .values()
            .chain(witness.latch_names.values())
            .any(|name| name.starts_with("= "));
        for (index, &literal) in witness.inputs.iter().enumerate() {
            let mapping = if has_mapping {
                mapped_literal(witness.input_names.get(&index))?
            } else {
                model.inputs.get(index).copied()
            };
            let variable = assign_variable(
                VariableKind::Input,
                mapping,
                &model_kinds,
                &mut shared,
                &mut next_variable,
            )?;
            variable_maps[witness_index][literal / 2] = variable;
            let name = mapping.map_or_else(
                || {
                    format!(
                        "w{witness_index}:{}",
                        witness
                            .input_names
                            .get(&index)
                            .map(String::as_str)
                            .unwrap_or("private-input")
                    )
                },
                |mapped| format!("= {mapped}"),
            );
            input_symbols.entry(variable).or_insert(name);
        }
    }
    let input_count = next_variable - 1;

    for (witness_index, witness) in witnesses.iter().enumerate() {
        let has_mapping = witness
            .input_names
            .values()
            .chain(witness.latch_names.values())
            .any(|name| name.starts_with("= "));
        for (index, latch) in witness.latches.iter().enumerate() {
            let mapping = if has_mapping {
                mapped_literal(witness.latch_names.get(&index))?
            } else {
                model.latches.get(index).map(|latch| latch.current)
            };
            let variable = assign_variable(
                VariableKind::Latch,
                mapping,
                &model_kinds,
                &mut shared,
                &mut next_variable,
            )?;
            variable_maps[witness_index][latch.current / 2] = variable;
            let name = mapping.map_or_else(
                || {
                    format!(
                        "w{witness_index}:{}",
                        witness
                            .latch_names
                            .get(&index)
                            .map(String::as_str)
                            .unwrap_or("private-latch")
                    )
                },
                |mapped| format!("= {mapped}"),
            );
            latch_symbols.entry(variable).or_insert(name);
        }
    }
    let latch_count = next_variable - 1 - input_count;

    let mut gate_by_operands = BTreeMap::<(usize, usize), usize>::new();
    let mut gates = Vec::<Gate>::new();
    for (witness_index, witness) in witnesses.iter().enumerate() {
        for gate in &witness.gates {
            let mut left = translate(gate.left, &variable_maps[witness_index])?;
            let mut right = translate(gate.right, &variable_maps[witness_index])?;
            if left > right {
                std::mem::swap(&mut left, &mut right);
            }
            let variable = if let Some(&variable) = gate_by_operands.get(&(left, right)) {
                variable
            } else {
                let variable = next_variable;
                next_variable = next_variable
                    .checked_add(1)
                    .ok_or_else(|| reject("composed witness variable overflow"))?;
                if variable > MAX_VARIABLES {
                    return Err(reject("composed witness exceeds variable limit"));
                }
                gate_by_operands.insert((left, right), variable);
                gates.push(Gate {
                    output: variable * 2,
                    left,
                    right,
                });
                variable
            };
            variable_maps[witness_index][gate.output / 2] = variable;
        }
    }

    let mut latch_definitions = BTreeMap::<usize, (usize, usize)>::new();
    for (witness_index, witness) in witnesses.iter().enumerate() {
        for latch in &witness.latches {
            let variable = variable_maps[witness_index][latch.current / 2];
            let definition = (
                translate(latch.next, &variable_maps[witness_index])?,
                translate(latch.reset, &variable_maps[witness_index])?,
            );
            if let Some(previous) = latch_definitions.insert(variable, definition)
                && previous != definition
            {
                return Err(reject("shared witness latch definitions disagree"));
            }
        }
    }
    if latch_definitions.len() != latch_count {
        return Err(reject("composed witness latch definitions are incomplete"));
    }

    let mut bads = BTreeSet::new();
    let mut constraints = BTreeSet::new();
    for (witness_index, witness) in witnesses.iter().enumerate() {
        let safety = if witness.bads.is_empty() {
            &witness.outputs
        } else {
            &witness.bads
        };
        if safety.is_empty() {
            return Err(reject("witness has no safety property"));
        }
        for &literal in safety {
            bads.insert(translate(literal, &variable_maps[witness_index])?);
        }
        for &literal in &witness.constraints {
            constraints.insert(translate(literal, &variable_maps[witness_index])?);
        }
    }

    let mut output = String::new();
    output.push_str(&format!(
        "aag {} {} {} 0 {} {} {} 0 0\n",
        next_variable - 1,
        input_count,
        latch_count,
        gates.len(),
        bads.len(),
        constraints.len()
    ));
    for variable in 1..=input_count {
        output.push_str(&format!("{}\n", variable * 2));
    }
    for variable in (input_count + 1)..=(input_count + latch_count) {
        let (next, reset) = latch_definitions[&variable];
        output.push_str(&format!("{} {next} {reset}\n", variable * 2));
    }
    for literal in &bads {
        output.push_str(&format!("{literal}\n"));
    }
    for literal in &constraints {
        output.push_str(&format!("{literal}\n"));
    }
    for gate in &gates {
        output.push_str(&format!("{} {} {}\n", gate.output, gate.left, gate.right));
    }
    for (index, variable) in (1..=input_count).enumerate() {
        if let Some(name) = input_symbols.get(&variable) {
            output.push_str(&format!("i{index} {name}\n"));
        }
    }
    for (index, variable) in ((input_count + 1)..=(input_count + latch_count)).enumerate() {
        if let Some(name) = latch_symbols.get(&variable) {
            output.push_str(&format!("l{index} {name}\n"));
        }
    }
    output.push_str("c\nGCC FM 2026 composed-witness baseline v1\n");
    Ok(output.into_bytes())
}

fn assign_variable(
    kind: VariableKind,
    mapping: Option<usize>,
    model_kinds: &[Option<VariableKind>],
    shared: &mut BTreeMap<(VariableKind, usize), usize>,
    next_variable: &mut usize,
) -> Result<usize, ComposedWitnessError> {
    if let Some(literal) = mapping {
        if model_kinds.get(literal / 2).copied().flatten() != Some(kind) {
            return Err(reject("witness mapping kind does not match model"));
        }
        if let Some(&variable) = shared.get(&(kind, literal)) {
            return Ok(variable);
        }
    }
    let variable = *next_variable;
    *next_variable = next_variable
        .checked_add(1)
        .ok_or_else(|| reject("composed witness variable overflow"))?;
    if let Some(literal) = mapping {
        shared.insert((kind, literal), variable);
    }
    Ok(variable)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &[u8] =
        b"aag 3 1 1 1 1\n2\n4 6 0\n4\n6 4 2\ni0 sensor\nl0 state\no0 bad\nc\nmodel\n";
    const WITNESS: &[u8] = b"aag 3 1 1 1 1\n2\n4 6 0\n4\n6 4 2\ni0 = 2\nl0 = 4\no0 invariant\nc\nWITNESS o0 model.aag\n";

    #[test]
    fn self_composition_coalesces_shared_state_and_gates() {
        let first = compose_safety_witnesses_v1(MODEL, &[WITNESS, WITNESS]).unwrap();
        let second = compose_safety_witnesses_v1(MODEL, &[WITNESS, WITNESS]).unwrap();
        assert_eq!(first, second);
        let composed = parse_aag(&first, true).unwrap();
        assert_eq!(composed.inputs.len(), 1);
        assert_eq!(composed.latches.len(), 1);
        assert_eq!(composed.gates.len(), 1);
        assert_eq!(composed.bads.len(), 1);
        assert_eq!(composed.input_names[&0], "= 2");
        assert_eq!(composed.latch_names[&0], "= 4");
    }

    #[test]
    fn private_variables_remain_disjoint() {
        let witness =
            b"aag 4 2 1 1 1\n2\n4\n6 6 0\n8\n8 6 4\ni0 = 2\ni1 helper\nl0 = 4\nc\nprivate\n";
        let composed = compose_safety_witnesses_v1(MODEL, &[witness, witness]).unwrap();
        let parsed = parse_aag(&composed, true).unwrap();
        assert_eq!(parsed.inputs.len(), 3);
        assert_eq!(parsed.latches.len(), 1);
    }

    #[test]
    fn unsupported_and_hostile_witnesses_fail_closed() {
        let mapping = b"aag 3 1 1 1 1\n2\n4 6 0\n4\n6 4 2\nc\nMAPPING 1\n2 2\n";
        assert!(compose_safety_witnesses_v1(MODEL, &[mapping, WITNESS]).is_err());
        let symbol_start = WITNESS
            .windows(3)
            .position(|bytes| bytes == b"i0 ")
            .unwrap();
        for end in 0..symbol_start {
            assert!(compose_safety_witnesses_v1(MODEL, &[&WITNESS[..end], WITNESS]).is_err());
        }
        let wrong_kind = b"aag 3 1 1 1 1\n2\n4 6 0\n4\n6 4 2\ni0 = 4\nl0 = 4\nc\nwrong kind\n";
        assert!(compose_safety_witnesses_v1(MODEL, &[wrong_kind, WITNESS]).is_err());
    }
}
