//! Strict, resource-bounded BTOR2 bit-vector semantic core.
//!
//! This module deliberately accepts a bounded bit-vector subset. Arrays,
//! fairness, liveness and unsupported operations fail closed instead of being
//! lowered silently. The normalized word-level graph is intended to become the
//! source-boundary for proof-carrying firmware counter and timer composition.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const BTOR2_CORE_VERSION: u32 = 1;
pub const MAX_BTOR2_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BTOR2_LINES: usize = 100_000;
pub const MAX_BTOR2_NODES: usize = 100_000;
pub const MAX_BIT_WIDTH: u32 = 64;

pub type NodeId = u64;
pub type WordValues = BTreeMap<NodeId, u64>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "BTOR2 line {}: {}", self.line, self.message)
    }
}

impl Error for ParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalError {
    MissingInput(NodeId),
    MissingState(NodeId),
    ConstraintViolation(NodeId),
    NonConstantInitialiser(NodeId),
    Internal(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInput(id) => write!(formatter, "missing BTOR2 input {id}"),
            Self::MissingState(id) => write!(formatter, "missing BTOR2 state {id}"),
            Self::ConstraintViolation(id) => write!(formatter, "BTOR2 constraint {id} is false"),
            Self::NonConstantInitialiser(id) => {
                write!(formatter, "BTOR2 state {id} has a non-constant initialiser")
            }
            Self::Internal(message) => formatter.write_str(message),
        }
    }
}

