//! Exact invariant-chained recurrence analysis for bounded BTOR2 property sets.

use crate::btor2::{self, BinaryOp, Btor2Model, NodeId, NodeKind, UnaryOp, WordValues};
use crate::btor2_region::{MAX_REGION_HORIZON, RegionPredicate};
use crate::btor2_search::SearchResult;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const MAX_CHAIN_STATES: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InvariantClaim {
    pub state: NodeId,
    pub width: u32,
    pub value: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecurrenceClaim {
    pub state: NodeId,
    pub width: u32,
    pub initial: u64,
    pub reset: u64,
    pub delta: u64,
    pub guard_invariant: Option<NodeId>,
    pub max_index: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MemberClaim {
    pub bad_property: NodeId,
    pub recurrence_state: NodeId,
    pub predicate: RegionPredicate,
    pub predicate_literal: u64,
    pub result: SearchResult,
    pub bad_frame: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChainAnalysis {
    pub source_sha256: String,
    pub query_horizon: u32,
    pub input: NodeId,
    pub invariants: Vec<InvariantClaim>,
    pub recurrences: Vec<RecurrenceClaim>,
    pub members: Vec<MemberClaim>,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChainError(String);

impl fmt::Display for ChainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ChainError {}

fn reject(message: impl Into<String>) -> ChainError {
    ChainError(message.into())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn constant(model: &Btor2Model, id: NodeId) -> Option<u64> {
    match model.nodes().get(&id)?.kind {
        NodeKind::Constant(value) => Some(value),
        _ => None,
    }
}

fn dependencies(model: &Btor2Model, root: NodeId) -> BTreeSet<NodeId> {
    let mut result = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        match model.nodes()[&id].kind {
            NodeKind::Input | NodeKind::State => {
                result.insert(id);
            }
            NodeKind::Constant(_) => {}
            NodeKind::Unary(_, value)
            | NodeKind::Slice { value, .. }
            | NodeKind::Uext { value, .. } => stack.push(value),
            NodeKind::Binary(_, left, right) => {
                stack.push(left);
                stack.push(right);
            }
            NodeKind::Concat { high, low } => {
                stack.push(high);
                stack.push(low);
            }
            NodeKind::Ite(condition, then_value, else_value) => {
                stack.push(condition);
                stack.push(then_value);
                stack.push(else_value);
            }
        }
    }
    result
}

fn evaluate_with_invariants(
    model: &Btor2Model,
    root: NodeId,
    input: NodeId,
    input_value: bool,
    invariants: &BTreeMap<NodeId, u64>,
) -> Option<u64> {
    model
        .evaluate(
            root,
            invariants,
            &WordValues::from([(input, u64::from(input_value))]),
        )
        .ok()
}

fn constant_under_invariants(
    model: &Btor2Model,
    root: NodeId,
    input: NodeId,
    invariants: &BTreeMap<NodeId, u64>,
) -> Option<u64> {
    let allowed = invariants
        .keys()
        .copied()
        .chain(std::iter::once(input))
        .collect::<BTreeSet<_>>();
    if !dependencies(model, root).is_subset(&allowed) {
        return None;
    }
    let low = evaluate_with_invariants(model, root, input, false, invariants)?;
    let high = evaluate_with_invariants(model, root, input, true, invariants)?;
    (low == high).then_some(low)
}

fn recognise_invariants(model: &Btor2Model, input: NodeId) -> BTreeMap<NodeId, InvariantClaim> {
    let mut result = BTreeMap::new();
    for state in model.states() {
        let Some(initial) = model.initialiser(*state).and_then(|id| constant(model, id)) else {
            continue;
        };
        let Some(next) = model.next_value(*state) else {
            continue;
        };
        let support = dependencies(model, next);
        if !support.is_subset(&BTreeSet::from([input, *state])) {
            continue;
        }
        let values = BTreeMap::from([(*state, initial)]);
        if [false, true]
            .into_iter()
            .all(|bit| evaluate_with_invariants(model, next, input, bit, &values) == Some(initial))
        {
            result.insert(
                *state,
                InvariantClaim {
                    state: *state,
                    width: model.nodes()[state].width,
                    value: initial,
                },
            );
        }
    }
    result
}

fn unwrap_zero_extension(model: &Btor2Model, mut id: NodeId) -> NodeId {
    loop {
        match model.nodes()[&id].kind {
            NodeKind::Uext { value, amount: 0 } => id = value,
            _ => return id,
        }
    }
}

fn add_delta(model: &Btor2Model, expression: NodeId, state: NodeId) -> Option<u64> {
    let expression = unwrap_zero_extension(model, expression);
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Add, left, right) if left == state => constant(model, right),
        NodeKind::Binary(BinaryOp::Add, left, right) if right == state => constant(model, left),
        _ => None,
    }
}

fn simplify_boolean(
    model: &Btor2Model,
    mut root: NodeId,
    input: NodeId,
    invariants: &BTreeMap<NodeId, u64>,
) -> NodeId {
    for _ in 0..64 {
        let replacement = match model.nodes()[&root].kind {
            NodeKind::Unary(UnaryOp::Not, inner) => match model.nodes()[&inner].kind {
                NodeKind::Unary(UnaryOp::Not, value) => Some(value),
                _ => None,
            },
            NodeKind::Binary(BinaryOp::And, left, right) => {
                match (
                    constant_under_invariants(model, left, input, invariants),
                    constant_under_invariants(model, right, input, invariants),
                ) {
                    (Some(1), _) => Some(right),
                    (_, Some(1)) => Some(left),
                    _ => None,
                }
            }
            NodeKind::Uext { value, amount: 0 } => Some(value),
            _ => None,
        };
        let Some(replacement) = replacement else {
            return root;
        };
        root = replacement;
    }
    root
}

fn recognise_predicate(
    model: &Btor2Model,
    bad_property: NodeId,
    input: NodeId,
    invariants: &BTreeMap<NodeId, u64>,
) -> Option<(NodeId, RegionPredicate, u64)> {
    let expression = model
        .bad_properties()
        .iter()
        .find_map(|(id, expression, _)| (*id == bad_property).then_some(*expression))?;
    let expression = simplify_boolean(model, expression, input, invariants);
    match model.nodes().get(&expression)?.kind {
        NodeKind::Binary(BinaryOp::Eq, left, right)
            if matches!(model.nodes()[&left].kind, NodeKind::State) =>
        {
            Some((left, RegionPredicate::Equal, constant(model, right)?))
        }
        NodeKind::Binary(BinaryOp::Eq, left, right)
            if matches!(model.nodes()[&right].kind, NodeKind::State) =>
        {
            Some((right, RegionPredicate::Equal, constant(model, left)?))
        }
        NodeKind::Binary(BinaryOp::Ugte, left, right)
            if matches!(model.nodes()[&left].kind, NodeKind::State) =>
        {
            Some((
                left,
                RegionPredicate::UnsignedGreaterEqual,
                constant(model, right)?,
            ))
        }
        _ => None,
    }
}

fn recognise_recurrence(
    model: &Btor2Model,
    state: NodeId,
    input: NodeId,
    invariants: &BTreeMap<NodeId, u64>,
    horizon: u32,
) -> Option<RecurrenceClaim> {
    let initial = model
        .initialiser(state)
        .and_then(|id| constant(model, id))?;
    let next = model.next_value(state)?;
    let (condition, reset_expression, advance) = match model.nodes()[&next].kind {
        NodeKind::Ite(condition, reset, advance) if condition == input => {
            (condition, reset, advance)
        }
        _ => return None,
    };
    debug_assert_eq!(condition, input);
    let reset = constant(model, reset_expression)?;
    if initial != reset {
        return None;
    }

    let (delta, guard_invariant) = if let Some(delta) = add_delta(model, advance, state) {
        (delta, None)
    } else {
        let (guard, increment, hold) = match model.nodes()[&advance].kind {
            NodeKind::Ite(guard, increment, hold) if hold == state => (guard, increment, hold),
            _ => return None,
        };
        debug_assert_eq!(hold, state);
        if constant_under_invariants(model, guard, input, invariants) != Some(1) {
            return None;
        }
        let guard_states = dependencies(model, guard)
            .into_iter()
            .filter(|id| *id != input)
            .collect::<Vec<_>>();
        if guard_states.len() != 1 || !invariants.contains_key(&guard_states[0]) {
            return None;
        }
        (add_delta(model, increment, state)?, Some(guard_states[0]))
    };
    if delta == 0 {
        return None;
    }
    let allowed = invariants
        .keys()
        .copied()
        .chain([input, state])
        .collect::<BTreeSet<_>>();
    if !dependencies(model, next).is_subset(&allowed) {
        return None;
    }
    let distance = delta.checked_mul(u64::from(horizon))?;
    reset
        .checked_add(distance)
        .filter(|value| *value <= mask(model.nodes()[&state].width))?;
    Some(RecurrenceClaim {
        state,
        width: model.nodes()[&state].width,
        initial,
        reset,
        delta,
        guard_invariant,
        max_index: u64::from(horizon),
    })
}

fn first_bad_frame(
    recurrence: &RecurrenceClaim,
    predicate: RegionPredicate,
    literal: u64,
) -> Option<u32> {
    let index = match predicate {
        RegionPredicate::Equal => {
            let distance = literal.checked_sub(recurrence.reset)?;
            distance
                .is_multiple_of(recurrence.delta)
                .then_some(distance / recurrence.delta)?
        }
        RegionPredicate::UnsignedGreaterEqual => literal
            .saturating_sub(recurrence.reset)
            .div_ceil(recurrence.delta),
    };
    (index <= recurrence.max_index)
        .then(|| u32::try_from(index).ok())
        .flatten()
}

pub(crate) fn analyse(
    source: &[u8],
    properties: &[NodeId],
    horizon: u32,
) -> Result<Option<ChainAnalysis>, ChainError> {
    if horizon > MAX_REGION_HORIZON {
        return Err(reject("invariant-chain horizon exceeds limit"));
    }
    let model = btor2::parse_bytes(source).map_err(|error| reject(error.to_string()))?;
    if properties.len() < 2
        || model.states().len() < 3
        || model.states().len() > MAX_CHAIN_STATES
        || model.inputs().len() != 1
        || model.nodes()[&model.inputs()[0]].width != 1
        || !model.constraints().is_empty()
    {
        return Ok(None);
    }
    let input = model.inputs()[0];
    let invariant_claims = recognise_invariants(&model, input);
    if invariant_claims.is_empty() {
        return Ok(None);
    }
    let invariant_values = invariant_claims
        .iter()
        .map(|(state, claim)| (*state, claim.value))
        .collect::<BTreeMap<_, _>>();
    let mut recurrences = BTreeMap::<NodeId, RecurrenceClaim>::new();
    let mut members = Vec::with_capacity(properties.len());
    let mut used_invariants = BTreeSet::new();
    for property in properties {
        let Some((state, predicate, predicate_literal)) =
            recognise_predicate(&model, *property, input, &invariant_values)
        else {
            return Ok(None);
        };
        let recurrence = if let Some(recurrence) = recurrences.get(&state) {
            recurrence.clone()
        } else {
            let Some(recurrence) =
                recognise_recurrence(&model, state, input, &invariant_values, horizon)
            else {
                return Ok(None);
            };
            recurrences.insert(state, recurrence.clone());
            recurrence
        };
        if let Some(invariant) = recurrence.guard_invariant {
            used_invariants.insert(invariant);
        }
        let bad_frame = first_bad_frame(&recurrence, predicate, predicate_literal);
        members.push(MemberClaim {
            bad_property: *property,
            recurrence_state: state,
            predicate,
            predicate_literal,
            result: if bad_frame.is_some() {
                SearchResult::Unsafe
            } else {
                SearchResult::Safe
            },
            bad_frame,
        });
    }
    if recurrences.len() < 2 || used_invariants.is_empty() {
        return Ok(None);
    }
    let invariants = used_invariants
        .into_iter()
        .map(|state| invariant_claims[&state].clone())
        .collect::<Vec<_>>();
    let recurrences = recurrences.into_values().collect::<Vec<_>>();
    let layers = u64::from(horizon) + 1;
    let logical_reachable_states = layers
        .checked_mul(layers + 1)
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| reject("invariant-chain logical state count overflowed"))?;
    Ok(Some(ChainAnalysis {
        source_sha256: digest(source),
        query_horizon: horizon,
        input,
        invariants,
        recurrences,
        members,
        logical_reachable_states,
    }))
}

pub(crate) fn verify(
    source: &[u8],
    properties: &[NodeId],
    horizon: u32,
    claimed: &ChainAnalysis,
) -> Result<(), ChainError> {
    let Some(expected) = analyse(source, properties, horizon)? else {
        return Err(reject("source does not admit invariant chaining"));
    };
    if &expected != claimed {
        return Err(reject(
            "invariant-chain claims do not match reconstructed source semantics",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUAL_TIMER: &[u8] = include_bytes!(
        "../corpus/rtl/opentitan-aon-timer/generated/dual-timer-predicate-set.btor2"
    );
    const SAME_WIDTH_CHAIN: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 reset\n4 zero 2\n5 state 2 a\n6 init 2 5 4\n7 state 2 b\n8 init 2 7 4\n9 state 2 invariant\n10 init 2 9 4\n11 one 2\n12 add 2 5 11\n13 ite 2 3 4 12\n14 next 2 5 13\n15 eq 1 9 4\n16 add 2 7 11\n17 ite 2 15 16 7\n18 ite 2 3 4 17\n19 next 2 7 18\n20 next 2 9 9\n21 constd 2 3\n22 ugte 1 5 21\n23 bad 22 a_bad\n24 constd 2 4\n25 ugte 1 7 24\n26 bad 25 b_bad\n";

    #[test]
    fn reconstructs_the_open_titan_invariant_chain_and_exact_frames() {
        let analysis = analyse(DUAL_TIMER, &[33, 37, 41], 9).unwrap().unwrap();
        assert_eq!(analysis.input, 2);
        assert_eq!(analysis.invariants.len(), 1);
        assert_eq!(analysis.invariants[0].state, 16);
        assert_eq!(analysis.invariants[0].value, 0);
        assert_eq!(analysis.recurrences.len(), 2);
        assert_eq!(
            analysis
                .recurrences
                .iter()
                .map(|claim| (claim.state, claim.guard_invariant))
                .collect::<Vec<_>>(),
            vec![(6, None), (23, Some(16))]
        );
        assert_eq!(
            analysis
                .members
                .iter()
                .map(|member| member.bad_frame)
                .collect::<Vec<_>>(),
            vec![Some(9), Some(5), Some(7)]
        );
        verify(DUAL_TIMER, &[33, 37, 41], 9, &analysis).unwrap();
    }

    #[test]
    fn rejects_changed_invariant_guard_and_malformed_cross_coupling() {
        let changed_guard = std::str::from_utf8(DUAL_TIMER)
            .unwrap()
            .replace("49 ite 14 20 15 48", "49 ite 14 19 15 48");
        assert!(
            analyse(changed_guard.as_bytes(), &[33, 37, 41], 9)
                .unwrap()
                .is_none()
        );

        let cross_coupled = std::str::from_utf8(DUAL_TIMER)
            .unwrap()
            .replace("43 add 21 23 42", "43 add 21 23 6");
        let error = analyse(cross_coupled.as_bytes(), &[33, 37, 41], 9).unwrap_err();
        assert!(error.to_string().contains("width 32 does not match 64"));
    }

    #[test]
    fn rejects_a_well_typed_cross_coupled_near_neighbour() {
        assert!(analyse(SAME_WIDTH_CHAIN, &[23, 26], 4).unwrap().is_some());
        let cross_coupled = std::str::from_utf8(SAME_WIDTH_CHAIN)
            .unwrap()
            .replace("16 add 2 7 11", "16 add 2 5 11");
        assert!(
            analyse(cross_coupled.as_bytes(), &[23, 26], 4)
                .unwrap()
                .is_none()
        );
    }
}