impl Error for EvalError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Not,
    Inc,
    Dec,
    Neg,
    Redor,
    Redand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Mul,
    Sll,
    Srl,
    Eq,
    Neq,
    Ult,
    Ulte,
    Ugt,
    Ugte,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Input,
    State,
    Constant(u64),
    Unary(UnaryOp, NodeId),
    Binary(BinaryOp, NodeId, NodeId),
    Ite(NodeId, NodeId, NodeId),
    Slice {
        value: NodeId,
        upper: u32,
        lower: u32,
    },
    Uext {
        value: NodeId,
        amount: u32,
    },
    Concat {
        high: NodeId,
        low: NodeId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub width: u32,
    pub kind: NodeKind,
    pub symbol: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Btor2Model {
    nodes: BTreeMap<NodeId, Node>,
    inputs: Vec<NodeId>,
    states: Vec<NodeId>,
    initialisers: BTreeMap<NodeId, NodeId>,
    next_values: BTreeMap<NodeId, NodeId>,
    initialiser_symbols: BTreeMap<NodeId, String>,
    next_symbols: BTreeMap<NodeId, String>,
    constraints: Vec<(NodeId, NodeId)>,
    bad: Vec<(NodeId, NodeId, Option<String>)>,
}

impl Btor2Model {
    pub fn nodes(&self) -> &BTreeMap<NodeId, Node> {
        &self.nodes
    }

    pub fn inputs(&self) -> &[NodeId] {
        &self.inputs
    }

    pub fn states(&self) -> &[NodeId] {
        &self.states
    }

    pub fn bad_properties(&self) -> &[(NodeId, NodeId, Option<String>)] {
        &self.bad
    }

    pub fn constraints(&self) -> &[(NodeId, NodeId)] {
        &self.constraints
    }

    pub fn initialiser(&self, state: NodeId) -> Option<NodeId> {
        self.initialisers.get(&state).copied()
    }

    pub fn next_value(&self, state: NodeId) -> Option<NodeId> {
        self.next_values.get(&state).copied()
    }

    pub fn initialiser_symbol(&self, state: NodeId) -> Option<&str> {
        self.initialiser_symbols.get(&state).map(String::as_str)
    }

    pub fn next_symbol(&self, state: NodeId) -> Option<&str> {
        self.next_symbols.get(&state).map(String::as_str)
    }

    pub fn max_width(&self) -> u32 {
        self.nodes
            .values()
            .map(|node| node.width)
            .max()
            .unwrap_or(0)
    }

    pub fn initial_state(&self) -> Result<WordValues, EvalError> {
        let mut result = BTreeMap::new();
        let inputs = BTreeMap::new();
        let states = BTreeMap::new();
        for state in &self.states {
            let expression = self
                .initialisers
                .get(state)
                .ok_or(EvalError::NonConstantInitialiser(*state))?;
            let value = self
                .evaluate(*expression, &states, &inputs)
                .map_err(|_| EvalError::NonConstantInitialiser(*state))?;
            result.insert(*state, value);
        }
        Ok(result)
    }

    pub fn step(&self, state: &WordValues, inputs: &WordValues) -> Result<WordValues, EvalError> {
        self.check_constraints(state, inputs)?;
        let mut next = BTreeMap::new();
        for state_id in &self.states {
            let expression = self.next_values.get(state_id).ok_or_else(|| {
                EvalError::Internal(format!("BTOR2 state {state_id} has no next value"))
            })?;
            next.insert(*state_id, self.evaluate(*expression, state, inputs)?);
        }
        Ok(next)
    }

    pub fn active_bad(
        &self,
        state: &WordValues,
        inputs: &WordValues,
    ) -> Result<Vec<NodeId>, EvalError> {
        self.check_constraints(state, inputs)?;
        let mut active = Vec::new();
        for (property, expression, _) in &self.bad {
            if self.evaluate(*expression, state, inputs)? != 0 {
                active.push(*property);
            }
        }
        Ok(active)
    }

    pub fn evaluate(
        &self,
        root: NodeId,
        state: &WordValues,
        inputs: &WordValues,
    ) -> Result<u64, EvalError> {
        fn visit(
            model: &Btor2Model,
            id: NodeId,
            state: &WordValues,
            inputs: &WordValues,
            memo: &mut BTreeMap<NodeId, u64>,
        ) -> Result<u64, EvalError> {
            if let Some(value) = memo.get(&id) {
                return Ok(*value);
            }
            let node = model.nodes.get(&id).ok_or_else(|| {
                EvalError::Internal(format!("missing normalized BTOR2 node {id}"))
            })?;
            let mask = width_mask(node.width);
            let value = match node.kind {
                NodeKind::Input => *inputs.get(&id).ok_or(EvalError::MissingInput(id))?,
                NodeKind::State => *state.get(&id).ok_or(EvalError::MissingState(id))?,
                NodeKind::Constant(value) => value,
                NodeKind::Unary(operator, operand) => {
                    let value = visit(model, operand, state, inputs, memo)?;
                    match operator {
                        UnaryOp::Not => !value,
                        UnaryOp::Inc => value.wrapping_add(1),
                        UnaryOp::Dec => value.wrapping_sub(1),
                        UnaryOp::Neg => 0u64.wrapping_sub(value),
                        UnaryOp::Redor => u64::from(value != 0),
                        UnaryOp::Redand => {
                            let operand_width = model.nodes[&operand].width;
                            u64::from(value == width_mask(operand_width))
                        }
                    }
                }
                NodeKind::Binary(operator, left, right) => {
                    let left = visit(model, left, state, inputs, memo)?;
                    let right = visit(model, right, state, inputs, memo)?;
                    match operator {
                        BinaryOp::And => left & right,
                        BinaryOp::Or => left | right,
                        BinaryOp::Xor => left ^ right,
                        BinaryOp::Add => left.wrapping_add(right),
                        BinaryOp::Sub => left.wrapping_sub(right),
                        BinaryOp::Mul => left.wrapping_mul(right),
                        BinaryOp::Sll => u32::try_from(right)
                            .ok()
                            .and_then(|amount| left.checked_shl(amount))
                            .unwrap_or(0),
                        BinaryOp::Srl => u32::try_from(right)
                            .ok()
                            .and_then(|amount| left.checked_shr(amount))
                            .unwrap_or(0),
                        BinaryOp::Eq => u64::from(left == right),
                        BinaryOp::Neq => u64::from(left != right),
                        BinaryOp::Ult => u64::from(left < right),
                        BinaryOp::Ulte => u64::from(left <= right),
                        BinaryOp::Ugt => u64::from(left > right),
                        BinaryOp::Ugte => u64::from(left >= right),
                    }
                }
                NodeKind::Ite(condition, then_value, else_value) => {
                    let selected = if visit(model, condition, state, inputs, memo)? != 0 {
                        then_value
                    } else {
                        else_value
                    };
                    visit(model, selected, state, inputs, memo)?
                }
                NodeKind::Slice {
                    value,
                    upper: _,
                    lower,
                } => visit(model, value, state, inputs, memo)? >> lower,
                NodeKind::Uext { value, amount: _ } => visit(model, value, state, inputs, memo)?,
                NodeKind::Concat { high, low } => {
                    let high_value = visit(model, high, state, inputs, memo)?;
                    let low_value = visit(model, low, state, inputs, memo)?;
                    let low_width = model.nodes[&low].width;
                    (high_value << low_width) | low_value
                }
            } & mask;
            memo.insert(id, value);
            Ok(value)
        }

        visit(self, root, state, inputs, &mut BTreeMap::new())
    }

    fn check_constraints(&self, state: &WordValues, inputs: &WordValues) -> Result<(), EvalError> {
        for (id, expression) in &self.constraints {
            if self.evaluate(*expression, state, inputs)? == 0 {
                return Err(EvalError::ConstraintViolation(*id));
            }
        }
        Ok(())
    }
}

fn width_mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn parse_u64(token: Option<&str>, line: usize, label: &str) -> Result<u64, ParseError> {
    token
        .ok_or_else(|| ParseError::new(line, format!("missing {label}")))?
        .parse()
        .map_err(|_| ParseError::new(line, format!("invalid {label}")))
}

fn expect_end<T: Iterator<Item = String> + ?Sized>(
    tokens: &mut T,
    line: usize,
) -> Result<(), ParseError> {
    if tokens.next().is_some() {
        Err(ParseError::new(line, "unexpected trailing tokens"))
    } else {
        Ok(())
    }
}

fn parse_with_roots(
    input: &str,
    additional_semantic_roots: &[NodeId],
    require_bad_property: bool,
) -> Result<Btor2Model, ParseError> {
    if input.len() > MAX_BTOR2_BYTES {
        return Err(ParseError::new(0, "input exceeds the 8 MiB limit"));
    }
    if input.as_bytes().contains(&0) {
        return Err(ParseError::new(0, "input contains NUL"));
    }
    if input.contains('\r') || (!input.is_empty() && !input.ends_with('\n')) {
        return Err(ParseError::new(
            0,
            "input must use canonical newline-terminated LF text",
        ));
    }

    let mut sorts = BTreeMap::<NodeId, u32>::new();
    let mut nodes = BTreeMap::<NodeId, Node>::new();
    let mut inputs = Vec::new();
    let mut states = Vec::new();
    let mut initialisers = BTreeMap::new();
    let mut next_values = BTreeMap::new();
    let mut initialiser_symbols = BTreeMap::new();
    let mut next_symbols = BTreeMap::new();
    let mut constraints = Vec::new();
    let mut bad = Vec::new();
    let mut identifiers = BTreeSet::new();
    let mut previous_id = 0;

    for (offset, raw) in input.lines().enumerate() {
        let line = offset + 1;
        if line > MAX_BTOR2_LINES {
            return Err(ParseError::new(line, "line count exceeds limit"));
        }
        let content = raw.split_once(';').map_or(raw, |(prefix, _)| prefix).trim();
        if content.is_empty() {
            continue;
        }
        if !content.is_ascii() {
            return Err(ParseError::new(line, "non-ASCII content is unsupported"));
        }
        let mut tokens = content.split_ascii_whitespace().map(str::to_string);
        let id = parse_u64(tokens.next().as_deref(), line, "node identifier")?;
        if id == 0 || id <= previous_id || !identifiers.insert(id) {
            return Err(ParseError::new(
                line,
                "identifiers must be unique and strictly increasing",
            ));
        }
        previous_id = id;
        let operation = tokens
            .next()
            .ok_or_else(|| ParseError::new(line, "missing operation"))?;
        if operation == "sort" {
            if tokens.next().as_deref() != Some("bitvec") {
                return Err(ParseError::new(line, "only bit-vector sorts are supported"));
            }
            let width = parse_u64(tokens.next().as_deref(), line, "bit width")?;
            if !(1..=u64::from(MAX_BIT_WIDTH)).contains(&width) {
                return Err(ParseError::new(line, "bit width must be in 1..=64"));
            }
            expect_end(&mut tokens, line)?;
            sorts.insert(id, width as u32);
            continue;
        }
        if nodes.len() >= MAX_BTOR2_NODES {
            return Err(ParseError::new(line, "node count exceeds limit"));
        }

        if matches!(operation.as_str(), "bad" | "constraint") {
            let expression = parse_u64(tokens.next().as_deref(), line, "property expression")?;
            require_width(&nodes, expression, 1, line)?;
            let symbol = tokens.next();
            expect_end(&mut tokens, line)?;
            if operation == "bad" {
                bad.push((id, expression, symbol));
            } else {
                if symbol.is_some() {
                    return Err(ParseError::new(line, "constraint symbols are unsupported"));
                }
                constraints.push((id, expression));
            }
            continue;
        }

        // BTOR2 outputs are named observation roots, not transition-system
        // semantics. Accept and validate them so standard Yosys `write_btor`
        // output can be consumed without treating an output as a property.
        if operation == "output" {
            let expression = parse_u64(tokens.next().as_deref(), line, "output expression")?;
            if !nodes.contains_key(&expression) {
                return Err(ParseError::new(
                    line,
                    format!("unknown or non-prior output expression {expression}"),
                ));
            }
            let _symbol = tokens.next();
            expect_end(&mut tokens, line)?;
            continue;
        }

        let sort = parse_u64(tokens.next().as_deref(), line, "sort identifier")?;
        let width = *sorts
            .get(&sort)
            .ok_or_else(|| ParseError::new(line, "unknown or non-prior sort identifier"))?;
        if matches!(operation.as_str(), "init" | "next") {
            let state = parse_u64(tokens.next().as_deref(), line, "state operand")?;
            let value = parse_u64(tokens.next().as_deref(), line, "value operand")?;
            require_kind(&nodes, state, NodeKindDiscriminant::State, line)?;
            require_width(&nodes, state, width, line)?;
            require_width(&nodes, value, width, line)?;
            let symbol = tokens.next();
            expect_end(&mut tokens, line)?;
            let target = if operation == "init" {
                &mut initialisers
            } else {
                &mut next_values
            };
            if target.insert(state, value).is_some() {
                return Err(ParseError::new(
                    line,
                    format!("duplicate {operation} for state {state}"),
                ));
            }
            if let Some(symbol) = symbol {
                let symbols = if operation == "init" {
                    &mut initialiser_symbols
                } else {
                    &mut next_symbols
                };
                symbols.insert(state, symbol);
            }
            continue;
        }

        let (kind, symbol) = parse_node_kind(&operation, width, &nodes, &mut tokens, line)?;
        let node = Node {
            id,
            width,
            kind,
            symbol,
        };
        if matches!(node.kind, NodeKind::Input) {
            inputs.push(id);
        }
        if matches!(node.kind, NodeKind::State) {
            states.push(id);
        }
        nodes.insert(id, node);
    }

    if states.is_empty() {
        return Err(ParseError::new(0, "model requires at least one state"));
    }
    if require_bad_property && bad.is_empty() {
        return Err(ParseError::new(
            0,
            "model requires at least one state and one bad property",
        ));
    }
    for root in additional_semantic_roots {
        if !nodes.contains_key(root) {
            return Err(ParseError::new(
                0,
                format!("unknown component semantic root {root}"),
            ));
        }
    }
    for state in &states {
        if !initialisers.contains_key(state) || !next_values.contains_key(state) {
            return Err(ParseError::new(
                0,
                format!("state {state} requires exactly one init and next"),
            ));
        }
        if !is_constant_expression(&nodes, initialisers[state]) {
            return Err(ParseError::new(
                0,
                format!("state {state} requires a constant initialiser"),
            ));
        }
    }
    let semantic_inputs = semantic_input_support(
        &nodes,
        next_values
            .values()
            .copied()
            .chain(constraints.iter().map(|(_, expression)| *expression))
            .chain(bad.iter().map(|(_, expression, _)| *expression))
            .chain(additional_semantic_roots.iter().copied()),
    );
    inputs.retain(|input| semantic_inputs.contains(input));
    Ok(Btor2Model {
        nodes,
        inputs,
        states,
        initialisers,
        next_values,
        initialiser_symbols,
        next_symbols,
        constraints,
        bad,
    })
}

pub fn parse(input: &str) -> Result<Btor2Model, ParseError> {
    parse_with_roots(input, &[], true)
}

pub fn parse_component(input: &str, semantic_roots: &[NodeId]) -> Result<Btor2Model, ParseError> {
    if semantic_roots.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ParseError::new(
            0,
            "component semantic roots must be unique and strictly ordered",
        ));
    }
    parse_with_roots(input, semantic_roots, false)
}

fn semantic_input_support(
    nodes: &BTreeMap<NodeId, Node>,
    roots: impl IntoIterator<Item = NodeId>,
) -> BTreeSet<NodeId> {
    let mut stack = roots.into_iter().collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    let mut inputs = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        match &nodes[&id].kind {
            NodeKind::Input => {
                inputs.insert(id);
            }
            NodeKind::State | NodeKind::Constant(_) => {}
            NodeKind::Unary(_, value)
            | NodeKind::Slice { value, .. }
            | NodeKind::Uext { value, .. } => stack.push(*value),
            NodeKind::Binary(_, left, right) => {
                stack.push(*left);
                stack.push(*right);
            }
            NodeKind::Concat { high, low } => {
                stack.push(*high);
                stack.push(*low);
            }
            NodeKind::Ite(condition, then_value, else_value) => {
                stack.push(*condition);
                stack.push(*then_value);
                stack.push(*else_value);
            }
        }
    }
    inputs
}

fn is_constant_expression(nodes: &BTreeMap<NodeId, Node>, root: NodeId) -> bool {
    fn visit(
        nodes: &BTreeMap<NodeId, Node>,
        id: NodeId,
        memo: &mut BTreeMap<NodeId, bool>,
    ) -> bool {
        if let Some(result) = memo.get(&id) {
            return *result;
        }
        let result = match &nodes[&id].kind {
            NodeKind::Input | NodeKind::State => false,
            NodeKind::Constant(_) => true,
            NodeKind::Unary(_, value)
            | NodeKind::Slice { value, .. }
            | NodeKind::Uext { value, .. } => visit(nodes, *value, memo),
            NodeKind::Binary(_, left, right) => {
                visit(nodes, *left, memo) && visit(nodes, *right, memo)
            }
            NodeKind::Concat { high, low } => visit(nodes, *high, memo) && visit(nodes, *low, memo),
            NodeKind::Ite(condition, then_value, else_value) => {
                visit(nodes, *condition, memo)
                    && visit(nodes, *then_value, memo)
                    && visit(nodes, *else_value, memo)
            }
        };
        memo.insert(id, result);
        result
    }
    visit(nodes, root, &mut BTreeMap::new())
}

pub fn parse_bytes(input: &[u8]) -> Result<Btor2Model, ParseError> {
    if input.len() > MAX_BTOR2_BYTES {
        return Err(ParseError::new(0, "input exceeds the 8 MiB limit"));
    }
    let text = std::str::from_utf8(input)
        .map_err(|_| ParseError::new(0, "input must contain valid UTF-8"))?;
    parse(text)
}

pub fn parse_component_bytes(
    input: &[u8],
    semantic_roots: &[NodeId],
) -> Result<Btor2Model, ParseError> {
    if input.len() > MAX_BTOR2_BYTES {
        return Err(ParseError::new(0, "input exceeds the 8 MiB limit"));
    }
    let text = std::str::from_utf8(input)
        .map_err(|_| ParseError::new(0, "input must contain valid UTF-8"))?;
    parse_component(text, semantic_roots)
}

#[derive(Clone, Copy)]
enum NodeKindDiscriminant {
    State,
}

fn require_kind(
    nodes: &BTreeMap<NodeId, Node>,
    id: NodeId,
    expected: NodeKindDiscriminant,
    line: usize,
) -> Result<(), ParseError> {
    let node = nodes
        .get(&id)
        .ok_or_else(|| ParseError::new(line, format!("unknown or non-prior node {id}")))?;
    let valid = matches!(
        (expected, &node.kind),
        (NodeKindDiscriminant::State, NodeKind::State)
    );
    if valid {
        Ok(())
    } else {
        Err(ParseError::new(
            line,
            format!("node {id} has the wrong kind"),
        ))
    }
}

fn require_width(
    nodes: &BTreeMap<NodeId, Node>,
    id: NodeId,
    width: u32,
    line: usize,
) -> Result<(), ParseError> {
    let actual = nodes
        .get(&id)
        .ok_or_else(|| ParseError::new(line, format!("unknown or non-prior node {id}")))?
        .width;
    if actual == width {
        Ok(())
    } else {
        Err(ParseError::new(
            line,
            format!("node {id} width {actual} does not match {width}"),
        ))
    }
}

fn parse_node_kind(
    operation: &str,
    width: u32,
    nodes: &BTreeMap<NodeId, Node>,
    tokens: &mut impl Iterator<Item = String>,
    line: usize,
) -> Result<(NodeKind, Option<String>), ParseError> {
    let kind = match operation {
        "input" | "state" => {
            let symbol = tokens.next();
            expect_end(tokens, line)?;
            return Ok((
                if operation == "input" {
                    NodeKind::Input
                } else {
                    NodeKind::State
                },
                symbol,
            ));
        }
        "zero" => NodeKind::Constant(0),
        "one" => NodeKind::Constant(1),
        "ones" => NodeKind::Constant(width_mask(width)),
        "const" | "constd" | "consth" => {
            let literal = tokens
                .next()
                .ok_or_else(|| ParseError::new(line, "missing constant"))?;
            NodeKind::Constant(parse_constant(operation, &literal, width, line)?)
        }
        "not" | "inc" | "dec" | "neg" => {
            let operand = parse_u64(tokens.next().as_deref(), line, "unary operand")?;
            require_width(nodes, operand, width, line)?;
            let operator = match operation {
                "not" => UnaryOp::Not,
                "inc" => UnaryOp::Inc,
                "dec" => UnaryOp::Dec,
                _ => UnaryOp::Neg,
            };
            NodeKind::Unary(operator, operand)
        }
        "redor" | "redand" => {
            if width != 1 {
                return Err(ParseError::new(
                    line,
                    "reduction result sort must be bit-vector 1",
                ));
            }
            let operand = parse_u64(tokens.next().as_deref(), line, "reduction operand")?;
            if !nodes.contains_key(&operand) {
                return Err(ParseError::new(
                    line,
                    "unknown or non-prior reduction operand",
                ));
            }
            NodeKind::Unary(
                if operation == "redor" {
                    UnaryOp::Redor
                } else {
                    UnaryOp::Redand
                },
                operand,
            )
        }
        "and" | "or" | "xor" | "add" | "sub" | "mul" | "sll" | "srl" => {
            let left = parse_u64(tokens.next().as_deref(), line, "left operand")?;
            let right = parse_u64(tokens.next().as_deref(), line, "right operand")?;
            require_width(nodes, left, width, line)?;
            require_width(nodes, right, width, line)?;
            let operator = match operation {
                "and" => BinaryOp::And,
                "or" => BinaryOp::Or,
                "xor" => BinaryOp::Xor,
                "add" => BinaryOp::Add,
                "sub" => BinaryOp::Sub,
                "mul" => BinaryOp::Mul,
                "sll" => BinaryOp::Sll,
                _ => BinaryOp::Srl,
            };
            NodeKind::Binary(operator, left, right)
        }
        "eq" | "neq" | "ult" | "ulte" | "ugt" | "ugte" => {
            if width != 1 {
                return Err(ParseError::new(
                    line,
                    "comparison result sort must be bit-vector 1",
                ));
            }
            let left = parse_u64(tokens.next().as_deref(), line, "left operand")?;
            let right = parse_u64(tokens.next().as_deref(), line, "right operand")?;
            let operand_width = nodes
                .get(&left)
                .ok_or_else(|| ParseError::new(line, "unknown comparison operand"))?
                .width;
            require_width(nodes, right, operand_width, line)?;
            let operator = match operation {
                "eq" => BinaryOp::Eq,
                "neq" => BinaryOp::Neq,
                "ult" => BinaryOp::Ult,
                "ulte" => BinaryOp::Ulte,
                "ugt" => BinaryOp::Ugt,
                _ => BinaryOp::Ugte,
            };
            NodeKind::Binary(operator, left, right)
        }
        "ite" => {
            let condition = parse_u64(tokens.next().as_deref(), line, "condition")?;
            let then_value = parse_u64(tokens.next().as_deref(), line, "then operand")?;
            let else_value = parse_u64(tokens.next().as_deref(), line, "else operand")?;
            require_width(nodes, condition, 1, line)?;
            require_width(nodes, then_value, width, line)?;
            require_width(nodes, else_value, width, line)?;
            NodeKind::Ite(condition, then_value, else_value)
        }
        "slice" => {
            let value = parse_u64(tokens.next().as_deref(), line, "slice operand")?;
            let upper = parse_u64(tokens.next().as_deref(), line, "slice upper")?;
            let lower = parse_u64(tokens.next().as_deref(), line, "slice lower")?;
            let source_width = nodes
                .get(&value)
                .ok_or_else(|| ParseError::new(line, "unknown slice operand"))?
                .width;
            if upper >= u64::from(source_width)
                || lower > upper
                || upper - lower + 1 != u64::from(width)
            {
                return Err(ParseError::new(
                    line,
                    "invalid slice bounds or result width",
                ));
            }
            NodeKind::Slice {
                value,
                upper: upper as u32,
                lower: lower as u32,
            }
        }
        "uext" => {
            let value = parse_u64(tokens.next().as_deref(), line, "extension operand")?;
            let amount = parse_u64(tokens.next().as_deref(), line, "extension amount")?;
            let source_width = nodes
                .get(&value)
                .ok_or_else(|| ParseError::new(line, "unknown extension operand"))?
                .width;
            if u64::from(source_width) + amount != u64::from(width) {
                return Err(ParseError::new(
                    line,
                    "extension amount disagrees with result width",
                ));
            }
            NodeKind::Uext {
                value,
                amount: amount as u32,
            }
        }
        "concat" => {
            let high = parse_u64(tokens.next().as_deref(), line, "high operand")?;
            let low = parse_u64(tokens.next().as_deref(), line, "low operand")?;
            let high_width = nodes
                .get(&high)
                .ok_or_else(|| ParseError::new(line, "unknown high concat operand"))?
                .width;
            let low_width = nodes
                .get(&low)
                .ok_or_else(|| ParseError::new(line, "unknown low concat operand"))?
                .width;
            if high_width.checked_add(low_width) != Some(width) {
                return Err(ParseError::new(
                    line,
                    "concat operand widths disagree with result width",
                ));
            }
            NodeKind::Concat { high, low }
        }
        _ => {
            return Err(ParseError::new(
                line,
                format!("unsupported operation `{operation}`"),
            ));
        }
    };
    let symbol = tokens.next();
    expect_end(tokens, line)?;
    Ok((kind, symbol))
}

fn parse_constant(
    operation: &str,
    literal: &str,
    width: u32,
    line: usize,
) -> Result<u64, ParseError> {
    let (radix, digits) = match operation {
        "const" => (2, literal),
        "consth" => (16, literal),
        _ => (10, literal),
    };
    if digits.is_empty() || digits.starts_with('-') {
        return Err(ParseError::new(
            line,
            "constant must be canonical unsigned text",
        ));
    }
    if operation == "const" && digits.len() != width as usize {
        return Err(ParseError::new(
            line,
            "binary constant width is not canonical",
        ));
    }
    if operation == "consth" && digits.len() != (width as usize).div_ceil(4) {
        return Err(ParseError::new(
            line,
            "hexadecimal constant width is not canonical",
        ));
    }
    if digits.len() > 1 && digits.starts_with('0') && operation == "constd" {
        return Err(ParseError::new(line, "decimal constant has a leading zero"));
    }
    let value = u64::from_str_radix(digits, radix)
        .map_err(|_| ParseError::new(line, "invalid or overflowing constant"))?;
    if value & !width_mask(width) != 0 {
        return Err(ParseError::new(line, "constant does not fit its sort"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATCHDOG: &str = "1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 kick\n4 zero 2\n5 state 2 timer\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n11 constd 2 3\n12 ugte 1 5 11\n13 bad 12 expired\n";

    #[test]
    fn parses_and_evaluates_word_level_watchdog() {
        let model = parse(WATCHDOG).unwrap();
        assert_eq!(model.inputs(), &[3]);
        assert_eq!(model.states(), &[5]);
        let mut state = model.initial_state().unwrap();
        let mut inputs = BTreeMap::from([(3, 0)]);
        assert!(model.active_bad(&state, &inputs).unwrap().is_empty());
        state = model.step(&state, &inputs).unwrap();
        state = model.step(&state, &inputs).unwrap();
        state = model.step(&state, &inputs).unwrap();
        assert_eq!(model.active_bad(&state, &inputs).unwrap(), vec![13]);
        inputs.insert(3, 1);
        assert_eq!(model.step(&state, &inputs).unwrap().get(&5), Some(&0));
    }

    #[test]
    fn accepts_yosys_outputs_and_prunes_unused_clock_inputs() {
        let source = "1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 reset\n4 input 1 clk\n5 zero 2\n6 state 2 count\n7 init 2 6 5\n8 one 2\n9 add 2 6 8\n10 ite 2 3 5 9\n11 next 2 6 10\n12 constd 2 9\n13 ugte 1 6 12\n14 output 13 bad\n15 bad 13 watchdog\n";
        let model = parse(source).unwrap();
        assert_eq!(model.inputs(), &[3]);
        assert_eq!(model.states(), &[6]);
        assert_eq!(
            model.bad_properties(),
            &[(15, 13, Some("watchdog".to_string()))]
        );

        for hostile in [
            source.replace("14 output 13 bad", "14 output 99 bad"),
            source.replace("14 output 13 bad", "14 output 13 bad extra"),
            source.replace("14 output 13 bad", "14 output"),
        ] {
            assert!(parse(&hostile).is_err());
        }
    }

    #[test]
    fn component_parser_accepts_property_free_sources_and_binds_selected_roots() {
        let source = "1 sort bitvec 1\n2 input 1 command\n3 state 1 state\n4 zero 1\n5 init 1 3 4\n6 next 1 3 3\n7 xor 1 3 2\n8 output 7 projected\n";
        assert_eq!(
            parse(source).unwrap_err().message,
            "model requires at least one state and one bad property"
        );
        let component = parse_component(source, &[7]).unwrap();
        assert_eq!(component.states(), &[3]);
        assert_eq!(component.inputs(), &[2]);
        assert!(component.bad_properties().is_empty());

        assert_eq!(
            parse_component(source, &[99]).unwrap_err().message,
            "unknown component semantic root 99"
        );
        assert_eq!(
            parse_component(source, &[7, 7]).unwrap_err().message,
            "component semantic roots must be unique and strictly ordered"
        );
    }

    #[test]
    fn accepts_one_yosys_symbol_on_state_edges_and_rejects_extra_tokens() {
        let source = "1 sort bitvec 1\n2 state 1 held\n3 zero 1\n4 init 1 2 3 init_symbol\n5 next 1 2 2 next_symbol\n6 bad 2 property\n";
        let model = parse(source).unwrap();
        assert_eq!(model.initialiser_symbol(2), Some("init_symbol"));
        assert_eq!(model.next_symbol(2), Some("next_symbol"));
        assert_eq!(model.next_symbol(99), None);
        assert!(parse(&source.replace("next_symbol", "next_symbol extra")).is_err());
        assert!(parse(&source.replace("init_symbol", "init_symbol extra")).is_err());
    }

    #[test]
    fn semantic_input_support_is_iterative_for_deep_valid_graphs() {
        let mut source = String::from("1 sort bitvec 1\n2 input 1 signal\n");
        let mut previous = 2;
        for id in 3..20_003 {
            source.push_str(&format!("{id} not 1 {previous}\n"));
            previous = id;
        }
        source.push_str(&format!(
            "20003 state 1 held\n20004 zero 1\n20005 init 1 20003 20004\n20006 next 1 20003 20003\n20007 bad {previous} deep\n"
        ));

        let model = parse(&source).unwrap();
        assert_eq!(model.inputs(), &[2]);
    }

    #[test]
    fn arithmetic_wraps_at_declared_word_width() {
        let model = parse(WATCHDOG).unwrap();
        let state = BTreeMap::from([(5, 255)]);
        let inputs = BTreeMap::from([(3, 0)]);
        assert_eq!(model.step(&state, &inputs).unwrap().get(&5), Some(&0));
    }

    #[test]
    fn rejects_unsupported_arrays_and_operations() {
        let error = parse("1 sort array 2 2\n").unwrap_err();
        assert!(error.message.contains("only bit-vector"));
        let error = parse("1 sort bitvec 1\n2 state 1 s\n3 zero 1\n4 init 1 2 3\n5 next 1 2 3\n6 udiv 1 2 2\n7 bad 6\n").unwrap_err();
        assert!(error.message.contains("unsupported operation"));
    }

    #[test]
    fn logical_shifts_match_btor2_word_semantics() {
        let source = "1 sort bitvec 1\n2 sort bitvec 4\n3 state 2 word\n4 state 2 amount\n5 zero 2\n6 init 2 3 5\n7 init 2 4 5\n8 next 2 3 3\n9 next 2 4 4\n10 sll 2 3 4 left\n11 srl 2 3 4 right\n12 redor 1 10\n13 bad 12 shifted\n";
        let model = parse(source).unwrap();
        let inputs = BTreeMap::new();
        for (word, amount, left, right) in [
            (0b0011, 1, 0b0110, 0b0001),
            (0b1001, 3, 0b1000, 0b0001),
            (0b1111, 4, 0, 0),
            (0b1111, 15, 0, 0),
        ] {
            let state = BTreeMap::from([(3, word), (4, amount)]);
            assert_eq!(model.evaluate(10, &state, &inputs).unwrap(), left);
            assert_eq!(model.evaluate(11, &state, &inputs).unwrap(), right);
        }
        assert!(parse(&source.replace("10 sll 2 3 4", "10 sll 1 3 4")).is_err());
    }

    #[test]
    fn reduction_or_matches_standard_word_semantics() {
        let source = "1 sort bitvec 1\n2 sort bitvec 12\n3 state 2 word\n4 zero 2\n5 init 2 3 4\n6 next 2 3 3\n7 redor 1 3 any_bit\n8 bad 7 reduced\n";
        let model = parse(source).unwrap();
        assert!(
            model
                .active_bad(&BTreeMap::from([(3, 0)]), &BTreeMap::new())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            model
                .active_bad(&BTreeMap::from([(3, 8)]), &BTreeMap::new())
                .unwrap(),
            vec![8]
        );
        assert_eq!(model.nodes()[&7].symbol.as_deref(), Some("any_bit"));
        assert!(parse(&source.replace("7 redor 1", "7 redor 2")).is_err());
        assert!(parse(&source.replace("7 redor 1 3", "7 redor 1 99")).is_err());
    }

    #[test]
    fn reduction_and_matches_standard_word_semantics() {
        let source = "1 sort bitvec 1\n2 sort bitvec 4\n3 state 2 word\n4 zero 2\n5 init 2 3 4\n6 next 2 3 3\n7 redand 1 3 all_bits\n8 bad 7 reduced\n";
        let model = parse(source).unwrap();
        assert!(
            model
                .active_bad(&BTreeMap::from([(3, 7)]), &BTreeMap::new())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            model
                .active_bad(&BTreeMap::from([(3, 15)]), &BTreeMap::new())
                .unwrap(),
            vec![8]
        );
        assert_eq!(model.nodes()[&7].symbol.as_deref(), Some("all_bits"));
        assert!(parse(&source.replace("7 redand 1", "7 redand 2")).is_err());
        assert!(parse(&source.replace("7 redand 1 3", "7 redand 1 99")).is_err());
    }

    #[test]
    fn concat_matches_standard_high_low_word_semantics() {
        let source = "1 sort bitvec 1\n2 sort bitvec 3\n3 sort bitvec 5\n4 sort bitvec 8\n5 state 4 word\n6 zero 4\n7 init 4 5 6\n8 next 4 5 5\n9 const 2 101\n10 const 3 10011\n11 concat 4 9 10 joined\n12 consth 4 b3\n13 eq 1 11 12\n14 bad 13 concat_ok\n";
        let model = parse(source).unwrap();
        assert_eq!(
            model
                .active_bad(&model.initial_state().unwrap(), &BTreeMap::new())
                .unwrap(),
            vec![14]
        );
        assert_eq!(model.nodes()[&11].symbol.as_deref(), Some("joined"));
        assert!(parse(&source.replace("11 concat 4 9 10", "11 concat 3 9 10")).is_err());
        assert!(parse(&source.replace("11 concat 4 9 10", "11 concat 4 99 10")).is_err());
        assert!(parse(&source.replace("11 concat 4 9 10", "11 concat 4 9 99")).is_err());
    }

    #[test]
    fn rejects_noncanonical_and_ill_typed_models() {
        assert!(parse(&WATCHDOG.replace("12 ugte 1", "12 ugte 2")).is_err());
        assert!(parse(WATCHDOG.trim_end()).is_err());
        assert!(parse(&WATCHDOG.replace("8 add 2 5 7", "8 add 2 5 3")).is_err());
        assert!(parse(&WATCHDOG.replace("10 next 2 5 9", "10 next 2 5 9\n10 next 2 5 9")).is_err());
    }

    #[test]
    fn constraints_fail_closed_before_transition_or_property() {
        let source = WATCHDOG.replace("13 bad 12 expired", "13 constraint 3\n14 bad 12 expired");
        let model = parse(&source).unwrap();
        let state = model.initial_state().unwrap();
        let inputs = BTreeMap::from([(3, 0)]);
        assert_eq!(
            model.step(&state, &inputs),
            Err(EvalError::ConstraintViolation(13))
        );
    }

    #[test]
    fn byte_boundary_rejects_invalid_utf8() {
        assert!(parse_bytes(&[0xff, b'\n']).is_err());
    }
}
