use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::Instant;
use varisat::{ExtendFormula, Lit, Solver, Var};

const RTL_ARTIFACT_SCHEMA_VERSION: usize = 4;
const FIRMWARE_CLI_CONTRACT_VERSION: usize = 2;
const AAG_INPUT_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
const YOSYS_MEMORY_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const YOSYS_FILE_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
const EVIDENCE_TOTAL_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn synthesis_memory_limit_kind() -> &'static str {
    if cfg!(target_os = "macos") {
        "unavailable"
    } else {
        "address-space"
    }
}

fn synthesis_memory_limit_bytes() -> u64 {
    if cfg!(target_os = "macos") {
        0
    } else {
        YOSYS_MEMORY_LIMIT_BYTES
    }
}

#[derive(Clone, Debug)]
struct Clause(Vec<(usize, bool)>);

#[derive(Clone, Debug)]
struct CachedHelperFormula {
    vars: usize,
    ratio: usize,
    family: String,
    seed: u64,
    clauses: Vec<Clause>,
    aligned_nodes: usize,
    order_nodes: usize,
    helper_nodes: usize,
}

type Literal = (usize, bool);

#[derive(Clone, Debug)]
struct Factor {
    scope: Vec<usize>,
    values: Vec<bool>,
}

impl Factor {
    fn from_clause(clause: &Clause) -> Self {
        let mut scope: Vec<_> = clause.0.iter().map(|&(v, _)| v).collect();
        scope.sort_unstable();
        scope.dedup();
        let values = (0..(1usize << scope.len()))
            .map(|bits| {
                clause.0.iter().any(|&(v, positive)| {
                    let pos = scope.binary_search(&v).unwrap();
                    (((bits >> pos) & 1) == 1) == positive
                })
            })
            .collect();
        Self { scope, values }
    }

    fn evaluate_from(&self, vars: &[usize], bits: usize) -> bool {
        let mut local = 0;
        for (i, &v) in self.scope.iter().enumerate() {
            let source = vars.binary_search(&v).unwrap();
            local |= ((bits >> source) & 1) << i;
        }
        self.values[local]
    }
}

#[derive(Debug)]
struct Layer {
    variable: usize,
    boundary: Vec<usize>,
    witness: Vec<Option<bool>>,
    bdd_nodes: usize,
}

#[derive(Debug)]
struct SolveResult {
    assignment: Option<Vec<bool>>,
    peak_boundary: usize,
    peak_entries: usize,
    peak_bdd_nodes: usize,
    layers: Vec<Layer>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct BddNode {
    variable: usize,
    low: usize,
    high: usize,
}

#[derive(Clone, Default)]
struct BddManager {
    nodes: Vec<BddNode>,
    node_hits: Vec<usize>,
    unique: HashMap<BddNode, usize>,
    apply_cache: HashMap<(bool, usize, usize), usize>,
    node_limit: Option<usize>,
    deadline: Option<Instant>,
    budget_exceeded: bool,
}

impl BddManager {
    fn node(&self, id: usize) -> BddNode {
        self.nodes[id - 2]
    }

    fn make(&mut self, variable: usize, low: usize, high: usize) -> usize {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.budget_exceeded = true;
            return 0;
        }
        if low == high {
            return low;
        }
        let node = BddNode {
            variable,
            low,
            high,
        };
        if let Some(&id) = self.unique.get(&node) {
            self.node_hits[id - 2] += 1;
            return id;
        }
        if self
            .node_limit
            .is_some_and(|limit| self.nodes.len() >= limit)
        {
            self.budget_exceeded = true;
            return 0;
        }
        let id = self.nodes.len() + 2;
        self.nodes.push(node);
        self.node_hits.push(0);
        self.unique.insert(node, id);
        id
    }

    fn literal(&mut self, variable: usize, positive: bool) -> usize {
        if positive {
            self.make(variable, 0, 1)
        } else {
            self.make(variable, 1, 0)
        }
    }

    fn apply(&mut self, is_and: bool, a: usize, b: usize) -> usize {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let terminal = if is_and {
            if a == 0 || b == 0 {
                Some(0)
            } else if a == 1 {
                Some(b)
            } else if a == b {
                Some(a)
            } else {
                None
            }
        } else if a == 1 || b == 1 {
            Some(1)
        } else if a == 0 {
            Some(b)
        } else if a == b {
            Some(a)
        } else {
            None
        };
        if let Some(value) = terminal {
            return value;
        }
        if let Some(&result) = self.apply_cache.get(&(is_and, a, b)) {
            if result >= 2 {
                self.node_hits[result - 2] += 1;
            }
            return result;
        }
        let av = self.node(a).variable;
        let bv = self.node(b).variable;
        let variable = av.min(bv);
        let an = self.node(a);
        let bn = self.node(b);
        let (al, ah) = if av == variable {
            (an.low, an.high)
        } else {
            (a, a)
        };
        let (bl, bh) = if bv == variable {
            (bn.low, bn.high)
        } else {
            (b, b)
        };
        let low = self.apply(is_and, al, bl);
        let high = self.apply(is_and, ah, bh);
        let result = self.make(variable, low, high);
        self.apply_cache.insert((is_and, a, b), result);
        result
    }

    fn and(&mut self, a: usize, b: usize) -> usize {
        self.apply(true, a, b)
    }

    fn or(&mut self, a: usize, b: usize) -> usize {
        self.apply(false, a, b)
    }

    fn exists(&mut self, root: usize, variable: usize, memo: &mut HashMap<usize, usize>) -> usize {
        if root < 2 {
            return root;
        }
        if let Some(&result) = memo.get(&root) {
            return result;
        }
        let node = self.node(root);
        let result = if node.variable > variable {
            root
        } else if node.variable == variable {
            self.or(node.low, node.high)
        } else {
            let low = self.exists(node.low, variable, memo);
            let high = self.exists(node.high, variable, memo);
            self.make(node.variable, low, high)
        };
        memo.insert(root, result);
        result
    }

    fn evaluate(&self, mut root: usize, assignment: &[bool]) -> bool {
        while root >= 2 {
            let node = self.node(root);
            root = if assignment[node.variable] {
                node.high
            } else {
                node.low
            };
        }
        root == 1
    }

    fn negate(&mut self, root: usize, memo: &mut HashMap<usize, usize>) -> usize {
        if root < 2 {
            return 1 - root;
        }
        if let Some(&result) = memo.get(&root) {
            return result;
        }
        let node = self.node(root);
        let low = self.negate(node.low, memo);
        let high = self.negate(node.high, memo);
        let result = self.make(node.variable, low, high);
        memo.insert(root, result);
        result
    }

    fn satisfying_assignment(&self, mut root: usize, variables: usize) -> Option<Vec<bool>> {
        if root == 0 {
            return None;
        }
        let mut assignment = vec![false; variables];
        while root >= 2 {
            let node = self.node(root);
            if node.low != 0 {
                assignment[node.variable] = false;
                root = node.low;
            } else {
                assignment[node.variable] = true;
                root = node.high;
            }
        }
        (root == 1).then_some(assignment)
    }
}

#[derive(Debug)]
struct RingStatePoint {
    ring: i32,
    processed: usize,
    residual_states: usize,
}

fn compile_formula_bdd(
    vars: usize,
    clauses: &[Clause],
    bdd_order: &[usize],
) -> (BddManager, usize) {
    let mut manager = BddManager::default();
    let formula = compile_formula_bdd_into(&mut manager, vars, clauses, bdd_order);
    (manager, formula)
}

fn compile_formula_bdd_into(
    manager: &mut BddManager,
    vars: usize,
    clauses: &[Clause],
    bdd_order: &[usize],
) -> usize {
    let mut rank = vec![usize::MAX; vars];
    for (level, &variable) in bdd_order.iter().enumerate() {
        rank[variable] = level;
    }
    let mut formula = 1;
    for clause in clauses {
        let mut clause_root = 0;
        for &(variable, positive) in &clause.0 {
            let literal = manager.literal(rank[variable], positive);
            clause_root = manager.or(clause_root, literal);
        }
        formula = manager.and(formula, clause_root);
    }
    formula
}

fn residual_state_count(manager: &BddManager, root: usize, prefix: usize) -> usize {
    let mut states = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(current) = stack.pop() {
        if current < 2 {
            states.insert(current);
            continue;
        }
        if !visited.insert(current) {
            continue;
        }
        let node = manager.node(current);
        if node.variable >= prefix {
            states.insert(current);
        } else {
            stack.push(node.low);
            stack.push(node.high);
        }
    }
    states.len()
}

fn flower_ring_state_profile(
    vars: usize,
    clauses: &[Clause],
    outside_in: bool,
) -> (usize, Vec<RingStatePoint>) {
    let coordinates = flower_coordinates(vars);
    let distance = |v: usize| {
        let (q, r) = coordinates[v];
        q.abs().max(r.abs()).max((q + r).abs())
    };
    let mut order: Vec<_> = (0..vars).collect();
    order.sort_by_key(|&v| {
        if outside_in {
            (std::cmp::Reverse(distance(v)), v)
        } else {
            (std::cmp::Reverse(-distance(v)), v)
        }
    });
    let (manager, root) = compile_formula_bdd(vars, clauses, &order);
    let mut profile = Vec::new();
    let mut processed = 0;
    while processed < vars {
        let ring = distance(order[processed]);
        while processed < vars && distance(order[processed]) == ring {
            processed += 1;
        }
        profile.push(RingStatePoint {
            ring,
            processed,
            residual_states: residual_state_count(&manager, root, processed),
        });
    }
    (manager.nodes.len(), profile)
}

#[derive(Debug)]
struct BddSolveResult {
    assignment: Option<Vec<bool>>,
    allocated_nodes: usize,
    live_nodes: usize,
    interaction_candidates: Vec<((Literal, Literal), usize)>,
}

#[derive(Clone)]
struct ProvenanceFactor {
    scope: Vec<usize>,
    root: usize,
    clauses: BTreeSet<usize>,
}

struct IncrementalBddCache {
    vars: usize,
    manager: BddManager,
    checkpoint_layers: Vec<usize>,
    checkpoints: Vec<Vec<ProvenanceFactor>>,
    recovery: Vec<(usize, usize)>,
    assignment: Option<Vec<bool>>,
}

struct DirectVariantResult {
    assignment: Option<Vec<bool>>,
    nodes: usize,
}

fn solve_tuple_natural(vars: usize, clauses: &[Clause]) -> DirectVariantResult {
    let mut manager = BddManager::default();
    let mut factors: Vec<(Vec<usize>, usize)> = clauses
        .iter()
        .map(|clause| {
            let (scope, root) = compile_clause_into(&mut manager, clause);
            (scope, root)
        })
        .collect();
    let mut recovery = Vec::with_capacity(vars);
    for variable in 0..vars {
        let mut combined = 1;
        let mut boundary = BTreeSet::new();
        let mut retained = Vec::new();
        for (scope, root) in factors {
            if scope.contains(&variable) {
                combined = manager.and(combined, root);
                boundary.extend(scope.into_iter().filter(|&item| item != variable));
            } else {
                retained.push((scope, root));
            }
        }
        let projected = manager.exists(combined, variable, &mut HashMap::new());
        retained.push((boundary.into_iter().collect(), projected));
        factors = retained;
        recovery.push((variable, combined));
    }
    let satisfiable = factors.iter().all(|factor| factor.1 == 1);
    let assignment = satisfiable.then(|| {
        let mut assignment = vec![false; vars];
        for &(variable, combined) in recovery.iter().rev() {
            if !manager.evaluate(combined, &assignment) {
                assignment[variable] = true;
                assert!(manager.evaluate(combined, &assignment));
            }
        }
        assignment
    });
    DirectVariantResult {
        assignment,
        nodes: manager.nodes.len(),
    }
}

fn solve_provenance_variant(
    vars: usize,
    clauses: &[Clause],
    keep_provenance: bool,
    store_checkpoints: bool,
    stride: usize,
) -> DirectVariantResult {
    let mut manager = BddManager::default();
    let mut factors: Vec<_> = clauses
        .iter()
        .enumerate()
        .map(|(index, clause)| {
            let (scope, root) = compile_clause_into(&mut manager, clause);
            ProvenanceFactor {
                scope,
                root,
                clauses: if keep_provenance {
                    BTreeSet::from([index])
                } else {
                    BTreeSet::new()
                },
            }
        })
        .collect();
    let mut recovery = Vec::with_capacity(vars);
    let mut _checkpoints = Vec::new();
    for variable in 0..vars {
        if store_checkpoints && variable % stride.max(1) == 0 {
            _checkpoints.push(factors.clone());
        }
        let mut combined = 1;
        let mut boundary = BTreeSet::new();
        let mut provenance = BTreeSet::new();
        let mut retained = Vec::new();
        for factor in factors {
            if factor.scope.contains(&variable) {
                combined = manager.and(combined, factor.root);
                boundary.extend(factor.scope.into_iter().filter(|&item| item != variable));
                if keep_provenance {
                    provenance.extend(factor.clauses);
                }
            } else {
                retained.push(factor);
            }
        }
        let projected = manager.exists(combined, variable, &mut HashMap::new());
        retained.push(ProvenanceFactor {
            scope: boundary.into_iter().collect(),
            root: projected,
            clauses: provenance,
        });
        factors = retained;
        recovery.push((variable, combined));
    }
    let satisfiable = factors.iter().all(|factor| factor.root == 1);
    let assignment = satisfiable.then(|| {
        let mut assignment = vec![false; vars];
        for &(variable, combined) in recovery.iter().rev() {
            if !manager.evaluate(combined, &assignment) {
                assignment[variable] = true;
                assert!(manager.evaluate(combined, &assignment));
            }
        }
        assignment
    });
    DirectVariantResult {
        assignment,
        nodes: manager.nodes.len(),
    }
}

struct BranchEvaluation {
    satisfiable: bool,
    branches: usize,
    reuse_us: u128,
    fresh_us: u128,
    reuse_nodes: usize,
    fresh_nodes: usize,
    cache_build_us: u128,
    sibling_us: u128,
    cache_nodes: usize,
    sibling_new_nodes: usize,
    checkpoint_count: usize,
    restored_layer: usize,
    replayed_layers: usize,
    first_satisfiable: bool,
    valid: bool,
}

fn preferred_branch_sign(clauses: &[Clause], variable: usize) -> bool {
    let positive = clauses
        .iter()
        .flat_map(|clause| &clause.0)
        .filter(|&&(item, sign)| item == variable && sign)
        .count();
    let negative = clauses
        .iter()
        .flat_map(|clause| &clause.0)
        .filter(|&&(item, sign)| item == variable && !sign)
        .count();
    positive >= negative
}

fn evaluate_branch_choice(
    vars: usize,
    clauses: &[Clause],
    variable: usize,
    stride: usize,
) -> BranchEvaluation {
    let preferred = preferred_branch_sign(clauses, variable);
    let mut first_formula = clauses.to_vec();
    first_formula.push(Clause(vec![(variable, preferred)]));
    let branch_clause = first_formula.len() - 1;
    let sibling_clause = Clause(vec![(variable, !preferred)]);
    let mut sibling_formula = first_formula.clone();
    sibling_formula[branch_clause] = sibling_clause.clone();

    let cache_start = Instant::now();
    let mut cache = build_incremental_bdd_cache_with_stride(vars, &first_formula, stride);
    let cache_build_us = cache_start.elapsed().as_micros();
    let cache_nodes = cache.manager.nodes.len();
    let checkpoint_count = cache.checkpoints.len();
    let first_sat = cache.assignment.is_some();
    let first_valid = cache
        .assignment
        .as_ref()
        .is_none_or(|assignment| satisfies(&first_formula, assignment));
    let (sibling_assignment, branches, sibling_us, sibling_new_nodes, restored_layer) = if first_sat
    {
        (None, 1, 0, 0, vars)
    } else {
        let sibling_start = Instant::now();
        let (assignment, new_nodes, _, restored) =
            incremental_clause_update(&mut cache, branch_clause, &sibling_clause);
        (
            assignment,
            2,
            sibling_start.elapsed().as_micros(),
            new_nodes,
            restored,
        )
    };
    let reuse_us = cache_build_us + sibling_us;
    let sibling_valid = sibling_assignment
        .as_ref()
        .is_none_or(|assignment| satisfies(&sibling_formula, assignment));
    let satisfiable = first_sat || sibling_assignment.is_some();
    let reuse_nodes = cache.manager.nodes.len();

    let natural: Vec<_> = (0..vars).collect();
    let fresh_start = Instant::now();
    let fresh_first = eliminate_with_bdds(vars, &first_formula, &natural);
    let mut fresh_nodes = fresh_first.allocated_nodes;
    let fresh_satisfiable = if fresh_first.assignment.is_some() {
        true
    } else {
        let fresh_sibling = eliminate_with_bdds(vars, &sibling_formula, &natural);
        fresh_nodes += fresh_sibling.allocated_nodes;
        fresh_sibling.assignment.is_some()
    };
    let fresh_us = fresh_start.elapsed().as_micros();
    BranchEvaluation {
        satisfiable,
        branches,
        reuse_us,
        fresh_us,
        reuse_nodes,
        fresh_nodes,
        cache_build_us,
        sibling_us,
        cache_nodes,
        sibling_new_nodes,
        checkpoint_count,
        restored_layer,
        replayed_layers: if first_sat { 0 } else { vars - restored_layer },
        first_satisfiable: first_sat,
        valid: first_valid && sibling_valid && satisfiable == fresh_satisfiable,
    }
}

fn evaluate_forced_branch_pair(
    vars: usize,
    clauses: &[Clause],
    variable: usize,
    stride: usize,
) -> BranchEvaluation {
    let preferred = preferred_branch_sign(clauses, variable);
    let mut first_formula = clauses.to_vec();
    first_formula.push(Clause(vec![(variable, preferred)]));
    let branch_clause = first_formula.len() - 1;
    let sibling_clause = Clause(vec![(variable, !preferred)]);
    let mut sibling_formula = first_formula.clone();
    sibling_formula[branch_clause] = sibling_clause.clone();
    let cache_start = Instant::now();
    let mut cache = build_incremental_bdd_cache_with_stride(vars, &first_formula, stride);
    let cache_build_us = cache_start.elapsed().as_micros();
    let cache_nodes = cache.manager.nodes.len();
    let checkpoint_count = cache.checkpoints.len();
    let first_assignment = cache.assignment.clone();
    let sibling_start = Instant::now();
    let (sibling_assignment, sibling_new_nodes, _, restored_layer) =
        incremental_clause_update(&mut cache, branch_clause, &sibling_clause);
    let sibling_us = sibling_start.elapsed().as_micros();
    let reuse_us = cache_build_us + sibling_us;
    let satisfiable = first_assignment.is_some() || sibling_assignment.is_some();
    let valid = first_assignment
        .as_ref()
        .is_none_or(|assignment| satisfies(&first_formula, assignment))
        && sibling_assignment
            .as_ref()
            .is_none_or(|assignment| satisfies(&sibling_formula, assignment));
    BranchEvaluation {
        satisfiable,
        branches: 2,
        reuse_us,
        fresh_us: 0,
        reuse_nodes: cache.manager.nodes.len(),
        fresh_nodes: 0,
        cache_build_us,
        sibling_us,
        cache_nodes,
        sibling_new_nodes,
        checkpoint_count,
        restored_layer,
        replayed_layers: vars - restored_layer,
        first_satisfiable: first_assignment.is_some(),
        valid,
    }
}

fn branching_scores(vars: usize, clauses: &[Clause], alpha: f64) -> Vec<f64> {
    let graph = primal_graph(vars, clauses);
    let mut positive = vec![0usize; vars];
    let mut negative = vec![0usize; vars];
    for clause in clauses {
        for &(variable, sign) in &clause.0 {
            if sign {
                positive[variable] += 1
            } else {
                negative[variable] += 1
            }
        }
    }
    (0..vars)
        .map(|variable| {
            let occurrence = positive[variable] + negative[variable];
            let balance = 1 + positive[variable].min(negative[variable]);
            let impact =
                (1 + occurrence) as f64 * (1 + graph[variable].len()) as f64 * balance as f64;
            let recomputation = (vars - variable).max(1) as f64;
            impact / recomputation.powf(alpha)
        })
        .collect()
}

fn supervised_branch_feature_matrix(vars: usize, clauses: &[Clause]) -> Vec<Vec<f64>> {
    let graph = primal_graph(vars, clauses);
    let scent = diffuse_scent_variant(vars, clauses, 2, 0);
    let scent_mean = scent.iter().sum::<f64>() / scent.len().max(1) as f64;
    let mut positive = vec![0usize; vars];
    let mut negative = vec![0usize; vars];
    let mut earliest_sum = vec![0usize; vars];
    let mut containing = vec![0usize; vars];
    for clause in clauses {
        let earliest = clause.0.iter().map(|literal| literal.0).min().unwrap_or(0);
        for &(variable, sign) in &clause.0 {
            containing[variable] += 1;
            earliest_sum[variable] += earliest;
            if sign {
                positive[variable] += 1
            } else {
                negative[variable] += 1
            }
        }
    }
    (0..vars)
        .map(|variable| {
            let neighbours: Vec<_> = graph[variable].iter().copied().collect();
            let neighbour_mean =
                neighbours.iter().sum::<usize>() as f64 / neighbours.len().max(1) as f64;
            let neighbour_span = neighbours
                .iter()
                .map(|&other| variable.abs_diff(other))
                .sum::<usize>() as f64
                / neighbours.len().max(1) as f64;
            let occurrence = positive[variable] + negative[variable];
            vec![
                1.0,
                variable as f64 / vars.max(1) as f64,
                occurrence as f64 / clauses.len().max(1) as f64,
                graph[variable].len() as f64 / vars.max(1) as f64,
                positive[variable].min(negative[variable]) as f64 / occurrence.max(1) as f64,
                positive[variable] as f64 / occurrence.max(1) as f64,
                neighbour_mean / vars.max(1) as f64,
                neighbour_span / vars.max(1) as f64,
                scent[variable] / scent_mean.max(1e-9),
                earliest_sum[variable] as f64
                    / containing[variable].max(1) as f64
                    / vars.max(1) as f64,
            ]
        })
        .collect()
}

fn compile_clause_into(manager: &mut BddManager, clause: &Clause) -> (Vec<usize>, usize) {
    let mut root = 0;
    let mut scope = Vec::new();
    for &(variable, positive) in &clause.0 {
        let literal = manager.literal(variable, positive);
        root = manager.or(root, literal);
        scope.push(variable);
    }
    scope.sort_unstable();
    scope.dedup();
    (scope, root)
}

fn build_incremental_bdd_cache(vars: usize, clauses: &[Clause]) -> IncrementalBddCache {
    build_incremental_bdd_cache_with_stride(vars, clauses, 1)
}

fn build_incremental_bdd_cache_with_stride(
    vars: usize,
    clauses: &[Clause],
    stride: usize,
) -> IncrementalBddCache {
    let mut manager = BddManager::default();
    let mut factors: Vec<_> = clauses
        .iter()
        .enumerate()
        .map(|(index, clause)| {
            let (scope, root) = compile_clause_into(&mut manager, clause);
            ProvenanceFactor {
                scope,
                root,
                clauses: BTreeSet::from([index]),
            }
        })
        .collect();
    let mut checkpoint_layers = Vec::new();
    let mut checkpoints = Vec::new();
    let mut recovery = Vec::with_capacity(vars);
    for variable in 0..vars {
        if variable % stride.max(1) == 0 {
            checkpoint_layers.push(variable);
            checkpoints.push(factors.clone());
        }
        let mut combined = 1;
        let mut boundary = BTreeSet::new();
        let mut provenance = BTreeSet::new();
        let mut retained = Vec::new();
        for factor in factors {
            if factor.scope.contains(&variable) {
                combined = manager.and(combined, factor.root);
                boundary.extend(factor.scope.into_iter().filter(|&item| item != variable));
                provenance.extend(factor.clauses);
            } else {
                retained.push(factor);
            }
        }
        let projected = manager.exists(combined, variable, &mut HashMap::new());
        retained.push(ProvenanceFactor {
            scope: boundary.into_iter().collect(),
            root: projected,
            clauses: provenance,
        });
        factors = retained;
        recovery.push((variable, combined));
    }
    let satisfiable = factors.iter().all(|factor| factor.root == 1);
    let assignment = satisfiable.then(|| {
        let mut assignment = vec![false; vars];
        for &(variable, combined) in recovery.iter().rev() {
            if !manager.evaluate(combined, &assignment) {
                assignment[variable] = true;
                assert!(manager.evaluate(combined, &assignment));
            }
        }
        assignment
    });
    IncrementalBddCache {
        vars,
        manager,
        checkpoint_layers,
        checkpoints,
        recovery,
        assignment,
    }
}

fn incremental_clause_update(
    cache: &mut IncrementalBddCache,
    clause_index: usize,
    replacement: &Clause,
) -> (Option<Vec<bool>>, usize, usize, usize) {
    let earliest = replacement
        .0
        .iter()
        .map(|literal| literal.0)
        .min()
        .unwrap_or(0);
    let checkpoint_index = cache
        .checkpoint_layers
        .iter()
        .rposition(|&layer| layer <= earliest)
        .unwrap();
    let checkpoint_layer = cache.checkpoint_layers[checkpoint_index];
    let mut factors = cache.checkpoints[checkpoint_index].clone();
    for variable in checkpoint_layer..earliest {
        let mut combined = 1;
        let mut boundary = BTreeSet::new();
        let mut provenance = BTreeSet::new();
        let mut retained = Vec::new();
        for factor in factors {
            if factor.scope.contains(&variable) {
                combined = cache.manager.and(combined, factor.root);
                boundary.extend(factor.scope.into_iter().filter(|&item| item != variable));
                provenance.extend(factor.clauses);
            } else {
                retained.push(factor);
            }
        }
        let projected = cache
            .manager
            .exists(combined, variable, &mut HashMap::new());
        retained.push(ProvenanceFactor {
            scope: boundary.into_iter().collect(),
            root: projected,
            clauses: provenance,
        });
        factors = retained;
    }
    let target = factors
        .iter()
        .position(|factor| factor.clauses.contains(&clause_index))
        .expect("changed clause must remain identifiable before its first variable");
    assert_eq!(factors[target].clauses.len(), 1);
    factors.remove(target);
    let (scope, root) = compile_clause_into(&mut cache.manager, replacement);
    factors.push(ProvenanceFactor {
        scope,
        root,
        clauses: BTreeSet::from([clause_index]),
    });
    let mut recovery = cache.recovery[..earliest].to_vec();
    let initial_nodes = cache.manager.nodes.len();
    for variable in earliest..cache.vars {
        let mut combined = 1;
        let mut boundary = BTreeSet::new();
        let mut provenance = BTreeSet::new();
        let mut retained = Vec::new();
        for factor in factors {
            if factor.scope.contains(&variable) {
                combined = cache.manager.and(combined, factor.root);
                boundary.extend(factor.scope.into_iter().filter(|&item| item != variable));
                provenance.extend(factor.clauses);
            } else {
                retained.push(factor);
            }
        }
        let projected = cache
            .manager
            .exists(combined, variable, &mut HashMap::new());
        retained.push(ProvenanceFactor {
            scope: boundary.into_iter().collect(),
            root: projected,
            clauses: provenance,
        });
        factors = retained;
        recovery.push((variable, combined));
    }
    let satisfiable = factors.iter().all(|factor| factor.root == 1);
    let assignment = satisfiable.then(|| {
        let mut assignment = vec![false; cache.vars];
        for &(variable, combined) in recovery.iter().rev() {
            if !cache.manager.evaluate(combined, &assignment) {
                assignment[variable] = true;
                assert!(cache.manager.evaluate(combined, &assignment));
            }
        }
        assignment
    });
    (
        assignment,
        cache.manager.nodes.len() - initial_nodes,
        earliest,
        checkpoint_layer,
    )
}

fn eliminate_with_bdds(num_vars: usize, clauses: &[Clause], order: &[usize]) -> BddSolveResult {
    let mut manager = BddManager::default();
    let mut factors: Vec<(Vec<usize>, usize)> = clauses
        .iter()
        .map(|clause| {
            let mut root = 0;
            let mut scope = Vec::new();
            for &(variable, positive) in &clause.0 {
                let literal = manager.literal(variable, positive);
                root = manager.or(root, literal);
                scope.push(variable);
            }
            scope.sort_unstable();
            scope.dedup();
            (scope, root)
        })
        .collect();
    let mut recovery = Vec::new();

    for &variable in order {
        let mut combined = 1;
        let mut boundary = BTreeSet::new();
        let mut retained = Vec::new();
        for (scope, root) in factors {
            if scope.contains(&variable) {
                combined = manager.and(combined, root);
                boundary.extend(scope.into_iter().filter(|&v| v != variable));
            } else {
                retained.push((scope, root));
            }
        }
        let projected = manager.exists(combined, variable, &mut HashMap::new());
        retained.push((boundary.iter().copied().collect(), projected));
        factors = retained;
        recovery.push((variable, combined));
    }

    let satisfiable = factors.iter().all(|(_, root)| *root == 1);
    let assignment = satisfiable.then(|| {
        let mut assignment = vec![false; num_vars];
        for &(variable, combined) in recovery.iter().rev() {
            assignment[variable] = false;
            if !manager.evaluate(combined, &assignment) {
                assignment[variable] = true;
                assert!(manager.evaluate(combined, &assignment));
            }
        }
        assignment
    });
    let mut reachable = BTreeSet::new();
    let mut stack: Vec<_> = factors.iter().map(|(_, root)| *root).collect();
    stack.extend(recovery.iter().map(|(_, root)| *root));
    while let Some(root) = stack.pop() {
        if root < 2 || !reachable.insert(root) {
            continue;
        }
        let node = manager.node(root);
        stack.push(node.low);
        stack.push(node.high);
    }

    let mut interactions: HashMap<(Literal, Literal), usize> = HashMap::new();
    for node in &manager.nodes {
        for (parent_sign, child) in [(false, node.low), (true, node.high)] {
            if child < 2 {
                continue;
            }
            let child_variable = manager.node(child).variable;
            for child_sign in [false, true] {
                let a = (node.variable, parent_sign);
                let b = (child_variable, child_sign);
                let pair = if a <= b { (a, b) } else { (b, a) };
                *interactions.entry(pair).or_default() += 1;
            }
        }
    }
    let mut interaction_candidates: Vec<_> = interactions.into_iter().collect();
    interaction_candidates.sort_by_key(|&(pair, score)| (std::cmp::Reverse(score), pair));
    BddSolveResult {
        assignment,
        allocated_nodes: manager.nodes.len(),
        live_nodes: reachable.len(),
        interaction_candidates,
    }
}

fn eliminate_with_bdds_ordered(
    num_vars: usize,
    clauses: &[Clause],
    elimination_order: &[usize],
    bdd_order: &[usize],
) -> BddSolveResult {
    assert_eq!(bdd_order.len(), num_vars);
    let mut rank = vec![usize::MAX; num_vars];
    for (level, &variable) in bdd_order.iter().enumerate() {
        assert!(variable < num_vars && rank[variable] == usize::MAX);
        rank[variable] = level;
    }
    let mapped_clauses: Vec<_> = clauses
        .iter()
        .map(|clause| {
            Clause(
                clause
                    .0
                    .iter()
                    .map(|&(variable, sign)| (rank[variable], sign))
                    .collect(),
            )
        })
        .collect();
    let mapped_elimination: Vec<_> = elimination_order.iter().map(|&v| rank[v]).collect();
    let mut result = eliminate_with_bdds(num_vars, &mapped_clauses, &mapped_elimination);
    if let Some(mapped_assignment) = result.assignment.take() {
        let mut assignment = vec![false; num_vars];
        for original in 0..num_vars {
            assignment[original] = mapped_assignment[rank[original]];
        }
        result.assignment = Some(assignment);
    }
    for ((a, b), _) in &mut result.interaction_candidates {
        a.0 = bdd_order[a.0];
        b.0 = bdd_order[b.0];
        if *a > *b {
            std::mem::swap(a, b);
        }
    }
    result
}

fn occurrence_order(vars: usize, clauses: &[Clause], descending: bool) -> Vec<usize> {
    let mut counts = vec![0usize; vars];
    for clause in clauses {
        for &(variable, _) in &clause.0 {
            counts[variable] += 1;
        }
    }
    let mut order: Vec<_> = (0..vars).collect();
    if descending {
        order.sort_by_key(|&v| (std::cmp::Reverse(counts[v]), v));
    } else {
        order.sort_by_key(|&v| (counts[v], v));
    }
    order
}

// Cheap, deterministic permutations inspired by musical structure. These are
// only ordering heuristics: they do not change the formula or inspect BDD cost.
fn metrical_order(base: &[usize], beats: usize) -> Vec<usize> {
    (0..beats)
        .flat_map(|beat| base.iter().skip(beat).step_by(beats).copied())
        .collect()
}

fn phrase_order(base: &[usize], phrase: usize) -> Vec<usize> {
    base.chunks(phrase)
        .enumerate()
        .flat_map(|(index, chunk)| {
            let mut notes = chunk.to_vec();
            if index % 2 == 1 {
                notes.reverse();
            }
            notes
        })
        .collect()
}

fn counterpoint_order(base: &[usize]) -> Vec<usize> {
    let mut result = Vec::with_capacity(base.len());
    let (mut left, mut right) = (0usize, base.len());
    while left < right {
        result.push(base[left]);
        left += 1;
        if left < right {
            right -= 1;
            result.push(base[right]);
        }
    }
    result
}

fn motif_order(vars: usize, clauses: &[Clause], base: &[usize]) -> Vec<usize> {
    let graph = primal_graph(vars, clauses);
    let mut signatures = vec![(0usize, 0usize); vars];
    for clause in clauses {
        for &(v, sign) in &clause.0 {
            if sign {
                signatures[v].0 += 1
            } else {
                signatures[v].1 += 1
            }
        }
    }
    let mut rank = vec![0usize; vars];
    for (i, &v) in base.iter().enumerate() {
        rank[v] = i;
    }
    let mut result: Vec<_> = (0..vars).collect();
    // Variables with the same local "tone colour" become a repeated motif.
    result.sort_by_key(|&v| (signatures[v], graph[v].len(), rank[v]));
    result
}

#[derive(Clone, Copy, Debug)]
struct PhraseRule {
    length: usize,
    mode: u8,
}

fn phrase_rule_name(rule: PhraseRule) -> &'static str {
    match rule.mode {
        0 => "alternate",
        1 => "all",
        2 => "boundary-forward",
        3 => "boundary-backward",
        4 => "dense-alternate",
        5 => "no-op",
        _ => unreachable!(),
    }
}

fn apply_phrase_rule(
    vars: usize,
    clauses: &[Clause],
    base: &[usize],
    rule: PhraseRule,
) -> Vec<usize> {
    let graph = primal_graph(vars, clauses);
    let mut rank = vec![0usize; vars];
    for (i, &v) in base.iter().enumerate() {
        rank[v] = i;
    }
    let mut result = Vec::with_capacity(vars);
    for (chunk_index, chunk) in base.chunks(rule.length).enumerate() {
        let start = chunk_index * rule.length;
        let end = (start + chunk.len()).min(vars);
        let backward: usize = chunk
            .iter()
            .map(|&v| graph[v].iter().filter(|&&n| rank[n] < start).count())
            .sum();
        let forward: usize = chunk
            .iter()
            .map(|&v| graph[v].iter().filter(|&&n| rank[n] >= end).count())
            .sum();
        let internal: usize = chunk
            .iter()
            .map(|&v| {
                graph[v]
                    .iter()
                    .filter(|&&n| rank[n] >= start && rank[n] < end)
                    .count()
            })
            .sum::<usize>()
            / 2;
        let reverse = match rule.mode {
            0 => chunk_index % 2 == 1,
            1 => true,
            2 => forward > backward,
            3 => backward > forward,
            4 => chunk_index % 2 == 1 && internal * 2 >= chunk.len(),
            5 => false,
            _ => unreachable!(),
        };
        if reverse {
            result.extend(chunk.iter().rev().copied());
        } else {
            result.extend(chunk.iter().copied());
        }
    }
    result
}

fn phrase_rules() -> Vec<PhraseRule> {
    let mut rules: Vec<_> = (2..=10)
        .flat_map(|length| (0..5).map(move |mode| PhraseRule { length, mode }))
        .collect();
    rules.push(PhraseRule { length: 1, mode: 5 });
    rules
}

#[derive(Clone, Copy, Debug)]
struct ScentRule {
    length: usize,
    hops: usize,
    strongest_first: bool,
}

fn diffuse_scent_variant(vars: usize, clauses: &[Clause], hops: usize, variant: u8) -> Vec<f64> {
    let graph = primal_graph(vars, clauses);
    let mut positive = vec![0usize; vars];
    let mut negative = vec![0usize; vars];
    for clause in clauses {
        for &(v, sign) in &clause.0 {
            if sign {
                positive[v] += 1;
            } else {
                negative[v] += 1;
            }
        }
    }
    // Local emission: frequent, highly connected variables with evidence for
    // both polarities emit the strongest ambiguity/tension signal.
    let mut scent: Vec<_> = (0..vars)
        .map(|v| match variant {
            0 => {
                let occurrences = positive[v] + negative[v];
                let conflict = positive[v].min(negative[v]);
                (1 + graph[v].len()) as f64 * (1 + conflict) as f64 * (1 + occurrences) as f64
            }
            1 => (1 + graph[v].len()) as f64,
            2 => (1 + positive[v].min(negative[v])) as f64,
            3 => {
                let mut x = (v as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15);
                x ^= x >> 30;
                x = x.wrapping_mul(0xBF58476D1CE4E5B9);
                (x ^ (x >> 27)) as f64
            }
            _ => unreachable!(),
        })
        .collect();
    for _ in 0..hops {
        let previous = scent.clone();
        for v in 0..vars {
            let neighbour_mean =
                graph[v].iter().map(|&n| previous[n]).sum::<f64>() / graph[v].len().max(1) as f64;
            // Attenuated diffusion retains local evidence while carrying a
            // summary of increasingly distant structure.
            scent[v] = 0.5 * previous[v] + 0.5 * neighbour_mean;
        }
    }
    scent
}

fn apply_scent_rule(
    vars: usize,
    clauses: &[Clause],
    base: &[usize],
    rule: ScentRule,
) -> Vec<usize> {
    apply_scent_rule_variant(vars, clauses, base, rule, 0)
}

fn apply_scent_rule_variant(
    vars: usize,
    clauses: &[Clause],
    base: &[usize],
    rule: ScentRule,
    variant: u8,
) -> Vec<usize> {
    if rule.hops == 0 && rule.length == 1 {
        return base.to_vec();
    }
    let scent = diffuse_scent_variant(vars, clauses, rule.hops, variant);
    base.chunks(rule.length)
        .flat_map(|chunk| {
            let mut phrase = chunk.to_vec();
            phrase.sort_by(|&a, &b| {
                let comparison = scent[a].total_cmp(&scent[b]);
                let comparison = if rule.strongest_first {
                    comparison.reverse()
                } else {
                    comparison
                };
                comparison.then_with(|| a.cmp(&b))
            });
            phrase
        })
        .collect()
}

fn scent_rules() -> Vec<ScentRule> {
    let mut rules: Vec<_> = (2..=10)
        .flat_map(|length| {
            (1..=4).flat_map(move |hops| {
                [false, true]
                    .into_iter()
                    .map(move |strongest_first| ScentRule {
                        length,
                        hops,
                        strongest_first,
                    })
            })
        })
        .collect();
    rules.push(ScentRule {
        length: 1,
        hops: 0,
        strongest_first: false,
    });
    rules
}

const SCENT_GATE_FEATURES: usize = 8;

fn scent_gate_features(vars: usize, clauses: &[Clause]) -> [f64; SCENT_GATE_FEATURES] {
    let graph = primal_graph(vars, clauses);
    let order = min_fill_order(vars, clauses);
    let mut rank = vec![0usize; vars];
    for (i, &v) in order.iter().enumerate() {
        rank[v] = i;
    }
    let degrees: Vec<_> = graph.iter().map(BTreeSet::len).collect();
    let degree_mean = degrees.iter().sum::<usize>() as f64 / vars.max(1) as f64;
    let degree_std = (degrees
        .iter()
        .map(|&d| (d as f64 - degree_mean).powi(2))
        .sum::<f64>()
        / vars.max(1) as f64)
        .sqrt();
    let mut triangles = 0usize;
    let mut wedges = 0usize;
    let mut span_sum = 0usize;
    let mut edges = 0usize;
    for v in 0..vars {
        let neighbours: Vec<_> = graph[v].iter().copied().collect();
        wedges += neighbours.len().saturating_sub(1) * neighbours.len() / 2;
        for (i, &a) in neighbours.iter().enumerate() {
            triangles += neighbours[i + 1..]
                .iter()
                .filter(|&&b| graph[a].contains(&b))
                .count();
        }
        for &n in graph[v].range((v + 1)..) {
            span_sum += rank[v].abs_diff(rank[n]);
            edges += 1;
        }
    }
    let mut positive = vec![0usize; vars];
    let mut negative = vec![0usize; vars];
    for clause in clauses {
        for &(v, sign) in &clause.0 {
            if sign {
                positive[v] += 1
            } else {
                negative[v] += 1
            }
        }
    }
    let polarity_conflict = (0..vars)
        .map(|v| positive[v].min(negative[v]))
        .sum::<usize>() as f64
        / clauses.len().max(1) as f64;
    let (width, work) = elimination_cost(vars, clauses, &order);
    [
        1.0,
        clauses.len() as f64 / vars.max(1) as f64,
        degree_mean / vars.max(1) as f64,
        degree_std / degree_mean.max(1e-9),
        triangles as f64 / wedges.max(1) as f64,
        span_sum as f64 / edges.max(1) as f64 / vars.max(1) as f64,
        width as f64 / vars.max(1) as f64,
        polarity_conflict + work.max(1.0).ln() / 100.0,
    ]
}

// Deliberately avoids graph construction, min-fill, and elimination simulation.
// Every literal is visited once; variable IDs provide a cheap locality signal.
fn cheap_structure_features(vars: usize, clauses: &[Clause]) -> [f64; SCENT_GATE_FEATURES] {
    let mut occurrences = vec![0usize; vars];
    let mut positive = vec![0usize; vars];
    let mut span_sum = 0usize;
    let mut pair_distance = 0usize;
    let mut pairs = 0usize;
    for clause in clauses {
        let mut low = vars;
        let mut high = 0usize;
        for &(variable, sign) in &clause.0 {
            occurrences[variable] += 1;
            positive[variable] += usize::from(sign);
            low = low.min(variable);
            high = high.max(variable);
        }
        span_sum += high.saturating_sub(low);
        for i in 0..clause.0.len() {
            for j in i + 1..clause.0.len() {
                pair_distance += clause.0[i].0.abs_diff(clause.0[j].0);
                pairs += 1;
            }
        }
    }
    let mean = occurrences.iter().sum::<usize>() as f64 / vars.max(1) as f64;
    let std = (occurrences
        .iter()
        .map(|&value| (value as f64 - mean).powi(2))
        .sum::<f64>()
        / vars.max(1) as f64)
        .sqrt();
    let active = occurrences.iter().filter(|&&value| value > 0).count();
    let conflict = (0..vars)
        .map(|variable| positive[variable].min(occurrences[variable] - positive[variable]))
        .sum::<usize>();
    [
        1.0,
        clauses.len() as f64 / vars.max(1) as f64,
        mean / clauses.len().max(1) as f64,
        std / mean.max(1e-9),
        active as f64 / vars.max(1) as f64,
        span_sum as f64 / clauses.len().max(1) as f64 / vars.max(1) as f64,
        pair_distance as f64 / pairs.max(1) as f64 / vars.max(1) as f64,
        conflict as f64 / occurrences.iter().sum::<usize>().max(1) as f64,
    ]
}

fn scent_gate_predict(
    training: &[([f64; SCENT_GATE_FEATURES], f64)],
    features: &[f64; SCENT_GATE_FEATURES],
    neighbours: usize,
) -> f64 {
    let mut mean = [0.0; SCENT_GATE_FEATURES];
    for (sample, _) in training {
        for i in 0..SCENT_GATE_FEATURES {
            mean[i] += sample[i];
        }
    }
    for value in &mut mean {
        *value /= training.len().max(1) as f64;
    }
    let mut scale = [0.0; SCENT_GATE_FEATURES];
    for (sample, _) in training {
        for i in 0..SCENT_GATE_FEATURES {
            scale[i] += (sample[i] - mean[i]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / training.len().max(1) as f64).sqrt().max(1e-9);
    }
    let mut distances: Vec<_> = training
        .iter()
        .map(|(sample, label)| {
            let distance = (1..SCENT_GATE_FEATURES)
                .map(|i| ((features[i] - sample[i]) / scale[i]).powi(2))
                .sum::<f64>();
            (distance, *label)
        })
        .collect();
    distances.sort_by(|a, b| a.0.total_cmp(&b.0));
    distances
        .iter()
        .take(neighbours)
        .map(|(_, y)| y)
        .sum::<f64>()
        / neighbours.min(distances.len()).max(1) as f64
}

fn support_distance(
    training: &[([f64; SCENT_GATE_FEATURES], f64)],
    features: &[f64; SCENT_GATE_FEATURES],
) -> f64 {
    let mut mean = [0.0; SCENT_GATE_FEATURES];
    for (sample, _) in training {
        for i in 1..SCENT_GATE_FEATURES {
            mean[i] += sample[i];
        }
    }
    for value in &mut mean {
        *value /= training.len().max(1) as f64;
    }
    let mut scale = [0.0; SCENT_GATE_FEATURES];
    for (sample, _) in training {
        for i in 1..SCENT_GATE_FEATURES {
            scale[i] += (sample[i] - mean[i]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / training.len().max(1) as f64).sqrt().max(1e-9);
    }
    training
        .iter()
        .map(|(sample, _)| {
            (1..SCENT_GATE_FEATURES)
                .map(|i| ((features[i] - sample[i]) / scale[i]).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

fn learn_regime_rejection(
    records: &[([f64; SCENT_GATE_FEATURES], f64, usize)],
    neighbours: usize,
) -> (f64, f64, f64, usize) {
    let mut oof = Vec::new();
    for &(features, label, regime) in records {
        let fold: Vec<_> = records
            .iter()
            .filter(|record| record.2 != regime)
            .map(|record| (record.0, record.1))
            .collect();
        oof.push((
            scent_gate_predict(&fold, &features, neighbours),
            support_distance(&fold, &features),
            label.exp(),
        ));
    }
    let mut prediction_thresholds = vec![f64::NEG_INFINITY, 0.0];
    prediction_thresholds.extend(oof.iter().map(|item| item.0));
    // Never permit an unbounded cap: a query farther away than every held-out
    // training regime is, by definition, outside observed support.
    let mut distance_thresholds = vec![0.0];
    distance_thresholds.extend(oof.iter().map(|item| item.1));
    let mut best = (f64::NEG_INFINITY, 0.0, 1.0, 0usize);
    for prediction_threshold in prediction_thresholds {
        for &distance_threshold in &distance_thresholds {
            let applied = oof
                .iter()
                .filter(|item| item.0 < prediction_threshold && item.1 <= distance_threshold)
                .count();
            let ratio = oof
                .iter()
                .map(|item| {
                    if item.0 < prediction_threshold && item.1 <= distance_threshold {
                        item.2
                    } else {
                        1.0
                    }
                })
                .sum::<f64>()
                / oof.len().max(1) as f64;
            if ratio < best.2 - 1e-12 || ((ratio - best.2).abs() <= 1e-12 && applied < best.3) {
                best = (prediction_threshold, distance_threshold, ratio, applied);
            }
        }
    }
    best
}

fn learn_scent_gate_threshold(
    training: &[([f64; SCENT_GATE_FEATURES], f64)],
    neighbours: usize,
) -> (f64, f64, usize) {
    let mut out_of_fold = Vec::with_capacity(training.len());
    for held_out in 0..training.len() {
        let fold: Vec<_> = training
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != held_out)
            .map(|(_, sample)| *sample)
            .collect();
        let prediction = scent_gate_predict(&fold, &training[held_out].0, neighbours);
        out_of_fold.push((prediction, training[held_out].1));
    }
    let mut predictions: Vec<_> = out_of_fold
        .iter()
        .map(|(prediction, _)| *prediction)
        .collect();
    predictions.sort_by(f64::total_cmp);
    predictions.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let mut thresholds = vec![f64::NEG_INFINITY];
    thresholds.extend(predictions.windows(2).map(|pair| (pair[0] + pair[1]) / 2.0));
    thresholds.push(f64::INFINITY);
    let mut best = (f64::NEG_INFINITY, 1.0, 0usize);
    for threshold in thresholds {
        let applied = out_of_fold
            .iter()
            .filter(|(prediction, _)| *prediction < threshold)
            .count();
        let mean_ratio = out_of_fold
            .iter()
            .map(|(prediction, actual_log_ratio)| {
                if *prediction < threshold {
                    actual_log_ratio.exp()
                } else {
                    1.0
                }
            })
            .sum::<f64>()
            / out_of_fold.len().max(1) as f64;
        if mean_ratio < best.1 - 1e-12 || ((mean_ratio - best.1).abs() <= 1e-12 && applied < best.2)
        {
            best = (threshold, mean_ratio, applied);
        }
    }
    best
}

fn helper_gate_predict(
    training: &[([f64; HELPER_GATE_FEATURES], f64)],
    features: &[f64; HELPER_GATE_FEATURES],
    neighbours: usize,
) -> f64 {
    let mut mean = [0.0; HELPER_GATE_FEATURES];
    for (sample, _) in training {
        for i in 0..HELPER_GATE_FEATURES {
            mean[i] += sample[i];
        }
    }
    for value in &mut mean {
        *value /= training.len().max(1) as f64;
    }
    let mut scale = [0.0; HELPER_GATE_FEATURES];
    for (sample, _) in training {
        for i in 0..HELPER_GATE_FEATURES {
            scale[i] += (sample[i] - mean[i]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / training.len().max(1) as f64).sqrt().max(1e-9);
    }
    let mut distances: Vec<_> = training
        .iter()
        .map(|(sample, label)| {
            let distance = (1..HELPER_GATE_FEATURES)
                .map(|i| ((features[i] - sample[i]) / scale[i]).powi(2))
                .sum::<f64>();
            (distance, *label)
        })
        .collect();
    distances.sort_by(|a, b| a.0.total_cmp(&b.0));
    distances
        .iter()
        .take(neighbours)
        .map(|(_, y)| y)
        .sum::<f64>()
        / neighbours.min(distances.len()).max(1) as f64
}

fn learn_helper_gate_threshold(
    training: &[([f64; HELPER_GATE_FEATURES], f64)],
    neighbours: usize,
) -> (f64, f64, usize) {
    let mut out_of_fold = Vec::with_capacity(training.len());
    for held_out in 0..training.len() {
        let fold: Vec<_> = training
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != held_out)
            .map(|(_, sample)| *sample)
            .collect();
        out_of_fold.push((
            helper_gate_predict(&fold, &training[held_out].0, neighbours),
            training[held_out].1,
        ));
    }
    let mut predictions: Vec<_> = out_of_fold.iter().map(|sample| sample.0).collect();
    predictions.sort_by(f64::total_cmp);
    predictions.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let mut thresholds = vec![f64::NEG_INFINITY];
    thresholds.extend(predictions.windows(2).map(|pair| (pair[0] + pair[1]) / 2.0));
    thresholds.push(f64::INFINITY);
    let mut best = (f64::NEG_INFINITY, 1.0, 0usize);
    for threshold in thresholds {
        let applied = out_of_fold
            .iter()
            .filter(|sample| sample.0 < threshold)
            .count();
        let ratio = out_of_fold
            .iter()
            .map(|sample| {
                if sample.0 < threshold {
                    sample.1.exp()
                } else {
                    1.0
                }
            })
            .sum::<f64>()
            / out_of_fold.len().max(1) as f64;
        if ratio < best.1 - 1e-12 || ((ratio - best.1).abs() <= 1e-12 && applied < best.2) {
            best = (threshold, ratio, applied);
        }
    }
    best
}

fn harmonic_order(vars: usize, clauses: &[Clause], base: &[usize]) -> Vec<usize> {
    let graph = primal_graph(vars, clauses);
    let rank: Vec<_> = {
        let mut rank = vec![0; vars];
        for (i, &v) in base.iter().enumerate() {
            rank[v] = i;
        }
        rank
    };
    let mut unused = vec![true; vars];
    let mut result = Vec::with_capacity(vars);
    let mut current = base[0];
    while result.len() < vars {
        result.push(current);
        unused[current] = false;
        current = (0..vars)
            .filter(|&v| unused[v])
            .min_by_key(|&v| {
                // Prefer a consonant (adjacent) variable with smooth movement
                // in the min-fill coordinate system.
                (
                    !graph[current].contains(&v),
                    rank[current].abs_diff(rank[v]),
                    rank[v],
                )
            })
            .unwrap_or(current);
    }
    result
}

fn tension_order(vars: usize, clauses: &[Clause], resolving: bool) -> Vec<usize> {
    let graph = primal_graph(vars, clauses);
    let mut positive = vec![0usize; vars];
    let mut negative = vec![0usize; vars];
    for clause in clauses {
        for &(v, sign) in &clause.0 {
            if sign {
                positive[v] += 1
            } else {
                negative[v] += 1
            }
        }
    }
    let mut order: Vec<_> = (0..vars).collect();
    order.sort_by_key(|&v| {
        let tension = graph[v].len() * (1 + positive[v].min(negative[v]));
        if resolving {
            tension
        } else {
            usize::MAX - tension
        }
    });
    order
}

#[derive(Debug)]
struct SiftResult {
    order: Vec<usize>,
    result: BddSolveResult,
    swaps_tested: usize,
    swaps_accepted: usize,
    passes: usize,
}

fn sift_bdd_order(
    vars: usize,
    clauses: &[Clause],
    elimination_order: &[usize],
    initial_order: &[usize],
    max_passes: usize,
    trials_per_pass: usize,
    guided: bool,
    seed: u64,
) -> SiftResult {
    let mut order = initial_order.to_vec();
    let mut result = eliminate_with_bdds_ordered(vars, clauses, elimination_order, &order);
    let mut swaps_tested = 0;
    let mut swaps_accepted = 0;
    let mut passes = 0;

    for pass in 0..max_passes {
        let mut improved = false;
        let mut positions: Vec<usize> = if pass % 2 == 0 {
            (0..vars.saturating_sub(1)).collect()
        } else {
            (0..vars.saturating_sub(1)).rev().collect()
        };
        if trials_per_pass < positions.len() {
            if guided {
                let mut interaction_strength: HashMap<(usize, usize), usize> = HashMap::new();
                for &((a, b), score) in &result.interaction_candidates {
                    let pair = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
                    *interaction_strength.entry(pair).or_default() += score;
                }
                positions.sort_by_key(|&position| {
                    let a = order[position];
                    let b = order[position + 1];
                    let pair = if a <= b { (a, b) } else { (b, a) };
                    std::cmp::Reverse(interaction_strength.get(&pair).copied().unwrap_or(0))
                });
            } else {
                Rng(seed ^ (pass as u64 + 1).wrapping_mul(0x9e37_79b9)).shuffle(&mut positions);
            }
            positions.truncate(trials_per_pass);
        }
        for position in positions {
            order.swap(position, position + 1);
            swaps_tested += 1;
            let candidate = eliminate_with_bdds_ordered(vars, clauses, elimination_order, &order);
            if candidate.allocated_nodes < result.allocated_nodes {
                result = candidate;
                swaps_accepted += 1;
                improved = true;
            } else {
                order.swap(position, position + 1);
            }
        }
        passes += 1;
        if !improved {
            break;
        }
    }
    SiftResult {
        order,
        result,
        swaps_tested,
        swaps_accepted,
        passes,
    }
}

/// Count non-terminal nodes in the reduced ordered BDD for a truth table.
/// Table bit `i` corresponds to BDD level `i`.
fn reduced_bdd_nodes(values: &[bool], variables: usize) -> usize {
    fn build(
        values: &[bool],
        level: usize,
        variables: usize,
        offset: usize,
        stride: usize,
        unique: &mut HashMap<(usize, usize, usize), usize>,
    ) -> usize {
        if level == variables {
            return usize::from(values[offset]);
        }
        let low = build(values, level + 1, variables, offset, stride * 2, unique);
        let high = build(
            values,
            level + 1,
            variables,
            offset + stride,
            stride * 2,
            unique,
        );
        if low == high {
            return low;
        }
        let next_id = unique.len() + 2;
        *unique.entry((level, low, high)).or_insert(next_id)
    }

    assert_eq!(values.len(), 1usize << variables);
    let mut unique = HashMap::new();
    build(values, 0, variables, 0, 1, &mut unique);
    unique.len()
}

fn eliminate(num_vars: usize, clauses: &[Clause], order: &[usize]) -> SolveResult {
    let mut factors: Vec<Factor> = clauses.iter().map(Factor::from_clause).collect();
    let mut layers = Vec::new();
    let mut peak_boundary = 0;
    let mut peak_entries = 1;
    let mut peak_bdd_nodes = 0;

    for &variable in order {
        let mut selected = Vec::new();
        let mut retained = Vec::new();
        for factor in factors {
            if factor.scope.contains(&variable) {
                selected.push(factor);
            } else {
                retained.push(factor);
            }
        }

        let mut set = BTreeSet::new();
        for factor in &selected {
            set.extend(factor.scope.iter().copied().filter(|&v| v != variable));
        }
        let boundary: Vec<_> = set.into_iter().collect();
        let entries = 1usize << boundary.len();
        peak_boundary = peak_boundary.max(boundary.len());
        peak_entries = peak_entries.max(entries);
        let mut witness = vec![None; entries];
        let mut projected = vec![false; entries];

        let mut combined_vars = boundary.clone();
        combined_vars.push(variable);
        combined_vars.sort_unstable();
        let variable_pos = combined_vars.binary_search(&variable).unwrap();

        for boundary_bits in 0..entries {
            for value in [false, true] {
                let mut combined_bits = 0;
                for (i, &v) in boundary.iter().enumerate() {
                    let pos = combined_vars.binary_search(&v).unwrap();
                    combined_bits |= ((boundary_bits >> i) & 1) << pos;
                }
                combined_bits |= (value as usize) << variable_pos;
                if selected
                    .iter()
                    .all(|f| f.evaluate_from(&combined_vars, combined_bits))
                {
                    projected[boundary_bits] = true;
                    witness[boundary_bits] = Some(value);
                    break;
                }
            }
        }

        let bdd_nodes = reduced_bdd_nodes(&projected, boundary.len());
        peak_bdd_nodes = peak_bdd_nodes.max(bdd_nodes);
        retained.push(Factor {
            scope: boundary.clone(),
            values: projected,
        });
        factors = retained;
        layers.push(Layer {
            variable,
            boundary,
            witness,
            bdd_nodes,
        });
    }

    let satisfiable = factors.iter().all(|f| f.values[0]);
    let assignment = satisfiable.then(|| {
        let mut assignment = vec![false; num_vars];
        for layer in layers.iter().rev() {
            let bits = layer
                .boundary
                .iter()
                .enumerate()
                .fold(0, |acc, (i, &v)| acc | ((assignment[v] as usize) << i));
            assignment[layer.variable] =
                layer.witness[bits].expect("reachable boundary assignment must have a witness");
        }
        assignment
    });
    SolveResult {
        assignment,
        peak_boundary,
        peak_entries,
        peak_bdd_nodes,
        layers,
    }
}

fn satisfies(clauses: &[Clause], assignment: &[bool]) -> bool {
    clauses
        .iter()
        .all(|c| c.0.iter().any(|&(v, sign)| assignment[v] == sign))
}

fn add_to_varisat(solver: &mut Solver<'_>, clauses: &[Clause]) {
    for clause in clauses {
        let literals: Vec<_> = clause
            .0
            .iter()
            .map(|&(variable, positive)| Lit::from_var(Var::from_index(variable), positive))
            .collect();
        solver.add_clause(&literals);
    }
}

fn solve_with_varisat(vars: usize, clauses: &[Clause]) -> Option<Vec<bool>> {
    let mut solver = Solver::new();
    add_to_varisat(&mut solver, clauses);
    if !solver.solve().expect("Varisat solve") {
        return None;
    }
    let mut assignment = vec![false; vars];
    for literal in solver.model().expect("Varisat model") {
        if literal.var().index() < vars {
            assignment[literal.var().index()] = literal.is_positive();
        }
    }
    Some(assignment)
}

fn brute_force(num_vars: usize, clauses: &[Clause]) -> Option<Vec<bool>> {
    (0..(1usize << num_vars)).find_map(|bits| {
        let assignment: Vec<_> = (0..num_vars).map(|v| ((bits >> v) & 1) == 1).collect();
        satisfies(clauses, &assignment).then_some(assignment)
    })
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.below(i + 1));
        }
    }
}

fn random_3sat(vars: usize, clauses: usize, seed: u64) -> Vec<Clause> {
    let mut rng = Rng(seed.max(1));
    (0..clauses)
        .map(|_| {
            let mut chosen = BTreeSet::new();
            while chosen.len() < 3.min(vars) {
                chosen.insert(rng.below(vars));
            }
            Clause(chosen.into_iter().map(|v| (v, rng.below(2) == 1)).collect())
        })
        .collect()
}

fn force_planted_satisfaction(
    mut clauses: Vec<Clause>,
    assignment: &[bool],
    seed: u64,
) -> Vec<Clause> {
    let mut rng = Rng(seed ^ 0xa076_1d64_78bd_642f);
    for clause in &mut clauses {
        if !clause
            .0
            .iter()
            .any(|&(variable, sign)| assignment[variable] == sign)
        {
            let index = rng.below(clause.0.len());
            let variable = clause.0[index].0;
            clause.0[index].1 = assignment[variable];
        }
    }
    clauses
}

fn planted_random_3sat(vars: usize, clauses: usize, seed: u64) -> Vec<Clause> {
    let assignment = planted_assignment(vars, seed);
    force_planted_satisfaction(random_3sat(vars, clauses, seed), &assignment, seed)
}

fn planted_banded_3sat(vars: usize, clauses: usize, seed: u64, width: usize) -> Vec<Clause> {
    let assignment = planted_assignment(vars, seed);
    force_planted_satisfaction(banded_3sat(vars, clauses, seed, width), &assignment, seed)
}

fn banded_3sat(vars: usize, clauses: usize, seed: u64, width: usize) -> Vec<Clause> {
    let mut rng = Rng(seed.max(1));
    (0..clauses)
        .map(|_| {
            let start = rng.below(vars);
            let end = (start + width.max(3)).min(vars);
            let start = end.saturating_sub(width.max(3));
            let mut chosen = BTreeSet::new();
            while chosen.len() < 3.min(end - start) {
                chosen.insert(start + rng.below(end - start));
            }
            Clause(chosen.into_iter().map(|v| (v, rng.below(2) == 1)).collect())
        })
        .collect()
}

fn identity_expanded_sat(vars: usize, ratio: usize, seed: u64) -> Vec<Clause> {
    let helpers = (vars / 4).max(1).min(vars.saturating_sub(3));
    let base_vars = vars - helpers;
    let base = random_3sat(base_vars, base_vars * ratio, seed);
    let mut expanded = Vec::with_capacity(base.len() + helpers);
    for (index, clause) in base.into_iter().enumerate() {
        if index < helpers {
            let variable = base_vars + index;
            let mut positive = clause.0.clone();
            positive.push((variable, true));
            let mut negative = clause.0;
            negative.push((variable, false));
            expanded.push(Clause(positive));
            expanded.push(Clause(negative));
        } else {
            expanded.push(clause);
        }
    }
    expanded
}

#[derive(Default)]
struct MathTrickStats {
    tautologies: usize,
    subsumed: usize,
    consensus_pairs: usize,
    passes: usize,
}

fn normalize_clause(mut literals: Vec<Literal>) -> Option<Clause> {
    literals.sort_unstable();
    literals.dedup();
    if literals
        .iter()
        .any(|&(variable, sign)| literals.contains(&(variable, !sign)))
    {
        None
    } else {
        Some(Clause(literals))
    }
}

fn mathematical_identity_preprocess(clauses: &[Clause]) -> (Vec<Clause>, MathTrickStats) {
    let mut stats = MathTrickStats::default();
    let mut current = Vec::new();
    for clause in clauses {
        if let Some(normalized) = normalize_clause(clause.0.clone()) {
            current.push(normalized);
        } else {
            stats.tautologies += 1;
        }
    }
    current.sort_by(|a, b| a.0.cmp(&b.0));
    current.dedup_by(|a, b| a.0 == b.0);
    loop {
        stats.passes += 1;
        let mut changed = false;
        let mut remove = BTreeSet::new();
        for i in 0..current.len() {
            let a: BTreeSet<_> = current[i].0.iter().copied().collect();
            for (j, clause) in current.iter().enumerate() {
                if i == j || remove.contains(&j) {
                    continue;
                }
                let b: BTreeSet<_> = clause.0.iter().copied().collect();
                if a.is_subset(&b) && (a.len() < b.len() || i < j) {
                    remove.insert(j);
                }
            }
        }
        if !remove.is_empty() {
            stats.subsumed += remove.len();
            current = current
                .into_iter()
                .enumerate()
                .filter(|(index, _)| !remove.contains(index))
                .map(|(_, clause)| clause)
                .collect();
            changed = true;
        }
        let mut replacement = None;
        'pairs: for i in 0..current.len() {
            for j in i + 1..current.len() {
                for &(variable, sign) in &current[i].0 {
                    if !current[j].0.contains(&(variable, !sign)) {
                        continue;
                    }
                    let mut left = current[i].0.clone();
                    let mut right = current[j].0.clone();
                    left.retain(|literal| literal.0 != variable);
                    right.retain(|literal| literal.0 != variable);
                    left.sort_unstable();
                    right.sort_unstable();
                    if left == right {
                        replacement = Some((i, j, Clause(left)));
                        break 'pairs;
                    }
                }
            }
        }
        if let Some((i, j, replacement_clause)) = replacement {
            current = current
                .into_iter()
                .enumerate()
                .filter(|(index, _)| *index != i && *index != j)
                .map(|(_, clause)| clause)
                .collect();
            current.push(replacement_clause);
            stats.consensus_pairs += 1;
            changed = true;
        }
        current.sort_by(|a, b| a.0.cmp(&b.0));
        current.dedup_by(|a, b| a.0 == b.0);
        if !changed {
            break;
        }
    }
    (current, stats)
}

fn flower_coordinates(vars: usize) -> Vec<(i32, i32)> {
    let mut radius = 0i32;
    while (1 + 3 * radius * (radius + 1)) < vars as i32 {
        radius += 1;
    }
    let mut coordinates = Vec::new();
    for q in -radius..=radius {
        for r in -radius..=radius {
            let distance = q.abs().max(r.abs()).max((q + r).abs());
            if distance <= radius {
                coordinates.push((q, r));
            }
        }
    }
    coordinates.sort_by_key(|&(q, r)| {
        let distance = q.abs().max(r.abs()).max((q + r).abs());
        (distance, q, r)
    });
    coordinates.truncate(vars);
    coordinates
}

fn hex_distance((aq, ar): (i32, i32), (bq, br): (i32, i32)) -> i32 {
    let q = aq - bq;
    let r = ar - br;
    q.abs().max(r.abs()).max((q + r).abs())
}

fn local_geometry_3sat(
    coordinates: &[(i32, i32, i32)],
    clauses: usize,
    seed: u64,
    planted: Option<&[bool]>,
) -> Vec<Clause> {
    let mut rng = Rng(seed.max(1));
    (0..clauses)
        .map(|_| {
            let center = rng.below(coordinates.len());
            let (cq, cr, cz) = coordinates[center];
            let mut pool: Vec<_> = coordinates
                .iter()
                .enumerate()
                .filter_map(|(index, &(q, r, z))| {
                    let planar = hex_distance((cq, cr), (q, r));
                    ((z == cz && planar <= 1) || (q == cq && r == cr && (z - cz).abs() == 1))
                        .then_some(index)
                })
                .collect();
            if pool.len() < 3 {
                pool = (0..coordinates.len()).collect();
            }
            rng.shuffle(&mut pool);
            let mut clause = Clause(
                pool.into_iter()
                    .take(3.min(coordinates.len()))
                    .map(|variable| (variable, rng.below(2) == 1))
                    .collect(),
            );
            if let Some(assignment) = planted {
                if !clause
                    .0
                    .iter()
                    .any(|&(variable, sign)| assignment[variable] == sign)
                {
                    clause.0[0].1 = assignment[clause.0[0].0];
                }
            }
            clause
        })
        .collect()
}

fn flower_3sat(vars: usize, clauses: usize, seed: u64) -> Vec<Clause> {
    let coordinates: Vec<_> = flower_coordinates(vars)
        .into_iter()
        .map(|(q, r)| (q, r, 0))
        .collect();
    local_geometry_3sat(&coordinates, clauses, seed, None)
}

fn planted_assignment(vars: usize, seed: u64) -> Vec<bool> {
    let mut rng = Rng(seed.max(1) ^ 0xd1b5_4a32_d192_ed03);
    (0..vars).map(|_| rng.below(2) == 1).collect()
}

fn planted_flower_3sat(vars: usize, clauses: usize, seed: u64) -> Vec<Clause> {
    let coordinates: Vec<_> = flower_coordinates(vars)
        .into_iter()
        .map(|(q, r)| (q, r, 0))
        .collect();
    let assignment = planted_assignment(vars, seed);
    local_geometry_3sat(&coordinates, clauses, seed, Some(&assignment))
}

fn symmetric_flower_3sat(vars: usize, target_clauses: usize) -> Vec<Clause> {
    let coordinates = flower_coordinates(vars);
    let index: HashMap<_, _> = coordinates
        .iter()
        .copied()
        .enumerate()
        .map(|(variable, coordinate)| (coordinate, variable))
        .collect();
    let directions = [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];
    let mut unique: BTreeSet<Vec<Literal>> = BTreeSet::new();

    for rule in 0..4 {
        for (center, &(q, r)) in coordinates.iter().enumerate() {
            for direction in 0..6 {
                let (aq, ar) = directions[direction];
                let second_direction = if rule < 2 {
                    (direction + 1) % 6
                } else {
                    (direction + 3) % 6
                };
                let (bq, br) = directions[second_direction];
                let (Some(&a), Some(&b)) =
                    (index.get(&(q + aq, r + ar)), index.get(&(q + bq, r + br)))
                else {
                    continue;
                };
                let mut clause = match rule {
                    0 => vec![(center, true), (a, false), (b, true)],
                    1 => vec![(center, false), (a, true), (b, true)],
                    2 => vec![(center, true), (a, true), (b, false)],
                    _ => vec![(center, false), (a, true), (b, true)],
                };
                clause.sort_unstable();
                unique.insert(clause);
            }
        }
        if unique.len() >= target_clauses {
            break;
        }
    }
    unique.into_iter().map(Clause).collect()
}

fn stacked_flower_3sat(vars: usize, clauses: usize, seed: u64) -> Vec<Clause> {
    let layers = 3.min(vars);
    let per_layer = vars.div_ceil(layers);
    let base = flower_coordinates(per_layer);
    let mut coordinates = Vec::new();
    for z in 0..layers {
        for &(q, r) in &base {
            if coordinates.len() == vars {
                break;
            }
            coordinates.push((q, r, z as i32));
        }
    }
    local_geometry_3sat(&coordinates, clauses, seed, None)
}

fn planted_stacked_flower_3sat(vars: usize, clauses: usize, seed: u64) -> Vec<Clause> {
    let layers = 3.min(vars);
    let per_layer = vars.div_ceil(layers);
    let base = flower_coordinates(per_layer);
    let mut coordinates = Vec::new();
    for z in 0..layers {
        for &(q, r) in &base {
            if coordinates.len() == vars {
                break;
            }
            coordinates.push((q, r, z as i32));
        }
    }
    let assignment = planted_assignment(vars, seed);
    local_geometry_3sat(&coordinates, clauses, seed, Some(&assignment))
}

fn flower_outside_in_order(vars: usize) -> Vec<usize> {
    let coordinates = flower_coordinates(vars);
    let mut order: Vec<_> = (0..vars).collect();
    order.sort_by_key(|&v| {
        let (q, r) = coordinates[v];
        let distance = q.abs().max(r.abs()).max((q + r).abs());
        (std::cmp::Reverse(distance), q, r)
    });
    order
}

fn primal_graph(vars: usize, clauses: &[Clause]) -> Vec<BTreeSet<usize>> {
    let mut graph = vec![BTreeSet::new(); vars];
    for clause in clauses {
        for &(a, _) in &clause.0 {
            for &(b, _) in &clause.0 {
                if a != b {
                    graph[a].insert(b);
                }
            }
        }
    }
    graph
}

fn exact_treewidth(vars: usize, clauses: &[Clause]) -> usize {
    assert!(
        vars <= 20,
        "exact treewidth oracle is limited to 20 vertices"
    );
    let graph = primal_graph(vars, clauses);
    let states = 1usize << vars;
    let mut dp = vec![usize::MAX; states];
    dp[0] = 0;
    for eliminated in 0..states {
        if dp[eliminated] == usize::MAX {
            continue;
        }
        for variable in 0..vars {
            if eliminated & (1usize << variable) != 0 {
                continue;
            }
            let mut seen = vec![false; vars];
            let mut stack = vec![variable];
            seen[variable] = true;
            let mut boundary = BTreeSet::new();
            while let Some(current) = stack.pop() {
                for &next in &graph[current] {
                    if next == variable || seen[next] {
                        continue;
                    }
                    seen[next] = true;
                    if eliminated & (1usize << next) != 0 {
                        stack.push(next);
                    } else {
                        boundary.insert(next);
                    }
                }
            }
            let next = eliminated | (1usize << variable);
            let width = dp[eliminated].max(boundary.len());
            dp[next] = dp[next].min(width);
        }
    }
    dp[states - 1]
}

fn exact_weighted_treewidth(weights: &[usize], graph: &[BTreeSet<usize>]) -> usize {
    let vars = weights.len();
    assert!(
        vars <= 20,
        "exact weighted treewidth is limited to 20 vertices"
    );
    if vars == 0 {
        return 0;
    }
    let states = 1usize << vars;
    let mut dp = vec![usize::MAX; states];
    dp[0] = 0;
    for eliminated in 0..states {
        if dp[eliminated] == usize::MAX {
            continue;
        }
        for variable in 0..vars {
            if eliminated & (1usize << variable) != 0 {
                continue;
            }
            let mut seen = vec![false; vars];
            let mut stack = vec![variable];
            seen[variable] = true;
            let mut boundary = BTreeSet::new();
            while let Some(current) = stack.pop() {
                for &next in &graph[current] {
                    if next == variable || seen[next] {
                        continue;
                    }
                    seen[next] = true;
                    if eliminated & (1usize << next) != 0 {
                        stack.push(next);
                    } else {
                        boundary.insert(next);
                    }
                }
            }
            let bag_bits =
                weights[variable] + boundary.iter().map(|&next| weights[next]).sum::<usize>();
            let width = dp[eliminated].max(bag_bits.saturating_sub(1));
            let next = eliminated | (1usize << variable);
            dp[next] = dp[next].min(width);
        }
    }
    dp[states - 1]
}

fn quotient_graph(vars: usize, clauses: &[Clause], groups: &[Vec<usize>]) -> Vec<BTreeSet<usize>> {
    let mut owner = vec![usize::MAX; vars];
    for (group, members) in groups.iter().enumerate() {
        for &variable in members {
            owner[variable] = group;
        }
    }
    let original = primal_graph(vars, clauses);
    let mut quotient = vec![BTreeSet::new(); groups.len()];
    for variable in 0..vars {
        for &next in &original[variable] {
            let left = owner[variable];
            let right = owner[next];
            if left != right {
                quotient[left].insert(right);
                quotient[right].insert(left);
            }
        }
    }
    quotient
}

struct ShakenFormula {
    vars: usize,
    clauses: Vec<Clause>,
    core_to_original: Vec<usize>,
    fixed: Vec<Option<bool>>,
    removed: usize,
    probes: usize,
    inverse_forced: usize,
    contradiction: bool,
}

struct SeededBranch {
    vars: usize,
    clauses: Vec<Clause>,
    core_to_original: Vec<usize>,
    boundary: Vec<usize>,
    interior: Vec<usize>,
    witnesses: Vec<Option<Vec<bool>>>,
    local_clauses: usize,
    summary_clauses: usize,
    compilation_trials: usize,
}

fn detachable_branch_candidate(
    vars: usize,
    clauses: &[Clause],
    max_interior: usize,
) -> Option<(Vec<usize>, Vec<usize>)> {
    let graph = primal_graph(vars, clauses);
    let mut best: Option<(Vec<usize>, Vec<usize>)> = None;
    let mut boundary_masks = vec![0usize];
    for first in 0..vars {
        boundary_masks.push(1usize << first);
        for second in first + 1..vars {
            boundary_masks.push((1usize << first) | (1usize << second));
        }
    }
    for boundary_mask in boundary_masks {
        let mut seen = vec![false; vars];
        for start in 0..vars {
            if boundary_mask & (1usize << start) != 0 || seen[start] {
                continue;
            }
            let mut stack = vec![start];
            seen[start] = true;
            let mut component = Vec::new();
            while let Some(variable) = stack.pop() {
                component.push(variable);
                for &next in &graph[variable] {
                    if boundary_mask & (1usize << next) == 0 && !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            if component.is_empty() || component.len() > max_interior {
                continue;
            }
            let component_set: BTreeSet<_> = component.iter().copied().collect();
            let actual_boundary: BTreeSet<_> = component
                .iter()
                .flat_map(|&variable| graph[variable].iter().copied())
                .filter(|variable| !component_set.contains(variable))
                .collect();
            if actual_boundary.is_empty() || actual_boundary.len() > 2 {
                continue;
            }
            let candidate = (component, actual_boundary.into_iter().collect::<Vec<_>>());
            if best.as_ref().is_none_or(|current| {
                candidate.0.len() > current.0.len()
                    || (candidate.0.len() == current.0.len() && candidate.1.len() < current.1.len())
            }) {
                best = Some(candidate);
            }
        }
    }
    best
}

fn fast_detachable_branch_candidates(
    vars: usize,
    clauses: &[Clause],
    max_interior: usize,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let graph = compact_primal_graph(vars, clauses);
    global_small_separator_candidates(&graph, max_interior)
}

fn compact_primal_graph(vars: usize, clauses: &[Clause]) -> Vec<Vec<usize>> {
    let mut graph = vec![Vec::new(); vars];
    for clause in clauses {
        for left in 0..clause.0.len() {
            let a = clause.0[left].0;
            for right in left + 1..clause.0.len() {
                let b = clause.0[right].0;
                if a != b {
                    graph[a].push(b);
                    graph[b].push(a);
                }
            }
        }
    }
    for neighbours in &mut graph {
        neighbours.sort_unstable();
        neighbours.dedup();
    }
    graph
}

/// Finds capped DFS subtrees whose exact external neighbourhood contains one or
/// two variables. One global traversal replaces pair enumeration and repeated
/// bounded expansion from every variable.
fn global_small_separator_candidates(
    graph: &[Vec<usize>],
    max_interior: usize,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let vars = graph.len();
    let unseen = usize::MAX;
    let mut discovery = vec![unseen; vars];
    let mut parent = vec![unseen; vars];
    let mut subtree_size = vec![0usize; vars];
    let mut preorder = Vec::with_capacity(vars);
    let mut candidates = Vec::new();

    for root in 0..vars {
        if discovery[root] != unseen {
            continue;
        }
        discovery[root] = preorder.len();
        subtree_size[root] = 1;
        preorder.push(root);
        let mut stack = vec![(root, 0usize)];
        while let Some((variable, next_edge)) = stack.last_mut() {
            if *next_edge < graph[*variable].len() {
                let next = graph[*variable][*next_edge];
                *next_edge += 1;
                if discovery[next] == unseen {
                    parent[next] = *variable;
                    discovery[next] = preorder.len();
                    subtree_size[next] = 1;
                    preorder.push(next);
                    stack.push((next, 0));
                }
                continue;
            }

            let (finished, _) = stack.pop().expect("non-empty DFS stack");
            let p = parent[finished];
            if p != unseen {
                let size = subtree_size[finished];
                subtree_size[p] += size;
            }
        }
    }

    // Every DFS subtree is a contiguous preorder interval. Scan only subtrees
    // within the cap and stop as soon as a third external neighbour appears.
    // This finds both articulation branches and exact two-vertex separators
    // without enumerating O(n^2) vertex pairs.
    for root in 0..vars {
        let size = subtree_size[root];
        if parent[root] == unseen || size == 0 || size > max_interior {
            continue;
        }
        let start = discovery[root];
        let end = start + size;
        let mut boundary = Vec::new();
        'scan: for &variable in &preorder[start..end] {
            for &next in &graph[variable] {
                let position = discovery[next];
                if (position < start || position >= end) && !boundary.contains(&next) {
                    boundary.push(next);
                    if boundary.len() > 2 {
                        break 'scan;
                    }
                }
            }
        }
        if !boundary.is_empty() && boundary.len() <= 2 {
            boundary.sort_unstable();
            candidates.push((preorder[start..end].to_vec(), boundary));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .len()
            .cmp(&left.0.len())
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut used_interior = vec![false; vars];
    let mut protected_boundary = vec![false; vars];
    candidates
        .into_iter()
        .filter(|(interior, boundary)| {
            if interior
                .iter()
                .any(|&v| used_interior[v] || protected_boundary[v])
                || boundary.iter().any(|&v| used_interior[v])
            {
                return false;
            }
            for &v in interior {
                used_interior[v] = true;
            }
            for &v in boundary {
                protected_boundary[v] = true;
            }
            true
        })
        .collect()
}

#[allow(dead_code)] // Retained as the frozen pre-rewrite discovery baseline.
fn fast_detachable_branch_candidates_in(
    graph: &[BTreeSet<usize>],
    max_interior: usize,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let vars = graph.len();
    let mut candidates = Vec::new();
    let mut consider = |interior: Vec<usize>| {
        if interior.is_empty() {
            return;
        }
        let interior_set: BTreeSet<_> = interior.iter().copied().collect();
        let boundary: BTreeSet<_> = interior
            .iter()
            .flat_map(|&variable| graph[variable].iter().copied())
            .filter(|variable| !interior_set.contains(variable))
            .collect();
        if !boundary.is_empty() && boundary.len() <= 2 {
            candidates.push((interior, boundary.into_iter().collect::<Vec<_>>()));
        }
    };
    for start in 0..vars {
        for length in 1..=max_interior.min(vars - start) {
            consider((start..start + length).collect());
        }
        let mut seen = vec![false; vars];
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        let mut prefix = Vec::new();
        while let Some(variable) = queue.pop_front() {
            prefix.push(variable);
            consider(prefix.clone());
            if prefix.len() == max_interior {
                break;
            }
            for &next in &graph[variable] {
                if !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }
    }
    candidates.sort_by(|left, right| {
        right
            .0
            .len()
            .cmp(&left.0.len())
            .then_with(|| left.1.len().cmp(&right.1.len()))
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates.dedup();
    let mut used_interior = BTreeSet::new();
    let mut protected_boundary = BTreeSet::new();
    let mut selected = Vec::new();
    for candidate in candidates {
        if candidate.0.iter().any(|variable| {
            used_interior.contains(variable) || protected_boundary.contains(variable)
        }) || candidate
            .1
            .iter()
            .any(|variable| used_interior.contains(variable))
        {
            continue;
        }
        used_interior.extend(candidate.0.iter().copied());
        protected_boundary.extend(candidate.1.iter().copied());
        selected.push(candidate);
    }
    selected
}

fn seed_detachable_branch(vars: usize, clauses: &[Clause], max_interior: usize) -> SeededBranch {
    let Some((mut interior, boundary)) = detachable_branch_candidate(vars, clauses, max_interior)
    else {
        return SeededBranch {
            vars,
            clauses: clauses.to_vec(),
            core_to_original: (0..vars).collect(),
            boundary: Vec::new(),
            interior: Vec::new(),
            witnesses: Vec::new(),
            local_clauses: 0,
            summary_clauses: 0,
            compilation_trials: 0,
        };
    };
    interior.sort_unstable();
    let interior_set: BTreeSet<_> = interior.iter().copied().collect();
    let local: Vec<_> = clauses
        .iter()
        .filter(|clause| clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    let mut witnesses = Vec::new();
    let mut summary = Vec::new();
    let mut compilation_trials = 0;
    for boundary_bits in 0..(1usize << boundary.len()) {
        let mut witness = None;
        for interior_bits in 0..(1usize << interior.len()) {
            compilation_trials += 1;
            let mut assignment = vec![false; vars];
            for (index, &variable) in boundary.iter().enumerate() {
                assignment[variable] = boundary_bits & (1usize << index) != 0;
            }
            let values: Vec<_> = interior
                .iter()
                .enumerate()
                .map(|(index, &variable)| {
                    let value = interior_bits & (1usize << index) != 0;
                    assignment[variable] = value;
                    value
                })
                .collect();
            if satisfies(&local, &assignment) {
                witness = Some(values);
                break;
            }
        }
        if witness.is_none() {
            summary.push(Clause(
                boundary
                    .iter()
                    .enumerate()
                    .map(|(index, &variable)| {
                        let value = boundary_bits & (1usize << index) != 0;
                        (variable, !value)
                    })
                    .collect(),
            ));
        }
        witnesses.push(witness);
    }
    let mut transformed: Vec<_> = clauses
        .iter()
        .filter(|clause| !clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    transformed.extend(summary.iter().cloned());
    let core_to_original: Vec<_> = (0..vars)
        .filter(|variable| !interior_set.contains(variable))
        .collect();
    let mut original_to_core = vec![usize::MAX; vars];
    for (core, &original) in core_to_original.iter().enumerate() {
        original_to_core[original] = core;
    }
    for clause in &mut transformed {
        for (variable, _) in &mut clause.0 {
            *variable = original_to_core[*variable];
        }
    }
    SeededBranch {
        vars: core_to_original.len(),
        clauses: transformed,
        core_to_original,
        boundary,
        interior,
        witnesses,
        local_clauses: local.len(),
        summary_clauses: summary.len(),
        compilation_trials,
    }
}

struct BddSeededBranch {
    vars: usize,
    clauses: Vec<Clause>,
    core_to_original: Vec<usize>,
    boundary: Vec<usize>,
    interior: Vec<usize>,
    manager: BddManager,
    root: usize,
    order: Vec<usize>,
    local_clauses: usize,
    summary_clauses: usize,
    summary: Vec<Clause>,
    allocated_nodes: usize,
    live_nodes: usize,
    cache_root: usize,
}

fn reachable_bdd_nodes(manager: &BddManager, root: usize) -> usize {
    let mut seen = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(current) = stack.pop() {
        if current < 2 || !seen.insert(current) {
            continue;
        }
        let node = manager.node(current);
        stack.push(node.low);
        stack.push(node.high);
    }
    seen.len()
}

fn compact_bdd(manager: &BddManager, root: usize) -> (BddManager, usize) {
    fn copy_node(
        source: &BddManager,
        target: &mut BddManager,
        current: usize,
        memo: &mut HashMap<usize, usize>,
    ) -> usize {
        if current < 2 {
            return current;
        }
        if let Some(&mapped) = memo.get(&current) {
            return mapped;
        }
        let node = source.node(current);
        let low = copy_node(source, target, node.low, memo);
        let high = copy_node(source, target, node.high, memo);
        let mapped = target.make(node.variable, low, high);
        target.node_hits[mapped - 2] += source.node_hits[current - 2];
        memo.insert(current, mapped);
        mapped
    }
    let mut compacted = BddManager::default();
    let compacted_root = copy_node(manager, &mut compacted, root, &mut HashMap::new());
    (compacted, compacted_root)
}

fn compact_bdd_roots(manager: &BddManager, roots: &[usize]) -> (BddManager, Vec<usize>) {
    fn copy_node(
        source: &BddManager,
        target: &mut BddManager,
        current: usize,
        memo: &mut HashMap<usize, usize>,
    ) -> usize {
        if current < 2 {
            return current;
        }
        if let Some(&mapped) = memo.get(&current) {
            return mapped;
        }
        let node = source.node(current);
        let low = copy_node(source, target, node.low, memo);
        let high = copy_node(source, target, node.high, memo);
        let mapped = target.make(node.variable, low, high);
        target.node_hits[mapped - 2] += source.node_hits[current - 2];
        memo.insert(current, mapped);
        mapped
    }
    let mut compacted = BddManager::default();
    let mut memo = HashMap::new();
    let mapped = roots
        .iter()
        .map(|&root| copy_node(manager, &mut compacted, root, &mut memo))
        .collect();
    (compacted, mapped)
}

fn seed_detachable_branch_bdd(
    vars: usize,
    clauses: &[Clause],
    max_interior: usize,
    order_strategy: &str,
) -> BddSeededBranch {
    let mut manager = BddManager::default();
    seed_detachable_branch_bdd_in(vars, clauses, max_interior, order_strategy, &mut manager)
}

fn seed_detachable_branch_bdd_in(
    vars: usize,
    clauses: &[Clause],
    max_interior: usize,
    order_strategy: &str,
    manager: &mut BddManager,
) -> BddSeededBranch {
    let Some((mut interior, boundary)) = detachable_branch_candidate(vars, clauses, max_interior)
    else {
        return BddSeededBranch {
            vars,
            clauses: clauses.to_vec(),
            core_to_original: (0..vars).collect(),
            boundary: Vec::new(),
            interior: Vec::new(),
            manager: BddManager::default(),
            root: 1,
            order: Vec::new(),
            local_clauses: 0,
            summary_clauses: 0,
            summary: Vec::new(),
            allocated_nodes: 0,
            live_nodes: 0,
            cache_root: 1,
        };
    };
    seed_bdd_candidate_in(
        vars,
        clauses,
        &mut interior,
        boundary,
        order_strategy,
        manager,
    )
}

fn seed_bdd_candidate_in(
    vars: usize,
    clauses: &[Clause],
    interior: &mut Vec<usize>,
    boundary: Vec<usize>,
    order_strategy: &str,
    manager: &mut BddManager,
) -> BddSeededBranch {
    interior.sort_unstable();
    let interior_set: BTreeSet<_> = interior.iter().copied().collect();
    let local: Vec<_> = clauses
        .iter()
        .filter(|clause| clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    let relevant: BTreeSet<_> = boundary.iter().chain(interior.iter()).copied().collect();
    let mut order = match order_strategy {
        "min-fill" => restricted_order(&local, &relevant, true),
        "min-degree" => restricted_order(&local, &relevant, false),
        "boundary-min-fill" => {
            let mut result = boundary.clone();
            result.extend(
                restricted_order(&local, &relevant, true)
                    .into_iter()
                    .filter(|variable| interior.contains(variable)),
            );
            result
        }
        "boundary-min-degree" => {
            let mut result = boundary.clone();
            result.extend(
                restricted_order(&local, &relevant, false)
                    .into_iter()
                    .filter(|variable| interior.contains(variable)),
            );
            result
        }
        _ => {
            let mut natural = boundary.clone();
            natural.extend(interior.iter().copied());
            natural
        }
    };
    order.retain(|variable| relevant.contains(variable));
    let order_rank: HashMap<_, _> = order
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, variable)| (variable, rank))
        .collect();
    let allocated_before = manager.nodes.len();
    let root = compile_formula_bdd_into(manager, vars, &local, &order);
    let mut relation = root;
    for variable in interior.iter() {
        relation = manager.exists(relation, order_rank[variable], &mut HashMap::new());
    }
    let mut summary = Vec::new();
    for boundary_bits in 0..(1usize << boundary.len()) {
        let mut rank_assignment = vec![false; order.len()];
        for (index, variable) in boundary.iter().enumerate() {
            rank_assignment[order_rank[variable]] = boundary_bits & (1usize << index) != 0;
        }
        if !manager.evaluate(relation, &rank_assignment) {
            summary.push(Clause(
                boundary
                    .iter()
                    .enumerate()
                    .map(|(index, &variable)| {
                        let value = boundary_bits & (1usize << index) != 0;
                        (variable, !value)
                    })
                    .collect(),
            ));
        }
    }
    let mut transformed: Vec<_> = clauses
        .iter()
        .filter(|clause| !clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    transformed.extend(summary.iter().cloned());
    let core_to_original: Vec<_> = (0..vars)
        .filter(|variable| !interior_set.contains(variable))
        .collect();
    let mut original_to_core = vec![usize::MAX; vars];
    for (core, &original) in core_to_original.iter().enumerate() {
        original_to_core[original] = core;
    }
    for clause in &mut transformed {
        for (variable, _) in &mut clause.0 {
            *variable = original_to_core[*variable];
        }
    }
    let allocated_nodes = manager.nodes.len() - allocated_before;
    let live_nodes = reachable_bdd_nodes(manager, root);
    let cache_root = root;
    let (manager, root) = compact_bdd(manager, root);
    BddSeededBranch {
        vars: core_to_original.len(),
        clauses: transformed,
        core_to_original,
        boundary,
        interior: interior.clone(),
        manager,
        root,
        order,
        local_clauses: local.len(),
        summary_clauses: summary.len(),
        summary,
        allocated_nodes,
        live_nodes,
        cache_root,
    }
}

fn restricted_order(
    clauses: &[Clause],
    relevant: &BTreeSet<usize>,
    use_min_fill: bool,
) -> Vec<usize> {
    let originals: Vec<_> = relevant.iter().copied().collect();
    let to_local: HashMap<_, _> = originals
        .iter()
        .copied()
        .enumerate()
        .map(|(local, original)| (original, local))
        .collect();
    let compact: Vec<_> = clauses
        .iter()
        .map(|clause| {
            Clause(
                clause
                    .0
                    .iter()
                    .filter_map(|&(variable, positive)| {
                        to_local.get(&variable).map(|&local| (local, positive))
                    })
                    .collect(),
            )
        })
        .collect();
    let compact_order = if use_min_fill {
        min_fill_order(originals.len(), &compact)
    } else {
        min_degree_order(originals.len(), &compact)
    };
    compact_order
        .into_iter()
        .map(|local| originals[local])
        .collect()
}

fn clause_incidence(vars: usize, clauses: &[Clause]) -> Vec<Vec<usize>> {
    let mut incidence = vec![Vec::new(); vars];
    for (clause_index, clause) in clauses.iter().enumerate() {
        for &(variable, _) in &clause.0 {
            incidence[variable].push(clause_index);
        }
    }
    incidence
}

fn indexed_local_clauses(
    interior: &[usize],
    incidence: &[Vec<usize>],
    clauses: &[Clause],
) -> Vec<Clause> {
    let mut indices: Vec<_> = interior
        .iter()
        .flat_map(|&variable| incidence[variable].iter().copied())
        .collect();
    indices.sort_unstable();
    indices.dedup();
    indices
        .into_iter()
        .map(|index| clauses[index].clone())
        .collect()
}

fn seed_bdd_from_local(
    vars: usize,
    local: &[Clause],
    mut interior: Vec<usize>,
    boundary: Vec<usize>,
    order_strategy: &str,
    manager: &mut BddManager,
) -> BddSeededBranch {
    interior.sort_unstable();
    let relevant: BTreeSet<_> = boundary.iter().chain(interior.iter()).copied().collect();
    let mut order = match order_strategy {
        "min-fill" => restricted_order(local, &relevant, true),
        "min-degree" => restricted_order(local, &relevant, false),
        _ => {
            let mut natural = boundary.clone();
            natural.extend(interior.iter().copied());
            natural
        }
    };
    order.retain(|variable| relevant.contains(variable));
    let order_rank: HashMap<_, _> = order
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, variable)| (variable, rank))
        .collect();
    let allocated_before = manager.nodes.len();
    let root = compile_formula_bdd_into(manager, vars, local, &order);
    let mut relation = root;
    for &variable in &interior {
        relation = manager.exists(relation, order_rank[&variable], &mut HashMap::new());
    }
    let mut summary = Vec::new();
    for boundary_bits in 0..(1usize << boundary.len()) {
        let mut rank_assignment = vec![false; order.len()];
        for (index, variable) in boundary.iter().enumerate() {
            rank_assignment[order_rank[variable]] = boundary_bits & (1usize << index) != 0;
        }
        if !manager.evaluate(relation, &rank_assignment) {
            summary.push(Clause(
                boundary
                    .iter()
                    .enumerate()
                    .map(|(index, &variable)| (variable, boundary_bits & (1usize << index) == 0))
                    .collect(),
            ));
        }
    }
    let allocated_nodes = manager.nodes.len() - allocated_before;
    let live_nodes = reachable_bdd_nodes(manager, root);
    let cache_root = root;
    let (manager, root) = compact_bdd(manager, root);
    BddSeededBranch {
        vars: vars - interior.len(),
        clauses: Vec::new(),
        core_to_original: Vec::new(),
        boundary,
        interior,
        manager,
        root,
        order,
        local_clauses: local.len(),
        summary_clauses: summary.len(),
        summary,
        allocated_nodes,
        live_nodes,
        cache_root,
    }
}

struct BoundedSeedAttempt {
    seed: Option<BddSeededBranch>,
    nodes: usize,
    node_exceeded: bool,
    time_exceeded: bool,
}

fn try_seed_bdd_candidate(
    vars: usize,
    clauses: &[Clause],
    mut interior: Vec<usize>,
    boundary: Vec<usize>,
    order_strategy: &str,
    node_limit: usize,
    time_limit: std::time::Duration,
) -> BoundedSeedAttempt {
    let start = Instant::now();
    let mut manager = BddManager {
        node_limit: Some(node_limit),
        deadline: Some(start + time_limit),
        ..BddManager::default()
    };
    let seed = seed_bdd_candidate_in(
        vars,
        clauses,
        &mut interior,
        boundary,
        order_strategy,
        &mut manager,
    );
    let elapsed = start.elapsed();
    let node_exceeded = manager.nodes.len() >= node_limit && manager.budget_exceeded;
    let time_exceeded = (manager.budget_exceeded && !node_exceeded) || elapsed > time_limit;
    BoundedSeedAttempt {
        seed: (!manager.budget_exceeded && !time_exceeded).then_some(seed),
        nodes: manager.nodes.len(),
        node_exceeded,
        time_exceeded,
    }
}

fn try_indexed_seed_bdd_candidate(
    vars: usize,
    clauses: &[Clause],
    incidence: &[Vec<usize>],
    interior: Vec<usize>,
    boundary: Vec<usize>,
    node_limit: usize,
    time_limit: std::time::Duration,
) -> BoundedSeedAttempt {
    let start = Instant::now();
    let local = indexed_local_clauses(&interior, incidence, clauses);
    let mut manager = BddManager {
        node_limit: Some(node_limit),
        deadline: Some(start + time_limit),
        ..BddManager::default()
    };
    let seed = seed_bdd_from_local(vars, &local, interior, boundary, "min-fill", &mut manager);
    let elapsed = start.elapsed();
    let node_exceeded = manager.nodes.len() >= node_limit && manager.budget_exceeded;
    let time_exceeded = (manager.budget_exceeded && !node_exceeded) || elapsed > time_limit;
    BoundedSeedAttempt {
        seed: (!manager.budget_exceeded && !time_exceeded).then_some(seed),
        nodes: manager.nodes.len(),
        node_exceeded,
        time_exceeded,
    }
}

fn regrow_bdd_seed(seed: &BddSeededBranch, original_assignment: &[bool]) -> Option<Vec<bool>> {
    if seed.interior.is_empty() {
        return Some(Vec::new());
    }
    fn compatible(
        seed: &BddSeededBranch,
        root: usize,
        original_assignment: &[bool],
        memo: &mut HashMap<usize, bool>,
    ) -> bool {
        if root < 2 {
            return root == 1;
        }
        if let Some(&result) = memo.get(&root) {
            return result;
        }
        let node = seed.manager.node(root);
        let original_variable = seed.order[node.variable];
        let result = if seed.boundary.contains(&original_variable) {
            compatible(
                seed,
                if original_assignment[original_variable] {
                    node.high
                } else {
                    node.low
                },
                original_assignment,
                memo,
            )
        } else {
            compatible(seed, node.low, original_assignment, memo)
                || compatible(seed, node.high, original_assignment, memo)
        };
        memo.insert(root, result);
        result
    }
    if !compatible(seed, seed.root, original_assignment, &mut HashMap::new()) {
        return None;
    }
    let mut rank_assignment = vec![false; seed.order.len()];
    let mut root = seed.root;
    while root >= 2 {
        let node = seed.manager.node(root);
        let original_variable = seed.order[node.variable];
        let value = if seed.boundary.contains(&original_variable) {
            original_assignment[original_variable]
        } else {
            !compatible(seed, node.low, original_assignment, &mut HashMap::new())
        };
        rank_assignment[node.variable] = value;
        root = if value { node.high } else { node.low };
    }
    if root == 0 {
        return None;
    }
    let rank: HashMap<_, _> = seed
        .order
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, variable)| (variable, rank))
        .collect();
    Some(
        seed.interior
            .iter()
            .map(|variable| rank_assignment[rank[variable]])
            .collect(),
    )
}

fn regrow_seed_chain(seeds: &[BddSeededBranch], final_assignment: &[bool]) -> Option<Vec<bool>> {
    let mut assignment = final_assignment.to_vec();
    for seed in seeds.iter().rev() {
        let previous_vars = seed.core_to_original.len() + seed.interior.len();
        let mut previous = vec![false; previous_vars];
        for (core, &original) in seed.core_to_original.iter().enumerate() {
            previous[original] = assignment[core];
        }
        let values = regrow_bdd_seed(seed, &previous)?;
        for (index, &variable) in seed.interior.iter().enumerate() {
            previous[variable] = values[index];
        }
        assignment = previous;
    }
    Some(assignment)
}

fn deployment_features(vars: usize, clauses: &[Clause], branch_cap: usize) -> Vec<f64> {
    let graph = primal_graph(vars, clauses);
    let degrees: Vec<_> = graph
        .iter()
        .map(|neighbors| neighbors.len() as f64)
        .collect();
    let mean_degree = degrees.iter().sum::<f64>() / vars.max(1) as f64;
    let degree_variance = degrees
        .iter()
        .map(|degree| (degree - mean_degree).powi(2))
        .sum::<f64>()
        / vars.max(1) as f64;
    let low_degree =
        degrees.iter().filter(|&&degree| degree <= 2.0).count() as f64 / vars.max(1) as f64;
    let locality = clauses
        .iter()
        .map(|clause| {
            let minimum = clause.0.iter().map(|literal| literal.0).min().unwrap_or(0);
            let maximum = clause.0.iter().map(|literal| literal.0).max().unwrap_or(0);
            (maximum - minimum) as f64 / vars.max(1) as f64
        })
        .sum::<f64>()
        / clauses.len().max(1) as f64;
    let candidate = detachable_branch_candidate(vars, clauses, branch_cap);
    vec![
        clauses.len() as f64 / vars.max(1) as f64,
        mean_degree / vars.max(1) as f64,
        degree_variance / (vars * vars).max(1) as f64,
        low_degree,
        locality,
        leaf_richness(vars, clauses) as f64 / vars.max(1) as f64,
        candidate
            .as_ref()
            .map_or(0.0, |item| item.0.len() as f64 / vars.max(1) as f64),
        candidate
            .as_ref()
            .map_or(0.0, |item| item.1.len() as f64 / 2.0),
    ]
}

struct DeploymentMeasurement {
    incremental_ns: u128,
    seeded_ns: u128,
    incremental_setup_ns: u128,
    seeded_setup_ns: u128,
    incremental_query_ns: u128,
    seeded_query_ns: u128,
    reconstruction_ns: u128,
    reconstruction_samples: usize,
    seeds: usize,
    removed: usize,
    live_nodes: usize,
    valid: bool,
}

fn measure_assumption_service(
    vars: usize,
    clauses: &[Clause],
    branch_cap: usize,
    max_seeds: usize,
    queries: usize,
) -> DeploymentMeasurement {
    measure_assumption_service_warm(vars, clauses, branch_cap, max_seeds, 0, queries)
}

fn measure_assumption_service_warm(
    vars: usize,
    clauses: &[Clause],
    branch_cap: usize,
    max_seeds: usize,
    warmup: usize,
    queries: usize,
) -> DeploymentMeasurement {
    let incremental_start = Instant::now();
    let mut incremental = Solver::new();
    add_to_varisat(&mut incremental, clauses);
    let incremental_setup_ns = incremental_start.elapsed().as_nanos();
    let seed_start = Instant::now();
    let mut current_vars = vars;
    let mut current_clauses = clauses.to_vec();
    let mut current_to_original: Vec<_> = (0..vars).collect();
    let mut seeds = Vec::new();
    for _ in 0..max_seeds {
        let compiled =
            seed_detachable_branch_bdd(current_vars, &current_clauses, branch_cap, "natural");
        if compiled.interior.is_empty() {
            break;
        }
        current_to_original = compiled
            .core_to_original
            .iter()
            .map(|&previous| current_to_original[previous])
            .collect();
        current_vars = compiled.vars;
        current_clauses = compiled.clauses.clone();
        seeds.push(compiled);
    }
    let mut seed_solver = Solver::new();
    add_to_varisat(&mut seed_solver, &current_clauses);
    let seeded_setup_ns = seed_start.elapsed().as_nanos();
    let mut incremental_query_ns = 0u128;
    let mut seeded_query_ns = 0u128;
    let mut reconstruction_ns = 0u128;
    let mut reconstruction_samples = 0usize;
    let mut valid = current_vars > 0;
    if current_vars > 0 {
        for query in 0..warmup + queries {
            let core_variable = query % current_vars;
            let original_variable = current_to_original[core_variable];
            let value = (query / current_vars + query) % 2 == 0;
            incremental.assume(&[Lit::from_var(Var::from_index(original_variable), value)]);
            let start = Instant::now();
            let incremental_sat = incremental.solve().expect("deployment incremental solve");
            let elapsed = start.elapsed().as_nanos();
            if query >= warmup {
                incremental_query_ns += elapsed;
            }
            seed_solver.assume(&[Lit::from_var(Var::from_index(core_variable), value)]);
            let start = Instant::now();
            let seed_sat = seed_solver.solve().expect("deployment seed solve");
            let elapsed = start.elapsed().as_nanos();
            if query >= warmup {
                seeded_query_ns += elapsed;
            }
            valid &= incremental_sat == seed_sat;
            if seed_sat && query >= warmup && (query < warmup + 4 || query + 1 == warmup + queries)
            {
                let reconstruction_start = Instant::now();
                let mut core_assignment = vec![false; current_vars];
                for literal in seed_solver.model().expect("deployment seed model") {
                    if literal.var().index() < current_vars {
                        core_assignment[literal.var().index()] = literal.is_positive();
                    }
                }
                valid &= regrow_seed_chain(&seeds, &core_assignment).is_some_and(|assignment| {
                    assignment[original_variable] == value && satisfies(clauses, &assignment)
                });
                reconstruction_ns += reconstruction_start.elapsed().as_nanos();
                reconstruction_samples += 1;
            }
        }
    }
    DeploymentMeasurement {
        incremental_ns: incremental_setup_ns + incremental_query_ns,
        seeded_ns: seeded_setup_ns + seeded_query_ns,
        incremental_setup_ns,
        seeded_setup_ns,
        incremental_query_ns,
        seeded_query_ns,
        reconstruction_ns,
        reconstruction_samples,
        seeds: seeds.len(),
        removed: seeds.iter().map(|seed| seed.interior.len()).sum(),
        live_nodes: seeds.iter().map(|seed| seed.live_nodes).sum(),
        valid,
    }
}

fn deployment_knn(training: &[(Vec<f64>, f64)], features: &[f64], skip: Option<usize>) -> f64 {
    let dimensions = features.len();
    let scales: Vec<_> = (0..dimensions)
        .map(|dimension| {
            let mean = training.iter().map(|item| item.0[dimension]).sum::<f64>()
                / training.len().max(1) as f64;
            (training
                .iter()
                .map(|item| (item.0[dimension] - mean).powi(2))
                .sum::<f64>()
                / training.len().max(1) as f64)
                .sqrt()
                .max(1e-9)
        })
        .collect();
    let mut neighbours: Vec<_> = training
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != skip)
        .map(|(_, item)| {
            let distance = (0..dimensions)
                .map(|dimension| {
                    ((features[dimension] - item.0[dimension]) / scales[dimension]).powi(2)
                })
                .sum::<f64>();
            (distance, item.1)
        })
        .collect();
    neighbours.sort_by(|left, right| left.0.total_cmp(&right.0));
    let count = 7.min(neighbours.len()).max(1);
    neighbours
        .iter()
        .take(count)
        .map(|item| item.1)
        .sum::<f64>()
        / count as f64
}

fn upper_error_margin(training: &[(Vec<f64>, f64)]) -> f64 {
    let mut errors: Vec<_> = training
        .iter()
        .enumerate()
        .map(|(index, item)| deployment_knn(training, &item.0, Some(index)) - item.1)
        .collect();
    errors.sort_by(|left, right| left.total_cmp(right));
    errors[((errors.len().saturating_sub(1)) * 9) / 10].max(0.0)
}

fn propagate_units(clauses: &[Clause], assignment: &mut [Option<bool>]) -> Result<Vec<Clause>, ()> {
    let mut current = clauses.to_vec();
    loop {
        let mut reduced = Vec::new();
        let mut units = Vec::new();
        for clause in &current {
            if clause
                .0
                .iter()
                .any(|&(variable, sign)| assignment[variable] == Some(sign))
            {
                continue;
            }
            let literals: Vec<_> = clause
                .0
                .iter()
                .copied()
                .filter(|&(variable, sign)| assignment[variable] != Some(!sign))
                .collect();
            if literals.is_empty() {
                return Err(());
            }
            if literals.len() == 1 {
                units.push(literals[0]);
            }
            reduced.push(Clause(literals));
        }
        let mut changed = false;
        for (variable, sign) in units {
            if assignment[variable].is_some_and(|value| value != sign) {
                return Err(());
            }
            if assignment[variable].is_none() {
                assignment[variable] = Some(sign);
                changed = true;
            }
        }
        current = reduced;
        if !changed {
            return Ok(current);
        }
    }
}

fn probe_contradiction(
    clauses: &[Clause],
    fixed: &[Option<bool>],
    variable: usize,
    value: bool,
    depth: usize,
    probes: &mut usize,
) -> bool {
    *probes += 1;
    let mut trial = fixed.to_vec();
    trial[variable] = Some(value);
    let Ok(reduced) = propagate_units(clauses, &mut trial) else {
        return true;
    };
    if depth <= 1 {
        return false;
    }
    let graph = primal_graph(trial.len(), &reduced);
    let mut candidates: Vec<_> = (0..trial.len())
        .filter(|&candidate| trial[candidate].is_none())
        .collect();
    candidates.sort_by_key(|&candidate| std::cmp::Reverse(graph[candidate].len()));
    candidates.into_iter().any(|candidate| {
        probe_contradiction(&reduced, &trial, candidate, false, depth - 1, probes)
            && probe_contradiction(&reduced, &trial, candidate, true, depth - 1, probes)
    })
}

fn leaf_richness(vars: usize, clauses: &[Clause]) -> usize {
    let graph = primal_graph(vars, clauses);
    let mut positive = vec![false; vars];
    let mut negative = vec![false; vars];
    let mut units = 0;
    for clause in clauses {
        units += usize::from(clause.0.len() == 1);
        for &(variable, sign) in &clause.0 {
            if sign {
                positive[variable] = true;
            } else {
                negative[variable] = true;
            }
        }
    }
    units
        + (0..vars)
            .filter(|&variable| {
                positive[variable] != negative[variable] || graph[variable].len() <= 2
            })
            .count()
}

fn shake_formula(
    vars: usize,
    clauses: &[Clause],
    inverse_depth: usize,
    probe_limit: usize,
) -> ShakenFormula {
    let mut fixed = vec![None; vars];
    let mut current = clauses.to_vec();
    let mut probes = 0;
    let mut inverse_forced = 0;
    let mut contradiction = false;
    loop {
        current = match propagate_units(&current, &mut fixed) {
            Ok(reduced) => reduced,
            Err(()) => {
                contradiction = true;
                Vec::new()
            }
        };
        if contradiction {
            break;
        }
        let mut positive = vec![false; vars];
        let mut negative = vec![false; vars];
        for clause in &current {
            for &(variable, sign) in &clause.0 {
                if sign {
                    positive[variable] = true;
                } else {
                    negative[variable] = true;
                }
            }
        }
        let mut changed = false;
        for variable in 0..vars {
            if fixed[variable].is_none() && positive[variable] != negative[variable] {
                fixed[variable] = Some(positive[variable]);
                changed = true;
            }
        }
        if changed {
            continue;
        }
        if inverse_depth > 0 {
            let graph = primal_graph(vars, &current);
            let mut candidates: Vec<_> = (0..vars)
                .filter(|&variable| fixed[variable].is_none())
                .collect();
            candidates.sort_by_key(|&variable| std::cmp::Reverse(graph[variable].len()));
            for variable in candidates.into_iter().take(probe_limit) {
                let false_fails = probe_contradiction(
                    &current,
                    &fixed,
                    variable,
                    false,
                    inverse_depth,
                    &mut probes,
                );
                let true_fails = probe_contradiction(
                    &current,
                    &fixed,
                    variable,
                    true,
                    inverse_depth,
                    &mut probes,
                );
                if false_fails && true_fails {
                    contradiction = true;
                    break;
                }
                if false_fails || true_fails {
                    fixed[variable] = Some(false_fails);
                    inverse_forced += 1;
                    changed = true;
                    break;
                }
            }
        }
        if contradiction || !changed {
            break;
        }
    }
    let mut active = vec![false; vars];
    for clause in &current {
        for &(variable, _) in &clause.0 {
            active[variable] = true;
        }
    }
    let core_to_original: Vec<_> = (0..vars).filter(|&variable| active[variable]).collect();
    let mut original_to_core = vec![usize::MAX; vars];
    for (core, &original) in core_to_original.iter().enumerate() {
        original_to_core[original] = core;
    }
    let compacted = current
        .iter()
        .map(|clause| {
            Clause(
                clause
                    .0
                    .iter()
                    .map(|&(variable, sign)| (original_to_core[variable], sign))
                    .collect(),
            )
        })
        .collect();
    ShakenFormula {
        vars: core_to_original.len(),
        clauses: compacted,
        core_to_original,
        removed: active.iter().filter(|&&item| !item).count(),
        fixed,
        probes,
        inverse_forced,
        contradiction,
    }
}

fn osmotic_helper_score(
    vars: usize,
    clauses: &[Clause],
    pair: (Literal, Literal),
    frequency: usize,
) -> f64 {
    let graph = primal_graph(vars, clauses);
    let (a, b) = (pair.0.0, pair.1.0);
    let shared = graph[a].intersection(&graph[b]).count();
    let pressure_difference = graph[a].len().abs_diff(graph[b].len());
    frequency as f64 * (shared + 1) as f64 / (pressure_difference + 1) as f64
}

fn warp_helper_score(
    vars: usize,
    clauses: &[Clause],
    pair: (Literal, Literal),
    frequency: usize,
) -> f64 {
    let order = min_fill_order(vars, clauses);
    let mut rank = vec![0usize; vars];
    for (level, &variable) in order.iter().enumerate() {
        rank[variable] = level;
    }
    let graph = primal_graph(vars, clauses);
    let (a, b) = (pair.0.0, pair.1.0);
    let forward_span = rank[a].abs_diff(rank[b]) + 1;
    let shared = graph[a].intersection(&graph[b]).count() + 1;
    frequency as f64 * forward_span as f64 * shared as f64
}

fn expand_two_greedy(
    vars: usize,
    clauses: &[Clause],
    candidate_limit: usize,
    warp: bool,
) -> (usize, Vec<Clause>, usize) {
    let mut current_vars = vars;
    let mut current = clauses.to_vec();
    let mut added = 0;
    for _ in 0..2 {
        let candidates: Vec<_> = recurring_pair_candidates(&current)
            .into_iter()
            .take(candidate_limit)
            .collect();
        let choice =
            if warp {
                candidates.iter().max_by(|a, b| {
                    warp_helper_score(current_vars, &current, a.0, a.1)
                        .total_cmp(&warp_helper_score(current_vars, &current, b.0, b.1))
                })
            } else {
                candidates.first()
            };
        let Some(&(pair, _)) = choice else { break };
        let Some(next) = add_pair_helper(current_vars, &current, pair) else {
            break;
        };
        current = next;
        current_vars += 1;
        added += 1;
    }
    (current_vars, current, added)
}

fn remove_with_fill(graph: &mut [BTreeSet<usize>], alive: &mut BTreeSet<usize>, v: usize) {
    let neighbors: Vec<_> = graph[v].intersection(alive).copied().collect();
    for &a in &neighbors {
        for &b in &neighbors {
            if a != b {
                graph[a].insert(b);
            }
        }
    }
    alive.remove(&v);
}

fn min_degree_order(vars: usize, clauses: &[Clause]) -> Vec<usize> {
    let mut graph = primal_graph(vars, clauses);
    let mut alive: BTreeSet<_> = (0..vars).collect();
    let mut order = Vec::new();
    while !alive.is_empty() {
        let &v = alive
            .iter()
            .min_by_key(|&&v| graph[v].intersection(&alive).count())
            .unwrap();
        remove_with_fill(&mut graph, &mut alive, v);
        order.push(v);
    }
    order
}

fn min_fill_order(vars: usize, clauses: &[Clause]) -> Vec<usize> {
    let mut graph = primal_graph(vars, clauses);
    let mut alive: BTreeSet<_> = (0..vars).collect();
    let mut order = Vec::new();
    while !alive.is_empty() {
        let &v = alive
            .iter()
            .min_by_key(|&&v| {
                let neighbors: Vec<_> = graph[v].intersection(&alive).copied().collect();
                let missing = neighbors
                    .iter()
                    .enumerate()
                    .map(|(i, &a)| {
                        neighbors[i + 1..]
                            .iter()
                            .filter(|&&b| !graph[a].contains(&b))
                            .count()
                    })
                    .sum::<usize>();
                (missing, neighbors.len(), v)
            })
            .unwrap();
        remove_with_fill(&mut graph, &mut alive, v);
        order.push(v);
    }
    order
}

fn elimination_cost(vars: usize, clauses: &[Clause], order: &[usize]) -> (usize, f64) {
    let mut graph = primal_graph(vars, clauses);
    let mut alive: BTreeSet<_> = (0..vars).collect();
    let mut max_width = 0;
    let mut estimated_work = 0.0;
    for &variable in order {
        let degree = graph[variable].intersection(&alive).count();
        max_width = max_width.max(degree);
        estimated_work += 2.0f64.powi(degree.min(1023) as i32);
        remove_with_fill(&mut graph, &mut alive, variable);
    }
    (max_width, estimated_work)
}

fn greedy_clique_lower_bound(vars: usize, clauses: &[Clause]) -> usize {
    if vars == 0 {
        return 0;
    }
    let graph = primal_graph(vars, clauses);
    let mut best = 1usize;
    for start in 0..vars {
        let mut clique = vec![start];
        let mut candidates = graph[start].clone();
        while !candidates.is_empty() {
            let next = *candidates
                .iter()
                .max_by_key(|&&candidate| {
                    (
                        graph[candidate].intersection(&candidates).count(),
                        graph[candidate].len(),
                    )
                })
                .expect("non-empty clique candidates");
            clique.push(next);
            candidates = candidates.intersection(&graph[next]).copied().collect();
        }
        best = best.max(clique.len());
    }
    best.saturating_sub(1)
}

fn minor_min_width_lower_bound(vars: usize, clauses: &[Clause]) -> usize {
    let mut graph = primal_graph(vars, clauses);
    let mut alive: BTreeSet<_> = (0..vars).collect();
    let mut lower = 0usize;
    while let Some(&variable) = alive
        .iter()
        .min_by_key(|&&v| (graph[v].intersection(&alive).count(), v))
    {
        let neighbours: Vec<_> = graph[variable].intersection(&alive).copied().collect();
        lower = lower.max(neighbours.len());
        let Some(&target) = neighbours
            .iter()
            .min_by_key(|&&v| (graph[v].intersection(&alive).count(), v))
        else {
            alive.remove(&variable);
            continue;
        };
        for neighbour in neighbours {
            if neighbour == target {
                continue;
            }
            graph[neighbour].remove(&variable);
            graph[neighbour].insert(target);
            graph[target].insert(neighbour);
        }
        graph[target].remove(&variable);
        graph[variable].clear();
        alive.remove(&variable);
    }
    lower
}

fn structural_treewidth_lower_bound(vars: usize, clauses: &[Clause]) -> usize {
    greedy_clique_lower_bound(vars, clauses).max(minor_min_width_lower_bound(vars, clauses))
}

#[derive(Debug)]
struct PredictedExpansion {
    vars: usize,
    clauses: Vec<Clause>,
    accepted: usize,
    recursive_accepted: usize,
    candidates_scored: usize,
    initial_width: usize,
    final_width: usize,
    initial_estimated_work: f64,
    final_estimated_work: f64,
}

fn predicted_joint_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    candidate_limit: usize,
) -> PredictedExpansion {
    let mut current_vars = vars;
    let mut current = clauses.to_vec();
    let initial_order = min_fill_order(current_vars, &current);
    let (initial_width, initial_estimated_work) =
        elimination_cost(current_vars, &current, &initial_order);
    let mut current_cost = (initial_width, initial_estimated_work);
    let mut accepted = 0;
    let mut recursive_accepted = 0;
    let mut candidates_scored = 0;

    while accepted < max_helpers {
        let mut best: Option<((Literal, Literal), Vec<Clause>, (usize, f64))> = None;
        for (pair, _) in recurring_pair_candidates(&current)
            .into_iter()
            .take(candidate_limit)
        {
            let Some(candidate) = add_pair_helper(current_vars, &current, pair) else {
                continue;
            };
            candidates_scored += 1;
            let candidate_vars = current_vars + 1;
            let candidate_order = min_fill_order(candidate_vars, &candidate);
            let cost = elimination_cost(candidate_vars, &candidate, &candidate_order);
            let better_than_best = best.as_ref().is_none_or(|(_, _, best_cost)| {
                cost.0 < best_cost.0 || (cost.0 == best_cost.0 && cost.1 < best_cost.1)
            });
            if better_than_best {
                best = Some((pair, candidate, cost));
            }
        }
        let Some((pair, next, next_cost)) = best else {
            break;
        };
        let improves = next_cost.0 < current_cost.0
            || (next_cost.0 == current_cost.0 && next_cost.1 < current_cost.1);
        if !improves {
            break;
        }
        if pair.0.0 >= vars || pair.1.0 >= vars {
            recursive_accepted += 1;
        }
        current = next;
        current_vars += 1;
        current_cost = next_cost;
        accepted += 1;
    }
    PredictedExpansion {
        vars: current_vars,
        clauses: current,
        accepted,
        recursive_accepted,
        candidates_scored,
        initial_width,
        final_width: current_cost.0,
        initial_estimated_work,
        final_estimated_work: current_cost.1,
    }
}

fn semantic_batch_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    interactions: &[((Literal, Literal), usize)],
) -> (usize, Vec<Clause>, usize, usize) {
    let interaction_score: HashMap<_, _> = interactions.iter().copied().collect();
    let mut candidates = recurring_pair_candidates(clauses);
    candidates.sort_by_key(|&(pair, frequency)| {
        let semantic = interaction_score.get(&pair).copied().unwrap_or(0);
        std::cmp::Reverse((frequency as u128) * (semantic as u128 + 1))
    });
    let mut current = clauses.to_vec();
    let mut current_vars = vars;
    let mut accepted = 0;
    let mut scored = 0;
    for (pair, _) in candidates {
        if accepted == max_helpers {
            break;
        }
        scored += 1;
        if let Some(next) = add_pair_helper(current_vars, &current, pair) {
            current = next;
            current_vars += 1;
            accepted += 1;
        }
    }
    (current_vars, current, accepted, scored)
}

fn scent_batch_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    hops: usize,
) -> (usize, Vec<Clause>, usize, usize) {
    let scent = diffuse_scent_variant(vars, clauses, hops, 0);
    let graph = primal_graph(vars, clauses);
    let candidates = scored_scent_candidates(clauses, &graph, &scent);
    let scored = candidates.len();
    let mut current = clauses.to_vec();
    let mut current_vars = vars;
    let mut accepted = 0;
    for (pair, _, _) in candidates {
        if accepted == max_helpers {
            break;
        }
        if let Some(next) = add_pair_helper(current_vars, &current, pair) {
            current = next;
            current_vars += 1;
            accepted += 1;
        }
    }
    (current_vars, current, accepted, scored)
}

fn scored_scent_candidates(
    clauses: &[Clause],
    graph: &[BTreeSet<usize>],
    scent: &[f64],
) -> Vec<((Literal, Literal), usize, f64)> {
    let mut candidates: Vec<_> = recurring_pair_candidates(clauses)
        .into_iter()
        .map(|(pair, frequency)| {
            let a = pair.0.0;
            let b = pair.1.0;
            let shared = graph[a].intersection(&graph[b]).count();
            let distinct = graph[a].symmetric_difference(&graph[b]).count();
            let score =
                (frequency as f64 + 1.0).ln() * (scent[a] + scent[b]) * (1.0 + shared as f64)
                    / (1.0 + distinct as f64).sqrt();
            (pair, frequency, score)
        })
        .collect();
    candidates.sort_by(|a, b| b.2.total_cmp(&a.2));
    candidates
}

const HELPER_GATE_FEATURES: usize = 7;

fn helper_gate_features(
    vars: usize,
    clauses: &[Clause],
    hops: usize,
    budget: usize,
) -> [f64; HELPER_GATE_FEATURES] {
    let scent = diffuse_scent_variant(vars, clauses, hops, 0);
    let graph = primal_graph(vars, clauses);
    let candidates = scored_scent_candidates(clauses, &graph, &scent);
    let scent_mean = scent.iter().sum::<f64>() / scent.len().max(1) as f64;
    let scent_variance = scent
        .iter()
        .map(|value| (value - scent_mean).powi(2))
        .sum::<f64>()
        / scent.len().max(1) as f64;
    let top_score = candidates
        .first()
        .map(|candidate| candidate.2)
        .unwrap_or(0.0);
    let next_score = candidates
        .get(budget)
        .map(|candidate| candidate.2)
        .unwrap_or(0.0);
    let frequency_sum = candidates
        .iter()
        .map(|candidate| candidate.1)
        .sum::<usize>();
    let top_frequency = candidates
        .iter()
        .take(budget)
        .map(|candidate| candidate.1)
        .sum::<usize>();
    let selected: Vec<_> = candidates.iter().take(budget).collect();
    let overlap = selected
        .iter()
        .map(|candidate| {
            let a = candidate.0.0.0;
            let b = candidate.0.1.0;
            graph[a].intersection(&graph[b]).count() as f64
                / graph[a].union(&graph[b]).count().max(1) as f64
        })
        .sum::<f64>()
        / selected.len().max(1) as f64;
    let mut variable_uses = HashMap::new();
    for candidate in &selected {
        *variable_uses.entry(candidate.0.0.0).or_insert(0usize) += 1;
        *variable_uses.entry(candidate.0.1.0).or_insert(0usize) += 1;
    }
    let interacting_uses = variable_uses
        .values()
        .map(|uses| uses.saturating_sub(1))
        .sum::<usize>();
    [
        1.0,
        scent.iter().copied().fold(0.0, f64::max) / scent_mean.max(1e-9),
        scent_variance.sqrt() / scent_mean.max(1e-9),
        (top_score - next_score) / top_score.max(1e-9),
        top_frequency as f64 / frequency_sum.max(1) as f64,
        overlap,
        interacting_uses as f64 / (2 * selected.len()).max(1) as f64,
    ]
}

const GRAPH_MESSAGE_WIDTH: usize = 8;

#[derive(Clone, Copy, Debug)]
struct MessageParams {
    self_weight: f64,
    clause_weight: f64,
    variable_weight: f64,
    candidate_weight: f64,
    memory_retention: f64,
}

const DEFAULT_MESSAGE_PARAMS: MessageParams = MessageParams {
    self_weight: 0.5,
    clause_weight: 1.0,
    variable_weight: 1.0,
    candidate_weight: 1.0,
    memory_retention: 1.0,
};

fn message_parameter_candidates(ghost: bool) -> Vec<MessageParams> {
    let profiles = [
        (1.0, 1.0, 1.0),
        (2.0, 1.0, 1.0),
        (1.0, 2.0, 1.0),
        (1.0, 1.0, 2.0),
    ];
    let retentions: &[f64] = if ghost { &[0.5, 1.0] } else { &[1.0] };
    let mut result = Vec::new();
    for self_weight in [0.25, 0.5, 0.75] {
        for &(clause_weight, variable_weight, candidate_weight) in &profiles {
            for &memory_retention in retentions {
                result.push(MessageParams {
                    self_weight,
                    clause_weight,
                    variable_weight,
                    candidate_weight,
                    memory_retention,
                });
            }
        }
    }
    result
}

fn helper_graph_features(
    vars: usize,
    clauses: &[Clause],
    hops: usize,
    candidate_limit: usize,
    mode: u8,
) -> Vec<f64> {
    helper_graph_features_with_params(
        vars,
        clauses,
        hops,
        candidate_limit,
        mode,
        DEFAULT_MESSAGE_PARAMS,
    )
}

fn helper_graph_features_with_params(
    vars: usize,
    clauses: &[Clause],
    hops: usize,
    candidate_limit: usize,
    mode: u8,
    params: MessageParams,
) -> Vec<f64> {
    let scent = diffuse_scent_variant(vars, clauses, hops, 0);
    let scent_mean = scent.iter().sum::<f64>() / scent.len().max(1) as f64;
    let graph = primal_graph(vars, clauses);
    let candidates = scored_scent_candidates(clauses, &graph, &scent);
    let candidates: Vec<_> = candidates.into_iter().take(candidate_limit).collect();
    let clause_offset = vars;
    let candidate_offset = vars + clauses.len();
    let node_count = candidate_offset + candidates.len();
    let mut adjacency = vec![BTreeSet::new(); node_count];
    let mut state = vec![[0.0; GRAPH_MESSAGE_WIDTH]; node_count];
    let max_score = candidates
        .first()
        .map(|candidate| candidate.2)
        .unwrap_or(1.0);
    let max_frequency = candidates
        .iter()
        .map(|candidate| candidate.1)
        .max()
        .unwrap_or(1);
    let mut positive = vec![0usize; vars];
    let mut negative = vec![0usize; vars];
    for (clause_index, clause) in clauses.iter().enumerate() {
        let clause_node = clause_offset + clause_index;
        state[clause_node][1] = 1.0;
        state[clause_node][3] = clause.0.iter().map(|literal| scent[literal.0]).sum::<f64>()
            / clause.0.len().max(1) as f64
            / scent_mean.max(1e-9);
        state[clause_node][4] = clause.0.len() as f64 / 3.0;
        state[clause_node][5] = clause.0.iter().filter(|literal| literal.1).count() as f64
            / clause.0.len().max(1) as f64;
        state[clause_node][7] = 1.0;
        for &(variable, sign) in &clause.0 {
            adjacency[variable].insert(clause_node);
            adjacency[clause_node].insert(variable);
            if sign {
                positive[variable] += 1
            } else {
                negative[variable] += 1
            }
        }
    }
    for variable in 0..vars {
        state[variable][0] = 1.0;
        state[variable][3] = scent[variable] / scent_mean.max(1e-9);
        state[variable][4] = graph[variable].len() as f64 / vars.max(1) as f64;
        state[variable][5] = positive[variable].min(negative[variable]) as f64
            / (positive[variable] + negative[variable]).max(1) as f64;
        state[variable][7] = 1.0;
    }
    for (index, &(pair, frequency, score)) in candidates.iter().enumerate() {
        let node = candidate_offset + index;
        state[node][2] = 1.0;
        state[node][3] = (scent[pair.0.0] + scent[pair.1.0]) / (2.0 * scent_mean.max(1e-9));
        state[node][4] = frequency as f64 / max_frequency.max(1) as f64;
        state[node][5] = score / max_score.max(1e-9);
        state[node][6] = f64::from(pair.0.1 == pair.1.1);
        state[node][7] = 1.0;
        for variable in [pair.0.0, pair.1.0] {
            adjacency[node].insert(variable);
            adjacency[variable].insert(node);
        }
    }
    for a in 0..candidates.len() {
        for b in a + 1..candidates.len() {
            let a_vars = [candidates[a].0.0.0, candidates[a].0.1.0];
            let b_vars = [candidates[b].0.0.0, candidates[b].0.1.0];
            if a_vars.iter().any(|variable| b_vars.contains(variable)) {
                adjacency[candidate_offset + a].insert(candidate_offset + b);
                adjacency[candidate_offset + b].insert(candidate_offset + a);
            }
        }
    }
    let initial_state = state.clone();
    let mut active = vec![true; node_count];
    if mode == 2 {
        for item in active.iter_mut().take(candidate_offset).skip(clause_offset) {
            *item = false;
        }
    }
    for round in 0..3 {
        let previous = state.clone();
        for node in 0..node_count {
            if !active[node] {
                continue;
            }
            let neighbours: Vec<_> = adjacency[node]
                .iter()
                .filter(|&&other| active[other])
                .copied()
                .collect();
            if neighbours.is_empty() {
                continue;
            }
            for feature in 0..GRAPH_MESSAGE_WIDTH {
                let mut weighted_sum = 0.0;
                let mut weight_sum = 0.0;
                for &other in &neighbours {
                    let weight = if other < clause_offset {
                        params.variable_weight
                    } else if other < candidate_offset {
                        params.clause_weight
                    } else {
                        params.candidate_weight
                    };
                    weighted_sum += weight * previous[other][feature];
                    weight_sum += weight;
                }
                let mean = weighted_sum / weight_sum.max(1e-9);
                state[node][feature] = params.self_weight * previous[node][feature]
                    + (1.0 - params.self_weight) * mean;
            }
        }
        if mode == 1 && round == 0 {
            for item in active.iter_mut().take(candidate_offset).skip(clause_offset) {
                *item = false;
            }
        }
        if (mode == 1 && round == 1) || (mode == 2 && round == 0) {
            for item in active.iter_mut().take(vars) {
                *item = false;
            }
        }
        if mode == 1 && (round == 0 || round == 1) {
            for node in candidate_offset..node_count {
                for feature in 0..GRAPH_MESSAGE_WIDTH {
                    state[node][feature] = params.memory_retention * state[node][feature]
                        + (1.0 - params.memory_retention) * initial_state[node][feature];
                }
            }
        }
    }
    let mut features = Vec::with_capacity(GRAPH_MESSAGE_WIDTH * 2);
    for feature in 0..GRAPH_MESSAGE_WIDTH {
        features.push(
            (0..candidates.len())
                .map(|index| state[candidate_offset + index][feature])
                .sum::<f64>()
                / candidates.len().max(1) as f64,
        );
    }
    for feature in 0..GRAPH_MESSAGE_WIDTH {
        features.push(
            (0..candidates.len())
                .map(|index| state[candidate_offset + index][feature])
                .fold(0.0, f64::max),
        );
    }
    features
}

fn vector_knn_predict(training: &[(Vec<f64>, f64)], features: &[f64], neighbours: usize) -> f64 {
    let width = features.len();
    let mut mean = vec![0.0; width];
    for (sample, _) in training {
        for i in 0..width {
            mean[i] += sample[i];
        }
    }
    for value in &mut mean {
        *value /= training.len().max(1) as f64;
    }
    let mut scale = vec![0.0; width];
    for (sample, _) in training {
        for i in 0..width {
            scale[i] += (sample[i] - mean[i]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / training.len().max(1) as f64).sqrt().max(1e-9);
    }
    let mut distances: Vec<_> = training
        .iter()
        .map(|(sample, label)| {
            let distance = (0..width)
                .map(|i| ((features[i] - sample[i]) / scale[i]).powi(2))
                .sum::<f64>();
            (distance, *label)
        })
        .collect();
    distances.sort_by(|a, b| a.0.total_cmp(&b.0));
    distances
        .iter()
        .take(neighbours)
        .map(|item| item.1)
        .sum::<f64>()
        / neighbours.min(distances.len()).max(1) as f64
}

fn learn_vector_threshold(training: &[(Vec<f64>, f64)], neighbours: usize) -> (f64, f64, usize) {
    let mut out_of_fold = Vec::new();
    for held_out in 0..training.len() {
        let fold: Vec<_> = training
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != held_out)
            .map(|(_, sample)| sample.clone())
            .collect();
        out_of_fold.push((
            vector_knn_predict(&fold, &training[held_out].0, neighbours),
            training[held_out].1,
        ));
    }
    let mut predictions: Vec<_> = out_of_fold.iter().map(|sample| sample.0).collect();
    predictions.sort_by(f64::total_cmp);
    let mut thresholds = vec![f64::NEG_INFINITY];
    thresholds.extend(predictions.windows(2).map(|pair| (pair[0] + pair[1]) / 2.0));
    thresholds.push(f64::INFINITY);
    let mut best = (f64::NEG_INFINITY, 1.0, 0usize);
    for threshold in thresholds {
        let applied = out_of_fold
            .iter()
            .filter(|sample| sample.0 < threshold)
            .count();
        let ratio = out_of_fold
            .iter()
            .map(|sample| {
                if sample.0 < threshold {
                    sample.1.exp()
                } else {
                    1.0
                }
            })
            .sum::<f64>()
            / out_of_fold.len().max(1) as f64;
        if ratio < best.1 - 1e-12 || ((ratio - best.1).abs() <= 1e-12 && applied < best.2) {
            best = (threshold, ratio, applied);
        }
    }
    best
}

const FEATURE_COUNT: usize = 7;
const FORMULA_FEATURE_COUNT: usize = 6;

fn formula_features(
    vars: usize,
    clauses: &[Clause],
    baseline: &BddSolveResult,
) -> [f64; FORMULA_FEATURE_COUNT] {
    let interaction_count = baseline.interaction_candidates.len();
    let interaction_sum: usize = baseline
        .interaction_candidates
        .iter()
        .map(|(_, score)| *score)
        .sum();
    let interaction_max = baseline
        .interaction_candidates
        .iter()
        .map(|(_, score)| *score)
        .max()
        .unwrap_or(0);
    [
        1.0,
        (baseline.allocated_nodes as f64 + 1.0).ln(),
        baseline.live_nodes as f64 / baseline.allocated_nodes.max(1) as f64,
        recurring_pair_candidates(clauses).len() as f64 / vars.max(1) as f64,
        interaction_sum as f64 / interaction_count.max(1) as f64,
        (interaction_max as f64 + 1.0).ln(),
    ]
}

fn formula_knn_prediction(
    training: &[([f64; FORMULA_FEATURE_COUNT], f64)],
    features: &[f64; FORMULA_FEATURE_COUNT],
    neighbors: usize,
) -> f64 {
    let mut scale = [0.0; FORMULA_FEATURE_COUNT];
    let mut mean = [0.0; FORMULA_FEATURE_COUNT];
    for (sample, _) in training {
        for i in 0..FORMULA_FEATURE_COUNT {
            mean[i] += sample[i];
        }
    }
    for value in &mut mean {
        *value /= training.len().max(1) as f64;
    }
    for (sample, _) in training {
        for i in 0..FORMULA_FEATURE_COUNT {
            scale[i] += (sample[i] - mean[i]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / training.len().max(1) as f64).sqrt().max(1e-9);
    }
    let mut distances: Vec<_> = training
        .iter()
        .map(|(sample, label)| {
            let distance = (0..FORMULA_FEATURE_COUNT)
                .map(|i| ((features[i] - sample[i]) / scale[i]).powi(2))
                .sum::<f64>();
            (distance, *label)
        })
        .collect();
    distances.sort_by(|a, b| a.0.total_cmp(&b.0));
    distances
        .iter()
        .take(neighbors)
        .map(|(_, label)| label)
        .sum::<f64>()
        / neighbors.min(distances.len()).max(1) as f64
}

fn helper_features(
    vars: usize,
    clauses: &[Clause],
    pair: (Literal, Literal),
    frequency: usize,
    order: &[usize],
    interactions: &[((Literal, Literal), usize)],
) -> [f64; FEATURE_COUNT] {
    let graph = primal_graph(vars, clauses);
    let mut rank = vec![0usize; vars];
    for (position, &variable) in order.iter().enumerate() {
        rank[variable] = position;
    }
    let interaction: HashMap<_, _> = interactions.iter().copied().collect();
    let a = pair.0.0;
    let b = pair.1.0;
    let common = graph[a].intersection(&graph[b]).count();
    [
        1.0,
        (frequency as f64 + 1.0).ln(),
        rank[a].abs_diff(rank[b]) as f64 / vars.max(1) as f64,
        (graph[a].len() + graph[b].len()) as f64 / (2 * vars.max(1)) as f64,
        common as f64 / vars.max(1) as f64,
        (interaction.get(&pair).copied().unwrap_or(0) as f64 + 1.0).ln(),
        f64::from(pair.0.1 == pair.1.1),
    ]
}

fn fit_ridge(samples: &[([f64; FEATURE_COUNT], f64)], ridge: f64) -> [f64; FEATURE_COUNT] {
    let mut matrix = [[0.0; FEATURE_COUNT + 1]; FEATURE_COUNT];
    for (features, label) in samples {
        for row in 0..FEATURE_COUNT {
            for column in 0..FEATURE_COUNT {
                matrix[row][column] += features[row] * features[column];
            }
            matrix[row][FEATURE_COUNT] += features[row] * label;
        }
    }
    for (index, row) in matrix.iter_mut().enumerate() {
        row[index] += ridge;
    }
    for pivot in 0..FEATURE_COUNT {
        let best = (pivot..FEATURE_COUNT)
            .max_by(|&a, &b| matrix[a][pivot].abs().total_cmp(&matrix[b][pivot].abs()))
            .unwrap();
        matrix.swap(pivot, best);
        let divisor = matrix[pivot][pivot];
        if divisor.abs() < 1e-12 {
            continue;
        }
        for column in pivot..=FEATURE_COUNT {
            matrix[pivot][column] /= divisor;
        }
        for row in 0..FEATURE_COUNT {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..=FEATURE_COUNT {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
        }
    }
    std::array::from_fn(|index| matrix[index][FEATURE_COUNT])
}

fn predict(weights: &[f64; FEATURE_COUNT], features: &[f64; FEATURE_COUNT]) -> f64 {
    weights
        .iter()
        .zip(features)
        .map(|(weight, feature)| weight * feature)
        .sum()
}

fn learned_batch_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    weights: &[f64; FEATURE_COUNT],
    baseline: &BddSolveResult,
    acceptance_threshold: f64,
) -> (usize, Vec<Clause>, usize, usize) {
    let order = min_fill_order(vars, clauses);
    let mut candidates: Vec<_> = recurring_pair_candidates(clauses)
        .into_iter()
        .map(|(pair, frequency)| {
            let features = helper_features(
                vars,
                clauses,
                pair,
                frequency,
                &order,
                &baseline.interaction_candidates,
            );
            (predict(weights, &features), pair)
        })
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut current = clauses.to_vec();
    let mut current_vars = vars;
    let mut accepted = 0;
    let scored = candidates.len();
    for (predicted_delta, pair) in candidates {
        if accepted == max_helpers || predicted_delta >= acceptance_threshold {
            break;
        }
        if let Some(next) = add_pair_helper(current_vars, &current, pair) {
            current = next;
            current_vars += 1;
            accepted += 1;
        }
    }
    (current_vars, current, accepted, scored)
}

fn knn_batch_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    samples: &[([f64; FEATURE_COUNT], f64)],
    baseline: &BddSolveResult,
    neighbors: usize,
) -> (usize, Vec<Clause>, usize, usize) {
    let mut mean = [0.0; FEATURE_COUNT];
    for (features, _) in samples {
        for i in 0..FEATURE_COUNT {
            mean[i] += features[i];
        }
    }
    for value in &mut mean {
        *value /= samples.len().max(1) as f64;
    }
    let mut scale = [0.0; FEATURE_COUNT];
    for (features, _) in samples {
        for i in 0..FEATURE_COUNT {
            scale[i] += (features[i] - mean[i]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / samples.len().max(1) as f64).sqrt().max(1e-9);
    }
    let order = min_fill_order(vars, clauses);
    let mut candidates: Vec<_> = recurring_pair_candidates(clauses)
        .into_iter()
        .map(|(pair, frequency)| {
            let features = helper_features(
                vars,
                clauses,
                pair,
                frequency,
                &order,
                &baseline.interaction_candidates,
            );
            let mut distances: Vec<_> = samples
                .iter()
                .map(|(training, label)| {
                    let distance = (0..FEATURE_COUNT)
                        .map(|i| ((features[i] - training[i]) / scale[i]).powi(2))
                        .sum::<f64>();
                    (distance, *label)
                })
                .collect();
            distances.sort_by(|a, b| a.0.total_cmp(&b.0));
            let prediction = distances
                .iter()
                .take(neighbors)
                .map(|(_, label)| label)
                .sum::<f64>()
                / neighbors.min(distances.len()).max(1) as f64;
            (prediction, pair)
        })
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    let scored = candidates.len();
    let mut current = clauses.to_vec();
    let mut current_vars = vars;
    let mut accepted = 0;
    for (prediction, pair) in candidates {
        if accepted == max_helpers || prediction >= 0.0 {
            break;
        }
        if let Some(next) = add_pair_helper(current_vars, &current, pair) {
            current = next;
            current_vars += 1;
            accepted += 1;
        }
    }
    (current_vars, current, accepted, scored)
}

fn choose_order(name: &str, vars: usize, clauses: &[Clause], seed: u64) -> Vec<usize> {
    match name {
        "natural" => (0..vars).collect(),
        "random" => {
            let mut order: Vec<_> = (0..vars).collect();
            Rng(seed.max(1)).shuffle(&mut order);
            order
        }
        "min-degree" => min_degree_order(vars, clauses),
        "min-fill" => min_fill_order(vars, clauses),
        "flower-outside-in" => flower_outside_in_order(vars),
        _ => panic!(
            "unknown order: {name}; use natural, random, min-degree, min-fill, or flower-outside-in"
        ),
    }
}

fn recurring_pair_candidates(clauses: &[Clause]) -> Vec<((Literal, Literal), usize)> {
    let mut frequencies: HashMap<(Literal, Literal), usize> = HashMap::new();
    for clause in clauses {
        for i in 0..clause.0.len() {
            for j in i + 1..clause.0.len() {
                let pair = if clause.0[i] <= clause.0[j] {
                    (clause.0[i], clause.0[j])
                } else {
                    (clause.0[j], clause.0[i])
                };
                *frequencies.entry(pair).or_default() += 1;
            }
        }
    }
    let mut candidates: Vec<_> = frequencies
        .into_iter()
        .filter(|(_, frequency)| *frequency >= 2)
        .collect();
    candidates.sort_by_key(|&(pair, frequency)| (std::cmp::Reverse(frequency), pair));
    candidates
}

fn add_pair_helper(
    vars: usize,
    clauses: &[Clause],
    (a, b): (Literal, Literal),
) -> Option<Vec<Clause>> {
    add_pair_helper_with_minimum(vars, clauses, (a, b), 2)
}

fn add_pair_helper_with_minimum(
    vars: usize,
    clauses: &[Clause],
    (a, b): (Literal, Literal),
    minimum_occurrences: usize,
) -> Option<Vec<Clause>> {
    let occurrences = clauses
        .iter()
        .filter(|clause| clause.0.contains(&a) && clause.0.contains(&b))
        .count();
    if occurrences < minimum_occurrences {
        return None;
    }
    let mut rewritten = clauses.to_vec();
    for clause in &mut rewritten {
        if clause.0.contains(&a) && clause.0.contains(&b) {
            clause.0.retain(|literal| *literal != a && *literal != b);
            clause.0.push((vars, true));
        }
    }
    rewritten.push(Clause(vec![(vars, false), a, b]));
    rewritten.push(Clause(vec![(vars, true), (a.0, !a.1)]));
    rewritten.push(Clause(vec![(vars, true), (b.0, !b.1)]));
    Some(rewritten)
}

fn expand_recurring_pairs(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
) -> (usize, Vec<Clause>, usize) {
    let mut rewritten = clauses.to_vec();
    let mut helpers = 0;
    for (pair, _) in recurring_pair_candidates(clauses) {
        if helpers == max_helpers {
            break;
        }
        if let Some(next) = add_pair_helper(vars + helpers, &rewritten, pair) {
            rewritten = next;
            helpers += 1;
        }
    }
    (vars + helpers, rewritten, helpers)
}

#[derive(Debug)]
struct GreedyExpansion {
    vars: usize,
    clauses: Vec<Clause>,
    result: BddSolveResult,
    accepted: usize,
    candidates_tested: usize,
    beneficial_trials: usize,
    recursive_accepted: usize,
}

fn feedback_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    first_round_limit: usize,
    later_round_limit: usize,
    order_name: &str,
    seed: u64,
    feedback_enabled: bool,
) -> GreedyExpansion {
    let mut current_vars = vars;
    let mut current = clauses.to_vec();
    let mut result = eliminate_with_bdds(
        current_vars,
        &current,
        &choose_order(order_name, current_vars, &current, seed),
    );
    let mut literal_scores: HashMap<Literal, f64> = HashMap::new();
    let mut candidates_tested = 0;
    let mut beneficial_trials = 0;
    let mut accepted = 0;
    let mut recursive_accepted = 0;

    while accepted < max_helpers {
        let mut candidates = recurring_pair_candidates(&current);
        candidates.sort_by(
            |&(left_pair, left_frequency), &(right_pair, right_frequency)| {
                let score = |pair: (Literal, Literal), frequency: usize| {
                    frequency as f64
                        + literal_scores.get(&pair.0).copied().unwrap_or(0.0)
                        + literal_scores.get(&pair.1).copied().unwrap_or(0.0)
                        + if feedback_enabled && (pair.0.0 >= vars || pair.1.0 >= vars) {
                            0.5
                        } else {
                            0.0
                        }
                };
                score(right_pair, right_frequency)
                    .total_cmp(&score(left_pair, left_frequency))
                    .then_with(|| left_pair.cmp(&right_pair))
            },
        );
        let limit = if accepted == 0 {
            first_round_limit
        } else {
            later_round_limit
        };
        let mut best: Option<((Literal, Literal), Vec<Clause>, BddSolveResult)> = None;
        for (pair, _) in candidates.into_iter().take(limit) {
            let Some(candidate) = add_pair_helper(current_vars, &current, pair) else {
                continue;
            };
            candidates_tested += 1;
            let candidate_vars = current_vars + 1;
            let order = choose_order(order_name, candidate_vars, &candidate, seed);
            let candidate_result = eliminate_with_bdds(candidate_vars, &candidate, &order);
            if candidate_result.allocated_nodes < result.allocated_nodes {
                beneficial_trials += 1;
                if best.as_ref().is_none_or(|(_, _, best_result)| {
                    candidate_result.allocated_nodes < best_result.allocated_nodes
                }) {
                    best = Some((pair, candidate, candidate_result));
                }
            }
        }
        let Some((pair, next, next_result)) = best else {
            break;
        };
        if pair.0.0 >= vars || pair.1.0 >= vars {
            recursive_accepted += 1;
        }
        let gain = (result.allocated_nodes - next_result.allocated_nodes) as f64
            / result.allocated_nodes.max(1) as f64;
        if feedback_enabled {
            for literal in [pair.0, pair.1] {
                *literal_scores.entry(literal).or_default() += gain * 10.0;
                *literal_scores.entry((literal.0, !literal.1)).or_default() += gain * 2.0;
            }
            literal_scores.insert((current_vars, true), gain * 12.0);
        }
        current = next;
        current_vars += 1;
        result = next_result;
        accepted += 1;
    }
    GreedyExpansion {
        vars: current_vars,
        clauses: current,
        result,
        accepted,
        candidates_tested,
        beneficial_trials,
        recursive_accepted,
    }
}

fn bdd_frontier_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    candidate_limit: usize,
    order_name: &str,
    seed: u64,
) -> GreedyExpansion {
    let mut current_vars = vars;
    let mut current = clauses.to_vec();
    let mut result = eliminate_with_bdds(
        current_vars,
        &current,
        &choose_order(order_name, current_vars, &current, seed),
    );
    let mut candidates_tested = 0;
    let mut beneficial_trials = 0;
    let mut accepted = 0;
    let mut recursive_accepted = 0;

    while accepted < max_helpers {
        let mut best: Option<((Literal, Literal), Vec<Clause>, BddSolveResult)> = None;
        let mut applicable = 0;
        for &(pair, _) in &result.interaction_candidates {
            let Some(candidate) = add_pair_helper_with_minimum(current_vars, &current, pair, 1)
            else {
                continue;
            };
            applicable += 1;
            if applicable > candidate_limit {
                break;
            }
            candidates_tested += 1;
            let candidate_vars = current_vars + 1;
            let order = choose_order(order_name, candidate_vars, &candidate, seed);
            let candidate_result = eliminate_with_bdds(candidate_vars, &candidate, &order);
            if candidate_result.allocated_nodes < result.allocated_nodes {
                beneficial_trials += 1;
                if best.as_ref().is_none_or(|(_, _, best_result)| {
                    candidate_result.allocated_nodes < best_result.allocated_nodes
                }) {
                    best = Some((pair, candidate, candidate_result));
                }
            }
        }
        let Some((pair, next, next_result)) = best else {
            break;
        };
        if pair.0.0 >= vars || pair.1.0 >= vars {
            recursive_accepted += 1;
        }
        current = next;
        current_vars += 1;
        result = next_result;
        accepted += 1;
    }
    GreedyExpansion {
        vars: current_vars,
        clauses: current,
        result,
        accepted,
        candidates_tested,
        beneficial_trials,
        recursive_accepted,
    }
}

fn greedy_expand(
    vars: usize,
    clauses: &[Clause],
    max_helpers: usize,
    candidate_limit: usize,
    order_name: &str,
    seed: u64,
    bdd_aligned: bool,
) -> GreedyExpansion {
    let mut current_vars = vars;
    let mut current = clauses.to_vec();
    let mut result = solve_bdd_strategy(current_vars, &current, order_name, seed, bdd_aligned);
    let mut candidates_tested = 0;
    let mut beneficial_trials = 0;
    let mut accepted = 0;
    let mut recursive_accepted = 0;

    while accepted < max_helpers {
        let candidates = recurring_pair_candidates(&current);
        let mut best: Option<((Literal, Literal), Vec<Clause>, BddSolveResult)> = None;
        for (pair, _) in candidates.into_iter().take(candidate_limit) {
            let Some(candidate) = add_pair_helper(current_vars, &current, pair) else {
                continue;
            };
            candidates_tested += 1;
            let candidate_vars = current_vars + 1;
            let candidate_result =
                solve_bdd_strategy(candidate_vars, &candidate, order_name, seed, bdd_aligned);
            if candidate_result.allocated_nodes < result.allocated_nodes {
                beneficial_trials += 1;
                if best.as_ref().is_none_or(|(_, _, best_result)| {
                    candidate_result.allocated_nodes < best_result.allocated_nodes
                }) {
                    best = Some((pair, candidate, candidate_result));
                }
            }
        }
        let Some((pair, next, next_result)) = best else {
            break;
        };
        if pair.0.0 >= vars || pair.1.0 >= vars {
            recursive_accepted += 1;
        }
        current = next;
        current_vars += 1;
        result = next_result;
        accepted += 1;
    }
    GreedyExpansion {
        vars: current_vars,
        clauses: current,
        result,
        accepted,
        candidates_tested,
        beneficial_trials,
        recursive_accepted,
    }
}

fn solve_bdd_strategy(
    vars: usize,
    clauses: &[Clause],
    order_name: &str,
    seed: u64,
    bdd_aligned: bool,
) -> BddSolveResult {
    let elimination = choose_order(order_name, vars, clauses, seed);
    if bdd_aligned {
        eliminate_with_bdds_ordered(vars, clauses, &elimination, &elimination)
    } else {
        eliminate_with_bdds(vars, clauses, &elimination)
    }
}

fn generate_formula(family: &str, vars: usize, ratio: usize, seed: u64) -> Vec<Clause> {
    match family {
        "random" => random_3sat(vars, vars * ratio, seed),
        "random-planted" => planted_random_3sat(vars, vars * ratio, seed),
        "banded" => banded_3sat(vars, vars * ratio, seed, 5),
        "banded-planted" => planted_banded_3sat(vars, vars * ratio, seed, 5),
        "banded-3" => banded_3sat(vars, vars * ratio, seed, 3),
        "banded-9" => banded_3sat(vars, vars * ratio, seed, 9),
        "identity-expanded" => identity_expanded_sat(vars, ratio, seed),
        "flower" => flower_3sat(vars, vars * ratio, seed),
        "stacked-flower" => stacked_flower_3sat(vars, vars * ratio, seed),
        "flower-planted" => planted_flower_3sat(vars, vars * ratio, seed),
        "flower-symmetric" => symmetric_flower_3sat(vars, vars * ratio),
        "stacked-flower-planted" => planted_stacked_flower_3sat(vars, vars * ratio, seed),
        _ => panic!("unknown family: {family}"),
    }
}

fn parse_dimacs(path: &Path) -> Result<(usize, Vec<Clause>), String> {
    let body = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut declared_vars = None;
    let mut declared_clauses = None;
    let mut literals = Vec::new();
    let mut clauses = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('%') {
            break;
        }
        if line.is_empty() || line.starts_with('c') {
            continue;
        }
        if line.starts_with('p') {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 4 || fields[1] != "cnf" {
                return Err(format!("invalid DIMACS header in {}", path.display()));
            }
            declared_vars = fields[2].parse::<usize>().ok();
            declared_clauses = fields[3].parse::<usize>().ok();
            continue;
        }
        for token in line.split_whitespace() {
            let literal = token
                .parse::<isize>()
                .map_err(|_| format!("invalid literal in {}", path.display()))?;
            if literal == 0 {
                clauses.push(Clause(std::mem::take(&mut literals)));
            } else {
                literals.push((literal.unsigned_abs() - 1, literal > 0));
            }
        }
    }
    let vars = declared_vars.ok_or_else(|| format!("missing header in {}", path.display()))?;
    if !literals.is_empty() || clauses.len() != declared_clauses.unwrap_or(clauses.len()) {
        return Err(format!("clause count mismatch in {}", path.display()));
    }
    if clauses
        .iter()
        .flat_map(|clause| clause.0.iter())
        .any(|&(variable, _)| variable >= vars)
    {
        return Err(format!("variable out of range in {}", path.display()));
    }
    Ok((vars, clauses))
}

struct CompiledArtifact {
    original_vars: usize,
    original_clauses: Vec<Clause>,
    core_vars: usize,
    core_to_original: Vec<usize>,
    core_clauses: Vec<Clause>,
    seeds: Vec<BddSeededBranch>,
    supports_reopening: bool,
}

fn compile_safe_artifact(
    vars: usize,
    clauses: &[Clause],
    branch_cap: usize,
    node_limit: usize,
    time_limit_ms: u64,
) -> CompiledArtifact {
    compile_safe_artifact_with_gate(vars, clauses, branch_cap, node_limit, time_limit_ms, "all")
}

fn solver_gate_accepts(gate: &str, seed: &BddSeededBranch, local: &[Clause]) -> bool {
    if gate == "all" {
        return true;
    }
    if gate == "none" {
        return false;
    }
    let local_literals: usize = local.iter().map(|clause| clause.0.len()).sum();
    let summary_literals: usize = seed.summary.iter().map(|clause| clause.0.len()).sum();
    let local_binary = local.iter().filter(|clause| clause.0.len() == 2).count();
    let summary_binary = seed
        .summary
        .iter()
        .filter(|clause| clause.0.len() == 2)
        .count();
    let preserves_binary_fraction = seed.summary.is_empty()
        || summary_binary * local.len().max(1) >= local_binary * seed.summary.len();
    let compresses = seed.summary.len() < local.len() && summary_literals < local_literals;
    match gate {
        "balanced" => {
            seed.interior.len() >= 4
                && compresses
                && preserves_binary_fraction
                && seed.live_nodes <= seed.interior.len() * 6 + 8
        }
        "strict" => {
            seed.interior.len() >= 8
                && seed.summary.len() * 3 <= local.len()
                && summary_literals * 3 <= local_literals
                && preserves_binary_fraction
                && seed.live_nodes <= seed.interior.len() * 3 + 4
        }
        _ => false,
    }
}

fn compile_safe_artifact_with_gate(
    vars: usize,
    clauses: &[Clause],
    branch_cap: usize,
    node_limit: usize,
    time_limit_ms: u64,
    gate: &str,
) -> CompiledArtifact {
    let candidates = fast_detachable_branch_candidates(vars, clauses, branch_cap);
    let incidence = clause_incidence(vars, clauses);
    let topology_graph = (gate == "topology").then(|| compact_primal_graph(vars, clauses));
    let mut seeds = Vec::new();
    let only_balanced = gate
        .strip_prefix("balanced-only-")
        .and_then(|value| value.parse::<usize>().ok());
    let balanced_prefix = gate
        .strip_prefix("balanced-prefix-")
        .and_then(|value| value.parse::<usize>().ok());
    let mut balanced_ordinal = 0usize;
    for (interior, boundary) in candidates {
        let attempt = try_indexed_seed_bdd_candidate(
            vars,
            clauses,
            &incidence,
            interior,
            boundary,
            node_limit,
            std::time::Duration::from_millis(time_limit_ms),
        );
        if let Some(seed) = attempt.seed {
            let local = indexed_local_clauses(&seed.interior, &incidence, clauses);
            let accepted = if gate == "topology" {
                let graph = topology_graph.as_ref().expect("topology graph");
                let interior_degree_sum: usize =
                    seed.interior.iter().map(|&v| graph[v].len()).sum();
                let boundary_degree_sum: usize =
                    seed.boundary.iter().map(|&v| graph[v].len()).sum();
                solver_gate_accepts("balanced", &seed, &local)
                    && (seed.interior.len() >= 28
                        || (interior_degree_sum >= 10 && boundary_degree_sum <= 9))
            } else if only_balanced.is_some() || balanced_prefix.is_some() {
                let eligible = solver_gate_accepts("balanced", &seed, &local);
                if eligible {
                    let ordinal = balanced_ordinal;
                    balanced_ordinal += 1;
                    only_balanced.is_some_and(|target| ordinal == target)
                        || balanced_prefix.is_some_and(|limit| ordinal < limit)
                } else {
                    false
                }
            } else {
                solver_gate_accepts(gate, &seed, &local)
            };
            if accepted {
                seeds.push(seed);
            }
        }
    }
    let all_interior: BTreeSet<_> = seeds
        .iter()
        .flat_map(|seed| seed.interior.iter().copied())
        .collect();
    let mut core_clauses: Vec<_> = clauses
        .iter()
        .filter(|clause| {
            !clause
                .0
                .iter()
                .any(|(variable, _)| all_interior.contains(variable))
        })
        .cloned()
        .collect();
    for seed in &seeds {
        core_clauses.extend(seed.summary.iter().cloned());
    }
    let core_to_original: Vec<_> = (0..vars)
        .filter(|variable| !all_interior.contains(variable))
        .collect();
    let mut original_to_core = vec![usize::MAX; vars];
    for (core, &original) in core_to_original.iter().enumerate() {
        original_to_core[original] = core;
    }
    for clause in &mut core_clauses {
        for (variable, _) in &mut clause.0 {
            *variable = original_to_core[*variable];
        }
    }
    CompiledArtifact {
        original_vars: vars,
        original_clauses: clauses.to_vec(),
        core_vars: core_to_original.len(),
        core_to_original,
        core_clauses,
        seeds,
        supports_reopening: true,
    }
}

fn artifact_push_clauses(output: &mut String, clauses: &[Clause]) {
    output.push_str(&format!("{} ", clauses.len()));
    for clause in clauses {
        output.push_str(&format!("{} ", clause.0.len()));
        for &(variable, positive) in &clause.0 {
            output.push_str(&format!("{} {} ", variable, usize::from(positive)));
        }
    }
}

fn artifact_push_vec(output: &mut String, values: &[usize]) {
    output.push_str(&format!("{} ", values.len()));
    for value in values {
        output.push_str(&format!("{} ", value));
    }
}

fn save_compiled_artifact(path: &Path, artifact: &CompiledArtifact) -> Result<(), String> {
    let mut output = format!("LSAT2 {} ", artifact.original_vars);
    artifact_push_clauses(&mut output, &artifact.original_clauses);
    output.push_str(&format!("{} ", artifact.core_vars));
    artifact_push_vec(&mut output, &artifact.core_to_original);
    artifact_push_clauses(&mut output, &artifact.core_clauses);
    output.push_str(&format!("{} ", artifact.seeds.len()));
    for seed in &artifact.seeds {
        artifact_push_vec(&mut output, &seed.interior);
        artifact_push_vec(&mut output, &seed.boundary);
        artifact_push_vec(&mut output, &seed.order);
        output.push_str(&format!("{} {} ", seed.root, seed.manager.nodes.len()));
        for node in &seed.manager.nodes {
            output.push_str(&format!("{} {} {} ", node.variable, node.low, node.high));
        }
        artifact_push_clauses(&mut output, &seed.summary);
    }
    fs::write(path, output).map_err(|error| format!("write {}: {error}", path.display()))
}

fn artifact_next<T: std::str::FromStr>(
    tokens: &mut std::str::SplitWhitespace<'_>,
) -> Result<T, String> {
    tokens
        .next()
        .ok_or_else(|| "truncated compiled artifact".to_string())?
        .parse()
        .map_err(|_| "invalid compiled artifact token".to_string())
}

fn artifact_read_vec(tokens: &mut std::str::SplitWhitespace<'_>) -> Result<Vec<usize>, String> {
    let length: usize = artifact_next(tokens)?;
    (0..length).map(|_| artifact_next(tokens)).collect()
}

fn artifact_read_clauses(
    tokens: &mut std::str::SplitWhitespace<'_>,
) -> Result<Vec<Clause>, String> {
    let count: usize = artifact_next(tokens)?;
    (0..count)
        .map(|_| {
            let length: usize = artifact_next(tokens)?;
            let literals = (0..length)
                .map(|_| Ok((artifact_next(tokens)?, artifact_next::<usize>(tokens)? != 0)))
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Clause(literals))
        })
        .collect()
}

fn load_compiled_artifact(path: &Path) -> Result<CompiledArtifact, String> {
    let body =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut tokens = body.split_whitespace();
    let version = tokens
        .next()
        .ok_or_else(|| "empty compiled artifact".to_string())?;
    if version != "LSAT1" && version != "LSAT2" {
        return Err("unsupported compiled artifact version".to_string());
    }
    let original_vars = artifact_next(&mut tokens)?;
    let original_clauses = artifact_read_clauses(&mut tokens)?;
    let core_vars = artifact_next(&mut tokens)?;
    let core_to_original = artifact_read_vec(&mut tokens)?;
    let core_clauses = artifact_read_clauses(&mut tokens)?;
    let seed_count: usize = artifact_next(&mut tokens)?;
    let mut seeds = Vec::new();
    for _ in 0..seed_count {
        let interior = artifact_read_vec(&mut tokens)?;
        let boundary = artifact_read_vec(&mut tokens)?;
        let order = artifact_read_vec(&mut tokens)?;
        let root = artifact_next(&mut tokens)?;
        let node_count: usize = artifact_next(&mut tokens)?;
        let mut manager = BddManager::default();
        for _ in 0..node_count {
            let node = BddNode {
                variable: artifact_next(&mut tokens)?,
                low: artifact_next(&mut tokens)?,
                high: artifact_next(&mut tokens)?,
            };
            let id = manager.nodes.len() + 2;
            manager.nodes.push(node);
            manager.node_hits.push(0);
            manager.unique.insert(node, id);
        }
        let summary = if version == "LSAT2" {
            artifact_read_clauses(&mut tokens)?
        } else {
            Vec::new()
        };
        seeds.push(BddSeededBranch {
            vars: 0,
            clauses: Vec::new(),
            core_to_original: Vec::new(),
            boundary,
            interior,
            manager,
            root,
            order,
            local_clauses: 0,
            summary_clauses: 0,
            summary,
            allocated_nodes: node_count,
            live_nodes: node_count,
            cache_root: root,
        });
    }
    if tokens.next().is_some() || core_to_original.len() != core_vars {
        return Err("inconsistent compiled artifact".to_string());
    }
    Ok(CompiledArtifact {
        original_vars,
        original_clauses,
        core_vars,
        core_to_original,
        core_clauses,
        seeds,
        supports_reopening: version == "LSAT2",
    })
}

fn query_compiled_artifact(
    artifact: &CompiledArtifact,
    assumptions: &[(usize, bool)],
) -> Result<Option<Vec<bool>>, String> {
    for &(original, _) in assumptions {
        if original >= artifact.original_vars {
            return Err(format!(
                "assumption variable {} is out of range",
                original + 1
            ));
        }
    }
    let mut original_to_core = vec![usize::MAX; artifact.original_vars];
    for (core, &original) in artifact.core_to_original.iter().enumerate() {
        original_to_core[original] = core;
    }
    let reopened: BTreeSet<_> = assumptions
        .iter()
        .filter(|(variable, _)| original_to_core[*variable] == usize::MAX)
        .flat_map(|(variable, _)| {
            artifact
                .seeds
                .iter()
                .enumerate()
                .filter(move |(_, seed)| seed.interior.contains(variable))
                .map(|(index, _)| index)
        })
        .collect();
    if !reopened.is_empty() {
        if !artifact.supports_reopening {
            return Err("this LSAT1 artifact lacks persisted summaries; recompile it to query compiled-away variables".to_string());
        }
        let closed_interior: BTreeSet<_> = artifact
            .seeds
            .iter()
            .enumerate()
            .filter(|(index, _)| !reopened.contains(index))
            .flat_map(|(_, seed)| seed.interior.iter().copied())
            .collect();
        let mut query_clauses: Vec<_> = artifact
            .original_clauses
            .iter()
            .filter(|clause| {
                !clause
                    .0
                    .iter()
                    .any(|(variable, _)| closed_interior.contains(variable))
            })
            .cloned()
            .collect();
        for (index, seed) in artifact.seeds.iter().enumerate() {
            if !reopened.contains(&index) {
                query_clauses.extend(seed.summary.iter().cloned());
            }
        }
        let mut solver = Solver::new();
        add_to_varisat(&mut solver, &query_clauses);
        solver.assume(
            &assumptions
                .iter()
                .map(|&(variable, value)| Lit::from_var(Var::from_index(variable), value))
                .collect::<Vec<_>>(),
        );
        if !solver
            .solve()
            .map_err(|error| format!("solver error: {error}"))?
        {
            return Ok(None);
        }
        let mut assignment = vec![false; artifact.original_vars];
        for literal in solver
            .model()
            .ok_or_else(|| "SAT solver returned no model".to_string())?
        {
            if literal.var().index() < artifact.original_vars {
                assignment[literal.var().index()] = literal.is_positive();
            }
        }
        for (index, seed) in artifact.seeds.iter().enumerate() {
            if reopened.contains(&index) {
                continue;
            }
            let values = regrow_bdd_seed(seed, &assignment)
                .ok_or_else(|| "compiled witness reconstruction failed".to_string())?;
            for (offset, &variable) in seed.interior.iter().enumerate() {
                assignment[variable] = values[offset];
            }
        }
        if !satisfies(&artifact.original_clauses, &assignment)
            || assumptions
                .iter()
                .any(|&(variable, value)| assignment[variable] != value)
        {
            return Err("selectively reopened artifact produced an invalid assignment".to_string());
        }
        return Ok(Some(assignment));
    }
    let mut solver = Solver::new();
    add_to_varisat(&mut solver, &artifact.core_clauses);
    let mut core_assumptions = Vec::new();
    for &(original, value) in assumptions {
        let core = original_to_core[original];
        debug_assert_ne!(core, usize::MAX);
        core_assumptions.push(Lit::from_var(Var::from_index(core), value));
    }
    solver.assume(&core_assumptions);
    if !solver
        .solve()
        .map_err(|error| format!("solver error: {error}"))?
    {
        return Ok(None);
    }
    let mut assignment = vec![false; artifact.original_vars];
    for literal in solver
        .model()
        .ok_or_else(|| "SAT solver returned no model".to_string())?
    {
        if literal.var().index() < artifact.core_vars {
            assignment[artifact.core_to_original[literal.var().index()]] = literal.is_positive();
        }
    }
    for seed in &artifact.seeds {
        let values = regrow_bdd_seed(seed, &assignment)
            .ok_or_else(|| "compiled witness reconstruction failed".to_string())?;
        for (index, &variable) in seed.interior.iter().enumerate() {
            assignment[variable] = values[index];
        }
    }
    if !satisfies(&artifact.original_clauses, &assignment)
        || assumptions
            .iter()
            .any(|&(variable, value)| assignment[variable] != value)
    {
        return Err("compiled artifact produced an invalid assignment".to_string());
    }
    Ok(Some(assignment))
}

fn find_dimacs_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "cnf" | "dimacs"))
        {
            output.push(path.to_path_buf());
        }
        return Ok(());
    }
    let entries =
        fs::read_dir(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", path.display()))?;
        find_dimacs_files(&entry.path(), output)?;
    }
    Ok(())
}

fn reopened_formula(artifact: &CompiledArtifact, reopened: usize) -> Vec<Clause> {
    let closed_interior: BTreeSet<_> = artifact
        .seeds
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != reopened)
        .flat_map(|(_, seed)| seed.interior.iter().copied())
        .collect();
    let mut clauses: Vec<_> = artifact
        .original_clauses
        .iter()
        .filter(|clause| {
            !clause
                .0
                .iter()
                .any(|(variable, _)| closed_interior.contains(variable))
        })
        .cloned()
        .collect();
    for (index, seed) in artifact.seeds.iter().enumerate() {
        if index != reopened {
            clauses.extend(seed.summary.iter().cloned());
        }
    }
    clauses
}

const CORPUS_HEADER: &str = "path,vars,clauses,candidates,seeds,rejected,removed,removed_fraction,compile_ns,artifact_bytes,bdd_nodes,baseline_setup_ns,compiled_setup_ns,queries,direct_queries,reopened_queries,baseline_query_ns,direct_query_ns,reopened_query_ns,compiled_query_ns,query_ratio,amortized_ratio,eligible,all_agree,witnesses_valid,status";
const PROFILE_HEADER: &str = "path,vars,clauses,parse_ns,graph_ns,enumeration_ns,candidates,compile_ns,accepted,rejected,removed,removed_fraction,bdd_nodes,last_stage,status";

struct CompileProfile {
    path: String,
    vars: usize,
    clauses: usize,
    parse_ns: u128,
    graph_ns: u128,
    enumeration_ns: u128,
    candidates: usize,
    compile_ns: u128,
    accepted: usize,
    removed: usize,
    bdd_nodes: usize,
    last_stage: &'static str,
}

fn write_profile_checkpoint(
    path: &Path,
    profile: &CompileProfile,
    status: &str,
) -> Result<(), String> {
    let escaped = profile.path.replace(',', "%2C");
    let row = format!(
        "{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{}\n",
        escaped,
        profile.vars,
        profile.clauses,
        profile.parse_ns,
        profile.graph_ns,
        profile.enumeration_ns,
        profile.candidates,
        profile.compile_ns,
        profile.accepted,
        profile.candidates.saturating_sub(profile.accepted),
        profile.removed,
        profile.removed as f64 / profile.vars.max(1) as f64,
        profile.bdd_nodes,
        profile.last_stage,
        status
    );
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, row)
        .map_err(|error| format!("write profile checkpoint {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("replace profile checkpoint {}: {error}", path.display()))
}

fn profile_single_formula(input: &Path, checkpoint: &Path) -> Result<(), String> {
    let mut profile = CompileProfile {
        path: input.to_string_lossy().to_string(),
        vars: 0,
        clauses: 0,
        parse_ns: 0,
        graph_ns: 0,
        enumeration_ns: 0,
        candidates: 0,
        compile_ns: 0,
        accepted: 0,
        removed: 0,
        bdd_nodes: 0,
        last_stage: "start",
    };
    write_profile_checkpoint(checkpoint, &profile, "running")?;
    let start = Instant::now();
    let (vars, clauses) = parse_dimacs(input)?;
    profile.parse_ns = start.elapsed().as_nanos();
    profile.vars = vars;
    profile.clauses = clauses.len();
    profile.last_stage = "parse";
    write_profile_checkpoint(checkpoint, &profile, "running")?;
    let start = Instant::now();
    let graph = compact_primal_graph(vars, &clauses);
    profile.graph_ns = start.elapsed().as_nanos();
    profile.last_stage = "graph";
    write_profile_checkpoint(checkpoint, &profile, "running")?;
    let start = Instant::now();
    let candidates = global_small_separator_candidates(&graph, 64);
    profile.enumeration_ns = start.elapsed().as_nanos();
    profile.candidates = candidates.len();
    profile.last_stage = "enumeration";
    write_profile_checkpoint(checkpoint, &profile, "running")?;
    let incidence = clause_incidence(vars, &clauses);
    let compile_start = Instant::now();
    for (interior, boundary) in candidates {
        let attempt = try_indexed_seed_bdd_candidate(
            vars,
            &clauses,
            &incidence,
            interior,
            boundary,
            100_000,
            std::time::Duration::from_millis(100),
        );
        if let Some(seed) = attempt.seed {
            profile.accepted += 1;
            profile.removed += seed.interior.len();
            profile.bdd_nodes += seed.manager.nodes.len();
        }
        profile.compile_ns = compile_start.elapsed().as_nanos();
        profile.last_stage = "compile";
        write_profile_checkpoint(checkpoint, &profile, "running")?;
    }
    profile.compile_ns = compile_start.elapsed().as_nanos();
    profile.last_stage = "complete";
    write_profile_checkpoint(checkpoint, &profile, "ok")
}

fn profile_corpus_isolated(
    root: &Path,
    output_path: &Path,
    timeout_seconds: u64,
) -> Result<(), String> {
    let mut paths = Vec::new();
    find_dimacs_files(root, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!("no DIMACS files under {}", root.display()));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let mut completed = BTreeSet::new();
    if output_path.exists() {
        for line in fs::read_to_string(output_path)
            .map_err(|error| format!("read {}: {error}", output_path.display()))?
            .lines()
            .skip(1)
        {
            if let Some(path) = line.split(',').next() {
                completed.insert(path.replace("%2C", ","));
            }
        }
    } else {
        fs::write(output_path, format!("{PROFILE_HEADER}\n"))
            .map_err(|error| format!("write {}: {error}", output_path.display()))?;
    }
    let executable = env::current_exe().map_err(|error| format!("locate executable: {error}"))?;
    for (index, path) in paths.iter().enumerate() {
        if completed.contains(&path.to_string_lossy().to_string()) {
            continue;
        }
        let checkpoint = std::env::temp_dir().join(format!(
            "layered-sat-profile-{}-{index}.csv",
            std::process::id()
        ));
        let mut child = Command::new(&executable)
            .arg("profile-single")
            .arg(path)
            .arg(&checkpoint)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("spawn profile {}: {error}", path.display()))?;
        let start = Instant::now();
        let mut final_status = "child-error";
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("wait profile: {error}"))?
            {
                if status.success() {
                    final_status = "ok";
                }
                break;
            }
            if start.elapsed() >= std::time::Duration::from_secs(timeout_seconds) {
                child
                    .kill()
                    .map_err(|error| format!("kill profile: {error}"))?;
                child
                    .wait()
                    .map_err(|error| format!("reap profile: {error}"))?;
                final_status = "timeout";
                break;
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }
        let mut row = fs::read_to_string(&checkpoint).unwrap_or_else(|_| {
            format!(
                "{},0,0,0,0,0,0,0,0,0,0,0.000000,0,start,running\n",
                path.to_string_lossy().replace(',', "%2C")
            )
        });
        row = row.trim_end().to_string();
        if final_status != "ok" {
            if let Some(position) = row.rfind(',') {
                row.replace_range(position + 1.., final_status);
            }
        }
        let mut output = fs::OpenOptions::new()
            .append(true)
            .open(output_path)
            .map_err(|error| format!("append {}: {error}", output_path.display()))?;
        writeln!(output, "{row}").map_err(|error| format!("append profile: {error}"))?;
        output
            .flush()
            .map_err(|error| format!("flush profile: {error}"))?;
        let _ = fs::remove_file(&checkpoint);
        println!(
            "[{}/{}] {} status={}",
            index + 1,
            paths.len(),
            path.display(),
            final_status
        );
    }
    Ok(())
}

const QUERY_RACE_HEADER: &str =
    "path,mode,variable,value,setup_ns,solve_ns,result,valid,stage,status";

fn write_query_checkpoint(
    checkpoint: &Path,
    input: &Path,
    mode: &str,
    variable: usize,
    value: bool,
    setup_ns: u128,
    solve_ns: u128,
    result: &str,
    valid: bool,
    stage: &str,
    status: &str,
) -> Result<(), String> {
    let row = format!(
        "{},{},{},{},{},{},{},{},{},{}\n",
        input.to_string_lossy().replace(',', "%2C"),
        mode,
        variable + 1,
        value,
        setup_ns,
        solve_ns,
        result,
        valid,
        stage,
        status
    );
    let temporary = checkpoint.with_extension("tmp");
    fs::write(&temporary, row).map_err(|error| format!("write query checkpoint: {error}"))?;
    fs::rename(&temporary, checkpoint).map_err(|error| format!("replace query checkpoint: {error}"))
}

fn query_race_worker(
    mode: &str,
    input: &Path,
    variable: usize,
    value: bool,
    checkpoint: &Path,
) -> Result<(), String> {
    let setup_start = Instant::now();
    if mode == "baseline" {
        let (vars, clauses) = parse_dimacs(input)?;
        let mut solver = Solver::new();
        add_to_varisat(&mut solver, &clauses);
        let setup_ns = setup_start.elapsed().as_nanos();
        write_query_checkpoint(
            checkpoint, input, mode, variable, value, setup_ns, 0, "pending", false, "solve",
            "running",
        )?;
        solver.assume(&[Lit::from_var(Var::from_index(variable), value)]);
        let solve_start = Instant::now();
        let sat = solver
            .solve()
            .map_err(|error| format!("solver error: {error}"))?;
        let solve_ns = solve_start.elapsed().as_nanos();
        let valid = if sat {
            let mut assignment = vec![false; vars];
            for literal in solver
                .model()
                .ok_or_else(|| "SAT solver returned no model".to_string())?
            {
                if literal.var().index() < vars {
                    assignment[literal.var().index()] = literal.is_positive();
                }
            }
            satisfies(&clauses, &assignment) && assignment[variable] == value
        } else {
            true
        };
        write_query_checkpoint(
            checkpoint,
            input,
            mode,
            variable,
            value,
            setup_ns,
            solve_ns,
            if sat { "sat" } else { "unsat" },
            valid,
            "complete",
            "ok",
        )
    } else if mode == "compiled" {
        let artifact = load_compiled_artifact(input)?;
        let core_variable = artifact
            .core_to_original
            .iter()
            .position(|&original| original == variable);
        if let Some(core_variable) = core_variable {
            let mut solver = Solver::new();
            add_to_varisat(&mut solver, &artifact.core_clauses);
            let setup_ns = setup_start.elapsed().as_nanos();
            write_query_checkpoint(
                checkpoint,
                input,
                mode,
                variable,
                value,
                setup_ns,
                0,
                "pending",
                false,
                "solve-direct",
                "running",
            )?;
            solver.assume(&[Lit::from_var(Var::from_index(core_variable), value)]);
            let solve_start = Instant::now();
            let sat = solver
                .solve()
                .map_err(|error| format!("solver error: {error}"))?;
            let valid = if sat {
                let mut assignment = vec![false; artifact.original_vars];
                for literal in solver
                    .model()
                    .ok_or_else(|| "SAT solver returned no model".to_string())?
                {
                    if literal.var().index() < artifact.core_vars {
                        assignment[artifact.core_to_original[literal.var().index()]] =
                            literal.is_positive();
                    }
                }
                for seed in &artifact.seeds {
                    let values = regrow_bdd_seed(seed, &assignment)
                        .ok_or_else(|| "compiled witness reconstruction failed".to_string())?;
                    for (index, &interior) in seed.interior.iter().enumerate() {
                        assignment[interior] = values[index];
                    }
                }
                satisfies(&artifact.original_clauses, &assignment) && assignment[variable] == value
            } else {
                true
            };
            let solve_ns = solve_start.elapsed().as_nanos();
            write_query_checkpoint(
                checkpoint,
                input,
                mode,
                variable,
                value,
                setup_ns,
                solve_ns,
                if sat { "sat" } else { "unsat" },
                valid,
                "complete-direct",
                "ok",
            )
        } else {
            let setup_ns = setup_start.elapsed().as_nanos();
            write_query_checkpoint(
                checkpoint,
                input,
                mode,
                variable,
                value,
                setup_ns,
                0,
                "pending",
                false,
                "solve-reopened",
                "running",
            )?;
            let solve_start = Instant::now();
            let result = query_compiled_artifact(&artifact, &[(variable, value)])?;
            let solve_ns = solve_start.elapsed().as_nanos();
            write_query_checkpoint(
                checkpoint,
                input,
                mode,
                variable,
                value,
                setup_ns,
                solve_ns,
                if result.is_some() { "sat" } else { "unsat" },
                true,
                "complete-reopened",
                "ok",
            )
        }
    } else {
        Err(format!("unknown query race mode: {mode}"))
    }
}

fn run_isolated_query_worker(
    executable: &Path,
    mode: &str,
    input: &Path,
    variable: usize,
    value: bool,
    checkpoint: &Path,
    timeout: std::time::Duration,
) -> Result<String, String> {
    let mut child = Command::new(executable)
        .arg("query-race-worker")
        .arg(mode)
        .arg(input)
        .arg(variable.to_string())
        .arg(value.to_string())
        .arg(checkpoint)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("spawn query worker: {error}"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(exit) = child
            .try_wait()
            .map_err(|error| format!("wait query worker: {error}"))?
        {
            break if exit.success() { "ok" } else { "child-error" };
        }
        if start.elapsed() >= timeout {
            child
                .kill()
                .map_err(|error| format!("kill query worker: {error}"))?;
            child
                .wait()
                .map_err(|error| format!("reap query worker: {error}"))?;
            break "timeout";
        }
        thread::sleep(std::time::Duration::from_millis(25));
    };
    let mut row = fs::read_to_string(checkpoint).unwrap_or_else(|_| {
        format!(
            "{},{},{},{},0,0,pending,false,start,running\n",
            input.to_string_lossy().replace(',', "%2C"),
            mode,
            variable + 1,
            value
        )
    });
    row = row.trim_end().to_string();
    if status != "ok" {
        if let Some(position) = row.rfind(',') {
            row.replace_range(position + 1.., status);
        }
    }
    let _ = fs::remove_file(checkpoint);
    Ok(row)
}

fn benchmark_query_race(
    input: &Path,
    output: &Path,
    queries: usize,
    timeout: std::time::Duration,
    gate: &str,
) -> Result<(), String> {
    let (vars, clauses) = parse_dimacs(input)?;
    let binary_fraction = clauses.iter().filter(|clause| clause.0.len() == 2).count() as f64
        / clauses.len().max(1) as f64;
    let effective_gate = if gate == "learned" {
        if binary_fraction < 0.40 {
            "balanced"
        } else {
            "none"
        }
    } else {
        gate
    };
    let artifact_path = std::env::temp_dir().join(format!(
        "layered-sat-query-race-{}.lsat",
        std::process::id()
    ));
    let artifact =
        compile_safe_artifact_with_gate(vars, &clauses, 64, 100_000, 100, effective_gate);
    println!(
        "gate={} effective_gate={} binary_fraction={:.6} seeds={} removed={} removed_fraction={:.6} core_clauses={}",
        gate,
        effective_gate,
        binary_fraction,
        artifact.seeds.len(),
        vars - artifact.core_vars,
        (vars - artifact.core_vars) as f64 / vars.max(1) as f64,
        artifact.core_clauses.len()
    );
    save_compiled_artifact(&artifact_path, &artifact)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create results: {error}"))?;
    }
    fs::write(output, format!("{QUERY_RACE_HEADER}\n"))
        .map_err(|error| format!("write query race: {error}"))?;
    let executable = env::current_exe().map_err(|error| format!("locate executable: {error}"))?;
    for query in 0..queries {
        let variable = query.saturating_mul(vars) / queries.max(1);
        let value = query % 2 == 0;
        for (mode, source) in [("baseline", input), ("compiled", artifact_path.as_path())] {
            let checkpoint = std::env::temp_dir().join(format!(
                "layered-sat-query-{}-{query}-{mode}.csv",
                std::process::id()
            ));
            let mut row = run_isolated_query_worker(
                &executable,
                mode,
                source,
                variable,
                value,
                &checkpoint,
                timeout,
            )?;
            if mode == "compiled" {
                if let Some(comma) = row.find(',') {
                    row.replace_range(..comma, &input.to_string_lossy().replace(',', "%2C"));
                }
            }
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(output)
                .map_err(|error| format!("append query race: {error}"))?;
            writeln!(file, "{row}").map_err(|error| format!("append query race: {error}"))?;
            file.flush()
                .map_err(|error| format!("flush query race: {error}"))?;
            println!(
                "query={}/{} mode={} variable={} value={} status={}",
                query + 1,
                queries,
                mode,
                variable + 1,
                value,
                row.rsplit(',').next().unwrap_or("unknown")
            );
        }
    }
    let _ = fs::remove_file(&artifact_path);
    Ok(())
}

fn benchmark_query_portfolio(
    input: &Path,
    output: &Path,
    query_start: usize,
    queries: usize,
    query_total: usize,
    deadline: std::time::Duration,
    gates: &[String],
) -> Result<(), String> {
    let (vars, clauses) = parse_dimacs(input)?;
    let executable = env::current_exe().map_err(|error| format!("locate executable: {error}"))?;
    let mut artifacts = Vec::new();
    for (index, gate) in gates.iter().enumerate() {
        let artifact = compile_safe_artifact_with_gate(vars, &clauses, 64, 100_000, 100, gate);
        let path = std::env::temp_dir().join(format!(
            "layered-sat-portfolio-{}-{index}.lsat",
            std::process::id()
        ));
        save_compiled_artifact(&path, &artifact)?;
        println!(
            "portfolio_gate={} seeds={} removed={}",
            gate,
            artifact.seeds.len(),
            vars - artifact.core_vars
        );
        artifacts.push((gate.clone(), path));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create portfolio output: {error}"))?;
    }
    fs::write(
        output,
        "path,query,variable,value,winner,result,wall_ns,worker_wall_ns,workers,status\n",
    )
    .map_err(|error| format!("write portfolio output: {error}"))?;

    for query in 0..queries {
        let query_index = query_start.saturating_add(query);
        let variable = query_index.saturating_mul(vars) / query_total.max(1);
        let value = query_index % 2 == 0;
        let mut specs = vec![("baseline".to_string(), "baseline", input.to_path_buf())];
        specs.extend(
            artifacts
                .iter()
                .map(|(gate, path)| (gate.clone(), "compiled", path.clone())),
        );
        let portfolio_start = Instant::now();
        let mut workers = Vec::new();
        for (index, (label, mode, source)) in specs.into_iter().enumerate() {
            let checkpoint = std::env::temp_dir().join(format!(
                "layered-sat-portfolio-worker-{}-{query}-{index}.csv",
                std::process::id()
            ));
            let child = Command::new(&executable)
                .arg("query-race-worker")
                .arg(mode)
                .arg(&source)
                .arg(variable.to_string())
                .arg(value.to_string())
                .arg(&checkpoint)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("spawn portfolio worker: {error}"))?;
            workers.push((label, child, checkpoint));
        }
        let worker_count = workers.len();
        let mut winner = None;
        while portfolio_start.elapsed() < deadline && winner.is_none() {
            for (index, (_, child, checkpoint)) in workers.iter_mut().enumerate() {
                if let Some(status) = child
                    .try_wait()
                    .map_err(|error| format!("wait portfolio worker: {error}"))?
                {
                    if status.success() {
                        let row = fs::read_to_string(checkpoint).unwrap_or_default();
                        let fields: Vec<_> = row.trim().split(',').collect();
                        if fields.len() == 10 && fields[9] == "ok" && fields[7] == "true" {
                            winner = Some((index, fields[6].to_string()));
                            break;
                        }
                    }
                }
            }
            if winner.is_none() {
                thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        let wall_ns = portfolio_start.elapsed().as_nanos();
        let winner_label = winner
            .as_ref()
            .map(|(index, _)| workers[*index].0.clone())
            .unwrap_or_else(|| "none".to_string());
        let result = winner
            .as_ref()
            .map(|(_, result)| result.clone())
            .unwrap_or_else(|| "pending".to_string());
        for (_, child, checkpoint) in &mut workers {
            if child
                .try_wait()
                .map_err(|error| format!("poll portfolio worker: {error}"))?
                .is_none()
            {
                child
                    .kill()
                    .map_err(|error| format!("kill portfolio worker: {error}"))?;
                child
                    .wait()
                    .map_err(|error| format!("reap portfolio worker: {error}"))?;
            }
            let _ = fs::remove_file(checkpoint);
        }
        let worker_wall_ns = wall_ns.saturating_mul(worker_count as u128);
        let status = if winner.is_some() { "ok" } else { "timeout" };
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(output)
            .map_err(|error| format!("append portfolio output: {error}"))?;
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{}",
            input.to_string_lossy().replace(',', "%2C"),
            query_index + 1,
            variable + 1,
            value,
            winner_label,
            result,
            wall_ns,
            worker_wall_ns,
            worker_count,
            status
        )
        .map_err(|error| format!("append portfolio output: {error}"))?;
        file.flush()
            .map_err(|error| format!("flush portfolio output: {error}"))?;
        println!(
            "portfolio query={}/{} winner={} wall_ms={:.3} worker_wall_ms={:.3} status={}",
            query_index + 1,
            query_total,
            winner_label,
            wall_ns as f64 / 1e6,
            worker_wall_ns as f64 / 1e6,
            status
        );
    }
    for (_, path) in artifacts {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn portfolio_totals(path: &Path) -> Result<(usize, u128), String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut completed = 0usize;
    let mut worker_wall_ns = 0u128;
    for row in text.lines().skip(1) {
        let fields: Vec<_> = row.split(',').collect();
        if fields.len() != 10 {
            continue;
        }
        worker_wall_ns = worker_wall_ns.saturating_add(fields[7].parse::<u128>().unwrap_or(0));
        completed += usize::from(fields[9] == "ok");
    }
    Ok((completed, worker_wall_ns))
}

fn benchmark_width_strategy_search(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    output: &Path,
) -> Result<(), String> {
    if vars > 20 {
        return Err("width strategy search supports at most 20 variables".to_string());
    }
    let original = generate_formula(family, vars, ratio, formula_seed);
    let original_sat = solve_with_varisat(vars, &original).is_some();
    let original_order = min_fill_order(vars, &original);
    let (original_width, _) = elimination_cost(vars, &original, &original_order);
    let original_exact_width = exact_treewidth(vars, &original);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create strategy output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "family,formula_seed,strategy,math_identities,inverse_depth,probe_limit,branch_cap,max_seeds,seed_order,original_vars,final_vars,removed,seeds,probes,inverse_forced,original_width,final_width,width_change,original_exact_width,final_exact_width,exact_width_change,seed_live_nodes,seed_allocated_nodes,sat_equivalent,reconstruction_valid")
        .map_err(|error| format!("write strategy header: {error}"))?;
    let depths = [0usize, 1, 2, 3];
    let probes = [0usize, 1, 2, 4];
    let caps = [2usize, 4, 6, 8];
    let seed_limits = [0usize, 1, 2, 4];
    let orders = ["natural", "min-fill", "min-degree", "boundary-min-fill"];
    let mut strategy = 0usize;
    for math_identities in [false, true] {
        for &inverse_depth in &depths {
            for &probe_limit in &probes {
                for &branch_cap in &caps {
                    for &max_seeds in &seed_limits {
                        for &seed_order in &orders {
                            strategy += 1;
                            let preprocessed = if math_identities {
                                mathematical_identity_preprocess(&original).0
                            } else {
                                original.clone()
                            };
                            let shaken =
                                shake_formula(vars, &preprocessed, inverse_depth, probe_limit);
                            let mut current_vars = shaken.vars;
                            let mut current_clauses = shaken.clauses.clone();
                            let mut seeds = Vec::new();
                            for _ in 0..max_seeds {
                                let seed = seed_detachable_branch_bdd(
                                    current_vars,
                                    &current_clauses,
                                    branch_cap,
                                    seed_order,
                                );
                                if seed.interior.is_empty() {
                                    break;
                                }
                                current_vars = seed.vars;
                                current_clauses = seed.clauses.clone();
                                seeds.push(seed);
                            }
                            let final_width = if shaken.contradiction || current_vars == 0 {
                                0
                            } else {
                                let order = min_fill_order(current_vars, &current_clauses);
                                elimination_cost(current_vars, &current_clauses, &order).0
                            };
                            let final_exact_width = if shaken.contradiction || current_vars == 0 {
                                0
                            } else {
                                exact_treewidth(current_vars, &current_clauses)
                            };
                            let core_assignment = if shaken.contradiction {
                                None
                            } else {
                                solve_with_varisat(current_vars, &current_clauses)
                            };
                            let core_sat = core_assignment.is_some();
                            let sat_equivalent = core_sat == original_sat;
                            let reconstruction_valid = if let Some(core) = core_assignment {
                                regrow_seed_chain(&seeds, &core).is_some_and(|shaken_values| {
                                    let mut reconstructed = vec![false; vars];
                                    for (variable, value) in shaken.fixed.iter().enumerate() {
                                        if let Some(value) = value {
                                            reconstructed[variable] = *value;
                                        }
                                    }
                                    for (core_variable, &original_variable) in
                                        shaken.core_to_original.iter().enumerate()
                                    {
                                        reconstructed[original_variable] =
                                            shaken_values[core_variable];
                                    }
                                    satisfies(&original, &reconstructed)
                                })
                            } else {
                                !original_sat
                            };
                            let live_nodes: usize = seeds.iter().map(|seed| seed.live_nodes).sum();
                            let allocated_nodes: usize =
                                seeds.iter().map(|seed| seed.allocated_nodes).sum();
                            writeln!(file, "{family},{formula_seed},{strategy},{math_identities},{inverse_depth},{probe_limit},{branch_cap},{max_seeds},{seed_order},{vars},{current_vars},{},{},{},{},{original_width},{final_width},{},{original_exact_width},{final_exact_width},{},{live_nodes},{allocated_nodes},{sat_equivalent},{reconstruction_valid}", vars.saturating_sub(current_vars), seeds.len(), shaken.probes, shaken.inverse_forced, final_width as isize - original_width as isize, final_exact_width as isize - original_exact_width as isize)
                                .map_err(|error| format!("write strategy row: {error}"))?;
                        }
                    }
                }
            }
        }
    }
    file.flush()
        .map_err(|error| format!("flush strategy output: {error}"))?;
    println!(
        "width strategy search family={family} strategies={strategy} original_width={original_width} output={}",
        output.display()
    );
    Ok(())
}

fn benchmark_frozen_width_strategy(
    family: &str,
    vars: usize,
    ratio: usize,
    start_seed: u64,
    trials: usize,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create frozen output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "family,vars,ratio,seed,strategy,original_upper_width,original_structural_lower,final_upper_width,final_structural_lower,upper_change,certified_strict_reduction,removed,seeds,probes,inverse_forced,seed_live_nodes,seed_allocated_nodes,transform_ns,original_sat,final_sat,sat_equivalent,reconstruction_valid")
        .map_err(|error| format!("write frozen header: {error}"))?;
    for trial in 0..trials {
        let seed = start_seed.saturating_add(trial as u64);
        let original = generate_formula(family, vars, ratio, seed);
        let original_width = elimination_cost(vars, &original, &min_fill_order(vars, &original)).0;
        let original_lower = structural_treewidth_lower_bound(vars, &original);
        let original_sat = solve_with_varisat(vars, &original).is_some();
        let transform_start = Instant::now();
        let preprocessed = mathematical_identity_preprocess(&original).0;
        let shaken = shake_formula(vars, &preprocessed, 1, 4);
        let mut current_vars = shaken.vars;
        let mut current_clauses = shaken.clauses.clone();
        let mut seeds = Vec::new();
        for _ in 0..2 {
            let compiled = seed_detachable_branch_bdd(current_vars, &current_clauses, 8, "natural");
            if compiled.interior.is_empty() {
                break;
            }
            current_vars = compiled.vars;
            current_clauses = compiled.clauses.clone();
            seeds.push(compiled);
        }
        let transform_ns = transform_start.elapsed().as_nanos();
        let final_width = if shaken.contradiction || current_vars == 0 {
            0
        } else {
            elimination_cost(
                current_vars,
                &current_clauses,
                &min_fill_order(current_vars, &current_clauses),
            )
            .0
        };
        let final_lower = if shaken.contradiction || current_vars == 0 {
            0
        } else {
            structural_treewidth_lower_bound(current_vars, &current_clauses)
        };
        let certified = final_width < original_lower;
        let core_assignment = if shaken.contradiction {
            None
        } else {
            solve_with_varisat(current_vars, &current_clauses)
        };
        let final_sat = core_assignment.is_some();
        let reconstruction_valid = if let Some(core) = core_assignment {
            regrow_seed_chain(&seeds, &core).is_some_and(|shaken_values| {
                let mut reconstructed = vec![false; vars];
                for (variable, value) in shaken.fixed.iter().enumerate() {
                    if let Some(value) = value {
                        reconstructed[variable] = *value;
                    }
                }
                for (core_variable, &original_variable) in
                    shaken.core_to_original.iter().enumerate()
                {
                    reconstructed[original_variable] = shaken_values[core_variable];
                }
                satisfies(&original, &reconstructed)
            })
        } else {
            !original_sat
        };
        let live_nodes: usize = seeds.iter().map(|seed| seed.live_nodes).sum();
        let allocated_nodes: usize = seeds.iter().map(|seed| seed.allocated_nodes).sum();
        writeln!(file, "{family},{vars},{ratio},{seed},1529,{original_width},{original_lower},{final_width},{final_lower},{},{certified},{},{},{},{},{live_nodes},{allocated_nodes},{transform_ns},{original_sat},{final_sat},{},{reconstruction_valid}", final_width as isize - original_width as isize, vars.saturating_sub(current_vars), seeds.len(), shaken.probes, shaken.inverse_forced, original_sat == final_sat)
            .map_err(|error| format!("write frozen row: {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("flush frozen output: {error}"))?;
    println!(
        "frozen width strategy family={family} vars={vars} trials={trials} output={}",
        output.display()
    );
    Ok(())
}

fn ceil_log2(value: usize) -> usize {
    if value <= 1 {
        0
    } else {
        usize::BITS as usize - (value - 1).leading_zeros() as usize
    }
}

fn benchmark_frontier_width_strategies(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    random_orders: usize,
    output: &Path,
) -> Result<(), String> {
    if vars > 20 {
        return Err("frontier exact-width search supports at most 20 variables".to_string());
    }
    let original = generate_formula(family, vars, ratio, formula_seed);
    let original_width = exact_treewidth(vars, &original);
    let original_sat = solve_with_varisat(vars, &original).is_some();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create frontier output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "family,formula_seed,strategy,order,prefix,interior,boundary,original_width,core_width,boundary_charge,bdd_information_charge,charged_width,core_change,charged_change,summary_clauses,seed_live_nodes,seed_allocated_nodes,sat_equivalent,reconstruction_valid")
        .map_err(|error| format!("write frontier header: {error}"))?;
    let mut orders = vec![
        ("natural".to_string(), (0..vars).collect::<Vec<_>>()),
        ("min-fill".to_string(), min_fill_order(vars, &original)),
        ("min-degree".to_string(), min_degree_order(vars, &original)),
        ("flower".to_string(), flower_outside_in_order(vars)),
    ];
    for index in 0..random_orders {
        let mut order: Vec<_> = (0..vars).collect();
        Rng(formula_seed ^ (index as u64 + 1).wrapping_mul(0x9e37_79b9)).shuffle(&mut order);
        orders.push((format!("random-{index}"), order));
    }
    let graph = primal_graph(vars, &original);
    let mut strategy = 0usize;
    for (order_name, order) in orders {
        for prefix in 1..vars {
            strategy += 1;
            let mut interior = order[..prefix].to_vec();
            let interior_set: BTreeSet<_> = interior.iter().copied().collect();
            let boundary: Vec<_> = interior
                .iter()
                .flat_map(|&variable| graph[variable].iter().copied())
                .filter(|variable| !interior_set.contains(variable))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let mut manager = BddManager::default();
            let compiled = seed_bdd_candidate_in(
                vars,
                &original,
                &mut interior,
                boundary.clone(),
                "min-fill",
                &mut manager,
            );
            let core_width = exact_treewidth(compiled.vars, &compiled.clauses);
            let bdd_charge = ceil_log2(compiled.live_nodes.saturating_add(2));
            let charged_width = core_width.max(boundary.len()).max(bdd_charge);
            let core_assignment = solve_with_varisat(compiled.vars, &compiled.clauses);
            let core_sat = core_assignment.is_some();
            let reconstruction_valid = if let Some(core) = core_assignment {
                let mut mapped = vec![false; vars];
                for (core_variable, &original_variable) in
                    compiled.core_to_original.iter().enumerate()
                {
                    mapped[original_variable] = core[core_variable];
                }
                regrow_bdd_seed(&compiled, &mapped).is_some_and(|values| {
                    for (index, &variable) in compiled.interior.iter().enumerate() {
                        mapped[variable] = values[index];
                    }
                    satisfies(&original, &mapped)
                })
            } else {
                !original_sat
            };
            writeln!(file, "{family},{formula_seed},{strategy},{order_name},{prefix},{},{},{original_width},{core_width},{},{bdd_charge},{charged_width},{},{},{},{},{},{},{}", compiled.interior.len(), boundary.len(), boundary.len(), core_width as isize - original_width as isize, charged_width as isize - original_width as isize, compiled.summary_clauses, compiled.live_nodes, compiled.allocated_nodes, core_sat == original_sat, reconstruction_valid)
                .map_err(|error| format!("write frontier row: {error}"))?;
        }
    }
    file.flush()
        .map_err(|error| format!("flush frontier output: {error}"))?;
    println!(
        "frontier width search family={family} seed={formula_seed} strategies={strategy} original_width={original_width} output={}",
        output.display()
    );
    Ok(())
}

fn projected_bdd_network_cnf(
    vars: usize,
    clauses: &[Clause],
    interior: &[usize],
    boundary: &[usize],
) -> (usize, Vec<Clause>, usize) {
    let interior_set: BTreeSet<_> = interior.iter().copied().collect();
    let local: Vec<_> = clauses
        .iter()
        .filter(|clause| clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    let relevant: BTreeSet<_> = boundary.iter().chain(interior.iter()).copied().collect();
    let order = restricted_order(&local, &relevant, true);
    let rank: HashMap<_, _> = order
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, variable)| (variable, rank))
        .collect();
    let mut manager = BddManager::default();
    let mut relation = compile_formula_bdd_into(&mut manager, vars, &local, &order);
    for &variable in interior {
        relation = manager.exists(relation, rank[&variable], &mut HashMap::new());
    }
    let (manager, relation) = compact_bdd(&manager, relation);
    let core_to_original: Vec<_> = (0..vars)
        .filter(|variable| !interior_set.contains(variable))
        .collect();
    let mut original_to_core = vec![usize::MAX; vars];
    for (core, &original) in core_to_original.iter().enumerate() {
        original_to_core[original] = core;
    }
    let core_vars = core_to_original.len();
    let mut transformed: Vec<_> = clauses
        .iter()
        .filter(|clause| !clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .map(|clause| {
            Clause(
                clause
                    .0
                    .iter()
                    .map(|&(variable, sign)| (original_to_core[variable], sign))
                    .collect(),
            )
        })
        .collect();
    let helper = |node: usize| core_vars + node - 2;
    for node_id in 2..manager.nodes.len() + 2 {
        let node = manager.node(node_id);
        let decision_original = order[node.variable];
        let decision = original_to_core[decision_original];
        let output = helper(node_id);
        for decision_value in [false, true] {
            let child = if decision_value { node.high } else { node.low };
            if child < 2 {
                let child_value = child == 1;
                for output_value in [false, true] {
                    if output_value != child_value {
                        transformed.push(Clause(vec![
                            (output, !output_value),
                            (decision, !decision_value),
                        ]));
                    }
                }
            } else {
                for output_value in [false, true] {
                    for child_value in [false, true] {
                        if output_value != child_value {
                            transformed.push(Clause(vec![
                                (output, !output_value),
                                (decision, !decision_value),
                                (helper(child), !child_value),
                            ]));
                        }
                    }
                }
            }
        }
    }
    if relation == 0 {
        transformed.push(Clause(Vec::new()));
    } else if relation >= 2 {
        transformed.push(Clause(vec![(helper(relation), true)]));
    }
    (
        core_vars + manager.nodes.len(),
        transformed,
        manager.nodes.len(),
    )
}

fn direct_bdd_network_cnf(
    vars: usize,
    clauses: &[Clause],
    interior: &[usize],
    boundary: &[usize],
) -> (usize, Vec<Clause>, usize) {
    let interior_set: BTreeSet<_> = interior.iter().copied().collect();
    let local: Vec<_> = clauses
        .iter()
        .filter(|clause| clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    let relevant: BTreeSet<_> = boundary.iter().chain(interior.iter()).copied().collect();
    let order = restricted_order(&local, &relevant, true);
    let mut manager = BddManager::default();
    let root = compile_formula_bdd_into(&mut manager, vars, &local, &order);
    let (manager, root) = compact_bdd(&manager, root);
    let mut transformed: Vec<_> = clauses
        .iter()
        .filter(|clause| !clause.0.iter().any(|(v, _)| interior_set.contains(v)))
        .cloned()
        .collect();
    let helper = |node: usize| vars + node - 2;
    for node_id in 2..manager.nodes.len() + 2 {
        let node = manager.node(node_id);
        let decision = order[node.variable];
        let output = helper(node_id);
        for decision_value in [false, true] {
            let child = if decision_value { node.high } else { node.low };
            if child < 2 {
                let child_value = child == 1;
                for output_value in [false, true] {
                    if output_value != child_value {
                        transformed.push(Clause(vec![
                            (output, !output_value),
                            (decision, !decision_value),
                        ]));
                    }
                }
            } else {
                for output_value in [false, true] {
                    for child_value in [false, true] {
                        if output_value != child_value {
                            transformed.push(Clause(vec![
                                (output, !output_value),
                                (decision, !decision_value),
                                (helper(child), !child_value),
                            ]));
                        }
                    }
                }
            }
        }
    }
    if root == 0 {
        transformed.push(Clause(Vec::new()));
    } else if root >= 2 {
        transformed.push(Clause(vec![(helper(root), true)]));
    }
    (vars + manager.nodes.len(), transformed, manager.nodes.len())
}

fn benchmark_direct_bdd_network_expansion(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    random_orders: usize,
    output: &Path,
) -> Result<(), String> {
    if vars > 20 {
        return Err("direct BDD expansion requires exact original width (max 20 vars)".to_string());
    }
    let original = generate_formula(family, vars, ratio, formula_seed);
    let original_width = exact_treewidth(vars, &original);
    let original_sat = solve_with_varisat(vars, &original).is_some();
    let graph = primal_graph(vars, &original);
    let mut orders = vec![
        ("natural".to_string(), (0..vars).collect::<Vec<_>>()),
        ("min-fill".to_string(), min_fill_order(vars, &original)),
        ("min-degree".to_string(), min_degree_order(vars, &original)),
        ("flower".to_string(), flower_outside_in_order(vars)),
    ];
    for index in 0..random_orders {
        let mut order: Vec<_> = (0..vars).collect();
        Rng(formula_seed ^ (index as u64 + 1).wrapping_mul(0xd1b5_4a32)).shuffle(&mut order);
        orders.push((format!("random-{index}"), order));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create direct network output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create direct network output: {error}"))?;
    writeln!(file, "family,formula_seed,strategy,order,prefix,interior,boundary,original_width,expanded_vars,expanded_clauses,bdd_helpers,expanded_upper_width,certified_change,size_ratio,sat_equivalent,witness_valid")
        .map_err(|error| format!("write direct network header: {error}"))?;
    let mut strategy = 0usize;
    for (order_name, order) in orders {
        for prefix in 1..=vars {
            strategy += 1;
            let interior = order[..prefix].to_vec();
            let interior_set: BTreeSet<_> = interior.iter().copied().collect();
            let boundary: Vec<_> = interior
                .iter()
                .flat_map(|&variable| graph[variable].iter().copied())
                .filter(|variable| !interior_set.contains(variable))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let (expanded_vars, expanded, helpers) =
                direct_bdd_network_cnf(vars, &original, &interior, &boundary);
            let expanded_width = elimination_cost(
                expanded_vars,
                &expanded,
                &min_fill_order(expanded_vars, &expanded),
            )
            .0;
            let assignment = solve_with_varisat(expanded_vars, &expanded);
            let expanded_sat = assignment.is_some();
            let witness_valid = assignment
                .as_ref()
                .is_some_and(|values| satisfies(&original, &values[..vars]))
                || (!original_sat && assignment.is_none());
            writeln!(file, "{family},{formula_seed},{strategy},{order_name},{prefix},{},{},{original_width},{expanded_vars},{},{helpers},{expanded_width},{},{:.6},{},{}", interior.len(), boundary.len(), expanded.len(), expanded_width as isize - original_width as isize, expanded.len() as f64 / original.len().max(1) as f64, expanded_sat == original_sat, witness_valid)
                .map_err(|error| format!("write direct network row: {error}"))?;
        }
    }
    file.flush()
        .map_err(|error| format!("flush direct network output: {error}"))?;
    println!(
        "direct BDD network expansion family={family} seed={formula_seed} strategies={strategy} original_width={original_width} output={}",
        output.display()
    );
    Ok(())
}

fn benchmark_finite_domain_groupings(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    strategies: usize,
    output: &Path,
) -> Result<(), String> {
    if vars > 20 {
        return Err("finite-domain grouping exact search supports at most 20 Booleans".to_string());
    }
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let original_width = exact_treewidth(vars, &formula);
    let witness = solve_with_varisat(vars, &formula);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create grouping output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create grouping output: {error}"))?;
    writeln!(file, "family,formula_seed,strategy,order_kind,max_group,groups,max_domain_bits,original_width,unweighted_group_width,weighted_group_width,weighted_change,total_domain_bits,assignment_roundtrip,witness_valid")
        .map_err(|error| format!("write grouping header: {error}"))?;
    let structural_orders = [
        (0..vars).collect::<Vec<_>>(),
        min_fill_order(vars, &formula),
        min_degree_order(vars, &formula),
    ];
    for strategy in 0..strategies {
        let max_group = 2 + strategy % 3;
        let order_kind = (strategy / 3) % 4;
        let variant = strategy / 12;
        let mut order = if order_kind < 3 {
            structural_orders[order_kind].clone()
        } else {
            let mut order: Vec<_> = (0..vars).collect();
            Rng((variant as u64 + 1).wrapping_mul(0x9e37_79b9)).shuffle(&mut order);
            order
        };
        if order_kind < 3 {
            order.rotate_left(variant % vars.max(1));
            if variant & 1 == 1 {
                order.reverse();
            }
        }
        let mut rng = Rng(formula_seed ^ (strategy as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93));
        let mut groups = Vec::new();
        let mut cursor = 0usize;
        while cursor < vars {
            let remaining = vars - cursor;
            let size = (1 + rng.below(max_group)).min(remaining);
            groups.push(order[cursor..cursor + size].to_vec());
            cursor += size;
        }
        let weights: Vec<_> = groups.iter().map(Vec::len).collect();
        let graph = quotient_graph(vars, &formula, &groups);
        let unweighted = exact_weighted_treewidth(&vec![1; groups.len()], &graph);
        let weighted = exact_weighted_treewidth(&weights, &graph);
        let (assignment_roundtrip, witness_valid) = if let Some(assignment) = &witness {
            let encoded: Vec<usize> = groups
                .iter()
                .map(|members| {
                    members
                        .iter()
                        .enumerate()
                        .fold(0usize, |bits, (bit, &variable)| {
                            bits | ((assignment[variable] as usize) << bit)
                        })
                })
                .collect();
            let mut decoded = vec![false; vars];
            for (group, members) in groups.iter().enumerate() {
                for (bit, &variable) in members.iter().enumerate() {
                    decoded[variable] = encoded[group] & (1usize << bit) != 0;
                }
            }
            (decoded == *assignment, satisfies(&formula, &decoded))
        } else {
            (true, true)
        };
        writeln!(file, "{family},{formula_seed},{},{},{max_group},{},{},{original_width},{unweighted},{weighted},{},{},{assignment_roundtrip},{witness_valid}", strategy + 1, ["natural", "min-fill", "min-degree", "random"][order_kind], groups.len(), weights.iter().copied().max().unwrap_or(0), weighted as isize - original_width as isize, weights.iter().sum::<usize>())
            .map_err(|error| format!("write grouping row: {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("flush grouping output: {error}"))?;
    println!(
        "finite-domain grouping family={family} seed={formula_seed} strategies={strategies} original_width={original_width} output={}",
        output.display()
    );
    Ok(())
}

fn invert_binary_matrix(rows: &[u32], vars: usize) -> Option<Vec<u32>> {
    let mut augmented: Vec<u64> = rows
        .iter()
        .enumerate()
        .map(|(index, &row)| row as u64 | (1u64 << (vars + index)))
        .collect();
    for column in 0..vars {
        let pivot = (column..vars).find(|&row| augmented[row] & (1u64 << column) != 0)?;
        augmented.swap(column, pivot);
        for row in 0..vars {
            if row != column && augmented[row] & (1u64 << column) != 0 {
                augmented[row] ^= augmented[column];
            }
        }
    }
    Some(
        augmented
            .into_iter()
            .map(|row| (row >> vars) as u32)
            .collect(),
    )
}

fn apply_binary_matrix(rows: &[u32], values: u32) -> u32 {
    rows.iter().enumerate().fold(0u32, |result, (index, &row)| {
        result | (((row & values).count_ones() & 1) << index)
    })
}

fn benchmark_affine_basis_strategies(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    strategies: usize,
    output: &Path,
) -> Result<(), String> {
    if vars > 20 {
        return Err("affine exact-width search supports at most 20 variables".to_string());
    }
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let original_width = exact_treewidth(vars, &formula);
    let witness = solve_with_varisat(vars, &formula);
    let witness_bits = witness.as_ref().map(|assignment| {
        assignment
            .iter()
            .enumerate()
            .fold(0u32, |bits, (index, &value)| {
                bits | ((value as u32) << index)
            })
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create affine output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create affine output: {error}"))?;
    writeln!(file, "family,formula_seed,strategy,row_operations,offset_weight,matrix_ones,original_width,min_fill_upper,screened_width,exact_checked,width_change,max_factor_arity,sum_factor_log2_entries,assignment_roundtrip,witness_valid")
        .map_err(|error| format!("write affine header: {error}"))?;
    let mut exact_cache: HashMap<Vec<Vec<usize>>, usize> = HashMap::new();
    for strategy in 0..strategies {
        let operations = strategy % 33;
        let mut rng = Rng(formula_seed ^ (strategy as u64 + 1).wrapping_mul(0xa076_1d64_78bd_642f));
        let mut matrix: Vec<u32> = (0..vars).map(|index| 1u32 << index).collect();
        for _ in 0..operations {
            let target = rng.below(vars);
            let mut source = rng.below(vars - 1);
            if source >= target {
                source += 1;
            }
            matrix[target] ^= matrix[source];
        }
        let offset = rng.next() as u32 & ((1u32 << vars) - 1);
        let mut transformed = Vec::with_capacity(formula.len());
        let mut max_arity = 0usize;
        let mut table_log2_entries = 0usize;
        for clause in &formula {
            let scope_mask = clause
                .0
                .iter()
                .fold(0u32, |mask, &(variable, _)| mask | matrix[variable]);
            let scope: Vec<_> = (0..vars)
                .filter(|&variable| scope_mask & (1u32 << variable) != 0)
                .collect();
            max_arity = max_arity.max(scope.len());
            table_log2_entries = table_log2_entries.saturating_add(scope.len());
            transformed.push(Clause(
                scope.into_iter().map(|variable| (variable, true)).collect(),
            ));
        }
        let upper = elimination_cost(vars, &transformed, &min_fill_order(vars, &transformed)).0;
        let graph_key: Vec<Vec<usize>> = primal_graph(vars, &transformed)
            .into_iter()
            .map(|neighbors| neighbors.into_iter().collect())
            .collect();
        let exact = if upper <= original_width {
            if let Some(&cached) = exact_cache.get(&graph_key) {
                cached
            } else {
                let width = exact_treewidth(vars, &transformed);
                exact_cache.insert(graph_key, width);
                width
            }
        } else {
            upper
        };
        let exact_checked = upper <= original_width;
        let (roundtrip, witness_valid) = if let (Some(bits), Some(inverse)) =
            (witness_bits, invert_binary_matrix(&matrix, vars))
        {
            let coordinates = apply_binary_matrix(&inverse, bits ^ offset);
            let reconstructed = apply_binary_matrix(&matrix, coordinates) ^ offset;
            let assignment: Vec<_> = (0..vars)
                .map(|index| reconstructed & (1u32 << index) != 0)
                .collect();
            (reconstructed == bits, satisfies(&formula, &assignment))
        } else {
            (witness.is_none(), witness.is_none())
        };
        writeln!(file, "{family},{formula_seed},{},{operations},{},{},{original_width},{upper},{exact},{exact_checked},{},{max_arity},{table_log2_entries},{roundtrip},{witness_valid}", strategy + 1, offset.count_ones(), matrix.iter().map(|row| row.count_ones() as usize).sum::<usize>(), exact as isize - original_width as isize)
            .map_err(|error| format!("write affine row: {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("flush affine output: {error}"))?;
    println!(
        "affine basis search family={family} seed={formula_seed} strategies={strategies} original_width={original_width} exact_graphs={} output={}",
        exact_cache.len(),
        output.display()
    );
    Ok(())
}

fn tensor_flatten_rank(values: &[i64; 8], axis: usize) -> usize {
    let mut rows = [[0i64; 4]; 2];
    for bits in 0..8 {
        let coordinates = [bits & 1, (bits >> 1) & 1, (bits >> 2) & 1];
        let row = coordinates[axis];
        let mut column = 0usize;
        for other in 0..3 {
            if other != axis {
                column = (column << 1) | coordinates[other];
            }
        }
        rows[row][column] = values[bits];
    }
    if rows.iter().all(|row| row.iter().all(|&value| value == 0)) {
        return 0;
    }
    let dependent = (0..4).all(|left| {
        (0..4).all(|right| rows[0][left] * rows[1][right] == rows[0][right] * rows[1][left])
    });
    if dependent { 1 } else { 2 }
}

fn benchmark_holographic_tensor_strategies(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    strategies: usize,
    output: &Path,
) -> Result<(), String> {
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let width = if vars <= 20 {
        exact_treewidth(vars, &formula)
    } else {
        elimination_cost(vars, &formula, &min_fill_order(vars, &formula)).0
    };
    let bases: Vec<[[i64; 2]; 2]> = vec![
        [[1, 0], [0, 1]],
        [[0, 1], [1, 0]],
        [[1, 0], [0, -1]],
        [[-1, 0], [0, 1]],
        [[1, 1], [1, -1]],
        [[1, -1], [1, 1]],
        [[1, 1], [0, 1]],
        [[1, 0], [1, 1]],
        [[1, -1], [0, 1]],
        [[1, 0], [-1, 1]],
        [[0, 1], [-1, 0]],
        [[0, -1], [1, 0]],
    ];
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create tensor output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create tensor output: {error}"))?;
    writeln!(file, "family,formula_seed,strategy,bases_used,treewidth,bond_bits,total_nonzeros,max_clause_nonzeros,rank1_flattenings,rank2_flattenings,clauses_with_rank_drop,all_bases_invertible")
        .map_err(|error| format!("write tensor header: {error}"))?;
    for strategy in 0..strategies {
        let mut rng = Rng(formula_seed ^ (strategy as u64 + 1).wrapping_mul(0x94d0_49bb_1331_11eb));
        let selected: Vec<_> = (0..vars).map(|_| rng.below(bases.len())).collect();
        let mut total_nonzeros = 0usize;
        let mut max_nonzeros = 0usize;
        let mut rank1 = 0usize;
        let mut rank2 = 0usize;
        let mut rank_drop_clauses = 0usize;
        for clause in &formula {
            let mut tensor = [0i64; 8];
            for output_bits in 0..8 {
                let mut value = 0i64;
                for input_bits in 0..8 {
                    let satisfies_clause =
                        clause.0.iter().enumerate().any(|(position, &(_, sign))| {
                            (input_bits & (1usize << position) != 0) == sign
                        });
                    if !satisfies_clause {
                        continue;
                    }
                    let mut coefficient = 1i64;
                    for position in 0..3 {
                        let input = (input_bits >> position) & 1;
                        let output = (output_bits >> position) & 1;
                        coefficient *= bases[selected[clause.0[position].0]][input][output];
                    }
                    value += coefficient;
                }
                tensor[output_bits] = value;
            }
            let nonzeros = tensor.iter().filter(|&&value| value != 0).count();
            total_nonzeros += nonzeros;
            max_nonzeros = max_nonzeros.max(nonzeros);
            let ranks = [
                tensor_flatten_rank(&tensor, 0),
                tensor_flatten_rank(&tensor, 1),
                tensor_flatten_rank(&tensor, 2),
            ];
            rank1 += ranks.iter().filter(|&&rank| rank == 1).count();
            rank2 += ranks.iter().filter(|&&rank| rank == 2).count();
            rank_drop_clauses += usize::from(ranks.iter().any(|&rank| rank < 2));
        }
        let all_invertible = selected.iter().all(|&index| {
            let basis = bases[index];
            basis[0][0] * basis[1][1] - basis[0][1] * basis[1][0] != 0
        });
        let bases_used = selected.iter().copied().collect::<BTreeSet<_>>().len();
        writeln!(file, "{family},{formula_seed},{},{bases_used},{width},1,{total_nonzeros},{max_nonzeros},{rank1},{rank2},{rank_drop_clauses},{all_invertible}", strategy + 1)
            .map_err(|error| format!("write tensor row: {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("flush tensor output: {error}"))?;
    println!(
        "holographic tensor search family={family} seed={formula_seed} strategies={strategies} width={width} output={}",
        output.display()
    );
    Ok(())
}

fn benchmark_holographic_network_cost(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    strategy: usize,
    output: &Path,
) -> Result<(), String> {
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let bases: Vec<[[f64; 2]; 2]> = vec![
        [[1.0, 0.0], [0.0, 1.0]],
        [[0.0, 1.0], [1.0, 0.0]],
        [[1.0, 0.0], [0.0, -1.0]],
        [[-1.0, 0.0], [0.0, 1.0]],
        [[1.0, 1.0], [1.0, -1.0]],
        [[1.0, -1.0], [1.0, 1.0]],
        [[1.0, 1.0], [0.0, 1.0]],
        [[1.0, 0.0], [1.0, 1.0]],
        [[1.0, -1.0], [0.0, 1.0]],
        [[1.0, 0.0], [-1.0, 1.0]],
        [[0.0, 1.0], [-1.0, 0.0]],
        [[0.0, -1.0], [1.0, 0.0]],
    ];
    let mut rng = Rng(formula_seed ^ (strategy as u64).wrapping_mul(0x94d0_49bb_1331_11eb));
    let selected: Vec<_> = (0..vars).map(|_| rng.below(bases.len())).collect();
    let mut degrees = vec![0usize; vars];
    let mut clause_nonzeros = 0usize;
    for clause in &formula {
        let mut tensor = [0.0f64; 8];
        for &(variable, _) in &clause.0 {
            degrees[variable] += 1;
        }
        for output_bits in 0..8 {
            for input_bits in 0..8 {
                if !clause
                    .0
                    .iter()
                    .enumerate()
                    .any(|(position, &(_, sign))| (input_bits & (1usize << position) != 0) == sign)
                {
                    continue;
                }
                let mut coefficient = 1.0;
                for position in 0..3 {
                    let input = (input_bits >> position) & 1;
                    let output = (output_bits >> position) & 1;
                    coefficient *= bases[selected[clause.0[position].0]][input][output];
                }
                tensor[output_bits] += coefficient;
            }
        }
        clause_nonzeros += tensor.iter().filter(|&&value| value.abs() > 1e-12).count();
    }
    let mut equality_nonzeros = 0usize;
    let mut equality_dense_entries = 0usize;
    let mut max_equality_nonzeros = 0usize;
    for variable in 0..vars {
        let degree = degrees[variable];
        if degree >= usize::BITS as usize {
            return Err("equality tensor degree exceeds addressable table".to_string());
        }
        let basis = bases[selected[variable]];
        let determinant = basis[0][0] * basis[1][1] - basis[0][1] * basis[1][0];
        let inverse = [
            [basis[1][1] / determinant, -basis[0][1] / determinant],
            [-basis[1][0] / determinant, basis[0][0] / determinant],
        ];
        let entries = 1usize << degree;
        equality_dense_entries = equality_dense_entries.saturating_add(entries);
        let mut nonzeros = 0usize;
        for output_bits in 0..entries {
            let mut value = 0.0;
            for original_value in 0..2 {
                let mut product = 1.0;
                for edge in 0..degree {
                    let output = (output_bits >> edge) & 1;
                    product *= inverse[output][original_value];
                }
                value += product;
            }
            nonzeros += usize::from(value.abs() > 1e-12);
        }
        equality_nonzeros += nonzeros;
        max_equality_nonzeros = max_equality_nonzeros.max(nonzeros);
    }
    let baseline_clause_nonzeros = formula.len() * 7;
    let baseline_equality_nonzeros = vars * 2;
    let baseline_total = baseline_clause_nonzeros + baseline_equality_nonzeros;
    let transformed_total = clause_nonzeros + equality_nonzeros;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create network cost output: {error}"))?;
    }
    fs::write(
        output,
        format!("family,formula_seed,strategy,clauses,variables,baseline_clause_nonzeros,transformed_clause_nonzeros,baseline_equality_nonzeros,transformed_equality_nonzeros,equality_dense_entries,max_equality_nonzeros,baseline_total_nonzeros,transformed_total_nonzeros,total_ratio\n{family},{formula_seed},{strategy},{},{vars},{baseline_clause_nonzeros},{clause_nonzeros},{baseline_equality_nonzeros},{equality_nonzeros},{equality_dense_entries},{max_equality_nonzeros},{baseline_total},{transformed_total},{:.6}\n", formula.len(), transformed_total as f64 / baseline_total.max(1) as f64),
    )
    .map_err(|error| format!("write network cost output: {error}"))?;
    println!(
        "holographic network cost family={family} seed={formula_seed} strategy={strategy} clause_nonzeros={clause_nonzeros} equality_nonzeros={equality_nonzeros} total_ratio={:.6}",
        transformed_total as f64 / baseline_total.max(1) as f64
    );
    Ok(())
}

fn canonical_residual_after_choice(
    residual: &[Vec<Literal>],
    variable: usize,
    value: bool,
    work: &mut usize,
) -> Vec<Vec<Literal>> {
    let mut next = Vec::new();
    for clause in residual {
        *work = work.saturating_add(clause.len());
        if clause
            .iter()
            .any(|&(candidate, sign)| candidate == variable && sign == value)
        {
            continue;
        }
        let mut reduced: Vec<_> = clause
            .iter()
            .copied()
            .filter(|&(candidate, _)| candidate != variable)
            .collect();
        reduced.sort_unstable();
        reduced.dedup();
        if reduced.is_empty() {
            return vec![Vec::new()];
        }
        next.push(reduced);
    }
    next.sort_unstable();
    next.dedup();
    next
}

fn continuation_frontier_profile(vars: usize, formula: &[Clause], order: &[usize]) -> Vec<u128> {
    let mut position = vec![0usize; vars];
    for (index, &variable) in order.iter().enumerate() {
        position[variable] = index;
    }
    let mut profile = Vec::with_capacity(vars.saturating_add(1));
    profile.push(1);
    for cut in 1..vars {
        let mut crossing_clauses = 0usize;
        let mut past_boundary = BTreeSet::new();
        for clause in formula {
            let crosses = clause
                .0
                .iter()
                .any(|&(variable, _)| position[variable] < cut)
                && clause
                    .0
                    .iter()
                    .any(|&(variable, _)| position[variable] >= cut);
            if crosses {
                crossing_clauses += 1;
                past_boundary.extend(
                    clause.0.iter().filter_map(|&(variable, _)| {
                        (position[variable] < cut).then_some(variable)
                    }),
                );
            }
        }
        let bits = crossing_clauses.min(past_boundary.len());
        profile.push(
            1u128
                .checked_shl(bits.min(127) as u32)
                .unwrap_or(u128::MAX)
                .saturating_add(1),
        );
    }
    profile.push(2);
    profile
}

fn ceil_log2_u128(value: u128) -> usize {
    if value <= 1 {
        0
    } else {
        (u128::BITS - (value - 1).leading_zeros()) as usize
    }
}

fn continuation_frontier_bound_bits(vars: usize, formula: &[Clause], order: &[usize]) -> usize {
    let maximum_states = continuation_frontier_profile(vars, formula, order)
        .into_iter()
        .max()
        .unwrap_or(1);
    ceil_log2_u128(maximum_states)
}

struct CompiledContinuation {
    order: Vec<usize>,
    transitions: Vec<Vec<[usize; 2]>>,
    residual_layers: Vec<Vec<Vec<Vec<Literal>>>>,
    terminal_sat: Vec<bool>,
    peak_classes: usize,
}

struct ContinuationScratch {
    reachable: Vec<bool>,
    next: Vec<bool>,
    parents: Vec<Vec<Option<(usize, bool)>>>,
}

impl ContinuationScratch {
    fn new(compiled: &CompiledContinuation) -> Self {
        let parents = (0..compiled.order.len())
            .map(|layer_index| {
                let size = if layer_index + 1 < compiled.transitions.len() {
                    compiled.transitions[layer_index + 1].len()
                } else {
                    compiled.terminal_sat.len()
                };
                vec![None; size]
            })
            .collect();
        Self {
            reachable: vec![false; compiled.peak_classes],
            next: vec![false; compiled.peak_classes],
            parents,
        }
    }
}

fn compile_continuation(formula: &[Clause], order: &[usize]) -> CompiledContinuation {
    let mut base: Vec<Vec<Literal>> = formula
        .iter()
        .map(|clause| {
            let mut literals = clause.0.clone();
            literals.sort_unstable();
            literals.dedup();
            literals
        })
        .collect();
    base.sort_unstable();
    base.dedup();
    let mut current = vec![base];
    let mut transitions = Vec::with_capacity(order.len());
    let mut residual_layers = vec![current.clone()];
    let mut peak_classes = 1usize;
    let mut work = 0usize;
    for &variable in order {
        let mut next_ids = HashMap::new();
        let mut next_residuals = Vec::new();
        let mut layer = vec![[0usize; 2]; current.len()];
        for (state, residual) in current.iter().enumerate() {
            for value in [false, true] {
                let canonical =
                    canonical_residual_after_choice(residual, variable, value, &mut work);
                let next = if let Some(&existing) = next_ids.get(&canonical) {
                    existing
                } else {
                    let id = next_residuals.len();
                    next_ids.insert(canonical.clone(), id);
                    next_residuals.push(canonical);
                    id
                };
                layer[state][value as usize] = next;
            }
        }
        transitions.push(layer);
        current = next_residuals;
        residual_layers.push(current.clone());
        peak_classes = peak_classes.max(current.len());
    }
    let terminal_sat = current.iter().map(Vec::is_empty).collect();
    CompiledContinuation {
        order: order.to_vec(),
        transitions,
        residual_layers,
        terminal_sat,
        peak_classes,
    }
}

fn apply_clause_change_to_residual(
    old: &[Vec<Literal>],
    changed: &[Literal],
    insertion: bool,
) -> Vec<Vec<Literal>> {
    let mut residual = old.to_vec();
    if residual != vec![Vec::new()] {
        if insertion {
            residual.push(changed.to_vec());
        } else if let Some(index) = residual.iter().position(|clause| clause == changed) {
            residual.remove(index);
        }
        residual.sort_unstable();
        residual.dedup();
    }
    residual
}

fn repair_continuation(
    compiled: &CompiledContinuation,
    changed_clause: &Clause,
    insertion: bool,
) -> CompiledContinuation {
    let mut position = vec![0usize; compiled.order.len()];
    for (index, &variable) in compiled.order.iter().enumerate() {
        position[variable] = index;
    }
    // Canonical residuals intentionally discard duplicate clauses. After
    // substitution, distinct source clauses can collapse to the same residual,
    // so deleting one source clause from a suffix requires provenance counts.
    // Until those are retained, deletion safely rebuilds from the root.
    let start = if insertion {
        changed_clause
            .0
            .iter()
            .map(|&(variable, _)| position[variable])
            .min()
            .unwrap_or(0)
    } else {
        0
    };
    let mut changed = changed_clause.0.clone();
    changed.sort_unstable();
    changed.dedup();
    let mut boundary_ids = HashMap::new();
    let mut boundary = Vec::new();
    let mut remap = Vec::with_capacity(compiled.residual_layers[start].len());
    for old in &compiled.residual_layers[start] {
        let residual = apply_clause_change_to_residual(old, &changed, insertion);
        let id = if let Some(&existing) = boundary_ids.get(&residual) {
            existing
        } else {
            let id = boundary.len();
            boundary_ids.insert(residual.clone(), id);
            boundary.push(residual);
            id
        };
        remap.push(id);
    }
    let mut transitions = compiled.transitions[..start].to_vec();
    if start > 0 {
        for targets in &mut transitions[start - 1] {
            targets[0] = remap[targets[0]];
            targets[1] = remap[targets[1]];
        }
    }
    let mut residual_layers: Vec<_> = compiled.residual_layers[..start]
        .iter()
        .map(|layer| {
            layer
                .iter()
                .map(|residual| apply_clause_change_to_residual(residual, &changed, insertion))
                .collect()
        })
        .collect();
    residual_layers.push(boundary.clone());
    let mut current = boundary;
    let mut peak_classes = residual_layers.iter().map(Vec::len).max().unwrap_or(1);
    let mut work = 0usize;
    for &variable in &compiled.order[start..] {
        let mut next_ids = HashMap::new();
        let mut next_residuals = Vec::new();
        let mut layer = vec![[0usize; 2]; current.len()];
        for (state, residual) in current.iter().enumerate() {
            for value in [false, true] {
                let canonical =
                    canonical_residual_after_choice(residual, variable, value, &mut work);
                let next = if let Some(&existing) = next_ids.get(&canonical) {
                    existing
                } else {
                    let id = next_residuals.len();
                    next_ids.insert(canonical.clone(), id);
                    next_residuals.push(canonical);
                    id
                };
                layer[state][value as usize] = next;
            }
        }
        transitions.push(layer);
        current = next_residuals;
        residual_layers.push(current.clone());
        peak_classes = peak_classes.max(current.len());
    }
    let terminal_sat = current.iter().map(Vec::is_empty).collect();
    CompiledContinuation {
        order: compiled.order.clone(),
        transitions,
        residual_layers,
        terminal_sat,
        peak_classes,
    }
}

fn query_continuation(
    compiled: &CompiledContinuation,
    assumptions: &[Option<bool>],
    scratch: &mut ContinuationScratch,
) -> Option<Vec<bool>> {
    scratch.reachable.fill(false);
    scratch.reachable[0] = true;
    let mut current_len = 1usize;
    for (layer_index, &variable) in compiled.order.iter().enumerate() {
        let next_len = if layer_index + 1 < compiled.transitions.len() {
            compiled.transitions[layer_index + 1].len()
        } else {
            compiled.terminal_sat.len()
        };
        scratch.next[..next_len].fill(false);
        scratch.parents[layer_index].fill(None);
        for state in 0..current_len {
            let is_reachable = scratch.reachable[state];
            if !is_reachable {
                continue;
            }
            for value in [false, true] {
                if assumptions[variable].is_some_and(|required| required != value) {
                    continue;
                }
                let target = compiled.transitions[layer_index][state][value as usize];
                if !scratch.next[target] {
                    scratch.next[target] = true;
                    scratch.parents[layer_index][target] = Some((state, value));
                }
            }
        }
        std::mem::swap(&mut scratch.reachable, &mut scratch.next);
        current_len = next_len;
    }
    let mut state = scratch.reachable[..current_len]
        .iter()
        .zip(&compiled.terminal_sat)
        .position(|(&is_reachable, &is_sat)| is_reachable && is_sat)?;
    let mut assignment = vec![false; assumptions.len()];
    for layer_index in (0..compiled.order.len()).rev() {
        let (previous, value) = scratch.parents[layer_index][state]?;
        assignment[compiled.order[layer_index]] = value;
        state = previous;
    }
    Some(assignment)
}

fn benchmark_continuation_reuse(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    query_count: usize,
    max_assumptions: usize,
    output: &Path,
) -> Result<(), String> {
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let order: Vec<_> = (0..vars).collect();
    let bound_bits = continuation_frontier_bound_bits(vars, &formula, &order);
    if bound_bits > 16 {
        return Err(format!(
            "continuation reuse rejected by 16-bit gate: bound is {bound_bits} bits"
        ));
    }
    let compile_start = Instant::now();
    let compiled = compile_continuation(&formula, &order);
    let compile_ns = compile_start.elapsed().as_nanos();
    let mut rng = Rng(formula_seed ^ 0xa076_1d64_78bd_642f);
    let mut queries = Vec::with_capacity(query_count);
    for query_index in 0..query_count {
        let mut assumptions = vec![None; vars];
        let width = 1 + query_index % max_assumptions.max(1);
        let mut chosen = BTreeSet::new();
        while chosen.len() < width.min(vars) {
            chosen.insert(rng.below(vars));
        }
        for variable in chosen {
            assumptions[variable] = Some(rng.next() & 1 == 1);
        }
        queries.push(assumptions);
    }
    let quotient_start = Instant::now();
    let mut scratch = ContinuationScratch::new(&compiled);
    let mut quotient_answers = Vec::with_capacity(queries.len());
    for assumptions in &queries {
        quotient_answers.push(query_continuation(&compiled, assumptions, &mut scratch));
    }
    let quotient_query_ns = quotient_start.elapsed().as_nanos();
    let varisat_start = Instant::now();
    let varisat_answers: Vec<_> = queries
        .iter()
        .map(|assumptions| {
            let mut queried = formula.clone();
            queried.extend(
                assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Clause(vec![(variable, value)]))
                    }),
            );
            solve_with_varisat(vars, &queried)
        })
        .collect();
    let varisat_query_ns = varisat_start.elapsed().as_nanos();
    let mut incremental_solver = Solver::new();
    add_to_varisat(&mut incremental_solver, &formula);
    let incremental_start = Instant::now();
    let incremental_answers: Vec<Option<Vec<bool>>> = queries
        .iter()
        .map(|assumptions| {
            let literals: Vec<_> = assumptions
                .iter()
                .enumerate()
                .filter_map(|(variable, value)| {
                    value.map(|value| Lit::from_var(Var::from_index(variable), value))
                })
                .collect();
            incremental_solver.assume(&literals);
            let sat = incremental_solver
                .solve()
                .expect("incremental Varisat solve");
            if !sat {
                return None;
            }
            let mut assignment = vec![false; vars];
            for literal in incremental_solver
                .model()
                .expect("incremental Varisat model")
            {
                if literal.var().index() < vars {
                    assignment[literal.var().index()] = literal.is_positive();
                }
            }
            Some(assignment)
        })
        .collect();
    let incremental_query_ns = incremental_start.elapsed().as_nanos();
    let agreement = quotient_answers
        .iter()
        .zip(&varisat_answers)
        .all(|(left, right)| left.is_some() == right.is_some());
    let sat_queries = quotient_answers
        .iter()
        .filter(|answer| answer.is_some())
        .count();
    let unsat_queries = query_count.saturating_sub(sat_queries);
    let incremental_agreement = quotient_answers
        .iter()
        .zip(&incremental_answers)
        .all(|(left, right)| left.is_some() == right.is_some());
    let incremental_witnesses_valid =
        incremental_answers
            .iter()
            .zip(&queries)
            .all(|(answer, assumptions)| {
                answer.as_ref().is_none_or(|assignment| {
                    satisfies(&formula, assignment)
                        && assumptions.iter().enumerate().all(|(variable, required)| {
                            required.is_none_or(|value| assignment[variable] == value)
                        })
                })
            });
    let witnesses_valid = quotient_answers
        .iter()
        .zip(&queries)
        .all(|(answer, assumptions)| {
            answer.as_ref().is_none_or(|assignment| {
                satisfies(&formula, assignment)
                    && assumptions.iter().enumerate().all(|(variable, required)| {
                        required.is_none_or(|value| assignment[variable] == value)
                    })
            })
        });
    let quotient_per_query = quotient_query_ns as f64 / query_count.max(1) as f64;
    let varisat_per_query = varisat_query_ns as f64 / query_count.max(1) as f64;
    let incremental_per_query = incremental_query_ns as f64 / query_count.max(1) as f64;
    let break_even_queries = if varisat_per_query > quotient_per_query {
        (compile_ns as f64 / (varisat_per_query - quotient_per_query)).ceil() as u128
    } else {
        u128::MAX
    };
    let incremental_break_even_queries = if incremental_per_query > quotient_per_query {
        (compile_ns as f64 / (incremental_per_query - quotient_per_query)).ceil() as u128
    } else {
        u128::MAX
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create reuse output: {error}"))?;
    }
    fs::write(
        output,
        format!("family,formula_seed,variables,queries,max_assumptions,sat_queries,unsat_queries,frontier_bound_bits,peak_classes,compile_ns,quotient_query_ns,fresh_varisat_query_ns,incremental_varisat_query_ns,quotient_ns_per_query,fresh_varisat_ns_per_query,incremental_varisat_ns_per_query,speedup_vs_fresh,speedup_vs_incremental,break_even_fresh_queries,break_even_incremental_queries,agreement,incremental_agreement,witnesses_valid,incremental_witnesses_valid\n{family},{formula_seed},{vars},{query_count},{max_assumptions},{sat_queries},{unsat_queries},{bound_bits},{},{compile_ns},{quotient_query_ns},{varisat_query_ns},{incremental_query_ns},{quotient_per_query:.3},{varisat_per_query:.3},{incremental_per_query:.3},{:.6},{:.6},{break_even_queries},{incremental_break_even_queries},{agreement},{incremental_agreement},{witnesses_valid},{incremental_witnesses_valid}\n", compiled.peak_classes, varisat_per_query / quotient_per_query.max(1.0), incremental_per_query / quotient_per_query.max(1.0)),
    )
    .map_err(|error| format!("write reuse output: {error}"))?;
    println!(
        "continuation reuse family={family} seed={formula_seed} vars={vars} queries={query_count} max_assumptions={max_assumptions} sat={sat_queries} unsat={unsat_queries} peak={} agreement={agreement} incremental_agreement={incremental_agreement} witnesses_valid={witnesses_valid} incremental_witnesses_valid={incremental_witnesses_valid} output={}",
        compiled.peak_classes,
        output.display()
    );
    Ok(())
}

fn temporal_memory_formula(width: usize, horizon: usize) -> (usize, Vec<Clause>) {
    let vars = width * (horizon + 1);
    let mut formula = Vec::with_capacity(2 * width * horizon);
    for time in 0..horizon {
        for bit in 0..width {
            let current = time * width + bit;
            let next = (time + 1) * width + bit;
            // next == current
            formula.push(Clause(vec![(current, false), (next, true)]));
            formula.push(Clause(vec![(current, true), (next, false)]));
        }
    }
    (vars, formula)
}

fn compile_temporal_memory_continuation(width: usize, horizon: usize) -> CompiledContinuation {
    assert!(width < usize::BITS as usize);
    let vars = width * (horizon + 1);
    let order: Vec<_> = (0..vars).collect();
    let live_states = 1usize << width;
    let contradiction = live_states;
    let mut transitions = Vec::with_capacity(vars);

    // The first frame chooses the remembered state.
    for bit in 0..width {
        let states = 1usize << bit;
        let mut layer = Vec::with_capacity(states);
        for state in 0..states {
            layer.push([state, state | (1usize << bit)]);
        }
        transitions.push(layer);
    }

    // Every later frame must reproduce it. A mismatching observation enters the
    // unique contradictory continuation, which remains contradictory forever.
    for _time in 1..=horizon {
        for bit in 0..width {
            let mut layer = Vec::with_capacity(live_states + 1);
            for state in 0..live_states {
                let required = (state >> bit) & 1;
                layer.push(if required == 0 {
                    [state, contradiction]
                } else {
                    [contradiction, state]
                });
            }
            layer.push([contradiction, contradiction]);
            transitions.push(layer);
        }
    }
    let mut terminal_sat = vec![true; live_states + 1];
    terminal_sat[contradiction] = false;
    CompiledContinuation {
        order,
        transitions,
        residual_layers: Vec::new(),
        terminal_sat,
        peak_classes: live_states + 1,
    }
}

fn query_temporal_memory_kernel(
    width: usize,
    horizon: usize,
    assumptions: &[Option<bool>],
) -> Option<Vec<bool>> {
    let vars = width * (horizon + 1);
    debug_assert_eq!(assumptions.len(), vars);
    let mut state = vec![None; width];
    for (variable, required) in assumptions.iter().enumerate() {
        let Some(value) = required else {
            continue;
        };
        let bit = variable % width;
        if state[bit].is_some_and(|known| known != *value) {
            return None;
        }
        state[bit] = Some(*value);
    }
    let state: Vec<_> = state
        .into_iter()
        .map(|value| value.unwrap_or(false))
        .collect();
    let mut assignment = Vec::with_capacity(vars);
    for _ in 0..=horizon {
        assignment.extend_from_slice(&state);
    }
    Some(assignment)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TemporalRule {
    Copy(usize),
    Negate(usize),
    Xor(usize, usize),
    Circuit(usize, usize, usize),
}

impl TemporalRule {
    fn dependencies(&self) -> Vec<usize> {
        let mut dependencies = match *self {
            Self::Copy(a) | Self::Negate(a) => vec![a],
            Self::Xor(a, b) => vec![a, b],
            Self::Circuit(a, b, c) => vec![a, b, c],
        };
        dependencies.sort_unstable();
        dependencies.dedup();
        dependencies
    }

    fn evaluate(&self, state: usize) -> bool {
        let bit = |index: usize| (state >> index) & 1 == 1;
        match *self {
            Self::Copy(a) => bit(a),
            Self::Negate(a) => !bit(a),
            Self::Xor(a, b) => bit(a) ^ bit(b),
            Self::Circuit(a, b, c) => (bit(a) & bit(b)) ^ bit(c),
        }
    }
}

fn temporal_rules(kind: &str, width: usize) -> Result<Vec<TemporalRule>, String> {
    if width < 3 && kind == "circuit" {
        return Err("circuit transition requires width at least 3".to_string());
    }
    (0..width)
        .map(|bit| match kind {
            "copy" => Ok(TemporalRule::Copy(bit)),
            "negate" => Ok(TemporalRule::Negate(bit)),
            "permute" => Ok(TemporalRule::Copy((bit + 1) % width)),
            "xor" => Ok(TemporalRule::Xor(bit, (bit + 1) % width)),
            "circuit" => Ok(TemporalRule::Circuit(
                bit,
                (bit + 1) % width,
                (bit + width - 1) % width,
            )),
            _ => Err(format!("unknown temporal transition kind: {kind}")),
        })
        .collect()
}

fn temporal_rule_clauses(
    rule: &TemporalRule,
    output: usize,
    current_offset: usize,
    next_offset: usize,
) -> Vec<Clause> {
    let dependencies = rule.dependencies();
    let mut clauses = Vec::with_capacity(1usize << dependencies.len());
    for pattern in 0..(1usize << dependencies.len()) {
        let mut state = 0usize;
        let mut literals = Vec::with_capacity(dependencies.len() + 1);
        for (position, &dependency) in dependencies.iter().enumerate() {
            let value = (pattern >> position) & 1 == 1;
            if value {
                state |= 1usize << dependency;
            }
            literals.push((current_offset + dependency, !value));
        }
        literals.push((next_offset + output, rule.evaluate(state)));
        literals.sort_unstable();
        clauses.push(Clause(literals));
    }
    clauses
}

fn temporal_vocabulary_formula(
    kind: &str,
    width: usize,
    horizon: usize,
) -> Result<(usize, Vec<Clause>), String> {
    let rules = temporal_rules(kind, width)?;
    let mut formula = Vec::new();
    for time in 0..horizon {
        let current = time * width;
        let next = (time + 1) * width;
        for (output, rule) in rules.iter().enumerate() {
            formula.extend(temporal_rule_clauses(rule, output, current, next));
        }
    }
    Ok((width * (horizon + 1), formula))
}

fn composed_transition_dependencies(
    kind: &str,
    width: usize,
    output: usize,
) -> Result<Vec<usize>, String> {
    if width < 4 {
        return Err("composed transitions require width at least 4".to_string());
    }
    let count = if matches!(kind, "cascade4" | "watchdog4") {
        4
    } else {
        3
    };
    match kind {
        "majority3" | "sensor-vote3" | "mux3" | "mixed3" | "cascade4" | "watchdog4" => {
            Ok((0..count).map(|offset| (output + offset) % width).collect())
        }
        "hub3" => {
            let mut dependencies = vec![output, 0, (output + 1) % width];
            dependencies.dedup();
            if dependencies.len() < 3 {
                for candidate in 0..width {
                    if !dependencies.contains(&candidate) {
                        dependencies.push(candidate);
                    }
                    if dependencies.len() == 3 {
                        break;
                    }
                }
            }
            Ok(dependencies)
        }
        "tree3" | "irregular3" => {
            let preferred = if kind == "tree3" {
                vec![
                    output,
                    output.saturating_sub(1) / 2,
                    (2 * output + 1) % width,
                ]
            } else {
                vec![output, (output * 3 + 1) % width, (output * 5 + 2) % width]
            };
            let mut dependencies = Vec::with_capacity(3);
            for candidate in preferred.into_iter().chain(0..width) {
                if !dependencies.contains(&candidate) {
                    dependencies.push(candidate);
                }
                if dependencies.len() == 3 {
                    break;
                }
            }
            Ok(dependencies)
        }
        _ => Err(format!("unknown composed transition kind: {kind}")),
    }
}

fn evaluate_composed_transition(kind: &str, values: &[bool]) -> bool {
    match kind {
        "majority3" | "sensor-vote3" => {
            (values[0] & values[1]) | (values[0] & values[2]) | (values[1] & values[2])
        }
        "mux3" => {
            if values[0] {
                values[1]
            } else {
                values[2]
            }
        }
        "mixed3" => (values[0] ^ values[1]) & !values[2],
        "cascade4" | "watchdog4" => (values[0] ^ values[1]) ^ (values[2] & values[3]),
        "hub3" => {
            if values[0] {
                values[1]
            } else {
                values[2]
            }
        }
        "tree3" => (values[0] & values[1]) | (values[0] & values[2]) | (values[1] & values[2]),
        "irregular3" => (values[0] ^ values[1]) & !values[2],
        _ => unreachable!("validated composed transition kind"),
    }
}

fn temporal_composition_formula(
    kind: &str,
    width: usize,
    horizon: usize,
) -> Result<(usize, Vec<Clause>), String> {
    let mut formula = Vec::new();
    for time in 0..horizon {
        let current = time * width;
        let next = (time + 1) * width;
        for output in 0..width {
            let dependencies = composed_transition_dependencies(kind, width, output)?;
            for pattern in 0..(1usize << dependencies.len()) {
                let mut values = Vec::with_capacity(dependencies.len());
                let mut literals = Vec::with_capacity(dependencies.len() + 1);
                for (position, &dependency) in dependencies.iter().enumerate() {
                    let value = (pattern >> position) & 1 == 1;
                    values.push(value);
                    literals.push((current + dependency, !value));
                }
                literals.push((next + output, evaluate_composed_transition(kind, &values)));
                literals.sort_unstable();
                formula.push(Clause(literals));
            }
        }
    }
    Ok((width * (horizon + 1), formula))
}

#[derive(Clone, Debug)]
struct AagLatch {
    current: usize,
    next: usize,
    initial: Option<bool>,
}

#[derive(Clone, Debug)]
struct AagAnd {
    output: usize,
    left: usize,
    right: usize,
}

#[derive(Clone, Debug)]
struct AagModel {
    max_variable: usize,
    inputs: Vec<usize>,
    input_names: Vec<String>,
    latches: Vec<AagLatch>,
    latch_names: Vec<String>,
    outputs: Vec<usize>,
    output_names: Vec<String>,
    ands: Vec<AagAnd>,
}

type AagTemporalEncoding = (usize, Vec<Clause>, Vec<Option<bool>>);
type AagPropertyQuery = (usize, usize, Vec<Option<bool>>);

fn parse_aag_usize(token: Option<&str>, context: &str) -> Result<usize, String> {
    token
        .ok_or_else(|| format!("missing {context}"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {context}"))
}

fn parse_aag(path: &Path) -> Result<AagModel, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("inspect ASCII AIGER {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "ASCII AIGER input is not a file: {}",
            path.display()
        ));
    }
    if metadata.len() > AAG_INPUT_LIMIT_BYTES {
        return Err(format!(
            "ASCII AIGER input exceeds safety limit {AAG_INPUT_LIMIT_BYTES} bytes"
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("read ASCII AIGER {}: {error}", path.display()))?;
    if bytes.len() as u64 > AAG_INPUT_LIMIT_BYTES {
        return Err(format!(
            "ASCII AIGER input exceeds safety limit {AAG_INPUT_LIMIT_BYTES} bytes"
        ));
    }
    if !bytes.is_ascii() {
        return Err("ASCII AIGER input contains non-ASCII bytes".to_string());
    }
    let source = std::str::from_utf8(&bytes)
        .map_err(|_| "ASCII AIGER input is not valid UTF-8".to_string())?;
    let mut lines = source.lines();
    let mut header = lines
        .next()
        .ok_or_else(|| "empty ASCII AIGER input".to_string())?
        .split_whitespace();
    if header.next() != Some("aag") {
        return Err("only ASCII AIGER (`aag`) input is supported".to_string());
    }
    let max_variable = parse_aag_usize(header.next(), "AIGER maximum variable")?;
    let inputs = parse_aag_usize(header.next(), "AIGER input count")?;
    let latch_count = parse_aag_usize(header.next(), "AIGER latch count")?;
    let output_count = parse_aag_usize(header.next(), "AIGER output count")?;
    let and_count = parse_aag_usize(header.next(), "AIGER AND count")?;
    if header.next().is_some() {
        return Err("extended AIGER headers are not supported yet".to_string());
    }
    if latch_count == 0 {
        return Err("AIGER model must contain at least one latch".to_string());
    }
    if max_variable > 1_000_000 {
        return Err("AIGER maximum variable exceeds safety limit 1000000".to_string());
    }
    let defined_variables = inputs
        .checked_add(latch_count)
        .and_then(|value| value.checked_add(and_count))
        .ok_or_else(|| "AIGER definition count overflow".to_string())?;
    if defined_variables != max_variable {
        return Err(format!(
            "AIGER header requires M = I + L + A; found {max_variable} != {defined_variables}"
        ));
    }
    if output_count > 1_000_000 {
        return Err("AIGER output count exceeds safety limit 1000000".to_string());
    }

    let mut input_literals = Vec::with_capacity(inputs);
    for index in 0..inputs {
        let line = lines
            .next()
            .ok_or_else(|| format!("truncated AIGER input section at input {index}"))?;
        let mut fields = line.split_whitespace();
        let literal = parse_aag_usize(fields.next(), "input literal")?;
        if fields.next().is_some() || literal == 0 || literal & 1 == 1 || literal / 2 > max_variable
        {
            return Err(format!("invalid AIGER input {index}"));
        }
        input_literals.push(literal);
    }
    let mut latches = Vec::with_capacity(latch_count);
    for index in 0..latch_count {
        let line = lines
            .next()
            .ok_or_else(|| format!("truncated AIGER latch section at latch {index}"))?;
        let fields: Vec<_> = line.split_whitespace().collect();
        if !(2..=3).contains(&fields.len()) {
            return Err(format!("invalid AIGER latch {index}"));
        }
        let current = parse_aag_usize(fields.first().copied(), "latch literal")?;
        let next = parse_aag_usize(fields.get(1).copied(), "latch next literal")?;
        if current == 0 || current & 1 == 1 || current / 2 > max_variable {
            return Err(format!(
                "invalid current literal {current} for latch {index}"
            ));
        }
        let initial = match fields.get(2).copied() {
            None | Some("0") => Some(false),
            Some("1") => Some(true),
            Some(value) if value.parse::<usize>().ok() == Some(current) => None,
            Some(_) => return Err(format!("unsupported initial value for latch {index}")),
        };
        latches.push(AagLatch {
            current,
            next,
            initial,
        });
    }
    let mut outputs = Vec::with_capacity(output_count);
    for index in 0..output_count {
        let line = lines
            .next()
            .ok_or_else(|| format!("truncated AIGER output section at output {index}"))?;
        let mut fields = line.split_whitespace();
        let literal = parse_aag_usize(fields.next(), "output literal")?;
        if fields.next().is_some() {
            return Err(format!("invalid AIGER output {index}"));
        }
        outputs.push(literal);
    }
    let mut ands = Vec::with_capacity(and_count);
    for index in 0..and_count {
        let line = lines
            .next()
            .ok_or_else(|| format!("truncated AIGER AND section at gate {index}"))?;
        let mut fields = line.split_whitespace();
        let output = parse_aag_usize(fields.next(), "AND output literal")?;
        let left = parse_aag_usize(fields.next(), "AND left literal")?;
        let right = parse_aag_usize(fields.next(), "AND right literal")?;
        if fields.next().is_some()
            || output == 0
            || output & 1 == 1
            || output / 2 > max_variable
            || left / 2 >= output / 2
            || right / 2 >= output / 2
        {
            return Err(format!("invalid or non-topological AIGER AND gate {index}"));
        }
        ands.push(AagAnd {
            output,
            left,
            right,
        });
    }
    let literal_limit = max_variable
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "AIGER literal range overflow".to_string())?;
    if input_literals
        .iter()
        .copied()
        .chain(latches.iter().flat_map(|latch| [latch.current, latch.next]))
        .chain(outputs.iter().copied())
        .chain(
            ands.iter()
                .flat_map(|gate| [gate.output, gate.left, gate.right]),
        )
        .any(|literal| literal > literal_limit)
    {
        return Err("AIGER literal exceeds declared maximum variable".to_string());
    }
    let mut definitions = BTreeSet::new();
    for &literal in &input_literals {
        if !definitions.insert(literal / 2) {
            return Err("duplicate AIGER variable definition".to_string());
        }
    }
    for latch in &latches {
        if !definitions.insert(latch.current / 2) {
            return Err("duplicate AIGER variable definition".to_string());
        }
    }
    for gate in &ands {
        if !definitions.insert(gate.output / 2) {
            return Err("duplicate AIGER variable definition".to_string());
        }
    }
    if latches
        .iter()
        .map(|latch| latch.next)
        .chain(outputs.iter().copied())
        .chain(ands.iter().flat_map(|gate| [gate.left, gate.right]))
        .any(|literal| literal >= 2 && !definitions.contains(&(literal / 2)))
    {
        return Err("AIGER literal references an undefined variable".to_string());
    }
    let mut input_names = (0..inputs)
        .map(|index| format!("input_{index}"))
        .collect::<Vec<_>>();
    let mut latch_names = (0..latch_count)
        .map(|index| format!("latch_{index}"))
        .collect::<Vec<_>>();
    let mut output_names = (0..output_count)
        .map(|index| format!("bad_{index}"))
        .collect::<Vec<_>>();
    let mut seen_symbols = BTreeSet::new();
    for line in lines {
        if line == "c" {
            break;
        }
        let Some((designator, name)) = line.split_once(' ') else {
            return Err("invalid AIGER symbol line".to_string());
        };
        if name.is_empty() || name.len() > 4096 {
            return Err("invalid AIGER symbol name".to_string());
        }
        let mut chars = designator.chars();
        let kind = chars
            .next()
            .ok_or_else(|| "empty AIGER symbol designator".to_string())?;
        let index = chars
            .as_str()
            .parse::<usize>()
            .map_err(|_| "invalid AIGER symbol index".to_string())?;
        if !seen_symbols.insert((kind, index)) {
            return Err("duplicate AIGER symbol definition".to_string());
        }
        let target = match kind {
            'i' => input_names.get_mut(index),
            'l' => latch_names.get_mut(index),
            'o' => output_names.get_mut(index),
            _ => None,
        }
        .ok_or_else(|| "unsupported or out-of-range AIGER symbol".to_string())?;
        *target = name.to_string();
    }
    Ok(AagModel {
        max_variable,
        inputs: input_literals,
        input_names,
        latches,
        latch_names,
        outputs,
        output_names,
        ands,
    })
}

fn evaluate_aag_literal(literal: usize, values: &[bool]) -> bool {
    if literal < 2 {
        return literal == 1;
    }
    let base = values[literal / 2];
    if literal & 1 == 1 { !base } else { base }
}

fn aag_temporal_formula(model: &AagModel, horizon: usize) -> Result<AagTemporalEncoding, String> {
    if horizon == 0 {
        return Err("AIGER horizon must be at least one".to_string());
    }
    if !model.inputs.is_empty() {
        return Err("deterministic CQ-SAT/GCC encoding requires zero primary inputs".to_string());
    }
    if model.latches.len() > 9 {
        return Err(format!(
            "deterministic CQ-SAT/GCC encoding supports at most 9 latches; found {}",
            model.latches.len()
        ));
    }
    let width = model.latches.len();
    let patterns = 1usize << width;
    let variables = horizon
        .checked_add(1)
        .and_then(|frames| frames.checked_mul(width))
        .ok_or_else(|| "deterministic AIGER variable count overflow".to_string())?;
    if variables > 2_000_000 {
        return Err(format!(
            "deterministic AIGER encoding requires {variables} variables; safety limit is 2000000"
        ));
    }
    let clause_count = horizon
        .checked_mul(width)
        .and_then(|value| value.checked_mul(patterns))
        .ok_or_else(|| "deterministic AIGER clause count overflow".to_string())?;
    if clause_count > 10_000_000 {
        return Err(format!(
            "deterministic AIGER encoding requires {clause_count} clauses; safety limit is 10000000"
        ));
    }
    let mut tables = Vec::with_capacity(patterns);
    for pattern in 0..patterns {
        let mut values = vec![false; model.max_variable + 1];
        for (bit, latch) in model.latches.iter().enumerate() {
            values[latch.current / 2] = pattern >> bit & 1 == 1;
        }
        for gate in &model.ands {
            values[gate.output / 2] = evaluate_aag_literal(gate.left, &values)
                && evaluate_aag_literal(gate.right, &values);
        }
        tables.push(
            model
                .latches
                .iter()
                .map(|latch| evaluate_aag_literal(latch.next, &values))
                .collect::<Vec<_>>(),
        );
    }
    let mut formula = Vec::with_capacity(clause_count);
    for time in 0..horizon {
        let current = time * width;
        let next = (time + 1) * width;
        for (pattern, values) in tables.iter().enumerate() {
            for (output, &value) in values.iter().enumerate() {
                let mut literals = Vec::with_capacity(width + 1);
                for bit in 0..width {
                    literals.push((current + bit, pattern >> bit & 1 == 0));
                }
                literals.push((next + output, value));
                literals.sort_unstable();
                formula.push(Clause(literals));
            }
        }
    }
    Ok((
        variables,
        formula,
        model.latches.iter().map(|latch| latch.initial).collect(),
    ))
}

fn aag_bad_state_patterns(model: &AagModel) -> Vec<usize> {
    let width = model.latches.len();
    (0..(1usize << width))
        .filter(|&pattern| {
            let mut values = vec![false; model.max_variable + 1];
            for (bit, latch) in model.latches.iter().enumerate() {
                values[latch.current / 2] = pattern >> bit & 1 == 1;
            }
            for gate in &model.ands {
                values[gate.output / 2] = evaluate_aag_literal(gate.left, &values)
                    && evaluate_aag_literal(gate.right, &values);
            }
            model
                .outputs
                .iter()
                .any(|&literal| evaluate_aag_literal(literal, &values))
        })
        .collect()
}

fn aag_property_query_space(
    model: &AagModel,
    horizon: usize,
) -> Result<Vec<AagPropertyQuery>, String> {
    let width = model.latches.len();
    let variables = width * (horizon + 1);
    let bad_patterns = aag_bad_state_patterns(model);
    if bad_patterns.is_empty() {
        return Err("AIGER model has no state where a declared output is true".to_string());
    }
    let upper_bound = bad_patterns
        .len()
        .checked_mul(horizon + 1)
        .ok_or_else(|| "AIGER property query count overflow".to_string())?;
    if upper_bound > 1_000_000 {
        return Err(format!(
            "AIGER exhaustive property space has up to {upper_bound} queries; safety limit is 1000000"
        ));
    }
    let initial: Vec<_> = model.latches.iter().map(|latch| latch.initial).collect();
    let mut queries = Vec::new();
    for frame in 0..=horizon {
        for &pattern in &bad_patterns {
            if frame == 0
                && initial.iter().enumerate().any(|(bit, required)| {
                    required.is_some_and(|value| value != (pattern >> bit & 1 == 1))
                })
            {
                continue;
            }
            let mut assumptions = vec![None; variables];
            assumptions[..width].copy_from_slice(&initial);
            for bit in 0..width {
                assumptions[frame * width + bit] = Some(pattern >> bit & 1 == 1);
            }
            queries.push((frame, pattern, assumptions));
        }
    }
    Ok(queries)
}

fn aag_property_queries(
    model: &AagModel,
    horizon: usize,
    query_count: usize,
) -> Result<Vec<Vec<Option<bool>>>, String> {
    let space = aag_property_query_space(model, horizon)?;
    Ok((0..query_count)
        .map(|index| space[index % space.len()].2.clone())
        .collect())
}

#[derive(Clone, Copy)]
enum AagCnfLiteral {
    Constant(bool),
    Variable(Literal),
}

impl AagCnfLiteral {
    fn negate(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Variable((variable, positive)) => Self::Variable((variable, !positive)),
        }
    }
}

#[derive(Clone)]
struct AagBmcQuery {
    frame: usize,
    output: usize,
    assumption: AagCnfLiteral,
}

struct AagBmcEncoding {
    variables: usize,
    clauses: Vec<Clause>,
    queries: Vec<AagBmcQuery>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AagInputConstraint {
    name: String,
    pattern: AagInputConstraintPattern,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AagInputConstraintPattern {
    Constant(bool),
    StartupReset {
        asserted_frames: usize,
        asserted_value: bool,
    },
}

impl AagInputConstraintPattern {
    fn value_at(&self, frame: usize) -> bool {
        match *self {
            Self::Constant(value) => value,
            Self::StartupReset {
                asserted_frames,
                asserted_value,
            } => {
                if frame < asserted_frames {
                    asserted_value
                } else {
                    !asserted_value
                }
            }
        }
    }

    fn report(&self) -> String {
        match *self {
            Self::Constant(value) => usize::from(value).to_string(),
            Self::StartupReset {
                asserted_frames,
                asserted_value,
            } => format!(
                "startup(asserted_frames={asserted_frames},asserted_value={})",
                usize::from(asserted_value)
            ),
        }
    }
}

fn resolve_aag_input_constraints(
    model: &AagModel,
    constraints: &[AagInputConstraint],
) -> Result<Vec<(usize, AagInputConstraintPattern)>, String> {
    let mut resolved = Vec::with_capacity(constraints.len());
    let mut seen = BTreeSet::new();
    for constraint in constraints {
        if !seen.insert(constraint.name.as_str()) {
            return Err(format!(
                "duplicate environment assumption: {}",
                constraint.name
            ));
        }
        let matches = model
            .input_names
            .iter()
            .enumerate()
            .filter(|(_, name)| *name == &constraint.name)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "environment assumption input `{}` matched {} synthesized inputs; expected exactly one",
                constraint.name,
                matches.len()
            ));
        }
        resolved.push((matches[0].0, constraint.pattern.clone()));
    }
    Ok(resolved)
}

fn aag_cnf_literal(model: &AagModel, frame: usize, literal: usize) -> AagCnfLiteral {
    if literal < 2 {
        return AagCnfLiteral::Constant(literal == 1);
    }
    AagCnfLiteral::Variable((
        frame * model.max_variable + literal / 2 - 1,
        literal & 1 == 0,
    ))
}

fn push_simplified_clause(clauses: &mut Vec<Clause>, literals: &[AagCnfLiteral]) {
    let mut clause = Vec::with_capacity(literals.len());
    for &literal in literals {
        match literal {
            AagCnfLiteral::Constant(true) => return,
            AagCnfLiteral::Constant(false) => {}
            AagCnfLiteral::Variable(literal) => clause.push(literal),
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
    clauses.push(Clause(clause));
}

fn encode_aag_equivalence(clauses: &mut Vec<Clause>, output: AagCnfLiteral, value: AagCnfLiteral) {
    push_simplified_clause(clauses, &[output.negate(), value]);
    push_simplified_clause(clauses, &[output, value.negate()]);
}

fn aag_bmc_encoding_with_constraints(
    model: &AagModel,
    horizon: usize,
    constraints: &[AagInputConstraint],
) -> Result<AagBmcEncoding, String> {
    let frames = horizon
        .checked_add(1)
        .ok_or_else(|| "AIGER horizon overflow".to_string())?;
    let variables = frames
        .checked_mul(model.max_variable)
        .ok_or_else(|| "AIGER BMC variable count overflow".to_string())?;
    if variables > 2_000_000 {
        return Err(format!(
            "AIGER BMC requires {variables} variables; safety limit is 2000000"
        ));
    }
    let resolved_constraints = resolve_aag_input_constraints(model, constraints)?;
    let clause_bound = frames
        .checked_mul(model.ands.len().saturating_mul(3))
        .and_then(|value| value.checked_add(horizon.saturating_mul(model.latches.len() * 2)))
        .and_then(|value| value.checked_add(frames.saturating_mul(resolved_constraints.len())))
        .ok_or_else(|| "AIGER BMC clause count overflow".to_string())?;
    if clause_bound > 10_000_000 {
        return Err(format!(
            "AIGER BMC requires up to {clause_bound} clauses; safety limit is 10000000"
        ));
    }
    let mut clauses = Vec::with_capacity(clause_bound + model.latches.len());
    for latch in &model.latches {
        if let Some(initial) = latch.initial {
            push_simplified_clause(
                &mut clauses,
                &[AagCnfLiteral::Variable((latch.current / 2 - 1, initial))],
            );
        }
    }
    for frame in 0..=horizon {
        for (input, pattern) in &resolved_constraints {
            let value = pattern.value_at(frame);
            push_simplified_clause(
                &mut clauses,
                &[AagCnfLiteral::Variable((
                    frame * model.max_variable + model.inputs[*input] / 2 - 1,
                    value,
                ))],
            );
        }
        for gate in &model.ands {
            let output = aag_cnf_literal(model, frame, gate.output);
            let left = aag_cnf_literal(model, frame, gate.left);
            let right = aag_cnf_literal(model, frame, gate.right);
            push_simplified_clause(&mut clauses, &[output.negate(), left]);
            push_simplified_clause(&mut clauses, &[output.negate(), right]);
            push_simplified_clause(&mut clauses, &[output, left.negate(), right.negate()]);
        }
        if frame < horizon {
            for latch in &model.latches {
                encode_aag_equivalence(
                    &mut clauses,
                    aag_cnf_literal(model, frame + 1, latch.current),
                    aag_cnf_literal(model, frame, latch.next),
                );
            }
        }
    }
    let mut queries = Vec::new();
    for frame in 0..=horizon {
        for (output, &literal) in model.outputs.iter().enumerate() {
            let assumption = aag_cnf_literal(model, frame, literal);
            if !matches!(assumption, AagCnfLiteral::Constant(false)) {
                queries.push(AagBmcQuery {
                    frame,
                    output,
                    assumption,
                });
            }
        }
    }
    Ok(AagBmcEncoding {
        variables,
        clauses,
        queries,
    })
}

fn aag_bmc_encoding(model: &AagModel, horizon: usize) -> Result<AagBmcEncoding, String> {
    aag_bmc_encoding_with_constraints(model, horizon, &[])
}

fn normalized_temporal_steps(
    formula: &[Clause],
    width: usize,
    horizon: usize,
) -> Result<Vec<Vec<Vec<Literal>>>, String> {
    let mut steps = vec![Vec::new(); horizon];
    for clause in formula {
        let minimum = clause
            .0
            .iter()
            .map(|&(variable, _)| variable / width)
            .min()
            .ok_or_else(|| "empty temporal clause".to_string())?;
        let maximum = clause
            .0
            .iter()
            .map(|&(variable, _)| variable / width)
            .max()
            .unwrap();
        if maximum != minimum + 1 || minimum >= horizon {
            return Err("clause is not local to adjacent temporal frames".to_string());
        }
        let offset = minimum * width;
        let mut literals: Vec<_> = clause
            .0
            .iter()
            .map(|&(variable, value)| (variable - offset, value))
            .collect();
        literals.sort_unstable();
        steps[minimum].push(literals);
    }
    for step in &mut steps {
        step.sort_unstable();
    }
    Ok(steps)
}

fn recognize_temporal_rules(
    formula: &[Clause],
    width: usize,
    horizon: usize,
) -> Result<Vec<TemporalRule>, String> {
    let steps = normalized_temporal_steps(formula, width, horizon)?;
    let template = steps
        .first()
        .ok_or_else(|| "temporal horizon must be positive".to_string())?;
    for (time, step) in steps.iter().enumerate().skip(1) {
        if step != template {
            return Err(format!("transition template changes at time {time}"));
        }
    }
    let mut rules = Vec::with_capacity(width);
    for output in 0..width {
        let output_variable = width + output;
        let local: Vec<_> = template
            .iter()
            .filter(|clause| {
                clause
                    .iter()
                    .any(|&(variable, _)| variable == output_variable)
            })
            .collect();
        if local.is_empty() {
            return Err(format!("output {output} has no defining clauses"));
        }
        let mut dependencies: Vec<_> = local
            .iter()
            .flat_map(|clause| clause.iter())
            .filter_map(|&(variable, _)| (variable < width).then_some(variable))
            .collect();
        dependencies.sort_unstable();
        dependencies.dedup();
        let mut table = Vec::with_capacity(1usize << dependencies.len());
        for pattern in 0..(1usize << dependencies.len()) {
            let mut valid = Vec::new();
            for output_value in [false, true] {
                let satisfies_local = local.iter().all(|clause| {
                    clause.iter().any(|&(variable, positive)| {
                        let value = if variable == output_variable {
                            output_value
                        } else {
                            let position = dependencies
                                .iter()
                                .position(|&dependency| dependency == variable)
                                .expect("recognized dependency");
                            (pattern >> position) & 1 == 1
                        };
                        value == positive
                    })
                });
                if satisfies_local {
                    valid.push(output_value);
                }
            }
            if valid.len() != 1 {
                return Err(format!(
                    "output {output} is not a deterministic local function"
                ));
            }
            table.push(valid[0]);
        }

        let mut candidates = Vec::new();
        for a in 0..width {
            candidates.push(TemporalRule::Copy(a));
            candidates.push(TemporalRule::Negate(a));
            for b in a + 1..width {
                candidates.push(TemporalRule::Xor(a, b));
            }
        }
        if width >= 3 {
            for a in 0..width {
                for b in 0..width {
                    for c in 0..width {
                        if a != b && a != c && b != c {
                            candidates.push(TemporalRule::Circuit(a, b, c));
                        }
                    }
                }
            }
        }
        let rule = candidates
            .into_iter()
            .find(|candidate| {
                candidate.dependencies() == dependencies
                    && (0..(1usize << dependencies.len())).all(|pattern| {
                        let mut state = 0usize;
                        for (position, &dependency) in dependencies.iter().enumerate() {
                            if (pattern >> position) & 1 == 1 {
                                state |= 1usize << dependency;
                            }
                        }
                        candidate.evaluate(state) == table[pattern]
                    })
            })
            .ok_or_else(|| format!("output {output} is outside the fixed vocabulary"))?;
        rules.push(rule);
    }
    Ok(rules)
}

#[derive(Clone)]
struct LocalBooleanFunction {
    dependencies: Vec<usize>,
    table: Vec<bool>,
}

fn recover_local_transition(
    formula: &[Clause],
    width: usize,
    horizon: usize,
) -> Result<Vec<LocalBooleanFunction>, String> {
    let steps = normalized_temporal_steps(formula, width, horizon)?;
    let template = steps
        .first()
        .ok_or_else(|| "temporal horizon must be positive".to_string())?;
    if steps.iter().skip(1).any(|step| step != template) {
        return Err("transition template is not repeated exactly".to_string());
    }
    let mut output_clauses: Vec<Vec<&Vec<Literal>>> = vec![Vec::new(); width];
    for clause in template {
        let mut outputs: Vec<_> = clause
            .iter()
            .filter(|&&(variable, _)| variable >= width)
            .map(|&(variable, _)| variable - width)
            .collect();
        outputs.sort_unstable();
        outputs.dedup();
        if outputs.len() != 1 {
            return Err(
                "local recovery requires every clause to constrain exactly one output".to_string(),
            );
        }
        output_clauses[outputs[0]].push(clause);
    }
    output_clauses
        .iter()
        .enumerate()
        .map(|(output, clauses)| {
            if clauses.is_empty() {
                return Err(format!("output {output} has no defining clauses"));
            }
            let output_variable = width + output;
            let mut dependencies: Vec<_> = clauses
                .iter()
                .flat_map(|clause| clause.iter())
                .filter_map(|&(variable, _)| (variable < width).then_some(variable))
                .collect();
            dependencies.sort_unstable();
            dependencies.dedup();
            let mut table = Vec::with_capacity(1usize << dependencies.len());
            for pattern in 0..(1usize << dependencies.len()) {
                let valid: Vec<_> = [false, true]
                    .into_iter()
                    .filter(|&output_value| {
                        clauses.iter().all(|clause| {
                            clause.iter().any(|&(variable, positive)| {
                                let value = if variable == output_variable {
                                    output_value
                                } else {
                                    let position = dependencies
                                        .iter()
                                        .position(|&dependency| dependency == variable)
                                        .expect("local dependency");
                                    (pattern >> position) & 1 == 1
                                };
                                value == positive
                            })
                        })
                    })
                    .collect();
                if valid.len() != 1 {
                    return Err(format!(
                        "output {output} is not a total deterministic local function"
                    ));
                }
                table.push(valid[0]);
            }
            Ok(LocalBooleanFunction {
                dependencies,
                table,
            })
        })
        .collect()
}

struct SymbolicTemporalTransition {
    width: usize,
    horizon: usize,
    functions: Vec<LocalBooleanFunction>,
}

impl SymbolicTemporalTransition {
    fn recognize(formula: &[Clause], width: usize, horizon: usize) -> Result<Self, String> {
        Ok(Self {
            width,
            horizon,
            functions: recover_local_transition(formula, width, horizon)?,
        })
    }

    fn next_state(&self, state: &[bool]) -> Vec<bool> {
        self.functions
            .iter()
            .map(|function| {
                let pattern = function.dependencies.iter().enumerate().fold(
                    0usize,
                    |pattern, (position, &dependency)| {
                        pattern | (usize::from(state[dependency]) << position)
                    },
                );
                function.table[pattern]
            })
            .collect()
    }

    fn query(&self, assumptions: &[Option<bool>]) -> Option<Vec<bool>> {
        let mut state: Vec<_> = assumptions
            .iter()
            .take(self.width)
            .copied()
            .collect::<Option<_>>()?;
        let mut assignment = Vec::with_capacity(self.width * (self.horizon + 1));
        for time in 0..=self.horizon {
            if (0..self.width).any(|bit| {
                assumptions[time * self.width + bit].is_some_and(|required| required != state[bit])
            }) {
                return None;
            }
            assignment.extend_from_slice(&state);
            if time < self.horizon {
                state = self.next_state(&state);
            }
        }
        Some(assignment)
    }

    fn representation_entries(&self) -> usize {
        self.functions
            .iter()
            .map(|function| function.dependencies.len() + function.table.len())
            .sum()
    }
}

fn compose_local_bdd(
    manager: &mut BddManager,
    inputs: &[usize],
    table: &[bool],
    level: usize,
    pattern: usize,
) -> usize {
    if level == inputs.len() {
        return usize::from(table[pattern]);
    }
    let low = compose_local_bdd(manager, inputs, table, level + 1, pattern);
    let high = compose_local_bdd(
        manager,
        inputs,
        table,
        level + 1,
        pattern | (1usize << level),
    );
    if low == high {
        return low;
    }
    let condition = inputs[level];
    let mut memo = HashMap::new();
    let not_condition = manager.negate(condition, &mut memo);
    let low_branch = manager.and(not_condition, low);
    let high_branch = manager.and(condition, high);
    manager.or(low_branch, high_branch)
}

struct SymbolicPreimageTransition {
    transition: SymbolicTemporalTransition,
    manager: BddManager,
    frames: Vec<Vec<usize>>,
    rank_to_variable: Vec<usize>,
    cycle_start: Option<usize>,
    cycle_length: usize,
}

enum HybridTemporalPreimage {
    Bdd(Box<SymbolicPreimageTransition>),
    Cdcl {
        solver: Solver<'static>,
        variables: usize,
    },
}

impl HybridTemporalPreimage {
    fn recognize(
        formula: &[Clause],
        width: usize,
        horizon: usize,
        node_limit: usize,
    ) -> Result<(Self, &'static str, Option<String>), String> {
        match SymbolicPreimageTransition::recognize_ordered(
            formula,
            width,
            horizon,
            node_limit,
            "dependency-guard",
        ) {
            Ok(preimage) => Ok((Self::Bdd(Box::new(preimage)), "bdd", None)),
            Err(error) if error.contains("growth guard") => {
                let mut solver = Solver::new();
                add_to_varisat(&mut solver, formula);
                Ok((
                    Self::Cdcl {
                        solver,
                        variables: width * (horizon + 1),
                    },
                    "cdcl-fallback",
                    Some(error),
                ))
            }
            Err(error) => Err(error),
        }
    }

    fn query(&mut self, assumptions: &[Option<bool>]) -> Option<Vec<bool>> {
        match self {
            Self::Bdd(preimage) => preimage.query(assumptions),
            Self::Cdcl { solver, variables } => {
                let literals: Vec<_> = assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Lit::from_var(Var::from_index(variable), value))
                    })
                    .collect();
                solver.assume(&literals);
                if !solver.solve().expect("hybrid temporal CDCL solve") {
                    return None;
                }
                let mut assignment = vec![false; *variables];
                for literal in solver.model().expect("hybrid temporal CDCL model") {
                    if literal.var().index() < *variables {
                        assignment[literal.var().index()] = literal.is_positive();
                    }
                }
                Some(assignment)
            }
        }
    }

    fn bdd_metrics(&self) -> (usize, usize, usize, usize) {
        match self {
            Self::Bdd(preimage) => {
                let (cycle_start, cycle_length) =
                    preimage.cycle().map_or((usize::MAX, 0usize), |cycle| cycle);
                (
                    preimage.bdd_nodes(),
                    preimage.compiled_frames(),
                    cycle_start,
                    cycle_length,
                )
            }
            Self::Cdcl { .. } => (0, 0, usize::MAX, 0),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum EncodedLiteral {
    Constant(bool),
    Variable(usize, bool),
}

impl EncodedLiteral {
    fn negated(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Variable(variable, positive) => Self::Variable(variable, !positive),
        }
    }
}

struct AigBuilder {
    next_variable: usize,
    gates: Vec<(usize, EncodedLiteral, EncodedLiteral)>,
    unique: HashMap<(EncodedLiteral, EncodedLiteral), EncodedLiteral>,
}

impl AigBuilder {
    fn new(first_gate_variable: usize) -> Self {
        Self {
            next_variable: first_gate_variable,
            gates: Vec::new(),
            unique: HashMap::new(),
        }
    }

    fn and(&mut self, a: EncodedLiteral, b: EncodedLiteral) -> EncodedLiteral {
        match (a, b) {
            (EncodedLiteral::Constant(false), _) | (_, EncodedLiteral::Constant(false)) => {
                return EncodedLiteral::Constant(false);
            }
            (EncodedLiteral::Constant(true), other) | (other, EncodedLiteral::Constant(true)) => {
                return other;
            }
            _ => {}
        }
        if a == b {
            return a;
        }
        if a == b.negated() {
            return EncodedLiteral::Constant(false);
        }
        let key = if a <= b { (a, b) } else { (b, a) };
        if let Some(&literal) = self.unique.get(&key) {
            return literal;
        }
        let output = EncodedLiteral::Variable(self.next_variable, true);
        self.next_variable += 1;
        self.gates.push((self.next_variable - 1, key.0, key.1));
        self.unique.insert(key, output);
        output
    }

    fn or(&mut self, a: EncodedLiteral, b: EncodedLiteral) -> EncodedLiteral {
        let both_false = self.and(a.negated(), b.negated());
        both_false.negated()
    }

    fn ite(
        &mut self,
        condition: EncodedLiteral,
        high: EncodedLiteral,
        low: EncodedLiteral,
    ) -> EncodedLiteral {
        if high == low {
            return high;
        }
        let high_branch = self.and(condition, high);
        let low_branch = self.and(condition.negated(), low);
        self.or(high_branch, low_branch)
    }
}

fn add_encoded_clause(solver: &mut Solver<'_>, literals: &[EncodedLiteral]) {
    if literals
        .iter()
        .any(|literal| matches!(literal, EncodedLiteral::Constant(true)))
    {
        return;
    }
    let clause: Vec<_> = literals
        .iter()
        .filter_map(|literal| match *literal {
            EncodedLiteral::Constant(false) => None,
            EncodedLiteral::Variable(variable, positive) => {
                Some(Lit::from_var(Var::from_index(variable), positive))
            }
            EncodedLiteral::Constant(true) => unreachable!(),
        })
        .collect();
    solver.add_clause(&clause);
}

fn encoded_bdd_literal(root: usize, positive: bool, original_variables: usize) -> EncodedLiteral {
    if root < 2 {
        EncodedLiteral::Constant((root == 1) == positive)
    } else {
        EncodedLiteral::Variable(original_variables + root - 2, positive)
    }
}

struct LazyBddEncoding {
    nodes: Vec<BddNode>,
    rank_to_variable: Vec<usize>,
    frames: Vec<Vec<usize>>,
    encoded_nodes: Vec<bool>,
    linked_variables: Vec<bool>,
}

fn encode_bdd_cone(
    solver: &mut Solver<'_>,
    lazy: &mut LazyBddEncoding,
    variables: usize,
    root: usize,
) -> usize {
    if root < 2 || lazy.encoded_nodes[root - 2] {
        return 0;
    }
    let node = lazy.nodes[root - 2];
    let mut added = encode_bdd_cone(solver, lazy, variables, node.low);
    added += encode_bdd_cone(solver, lazy, variables, node.high);
    let output = variables + root - 2;
    let input = lazy.rank_to_variable[node.variable];
    let high = |positive| encoded_bdd_literal(node.high, positive, variables);
    let low = |positive| encoded_bdd_literal(node.low, positive, variables);
    add_encoded_clause(
        solver,
        &[
            EncodedLiteral::Variable(input, false),
            high(false),
            EncodedLiteral::Variable(output, true),
        ],
    );
    add_encoded_clause(
        solver,
        &[
            EncodedLiteral::Variable(input, false),
            high(true),
            EncodedLiteral::Variable(output, false),
        ],
    );
    add_encoded_clause(
        solver,
        &[
            EncodedLiteral::Variable(input, true),
            low(false),
            EncodedLiteral::Variable(output, true),
        ],
    );
    add_encoded_clause(
        solver,
        &[
            EncodedLiteral::Variable(input, true),
            low(true),
            EncodedLiteral::Variable(output, false),
        ],
    );
    lazy.encoded_nodes[root - 2] = true;
    added + 1
}

fn link_lazy_frame_variable(
    solver: &mut Solver<'_>,
    lazy: &mut LazyBddEncoding,
    variables: usize,
    width: usize,
    variable: usize,
) -> usize {
    if lazy.linked_variables[variable] {
        return 0;
    }
    let time = variable / width;
    let bit = variable % width;
    let root = lazy.frames[time][bit];
    let added = encode_bdd_cone(solver, lazy, variables, root);
    let literal = encoded_bdd_literal(root, true, variables);
    add_encoded_clause(
        solver,
        &[EncodedLiteral::Variable(variable, false), literal],
    );
    add_encoded_clause(
        solver,
        &[EncodedLiteral::Variable(variable, true), literal.negated()],
    );
    lazy.linked_variables[variable] = true;
    added
}

fn evaluate_bdd_root(lazy: &LazyBddEncoding, assignment: &[bool], mut root: usize) -> bool {
    while root >= 2 {
        let node = lazy.nodes[root - 2];
        let variable = lazy.rank_to_variable[node.variable];
        root = if assignment[variable] {
            node.high
        } else {
            node.low
        };
    }
    root == 1
}

struct CheckpointCdclPreimage {
    solver: Solver<'static>,
    variables: usize,
    checkpoint: usize,
    bdd_nodes: usize,
    encoding_nodes: usize,
    encoding_clauses: usize,
    width: usize,
    lazy: Option<LazyBddEncoding>,
}

impl CheckpointCdclPreimage {
    fn recognize_with_encoding(
        formula: &[Clause],
        width: usize,
        horizon: usize,
        checkpoint: usize,
        node_limit: usize,
        encoding: &str,
    ) -> Result<Self, String> {
        if checkpoint == 0 || checkpoint >= horizon {
            return Err("checkpoint must be inside the temporal horizon".to_string());
        }
        let prefix: Vec<_> = formula
            .iter()
            .filter(|clause| {
                clause
                    .0
                    .iter()
                    .map(|&(variable, _)| variable / width)
                    .min()
                    .is_some_and(|time| time < checkpoint)
            })
            .cloned()
            .collect();
        let preimage = SymbolicPreimageTransition::recognize_ordered(
            &prefix,
            width,
            checkpoint,
            node_limit,
            "dependency",
        )?;
        let variables = width * (horizon + 1);
        let mut solver = Solver::new();
        let mut lazy = None;
        let (roots, mut encoding_nodes, mut encoding_clauses) = match encoding {
            "bdd" => {
                for (index, node) in preimage.manager.nodes.iter().copied().enumerate() {
                    let output = variables + index;
                    let input = preimage.rank_to_variable[node.variable];
                    let high = |positive| encoded_bdd_literal(node.high, positive, variables);
                    let low = |positive| encoded_bdd_literal(node.low, positive, variables);
                    add_encoded_clause(
                        &mut solver,
                        &[
                            EncodedLiteral::Variable(input, false),
                            high(false),
                            EncodedLiteral::Variable(output, true),
                        ],
                    );
                    add_encoded_clause(
                        &mut solver,
                        &[
                            EncodedLiteral::Variable(input, false),
                            high(true),
                            EncodedLiteral::Variable(output, false),
                        ],
                    );
                    add_encoded_clause(
                        &mut solver,
                        &[
                            EncodedLiteral::Variable(input, true),
                            low(false),
                            EncodedLiteral::Variable(output, true),
                        ],
                    );
                    add_encoded_clause(
                        &mut solver,
                        &[
                            EncodedLiteral::Variable(input, true),
                            low(true),
                            EncodedLiteral::Variable(output, false),
                        ],
                    );
                }
                let roots = (0..=checkpoint)
                    .map(|time| {
                        let frame = preimage.frame(time);
                        frame
                            .iter()
                            .map(|&root| encoded_bdd_literal(root, true, variables))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                (roots, preimage.bdd_nodes(), preimage.bdd_nodes() * 4)
            }
            "aig" => {
                let mut builder = AigBuilder::new(variables);
                let mut literals = vec![
                    EncodedLiteral::Constant(false),
                    EncodedLiteral::Constant(true),
                ];
                for node in preimage.manager.nodes.iter().copied() {
                    let condition =
                        EncodedLiteral::Variable(preimage.rank_to_variable[node.variable], true);
                    literals.push(builder.ite(condition, literals[node.high], literals[node.low]));
                }
                let roots = (0..=checkpoint)
                    .map(|time| {
                        preimage
                            .frame(time)
                            .iter()
                            .map(|&root| literals[root])
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                for &(output, a, b) in &builder.gates {
                    let out = EncodedLiteral::Variable(output, true);
                    add_encoded_clause(&mut solver, &[a.negated(), b.negated(), out]);
                    add_encoded_clause(&mut solver, &[a, out.negated()]);
                    add_encoded_clause(&mut solver, &[b, out.negated()]);
                }
                (roots, builder.gates.len(), builder.gates.len() * 3)
            }
            "lazy-bdd" => {
                let expanded_frames = (0..=checkpoint)
                    .map(|time| preimage.frame(time).to_vec())
                    .collect::<Vec<_>>();
                let roots = expanded_frames
                    .iter()
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|&root| encoded_bdd_literal(root, true, variables))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                lazy = Some(LazyBddEncoding {
                    nodes: preimage.manager.nodes.clone(),
                    rank_to_variable: preimage.rank_to_variable.clone(),
                    frames: expanded_frames,
                    encoded_nodes: vec![false; preimage.bdd_nodes()],
                    linked_variables: vec![false; variables],
                });
                (roots, 0, 0)
            }
            _ => return Err(format!("unknown checkpoint encoding: {encoding}")),
        };
        let first_linked_frame = if encoding == "lazy-bdd" {
            checkpoint
        } else {
            1
        };
        for (time, frame) in roots
            .iter()
            .enumerate()
            .take(checkpoint + 1)
            .skip(first_linked_frame)
        {
            for (bit, &root) in frame.iter().enumerate().take(width) {
                let variable = time * width + bit;
                if let Some(lazy) = lazy.as_mut() {
                    encoding_nodes +=
                        link_lazy_frame_variable(&mut solver, lazy, variables, width, variable);
                    encoding_clauses = encoding_nodes * 4;
                } else {
                    add_encoded_clause(
                        &mut solver,
                        &[EncodedLiteral::Variable(variable, false), root],
                    );
                    add_encoded_clause(
                        &mut solver,
                        &[EncodedLiteral::Variable(variable, true), root.negated()],
                    );
                }
            }
        }
        let suffix: Vec<_> = formula
            .iter()
            .filter(|clause| {
                clause
                    .0
                    .iter()
                    .map(|&(variable, _)| variable / width)
                    .min()
                    .is_some_and(|time| time >= checkpoint)
            })
            .cloned()
            .collect();
        add_to_varisat(&mut solver, &suffix);
        Ok(Self {
            solver,
            variables,
            checkpoint,
            bdd_nodes: preimage.bdd_nodes(),
            encoding_nodes,
            encoding_clauses,
            width,
            lazy,
        })
    }

    fn query(&mut self, assumptions: &[Option<bool>]) -> Option<Vec<bool>> {
        let mut translated = Vec::new();
        if let Some(lazy) = self.lazy.as_mut() {
            for (variable, value) in assumptions.iter().enumerate() {
                let time = variable / self.width;
                if let Some(value) = value
                    && time > 0
                    && time < self.checkpoint
                {
                    let root = lazy.frames[time][variable % self.width];
                    self.encoding_nodes +=
                        encode_bdd_cone(&mut self.solver, lazy, self.variables, root);
                    translated.push(encoded_bdd_literal(root, *value, self.variables));
                }
            }
            self.encoding_clauses = self.encoding_nodes * 4;
        }
        for (variable, value) in assumptions.iter().enumerate() {
            let time = variable / self.width;
            if let Some(value) = value
                && (self.lazy.is_none() || time == 0 || time >= self.checkpoint)
            {
                translated.push(EncodedLiteral::Variable(variable, *value));
            }
        }
        if translated
            .iter()
            .any(|literal| matches!(literal, EncodedLiteral::Constant(false)))
        {
            return None;
        }
        let literals: Vec<_> = translated
            .iter()
            .filter_map(|literal| match *literal {
                EncodedLiteral::Variable(variable, positive) => {
                    Some(Lit::from_var(Var::from_index(variable), positive))
                }
                EncodedLiteral::Constant(true) => None,
                EncodedLiteral::Constant(false) => unreachable!(),
            })
            .collect();
        self.solver.assume(&literals);
        if !self.solver.solve().expect("checkpoint CDCL solve") {
            return None;
        }
        let mut assignment = vec![false; self.variables];
        for literal in self.solver.model().expect("checkpoint CDCL model") {
            if literal.var().index() < self.variables {
                assignment[literal.var().index()] = literal.is_positive();
            }
        }
        if let Some(lazy) = &self.lazy {
            for time in 1..=self.checkpoint {
                for bit in 0..self.width {
                    assignment[time * self.width + bit] =
                        evaluate_bdd_root(lazy, &assignment, lazy.frames[time][bit]);
                }
            }
        }
        Some(assignment)
    }
}

fn generalized_checkpoint_core(
    preimage: &mut SymbolicPreimageTransition,
    checkpoint: usize,
    width: usize,
    constraint: usize,
    state: &[bool],
) -> Vec<usize> {
    let literals = state
        .iter()
        .enumerate()
        .map(|(bit, &value)| {
            let root = preimage.frame(checkpoint)[bit];
            if value {
                root
            } else {
                let mut memo = HashMap::new();
                preimage.manager.negate(root, &mut memo)
            }
        })
        .collect::<Vec<_>>();
    let mut suffix = vec![1usize; width + 1];
    for bit in (0..width).rev() {
        suffix[bit] = preimage.manager.and(literals[bit], suffix[bit + 1]);
    }
    let mut prefix = constraint;
    let mut core = Vec::with_capacity(width);
    for bit in 0..width {
        let without_current = preimage.manager.and(prefix, suffix[bit + 1]);
        if without_current != 0 {
            core.push(bit);
            prefix = preimage.manager.and(prefix, literals[bit]);
        }
    }
    debug_assert_eq!(prefix, 0);
    core
}

struct NativeBddTheoryPreimage {
    solver: Solver<'static>,
    preimage: SymbolicPreimageTransition,
    variables: usize,
    width: usize,
    checkpoint: usize,
    activations: Vec<Var>,
    theory_conflicts: usize,
    theory_clauses: usize,
    learned_literals: usize,
    max_learned_width: usize,
    global_clauses: usize,
    global_literals: usize,
}

struct CqPortfolioDecision {
    specialized: bool,
    reason: &'static str,
    clauses_per_bit_step: f64,
    maximum_fanout: usize,
    assumptions_per_query: f64,
}

fn cq_portfolio_decision(
    formula: &[Clause],
    width: usize,
    horizon: usize,
    expected_queries: usize,
    assumptions_per_query: f64,
) -> Result<CqPortfolioDecision, String> {
    let clauses_per_bit_step = formula.len() as f64 / horizon.max(1) as f64 / width as f64;
    let mut maximum_fanout = 0usize;
    let (specialized, reason) = if width <= 9
        && clauses_per_bit_step >= 12.0
        && expected_queries >= 8
        && assumptions_per_query <= width as f64
    {
        (true, "dense-transition")
    } else if width <= 7 && expected_queries >= 128 && assumptions_per_query <= width as f64 {
        let functions = recover_local_transition(formula, width, horizon)?;
        let mut fanout = vec![0usize; width];
        for function in &functions {
            for &dependency in &function.dependencies {
                fanout[dependency] += 1;
            }
        }
        maximum_fanout = fanout.into_iter().max().unwrap_or(0);
        if maximum_fanout >= width.saturating_sub(1) {
            (true, "narrow-hub")
        } else {
            (false, "cdcl-fallback")
        }
    } else {
        (false, "cdcl-fallback")
    };
    Ok(CqPortfolioDecision {
        specialized,
        reason,
        clauses_per_bit_step,
        maximum_fanout,
        assumptions_per_query,
    })
}

impl NativeBddTheoryPreimage {
    fn recognize(
        formula: &[Clause],
        width: usize,
        horizon: usize,
        checkpoint: usize,
        node_limit: usize,
    ) -> Result<Self, String> {
        if checkpoint == 0 || checkpoint >= horizon {
            return Err("checkpoint must be inside the temporal horizon".to_string());
        }
        let prefix = formula
            .iter()
            .filter(|clause| {
                clause
                    .0
                    .iter()
                    .map(|&(variable, _)| variable / width)
                    .min()
                    .is_some_and(|time| time < checkpoint)
            })
            .cloned()
            .collect::<Vec<_>>();
        let preimage = SymbolicPreimageTransition::recognize_ordered(
            &prefix,
            width,
            checkpoint,
            node_limit,
            "dependency",
        )?;
        let suffix = formula
            .iter()
            .filter(|clause| {
                clause
                    .0
                    .iter()
                    .map(|&(variable, _)| variable / width)
                    .min()
                    .is_some_and(|time| time >= checkpoint)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut solver = Solver::new();
        add_to_varisat(&mut solver, &suffix);
        let mut result = Self {
            solver,
            preimage,
            variables: width * (horizon + 1),
            width,
            checkpoint,
            activations: Vec::new(),
            theory_conflicts: 0,
            theory_clauses: 0,
            learned_literals: 0,
            max_learned_width: 0,
            global_clauses: 0,
            global_literals: 0,
        };
        result.compile_global_checkpoint_clauses(4_096);
        Ok(result)
    }

    fn compile_global_checkpoint_clauses(&mut self, clause_limit: usize) {
        if self.width >= usize::BITS as usize || self.width > 16 {
            return;
        }
        let mut forbidden_cores: Vec<Vec<(usize, bool)>> = Vec::new();
        for bits in 0..(1usize << self.width) {
            if forbidden_cores.len() >= clause_limit {
                break;
            }
            let state = (0..self.width)
                .map(|bit| bits & (1usize << bit) != 0)
                .collect::<Vec<_>>();
            if forbidden_cores
                .iter()
                .any(|core| core.iter().all(|&(bit, value)| state[bit] == value))
            {
                continue;
            }
            let mut joint = 1usize;
            for (bit, &value) in state.iter().enumerate() {
                let root = self.preimage.frame(self.checkpoint)[bit];
                let literal = if value {
                    root
                } else {
                    let mut memo = HashMap::new();
                    self.preimage.manager.negate(root, &mut memo)
                };
                joint = self.preimage.manager.and(joint, literal);
                if joint == 0 {
                    break;
                }
            }
            if joint != 0 {
                continue;
            }
            let core_bits = generalized_checkpoint_core(
                &mut self.preimage,
                self.checkpoint,
                self.width,
                1,
                &state,
            );
            let core = core_bits
                .iter()
                .map(|&bit| (bit, state[bit]))
                .collect::<Vec<_>>();
            let clause = core
                .iter()
                .map(|&(bit, value)| {
                    Lit::from_var(Var::from_index(self.checkpoint * self.width + bit), !value)
                })
                .collect::<Vec<_>>();
            self.solver.add_clause(&clause);
            self.global_literals += core.len();
            self.global_clauses += 1;
            forbidden_cores.push(core);
        }
    }

    fn query(&mut self, assumptions: &[Option<bool>]) -> Option<Vec<bool>> {
        let mut constraint = 1usize;
        for (variable, required) in assumptions.iter().enumerate() {
            let Some(required) = required else { continue };
            let time = variable / self.width;
            if time > self.checkpoint {
                continue;
            }
            let root = self.preimage.frame(time)[variable % self.width];
            let literal = if *required {
                root
            } else {
                let mut memo = HashMap::new();
                self.preimage.manager.negate(root, &mut memo)
            };
            constraint = self.preimage.manager.and(constraint, literal);
            if constraint == 0 {
                return None;
            }
        }

        let activation = Var::from_index(self.variables + self.activations.len());
        let mut base_assumptions = self
            .activations
            .iter()
            .map(|&old| Lit::from_var(old, false))
            .collect::<Vec<_>>();
        base_assumptions.push(Lit::from_var(activation, true));
        self.activations.push(activation);
        for (variable, required) in assumptions.iter().enumerate() {
            if variable / self.width >= self.checkpoint
                && let Some(value) = required
            {
                base_assumptions.push(Lit::from_var(Var::from_index(variable), *value));
            }
        }

        for bit in 0..self.width {
            let root = self.preimage.frame(self.checkpoint)[bit];
            let mut memo = HashMap::new();
            let not_root = self.preimage.manager.negate(root, &mut memo);
            if self.preimage.manager.and(constraint, not_root) == 0 {
                base_assumptions.push(Lit::from_var(
                    Var::from_index(self.checkpoint * self.width + bit),
                    true,
                ));
            } else if self.preimage.manager.and(constraint, root) == 0 {
                base_assumptions.push(Lit::from_var(
                    Var::from_index(self.checkpoint * self.width + bit),
                    false,
                ));
            }
        }

        for first in 0..self.width {
            for second in (first + 1)..self.width {
                for first_value in [false, true] {
                    for second_value in [false, true] {
                        let mut candidate = constraint;
                        for (bit, value) in [(first, first_value), (second, second_value)] {
                            let root = self.preimage.frame(self.checkpoint)[bit];
                            let literal = if value {
                                root
                            } else {
                                let mut memo = HashMap::new();
                                self.preimage.manager.negate(root, &mut memo)
                            };
                            candidate = self.preimage.manager.and(candidate, literal);
                        }
                        if candidate == 0 {
                            self.solver.add_clause(&[
                                Lit::from_var(activation, false),
                                Lit::from_var(
                                    Var::from_index(self.checkpoint * self.width + first),
                                    !first_value,
                                ),
                                Lit::from_var(
                                    Var::from_index(self.checkpoint * self.width + second),
                                    !second_value,
                                ),
                            ]);
                            self.theory_clauses += 1;
                        }
                    }
                }
            }
        }

        loop {
            self.solver.assume(&base_assumptions);
            if !self.solver.solve().expect("native BDD theory solve") {
                return None;
            }
            let model = self.solver.model().expect("native BDD theory model");
            let mut assignment = vec![false; self.variables];
            for literal in model {
                if literal.var().index() < self.variables {
                    assignment[literal.var().index()] = literal.is_positive();
                }
            }
            let checkpoint_state = (0..self.width)
                .map(|bit| assignment[self.checkpoint * self.width + bit])
                .collect::<Vec<_>>();
            let mut joint = constraint;
            for (bit, &value) in checkpoint_state.iter().enumerate() {
                let root = self.preimage.frame(self.checkpoint)[bit];
                let literal = if value {
                    root
                } else {
                    let mut memo = HashMap::new();
                    self.preimage.manager.negate(root, &mut memo)
                };
                joint = self.preimage.manager.and(joint, literal);
                if joint == 0 {
                    break;
                }
            }
            if let Some(ranked) = self
                .preimage
                .manager
                .satisfying_assignment(joint, self.width)
            {
                for time in 0..=self.checkpoint {
                    for bit in 0..self.width {
                        assignment[time * self.width + bit] = self
                            .preimage
                            .manager
                            .evaluate(self.preimage.frame(time)[bit], &ranked);
                    }
                }
                return Some(assignment);
            }

            let core = generalized_checkpoint_core(
                &mut self.preimage,
                self.checkpoint,
                self.width,
                constraint,
                &checkpoint_state,
            );

            let mut block = Vec::with_capacity(core.len() + 1);
            block.push(Lit::from_var(activation, false));
            for &bit in &core {
                block.push(Lit::from_var(
                    Var::from_index(self.checkpoint * self.width + bit),
                    !checkpoint_state[bit],
                ));
            }
            self.solver.add_clause(&block);
            self.theory_conflicts += 1;
            self.theory_clauses += 1;
            self.learned_literals += core.len();
            self.max_learned_width = self.max_learned_width.max(core.len());
        }
    }
}

impl SymbolicPreimageTransition {
    fn recognize_ordered(
        formula: &[Clause],
        width: usize,
        horizon: usize,
        node_limit: usize,
        order_kind: &str,
    ) -> Result<Self, String> {
        let transition = SymbolicTemporalTransition::recognize(formula, width, horizon)?;
        let growth_guard = order_kind == "dependency-guard";
        let effective_order = if growth_guard {
            "dependency"
        } else {
            order_kind
        };
        let rank_to_variable =
            preimage_variable_order(&transition.functions, width, effective_order)?;
        let mut variable_to_rank = vec![0usize; width];
        for (rank, &variable) in rank_to_variable.iter().enumerate() {
            variable_to_rank[variable] = rank;
        }
        let mut manager = BddManager {
            node_limit: Some(node_limit),
            ..BddManager::default()
        };
        let initial: Vec<_> = (0..width)
            .map(|variable| manager.literal(variable_to_rank[variable], true))
            .collect();
        let mut frames = vec![initial];
        let mut seen = HashMap::new();
        seen.insert(frames[0].clone(), 0usize);
        let mut cycle_start = None;
        let mut cycle_length = 0usize;
        let mut previous_nodes = manager.nodes.len();
        let mut previous_increment = 0usize;
        for time in 0..horizon {
            let previous = &frames[time];
            let next: Vec<_> = transition
                .functions
                .iter()
                .map(|function| {
                    let inputs: Vec<_> = function
                        .dependencies
                        .iter()
                        .map(|&dependency| previous[dependency])
                        .collect();
                    compose_local_bdd(&mut manager, &inputs, &function.table, 0, 0)
                })
                .collect();
            if manager.budget_exceeded {
                return Err(format!("BDD node limit exceeded at frame {}", time + 1));
            }
            let current_nodes = manager.nodes.len();
            let increment = current_nodes.saturating_sub(previous_nodes);
            if growth_guard
                && time >= 2
                && increment > previous_increment
                && current_nodes.saturating_add(increment.saturating_mul(4)) > node_limit
            {
                return Err(format!(
                    "BDD growth guard projected node exhaustion at frame {} ({current_nodes} nodes)",
                    time + 1
                ));
            }
            previous_nodes = current_nodes;
            previous_increment = increment;
            if let Some(&previous_time) = seen.get(&next) {
                cycle_start = Some(previous_time);
                cycle_length = time + 1 - previous_time;
                break;
            }
            seen.insert(next.clone(), time + 1);
            frames.push(next);
        }
        manager.node_limit = None;
        Ok(Self {
            transition,
            manager,
            frames,
            rank_to_variable,
            cycle_start,
            cycle_length,
        })
    }

    fn frame(&self, time: usize) -> &[usize] {
        if time < self.frames.len() {
            return &self.frames[time];
        }
        let start = self
            .cycle_start
            .expect("time beyond compiled acyclic frames");
        &self.frames[start + (time - start) % self.cycle_length]
    }

    fn query(&mut self, assumptions: &[Option<bool>]) -> Option<Vec<bool>> {
        let mut constraint = 1usize;
        for (variable, required) in assumptions.iter().enumerate() {
            let Some(required) = required else {
                continue;
            };
            let time = variable / self.transition.width;
            let bit = variable % self.transition.width;
            let root = self.frame(time)[bit];
            let literal = if *required {
                root
            } else {
                let mut memo = HashMap::new();
                self.manager.negate(root, &mut memo)
            };
            constraint = self.manager.and(constraint, literal);
            if constraint == 0 {
                return None;
            }
        }
        let ranked = self
            .manager
            .satisfying_assignment(constraint, self.transition.width)?;
        let mut initial = vec![false; self.transition.width];
        for (rank, &variable) in self.rank_to_variable.iter().enumerate() {
            initial[variable] = ranked[rank];
        }
        let mut concrete = vec![None; assumptions.len()];
        for (bit, &value) in initial.iter().enumerate() {
            concrete[bit] = Some(value);
        }
        for (variable, required) in assumptions.iter().enumerate().skip(self.transition.width) {
            concrete[variable] = *required;
        }
        self.transition.query(&concrete)
    }

    fn bdd_nodes(&self) -> usize {
        self.manager.nodes.len()
    }

    fn compiled_frames(&self) -> usize {
        self.frames.len()
    }

    fn cycle(&self) -> Option<(usize, usize)> {
        self.cycle_start.map(|start| (start, self.cycle_length))
    }
}

fn preimage_variable_order(
    functions: &[LocalBooleanFunction],
    width: usize,
    kind: &str,
) -> Result<Vec<usize>, String> {
    match kind {
        "natural" => Ok((0..width).collect()),
        "reverse" => Ok((0..width).rev().collect()),
        "evenodd" => Ok((0..width)
            .filter(|variable| variable % 2 == 0)
            .chain((0..width).filter(|variable| variable % 2 == 1))
            .collect()),
        "dependency" => {
            let mut adjacency = vec![BTreeSet::new(); width];
            for function in functions {
                for &left in &function.dependencies {
                    for &right in &function.dependencies {
                        if left != right {
                            adjacency[left].insert(right);
                        }
                    }
                }
            }
            let start = (0..width)
                .max_by_key(|&variable| (adjacency[variable].len(), usize::MAX - variable))
                .unwrap_or(0);
            let mut order = Vec::with_capacity(width);
            let mut seen = vec![false; width];
            let mut frontier = vec![start];
            while let Some(variable) = frontier.pop() {
                if seen[variable] {
                    continue;
                }
                seen[variable] = true;
                order.push(variable);
                let mut neighbours: Vec<_> = adjacency[variable]
                    .iter()
                    .copied()
                    .filter(|&next| !seen[next])
                    .collect();
                neighbours.sort_by_key(|&next| (adjacency[next].len(), next));
                frontier.extend(neighbours.into_iter().rev());
            }
            order.extend((0..width).filter(|&variable| !seen[variable]));
            Ok(order)
        }
        _ => Err(format!("unknown preimage variable order: {kind}")),
    }
}

struct RecognizedTemporalKernel {
    width: usize,
    horizon: usize,
    jumps: Vec<Vec<usize>>,
}

impl RecognizedTemporalKernel {
    fn recognize(formula: &[Clause], width: usize, horizon: usize) -> Result<Self, String> {
        let rules = recognize_temporal_rules(formula, width, horizon)?;
        let states = 1usize << width;
        let mut base = vec![0usize; states];
        for (state, target) in base.iter_mut().enumerate() {
            for (output, rule) in rules.iter().enumerate() {
                if rule.evaluate(state) {
                    *target |= 1usize << output;
                }
            }
        }
        Ok(Self::from_base(width, horizon, base))
    }

    fn recognize_exact_composition(
        formula: &[Clause],
        width: usize,
        horizon: usize,
    ) -> Result<Self, String> {
        let steps = normalized_temporal_steps(formula, width, horizon)?;
        let template = steps
            .first()
            .ok_or_else(|| "temporal horizon must be positive".to_string())?;
        if steps.iter().skip(1).any(|step| step != template) {
            return Err("transition template is not repeated exactly".to_string());
        }
        let states = 1usize << width;
        let mut base = vec![0usize; states];
        for current in 0..states {
            let mut target = None;
            for next in 0..states {
                let satisfies = template.iter().all(|clause| {
                    clause.iter().any(|&(variable, positive)| {
                        let value = if variable < width {
                            (current >> variable) & 1 == 1
                        } else {
                            (next >> (variable - width)) & 1 == 1
                        };
                        value == positive
                    })
                });
                if satisfies {
                    if target.replace(next).is_some() {
                        return Err(format!(
                            "transition is nondeterministic for state {current}"
                        ));
                    }
                }
            }
            base[current] =
                target.ok_or_else(|| format!("transition is incomplete for state {current}"))?;
        }
        Ok(Self::from_base(width, horizon, base))
    }

    fn recognize_local_composition(
        formula: &[Clause],
        width: usize,
        horizon: usize,
    ) -> Result<Self, String> {
        let steps = normalized_temporal_steps(formula, width, horizon)?;
        let template = steps
            .first()
            .ok_or_else(|| "temporal horizon must be positive".to_string())?;
        if steps.iter().skip(1).any(|step| step != template) {
            return Err("transition template is not repeated exactly".to_string());
        }

        let mut output_clauses: Vec<Vec<&Vec<Literal>>> = vec![Vec::new(); width];
        for clause in template {
            let mut outputs: Vec<_> = clause
                .iter()
                .filter(|&&(variable, _)| variable >= width)
                .map(|&(variable, _)| variable - width)
                .collect();
            outputs.sort_unstable();
            outputs.dedup();
            if outputs.len() != 1 {
                return Err(
                    "local recovery requires every clause to constrain exactly one output"
                        .to_string(),
                );
            }
            output_clauses[outputs[0]].push(clause);
        }

        let mut recovered = Vec::with_capacity(width);
        for (output, clauses) in output_clauses.iter().enumerate() {
            if clauses.is_empty() {
                return Err(format!("output {output} has no defining clauses"));
            }
            let output_variable = width + output;
            let mut dependencies: Vec<_> = clauses
                .iter()
                .flat_map(|clause| clause.iter())
                .filter_map(|&(variable, _)| (variable < width).then_some(variable))
                .collect();
            dependencies.sort_unstable();
            dependencies.dedup();
            let mut table = Vec::with_capacity(1usize << dependencies.len());
            for pattern in 0..(1usize << dependencies.len()) {
                let valid: Vec<_> = [false, true]
                    .into_iter()
                    .filter(|&output_value| {
                        clauses.iter().all(|clause| {
                            clause.iter().any(|&(variable, positive)| {
                                let value = if variable == output_variable {
                                    output_value
                                } else {
                                    let position = dependencies
                                        .iter()
                                        .position(|&dependency| dependency == variable)
                                        .expect("local dependency");
                                    (pattern >> position) & 1 == 1
                                };
                                value == positive
                            })
                        })
                    })
                    .collect();
                if valid.len() != 1 {
                    return Err(format!(
                        "output {output} is not a total deterministic local function"
                    ));
                }
                table.push(valid[0]);
            }
            recovered.push((dependencies, table));
        }

        let states = 1usize << width;
        let mut base = vec![0usize; states];
        for (state, target) in base.iter_mut().enumerate() {
            for (output, (dependencies, table)) in recovered.iter().enumerate() {
                let mut pattern = 0usize;
                for (position, &dependency) in dependencies.iter().enumerate() {
                    pattern |= ((state >> dependency) & 1) << position;
                }
                if table[pattern] {
                    *target |= 1usize << output;
                }
            }
        }
        Ok(Self::from_base(width, horizon, base))
    }

    fn from_base(width: usize, horizon: usize, base: Vec<usize>) -> Self {
        let levels = (usize::BITS - horizon.max(1).leading_zeros()) as usize;
        let mut jumps = vec![base];
        for level in 1..levels {
            let previous = &jumps[level - 1];
            let next = previous.iter().map(|&state| previous[state]).collect();
            jumps.push(next);
        }
        Self {
            width,
            horizon,
            jumps,
        }
    }

    fn advance(&self, mut state: usize, mut steps: usize) -> usize {
        let mut level = 0usize;
        while steps > 0 {
            if steps & 1 == 1 {
                state = self.jumps[level][state];
            }
            steps >>= 1;
            level += 1;
        }
        state
    }

    fn query(&self, assumptions: &[Option<bool>]) -> Option<Vec<bool>> {
        let states = 1usize << self.width;
        let observations: Vec<_> = assumptions
            .iter()
            .enumerate()
            .filter_map(|(variable, value)| {
                value.map(|value| (variable / self.width, variable % self.width, value))
            })
            .collect();
        let initial = (0..states).find(|&initial| {
            observations
                .iter()
                .all(|&(time, bit, value)| ((self.advance(initial, time) >> bit) & 1 == 1) == value)
        })?;
        let mut assignment = Vec::with_capacity(self.width * (self.horizon + 1));
        let mut state = initial;
        for time in 0..=self.horizon {
            for bit in 0..self.width {
                assignment.push((state >> bit) & 1 == 1);
            }
            if time < self.horizon {
                state = self.jumps[0][state];
            }
        }
        Some(assignment)
    }
}

fn parse_size_grid(value: &str, name: &str) -> Result<Vec<usize>, String> {
    let values: Result<Vec<_>, _> = value
        .split(',')
        .filter(|item| !item.is_empty())
        .map(str::parse::<usize>)
        .collect();
    let values = values.map_err(|_| format!("invalid {name} grid"))?;
    if values.is_empty() || values.contains(&0) {
        return Err(format!("{name} grid must contain positive integers"));
    }
    Ok(values)
}

fn benchmark_continuation_temporal_phase(
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    max_bound_bits: usize,
    seed: u64,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create temporal output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "width,horizon,variables,clauses,queries,max_bound_bits,frontier_bound_bits,admitted,peak_classes,total_layer_states,compile_ns,quotient_ns_per_query,kernel_ns_per_query,incremental_varisat_ns_per_query,quotient_speedup_vs_incremental,kernel_speedup_vs_incremental,quotient_break_even_queries,sat_queries,unsat_queries,agreement,kernel_agreement,witnesses_valid,kernel_witnesses_valid")
        .map_err(|error| format!("write temporal header: {error}"))?;

    for &width in widths {
        for &horizon in horizons {
            let vars = width * (horizon + 1);
            let clause_count = 2 * width * horizon;
            let bound_bits = width + 1;
            if bound_bits > max_bound_bits {
                writeln!(file, "{width},{horizon},{vars},{clause_count},{query_count},{max_bound_bits},{bound_bits},false,0,0,0,0,0,0,0,0,0,0,true,true,true,true,true")
                    .map_err(|error| format!("write rejected temporal row: {error}"))?;
                continue;
            }

            let (_, formula) = temporal_memory_formula(width, horizon);
            let compile_start = Instant::now();
            let compiled = compile_temporal_memory_continuation(width, horizon);
            let compile_ns = compile_start.elapsed().as_nanos();
            let total_layer_states: usize =
                compiled.transitions.iter().map(Vec::len).sum::<usize>()
                    + compiled.terminal_sat.len();
            let mut rng = Rng(seed ^ (width as u64).rotate_left(17) ^ horizon as u64);
            let mut queries = Vec::with_capacity(query_count);
            for query_index in 0..query_count {
                let mut assumptions = vec![None; vars];
                let observed_bits = 1 + query_index % width.min(4);
                let mut chosen = BTreeSet::new();
                while chosen.len() < observed_bits {
                    chosen.insert(rng.below(width));
                }
                for bit in chosen {
                    let first_time = rng.below(horizon + 1);
                    let mut second_time = rng.below(horizon + 1);
                    if horizon > 0 && second_time == first_time {
                        second_time = (second_time + 1) % (horizon + 1);
                    }
                    assumptions[first_time * width + bit] = Some(rng.next() & 1 == 1);
                    assumptions[second_time * width + bit] = Some(rng.next() & 1 == 1);
                }
                queries.push(assumptions);
            }

            let mut scratch = ContinuationScratch::new(&compiled);
            let quotient_start = Instant::now();
            let quotient_answers: Vec<_> = queries
                .iter()
                .map(|assumptions| query_continuation(&compiled, assumptions, &mut scratch))
                .collect();
            let quotient_ns = quotient_start.elapsed().as_nanos();

            let kernel_start = Instant::now();
            let kernel_answers: Vec<_> = queries
                .iter()
                .map(|assumptions| query_temporal_memory_kernel(width, horizon, assumptions))
                .collect();
            let kernel_ns = kernel_start.elapsed().as_nanos();

            let mut solver = Solver::new();
            add_to_varisat(&mut solver, &formula);
            let varisat_start = Instant::now();
            let varisat_answers: Vec<Option<Vec<bool>>> = queries
                .iter()
                .map(|assumptions| {
                    let literals: Vec<_> = assumptions
                        .iter()
                        .enumerate()
                        .filter_map(|(variable, value)| {
                            value.map(|value| Lit::from_var(Var::from_index(variable), value))
                        })
                        .collect();
                    solver.assume(&literals);
                    if !solver.solve().expect("temporal Varisat solve") {
                        return None;
                    }
                    let mut assignment = vec![false; vars];
                    for literal in solver.model().expect("temporal Varisat model") {
                        if literal.var().index() < vars {
                            assignment[literal.var().index()] = literal.is_positive();
                        }
                    }
                    Some(assignment)
                })
                .collect();
            let varisat_ns = varisat_start.elapsed().as_nanos();

            let agreement = quotient_answers
                .iter()
                .zip(&varisat_answers)
                .all(|(left, right)| left.is_some() == right.is_some());
            let kernel_agreement = kernel_answers
                .iter()
                .zip(&varisat_answers)
                .all(|(left, right)| left.is_some() == right.is_some());
            let witnesses_valid =
                quotient_answers
                    .iter()
                    .zip(&queries)
                    .all(|(answer, assumptions)| {
                        answer.as_ref().is_none_or(|assignment| {
                            satisfies(&formula, assignment)
                                && assumptions.iter().enumerate().all(|(variable, required)| {
                                    required.is_none_or(|value| assignment[variable] == value)
                                })
                        })
                    });
            let kernel_witnesses_valid =
                kernel_answers
                    .iter()
                    .zip(&queries)
                    .all(|(answer, assumptions)| {
                        answer.as_ref().is_none_or(|assignment| {
                            satisfies(&formula, assignment)
                                && assumptions.iter().enumerate().all(|(variable, required)| {
                                    required.is_none_or(|value| assignment[variable] == value)
                                })
                        })
                    });
            let sat_queries = quotient_answers
                .iter()
                .filter(|answer| answer.is_some())
                .count();
            let unsat_queries = query_count - sat_queries;
            let quotient_per_query = quotient_ns as f64 / query_count as f64;
            let kernel_per_query = kernel_ns as f64 / query_count as f64;
            let varisat_per_query = varisat_ns as f64 / query_count as f64;
            let quotient_speedup = varisat_per_query / quotient_per_query.max(1.0);
            let kernel_speedup = varisat_per_query / kernel_per_query.max(1.0);
            let break_even = if varisat_per_query > quotient_per_query {
                (compile_ns as f64 / (varisat_per_query - quotient_per_query)).ceil() as u128
            } else {
                u128::MAX
            };
            writeln!(file, "{width},{horizon},{vars},{},{query_count},{max_bound_bits},{bound_bits},true,{},{total_layer_states},{compile_ns},{quotient_per_query:.3},{kernel_per_query:.3},{varisat_per_query:.3},{quotient_speedup:.6},{kernel_speedup:.6},{break_even},{sat_queries},{unsat_queries},{agreement},{kernel_agreement},{witnesses_valid},{kernel_witnesses_valid}", formula.len(), compiled.peak_classes)
                .map_err(|error| format!("write temporal row: {error}"))?;
            file.flush()
                .map_err(|error| format!("flush temporal output: {error}"))?;
            println!(
                "temporal phase width={width} horizon={horizon} vars={vars} admitted=true peak={} quotient_speedup={quotient_speedup:.3} kernel_speedup={kernel_speedup:.3} agreement={agreement} kernel_agreement={kernel_agreement} witnesses_valid={witnesses_valid} kernel_witnesses_valid={kernel_witnesses_valid}",
                compiled.peak_classes
            );
        }
    }
    Ok(())
}

fn benchmark_temporal_vocabulary(
    kinds: &[&str],
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    max_width: usize,
    seed: u64,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create temporal vocabulary output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,max_width,admitted,recognition_ns,jump_table_states,kernel_ns_per_query,incremental_varisat_ns_per_query,speedup_vs_incremental,break_even_queries,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write temporal vocabulary header: {error}"))?;
    for &kind in kinds {
        for &width in widths {
            for &horizon in horizons {
                let vars = width * (horizon + 1);
                if width > max_width {
                    writeln!(file, "{kind},{width},{horizon},{vars},0,{query_count},{max_width},false,0,0,0,0,0,0,0,0,true,true,width_gate")
                        .map_err(|error| format!("write vocabulary rejection: {error}"))?;
                    continue;
                }
                let (_, formula) = temporal_vocabulary_formula(kind, width, horizon)?;
                let recognition_start = Instant::now();
                let kernel = RecognizedTemporalKernel::recognize(&formula, width, horizon)?;
                let recognition_ns = recognition_start.elapsed().as_nanos();
                let jump_table_states: usize = kernel.jumps.iter().map(Vec::len).sum();
                let mut rng =
                    Rng(seed ^ (width as u64).rotate_left(13) ^ (horizon as u64).rotate_left(31));
                let mut queries = Vec::with_capacity(query_count);
                for query_index in 0..query_count {
                    let mut assumptions = vec![None; vars];
                    let observations = 2 + query_index % 5;
                    for _ in 0..observations {
                        let time = rng.below(horizon + 1);
                        let bit = rng.below(width);
                        assumptions[time * width + bit] = Some(rng.next() & 1 == 1);
                    }
                    queries.push(assumptions);
                }
                let kernel_start = Instant::now();
                let kernel_answers: Vec<_> = queries
                    .iter()
                    .map(|assumptions| kernel.query(assumptions))
                    .collect();
                let kernel_ns = kernel_start.elapsed().as_nanos();

                let mut solver = Solver::new();
                add_to_varisat(&mut solver, &formula);
                let varisat_start = Instant::now();
                let varisat_answers: Vec<Option<Vec<bool>>> = queries
                    .iter()
                    .map(|assumptions| {
                        let literals: Vec<_> = assumptions
                            .iter()
                            .enumerate()
                            .filter_map(|(variable, value)| {
                                value.map(|value| Lit::from_var(Var::from_index(variable), value))
                            })
                            .collect();
                        solver.assume(&literals);
                        if !solver.solve().expect("temporal vocabulary Varisat solve") {
                            return None;
                        }
                        let mut assignment = vec![false; vars];
                        for literal in solver.model().expect("temporal vocabulary model") {
                            if literal.var().index() < vars {
                                assignment[literal.var().index()] = literal.is_positive();
                            }
                        }
                        Some(assignment)
                    })
                    .collect();
                let varisat_ns = varisat_start.elapsed().as_nanos();
                let agreement = kernel_answers
                    .iter()
                    .zip(&varisat_answers)
                    .all(|(left, right)| left.is_some() == right.is_some());
                let witnesses_valid =
                    kernel_answers
                        .iter()
                        .zip(&queries)
                        .all(|(answer, assumptions)| {
                            answer.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, assignment)
                                    && assumptions.iter().enumerate().all(|(variable, required)| {
                                        required.is_none_or(|value| assignment[variable] == value)
                                    })
                            })
                        });
                let sat_queries = kernel_answers
                    .iter()
                    .filter(|answer| answer.is_some())
                    .count();
                let unsat_queries = query_count - sat_queries;
                let kernel_per_query = kernel_ns as f64 / query_count as f64;
                let varisat_per_query = varisat_ns as f64 / query_count as f64;
                let speedup = varisat_per_query / kernel_per_query.max(1.0);
                let break_even = if varisat_per_query > kernel_per_query {
                    (recognition_ns as f64 / (varisat_per_query - kernel_per_query)).ceil() as u128
                } else {
                    u128::MAX
                };
                writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{max_width},true,{recognition_ns},{jump_table_states},{kernel_per_query:.3},{varisat_per_query:.3},{speedup:.6},{break_even},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},ok", formula.len())
                    .map_err(|error| format!("write temporal vocabulary row: {error}"))?;
                file.flush()
                    .map_err(|error| format!("flush temporal vocabulary output: {error}"))?;
                println!(
                    "temporal vocabulary kind={kind} width={width} horizon={horizon} vars={vars} speedup={speedup:.3} agreement={agreement} witnesses_valid={witnesses_valid}"
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn benchmark_temporal_compositions(
    kinds: &[&str],
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    max_width: usize,
    seed: u64,
    output: &Path,
    local_recovery: bool,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create temporal composition output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,max_width,admitted,recognition_ns,jump_table_states,kernel_ns_per_query,incremental_varisat_ns_per_query,speedup_vs_incremental,break_even_queries,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write temporal composition header: {error}"))?;
    for &kind in kinds {
        for &width in widths {
            for &horizon in horizons {
                let vars = width * (horizon + 1);
                if width > max_width {
                    writeln!(file, "{kind},{width},{horizon},{vars},0,{query_count},{max_width},false,0,0,0,0,0,0,0,0,true,true,width_gate")
                        .map_err(|error| format!("write composition rejection: {error}"))?;
                    continue;
                }
                let (_, formula) = temporal_composition_formula(kind, width, horizon)?;
                let recognition_start = Instant::now();
                let kernel = if local_recovery {
                    RecognizedTemporalKernel::recognize_local_composition(&formula, width, horizon)?
                } else {
                    RecognizedTemporalKernel::recognize_exact_composition(&formula, width, horizon)?
                };
                let recognition_ns = recognition_start.elapsed().as_nanos();
                let jump_table_states: usize = kernel.jumps.iter().map(Vec::len).sum();
                let mut rng =
                    Rng(seed ^ (width as u64).rotate_left(11) ^ (horizon as u64).rotate_left(29));
                let mut queries = Vec::with_capacity(query_count);
                for query_index in 0..query_count {
                    let mut assumptions = vec![None; vars];
                    for _ in 0..(2 + query_index % 5) {
                        let variable = rng.below(vars);
                        assumptions[variable] = Some(rng.next() & 1 == 1);
                    }
                    queries.push(assumptions);
                }
                let kernel_start = Instant::now();
                let kernel_answers: Vec<_> = queries
                    .iter()
                    .map(|assumptions| kernel.query(assumptions))
                    .collect();
                let kernel_ns = kernel_start.elapsed().as_nanos();

                let mut solver = Solver::new();
                add_to_varisat(&mut solver, &formula);
                let varisat_start = Instant::now();
                let varisat_answers: Vec<Option<Vec<bool>>> = queries
                    .iter()
                    .map(|assumptions| {
                        let literals: Vec<_> = assumptions
                            .iter()
                            .enumerate()
                            .filter_map(|(variable, value)| {
                                value.map(|value| Lit::from_var(Var::from_index(variable), value))
                            })
                            .collect();
                        solver.assume(&literals);
                        if !solver.solve().expect("temporal composition Varisat solve") {
                            return None;
                        }
                        let mut assignment = vec![false; vars];
                        for literal in solver.model().expect("temporal composition model") {
                            if literal.var().index() < vars {
                                assignment[literal.var().index()] = literal.is_positive();
                            }
                        }
                        Some(assignment)
                    })
                    .collect();
                let varisat_ns = varisat_start.elapsed().as_nanos();
                let agreement = kernel_answers
                    .iter()
                    .zip(&varisat_answers)
                    .all(|(left, right)| left.is_some() == right.is_some());
                let witnesses_valid =
                    kernel_answers
                        .iter()
                        .zip(&queries)
                        .all(|(answer, assumptions)| {
                            answer.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, assignment)
                                    && assumptions.iter().enumerate().all(|(variable, required)| {
                                        required.is_none_or(|value| assignment[variable] == value)
                                    })
                            })
                        });
                let sat_queries = kernel_answers
                    .iter()
                    .filter(|answer| answer.is_some())
                    .count();
                let unsat_queries = query_count - sat_queries;
                let kernel_per_query = kernel_ns as f64 / query_count as f64;
                let varisat_per_query = varisat_ns as f64 / query_count as f64;
                let speedup = varisat_per_query / kernel_per_query.max(1.0);
                let break_even = if varisat_per_query > kernel_per_query {
                    (recognition_ns as f64 / (varisat_per_query - kernel_per_query)).ceil() as u128
                } else {
                    u128::MAX
                };
                writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{max_width},true,{recognition_ns},{jump_table_states},{kernel_per_query:.3},{varisat_per_query:.3},{speedup:.6},{break_even},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},ok", formula.len())
                    .map_err(|error| format!("write temporal composition row: {error}"))?;
                file.flush()
                    .map_err(|error| format!("flush temporal composition output: {error}"))?;
                println!(
                    "temporal composition recognizer={} kind={kind} width={width} horizon={horizon} vars={vars} speedup={speedup:.3} agreement={agreement} witnesses_valid={witnesses_valid}",
                    if local_recovery { "local" } else { "exact" }
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn benchmark_symbolic_temporal_compositions(
    kinds: &[&str],
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    max_width: usize,
    seed: u64,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create symbolic temporal output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,max_width,admitted,recognition_ns,representation_entries,symbolic_ns_per_query,incremental_varisat_ns_per_query,speedup_vs_incremental,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write symbolic temporal header: {error}"))?;
    for &kind in kinds {
        for &width in widths {
            for &horizon in horizons {
                let vars = width * (horizon + 1);
                if width > max_width {
                    writeln!(file, "{kind},{width},{horizon},{vars},0,{query_count},{max_width},false,0,0,0,0,0,0,0,true,true,width_gate")
                        .map_err(|error| format!("write symbolic rejection: {error}"))?;
                    continue;
                }
                let (_, formula) = temporal_composition_formula(kind, width, horizon)?;
                let recognition_start = Instant::now();
                let transition = SymbolicTemporalTransition::recognize(&formula, width, horizon)?;
                let recognition_ns = recognition_start.elapsed().as_nanos();
                let representation_entries = transition.representation_entries();
                let mut rng =
                    Rng(seed ^ (width as u64).rotate_left(17) ^ (horizon as u64).rotate_left(37));
                let mut queries = Vec::with_capacity(query_count);
                for query_index in 0..query_count {
                    let mut assumptions = vec![None; vars];
                    for value in assumptions.iter_mut().take(width) {
                        *value = Some(rng.next() & 1 == 1);
                    }
                    for _ in 0..(1 + query_index % 4) {
                        assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                    }
                    queries.push(assumptions);
                }
                let symbolic_start = Instant::now();
                let symbolic_answers: Vec<_> = queries
                    .iter()
                    .map(|assumptions| transition.query(assumptions))
                    .collect();
                let symbolic_ns = symbolic_start.elapsed().as_nanos();

                let mut solver = Solver::new();
                add_to_varisat(&mut solver, &formula);
                let varisat_start = Instant::now();
                let varisat_answers: Vec<_> = queries
                    .iter()
                    .map(|assumptions| {
                        let literals: Vec<_> = assumptions
                            .iter()
                            .enumerate()
                            .filter_map(|(variable, value)| {
                                value.map(|value| Lit::from_var(Var::from_index(variable), value))
                            })
                            .collect();
                        solver.assume(&literals);
                        solver.solve().expect("symbolic temporal Varisat solve")
                    })
                    .collect();
                let varisat_ns = varisat_start.elapsed().as_nanos();
                let agreement = symbolic_answers
                    .iter()
                    .zip(&varisat_answers)
                    .all(|(answer, &sat)| answer.is_some() == sat);
                let witnesses_valid =
                    symbolic_answers
                        .iter()
                        .zip(&queries)
                        .all(|(answer, assumptions)| {
                            answer.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, assignment)
                                    && assumptions.iter().enumerate().all(|(variable, required)| {
                                        required.is_none_or(|value| assignment[variable] == value)
                                    })
                            })
                        });
                let sat_queries = symbolic_answers
                    .iter()
                    .filter(|answer| answer.is_some())
                    .count();
                let unsat_queries = query_count - sat_queries;
                let symbolic_per_query = symbolic_ns as f64 / query_count as f64;
                let varisat_per_query = varisat_ns as f64 / query_count as f64;
                let speedup = varisat_per_query / symbolic_per_query.max(1.0);
                writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{max_width},true,{recognition_ns},{representation_entries},{symbolic_per_query:.3},{varisat_per_query:.3},{speedup:.6},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},ok", formula.len())
                    .map_err(|error| format!("write symbolic temporal row: {error}"))?;
                file.flush()
                    .map_err(|error| format!("flush symbolic temporal output: {error}"))?;
                println!(
                    "symbolic temporal kind={kind} width={width} horizon={horizon} entries={representation_entries} speedup={speedup:.3} agreement={agreement} witnesses_valid={witnesses_valid}"
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn benchmark_symbolic_preimages(
    kinds: &[&str],
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    node_limit: usize,
    seed: u64,
    output: &Path,
    order_kind: &str,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create preimage output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,node_limit,order,backend,admitted,recognition_ns,bdd_nodes,compiled_frames,cycle_start,cycle_length,preimage_ns_per_query,incremental_varisat_ns_per_query,speedup_vs_incremental,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write preimage header: {error}"))?;
    for &kind in kinds {
        for &width in widths {
            for &horizon in horizons {
                let vars = width * (horizon + 1);
                let (_, formula) = temporal_composition_formula(kind, width, horizon)?;
                let recognition_start = Instant::now();
                let engine_result = if order_kind == "hybrid" {
                    HybridTemporalPreimage::recognize(&formula, width, horizon, node_limit)
                } else {
                    SymbolicPreimageTransition::recognize_ordered(
                        &formula, width, horizon, node_limit, order_kind,
                    )
                    .map(|preimage| (HybridTemporalPreimage::Bdd(Box::new(preimage)), "bdd", None))
                };
                let (mut preimage, backend, fallback_reason) = match engine_result {
                    Ok(engine) => engine,
                    Err(error) => {
                        let recognition_ns = recognition_start.elapsed().as_nanos();
                        writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{node_limit},{order_kind},rejected,false,{recognition_ns},{node_limit},0,0,0,0,0,0,0,0,0,true,true,{}", formula.len(), error.replace(',', ";"))
                            .map_err(|write_error| format!("write preimage rejection: {write_error}"))?;
                        continue;
                    }
                };
                let recognition_ns = recognition_start.elapsed().as_nanos();
                let (bdd_nodes, compiled_frames, cycle_start, cycle_length) =
                    preimage.bdd_metrics();
                let mut rng =
                    Rng(seed ^ (width as u64).rotate_left(19) ^ (horizon as u64).rotate_left(41));
                let mut queries = Vec::with_capacity(query_count);
                for query_index in 0..query_count {
                    let mut assumptions = vec![None; vars];
                    for _ in 0..(2 + query_index % 7) {
                        assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                    }
                    queries.push(assumptions);
                }
                let preimage_start = Instant::now();
                let preimage_answers: Vec<_> = queries
                    .iter()
                    .map(|assumptions| preimage.query(assumptions))
                    .collect();
                let preimage_ns = preimage_start.elapsed().as_nanos();
                let mut solver = Solver::new();
                add_to_varisat(&mut solver, &formula);
                let varisat_start = Instant::now();
                let varisat_answers: Vec<_> = queries
                    .iter()
                    .map(|assumptions| {
                        let literals: Vec<_> = assumptions
                            .iter()
                            .enumerate()
                            .filter_map(|(variable, value)| {
                                value.map(|value| Lit::from_var(Var::from_index(variable), value))
                            })
                            .collect();
                        solver.assume(&literals);
                        solver.solve().expect("preimage Varisat solve")
                    })
                    .collect();
                let varisat_ns = varisat_start.elapsed().as_nanos();
                let agreement = preimage_answers
                    .iter()
                    .zip(&varisat_answers)
                    .all(|(answer, &sat)| answer.is_some() == sat);
                let witnesses_valid =
                    preimage_answers
                        .iter()
                        .zip(&queries)
                        .all(|(answer, assumptions)| {
                            answer.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, assignment)
                                    && assumptions.iter().enumerate().all(|(variable, required)| {
                                        required.is_none_or(|value| assignment[variable] == value)
                                    })
                            })
                        });
                let sat_queries = preimage_answers
                    .iter()
                    .filter(|answer| answer.is_some())
                    .count();
                let unsat_queries = query_count - sat_queries;
                let preimage_per_query = preimage_ns as f64 / query_count as f64;
                let varisat_per_query = varisat_ns as f64 / query_count as f64;
                let speedup = varisat_per_query / preimage_per_query.max(1.0);
                let status = fallback_reason.unwrap_or_else(|| "ok".to_string());
                writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{node_limit},{order_kind},{backend},true,{recognition_ns},{bdd_nodes},{compiled_frames},{cycle_start},{cycle_length},{preimage_per_query:.3},{varisat_per_query:.3},{speedup:.6},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},{}", formula.len(), status.replace(',', ";"))
                    .map_err(|error| format!("write preimage row: {error}"))?;
                file.flush()
                    .map_err(|error| format!("flush preimage output: {error}"))?;
                println!(
                    "symbolic preimage kind={kind} width={width} horizon={horizon} backend={backend} nodes={bdd_nodes} frames={compiled_frames} cycle={cycle_start}/{cycle_length} speedup={speedup:.3} agreement={agreement} witnesses_valid={witnesses_valid}"
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn benchmark_checkpoint_cdcl(
    kind: &str,
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    checkpoint: usize,
    node_limit: usize,
    seed: u64,
    output: &Path,
    encoding: &str,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create checkpoint output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,checkpoint,node_limit,encoding,recognition_ns,bdd_prefix_nodes,encoding_nodes,encoding_clauses,checkpoint_ns_per_query,full_cdcl_ns_per_query,speedup_vs_full,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write checkpoint header: {error}"))?;
    for &width in widths {
        for &horizon in horizons {
            let vars = width * (horizon + 1);
            let (_, formula) = temporal_composition_formula(kind, width, horizon)?;
            let recognition_start = Instant::now();
            let mut engine = CheckpointCdclPreimage::recognize_with_encoding(
                &formula,
                width,
                horizon,
                checkpoint.min(horizon.saturating_sub(1)),
                node_limit,
                encoding,
            )?;
            let recognition_ns = recognition_start.elapsed().as_nanos();
            let mut rng =
                Rng(seed ^ (width as u64).rotate_left(23) ^ (horizon as u64).rotate_left(43));
            let mut queries = Vec::with_capacity(query_count);
            for query_index in 0..query_count {
                let mut assumptions = vec![None; vars];
                for _ in 0..(2 + query_index % 7) {
                    assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                }
                queries.push(assumptions);
            }
            let checkpoint_start = Instant::now();
            let checkpoint_answers: Vec<_> = queries
                .iter()
                .map(|assumptions| engine.query(assumptions))
                .collect();
            let checkpoint_ns = checkpoint_start.elapsed().as_nanos();
            let mut solver = Solver::new();
            add_to_varisat(&mut solver, &formula);
            let full_start = Instant::now();
            let full_answers: Vec<_> = queries
                .iter()
                .map(|assumptions| {
                    let literals: Vec<_> = assumptions
                        .iter()
                        .enumerate()
                        .filter_map(|(variable, value)| {
                            value.map(|value| Lit::from_var(Var::from_index(variable), value))
                        })
                        .collect();
                    solver.assume(&literals);
                    solver.solve().expect("full checkpoint baseline solve")
                })
                .collect();
            let full_ns = full_start.elapsed().as_nanos();
            let agreement = checkpoint_answers
                .iter()
                .zip(&full_answers)
                .all(|(answer, &sat)| answer.is_some() == sat);
            let witnesses_valid =
                checkpoint_answers
                    .iter()
                    .zip(&queries)
                    .all(|(answer, assumptions)| {
                        answer.as_ref().is_none_or(|assignment| {
                            satisfies(&formula, assignment)
                                && assumptions.iter().enumerate().all(|(variable, required)| {
                                    required.is_none_or(|value| assignment[variable] == value)
                                })
                        })
                    });
            let sat_queries = checkpoint_answers
                .iter()
                .filter(|answer| answer.is_some())
                .count();
            let unsat_queries = query_count - sat_queries;
            let checkpoint_per_query = checkpoint_ns as f64 / query_count as f64;
            let full_per_query = full_ns as f64 / query_count as f64;
            let speedup = full_per_query / checkpoint_per_query.max(1.0);
            writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{},{node_limit},{encoding},{recognition_ns},{},{},{},{checkpoint_per_query:.3},{full_per_query:.3},{speedup:.6},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},ok", formula.len(), engine.checkpoint, engine.bdd_nodes, engine.encoding_nodes, engine.encoding_clauses)
                .map_err(|error| format!("write checkpoint row: {error}"))?;
            file.flush()
                .map_err(|error| format!("flush checkpoint output: {error}"))?;
            println!(
                "checkpoint CDCL kind={kind} width={width} horizon={horizon} checkpoint={} encoding={encoding} bdd_nodes={} encoding_nodes={} encoding_clauses={} speedup={speedup:.3} agreement={agreement} witnesses_valid={witnesses_valid}",
                engine.checkpoint, engine.bdd_nodes, engine.encoding_nodes, engine.encoding_clauses
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn benchmark_native_bdd_theory(
    kind: &str,
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    checkpoint: usize,
    node_limit: usize,
    seed: u64,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create theory output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,checkpoint,node_limit,recognition_ns,bdd_nodes,global_clauses,average_global_width,theory_clauses,theory_conflicts,average_learned_width,max_learned_width,theory_ns_per_query,full_cdcl_ns_per_query,speedup_vs_full,break_even_queries,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write theory header: {error}"))?;
    for &width in widths {
        for &horizon in horizons {
            let vars = width * (horizon + 1);
            let (_, formula) = temporal_composition_formula(kind, width, horizon)?;
            let effective_checkpoint = checkpoint.min(horizon.saturating_sub(1));
            let recognition_start = Instant::now();
            let mut engine = NativeBddTheoryPreimage::recognize(
                &formula,
                width,
                horizon,
                effective_checkpoint,
                node_limit,
            )?;
            let recognition_ns = recognition_start.elapsed().as_nanos();
            let bdd_nodes = engine.preimage.bdd_nodes();
            let mut rng =
                Rng(seed ^ (width as u64).rotate_left(23) ^ (horizon as u64).rotate_left(43));
            let mut queries = Vec::with_capacity(query_count);
            for query_index in 0..query_count {
                let mut assumptions = vec![None; vars];
                for _ in 0..(2 + query_index % 7) {
                    assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                }
                queries.push(assumptions);
            }
            let theory_start = Instant::now();
            let theory_answers = queries
                .iter()
                .map(|assumptions| engine.query(assumptions))
                .collect::<Vec<_>>();
            let theory_ns = theory_start.elapsed().as_nanos();
            let mut solver = Solver::new();
            add_to_varisat(&mut solver, &formula);
            let full_start = Instant::now();
            let full_answers = queries
                .iter()
                .map(|assumptions| {
                    let literals = assumptions
                        .iter()
                        .enumerate()
                        .filter_map(|(variable, value)| {
                            value.map(|value| Lit::from_var(Var::from_index(variable), value))
                        })
                        .collect::<Vec<_>>();
                    solver.assume(&literals);
                    solver.solve().expect("native theory baseline solve")
                })
                .collect::<Vec<_>>();
            let full_ns = full_start.elapsed().as_nanos();
            let agreement = theory_answers
                .iter()
                .zip(&full_answers)
                .all(|(answer, &sat)| answer.is_some() == sat);
            let witnesses_valid =
                theory_answers
                    .iter()
                    .zip(&queries)
                    .all(|(answer, assumptions)| {
                        answer.as_ref().is_none_or(|assignment| {
                            satisfies(&formula, assignment)
                                && assumptions.iter().enumerate().all(|(variable, required)| {
                                    required.is_none_or(|value| assignment[variable] == value)
                                })
                        })
                    });
            let sat_queries = theory_answers
                .iter()
                .filter(|answer| answer.is_some())
                .count();
            let unsat_queries = query_count - sat_queries;
            let theory_per_query = theory_ns as f64 / query_count as f64;
            let full_per_query = full_ns as f64 / query_count as f64;
            let speedup = full_per_query / theory_per_query.max(1.0);
            let break_even_queries = if full_per_query > theory_per_query {
                recognition_ns as f64 / (full_per_query - theory_per_query)
            } else {
                f64::INFINITY
            };
            let average_learned_width =
                engine.learned_literals as f64 / engine.theory_conflicts.max(1) as f64;
            let average_global_width =
                engine.global_literals as f64 / engine.global_clauses.max(1) as f64;
            writeln!(file, "{kind},{width},{horizon},{vars},{},{query_count},{effective_checkpoint},{node_limit},{recognition_ns},{bdd_nodes},{},{average_global_width:.3},{},{},{average_learned_width:.3},{},{theory_per_query:.3},{full_per_query:.3},{speedup:.6},{break_even_queries:.3},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},ok", formula.len(), engine.global_clauses, engine.theory_clauses, engine.theory_conflicts, engine.max_learned_width)
                .map_err(|error| format!("write theory row: {error}"))?;
            file.flush()
                .map_err(|error| format!("flush theory output: {error}"))?;
            println!(
                "native BDD theory kind={kind} width={width} horizon={horizon} checkpoint={effective_checkpoint} bdd_nodes={bdd_nodes} global_clauses={} global_width={average_global_width:.2} conflicts={} learned_width={average_learned_width:.2}/{} speedup={speedup:.3} break_even={break_even_queries:.1} agreement={agreement} witnesses_valid={witnesses_valid}",
                engine.global_clauses, engine.theory_conflicts, engine.max_learned_width
            );
        }
    }
    Ok(())
}

struct CqPortfolioRun {
    backend: &'static str,
    first_sat_query: Option<usize>,
    first_witness: Option<Vec<bool>>,
}

#[allow(clippy::too_many_arguments)]
fn write_cq_portfolio_case(
    file: &mut fs::File,
    kind: &str,
    width: usize,
    horizon: usize,
    formula: &[Clause],
    initial: &[Option<bool>],
    provided_queries: Option<Vec<Vec<Option<bool>>>>,
    query_count: usize,
    checkpoint: usize,
    node_limit: usize,
    seed: u64,
) -> Result<CqPortfolioRun, String> {
    if initial.len() != width {
        return Err("portfolio initial-state width mismatch".to_string());
    }
    let variables = width * (horizon + 1);
    let effective_checkpoint = checkpoint.min(horizon.saturating_sub(1));
    let queries = if let Some(queries) = provided_queries {
        if queries.len() != query_count
            || queries
                .iter()
                .any(|assumptions| assumptions.len() != variables)
        {
            return Err("provided portfolio query dimensions do not match".to_string());
        }
        queries
    } else {
        let mut rng = Rng(seed ^ (width as u64).rotate_left(23) ^ (horizon as u64).rotate_left(43));
        let mut queries = Vec::with_capacity(query_count);
        for query_index in 0..query_count {
            let mut assumptions = vec![None; variables];
            for _ in 0..(2 + query_index % 7) {
                assumptions[rng.below(variables)] = Some(rng.next() & 1 == 1);
            }
            assumptions[..width].copy_from_slice(initial);
            queries.push(assumptions);
        }
        queries
    };
    let gate_start = Instant::now();
    let assumptions_per_query = queries
        .iter()
        .map(|assumptions| assumptions.iter().filter(|value| value.is_some()).count())
        .sum::<usize>() as f64
        / query_count as f64;
    let decision =
        cq_portfolio_decision(formula, width, horizon, query_count, assumptions_per_query)?;
    let gate_ns = gate_start.elapsed().as_nanos();

    let mut recognition_ns = gate_ns;
    let mut specialized_recognition_ns = 0u128;
    let mut bdd_nodes = 0usize;
    let mut global_clauses = 0usize;
    let portfolio_start = Instant::now();
    let portfolio_answers = if decision.specialized {
        let recognition_start = Instant::now();
        let mut engine = NativeBddTheoryPreimage::recognize(
            formula,
            width,
            horizon,
            effective_checkpoint,
            node_limit,
        )?;
        specialized_recognition_ns = recognition_start.elapsed().as_nanos();
        recognition_ns += specialized_recognition_ns;
        bdd_nodes = engine.preimage.bdd_nodes();
        global_clauses = engine.global_clauses;
        queries
            .iter()
            .map(|assumptions| engine.query(assumptions))
            .collect::<Vec<_>>()
    } else {
        let mut solver = Solver::new();
        add_to_varisat(&mut solver, formula);
        queries
            .iter()
            .map(|assumptions| {
                let literals = assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Lit::from_var(Var::from_index(variable), value))
                    })
                    .collect::<Vec<_>>();
                solver.assume(&literals);
                if !solver.solve().expect("portfolio CDCL solve") {
                    return None;
                }
                let mut assignment = vec![false; variables];
                for literal in solver.model().expect("portfolio CDCL model") {
                    if literal.var().index() < variables {
                        assignment[literal.var().index()] = literal.is_positive();
                    }
                }
                Some(assignment)
            })
            .collect::<Vec<_>>()
    };
    let mut portfolio_ns = portfolio_start
        .elapsed()
        .as_nanos()
        .saturating_sub(specialized_recognition_ns);

    let mut baseline = Solver::new();
    add_to_varisat(&mut baseline, formula);
    let baseline_start = Instant::now();
    let baseline_answers = queries
        .iter()
        .map(|assumptions| {
            let literals = assumptions
                .iter()
                .enumerate()
                .filter_map(|(variable, value)| {
                    value.map(|value| Lit::from_var(Var::from_index(variable), value))
                })
                .collect::<Vec<_>>();
            baseline.assume(&literals);
            baseline.solve().expect("portfolio baseline solve")
        })
        .collect::<Vec<_>>();
    let baseline_ns = baseline_start.elapsed().as_nanos();
    if !decision.specialized {
        portfolio_ns = baseline_ns;
    }
    let agreement = portfolio_answers
        .iter()
        .zip(&baseline_answers)
        .all(|(answer, &sat)| answer.is_some() == sat);
    let witnesses_valid = portfolio_answers
        .iter()
        .zip(&queries)
        .all(|(answer, assumptions)| {
            answer.as_ref().is_none_or(|assignment| {
                satisfies(formula, assignment)
                    && assumptions.iter().enumerate().all(|(variable, required)| {
                        required.is_none_or(|value| assignment[variable] == value)
                    })
            })
        });
    let sat_queries = portfolio_answers
        .iter()
        .filter(|answer| answer.is_some())
        .count();
    let first_sat_query = portfolio_answers.iter().position(Option::is_some);
    let first_witness = first_sat_query
        .and_then(|index| portfolio_answers[index].as_ref())
        .cloned();
    let unsat_queries = query_count - sat_queries;
    let portfolio_per_query = portfolio_ns as f64 / query_count as f64;
    let baseline_per_query = baseline_ns as f64 / query_count as f64;
    let query_speedup = baseline_per_query / portfolio_per_query.max(1.0);
    let amortized_per_query = portfolio_per_query + recognition_ns as f64 / query_count as f64;
    let amortized_speedup = baseline_per_query / amortized_per_query.max(1.0);
    let backend = if decision.specialized {
        "cq-gcc"
    } else {
        "cdcl"
    };
    writeln!(file, "{kind},{width},{horizon},{variables},{},{query_count},{effective_checkpoint},{node_limit},{backend},{},{:.3},{},{:.3},{gate_ns},{recognition_ns},{bdd_nodes},{global_clauses},{portfolio_per_query:.3},{baseline_per_query:.3},{query_speedup:.6},{amortized_speedup:.6},{sat_queries},{unsat_queries},{agreement},{witnesses_valid},ok", formula.len(), decision.reason, decision.clauses_per_bit_step, decision.maximum_fanout, decision.assumptions_per_query)
                .map_err(|error| format!("write portfolio row: {error}"))?;
    file.flush()
        .map_err(|error| format!("flush portfolio output: {error}"))?;
    println!(
        "CQ portfolio kind={kind} width={width} horizon={horizon} backend={backend} reason={} query_speedup={query_speedup:.3} amortized_speedup={amortized_speedup:.3} agreement={agreement} witnesses_valid={witnesses_valid}",
        decision.reason
    );
    Ok(CqPortfolioRun {
        backend,
        first_sat_query,
        first_witness,
    })
}

fn create_cq_portfolio_output(output: &Path) -> Result<fs::File, String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create portfolio output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create {}: {error}", output.display()))?;
    writeln!(file, "kind,width,horizon,variables,clauses,queries,checkpoint,node_limit,backend,gate_reason,clauses_per_bit_step,maximum_fanout,assumptions_per_query,gate_ns,recognition_ns,bdd_nodes,global_clauses,portfolio_ns_per_query,full_cdcl_ns_per_query,query_speedup,amortized_speedup,sat_queries,unsat_queries,agreement,witnesses_valid,status")
        .map_err(|error| format!("write portfolio header: {error}"))?;
    Ok(file)
}

#[allow(clippy::too_many_arguments)]
fn benchmark_cq_portfolio(
    kind: &str,
    widths: &[usize],
    horizons: &[usize],
    query_count: usize,
    checkpoint: usize,
    node_limit: usize,
    seed: u64,
    output: &Path,
) -> Result<(), String> {
    let mut file = create_cq_portfolio_output(output)?;
    for &width in widths {
        for &horizon in horizons {
            let (_, formula) = temporal_composition_formula(kind, width, horizon)?;
            write_cq_portfolio_case(
                &mut file,
                kind,
                width,
                horizon,
                &formula,
                &vec![None; width],
                None,
                query_count,
                checkpoint,
                node_limit,
                seed,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn benchmark_cq_aiger(
    input: &Path,
    horizon: usize,
    query_count: usize,
    checkpoint: usize,
    node_limit: usize,
    seed: u64,
    output: &Path,
) -> Result<(), String> {
    let model = parse_aag(input)?;
    let (_, formula, initial) = aag_temporal_formula(&model, horizon)?;
    let property_queries = aag_property_queries(&model, horizon, query_count)?;
    let label = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("external.aag");
    let mut file = create_cq_portfolio_output(output)?;
    write_cq_portfolio_case(
        &mut file,
        label,
        model.latches.len(),
        horizon,
        &formula,
        &initial,
        Some(property_queries),
        query_count,
        checkpoint,
        node_limit,
        seed,
    )?;
    println!(
        "AIGER model={} latches={} outputs={} ands={}",
        input.display(),
        model.latches.len(),
        model.outputs.len(),
        model.ands.len()
    );
    Ok(())
}

fn write_aiger_safety_result(
    path: &Path,
    input: &Path,
    horizon: usize,
    width: usize,
    run: &CqPortfolioRun,
    query_metadata: &[AagPropertyQuery],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create AIGER result directory: {error}"))?;
    }
    let body = if let Some(index) = run.first_sat_query {
        let (bad_frame, bad_pattern, _) = &query_metadata[index];
        let witness = run
            .first_witness
            .as_ref()
            .ok_or_else(|| "unsafe AIGER result is missing its witness".to_string())?;
        let mut lines = vec![
            "status=UNSAFE".to_string(),
            format!("input={}", input.display()),
            format!("horizon={horizon}"),
            format!("backend={}", run.backend),
            format!("bad_frame={bad_frame}"),
            format!("bad_pattern={bad_pattern}"),
            "frame,state_bits_low_to_high".to_string(),
        ];
        for frame in 0..=horizon {
            let state = (0..width)
                .map(|bit| {
                    if witness[frame * width + bit] {
                        '1'
                    } else {
                        '0'
                    }
                })
                .collect::<String>();
            lines.push(format!("{frame},{state}"));
        }
        lines.join("\n") + "\n"
    } else {
        format!(
            "status=SAFE\ninput={}\nhorizon={horizon}\nbackend={}\n",
            input.display(),
            run.backend
        )
    };
    fs::write(path, body).map_err(|error| format!("write {}: {error}", path.display()))
}

struct AagBmcRun {
    first_sat_query: Option<usize>,
    first_witness: Option<Vec<bool>>,
    sat_queries: usize,
    witnesses_valid: bool,
    query_ns: u128,
}

fn validate_aag_trace(model: &AagModel, horizon: usize, assignment: &[bool]) -> bool {
    if assignment.len() < (horizon + 1) * model.max_variable {
        return false;
    }
    for latch in &model.latches {
        if latch
            .initial
            .is_some_and(|initial| assignment[latch.current / 2 - 1] != initial)
        {
            return false;
        }
    }
    for frame in 0..=horizon {
        let offset = frame * model.max_variable;
        let mut values = vec![false; model.max_variable + 1];
        for &literal in &model.inputs {
            values[literal / 2] = assignment[offset + literal / 2 - 1];
        }
        for latch in &model.latches {
            values[latch.current / 2] = assignment[offset + latch.current / 2 - 1];
        }
        for gate in &model.ands {
            let value = evaluate_aag_literal(gate.left, &values)
                && evaluate_aag_literal(gate.right, &values);
            if assignment[offset + gate.output / 2 - 1] != value {
                return false;
            }
            values[gate.output / 2] = value;
        }
        if frame < horizon {
            let next_offset = offset + model.max_variable;
            for latch in &model.latches {
                if assignment[next_offset + latch.current / 2 - 1]
                    != evaluate_aag_literal(latch.next, &values)
                {
                    return false;
                }
            }
        }
    }
    true
}

fn run_aag_bmc(
    model: &AagModel,
    horizon: usize,
    encoding: &AagBmcEncoding,
) -> Result<AagBmcRun, String> {
    if encoding.queries.is_empty() {
        return Ok(AagBmcRun {
            first_sat_query: None,
            first_witness: None,
            sat_queries: 0,
            witnesses_valid: true,
            query_ns: 0,
        });
    }
    let mut solver = Solver::new();
    add_to_varisat(&mut solver, &encoding.clauses);
    let aggregate_is_true = encoding
        .queries
        .iter()
        .any(|query| matches!(query.assumption, AagCnfLiteral::Constant(true)));
    if !aggregate_is_true {
        let clause = encoding
            .queries
            .iter()
            .filter_map(|query| match query.assumption {
                AagCnfLiteral::Variable((variable, positive)) => {
                    Some(Lit::from_var(Var::from_index(variable), positive))
                }
                AagCnfLiteral::Constant(_) => None,
            })
            .collect::<Vec<_>>();
        if clause.is_empty() {
            return Ok(AagBmcRun {
                first_sat_query: None,
                first_witness: None,
                sat_queries: 0,
                witnesses_valid: true,
                query_ns: 0,
            });
        }
        solver.add_clause(&clause);
    }
    let query_start = Instant::now();
    let sat = solver
        .solve()
        .map_err(|error| format!("solve aggregate AIGER BMC query: {error}"))?;
    let query_ns = query_start.elapsed().as_nanos();
    if !sat {
        return Ok(AagBmcRun {
            first_sat_query: None,
            first_witness: None,
            sat_queries: 0,
            witnesses_valid: true,
            query_ns,
        });
    }
    let mut assignment = vec![false; encoding.variables];
    for literal in solver
        .model()
        .ok_or_else(|| "AIGER BMC SAT result has no model".to_string())?
    {
        if literal.var().index() < encoding.variables {
            assignment[literal.var().index()] = literal.is_positive();
        }
    }
    let first_sat_query = encoding
        .queries
        .iter()
        .position(|query| match query.assumption {
            AagCnfLiteral::Constant(value) => value,
            AagCnfLiteral::Variable((variable, positive)) => assignment[variable] == positive,
        })
        .ok_or_else(|| "aggregate AIGER model does not satisfy a bad output".to_string())?;
    let witnesses_valid = satisfies(&encoding.clauses, &assignment)
        && validate_aag_trace(model, horizon, &assignment);
    Ok(AagBmcRun {
        first_sat_query: Some(first_sat_query),
        first_witness: Some(assignment),
        sat_queries: 1,
        witnesses_valid,
        query_ns,
    })
}

fn solve_aag_query(solver: &mut Solver, query: &AagBmcQuery) -> Result<bool, String> {
    match query.assumption {
        AagCnfLiteral::Constant(value) => Ok(value),
        AagCnfLiteral::Variable((variable, positive)) => {
            solver.assume(&[Lit::from_var(Var::from_index(variable), positive)]);
            solver
                .solve()
                .map_err(|error| format!("solve AIGER property query: {error}"))
        }
    }
}

fn aag_property_batch(
    model: &AagModel,
    horizon: usize,
    encoding: &AagBmcEncoding,
) -> (Vec<Clause>, Vec<AagBmcQuery>) {
    let mut selector_clauses = Vec::new();
    let mut property_queries = Vec::with_capacity(model.outputs.len());
    for output in 0..model.outputs.len() {
        let candidates = encoding
            .queries
            .iter()
            .filter(|query| query.output == output)
            .map(|query| query.assumption)
            .collect::<Vec<_>>();
        let assumption = if candidates
            .iter()
            .any(|candidate| matches!(candidate, AagCnfLiteral::Constant(true)))
        {
            AagCnfLiteral::Constant(true)
        } else {
            let selector = encoding.variables + output;
            let mut clause = vec![(selector, false)];
            clause.extend(candidates.iter().filter_map(|candidate| match candidate {
                AagCnfLiteral::Variable(literal) => Some(*literal),
                AagCnfLiteral::Constant(_) => None,
            }));
            if clause.len() == 1 {
                AagCnfLiteral::Constant(false)
            } else {
                selector_clauses.push(Clause(clause));
                AagCnfLiteral::Variable((selector, true))
            }
        };
        property_queries.push(AagBmcQuery {
            frame: horizon,
            output,
            assumption,
        });
    }
    (selector_clauses, property_queries)
}

fn aiger_reuse_gate(clause_count: usize, property_count: usize) -> bool {
    property_count >= 2 && clause_count <= 15_000
}

fn benchmark_aiger_query_reuse(
    input: &Path,
    horizons: &[usize],
    repeats: usize,
    output: &Path,
) -> Result<(), String> {
    if horizons.is_empty() {
        return Err("AIGER reuse benchmark requires at least one horizon".to_string());
    }
    if repeats == 0 || repeats > 10_000 {
        return Err("AIGER reuse benchmark repeats must be between 1 and 10000".to_string());
    }
    let model = parse_aag(input)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create AIGER reuse benchmark output: {error}"))?;
    }
    let mut file = fs::File::create(output)
        .map_err(|error| format!("create AIGER reuse benchmark output: {error}"))?;
    writeln!(file, "input,horizon,latches,inputs,outputs,ands,variables,clauses,distinct_queries,reuse_batch,repeats,total_queries,sat_queries,encoding_ns,reusable_build_ns,reusable_query_ns,cold_total_ns,reusable_amortized_ns,cold_amortized_ns,query_speedup,full_speedup,selected_backend,selected_ns,selected_speedup,agreement,status")
        .map_err(|error| format!("write AIGER reuse benchmark header: {error}"))?;
    for &horizon in horizons {
        if horizon == 0 {
            return Err("AIGER reuse benchmark horizons must be at least one".to_string());
        }
        let encoding_start = Instant::now();
        let encoding = aag_bmc_encoding(&model, horizon)?;
        let encoding_ns = encoding_start.elapsed().as_nanos();
        if encoding.queries.is_empty() {
            return Err(format!(
                "AIGER model has no property queries at horizon {horizon}"
            ));
        }
        let (selector_clauses, property_queries) = aag_property_batch(&model, horizon, &encoding);
        let total_queries = property_queries
            .len()
            .checked_mul(repeats)
            .ok_or_else(|| "AIGER reuse query count overflow".to_string())?;
        let mut reusable_build_ns = 0u128;
        let mut reusable_query_ns = 0u128;
        let mut reusable_answers = Vec::with_capacity(total_queries);
        for _ in 0..repeats {
            for batch in property_queries.chunks(2) {
                let build_start = Instant::now();
                let mut reusable = Solver::new();
                add_to_varisat(&mut reusable, &encoding.clauses);
                add_to_varisat(&mut reusable, &selector_clauses);
                reusable_build_ns += build_start.elapsed().as_nanos();
                let query_start = Instant::now();
                for query in batch {
                    reusable_answers.push(solve_aag_query(&mut reusable, query)?);
                }
                reusable_query_ns += query_start.elapsed().as_nanos();
            }
        }
        let cold_start = Instant::now();
        let mut cold_answers = Vec::with_capacity(total_queries);
        for _ in 0..repeats {
            for query in &property_queries {
                let mut cold = Solver::new();
                add_to_varisat(&mut cold, &encoding.clauses);
                add_to_varisat(&mut cold, &selector_clauses);
                cold_answers.push(solve_aag_query(&mut cold, query)?);
            }
        }
        let cold_total_ns = cold_start.elapsed().as_nanos();
        let agreement = reusable_answers == cold_answers;
        if !agreement {
            return Err(format!(
                "reusable and cold BMC disagree at horizon {horizon}"
            ));
        }
        let sat_queries = reusable_answers.iter().filter(|&&answer| answer).count();
        let reusable_amortized_ns = encoding_ns + reusable_build_ns + reusable_query_ns;
        let cold_amortized_ns = encoding_ns + cold_total_ns;
        let query_speedup = cold_total_ns as f64 / reusable_query_ns.max(1) as f64;
        let full_speedup = cold_amortized_ns as f64 / reusable_amortized_ns.max(1) as f64;
        let selected_reuse = aiger_reuse_gate(encoding.clauses.len(), property_queries.len());
        let (selected_backend, selected_ns) = if selected_reuse {
            ("bounded-reuse", reusable_amortized_ns)
        } else {
            ("cold-bmc", cold_amortized_ns)
        };
        let selected_speedup = cold_amortized_ns as f64 / selected_ns.max(1) as f64;
        writeln!(file, "{},{horizon},{},{},{},{},{},{},{},2,{repeats},{total_queries},{sat_queries},{encoding_ns},{reusable_build_ns},{reusable_query_ns},{cold_total_ns},{reusable_amortized_ns},{cold_amortized_ns},{query_speedup:.6},{full_speedup:.6},{selected_backend},{selected_ns},{selected_speedup:.6},{agreement},ok", report_csv_field(&input.to_string_lossy()), model.latches.len(), model.inputs.len(), model.outputs.len(), model.ands.len(), encoding.variables + property_queries.len(), encoding.clauses.len() + selector_clauses.len(), property_queries.len())
            .map_err(|error| format!("write AIGER reuse benchmark row: {error}"))?;
        println!(
            "AIGER reuse horizon={horizon} queries={total_queries} query_speedup={query_speedup:.3} full_speedup={full_speedup:.3} agreement={agreement}"
        );
    }
    file.flush()
        .map_err(|error| format!("flush AIGER reuse benchmark output: {error}"))
}

fn earliest_aag_counterexample(
    model: &AagModel,
    unsafe_horizon: usize,
    constraints: &[AagInputConstraint],
) -> Result<(AagBmcEncoding, AagBmcRun), String> {
    let mut low = 0usize;
    let mut high = unsafe_horizon;
    while low < high {
        let middle = low + (high - low) / 2;
        let encoding = aag_bmc_encoding_with_constraints(model, middle, constraints)?;
        if run_aag_bmc(model, middle, &encoding)?
            .first_sat_query
            .is_some()
        {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let encoding = aag_bmc_encoding_with_constraints(model, low, constraints)?;
    let run = run_aag_bmc(model, low, &encoding)?;
    if run.first_sat_query.is_none() {
        return Err("failed to reproduce AIGER counterexample at minimal horizon".to_string());
    }
    Ok((encoding, run))
}

fn report_csv_field(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn append_named_aag_trace(
    lines: &mut Vec<String>,
    model: &AagModel,
    witness: &[bool],
    bad_frame: usize,
) {
    let mut header = vec!["named_frame".to_string()];
    header.extend(model.latch_names.iter().map(|name| report_csv_field(name)));
    header.extend(model.input_names.iter().map(|name| report_csv_field(name)));
    lines.push(header.join(","));
    for frame in 0..=bad_frame {
        let offset = frame * model.max_variable;
        let mut row = vec![frame.to_string()];
        row.extend(
            model
                .latches
                .iter()
                .map(|latch| usize::from(witness[offset + latch.current / 2 - 1]).to_string()),
        );
        row.extend(
            model
                .inputs
                .iter()
                .map(|literal| usize::from(witness[offset + literal / 2 - 1]).to_string()),
        );
        lines.push(row.join(","));
    }
}

#[allow(clippy::too_many_arguments)]
fn write_general_aiger_safety_result(
    path: &Path,
    input: &Path,
    horizon: usize,
    model: &AagModel,
    encoding: &AagBmcEncoding,
    run: &AagBmcRun,
    gate_reason: &str,
    constraints: &[AagInputConstraint],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create AIGER result directory: {error}"))?;
    }
    let body = if let Some(index) = run.first_sat_query {
        let query = &encoding.queries[index];
        let witness = run
            .first_witness
            .as_ref()
            .ok_or_else(|| "unsafe AIGER result is missing its witness".to_string())?;
        let mut lines = vec![
            "status=UNSAFE".to_string(),
            format!("input={}", input.display()),
            format!("horizon={horizon}"),
            "backend=cdcl".to_string(),
            format!("gate_reason={gate_reason}"),
            format!("assumption_count={}", constraints.len()),
            format!("bad_frame={}", query.frame),
            format!("bad_output={}", query.output),
            format!("bad_output_name={}", model.output_names[query.output]),
        ];
        lines.extend(constraints.iter().enumerate().map(|(index, constraint)| {
            format!(
                "assumption_{index}={}={}",
                constraint.name,
                constraint.pattern.report()
            )
        }));
        lines.push("frame,latch_bits_low_to_high,input_bits_low_to_high".to_string());
        for frame in 0..=query.frame {
            let latches = model
                .latches
                .iter()
                .map(|latch| {
                    let variable = frame * model.max_variable + latch.current / 2 - 1;
                    if witness[variable] { '1' } else { '0' }
                })
                .collect::<String>();
            let inputs = model
                .inputs
                .iter()
                .map(|literal| {
                    let variable = frame * model.max_variable + literal / 2 - 1;
                    if witness[variable] { '1' } else { '0' }
                })
                .collect::<String>();
            lines.push(format!("{frame},{latches},{inputs}"));
        }
        append_named_aag_trace(&mut lines, model, witness, query.frame);
        lines.join("\n") + "\n"
    } else {
        let mut lines = vec![
            "status=SAFE".to_string(),
            format!("input={}", input.display()),
            format!("horizon={horizon}"),
            "backend=cdcl".to_string(),
            format!("gate_reason={gate_reason}"),
            format!("assumption_count={}", constraints.len()),
        ];
        lines.extend(constraints.iter().enumerate().map(|(index, constraint)| {
            format!(
                "assumption_{index}={}={}",
                constraint.name,
                constraint.pattern.report()
            )
        }));
        lines.join("\n") + "\n"
    };
    fs::write(path, body).map_err(|error| format!("write {}: {error}", path.display()))
}

#[allow(clippy::too_many_arguments)]
fn verify_general_aiger(
    input: &Path,
    model: &AagModel,
    horizon: usize,
    checkpoint: usize,
    node_limit: usize,
    output: &Path,
    safety_result: &Path,
    constraints: &[AagInputConstraint],
) -> Result<(), String> {
    let gate_start = Instant::now();
    let gate_reason = if model.inputs.is_empty() {
        "aiger-width-limit"
    } else {
        "aiger-primary-inputs"
    };
    let gate_ns = gate_start.elapsed().as_nanos();
    let encoding_start = Instant::now();
    let encoding = aag_bmc_encoding_with_constraints(model, horizon, constraints)?;
    let encoding_ns = encoding_start.elapsed().as_nanos();
    let mut run = run_aag_bmc(model, horizon, &encoding)?;
    let trace_search_start = Instant::now();
    let earliest = if run.first_sat_query.is_some() {
        Some(earliest_aag_counterexample(model, horizon, constraints)?)
    } else {
        None
    };
    run.query_ns += trace_search_start.elapsed().as_nanos();
    let query_count = usize::from(!encoding.queries.is_empty());
    let unsat_queries = query_count.saturating_sub(run.sat_queries);
    let per_query = run.query_ns as f64 / query_count.max(1) as f64;
    let amortized = per_query + (gate_ns + encoding_ns) as f64 / query_count.max(1) as f64;
    let label = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("external.aag");
    let density =
        encoding.clauses.len() as f64 / horizon.max(1) as f64 / model.latches.len().max(1) as f64;
    let assumptions_per_query = encoding
        .queries
        .iter()
        .filter(|query| matches!(query.assumption, AagCnfLiteral::Variable(_)))
        .count() as f64;
    let mut file = create_cq_portfolio_output(output)?;
    writeln!(file, "{label},{},{horizon},{},{},{query_count},{},{node_limit},cdcl,{gate_reason},{density:.3},0,{assumptions_per_query:.3},{gate_ns},{encoding_ns},0,0,{per_query:.3},{per_query:.3},1.000000,{:.6},{},{unsat_queries},true,{},ok", model.latches.len(), encoding.variables, encoding.clauses.len(), checkpoint.min(horizon.saturating_sub(1)), per_query / amortized.max(1.0), run.sat_queries, run.witnesses_valid)
        .map_err(|error| format!("write general AIGER portfolio row: {error}"))?;
    let (trace_encoding, trace_run) = earliest
        .as_ref()
        .map_or((&encoding, &run), |(encoding, run)| (encoding, run));
    write_general_aiger_safety_result(
        safety_result,
        input,
        horizon,
        model,
        trace_encoding,
        trace_run,
        gate_reason,
        constraints,
    )?;
    if let Some(index) = trace_run.first_sat_query {
        println!(
            "AIGER safety status=UNSAFE bad_frame={} bad_output={} backend=cdcl reason={gate_reason} witness={}",
            trace_encoding.queries[index].frame,
            trace_encoding.queries[index].output,
            safety_result.display()
        );
    } else {
        println!(
            "AIGER safety status=SAFE horizon={horizon} backend=cdcl reason={gate_reason} result={}",
            safety_result.display()
        );
    }
    Ok(())
}

fn verify_cq_aiger_with_constraints(
    input: &Path,
    horizon: usize,
    checkpoint: usize,
    node_limit: usize,
    output: &Path,
    safety_result: &Path,
    constraints: &[AagInputConstraint],
) -> Result<(), String> {
    let model = parse_aag(input)?;
    if !model.inputs.is_empty() || model.latches.len() > 9 {
        return verify_general_aiger(
            input,
            &model,
            horizon,
            checkpoint,
            node_limit,
            output,
            safety_result,
            constraints,
        );
    }
    if !constraints.is_empty() {
        return Err(
            "environment assumptions require an AIGER model with primary inputs".to_string(),
        );
    }
    let (_, formula, initial) = aag_temporal_formula(&model, horizon)?;
    if aag_bad_state_patterns(&model).is_empty() {
        let label = input
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("external.aag");
        let mut file = create_cq_portfolio_output(output)?;
        let width = model.latches.len();
        let variables = width * (horizon + 1);
        let density = formula.len() as f64 / horizon as f64 / width as f64;
        writeln!(file, "{label},{width},{horizon},{variables},{},0,{},{},static,constant-false-output,{density:.3},0,0.000,0,0,0,0,0.000,0.000,1.000000,1.000000,0,0,true,true,ok", formula.len(), checkpoint.min(horizon.saturating_sub(1)), node_limit)
            .map_err(|error| format!("write constant-safe AIGER row: {error}"))?;
        let run = CqPortfolioRun {
            backend: "static",
            first_sat_query: None,
            first_witness: None,
        };
        write_aiger_safety_result(safety_result, input, horizon, width, &run, &[])?;
        println!(
            "AIGER safety status=SAFE horizon={horizon} backend=static result={}",
            safety_result.display()
        );
        return Ok(());
    }
    let query_metadata = aag_property_query_space(&model, horizon)?;
    let queries = query_metadata
        .iter()
        .map(|(_, _, assumptions)| assumptions.clone())
        .collect::<Vec<_>>();
    let label = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("external.aag");
    let mut file = create_cq_portfolio_output(output)?;
    let run = write_cq_portfolio_case(
        &mut file,
        label,
        model.latches.len(),
        horizon,
        &formula,
        &initial,
        Some(queries),
        query_metadata.len(),
        checkpoint,
        node_limit,
        0,
    )?;
    write_aiger_safety_result(
        safety_result,
        input,
        horizon,
        model.latches.len(),
        &run,
        &query_metadata,
    )?;
    if let Some(index) = run.first_sat_query {
        println!(
            "AIGER safety status=UNSAFE bad_frame={} backend={} witness={}",
            query_metadata[index].0,
            run.backend,
            safety_result.display()
        );
    } else {
        println!(
            "AIGER safety status=SAFE horizon={horizon} backend={} result={}",
            run.backend,
            safety_result.display()
        );
    }
    Ok(())
}

fn verify_cq_aiger(
    input: &Path,
    horizon: usize,
    checkpoint: usize,
    node_limit: usize,
    output: &Path,
    safety_result: &Path,
) -> Result<(), String> {
    verify_cq_aiger_with_constraints(
        input,
        horizon,
        checkpoint,
        node_limit,
        output,
        safety_result,
        &[],
    )
}

fn firmware_safety_gate_with_constraints(
    input: &Path,
    horizon: usize,
    artifact_dir: &Path,
    constraints: &[AagInputConstraint],
) -> Result<bool, String> {
    fs::create_dir_all(artifact_dir)
        .map_err(|error| format!("create firmware safety artifact directory: {error}"))?;
    let metrics = artifact_dir.join("solver-metrics.csv");
    let report = artifact_dir.join("safety-report.txt");
    verify_cq_aiger_with_constraints(input, horizon, 10, 200_000, &metrics, &report, constraints)?;
    let result = fs::read_to_string(&report)
        .map_err(|error| format!("read firmware safety report: {error}"))?;
    let safe = result.lines().next() == Some("status=SAFE");
    if safe {
        println!(
            "::notice title=Firmware safety gate passed::No declared bad state is reachable through frame {horizon}"
        );
        println!(
            "firmware-safety-gate status=SAFE report={}",
            report.display()
        );
    } else if result.lines().next() == Some("status=UNSAFE") {
        let bad_frame = result
            .lines()
            .find_map(|line| line.strip_prefix("bad_frame="))
            .unwrap_or("unknown");
        println!(
            "::error title=Firmware safety gate failed::A declared bad state is reachable at frame {bad_frame}; download the firmware-safety-report artifact to replay it"
        );
        println!(
            "firmware-safety-gate status=UNSAFE bad_frame={bad_frame} report={}",
            report.display()
        );
    } else {
        return Err(format!(
            "firmware safety report has no recognized status: {}",
            report.display()
        ));
    }
    Ok(safe)
}

fn firmware_safety_gate(input: &Path, horizon: usize, artifact_dir: &Path) -> Result<bool, String> {
    firmware_safety_gate_with_constraints(input, horizon, artifact_dir, &[])
}

fn valid_verilog_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| {
            character == '_' || character == '$' || character.is_ascii_alphanumeric()
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RtlProjectConfig {
    document: Vec<u8>,
    version: usize,
    top: String,
    horizon: usize,
    sources: Vec<PathBuf>,
    include_dirs: Vec<PathBuf>,
    parameters: Vec<(String, String)>,
    clock: (String, String),
    reset: RtlResetPolicy,
    assumptions: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RtlResetPolicy {
    None,
    Deasserted {
        signal: String,
        level: bool,
    },
    Startup {
        signal: String,
        active_low: bool,
        asserted_frames: usize,
    },
}

fn rtl_reset_policy_label(policy: &RtlResetPolicy) -> String {
    match policy {
        RtlResetPolicy::None => "none".to_string(),
        RtlResetPolicy::Deasserted { signal, level } => format!(
            "{signal}:deasserted-{}",
            if *level { "high" } else { "low" }
        ),
        RtlResetPolicy::Startup {
            signal,
            active_low,
            asserted_frames,
        } => format!(
            "{signal}:active-{}:{asserted_frames}",
            if *active_low { "low" } else { "high" }
        ),
    }
}

#[derive(Debug, Clone)]
struct RtlIncludeSnapshot {
    label: String,
    files: Vec<(PathBuf, Vec<u8>)>,
}

#[derive(Debug, Clone)]
struct RtlBuildOptions {
    parameters: Vec<(String, String)>,
    clock: (String, String),
    reset: RtlResetPolicy,
    includes: Vec<RtlIncludeSnapshot>,
    project_config: Vec<u8>,
}

fn path_within(base: &Path, relative: &Path, label: &str) -> Result<PathBuf, String> {
    let resolved = fs::canonicalize(base.join(relative))
        .map_err(|error| format!("resolve {label} {}: {error}", relative.display()))?;
    if !resolved.starts_with(base) {
        return Err(format!(
            "{label} escapes the project directory: {}",
            relative.display()
        ));
    }
    Ok(resolved)
}

fn load_rtl_build_options(
    config_path: &Path,
    config: &RtlProjectConfig,
) -> Result<(Vec<PathBuf>, Option<PathBuf>, RtlBuildOptions), String> {
    let config_path = fs::canonicalize(config_path).map_err(|error| {
        format!(
            "resolve RTL project config {}: {error}",
            config_path.display()
        )
    })?;
    let base = config_path
        .parent()
        .ok_or_else(|| "RTL project config has no parent directory".to_string())?;
    let sources = config
        .sources
        .iter()
        .map(|path| path_within(base, path, "RTL source"))
        .collect::<Result<Vec<_>, _>>()?;
    let assumptions = config
        .assumptions
        .as_ref()
        .map(|path| path_within(base, path, "assumptions file"))
        .transpose()?;
    let mut total_files = 0usize;
    let mut total_bytes = 0u64;
    let mut includes = Vec::new();
    for (index, relative) in config.include_dirs.iter().enumerate() {
        let directory = path_within(base, relative, "include directory")?;
        if !directory.is_dir() {
            return Err(format!(
                "include path is not a directory: {}",
                relative.display()
            ));
        }
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("read include directory {}: {error}", relative.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("read include entry: {error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        let mut files = Vec::new();
        for entry in entries {
            let kind = entry
                .file_type()
                .map_err(|error| format!("inspect include entry: {error}"))?;
            if !kind.is_file() {
                return Err(format!(
                    "include directories may contain only regular files: {}",
                    entry.path().display()
                ));
            }
            let bytes = fs::read(entry.path()).map_err(|error| {
                format!("read include file {}: {error}", entry.path().display())
            })?;
            if bytes.len() > 1024 * 1024 {
                return Err(format!(
                    "include file exceeds 1 MiB safety limit: {}",
                    entry.path().display()
                ));
            }
            total_files += 1;
            total_bytes = total_bytes
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| "include byte count overflow".to_string())?;
            if total_files > 256 || total_bytes > 10 * 1024 * 1024 {
                return Err("include snapshot exceeds 256 files or 10 MiB safety limit".to_string());
            }
            files.push((PathBuf::from(entry.file_name()), bytes));
        }
        includes.push(RtlIncludeSnapshot {
            label: format!("include-{index:04}"),
            files,
        });
    }
    Ok((
        sources,
        assumptions,
        RtlBuildOptions {
            parameters: config.parameters.clone(),
            clock: config.clock.clone(),
            reset: config.reset.clone(),
            includes,
            project_config: config.document.clone(),
        },
    ))
}

fn safe_project_relative_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "project path must be a non-empty relative path without traversal: `{value}`"
        ));
    }
    Ok(path)
}

fn parse_rtl_project_config(path: &Path) -> Result<RtlProjectConfig, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read RTL project config {}: {error}", path.display()))?;
    if bytes.len() > 65_536 {
        return Err("RTL project config exceeds safety limit 65536 bytes".to_string());
    }
    let body = std::str::from_utf8(&bytes)
        .map_err(|_| "RTL project config must be valid UTF-8".to_string())?;
    let mut scalar = BTreeMap::new();
    let mut sources = Vec::new();
    let mut include_dirs = Vec::new();
    let mut parameters = Vec::new();
    for (index, raw) in body.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid RTL project config line {}", index + 1))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "source" => sources.push(safe_project_relative_path(value)?),
            "include_dir" => include_dirs.push(safe_project_relative_path(value)?),
            "parameter" => {
                let (name, setting) = value.split_once(':').ok_or_else(|| {
                    format!(
                        "invalid parameter at line {}: expected NAME:VALUE",
                        index + 1
                    )
                })?;
                if !valid_verilog_identifier(name)
                    || setting.is_empty()
                    || !setting
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '\''))
                {
                    return Err(format!("invalid parameter at line {}", index + 1));
                }
                parameters.push((name.to_string(), setting.to_string()));
            }
            "project_version" | "top" | "horizon" | "clock" | "reset" | "assumptions" => {
                if value.is_empty() || scalar.insert(key.to_string(), value.to_string()).is_some() {
                    return Err(format!("duplicate or empty RTL project field `{key}`"));
                }
            }
            _ => return Err(format!("unknown RTL project field `{key}`")),
        }
    }
    let version = scalar
        .get("project_version")
        .ok_or_else(|| "RTL project config is missing `project_version`".to_string())?
        .parse::<usize>()
        .map_err(|_| "RTL project version must be an integer".to_string())?;
    if !matches!(version, 1 | 2) {
        return Err("RTL project config supports project_version=1 or 2".to_string());
    }
    let top = scalar
        .remove("top")
        .ok_or_else(|| "RTL project config is missing `top`".to_string())?;
    if !valid_verilog_identifier(&top) {
        return Err("RTL project top must be a simple Verilog identifier".to_string());
    }
    let horizon = scalar
        .remove("horizon")
        .ok_or_else(|| "RTL project config is missing `horizon`".to_string())?
        .parse::<usize>()
        .map_err(|_| "RTL project horizon must be an integer".to_string())?;
    if horizon == 0 || horizon > 1_000_000 {
        return Err("RTL project horizon must be between 1 and 1000000".to_string());
    }
    if sources.is_empty() || sources.len() > 64 {
        return Err("RTL project must contain between 1 and 64 source files".to_string());
    }
    if include_dirs.len() > 16 || parameters.len() > 64 {
        return Err("RTL project include or parameter count exceeds safety limit".to_string());
    }
    let clock = scalar
        .remove("clock")
        .ok_or_else(|| "RTL project config is missing `clock`".to_string())?;
    let (clock_signal, clock_edge) = clock
        .split_once(':')
        .ok_or_else(|| "clock must be SIGNAL:posedge or SIGNAL:negedge".to_string())?;
    if !valid_verilog_identifier(clock_signal) || !matches!(clock_edge, "posedge" | "negedge") {
        return Err("clock must be SIGNAL:posedge or SIGNAL:negedge".to_string());
    }
    let reset = match scalar
        .remove("reset")
        .ok_or_else(|| "RTL project config is missing `reset`".to_string())?
        .as_str()
    {
        "none" => RtlResetPolicy::None,
        value => {
            let parts = value.split(':').collect::<Vec<_>>();
            let signal = parts.first().copied().unwrap_or_default();
            if !valid_verilog_identifier(signal) {
                return Err("reset signal is invalid".to_string());
            }
            match parts.as_slice() {
                [_, state] => match *state {
                "deasserted-low" => RtlResetPolicy::Deasserted {
                    signal: signal.to_string(),
                    level: false,
                },
                "deasserted-high" => RtlResetPolicy::Deasserted {
                    signal: signal.to_string(),
                    level: true,
                },
                    _ => return Err("reset must be none, SIGNAL:deasserted-low/deasserted-high, or SIGNAL:active-low/active-high:ASSERTED_FRAMES".to_string()),
                },
                [_, active @ ("active-low" | "active-high"), frames] => {
                    if version < 2 {
                        return Err("startup reset sequences require project_version=2".to_string());
                    }
                    let asserted_frames = frames.parse::<usize>().map_err(|_| "reset asserted frame count must be an integer".to_string())?;
                    if asserted_frames == 0 || asserted_frames > horizon {
                        return Err("reset asserted frame count must be between 1 and the horizon".to_string());
                    }
                    RtlResetPolicy::Startup {
                        signal: signal.to_string(),
                        active_low: *active == "active-low",
                        asserted_frames,
                    }
                }
                _ => return Err("reset must be none, SIGNAL:deasserted-low/deasserted-high, or SIGNAL:active-low/active-high:ASSERTED_FRAMES".to_string()),
            }
        }
    };
    let assumptions = scalar
        .remove("assumptions")
        .map(|value| safe_project_relative_path(&value))
        .transpose()?;
    Ok(RtlProjectConfig {
        document: bytes,
        version,
        top,
        horizon,
        sources,
        include_dirs,
        parameters,
        clock: (clock_signal.to_string(), clock_edge.to_string()),
        reset,
        assumptions,
    })
}

fn parse_environment_assumptions(
    path: &Path,
) -> Result<(Vec<AagInputConstraint>, Vec<u8>), String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "inspect environment assumptions {}: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "environment assumptions are not a file: {}",
            path.display()
        ));
    }
    if metadata.len() > 65_536 {
        return Err("environment assumptions exceed safety limit 65536 bytes".to_string());
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("read environment assumptions {}: {error}", path.display()))?;
    if bytes.len() > 65_536 {
        return Err("environment assumptions exceed safety limit 65536 bytes".to_string());
    }
    let body = std::str::from_utf8(&bytes)
        .map_err(|_| "environment assumptions must be valid UTF-8".to_string())?;
    let mut constraints = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, raw) in body.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').ok_or_else(|| {
            format!(
                "invalid environment assumption at line {}: expected NAME=0 or NAME=1",
                index + 1
            )
        })?;
        let name = name.trim();
        let value = value.trim();
        if !valid_verilog_identifier(name) || !matches!(value, "0" | "1") {
            return Err(format!(
                "invalid environment assumption at line {}: expected NAME=0 or NAME=1",
                index + 1
            ));
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("duplicate environment assumption `{name}`"));
        }
        constraints.push(AagInputConstraint {
            name: name.to_string(),
            pattern: AagInputConstraintPattern::Constant(value == "1"),
        });
        if constraints.len() > 256 {
            return Err("environment assumptions exceed safety limit 256 entries".to_string());
        }
    }
    if constraints.is_empty() {
        return Err("environment assumptions file contains no assumptions".to_string());
    }
    Ok((constraints, bytes))
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| format!("remove {}: {error}", destination.display()))?;
    }
    fs::rename(source, destination).map_err(|error| {
        format!(
            "publish {} as {}: {error}",
            source.display(),
            destination.display()
        )
    })
}

fn configure_contained_process(
    command: &mut Command,
    memory_limit_bytes: u64,
    file_limit_bytes: u64,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        #[cfg(not(target_os = "macos"))]
        let memory_limit = libc::rlim_t::try_from(memory_limit_bytes).map_err(|_| {
            "process memory limit is not representable on this platform".to_string()
        })?;
        #[cfg(target_os = "macos")]
        let _ = memory_limit_bytes;
        let file_limit = libc::rlim_t::try_from(file_limit_bytes)
            .map_err(|_| "process file limit is not representable on this platform".to_string())?;
        // SAFETY: pre_exec runs after fork and before exec. The closure only invokes
        // async-signal-safe libc functions and constructs errors from errno on failure.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let memory = libc::rlimit {
                        rlim_cur: memory_limit,
                        rlim_max: memory_limit,
                    };
                    if libc::setrlimit(libc::RLIMIT_AS, &memory) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                let file = libc::rlimit {
                    rlim_cur: file_limit,
                    rlim_max: file_limit,
                };
                if libc::setrlimit(libc::RLIMIT_FSIZE, &file) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (command, memory_limit_bytes, file_limit_bytes);
        Err("contained synthesis currently requires Linux or macOS".to_string())
    }
}

#[cfg(unix)]
fn kill_process_group(process_id: u32) -> Result<(), String> {
    let process_group = i32::try_from(process_id)
        .map_err(|_| "child process identifier exceeds platform range".to_string())?;
    // SAFETY: a negative PID asks kill(2) to signal the process group created by setsid.
    if unsafe { libc::kill(-process_group, libc::SIGKILL) } == -1 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(format!("terminate contained process group: {error}"));
        }
    }
    Ok(())
}

fn wait_for_contained_process(
    child: &mut Child,
    timeout: std::time::Duration,
    label: &str,
) -> Result<Option<ExitStatus>, String> {
    let process_id = child.id();
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("wait for {label}: {error}"))?
        {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            #[cfg(unix)]
            kill_process_group(process_id)?;
            child
                .wait()
                .map_err(|error| format!("reap timed-out {label}: {error}"))?;
            return Ok(None);
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn is_rtl_source_snapshot(name: &str) -> bool {
    name == "source.sv"
        || name
            .strip_prefix("source-")
            .and_then(|rest| rest.strip_suffix(".sv"))
            .is_some_and(|index| index.len() == 4 && index.chars().all(|c| c.is_ascii_digit()))
}

fn is_rtl_include_snapshot(name: &str) -> bool {
    name.strip_prefix("include-").is_some_and(|index| {
        index.len() == 4 && index.chars().all(|character| character.is_ascii_digit())
    })
}

fn source_revision() -> String {
    env::var("GITHUB_SHA")
        .ok()
        .filter(|value| {
            (7..=64).contains(&value.len())
                && value.chars().all(|character| character.is_ascii_hexdigit())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn report_value(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\n', "%0A")
        .replace('\r', "%0D")
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect evidence file {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "evidence path is not a regular file: {}",
            path.display()
        ));
    }
    let mut file = fs::File::open(path)
        .map_err(|error| format!("open evidence file {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|error| format!("hash evidence file {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn collect_evidence_files(root: &Path, directory: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("read evidence directory {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("read evidence entry: {error}"))?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("inspect evidence entry: {error}"))?;
        if kind.is_symlink() {
            return Err(format!(
                "evidence bundle contains a symlink: {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            files.extend(collect_evidence_files(root, &entry.path())?);
        } else if kind.is_file() {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| format!("evidence path escaped bundle: {}", path.display()))?;
            let relative = relative
                .to_str()
                .ok_or_else(|| "evidence paths must be valid UTF-8".to_string())?;
            if relative.contains('\n') || relative.contains('\r') {
                return Err("evidence paths may not contain line breaks".to_string());
            }
            files.push(relative.to_string());
        } else {
            return Err(format!(
                "evidence bundle contains a non-file entry: {}",
                entry.path().display()
            ));
        }
        if files.len() > 4096 {
            return Err("evidence bundle exceeds 4096 files".to_string());
        }
    }
    files.sort();
    Ok(files)
}

fn write_evidence_index(root: &Path) -> Result<String, String> {
    let index = root.join("evidence.sha256");
    let manifest = root.join("run-manifest.txt");
    if index.exists() || manifest.exists() {
        return Err("evidence index must be generated before manifest publication".to_string());
    }
    let files = collect_evidence_files(root, root)?;
    let mut body = String::new();
    for relative in files {
        let digest = sha256_file(&root.join(&relative))?;
        body.push_str(&format!("{digest}  {relative}\n"));
    }
    if body.len() > 1_048_576 {
        return Err("evidence index exceeds 1048576 bytes".to_string());
    }
    fs::write(&index, body).map_err(|error| format!("write evidence index: {error}"))?;
    sha256_file(&index)
}

fn validate_evidence_index(root: &Path, expected_index_digest: &str) -> Result<(), String> {
    if expected_index_digest.len() != 64
        || !expected_index_digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err("evidence index SHA-256 is malformed".to_string());
    }
    let index = root.join("evidence.sha256");
    let metadata =
        fs::symlink_metadata(&index).map_err(|error| format!("inspect evidence index: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err("evidence index is not a regular file".to_string());
    }
    if metadata.len() > 1_048_576 {
        return Err("evidence index exceeds 1048576 bytes".to_string());
    }
    if sha256_file(&index)? != expected_index_digest {
        return Err("evidence index SHA-256 disagrees with manifest".to_string());
    }
    let body =
        fs::read_to_string(&index).map_err(|error| format!("read evidence index: {error}"))?;
    let mut previous = None::<String>;
    let mut count = 0usize;
    let mut total_bytes = 0u64;
    for (line_number, line) in body.lines().enumerate() {
        let (digest, relative) = line
            .split_once("  ")
            .ok_or_else(|| format!("invalid evidence index line {}", line_number + 1))?;
        if digest.len() != 64
            || !digest
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(format!(
                "invalid evidence digest at line {}",
                line_number + 1
            ));
        }
        let relative_path = Path::new(relative);
        if relative.is_empty()
            || relative == "evidence.sha256"
            || relative == "run-manifest.txt"
            || relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(format!("invalid evidence path at line {}", line_number + 1));
        }
        if previous.as_deref().is_some_and(|value| value >= relative) {
            return Err("evidence index paths must be unique and sorted".to_string());
        }
        let evidence_path = root.join(relative);
        let evidence_metadata = fs::symlink_metadata(&evidence_path)
            .map_err(|error| format!("inspect evidence `{relative}`: {error}"))?;
        if !evidence_metadata.file_type().is_file() {
            return Err(format!("evidence path is not a regular file: {relative}"));
        }
        if evidence_metadata.len() > YOSYS_FILE_LIMIT_BYTES {
            return Err(format!(
                "evidence file exceeds {YOSYS_FILE_LIMIT_BYTES} bytes: {relative}"
            ));
        }
        total_bytes = total_bytes
            .checked_add(evidence_metadata.len())
            .ok_or_else(|| "evidence byte count overflow".to_string())?;
        if total_bytes > EVIDENCE_TOTAL_LIMIT_BYTES {
            return Err(format!(
                "evidence bundle exceeds {EVIDENCE_TOTAL_LIMIT_BYTES} indexed bytes"
            ));
        }
        if sha256_file(&evidence_path)? != digest {
            return Err(format!("evidence SHA-256 mismatch for `{relative}`"));
        }
        previous = Some(relative.to_string());
        count += 1;
        if count > 4096 {
            return Err("evidence index exceeds 4096 files".to_string());
        }
    }
    if count == 0 {
        return Err("evidence index contains no files".to_string());
    }
    Ok(())
}

fn read_rtl_manifest(path: &Path) -> Result<Vec<(String, String)>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect RTL artifact manifest {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() > 65_536 {
        return Err(
            "RTL artifact manifest must be a regular file no larger than 65536 bytes".to_string(),
        );
    }
    let body = fs::read_to_string(path)
        .map_err(|error| format!("read RTL artifact manifest {}: {error}", path.display()))?;
    if body.len() > 65_536 {
        return Err("RTL artifact manifest exceeds 65536 bytes".to_string());
    }
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in body.lines().enumerate() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid RTL artifact manifest line {}", index + 1))?;
        if key.is_empty()
            || !key
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric())
            || value.is_empty()
        {
            return Err(format!("invalid RTL artifact manifest line {}", index + 1));
        }
        if !seen.insert(key.to_string()) {
            return Err(format!("duplicate RTL artifact manifest field `{key}`"));
        }
        fields.push((key.to_string(), value.to_string()));
    }
    Ok(fields)
}

fn rtl_manifest_value<'a>(fields: &'a [(String, String)], key: &str) -> Result<&'a str, String> {
    fields
        .iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value.as_str()))
        .ok_or_else(|| format!("missing RTL artifact manifest field `{key}`"))
}

fn validate_rtl_artifact_bundle(artifact_dir: &Path) -> Result<(), String> {
    let root_metadata = fs::symlink_metadata(artifact_dir).map_err(|error| {
        format!(
            "inspect RTL artifact bundle {}: {error}",
            artifact_dir.display()
        )
    })?;
    if !root_metadata.file_type().is_dir() {
        return Err(format!(
            "RTL artifact bundle is not a directory: {}",
            artifact_dir.display()
        ));
    }
    let fields = read_rtl_manifest(&artifact_dir.join("run-manifest.txt"))?;
    if rtl_manifest_value(&fields, "schema_version")? != RTL_ARTIFACT_SCHEMA_VERSION.to_string() {
        return Err(format!(
            "unsupported RTL artifact schema; expected {}",
            RTL_ARTIFACT_SCHEMA_VERSION
        ));
    }
    if rtl_manifest_value(&fields, "firmware_cli_version")?
        != FIRMWARE_CLI_CONTRACT_VERSION.to_string()
    {
        return Err(format!(
            "unsupported firmware CLI contract; expected {}",
            FIRMWARE_CLI_CONTRACT_VERSION
        ));
    }
    let status = rtl_manifest_value(&fields, "status")?;
    if !matches!(status, "SAFE" | "UNSAFE") {
        return Err("RTL artifact status must be SAFE or UNSAFE".to_string());
    }
    let source_count = rtl_manifest_value(&fields, "source_count")?
        .parse::<usize>()
        .map_err(|_| "invalid RTL artifact source_count".to_string())?;
    if !(1..=64).contains(&source_count) {
        return Err("RTL artifact source_count must be between 1 and 64".to_string());
    }
    let ordered_source = (0..source_count)
        .map(|index| rtl_manifest_value(&fields, &format!("source_{index}")))
        .collect::<Result<Vec<_>, _>>()?
        .join(";");
    if rtl_manifest_value(&fields, "source")? != ordered_source {
        return Err("RTL artifact source aggregate disagrees with ordered sources".to_string());
    }
    let revision = rtl_manifest_value(&fields, "source_revision")?;
    if revision != "unknown"
        && (!(7..=64).contains(&revision.len())
            || !revision
                .chars()
                .all(|character| character.is_ascii_hexdigit()))
    {
        return Err("RTL artifact source_revision is malformed".to_string());
    }
    let assumption_count = rtl_manifest_value(&fields, "assumption_count")?
        .parse::<usize>()
        .map_err(|_| "invalid RTL artifact assumption_count".to_string())?;
    if assumption_count > 256 {
        return Err("RTL artifact assumption_count exceeds 256".to_string());
    }
    let source_bytes = rtl_manifest_value(&fields, "source_bytes")?
        .parse::<u64>()
        .map_err(|_| "invalid RTL artifact numeric field `source_bytes`".to_string())?;
    let horizon = rtl_manifest_value(&fields, "horizon")?
        .parse::<u64>()
        .map_err(|_| "invalid RTL artifact numeric field `horizon`".to_string())?;
    if horizon == 0 {
        return Err("RTL artifact horizon must be at least one".to_string());
    }
    for key in [
        "include_dir_count",
        "include_file_count",
        "include_bytes",
        "parameter_count",
        "synthesis_timeout_seconds",
        "synthesis_memory_limit_bytes",
        "synthesis_file_limit_bytes",
    ] {
        rtl_manifest_value(&fields, key)?
            .parse::<u64>()
            .map_err(|_| format!("invalid RTL artifact numeric field `{key}`"))?;
    }
    if rtl_manifest_value(&fields, "process_group_timeout_kill")? != "true" {
        return Err("RTL artifact process-group containment is not asserted".to_string());
    }
    if !valid_verilog_identifier(rtl_manifest_value(&fields, "top")?) {
        return Err("RTL artifact top is not a simple Verilog identifier".to_string());
    }
    let platform = rtl_manifest_value(&fields, "containment_platform")?;
    let memory_kind = rtl_manifest_value(&fields, "synthesis_memory_limit_kind")?;
    let memory_bytes = rtl_manifest_value(&fields, "synthesis_memory_limit_bytes")?;
    match platform {
        "linux"
            if memory_kind == "address-space"
                && memory_bytes == YOSYS_MEMORY_LIMIT_BYTES.to_string() => {}
        "macos" if memory_kind == "unavailable" && memory_bytes == "0" => {}
        _ => return Err("RTL artifact containment fields are inconsistent".to_string()),
    }
    if rtl_manifest_value(&fields, "synthesis_timeout_seconds")? != "120"
        || rtl_manifest_value(&fields, "synthesis_file_limit_bytes")?
            != YOSYS_FILE_LIMIT_BYTES.to_string()
    {
        return Err("RTL artifact synthesis limits do not match schema v4".to_string());
    }
    if !rtl_manifest_value(&fields, "yosys")?.starts_with("Yosys ") {
        return Err("RTL artifact Yosys version is malformed".to_string());
    }

    let mut expected_keys = vec![
        "status".to_string(),
        "schema_version".to_string(),
        "firmware_cli_version".to_string(),
        "source".to_string(),
        "source_count".to_string(),
    ];
    expected_keys.extend((0..source_count).map(|index| format!("source_{index}")));
    expected_keys.extend(
        [
            "source_revision",
            "source_bytes",
            "assumption_source",
            "assumption_count",
            "project_config",
            "include_dir_count",
            "include_file_count",
            "include_bytes",
            "parameter_count",
            "parameters",
            "clock_policy",
            "reset_policy",
            "top",
            "horizon",
            "synthesis_timeout_seconds",
            "containment_platform",
            "process_group_timeout_kill",
            "synthesis_memory_limit_kind",
            "synthesis_memory_limit_bytes",
            "synthesis_file_limit_bytes",
            "yosys",
            "evidence_digest_algorithm",
            "evidence_index",
            "evidence_index_sha256",
        ]
        .into_iter()
        .map(str::to_string),
    );
    let actual_keys = fields
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    if actual_keys != expected_keys {
        return Err("RTL artifact manifest fields or ordering do not match schema v4".to_string());
    }
    if rtl_manifest_value(&fields, "evidence_digest_algorithm")? != "sha256"
        || rtl_manifest_value(&fields, "evidence_index")? != "evidence.sha256"
    {
        return Err("RTL artifact evidence digest contract is invalid".to_string());
    }
    validate_evidence_index(
        artifact_dir,
        rtl_manifest_value(&fields, "evidence_index_sha256")?,
    )?;

    let expected_sources = if source_count == 1 {
        vec!["source.sv".to_string()]
    } else {
        (0..source_count)
            .map(|index| format!("source-{index:04}.sv"))
            .collect::<Vec<_>>()
    };
    for name in [
        "model.aag",
        "signal.map",
        "synthesis.ys",
        "yosys.log",
        "yosys-errors.log",
        "solver-metrics.csv",
        "safety-report.txt",
    ]
    .into_iter()
    .chain(expected_sources.iter().map(String::as_str))
    {
        if !artifact_dir.join(name).is_file() {
            return Err(format!("RTL artifact bundle is missing `{name}`"));
        }
    }
    let snapshot_bytes = expected_sources.iter().try_fold(0u64, |total, name| {
        let bytes = fs::metadata(artifact_dir.join(name))
            .map_err(|error| format!("inspect RTL source snapshot `{name}`: {error}"))?
            .len();
        total
            .checked_add(bytes)
            .ok_or_else(|| "RTL source snapshot byte count overflow".to_string())
    })?;
    if snapshot_bytes != source_bytes {
        return Err("RTL artifact source_bytes disagrees with snapshots".to_string());
    }
    let assumptions_exist = artifact_dir.join("assumptions.txt").is_file();
    if assumptions_exist != (assumption_count > 0) {
        return Err(
            "RTL artifact assumptions snapshot does not match assumption_count".to_string(),
        );
    }
    let assumption_source = rtl_manifest_value(&fields, "assumption_source")?;
    if (assumption_source == "none") != (assumption_count == 0) {
        return Err("RTL artifact assumption_source disagrees with assumption_count".to_string());
    }
    if assumptions_exist {
        let (constraints, _) =
            parse_environment_assumptions(&artifact_dir.join("assumptions.txt"))?;
        if constraints.len() != assumption_count {
            return Err("RTL artifact assumption_count disagrees with snapshot".to_string());
        }
    }
    let project_config = rtl_manifest_value(&fields, "project_config")?;
    let include_dir_count = rtl_manifest_value(&fields, "include_dir_count")?
        .parse::<usize>()
        .unwrap();
    let include_file_count = rtl_manifest_value(&fields, "include_file_count")?
        .parse::<usize>()
        .unwrap();
    let include_bytes = rtl_manifest_value(&fields, "include_bytes")?
        .parse::<u64>()
        .unwrap();
    let parameter_count = rtl_manifest_value(&fields, "parameter_count")?
        .parse::<usize>()
        .unwrap();
    if (project_config == "none")
        != (include_dir_count == 0
            && parameter_count == 0
            && rtl_manifest_value(&fields, "clock_policy")? == "unspecified"
            && rtl_manifest_value(&fields, "reset_policy")? == "unspecified")
    {
        return Err("RTL project configuration fields are inconsistent".to_string());
    }
    if project_config != "none" && project_config != "cq-project.conf" {
        return Err("RTL artifact project_config is invalid".to_string());
    }
    if (project_config == "cq-project.conf") != artifact_dir.join("cq-project.conf").is_file() {
        return Err("RTL project config snapshot is missing or unexpected".to_string());
    }
    if project_config == "cq-project.conf" {
        let config = parse_rtl_project_config(&artifact_dir.join("cq-project.conf"))?;
        let config_parameters = if config.parameters.is_empty() {
            "none".to_string()
        } else {
            config
                .parameters
                .iter()
                .map(|(name, value)| format!("{name}:{value}"))
                .collect::<Vec<_>>()
                .join(";")
        };
        let config_reset = rtl_reset_policy_label(&config.reset);
        if config.top != rtl_manifest_value(&fields, "top")?
            || config.horizon.to_string() != rtl_manifest_value(&fields, "horizon")?
            || config.sources.len() != source_count
            || config.include_dirs.len() != include_dir_count
            || config.parameters.len() != parameter_count
            || config_parameters != rtl_manifest_value(&fields, "parameters")?
            || format!("{}:{}", config.clock.0, config.clock.1)
                != rtl_manifest_value(&fields, "clock_policy")?
            || config_reset != rtl_manifest_value(&fields, "reset_policy")?
            || config.assumptions.is_some() != (assumption_count > 0)
        {
            return Err("RTL project config snapshot disagrees with manifest".to_string());
        }
    }
    let mut actual_include_files = 0usize;
    let mut actual_include_bytes = 0u64;
    for index in 0..include_dir_count {
        let directory = artifact_dir.join(format!("include-{index:04}"));
        if !directory.is_dir() {
            return Err("RTL include snapshot directory is missing".to_string());
        }
        for entry in
            fs::read_dir(directory).map_err(|error| format!("read include snapshot: {error}"))?
        {
            let entry = entry.map_err(|error| format!("read include snapshot entry: {error}"))?;
            if !entry
                .file_type()
                .map_err(|error| format!("inspect include snapshot: {error}"))?
                .is_file()
            {
                return Err("RTL include snapshot contains a non-file".to_string());
            }
            actual_include_files += 1;
            actual_include_bytes += entry
                .metadata()
                .map_err(|error| format!("inspect include snapshot: {error}"))?
                .len();
        }
    }
    if actual_include_files != include_file_count || actual_include_bytes != include_bytes {
        return Err("RTL include snapshot counts disagree with manifest".to_string());
    }
    let actual_include_dirs = fs::read_dir(artifact_dir)
        .map_err(|error| format!("inspect RTL artifact includes: {error}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && is_rtl_include_snapshot(&entry.file_name().to_string_lossy())
        })
        .count();
    if actual_include_dirs != include_dir_count {
        return Err("RTL include snapshot directories disagree with manifest".to_string());
    }
    let parameters = rtl_manifest_value(&fields, "parameters")?;
    if (parameters == "none") != (parameter_count == 0)
        || (parameter_count > 0 && parameters.split(';').count() != parameter_count)
    {
        return Err("RTL parameter count disagrees with manifest".to_string());
    }
    let mut actual_sources = Vec::new();
    for entry in fs::read_dir(artifact_dir)
        .map_err(|error| format!("inspect RTL artifact bundle: {error}"))?
    {
        let entry = entry.map_err(|error| format!("inspect RTL artifact entry: {error}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if is_rtl_source_snapshot(&name) {
            actual_sources.push(name);
        }
    }
    actual_sources.sort();
    if actual_sources != expected_sources {
        return Err("RTL artifact source snapshots do not match source_count".to_string());
    }

    let report = fs::File::open(artifact_dir.join("safety-report.txt"))
        .map_err(|error| format!("open RTL safety report: {error}"))?;
    let mut lines = BufReader::new(report).lines();
    if lines
        .next()
        .transpose()
        .map_err(|error| format!("read RTL safety status: {error}"))?
        .as_deref()
        != Some(&format!("status={status}"))
    {
        return Err("RTL safety report status disagrees with manifest".to_string());
    }
    if lines
        .next()
        .transpose()
        .map_err(|error| format!("read RTL safety schema: {error}"))?
        .as_deref()
        != Some(&format!("schema_version={RTL_ARTIFACT_SCHEMA_VERSION}"))
    {
        return Err("RTL safety report schema disagrees with manifest".to_string());
    }
    if lines
        .next()
        .transpose()
        .map_err(|error| format!("read RTL safety CLI contract: {error}"))?
        .as_deref()
        != Some(&format!(
            "firmware_cli_version={FIRMWARE_CLI_CONTRACT_VERSION}"
        ))
    {
        return Err("RTL safety report CLI contract disagrees with manifest".to_string());
    }
    println!(
        "firmware-artifact-validate status=VALID schema={} result={status} bundle={}",
        RTL_ARTIFACT_SCHEMA_VERSION,
        artifact_dir.display()
    );
    Ok(())
}

fn annotate_rtl_safety_report(
    report: &Path,
    source: &str,
    top: &str,
    yosys_version: &str,
    build_options: Option<&RtlBuildOptions>,
) -> Result<(), String> {
    let body =
        fs::read_to_string(report).map_err(|error| format!("read RTL safety report: {error}"))?;
    let revision = source_revision();
    let mut lines = body.lines();
    let status = lines
        .next()
        .ok_or_else(|| "RTL safety report is empty".to_string())?;
    let mut annotated = vec![
        status.to_string(),
        format!("schema_version={RTL_ARTIFACT_SCHEMA_VERSION}"),
        format!("firmware_cli_version={FIRMWARE_CLI_CONTRACT_VERSION}"),
        format!("source={}", report_value(source)),
        format!("source_revision={revision}"),
        format!("top={top}"),
        format!(
            "clock_policy={}",
            build_options.map_or_else(
                || "unspecified".to_string(),
                |options| format!("{}:{}", options.clock.0, options.clock.1)
            )
        ),
        format!(
            "reset_policy={}",
            build_options.map_or_else(
                || "unspecified".to_string(),
                |options| rtl_reset_policy_label(&options.reset)
            )
        ),
        format!("yosys={yosys_version}"),
        format!("containment_platform={}", std::env::consts::OS),
        "process_group_timeout_kill=true".to_string(),
        format!(
            "synthesis_memory_limit_kind={}",
            synthesis_memory_limit_kind()
        ),
        format!(
            "synthesis_memory_limit_bytes={}",
            synthesis_memory_limit_bytes()
        ),
        format!("synthesis_file_limit_bytes={YOSYS_FILE_LIMIT_BYTES}"),
        "generated_model=model.aag".to_string(),
    ];
    annotated.extend(
        lines
            .filter(|line| !line.starts_with("input="))
            .map(str::to_string),
    );
    fs::write(report, annotated.join("\n") + "\n")
        .map_err(|error| format!("write annotated RTL safety report: {error}"))
}

fn firmware_rtl_project_safety_gate_with_assumptions(
    inputs: &[PathBuf],
    top: &str,
    horizon: usize,
    artifact_dir: &Path,
    assumptions_path: Option<&Path>,
    build_options: Option<&RtlBuildOptions>,
) -> Result<bool, String> {
    if !valid_verilog_identifier(top) {
        return Err("RTL top must be a simple Verilog identifier".to_string());
    }
    if inputs.is_empty() || inputs.len() > 64 {
        return Err("RTL project must contain between 1 and 64 source files".to_string());
    }
    let assumption_document = assumptions_path
        .map(parse_environment_assumptions)
        .transpose()?;
    let mut effective_assumptions = assumption_document
        .as_ref()
        .map_or_else(Vec::new, |(constraints, _)| constraints.clone());
    let reset_constraint = build_options.and_then(|options| match &options.reset {
        RtlResetPolicy::None => None,
        RtlResetPolicy::Deasserted { signal, level } => Some(AagInputConstraint {
            name: signal.clone(),
            pattern: AagInputConstraintPattern::Constant(*level),
        }),
        RtlResetPolicy::Startup {
            signal,
            active_low,
            asserted_frames,
        } => Some(AagInputConstraint {
            name: signal.clone(),
            pattern: AagInputConstraintPattern::StartupReset {
                asserted_frames: *asserted_frames,
                asserted_value: !*active_low,
            },
        }),
    });
    if let Some(reset_constraint) = reset_constraint {
        if effective_assumptions
            .iter()
            .any(|constraint| constraint.name == reset_constraint.name.as_str())
        {
            return Err(format!(
                "reset signal `{}` duplicates an environment assumption",
                reset_constraint.name
            ));
        }
        effective_assumptions.push(reset_constraint);
    }
    let assumptions = effective_assumptions.as_slice();
    let mut sources = Vec::with_capacity(inputs.len());
    let mut seen_sources = BTreeSet::new();
    let mut source_bytes = 0u64;
    for input in inputs {
        let source = fs::canonicalize(input)
            .map_err(|error| format!("resolve RTL source {}: {error}", input.display()))?;
        if !source.is_file() {
            return Err(format!("RTL source is not a file: {}", source.display()));
        }
        if !seen_sources.insert(source.clone()) {
            return Err(format!("duplicate RTL source: {}", input.display()));
        }
        let metadata_bytes = fs::metadata(&source)
            .map_err(|error| format!("inspect RTL source {}: {error}", source.display()))?
            .len();
        if metadata_bytes > 10 * 1024 * 1024 {
            return Err(format!(
                "RTL source {} is {metadata_bytes} bytes; per-file safety limit is 10485760",
                input.display()
            ));
        }
        let bytes = fs::read(&source)
            .map_err(|error| format!("read RTL source {}: {error}", source.display()))?;
        if bytes.len() > 10 * 1024 * 1024 {
            return Err(format!(
                "RTL source {} exceeds per-file safety limit 10485760",
                input.display()
            ));
        }
        source_bytes = source_bytes
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| "RTL project source byte count overflow".to_string())?;
        sources.push((source, bytes));
    }
    if source_bytes > 25 * 1024 * 1024 {
        return Err(format!(
            "RTL project is {source_bytes} bytes; total safety limit is 26214400"
        ));
    }
    let source_labels = inputs
        .iter()
        .map(|input| input.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let source_label = source_labels.join(";");
    fs::create_dir_all(artifact_dir)
        .map_err(|error| format!("create RTL safety artifact directory: {error}"))?;
    let artifact_metadata = fs::symlink_metadata(artifact_dir)
        .map_err(|error| format!("inspect RTL safety artifact directory: {error}"))?;
    if !artifact_metadata.file_type().is_dir() {
        return Err("RTL safety artifact path must be a real directory, not a symlink".to_string());
    }
    for entry in fs::read_dir(artifact_dir)
        .map_err(|error| format!("inspect RTL safety artifact directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("inspect RTL safety artifact entry: {error}"))?;
        if entry
            .file_type()
            .map_err(|error| format!("inspect RTL safety artifact entry: {error}"))?
            .is_symlink()
        {
            return Err(format!(
                "RTL safety artifact directory contains a symlink: {}",
                entry.path().display()
            ));
        }
    }
    let stage = artifact_dir.join(format!(".rtl-stage-{}", std::process::id()));
    fs::create_dir(&stage)
        .map_err(|error| format!("create RTL safety staging directory: {error}"))?;
    let stage = fs::canonicalize(&stage)
        .map_err(|error| format!("resolve RTL safety staging directory: {error}"))?;
    let model = stage.join("model.aag");
    let synthesis = stage.join("synthesis.ys");
    let yosys_log = stage.join("yosys.log");
    let yosys_errors = stage.join("yosys-errors.log");
    let staged_source_names = sources
        .iter()
        .enumerate()
        .map(|(index, (source, bytes))| {
            let name = if sources.len() == 1 {
                "source.sv".to_string()
            } else {
                format!("source-{index:04}.sv")
            };
            fs::write(stage.join(&name), bytes)
                .map_err(|error| format!("stage RTL source {}: {error}", source.display()))?;
            Ok(name)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let staged_sources = staged_source_names.join(" ");
    let mut include_flags = Vec::new();
    if let Some(options) = build_options {
        fs::write(stage.join("cq-project.conf"), &options.project_config)
            .map_err(|error| format!("stage RTL project config: {error}"))?;
        for include in &options.includes {
            let directory = stage.join(&include.label);
            fs::create_dir(&directory)
                .map_err(|error| format!("stage include directory: {error}"))?;
            for (name, bytes) in &include.files {
                fs::write(directory.join(name), bytes)
                    .map_err(|error| format!("stage include file {}: {error}", name.display()))?;
            }
            include_flags.push(format!("-I{}", include.label));
        }
    }
    if let Some((_, bytes)) = &assumption_document {
        fs::write(stage.join("assumptions.txt"), bytes)
            .map_err(|error| format!("stage environment assumptions: {error}"))?;
    }
    let parameter_commands = build_options.map_or_else(String::new, |options| {
        options
            .parameters
            .iter()
            .map(|(name, value)| format!("chparam -set {name} {value} {top}\n"))
            .collect()
    });
    let clock_check = build_options.map_or_else(String::new, |options| {
        format!("select -assert-count 1 {top}/{}\n", options.clock.0)
    });
    let includes = include_flags.join(" ");
    let script = format!(
        "read_verilog -formal -sv -D CQ_AIGER_EXPORT {includes} {staged_sources}\n{parameter_commands}prep -top {top}\n{clock_check}flatten\nasync2sync\nopt\nmemory_map\nopt\ntechmap\nopt\ndffunmap\npmuxtree\nsimplemap\ndffunmap\naigmap\nsetundef -zero\nwrite_aiger -ascii -symbols -map signal.map model.aag\n"
    );
    fs::write(&synthesis, script)
        .map_err(|error| format!("write Yosys synthesis script: {error}"))?;
    let error_file = fs::File::create(&yosys_errors)
        .map_err(|error| format!("create Yosys error log: {error}"))?;
    let mut yosys_command = Command::new("yosys");
    yosys_command
        .arg("-l")
        .arg("yosys.log")
        .arg("-s")
        .arg("synthesis.ys")
        .current_dir(&stage)
        .stdout(Stdio::null())
        .stderr(Stdio::from(error_file));
    configure_contained_process(
        &mut yosys_command,
        YOSYS_MEMORY_LIMIT_BYTES,
        YOSYS_FILE_LIMIT_BYTES,
    )?;
    let mut child = yosys_command
        .spawn()
        .map_err(|error| format!("run Yosys synthesis: {error}"))?;
    let Some(status) = wait_for_contained_process(
        &mut child,
        std::time::Duration::from_secs(120),
        "Yosys synthesis",
    )?
    else {
        return Err(format!(
            "Yosys synthesis exceeded 120 seconds; inspect {} and {}",
            yosys_log.display(),
            yosys_errors.display()
        ));
    };
    if !status.success() {
        let error_tail = fs::read_to_string(&yosys_errors)
            .ok()
            .and_then(|body| body.lines().last().map(str::to_string))
            .filter(|line| !line.is_empty())
            .or_else(|| {
                fs::read_to_string(&yosys_log)
                    .ok()
                    .and_then(|body| body.lines().last().map(str::to_string))
                    .filter(|line| !line.is_empty())
            })
            .unwrap_or_else(|| "no Yosys diagnostic was captured".to_string());
        return Err(format!(
            "Yosys synthesis failed: {error_tail}; inspect {} and {}",
            yosys_log.display(),
            yosys_errors.display()
        ));
    }
    let yosys_version = fs::read_to_string(&yosys_log)
        .map_err(|error| format!("read Yosys synthesis log: {error}"))?
        .lines()
        .find(|line| line.trim_start().starts_with("Yosys "))
        .map(str::trim)
        .ok_or_else(|| "Yosys synthesis log contains no version banner".to_string())?
        .to_string();
    let safe = firmware_safety_gate_with_constraints(&model, horizon, &stage, assumptions)?;
    annotate_rtl_safety_report(
        &stage.join("safety-report.txt"),
        &source_label,
        top,
        &yosys_version,
        build_options,
    )?;
    let status_name = if safe { "SAFE" } else { "UNSAFE" };
    let revision = source_revision();
    let manifest_sources = source_labels
        .iter()
        .enumerate()
        .map(|(index, source)| format!("source_{index}={}", report_value(source)))
        .collect::<Vec<_>>()
        .join("\n");
    let assumption_source_label = assumptions_path.map_or_else(
        || "none".to_string(),
        |path| report_value(&path.to_string_lossy()),
    );
    let assumption_count = assumption_document
        .as_ref()
        .map_or(0, |(constraints, _)| constraints.len());
    let project_config = if build_options.is_some() {
        "cq-project.conf"
    } else {
        "none"
    };
    let include_dir_count = build_options.map_or(0, |options| options.includes.len());
    let include_file_count = build_options.map_or(0, |options| {
        options
            .includes
            .iter()
            .map(|include| include.files.len())
            .sum()
    });
    let include_bytes: usize = build_options.map_or(0, |options| {
        options
            .includes
            .iter()
            .flat_map(|include| &include.files)
            .map(|(_, bytes)| bytes.len())
            .sum()
    });
    let parameters = build_options.map_or_else(
        || "none".to_string(),
        |options| {
            if options.parameters.is_empty() {
                "none".to_string()
            } else {
                options
                    .parameters
                    .iter()
                    .map(|(name, value)| format!("{name}:{value}"))
                    .collect::<Vec<_>>()
                    .join(";")
            }
        },
    );
    let parameter_count = build_options.map_or(0, |options| options.parameters.len());
    let clock_policy = build_options.map_or_else(
        || "unspecified".to_string(),
        |options| format!("{}:{}", options.clock.0, options.clock.1),
    );
    let reset_policy = build_options.map_or_else(
        || "unspecified".to_string(),
        |options| rtl_reset_policy_label(&options.reset),
    );
    let evidence_index_sha256 = write_evidence_index(&stage)?;
    fs::write(
        stage.join("run-manifest.txt"),
        format!(
            "status={status_name}\nschema_version={RTL_ARTIFACT_SCHEMA_VERSION}\nfirmware_cli_version={FIRMWARE_CLI_CONTRACT_VERSION}\nsource={}\nsource_count={}\n{manifest_sources}\nsource_revision={revision}\nsource_bytes={source_bytes}\nassumption_source={assumption_source_label}\nassumption_count={assumption_count}\nproject_config={project_config}\ninclude_dir_count={include_dir_count}\ninclude_file_count={include_file_count}\ninclude_bytes={include_bytes}\nparameter_count={parameter_count}\nparameters={parameters}\nclock_policy={clock_policy}\nreset_policy={reset_policy}\ntop={top}\nhorizon={horizon}\nsynthesis_timeout_seconds=120\ncontainment_platform={}\nprocess_group_timeout_kill=true\nsynthesis_memory_limit_kind={}\nsynthesis_memory_limit_bytes={}\nsynthesis_file_limit_bytes={YOSYS_FILE_LIMIT_BYTES}\nyosys={yosys_version}\nevidence_digest_algorithm=sha256\nevidence_index=evidence.sha256\nevidence_index_sha256={evidence_index_sha256}\n",
            report_value(&source_label), sources.len(), std::env::consts::OS,
            synthesis_memory_limit_kind(), synthesis_memory_limit_bytes()
        ),
    )
    .map_err(|error| format!("write RTL safety manifest: {error}"))?;
    let published_manifest = artifact_dir.join("run-manifest.txt");
    if published_manifest.exists() {
        fs::remove_file(&published_manifest)
            .map_err(|error| format!("remove stale RTL safety manifest: {error}"))?;
    }
    for entry in fs::read_dir(artifact_dir)
        .map_err(|error| format!("inspect RTL safety artifact directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("inspect RTL safety artifact: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_rtl_source_snapshot(&name) && entry.path().is_file() {
            fs::remove_file(entry.path())
                .map_err(|error| format!("remove stale RTL source snapshot: {error}"))?;
        }
        if is_rtl_include_snapshot(&name) && entry.path().is_dir() {
            fs::remove_dir_all(entry.path())
                .map_err(|error| format!("remove stale RTL include snapshot: {error}"))?;
        }
    }
    let mut published_names = vec![
        "model.aag",
        "signal.map",
        "synthesis.ys",
        "yosys.log",
        "yosys-errors.log",
        "solver-metrics.csv",
        "safety-report.txt",
        "evidence.sha256",
    ];
    published_names.extend(staged_source_names.iter().map(String::as_str));
    if assumptions_path.is_some() {
        published_names.push("assumptions.txt");
    } else {
        let stale = artifact_dir.join("assumptions.txt");
        if stale.is_file() {
            fs::remove_file(stale)
                .map_err(|error| format!("remove stale environment assumptions: {error}"))?;
        }
    }
    for name in published_names {
        replace_file(&stage.join(name), &artifact_dir.join(name))?;
    }
    if let Some(options) = build_options {
        replace_file(
            &stage.join("cq-project.conf"),
            &artifact_dir.join("cq-project.conf"),
        )?;
        for include in &options.includes {
            let destination = artifact_dir.join(&include.label);
            if destination.exists() {
                fs::remove_dir_all(&destination)
                    .map_err(|error| format!("remove stale include snapshot: {error}"))?;
            }
            fs::rename(stage.join(&include.label), &destination)
                .map_err(|error| format!("publish include snapshot: {error}"))?;
        }
    } else {
        let stale = artifact_dir.join("cq-project.conf");
        if stale.is_file() {
            fs::remove_file(stale)
                .map_err(|error| format!("remove stale project config: {error}"))?;
        }
    }
    replace_file(&stage.join("run-manifest.txt"), &published_manifest)?;
    fs::remove_dir(&stage)
        .map_err(|error| format!("remove RTL safety staging directory: {error}"))?;
    println!(
        "firmware-rtl-safety-gate status={status_name} source={} top={top} report={}",
        source_label,
        artifact_dir.join("safety-report.txt").display()
    );
    Ok(safe)
}

fn firmware_rtl_project_safety_gate(
    inputs: &[PathBuf],
    top: &str,
    horizon: usize,
    artifact_dir: &Path,
) -> Result<bool, String> {
    firmware_rtl_project_safety_gate_with_assumptions(
        inputs,
        top,
        horizon,
        artifact_dir,
        None,
        None,
    )
}

fn firmware_rtl_safety_gate(
    input: &Path,
    top: &str,
    horizon: usize,
    artifact_dir: &Path,
) -> Result<bool, String> {
    firmware_rtl_project_safety_gate(&[input.to_path_buf()], top, horizon, artifact_dir)
}

fn firmware_rtl_config_safety_gate(
    config_path: &Path,
    artifact_dir: &Path,
) -> Result<bool, String> {
    let config = parse_rtl_project_config(config_path)?;
    let (sources, assumptions, options) = load_rtl_build_options(config_path, &config)?;
    firmware_rtl_project_safety_gate_with_assumptions(
        &sources,
        &config.top,
        config.horizon,
        artifact_dir,
        assumptions.as_deref(),
        Some(&options),
    )
}

fn run_firmware_gate_cli(args: &[String]) -> Result<Option<bool>, String> {
    match args.first().map(String::as_str) {
        Some("firmware-cli-version") => {
            if args.len() != 1 {
                return Err("usage: continuation-quotient-sat firmware-cli-version".to_string());
            }
            println!(
                "firmware_cli_version={FIRMWARE_CLI_CONTRACT_VERSION} artifact_schema_version={RTL_ARTIFACT_SCHEMA_VERSION}"
            );
            Ok(Some(true))
        }
        Some("firmware-artifact-validate") => {
            if args.len() != 2 {
                return Err(
                    "usage: continuation-quotient-sat firmware-artifact-validate ARTIFACT_DIR"
                        .to_string(),
                );
            }
            validate_rtl_artifact_bundle(Path::new(&args[1]))?;
            Ok(Some(true))
        }
        Some("firmware-rtl-config-safety-gate") => {
            if args.len() != 3 {
                return Err("usage: continuation-quotient-sat firmware-rtl-config-safety-gate PROJECT.conf ARTIFACT_DIR".to_string());
            }
            firmware_rtl_config_safety_gate(Path::new(&args[1]), Path::new(&args[2])).map(Some)
        }
        Some("firmware-safety-gate") => {
            if args.len() != 4 {
                return Err("usage: continuation-quotient-sat firmware-safety-gate INPUT.aag HORIZON ARTIFACT_DIR".to_string());
            }
            let horizon = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid firmware safety horizon".to_string())?
                .max(1);
            firmware_safety_gate(Path::new(&args[1]), horizon, Path::new(&args[3])).map(Some)
        }
        Some("firmware-rtl-safety-gate") => {
            if args.len() != 5 {
                return Err("usage: continuation-quotient-sat firmware-rtl-safety-gate INPUT.sv TOP HORIZON ARTIFACT_DIR".to_string());
            }
            let horizon = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid RTL firmware safety horizon".to_string())?
                .max(1);
            firmware_rtl_safety_gate(Path::new(&args[1]), &args[2], horizon, Path::new(&args[4]))
                .map(Some)
        }
        Some("firmware-rtl-project-safety-gate") => {
            if args.len() < 5 {
                return Err("usage: continuation-quotient-sat firmware-rtl-project-safety-gate TOP HORIZON ARTIFACT_DIR SOURCE.sv SOURCE2.sv [...]".to_string());
            }
            let horizon = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid RTL project safety horizon".to_string())?
                .max(1);
            let sources = args[4..].iter().map(PathBuf::from).collect::<Vec<_>>();
            firmware_rtl_project_safety_gate(&sources, &args[1], horizon, Path::new(&args[3]))
                .map(Some)
        }
        Some("firmware-rtl-constrained-project-safety-gate") => {
            if args.len() < 6 {
                return Err("usage: continuation-quotient-sat firmware-rtl-constrained-project-safety-gate TOP HORIZON ARTIFACT_DIR ASSUMPTIONS.txt SOURCE.sv SOURCE2.sv [...]".to_string());
            }
            let horizon = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid constrained RTL project safety horizon".to_string())?
                .max(1);
            let sources = args[5..].iter().map(PathBuf::from).collect::<Vec<_>>();
            firmware_rtl_project_safety_gate_with_assumptions(
                &sources,
                &args[1],
                horizon,
                Path::new(&args[3]),
                Some(Path::new(&args[4])),
                None,
            )
            .map(Some)
        }
        _ => Ok(None),
    }
}

fn benchmark_continuation_dimacs(
    input: &Path,
    query_count: usize,
    max_assumptions: usize,
    output: &Path,
) -> Result<(), String> {
    let (vars, formula) = parse_dimacs(input)?;
    let order_start = Instant::now();
    let order = min_fill_order(vars, &formula);
    let order_ns = order_start.elapsed().as_nanos();
    let profile = continuation_frontier_profile(vars, &formula, &order);
    let bound_bits = ceil_log2_u128(profile.iter().copied().max().unwrap_or(1));
    let profile_state_bound = profile.iter().copied().fold(0u128, u128::saturating_add);
    let admitted = bound_bits <= 16;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create DIMACS output: {error}"))?;
    }
    let header = "input,variables,clauses,queries,max_assumptions,order_kind,order_ns,frontier_bound_bits,profile_state_bound,admitted,peak_classes,total_layer_states,transition_bytes,repair_residual_bytes,compile_ns,quotient_ns_per_query,incremental_varisat_ns_per_query,speedup_vs_incremental,break_even_queries,sat_queries,unsat_queries,agreement,witnesses_valid\n";
    if !admitted {
        fs::write(
            output,
            format!("{header}{},{vars},{},{query_count},{max_assumptions},min-fill,{order_ns},{bound_bits},{profile_state_bound},false,0,0,0,0,0,0,0,0,0,0,0,true,true\n", input.display(), formula.len()),
        )
        .map_err(|error| format!("write rejected DIMACS output: {error}"))?;
        println!(
            "continuation DIMACS input={} admitted=false bound_bits={bound_bits} output={}",
            input.display(),
            output.display()
        );
        return Ok(());
    }
    let compile_start = Instant::now();
    let compiled = compile_continuation(&formula, &order);
    let compile_ns = compile_start.elapsed().as_nanos();
    let total_layer_states: usize = compiled.residual_layers.iter().map(Vec::len).sum();
    let transition_bytes: usize = compiled
        .transitions
        .iter()
        .map(|layer| layer.len() * std::mem::size_of::<[usize; 2]>())
        .sum::<usize>()
        .saturating_add(compiled.terminal_sat.len())
        .saturating_add(compiled.order.len() * std::mem::size_of::<usize>());
    let repair_residual_bytes: usize = compiled
        .residual_layers
        .iter()
        .flat_map(|layer| layer.iter())
        .flat_map(|residual| residual.iter())
        .map(|clause| clause.len() * std::mem::size_of::<Literal>())
        .sum();
    let mut rng = Rng(formula
        .iter()
        .flat_map(|clause| clause.0.iter())
        .fold(vars as u64 ^ 0xd6e8_feb8_6659_fd93, |hash, &(v, sign)| {
            hash.rotate_left(7) ^ v as u64 ^ (sign as u64)
        }));
    let mut queries = Vec::with_capacity(query_count);
    for query_index in 0..query_count {
        let mut assumptions = vec![None; vars];
        let width = 1 + query_index % max_assumptions.max(1).min(vars.max(1));
        let mut chosen = BTreeSet::new();
        while chosen.len() < width.min(vars) {
            chosen.insert(rng.below(vars));
        }
        for variable in chosen {
            assumptions[variable] = Some(rng.next() & 1 == 1);
        }
        queries.push(assumptions);
    }
    let mut scratch = ContinuationScratch::new(&compiled);
    let quotient_start = Instant::now();
    let quotient_answers: Vec<_> = queries
        .iter()
        .map(|assumptions| query_continuation(&compiled, assumptions, &mut scratch))
        .collect();
    let quotient_ns = quotient_start.elapsed().as_nanos();
    let mut solver = Solver::new();
    add_to_varisat(&mut solver, &formula);
    let varisat_start = Instant::now();
    let varisat_answers: Vec<_> = queries
        .iter()
        .map(|assumptions| solve_varisat_assumptions(&mut solver, assumptions, vars))
        .collect();
    let varisat_ns = varisat_start.elapsed().as_nanos();
    let agreement = quotient_answers
        .iter()
        .zip(&varisat_answers)
        .all(|(left, right)| left.is_some() == right.is_some());
    let witnesses_valid = quotient_answers
        .iter()
        .zip(&queries)
        .all(|(answer, assumptions)| {
            answer.as_ref().is_none_or(|assignment| {
                satisfies(&formula, assignment)
                    && assumptions.iter().enumerate().all(|(variable, required)| {
                        required.is_none_or(|value| assignment[variable] == value)
                    })
            })
        });
    let sat_queries = quotient_answers
        .iter()
        .filter(|answer| answer.is_some())
        .count();
    let unsat_queries = query_count - sat_queries;
    let quotient_per_query = quotient_ns as f64 / query_count.max(1) as f64;
    let varisat_per_query = varisat_ns as f64 / query_count.max(1) as f64;
    let break_even_queries = if varisat_per_query > quotient_per_query {
        (compile_ns as f64 / (varisat_per_query - quotient_per_query)).ceil() as u128
    } else {
        u128::MAX
    };
    fs::write(
        output,
        format!("{header}{},{vars},{},{query_count},{max_assumptions},min-fill,{order_ns},{bound_bits},{profile_state_bound},true,{},{total_layer_states},{transition_bytes},{repair_residual_bytes},{compile_ns},{quotient_per_query:.3},{varisat_per_query:.3},{:.6},{break_even_queries},{sat_queries},{unsat_queries},{agreement},{witnesses_valid}\n", input.display(), formula.len(), compiled.peak_classes, varisat_per_query / quotient_per_query.max(1.0)),
    )
    .map_err(|error| format!("write DIMACS output: {error}"))?;
    println!(
        "continuation DIMACS input={} admitted=true peak={} speedup={:.6} agreement={agreement} witnesses_valid={witnesses_valid} output={}",
        input.display(),
        compiled.peak_classes,
        varisat_per_query / quotient_per_query.max(1.0),
        output.display()
    );
    Ok(())
}

fn benchmark_continuation_repairs(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    update_count: usize,
    query_count: usize,
    output: &Path,
) -> Result<(), String> {
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let order: Vec<_> = (0..vars).collect();
    let bound_bits = continuation_frontier_bound_bits(vars, &formula, &order);
    if bound_bits > 16 {
        return Err(format!(
            "continuation repair rejected by 16-bit gate: bound is {bound_bits} bits"
        ));
    }
    let base = compile_continuation(&formula, &order);
    let base_witness = solve_with_varisat(vars, &formula)
        .ok_or_else(|| "repair benchmark requires a satisfiable base formula".to_string())?;
    let mut rng = Rng(formula_seed ^ 0xe703_7ed1_a0b4_28db);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create repair output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create repair output: {error}"))?;
    writeln!(file, "family,formula_seed,update,kind,start_layer,updated_bound_bits,local_repair_ns,full_recompile_ns,repair_speedup,cdcl_update_ns,local_query_ns,cdcl_query_ns,local_ns_per_query,cdcl_ns_per_query,query_speedup,break_even_queries,local_total_ns,cdcl_total_ns,total_speedup,local_peak_classes,full_peak_classes,queries,sat_queries,unsat_queries,local_full_agreement,varisat_agreement,witnesses_valid")
        .map_err(|error| format!("write repair header: {error}"))?;
    for update in 0..update_count {
        let insertion = update % 2 == 0;
        let changed_clause = if insertion {
            let mut variables = BTreeSet::new();
            while variables.len() < 3.min(vars) {
                variables.insert(rng.below(vars));
            }
            let mut literals: Vec<_> = variables
                .into_iter()
                .map(|variable| (variable, rng.next() & 1 == 1))
                .collect();
            if !literals
                .iter()
                .any(|&(variable, sign)| base_witness[variable] == sign)
            {
                literals[0].1 = base_witness[literals[0].0];
            }
            Clause(literals)
        } else {
            formula[rng.below(formula.len())].clone()
        };
        let mut updated = formula.clone();
        if insertion {
            updated.push(changed_clause.clone());
        } else if let Some(index) = updated
            .iter()
            .position(|clause| clause.0 == changed_clause.0)
        {
            updated.remove(index);
        }
        let start_layer = changed_clause
            .0
            .iter()
            .map(|&(variable, _)| variable)
            .min()
            .unwrap_or(0);
        let repair_start = Instant::now();
        let repaired = repair_continuation(&base, &changed_clause, insertion);
        let local_repair_ns = repair_start.elapsed().as_nanos();
        let full_start = Instant::now();
        let full = compile_continuation(&updated, &order);
        let full_recompile_ns = full_start.elapsed().as_nanos();
        let updated_bound_bits = continuation_frontier_bound_bits(vars, &updated, &order);
        let mut queries = Vec::with_capacity(query_count);
        for query_index in 0..query_count {
            let mut assumptions = vec![None; vars];
            let width = 1 + query_index % 10.min(vars.max(1));
            let mut chosen = BTreeSet::new();
            while chosen.len() < width.min(vars) {
                chosen.insert(rng.below(vars));
            }
            for variable in chosen {
                assumptions[variable] = Some(rng.next() & 1 == 1);
            }
            queries.push(assumptions);
        }
        let mut repaired_scratch = ContinuationScratch::new(&repaired);
        let mut full_scratch = ContinuationScratch::new(&full);
        let local_query_start = Instant::now();
        let repaired_answers: Vec<_> = queries
            .iter()
            .map(|assumptions| query_continuation(&repaired, assumptions, &mut repaired_scratch))
            .collect();
        let local_query_ns = local_query_start.elapsed().as_nanos();
        let mut cdcl_solver = Solver::new();
        let cdcl_update_ns = if insertion {
            add_to_varisat(&mut cdcl_solver, &formula);
            let update_start = Instant::now();
            add_to_varisat(&mut cdcl_solver, std::slice::from_ref(&changed_clause));
            update_start.elapsed().as_nanos()
        } else {
            let update_start = Instant::now();
            add_to_varisat(&mut cdcl_solver, &updated);
            update_start.elapsed().as_nanos()
        };
        let cdcl_query_start = Instant::now();
        let cdcl_answers: Vec<Option<Vec<bool>>> = queries
            .iter()
            .map(|assumptions| {
                let literals: Vec<_> = assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Lit::from_var(Var::from_index(variable), value))
                    })
                    .collect();
                cdcl_solver.assume(&literals);
                if !cdcl_solver.solve().expect("repair Varisat solve") {
                    return None;
                }
                let mut assignment = vec![false; vars];
                for literal in cdcl_solver.model().expect("repair Varisat model") {
                    if literal.var().index() < vars {
                        assignment[literal.var().index()] = literal.is_positive();
                    }
                }
                Some(assignment)
            })
            .collect();
        let cdcl_query_ns = cdcl_query_start.elapsed().as_nanos();
        let mut local_full_agreement = true;
        let mut varisat_agreement = true;
        let mut witnesses_valid = true;
        let mut sat_queries = 0usize;
        for ((assumptions, repaired_answer), cdcl_answer) in
            queries.iter().zip(&repaired_answers).zip(&cdcl_answers)
        {
            let full_answer = query_continuation(&full, assumptions, &mut full_scratch);
            sat_queries += repaired_answer.is_some() as usize;
            local_full_agreement &= repaired_answer.is_some() == full_answer.is_some();
            varisat_agreement &= repaired_answer.is_some() == cdcl_answer.is_some();
            witnesses_valid &= repaired_answer.as_ref().is_none_or(|assignment| {
                satisfies(&updated, assignment)
                    && assumptions.iter().enumerate().all(|(variable, required)| {
                        required.is_none_or(|value| assignment[variable] == value)
                    })
            });
            witnesses_valid &= cdcl_answer.as_ref().is_none_or(|assignment| {
                satisfies(&updated, assignment)
                    && assumptions.iter().enumerate().all(|(variable, required)| {
                        required.is_none_or(|value| assignment[variable] == value)
                    })
            });
        }
        let unsat_queries = query_count.saturating_sub(sat_queries);
        let local_per_query = local_query_ns as f64 / query_count as f64;
        let cdcl_per_query = cdcl_query_ns as f64 / query_count as f64;
        let break_even_queries = if cdcl_per_query > local_per_query {
            (local_repair_ns.saturating_sub(cdcl_update_ns) as f64
                / (cdcl_per_query - local_per_query))
                .ceil() as u128
        } else {
            u128::MAX
        };
        let local_total_ns = local_repair_ns.saturating_add(local_query_ns);
        let cdcl_total_ns = cdcl_update_ns.saturating_add(cdcl_query_ns);
        writeln!(file, "{family},{formula_seed},{},{},{start_layer},{updated_bound_bits},{local_repair_ns},{full_recompile_ns},{:.6},{cdcl_update_ns},{local_query_ns},{cdcl_query_ns},{local_per_query:.3},{cdcl_per_query:.3},{:.6},{break_even_queries},{local_total_ns},{cdcl_total_ns},{:.6},{},{},{query_count},{sat_queries},{unsat_queries},{local_full_agreement},{varisat_agreement},{witnesses_valid}", update + 1, if insertion { "insert" } else { "delete" }, full_recompile_ns as f64 / local_repair_ns.max(1) as f64, cdcl_per_query / local_per_query.max(1.0), cdcl_total_ns as f64 / local_total_ns.max(1) as f64, repaired.peak_classes, full.peak_classes)
            .map_err(|error| format!("write repair row: {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("flush repair output: {error}"))?;
    println!(
        "continuation repairs family={family} seed={formula_seed} vars={vars} updates={update_count} queries={query_count} output={}",
        output.display()
    );
    Ok(())
}

fn solve_varisat_assumptions(
    solver: &mut Solver<'_>,
    assumptions: &[Option<bool>],
    vars: usize,
) -> Option<Vec<bool>> {
    let literals: Vec<_> = assumptions
        .iter()
        .enumerate()
        .filter_map(|(variable, value)| {
            value.map(|value| Lit::from_var(Var::from_index(variable), value))
        })
        .collect();
    solver.assume(&literals);
    if !solver.solve().expect("hybrid Varisat solve") {
        return None;
    }
    let mut assignment = vec![false; vars];
    for literal in solver.model().expect("hybrid Varisat model") {
        if literal.var().index() < vars {
            assignment[literal.var().index()] = literal.is_positive();
        }
    }
    Some(assignment)
}

fn benchmark_continuation_hybrid(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    phase_count: usize,
    update_span: usize,
    output: &Path,
) -> Result<(), String> {
    let mut formula = generate_formula(family, vars, ratio, formula_seed);
    let order: Vec<_> = (0..vars).collect();
    let mut rng = Rng(formula_seed ^ 0x8ebc_6af0_9c88_c6e3);
    let mut baseline_solver = Solver::new();
    let baseline_setup_start = Instant::now();
    add_to_varisat(&mut baseline_solver, &formula);
    let mut baseline_total_ns = baseline_setup_start.elapsed().as_nanos();
    let mut hybrid_solver = Solver::new();
    let hybrid_setup_start = Instant::now();
    add_to_varisat(&mut hybrid_solver, &formula);
    let mut hybrid_total_ns = hybrid_setup_start.elapsed().as_nanos();
    let mut compiled: Option<CompiledContinuation> = None;
    let query_schedule = [100usize, 5_000, 25_000, 20_000, 1_000, 30_000];
    let width_schedule = [3usize, 10, 10, 40, 5, 20];
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create hybrid output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create hybrid output: {error}"))?;
    writeln!(file, "family,formula_seed,phase,update_kind,start_layer,queries,max_assumptions,declared_horizon_queries,frontier_bound_bits,profile_state_bound,suffix_state_bound,required_horizon_queries,decision,decision_ns,hybrid_phase_ns,baseline_phase_ns,phase_speedup,sat_queries,unsat_queries,agreement,witnesses_valid")
        .map_err(|error| format!("write hybrid header: {error}"))?;
    let mut all_agree = true;
    let mut all_valid = true;
    for phase in 0..phase_count {
        let queries_count = query_schedule[phase % query_schedule.len()];
        let max_assumptions = width_schedule[phase % width_schedule.len()].min(vars.max(1));
        let stable_end = ((phase / update_span.max(1) + 1) * update_span.max(1)).min(phase_count);
        let declared_horizon_queries: usize = (phase..stable_end)
            .filter(|&future| width_schedule[future % width_schedule.len()] <= 20)
            .map(|future| query_schedule[future % query_schedule.len()])
            .sum();
        let mut update_kind = "none";
        let mut start_layer = 0usize;
        let mut changed: Option<(Clause, bool)> = None;
        let mut baseline_update_ns = 0u128;
        if phase > 0 && phase % update_span.max(1) == 0 {
            let insertion = (phase / update_span.max(1)) % 2 == 1;
            update_kind = if insertion { "insert" } else { "delete" };
            let clause = if insertion {
                let witness = solve_with_varisat(vars, &formula)
                    .ok_or_else(|| "hybrid workload unexpectedly became UNSAT".to_string())?;
                let mut variables = BTreeSet::new();
                while variables.len() < 3.min(vars) {
                    variables.insert(rng.below(vars));
                }
                let mut literals: Vec<_> = variables
                    .into_iter()
                    .map(|variable| (variable, rng.next() & 1 == 1))
                    .collect();
                if !literals
                    .iter()
                    .any(|&(variable, sign)| witness[variable] == sign)
                {
                    literals[0].1 = witness[literals[0].0];
                }
                Clause(literals)
            } else {
                formula[rng.below(formula.len())].clone()
            };
            start_layer = clause
                .0
                .iter()
                .map(|&(variable, _)| variable)
                .min()
                .unwrap_or(0);
            if insertion {
                formula.push(clause.clone());
                let update_start = Instant::now();
                add_to_varisat(&mut baseline_solver, std::slice::from_ref(&clause));
                baseline_update_ns = update_start.elapsed().as_nanos();
            } else {
                let index = formula
                    .iter()
                    .position(|candidate| candidate.0 == clause.0)
                    .unwrap();
                formula.remove(index);
                let update_start = Instant::now();
                baseline_solver = Solver::new();
                add_to_varisat(&mut baseline_solver, &formula);
                baseline_update_ns = update_start.elapsed().as_nanos();
            }
            changed = Some((clause, insertion));
        }
        let hybrid_update_start = Instant::now();
        if let Some((clause, insertion)) = &changed {
            if *insertion {
                add_to_varisat(&mut hybrid_solver, std::slice::from_ref(clause));
            } else {
                hybrid_solver = Solver::new();
                add_to_varisat(&mut hybrid_solver, &formula);
            }
        }
        let hybrid_update_ns = hybrid_update_start.elapsed().as_nanos();
        let profile = continuation_frontier_profile(vars, &formula, &order);
        let bound_bits = ceil_log2_u128(profile.iter().copied().max().unwrap_or(1));
        let profile_state_bound = profile.iter().copied().fold(0u128, u128::saturating_add);
        let suffix_state_bound = profile[start_layer.min(profile.len())..]
            .iter()
            .copied()
            .fold(0u128, u128::saturating_add);
        let decision_start = Instant::now();
        let query_regime = max_assumptions <= 20;
        let compilation_threshold = profile_state_bound
            .saturating_mul(16)
            .min(usize::MAX as u128) as usize;
        let repair_threshold = suffix_state_bound
            .saturating_mul(16)
            .min(usize::MAX as u128) as usize;
        let mut decision = "cdcl";
        if let Some((clause, insertion)) = &changed {
            if compiled.is_some()
                && *insertion
                && bound_bits <= 16
                && query_regime
                && declared_horizon_queries >= repair_threshold
                && start_layer >= 10
            {
                compiled = Some(repair_continuation(
                    compiled.as_ref().unwrap(),
                    clause,
                    *insertion,
                ));
                decision = "repair";
            } else {
                compiled = None;
            }
        }
        if compiled.is_none()
            && bound_bits <= 16
            && query_regime
            && declared_horizon_queries >= compilation_threshold
        {
            compiled = Some(compile_continuation(&formula, &order));
            decision = "compile";
        } else if compiled.is_some() && query_regime {
            decision = if decision == "repair" {
                "repair"
            } else {
                "quotient"
            };
        }
        let decision_ns = decision_start.elapsed().as_nanos();
        let mut queries = Vec::with_capacity(queries_count);
        for query_index in 0..queries_count {
            let mut assumptions = vec![None; vars];
            let width = 1 + query_index % max_assumptions;
            let mut chosen = BTreeSet::new();
            while chosen.len() < width.min(vars) {
                chosen.insert(rng.below(vars));
            }
            for variable in chosen {
                assumptions[variable] = Some(rng.next() & 1 == 1);
            }
            queries.push(assumptions);
        }
        let baseline_query_start = Instant::now();
        let baseline_answers: Vec<_> = queries
            .iter()
            .map(|assumptions| solve_varisat_assumptions(&mut baseline_solver, assumptions, vars))
            .collect();
        let baseline_query_ns = baseline_query_start.elapsed().as_nanos();
        let hybrid_query_start = Instant::now();
        let hybrid_answers: Vec<_> = if decision == "cdcl" {
            queries
                .iter()
                .map(|assumptions| solve_varisat_assumptions(&mut hybrid_solver, assumptions, vars))
                .collect()
        } else {
            let active = compiled.as_ref().unwrap();
            let mut scratch = ContinuationScratch::new(active);
            queries
                .iter()
                .map(|assumptions| query_continuation(active, assumptions, &mut scratch))
                .collect()
        };
        let hybrid_query_ns = hybrid_query_start.elapsed().as_nanos();
        let agreement = hybrid_answers
            .iter()
            .zip(&baseline_answers)
            .all(|(left, right)| left.is_some() == right.is_some());
        let witnesses_valid = hybrid_answers
            .iter()
            .zip(&queries)
            .all(|(answer, assumptions)| {
                answer.as_ref().is_none_or(|assignment| {
                    satisfies(&formula, assignment)
                        && assumptions.iter().enumerate().all(|(variable, required)| {
                            required.is_none_or(|value| assignment[variable] == value)
                        })
                })
            });
        let sat_queries = hybrid_answers
            .iter()
            .filter(|answer| answer.is_some())
            .count();
        let unsat_queries = queries_count - sat_queries;
        let hybrid_phase_ns = hybrid_update_ns
            .saturating_add(decision_ns)
            .saturating_add(hybrid_query_ns);
        let baseline_phase_ns = baseline_update_ns.saturating_add(baseline_query_ns);
        hybrid_total_ns = hybrid_total_ns.saturating_add(hybrid_phase_ns);
        baseline_total_ns = baseline_total_ns.saturating_add(baseline_phase_ns);
        all_agree &= agreement;
        all_valid &= witnesses_valid;
        let required_horizon_queries = if decision == "repair" {
            repair_threshold
        } else {
            compilation_threshold
        };
        writeln!(file, "{family},{formula_seed},{},{update_kind},{start_layer},{queries_count},{max_assumptions},{declared_horizon_queries},{bound_bits},{profile_state_bound},{suffix_state_bound},{required_horizon_queries},{decision},{decision_ns},{hybrid_phase_ns},{baseline_phase_ns},{:.6},{sat_queries},{unsat_queries},{agreement},{witnesses_valid}", phase + 1, baseline_phase_ns as f64 / hybrid_phase_ns.max(1) as f64)
            .map_err(|error| format!("write hybrid row: {error}"))?;
    }
    writeln!(file, "{family},{formula_seed},total,none,0,0,0,0,0,0,0,0,summary,0,{hybrid_total_ns},{baseline_total_ns},{:.6},0,0,{all_agree},{all_valid}", baseline_total_ns as f64 / hybrid_total_ns.max(1) as f64)
        .map_err(|error| format!("write hybrid total: {error}"))?;
    file.flush()
        .map_err(|error| format!("flush hybrid output: {error}"))?;
    println!(
        "continuation hybrid family={family} seed={formula_seed} vars={vars} phases={phase_count} speedup={:.6} agreement={all_agree} witnesses_valid={all_valid} output={}",
        baseline_total_ns as f64 / hybrid_total_ns.max(1) as f64,
        output.display()
    );
    Ok(())
}

fn benchmark_continuation_quotients(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    strategies: usize,
    gate_limit_bits: Option<usize>,
    output: &Path,
) -> Result<(), String> {
    if vars > 256 {
        return Err("continuation quotient search supports at most 256 variables".to_string());
    }
    let formula = generate_formula(family, vars, ratio, formula_seed);
    let (reference_width, reference_kind) = if vars <= 20 {
        (exact_treewidth(vars, &formula), "exact")
    } else {
        (
            structural_treewidth_lower_bound(vars, &formula),
            "structural-lower-bound",
        )
    };
    let base: Vec<Vec<Literal>> = formula
        .iter()
        .map(|clause| {
            let mut literals = clause.0.clone();
            literals.sort_unstable();
            literals.dedup();
            literals
        })
        .collect();
    let structural = [
        (0..vars).collect::<Vec<_>>(),
        min_fill_order(vars, &formula),
        min_degree_order(vars, &formula),
        flower_outside_in_order(vars),
    ];
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create quotient output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create quotient output: {error}"))?;
    writeln!(file, "family,formula_seed,strategy,order_kind,reference_width,reference_kind,order_width,frontier_bound_bits,gate_limit_bits,gate_admitted,route,peak_classes,peak_class_bits,class_bit_change_vs_reference,canonical_work,bruteforce_literal_work,work_ratio,final_classes,sat,witness_valid,gate_ns,quotient_ns,varisat_ns,routed_total_ns,time_ratio_vs_varisat")
        .map_err(|error| format!("write quotient header: {error}"))?;
    let literal_count = formula.iter().map(|clause| clause.0.len()).sum::<usize>();
    let brute_work =
        (1u128.checked_shl(vars as u32).unwrap_or(u128::MAX)).saturating_mul(literal_count as u128);
    let brute_work_float = 2f64.powi(vars as i32) * literal_count as f64;
    let varisat_start = Instant::now();
    let varisat_witness = solve_with_varisat(vars, &formula);
    let varisat_ns = varisat_start.elapsed().as_nanos().max(1);
    for strategy in 0..strategies {
        let kind = strategy % 5;
        let mut order = if kind < 4 {
            structural[kind].clone()
        } else {
            let mut order: Vec<_> = (0..vars).collect();
            Rng((strategy as u64 + 1).wrapping_mul(0x9e37_79b9)).shuffle(&mut order);
            order
        };
        if kind < 4 {
            let variant = strategy / 5;
            order.rotate_left(variant % vars.max(1));
            if variant & 1 == 1 {
                order.reverse();
            }
        }
        let order_width = elimination_cost(vars, &formula, &order).0;
        let gate_start = Instant::now();
        let frontier_bound_bits = continuation_frontier_bound_bits(vars, &formula, &order);
        let gate_ns = gate_start.elapsed().as_nanos();
        let gate_admitted = gate_limit_bits.is_none_or(|limit| frontier_bound_bits <= limit);
        if !gate_admitted {
            let sat = varisat_witness.is_some();
            let routed_total_ns = gate_ns.saturating_add(varisat_ns);
            writeln!(file, "{family},{formula_seed},{},{},{reference_width},{reference_kind},{order_width},{frontier_bound_bits},{},{gate_admitted},varisat,0,0,0,0,{brute_work},0.000000000,0,{sat},true,{gate_ns},0,{varisat_ns},{routed_total_ns},{:.6}", strategy + 1, ["natural", "min-fill", "min-degree", "flower", "random"][kind], gate_limit_bits.unwrap_or(0), routed_total_ns as f64 / varisat_ns as f64)
                .map_err(|error| format!("write gated quotient row: {error}"))?;
            continue;
        }
        let quotient_start = Instant::now();
        let mut classes: HashMap<Vec<Vec<Literal>>, Vec<bool>> = HashMap::new();
        classes.insert(base.clone(), vec![false; vars]);
        let mut peak_classes = 1usize;
        let mut work = 0usize;
        for &variable in &order {
            let mut next = HashMap::new();
            for (residual, representative) in classes {
                for value in [false, true] {
                    let canonical =
                        canonical_residual_after_choice(&residual, variable, value, &mut work);
                    let mut assignment = representative.clone();
                    assignment[variable] = value;
                    next.entry(canonical).or_insert(assignment);
                }
            }
            classes = next;
            peak_classes = peak_classes.max(classes.len());
        }
        let quotient_ns = quotient_start.elapsed().as_nanos();
        let satisfying = classes.get(&Vec::<Vec<Literal>>::new());
        let sat = satisfying.is_some();
        let witness_valid = satisfying.is_some_and(|assignment| satisfies(&formula, assignment))
            || (!sat && varisat_witness.is_none());
        let class_bits = ceil_log2(peak_classes);
        let routed_total_ns = gate_ns.saturating_add(quotient_ns);
        writeln!(file, "{family},{formula_seed},{},{},{reference_width},{reference_kind},{order_width},{frontier_bound_bits},{},{gate_admitted},quotient,{peak_classes},{class_bits},{},{work},{brute_work},{:.9},{},{sat},{witness_valid},{gate_ns},{quotient_ns},{varisat_ns},{routed_total_ns},{:.6}", strategy + 1, ["natural", "min-fill", "min-degree", "flower", "random"][kind], gate_limit_bits.unwrap_or(0), class_bits as isize - reference_width as isize, work as f64 / brute_work_float.max(1.0), classes.len(), routed_total_ns as f64 / varisat_ns as f64)
            .map_err(|error| format!("write quotient row: {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("flush quotient output: {error}"))?;
    println!(
        "continuation quotient search family={family} seed={formula_seed} strategies={strategies} reference_width={reference_width} reference_kind={reference_kind} output={}",
        output.display()
    );
    Ok(())
}

fn benchmark_bdd_network_expansion(
    family: &str,
    vars: usize,
    ratio: usize,
    formula_seed: u64,
    random_orders: usize,
    output: &Path,
) -> Result<(), String> {
    if vars > 20 {
        return Err(
            "BDD network expansion requires an exact original width (max 20 vars)".to_string(),
        );
    }
    let original = generate_formula(family, vars, ratio, formula_seed);
    let original_width = exact_treewidth(vars, &original);
    let original_sat = solve_with_varisat(vars, &original).is_some();
    let graph = primal_graph(vars, &original);
    let mut orders = vec![
        ("natural".to_string(), (0..vars).collect::<Vec<_>>()),
        ("min-fill".to_string(), min_fill_order(vars, &original)),
        ("min-degree".to_string(), min_degree_order(vars, &original)),
        ("flower".to_string(), flower_outside_in_order(vars)),
    ];
    for index in 0..random_orders {
        let mut order: Vec<_> = (0..vars).collect();
        Rng(formula_seed ^ (index as u64 + 1).wrapping_mul(0x517c_c1b7)).shuffle(&mut order);
        orders.push((format!("random-{index}"), order));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create network output: {error}"))?;
    }
    let mut file =
        fs::File::create(output).map_err(|error| format!("create network output: {error}"))?;
    writeln!(file, "family,formula_seed,strategy,order,prefix,interior,boundary,original_width,expanded_vars,expanded_clauses,relation_helpers,expanded_upper_width,reconstruction_live_nodes,reconstruction_allocated_nodes,reconstruction_information_charge,fully_charged_width,certified_change,sat_equivalent,reconstruction_valid")
        .map_err(|error| format!("write network header: {error}"))?;
    let mut strategy = 0usize;
    for (order_name, order) in orders {
        for prefix in 1..vars {
            strategy += 1;
            let mut interior = order[..prefix].to_vec();
            let interior_set: BTreeSet<_> = interior.iter().copied().collect();
            let boundary: Vec<_> = interior
                .iter()
                .flat_map(|&variable| graph[variable].iter().copied())
                .filter(|variable| !interior_set.contains(variable))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let mut manager = BddManager::default();
            let reconstruction_seed = seed_bdd_candidate_in(
                vars,
                &original,
                &mut interior,
                boundary.clone(),
                "min-fill",
                &mut manager,
            );
            let (expanded_vars, expanded, helpers) =
                projected_bdd_network_cnf(vars, &original, &interior, &boundary);
            let expanded_width = elimination_cost(
                expanded_vars,
                &expanded,
                &min_fill_order(expanded_vars, &expanded),
            )
            .0;
            let reconstruction_charge = ceil_log2(reconstruction_seed.live_nodes.saturating_add(2));
            let fully_charged_width = expanded_width.max(reconstruction_charge);
            let expanded_assignment = solve_with_varisat(expanded_vars, &expanded);
            let expanded_sat = expanded_assignment.is_some();
            let reconstruction_valid = if let Some(values) = expanded_assignment {
                let mut mapped = vec![false; vars];
                for (core, &original_variable) in
                    reconstruction_seed.core_to_original.iter().enumerate()
                {
                    mapped[original_variable] = values[core];
                }
                regrow_bdd_seed(&reconstruction_seed, &mapped).is_some_and(|inside| {
                    for (index, &variable) in reconstruction_seed.interior.iter().enumerate() {
                        mapped[variable] = inside[index];
                    }
                    satisfies(&original, &mapped)
                })
            } else {
                !original_sat
            };
            writeln!(file, "{family},{formula_seed},{strategy},{order_name},{prefix},{},{},{original_width},{expanded_vars},{},{helpers},{expanded_width},{},{},{reconstruction_charge},{fully_charged_width},{},{},{}", interior.len(), boundary.len(), expanded.len(), reconstruction_seed.live_nodes, reconstruction_seed.allocated_nodes, fully_charged_width as isize - original_width as isize, expanded_sat == original_sat, reconstruction_valid)
                .map_err(|error| format!("write network row: {error}"))?;
        }
    }
    file.flush()
        .map_err(|error| format!("flush network output: {error}"))?;
    println!(
        "BDD network expansion family={family} seed={formula_seed} strategies={strategy} original_width={original_width} output={}",
        output.display()
    );
    Ok(())
}

fn benchmark_query_calibrated(
    input: &Path,
    output: &Path,
    calibration_queries: usize,
    evaluation_queries: usize,
    deadline: std::time::Duration,
    gate: &str,
) -> Result<(), String> {
    let total = calibration_queries.saturating_add(evaluation_queries);
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("calibrated");
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let calibration_path = parent.join(format!("{stem}-calibration.csv"));
    let evaluation_path = parent.join(format!("{stem}-evaluation.csv"));
    let baseline_path = parent.join(format!("{stem}-baseline.csv"));
    benchmark_query_portfolio(
        input,
        &calibration_path,
        0,
        calibration_queries,
        total,
        deadline,
        &[gate.to_string()],
    )?;
    let calibration_text = fs::read_to_string(&calibration_path)
        .map_err(|error| format!("read calibration: {error}"))?;
    let helper_wins = calibration_text
        .lines()
        .skip(1)
        .filter(|row| row.split(',').nth(4) == Some(gate))
        .count();
    let selected = helper_wins > 0;
    let selected_gates = if selected {
        vec![gate.to_string()]
    } else {
        Vec::new()
    };
    benchmark_query_portfolio(
        input,
        &evaluation_path,
        calibration_queries,
        evaluation_queries,
        total,
        deadline,
        &selected_gates,
    )?;
    benchmark_query_portfolio(input, &baseline_path, 0, total, total, deadline, &[])?;
    let (calibration_completed, calibration_worker_ns) = portfolio_totals(&calibration_path)?;
    let (evaluation_completed, evaluation_worker_ns) = portfolio_totals(&evaluation_path)?;
    let (baseline_completed, baseline_worker_ns) = portfolio_totals(&baseline_path)?;
    let calibrated_worker_ns = calibration_worker_ns.saturating_add(evaluation_worker_ns);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create calibrated output: {error}"))?;
    }
    fs::write(
        output,
        format!(
            "path,gate,selected,calibration_queries,helper_wins,calibration_completed,evaluation_queries,evaluation_completed,baseline_completed,calibrated_worker_wall_ns,baseline_worker_wall_ns,worker_wall_delta_ns\n{},{},{},{},{},{},{},{},{},{},{},{}\n",
            input.to_string_lossy().replace(',', "%2C"), gate, selected, calibration_queries,
            helper_wins, calibration_completed, evaluation_queries, evaluation_completed,
            baseline_completed, calibrated_worker_ns, baseline_worker_ns,
            calibrated_worker_ns as i128 - baseline_worker_ns as i128
        ),
    )
    .map_err(|error| format!("write calibrated output: {error}"))?;
    println!(
        "calibrated selected={} helper_wins={} evaluation_completed={}/{} baseline_completed={}/{} worker_delta_ms={:.3}",
        selected,
        helper_wins,
        evaluation_completed,
        evaluation_queries,
        baseline_completed,
        total,
        (calibrated_worker_ns as i128 - baseline_worker_ns as i128) as f64 / 1e6
    );
    Ok(())
}

fn export_balanced_candidates(input: &Path, output: &Path) -> Result<(), String> {
    let (vars, clauses) = parse_dimacs(input)?;
    let candidates = fast_detachable_branch_candidates(vars, &clauses, 64);
    let incidence = clause_incidence(vars, &clauses);
    let mut rows = vec!["ordinal,interior,boundary,local_clauses,local_literals,local_binary,summary_clauses,summary_literals,summary_binary,live_bdd_nodes,allocated_bdd_nodes".to_string()];
    let mut ordinal = 0usize;
    for (interior, boundary) in candidates {
        let attempt = try_indexed_seed_bdd_candidate(
            vars,
            &clauses,
            &incidence,
            interior,
            boundary,
            100_000,
            std::time::Duration::from_millis(100),
        );
        let Some(seed) = attempt.seed else { continue };
        let local = indexed_local_clauses(&seed.interior, &incidence, &clauses);
        if !solver_gate_accepts("balanced", &seed, &local) {
            continue;
        }
        let local_literals: usize = local.iter().map(|clause| clause.0.len()).sum();
        let local_binary = local.iter().filter(|clause| clause.0.len() == 2).count();
        let summary_literals: usize = seed.summary.iter().map(|clause| clause.0.len()).sum();
        let summary_binary = seed
            .summary
            .iter()
            .filter(|clause| clause.0.len() == 2)
            .count();
        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            ordinal,
            seed.interior.len(),
            seed.boundary.len(),
            local.len(),
            local_literals,
            local_binary,
            seed.summary.len(),
            summary_literals,
            summary_binary,
            seed.live_nodes,
            seed.allocated_nodes
        ));
        ordinal += 1;
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create candidate output: {error}"))?;
    }
    fs::write(output, format!("{}\n", rows.join("\n")))
        .map_err(|error| format!("write candidate output: {error}"))?;
    println!(
        "balanced_candidates={} output={}",
        ordinal,
        output.display()
    );
    Ok(())
}

fn graph_distances(graph: &[Vec<usize>], start: usize) -> Vec<usize> {
    let mut distance = vec![usize::MAX; graph.len()];
    if start >= graph.len() {
        return distance;
    }
    distance[start] = 0;
    let mut queue = VecDeque::from([start]);
    while let Some(variable) = queue.pop_front() {
        for &next in &graph[variable] {
            if distance[next] == usize::MAX {
                distance[next] = distance[variable] + 1;
                queue.push_back(next);
            }
        }
    }
    distance
}

fn export_query_candidate_features(
    input: &Path,
    output: &Path,
    queries: &[(usize, bool)],
) -> Result<(), String> {
    let (vars, clauses) = parse_dimacs(input)?;
    let graph = compact_primal_graph(vars, &clauses);
    let candidates = global_small_separator_candidates(&graph, 64);
    let incidence = clause_incidence(vars, &clauses);
    let query_distances: Vec<_> = queries
        .iter()
        .map(|&(variable, _)| graph_distances(&graph, variable))
        .collect();
    let mut rows = vec!["ordinal,query_variable,query_value,interior,boundary,boundary_degree_sum,boundary_degree_max,boundary_occurrences,interior_degree_sum,query_to_boundary,query_to_interior,query_neighbour_overlap".to_string()];
    let mut ordinal = 0usize;
    for (interior, boundary) in candidates {
        let attempt = try_indexed_seed_bdd_candidate(
            vars,
            &clauses,
            &incidence,
            interior,
            boundary,
            100_000,
            std::time::Duration::from_millis(100),
        );
        let Some(seed) = attempt.seed else { continue };
        let local = indexed_local_clauses(&seed.interior, &incidence, &clauses);
        if !solver_gate_accepts("balanced", &seed, &local) {
            continue;
        }
        let boundary_degree_sum: usize = seed.boundary.iter().map(|&v| graph[v].len()).sum();
        let boundary_degree_max = seed
            .boundary
            .iter()
            .map(|&v| graph[v].len())
            .max()
            .unwrap_or(0);
        let boundary_occurrences: usize = seed.boundary.iter().map(|&v| incidence[v].len()).sum();
        let interior_degree_sum: usize = seed.interior.iter().map(|&v| graph[v].len()).sum();
        let interior_set: BTreeSet<_> = seed.interior.iter().copied().collect();
        let boundary_set: BTreeSet<_> = seed.boundary.iter().copied().collect();
        for (query_index, &(query_variable, query_value)) in queries.iter().enumerate() {
            let distances = &query_distances[query_index];
            let boundary_distance = seed
                .boundary
                .iter()
                .map(|&v| distances[v])
                .min()
                .unwrap_or(usize::MAX);
            let interior_distance = seed
                .interior
                .iter()
                .map(|&v| distances[v])
                .min()
                .unwrap_or(usize::MAX);
            let overlap = graph[query_variable]
                .iter()
                .filter(|v| interior_set.contains(v) || boundary_set.contains(v))
                .count();
            let display_distance = |distance: usize| {
                if distance == usize::MAX {
                    "-1".to_string()
                } else {
                    distance.to_string()
                }
            };
            rows.push(format!(
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                ordinal,
                query_variable + 1,
                query_value,
                seed.interior.len(),
                seed.boundary.len(),
                boundary_degree_sum,
                boundary_degree_max,
                boundary_occurrences,
                interior_degree_sum,
                display_distance(boundary_distance),
                display_distance(interior_distance),
                overlap
            ));
        }
        ordinal += 1;
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create query features: {error}"))?;
    }
    fs::write(output, format!("{}\n", rows.join("\n")))
        .map_err(|error| format!("write query features: {error}"))?;
    println!(
        "balanced_candidates={} queries={} output={}",
        ordinal,
        queries.len(),
        output.display()
    );
    Ok(())
}

fn benchmark_corpus_inner(root: &Path, output_path: &Path, queries: usize) -> Result<(), String> {
    let mut paths = Vec::new();
    find_dimacs_files(root, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "no .cnf or .dimacs files found under {}",
            root.display()
        ));
    }
    let mut rows = vec![CORPUS_HEADER.to_string()];
    for (case_index, path) in paths.iter().enumerate() {
        let (vars, clauses) = parse_dimacs(path)?;
        if vars == 0 {
            continue;
        }
        let candidate_count = fast_detachable_branch_candidates(vars, &clauses, 64).len();
        let compile_start = Instant::now();
        let artifact = compile_safe_artifact(vars, &clauses, 64, 100_000, 100);
        let compile_ns = compile_start.elapsed().as_nanos();
        let artifact_path = std::env::temp_dir().join(format!(
            "layered-sat-corpus-{}-{case_index}.lsat",
            std::process::id()
        ));
        save_compiled_artifact(&artifact_path, &artifact)?;
        let artifact_bytes = fs::metadata(&artifact_path)
            .map_err(|error| format!("stat {}: {error}", artifact_path.display()))?
            .len();
        fs::remove_file(&artifact_path)
            .map_err(|error| format!("remove {}: {error}", artifact_path.display()))?;

        let baseline_setup_start = Instant::now();
        let mut baseline = Solver::new();
        add_to_varisat(&mut baseline, &clauses);
        let baseline_setup_ns = baseline_setup_start.elapsed().as_nanos();
        let compiled_setup_start = Instant::now();
        let mut direct = Solver::new();
        add_to_varisat(&mut direct, &artifact.core_clauses);
        let mut reopened_solvers = Vec::new();
        for seed_index in 0..artifact.seeds.len() {
            let mut solver = Solver::new();
            add_to_varisat(&mut solver, &reopened_formula(&artifact, seed_index));
            reopened_solvers.push(solver);
        }
        let compiled_setup_ns = compiled_setup_start.elapsed().as_nanos();
        let mut original_to_core = vec![usize::MAX; vars];
        for (core, &original) in artifact.core_to_original.iter().enumerate() {
            original_to_core[original] = core;
        }
        let mut owner = vec![usize::MAX; vars];
        for (seed_index, seed) in artifact.seeds.iter().enumerate() {
            for &variable in &seed.interior {
                owner[variable] = seed_index;
            }
        }
        let mut baseline_query_ns = 0u128;
        let mut direct_query_ns = 0u128;
        let mut reopened_query_ns = 0u128;
        let mut direct_queries = 0usize;
        let mut reopened_queries = 0usize;
        let mut all_agree = true;
        let mut witnesses_valid = true;
        for query in 0..queries {
            let variable = query % vars;
            let value = (query / vars + query) % 2 == 0;
            baseline.assume(&[Lit::from_var(Var::from_index(variable), value)]);
            let start = Instant::now();
            let baseline_sat = baseline
                .solve()
                .map_err(|error| format!("baseline solve: {error}"))?;
            baseline_query_ns += start.elapsed().as_nanos();
            let compiled_sat = if owner[variable] == usize::MAX {
                direct.assume(&[Lit::from_var(
                    Var::from_index(original_to_core[variable]),
                    value,
                )]);
                let start = Instant::now();
                let sat = direct
                    .solve()
                    .map_err(|error| format!("direct solve: {error}"))?;
                direct_query_ns += start.elapsed().as_nanos();
                direct_queries += 1;
                sat
            } else {
                let solver = &mut reopened_solvers[owner[variable]];
                solver.assume(&[Lit::from_var(Var::from_index(variable), value)]);
                let start = Instant::now();
                let sat = solver
                    .solve()
                    .map_err(|error| format!("reopened solve: {error}"))?;
                reopened_query_ns += start.elapsed().as_nanos();
                reopened_queries += 1;
                sat
            };
            all_agree &= baseline_sat == compiled_sat;
            if query < 4 || query + 1 == queries {
                let reconstructed = query_compiled_artifact(&artifact, &[(variable, value)])?;
                witnesses_valid &= reconstructed.is_some() == baseline_sat;
                witnesses_valid &= reconstructed.as_ref().is_none_or(|assignment| {
                    assignment[variable] == value && satisfies(&clauses, assignment)
                });
            }
        }
        let compiled_query_ns = direct_query_ns + reopened_query_ns;
        let removed = vars - artifact.core_vars;
        let path_text = path.to_string_lossy().replace(',', "%2C");
        rows.push(format!(
            "{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{},ok",
            path_text,
            vars,
            clauses.len(),
            candidate_count,
            artifact.seeds.len(),
            candidate_count.saturating_sub(artifact.seeds.len()),
            removed,
            removed as f64 / vars as f64,
            compile_ns,
            artifact_bytes,
            artifact
                .seeds
                .iter()
                .map(|seed| seed.manager.nodes.len())
                .sum::<usize>(),
            baseline_setup_ns,
            compiled_setup_ns,
            queries,
            direct_queries,
            reopened_queries,
            baseline_query_ns,
            direct_query_ns,
            reopened_query_ns,
            compiled_query_ns,
            compiled_query_ns as f64 / baseline_query_ns.max(1) as f64,
            (compile_ns + compiled_setup_ns + compiled_query_ns) as f64
                / (baseline_setup_ns + baseline_query_ns).max(1) as f64,
            removed * 10 >= vars * 3,
            all_agree,
            witnesses_valid
        ));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(output_path, rows.join("\n") + "\n")
        .map_err(|error| format!("write {}: {error}", output_path.display()))?;
    println!(
        "benchmarked formulas={} queries_per_formula={} output={}",
        rows.len() - 1,
        queries,
        output_path.display()
    );
    Ok(())
}

fn benchmark_corpus_isolated(
    root: &Path,
    output_path: &Path,
    queries: usize,
    timeout_seconds: u64,
) -> Result<(), String> {
    let mut paths = Vec::new();
    find_dimacs_files(root, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "no .cnf or .dimacs files found under {}",
            root.display()
        ));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let mut completed = BTreeSet::new();
    if output_path.exists() {
        let body = fs::read_to_string(output_path)
            .map_err(|error| format!("read {}: {error}", output_path.display()))?;
        for line in body.lines().skip(1) {
            if let Some(path) = line.split(',').next() {
                completed.insert(path.replace("%2C", ","));
            }
        }
    } else {
        fs::write(output_path, format!("{CORPUS_HEADER}\n"))
            .map_err(|error| format!("write {}: {error}", output_path.display()))?;
    }
    let executable =
        env::current_exe().map_err(|error| format!("locate current executable: {error}"))?;
    let mut attempted = 0usize;
    for (index, path) in paths.iter().enumerate() {
        let path_text = path.to_string_lossy().to_string();
        if completed.contains(&path_text) {
            continue;
        }
        attempted += 1;
        let temporary = std::env::temp_dir().join(format!(
            "layered-sat-isolated-{}-{index}.csv",
            std::process::id()
        ));
        let mut child = Command::new(&executable)
            .arg("benchmark-single")
            .arg(path)
            .arg(&temporary)
            .arg(queries.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("spawn benchmark for {}: {error}", path.display()))?;
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("wait for {}: {error}", path.display()))?
            {
                break if status.success() {
                    "ok"
                } else {
                    "child-error"
                };
            }
            if start.elapsed() >= std::time::Duration::from_secs(timeout_seconds) {
                child
                    .kill()
                    .map_err(|error| format!("kill timed out {}: {error}", path.display()))?;
                child
                    .wait()
                    .map_err(|error| format!("reap {}: {error}", path.display()))?;
                break "timeout";
            }
            thread::sleep(std::time::Duration::from_millis(100));
        };
        let row = if status == "ok" {
            let body = fs::read_to_string(&temporary).map_err(|error| {
                format!("read isolated result {}: {error}", temporary.display())
            })?;
            body.lines()
                .nth(1)
                .ok_or_else(|| format!("missing isolated row for {}", path.display()))?
                .to_string()
        } else {
            format!(
                "{}{}{}",
                path_text.replace(',', "%2C"),
                ",".repeat(25),
                status
            )
        };
        let mut output = fs::OpenOptions::new()
            .append(true)
            .open(output_path)
            .map_err(|error| format!("append {}: {error}", output_path.display()))?;
        writeln!(output, "{row}")
            .map_err(|error| format!("append {}: {error}", output_path.display()))?;
        output
            .flush()
            .map_err(|error| format!("flush {}: {error}", output_path.display()))?;
        let _ = fs::remove_file(&temporary);
        println!(
            "[{}/{}] {} status={}",
            index + 1,
            paths.len(),
            path.display(),
            status
        );
    }
    println!(
        "corpus complete discovered={} attempted={} output={}",
        paths.len(),
        attempted,
        output_path.display()
    );
    Ok(())
}

fn run_artifact_cli(args: &[String]) -> Result<bool, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(false);
    };
    match command {
        "benchmark-continuation-quotients" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-continuation-quotients FAMILY VARS RATIO SEED STRATEGIES OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid quotient variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid quotient ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid quotient seed".to_string())?;
            let strategies = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid quotient strategy count".to_string())?
                .max(1);
            benchmark_continuation_quotients(
                &args[1],
                vars,
                ratio,
                seed,
                strategies,
                None,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-gate" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-continuation-gate FAMILY VARS RATIO SEED MAX_BOUND_BITS OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid gated quotient variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid gated quotient ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid gated quotient seed".to_string())?;
            let limit = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid continuation gate bound".to_string())?;
            benchmark_continuation_quotients(
                &args[1],
                vars,
                ratio,
                seed,
                1,
                Some(limit),
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-reuse" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-continuation-reuse FAMILY VARS RATIO SEED QUERIES OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid continuation reuse variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid continuation reuse ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid continuation reuse seed".to_string())?;
            let queries = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid continuation reuse query count".to_string())?
                .max(1);
            benchmark_continuation_reuse(
                &args[1],
                vars,
                ratio,
                seed,
                queries,
                3,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-dimacs" => {
            if args.len() != 5 {
                return Err("usage: layered-sat benchmark-continuation-dimacs INPUT.cnf QUERIES MAX_ASSUMPTIONS OUTPUT.csv".to_string());
            }
            let queries = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid DIMACS continuation query count".to_string())?
                .max(1);
            let max_assumptions = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid DIMACS maximum assumptions".to_string())?
                .max(1);
            benchmark_continuation_dimacs(
                Path::new(&args[1]),
                queries,
                max_assumptions,
                Path::new(&args[4]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-reuse-stress" => {
            if args.len() != 8 {
                return Err("usage: layered-sat benchmark-continuation-reuse-stress FAMILY VARS RATIO SEED QUERIES MAX_ASSUMPTIONS OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid continuation stress variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid continuation stress ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid continuation stress seed".to_string())?;
            let queries = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid continuation stress query count".to_string())?
                .max(1);
            let max_assumptions = args[6]
                .parse::<usize>()
                .map_err(|_| "invalid maximum assumption count".to_string())?
                .clamp(1, vars.max(1));
            benchmark_continuation_reuse(
                &args[1],
                vars,
                ratio,
                seed,
                queries,
                max_assumptions,
                Path::new(&args[7]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-temporal-phase" => {
            if args.len() != 7 {
                return Err("usage: continuation-quotient-sat benchmark-continuation-temporal-phase WIDTHS HORIZONS QUERIES MAX_BOUND_BITS SEED OUTPUT.csv (WIDTHS/HORIZONS are comma-separated)".to_string());
            }
            let widths = parse_size_grid(&args[1], "width")?;
            let horizons = parse_size_grid(&args[2], "horizon")?;
            let queries = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid temporal query count".to_string())?
                .max(1);
            let max_bound_bits = args[4]
                .parse::<usize>()
                .map_err(|_| "invalid temporal bound".to_string())?;
            let seed = args[5]
                .parse::<u64>()
                .map_err(|_| "invalid temporal seed".to_string())?;
            benchmark_continuation_temporal_phase(
                &widths,
                &horizons,
                queries,
                max_bound_bits,
                seed,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-temporal-vocabulary" => {
            if args.len() != 8 {
                return Err("usage: continuation-quotient-sat benchmark-temporal-vocabulary KINDS WIDTHS HORIZONS QUERIES MAX_WIDTH SEED OUTPUT.csv (grids are comma-separated)".to_string());
            }
            let kinds: Vec<_> = args[1].split(',').filter(|kind| !kind.is_empty()).collect();
            if kinds.is_empty() {
                return Err("transition kind grid must not be empty".to_string());
            }
            let widths = parse_size_grid(&args[2], "width")?;
            let horizons = parse_size_grid(&args[3], "horizon")?;
            let queries = args[4]
                .parse::<usize>()
                .map_err(|_| "invalid vocabulary query count".to_string())?
                .max(1);
            let max_width = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid vocabulary width gate".to_string())?;
            let seed = args[6]
                .parse::<u64>()
                .map_err(|_| "invalid vocabulary seed".to_string())?;
            benchmark_temporal_vocabulary(
                &kinds,
                &widths,
                &horizons,
                queries,
                max_width,
                seed,
                Path::new(&args[7]),
            )?;
            Ok(true)
        }
        "benchmark-temporal-compositions"
        | "benchmark-local-temporal-compositions"
        | "benchmark-symbolic-temporal-compositions" => {
            if args.len() != 8 {
                return Err("usage: continuation-quotient-sat benchmark-{local-}temporal-compositions KINDS WIDTHS HORIZONS QUERIES MAX_WIDTH SEED OUTPUT.csv (grids are comma-separated)".to_string());
            }
            let kinds: Vec<_> = args[1].split(',').filter(|kind| !kind.is_empty()).collect();
            if kinds.is_empty() {
                return Err("composition kind grid must not be empty".to_string());
            }
            let widths = parse_size_grid(&args[2], "width")?;
            let horizons = parse_size_grid(&args[3], "horizon")?;
            let queries = args[4]
                .parse::<usize>()
                .map_err(|_| "invalid composition query count".to_string())?
                .max(1);
            let max_width = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid composition width gate".to_string())?;
            let seed = args[6]
                .parse::<u64>()
                .map_err(|_| "invalid composition seed".to_string())?;
            if args[0] == "benchmark-symbolic-temporal-compositions" {
                benchmark_symbolic_temporal_compositions(
                    &kinds,
                    &widths,
                    &horizons,
                    queries,
                    max_width,
                    seed,
                    Path::new(&args[7]),
                )?;
            } else {
                benchmark_temporal_compositions(
                    &kinds,
                    &widths,
                    &horizons,
                    queries,
                    max_width,
                    seed,
                    Path::new(&args[7]),
                    args[0] == "benchmark-local-temporal-compositions",
                )?;
            }
            Ok(true)
        }
        "benchmark-symbolic-preimages" => {
            if args.len() != 9 {
                return Err("usage: continuation-quotient-sat benchmark-symbolic-preimages KINDS WIDTHS HORIZONS QUERIES NODE_LIMIT ORDER SEED OUTPUT.csv".to_string());
            }
            let kinds: Vec<_> = args[1].split(',').filter(|kind| !kind.is_empty()).collect();
            let widths = parse_size_grid(&args[2], "width")?;
            let horizons = parse_size_grid(&args[3], "horizon")?;
            let queries = args[4]
                .parse::<usize>()
                .map_err(|_| "invalid preimage query count".to_string())?
                .max(1);
            let node_limit = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid preimage node limit".to_string())?;
            let seed = args[7]
                .parse::<u64>()
                .map_err(|_| "invalid preimage seed".to_string())?;
            benchmark_symbolic_preimages(
                &kinds,
                &widths,
                &horizons,
                queries,
                node_limit,
                seed,
                Path::new(&args[8]),
                &args[6],
            )?;
            Ok(true)
        }
        "benchmark-checkpoint-cdcl" | "benchmark-checkpoint-aig" | "benchmark-checkpoint-lazy" => {
            if args.len() != 9 {
                return Err("usage: continuation-quotient-sat benchmark-checkpoint-cdcl KIND WIDTHS HORIZONS QUERIES CHECKPOINT NODE_LIMIT SEED OUTPUT.csv".to_string());
            }
            let widths = parse_size_grid(&args[2], "width")?;
            let horizons = parse_size_grid(&args[3], "horizon")?;
            let queries = args[4]
                .parse::<usize>()
                .map_err(|_| "invalid checkpoint query count".to_string())?
                .max(1);
            let checkpoint = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid checkpoint frame".to_string())?;
            let node_limit = args[6]
                .parse::<usize>()
                .map_err(|_| "invalid checkpoint node limit".to_string())?;
            let seed = args[7]
                .parse::<u64>()
                .map_err(|_| "invalid checkpoint seed".to_string())?;
            benchmark_checkpoint_cdcl(
                &args[1],
                &widths,
                &horizons,
                queries,
                checkpoint,
                node_limit,
                seed,
                Path::new(&args[8]),
                match args[0].as_str() {
                    "benchmark-checkpoint-aig" => "aig",
                    "benchmark-checkpoint-lazy" => "lazy-bdd",
                    _ => "bdd",
                },
            )?;
            Ok(true)
        }
        "benchmark-native-bdd-theory" => {
            if args.len() != 9 {
                return Err("usage: continuation-quotient-sat benchmark-native-bdd-theory KIND WIDTHS HORIZONS QUERIES CHECKPOINT NODE_LIMIT SEED OUTPUT.csv".to_string());
            }
            benchmark_native_bdd_theory(
                &args[1],
                &parse_size_grid(&args[2], "width")?,
                &parse_size_grid(&args[3], "horizon")?,
                args[4]
                    .parse::<usize>()
                    .map_err(|_| "invalid theory query count".to_string())?
                    .max(1),
                args[5]
                    .parse::<usize>()
                    .map_err(|_| "invalid theory checkpoint".to_string())?,
                args[6]
                    .parse::<usize>()
                    .map_err(|_| "invalid theory node limit".to_string())?,
                args[7]
                    .parse::<u64>()
                    .map_err(|_| "invalid theory seed".to_string())?,
                Path::new(&args[8]),
            )?;
            Ok(true)
        }
        "benchmark-cq-portfolio" => {
            if args.len() != 9 {
                return Err("usage: continuation-quotient-sat benchmark-cq-portfolio KIND WIDTHS HORIZONS QUERIES CHECKPOINT NODE_LIMIT SEED OUTPUT.csv".to_string());
            }
            benchmark_cq_portfolio(
                &args[1],
                &parse_size_grid(&args[2], "width")?,
                &parse_size_grid(&args[3], "horizon")?,
                args[4]
                    .parse::<usize>()
                    .map_err(|_| "invalid portfolio query count".to_string())?
                    .max(1),
                args[5]
                    .parse::<usize>()
                    .map_err(|_| "invalid portfolio checkpoint".to_string())?,
                args[6]
                    .parse::<usize>()
                    .map_err(|_| "invalid portfolio node limit".to_string())?,
                args[7]
                    .parse::<u64>()
                    .map_err(|_| "invalid portfolio seed".to_string())?,
                Path::new(&args[8]),
            )?;
            Ok(true)
        }
        "benchmark-cq-aiger" => {
            if args.len() != 8 {
                return Err("usage: continuation-quotient-sat benchmark-cq-aiger INPUT.aag HORIZON QUERIES CHECKPOINT NODE_LIMIT SEED OUTPUT.csv".to_string());
            }
            benchmark_cq_aiger(
                Path::new(&args[1]),
                args[2]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER horizon".to_string())?
                    .max(1),
                args[3]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER query count".to_string())?
                    .max(1),
                args[4]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER checkpoint".to_string())?,
                args[5]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER node limit".to_string())?,
                args[6]
                    .parse::<u64>()
                    .map_err(|_| "invalid AIGER seed".to_string())?,
                Path::new(&args[7]),
            )?;
            Ok(true)
        }
        "benchmark-aiger-query-reuse" => {
            if args.len() != 5 {
                return Err("usage: continuation-quotient-sat benchmark-aiger-query-reuse INPUT.aag HORIZONS REPEATS OUTPUT.csv".to_string());
            }
            benchmark_aiger_query_reuse(
                Path::new(&args[1]),
                &parse_size_grid(&args[2], "horizon")?,
                args[3]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER reuse repeat count".to_string())?,
                Path::new(&args[4]),
            )?;
            Ok(true)
        }
        "verify-cq-aiger" => {
            if args.len() != 7 {
                return Err("usage: continuation-quotient-sat verify-cq-aiger INPUT.aag HORIZON CHECKPOINT NODE_LIMIT OUTPUT.csv SAFETY_RESULT.txt".to_string());
            }
            verify_cq_aiger(
                Path::new(&args[1]),
                args[2]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER horizon".to_string())?
                    .max(1),
                args[3]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER checkpoint".to_string())?,
                args[4]
                    .parse::<usize>()
                    .map_err(|_| "invalid AIGER node limit".to_string())?,
                Path::new(&args[5]),
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-repairs" => {
            if args.len() != 8 {
                return Err("usage: layered-sat benchmark-continuation-repairs FAMILY VARS RATIO SEED UPDATES QUERIES_PER_UPDATE OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid continuation repair variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid continuation repair ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid continuation repair seed".to_string())?;
            let updates = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid continuation repair update count".to_string())?
                .max(1);
            let queries = args[6]
                .parse::<usize>()
                .map_err(|_| "invalid continuation repair query count".to_string())?
                .max(1);
            benchmark_continuation_repairs(
                &args[1],
                vars,
                ratio,
                seed,
                updates,
                queries,
                Path::new(&args[7]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-hybrid" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-continuation-hybrid FAMILY VARS RATIO SEED PHASES OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid continuation hybrid variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid continuation hybrid ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid continuation hybrid seed".to_string())?;
            let phases = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid continuation hybrid phase count".to_string())?
                .max(1);
            benchmark_continuation_hybrid(
                &args[1],
                vars,
                ratio,
                seed,
                phases,
                1,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-continuation-hybrid-stable" => {
            if args.len() != 8 {
                return Err("usage: layered-sat benchmark-continuation-hybrid-stable FAMILY VARS RATIO SEED PHASES STABLE_SPAN OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid stable hybrid variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid stable hybrid ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid stable hybrid seed".to_string())?;
            let phases = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid stable hybrid phase count".to_string())?
                .max(1);
            let stable_span = args[6]
                .parse::<usize>()
                .map_err(|_| "invalid declared stable span".to_string())?
                .max(1);
            benchmark_continuation_hybrid(
                &args[1],
                vars,
                ratio,
                seed,
                phases,
                stable_span,
                Path::new(&args[7]),
            )?;
            Ok(true)
        }
        "benchmark-holographic-network-cost" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-holographic-network-cost FAMILY VARS RATIO SEED STRATEGY OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid network-cost variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid network-cost ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid network-cost seed".to_string())?;
            let strategy = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid network-cost strategy".to_string())?;
            benchmark_holographic_network_cost(
                &args[1],
                vars,
                ratio,
                seed,
                strategy,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-holographic-tensor-strategies" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-holographic-tensor-strategies FAMILY VARS RATIO SEED STRATEGIES OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid tensor variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid tensor ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid tensor seed".to_string())?;
            let strategies = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid tensor strategy count".to_string())?
                .max(1);
            benchmark_holographic_tensor_strategies(
                &args[1],
                vars,
                ratio,
                seed,
                strategies,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-affine-basis-strategies" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-affine-basis-strategies FAMILY VARS RATIO SEED STRATEGIES OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid affine variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid affine ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid affine seed".to_string())?;
            let strategies = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid affine strategy count".to_string())?
                .max(1);
            benchmark_affine_basis_strategies(
                &args[1],
                vars,
                ratio,
                seed,
                strategies,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-finite-domain-groupings" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-finite-domain-groupings FAMILY VARS RATIO SEED STRATEGIES OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid grouping variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid grouping ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid grouping seed".to_string())?;
            let strategies = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid grouping strategy count".to_string())?
                .max(1);
            benchmark_finite_domain_groupings(
                &args[1],
                vars,
                ratio,
                seed,
                strategies,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-direct-bdd-network-expansion" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-direct-bdd-network-expansion FAMILY VARS RATIO SEED RANDOM-ORDERS OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid direct network variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid direct network ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid direct network seed".to_string())?;
            let random_orders = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid direct network random order count".to_string())?;
            benchmark_direct_bdd_network_expansion(
                &args[1],
                vars,
                ratio,
                seed,
                random_orders,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-bdd-network-expansion" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-bdd-network-expansion FAMILY VARS RATIO SEED RANDOM-ORDERS OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid network variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid network ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid network seed".to_string())?;
            let random_orders = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid network random order count".to_string())?;
            benchmark_bdd_network_expansion(
                &args[1],
                vars,
                ratio,
                seed,
                random_orders,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-frontier-width-strategies" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-frontier-width-strategies FAMILY VARS RATIO SEED RANDOM-ORDERS OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid frontier variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid frontier ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid frontier seed".to_string())?;
            let random_orders = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid random order count".to_string())?;
            benchmark_frontier_width_strategies(
                &args[1],
                vars,
                ratio,
                seed,
                random_orders,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-frozen-width-strategy" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-frozen-width-strategy FAMILY VARS RATIO START-SEED TRIALS OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid frozen variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid frozen ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid frozen seed".to_string())?;
            let trials = args[5]
                .parse::<usize>()
                .map_err(|_| "invalid frozen trial count".to_string())?
                .max(1);
            benchmark_frozen_width_strategy(
                &args[1],
                vars,
                ratio,
                seed,
                trials,
                Path::new(&args[6]),
            )?;
            Ok(true)
        }
        "benchmark-width-strategies" => {
            if args.len() != 6 {
                return Err("usage: layered-sat benchmark-width-strategies FAMILY VARS RATIO SEED OUTPUT.csv".to_string());
            }
            let vars = args[2]
                .parse::<usize>()
                .map_err(|_| "invalid strategy variable count".to_string())?;
            let ratio = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid strategy ratio".to_string())?;
            let seed = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid strategy seed".to_string())?;
            benchmark_width_strategy_search(&args[1], vars, ratio, seed, Path::new(&args[5]))?;
            Ok(true)
        }
        "benchmark-query-calibrated" => {
            if args.len() != 7 {
                return Err("usage: layered-sat benchmark-query-calibrated INPUT.cnf OUTPUT.csv CALIBRATION-QUERIES EVALUATION-QUERIES DEADLINE-MS GATE".to_string());
            }
            let calibration = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid calibration query count".to_string())?
                .max(1);
            let evaluation = args[4]
                .parse::<usize>()
                .map_err(|_| "invalid evaluation query count".to_string())?
                .max(1);
            let deadline_ms = args[5]
                .parse::<u64>()
                .map_err(|_| "invalid calibrated deadline".to_string())?
                .max(1);
            benchmark_query_calibrated(
                Path::new(&args[1]),
                Path::new(&args[2]),
                calibration,
                evaluation,
                std::time::Duration::from_millis(deadline_ms),
                &args[6],
            )?;
            Ok(true)
        }
        "benchmark-query-portfolio" => {
            if args.len() < 5 {
                return Err("usage: layered-sat benchmark-query-portfolio INPUT.cnf OUTPUT.csv QUERIES DEADLINE-MS [GATE ...]".to_string());
            }
            let queries = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid portfolio query count".to_string())?
                .max(1);
            let deadline_ms = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid portfolio deadline".to_string())?
                .max(1);
            benchmark_query_portfolio(
                Path::new(&args[1]),
                Path::new(&args[2]),
                0,
                queries,
                queries,
                std::time::Duration::from_millis(deadline_ms),
                &args[5..],
            )?;
            Ok(true)
        }
        "benchmark-query-micro" => {
            if args.len() < 5 {
                return Err("usage: layered-sat benchmark-query-micro INPUT.cnf OUTPUT.csv QUERIES TIMEOUT-MS [GATE]".to_string());
            }
            let queries = args[3]
                .parse::<usize>()
                .map_err(|_| "invalid micro query count".to_string())?
                .max(1);
            let timeout_ms = args[4]
                .parse::<u64>()
                .map_err(|_| "invalid micro timeout".to_string())?
                .max(1);
            let gate = args.get(5).map(String::as_str).unwrap_or("all");
            let indexed_gate = gate
                .strip_prefix("balanced-only-")
                .or_else(|| gate.strip_prefix("balanced-prefix-"))
                .is_some_and(|value| value.parse::<usize>().is_ok());
            if !matches!(
                gate,
                "all" | "balanced" | "strict" | "none" | "learned" | "topology"
            ) && !indexed_gate
            {
                return Err(format!("unknown solver gate: {gate}"));
            }
            benchmark_query_race(
                Path::new(&args[1]),
                Path::new(&args[2]),
                queries,
                std::time::Duration::from_millis(timeout_ms),
                gate,
            )?;
            Ok(true)
        }
        "export-query-candidate-features" => {
            if args.len() < 4 {
                return Err("usage: layered-sat export-query-candidate-features INPUT.cnf OUTPUT.csv SIGNED-LITERAL ...".to_string());
            }
            let queries = args[3..]
                .iter()
                .map(|value| {
                    let literal: isize = value
                        .parse()
                        .map_err(|_| format!("invalid signed literal: {value}"))?;
                    if literal == 0 {
                        return Err("literal zero is not a query".to_string());
                    }
                    Ok((literal.unsigned_abs() - 1, literal > 0))
                })
                .collect::<Result<Vec<_>, String>>()?;
            export_query_candidate_features(Path::new(&args[1]), Path::new(&args[2]), &queries)?;
            Ok(true)
        }
        "export-balanced-candidates" => {
            if args.len() != 3 {
                return Err(
                    "usage: layered-sat export-balanced-candidates INPUT.cnf OUTPUT.csv"
                        .to_string(),
                );
            }
            export_balanced_candidates(Path::new(&args[1]), Path::new(&args[2]))?;
            Ok(true)
        }
        "benchmark-query-race" => {
            if args.len() < 3 {
                return Err("usage: layered-sat benchmark-query-race INPUT.cnf OUTPUT.csv [queries] [timeout-seconds] [all|balanced|strict|none|learned|topology|balanced-only-N|balanced-prefix-N]".to_string());
            }
            let queries = args
                .get(3)
                .and_then(|value| value.parse().ok())
                .unwrap_or(8usize)
                .max(1);
            let timeout_seconds = args
                .get(4)
                .and_then(|value| value.parse().ok())
                .unwrap_or(10u64)
                .max(1);
            let gate = args.get(5).map(String::as_str).unwrap_or("all");
            let indexed_gate = gate
                .strip_prefix("balanced-only-")
                .or_else(|| gate.strip_prefix("balanced-prefix-"))
                .is_some_and(|value| value.parse::<usize>().is_ok());
            if !matches!(
                gate,
                "all" | "balanced" | "strict" | "none" | "learned" | "topology"
            ) && !indexed_gate
            {
                return Err(format!("unknown solver gate: {gate}"));
            }
            benchmark_query_race(
                Path::new(&args[1]),
                Path::new(&args[2]),
                queries,
                std::time::Duration::from_secs(timeout_seconds),
                gate,
            )?;
            Ok(true)
        }
        "query-race-worker" => {
            if args.len() != 6 {
                return Err("internal query-race-worker usage error".to_string());
            }
            let variable = args[3]
                .parse()
                .map_err(|_| "invalid worker variable".to_string())?;
            let value = args[4]
                .parse()
                .map_err(|_| "invalid worker value".to_string())?;
            query_race_worker(
                &args[1],
                Path::new(&args[2]),
                variable,
                value,
                Path::new(&args[5]),
            )?;
            Ok(true)
        }
        "profile-corpus" => {
            if args.len() < 3 {
                return Err(
                    "usage: layered-sat profile-corpus INPUT_DIR OUTPUT.csv [timeout-seconds]"
                        .to_string(),
                );
            }
            let timeout_seconds = args
                .get(3)
                .and_then(|value| value.parse().ok())
                .unwrap_or(10u64)
                .max(1);
            profile_corpus_isolated(Path::new(&args[1]), Path::new(&args[2]), timeout_seconds)?;
            Ok(true)
        }
        "profile-single" => {
            if args.len() != 3 {
                return Err("internal profile-single usage error".to_string());
            }
            profile_single_formula(Path::new(&args[1]), Path::new(&args[2]))?;
            Ok(true)
        }
        "benchmark-corpus" => {
            if args.len() < 3 {
                return Err(
                    "usage: layered-sat benchmark-corpus INPUT_DIR OUTPUT.csv [queries]"
                        .to_string(),
                );
            }
            let queries = args
                .get(3)
                .and_then(|value| value.parse().ok())
                .unwrap_or(100_000usize)
                .max(1);
            let timeout_seconds = args
                .get(4)
                .and_then(|value| value.parse().ok())
                .unwrap_or(600u64)
                .max(1);
            benchmark_corpus_isolated(
                Path::new(&args[1]),
                Path::new(&args[2]),
                queries,
                timeout_seconds,
            )?;
            Ok(true)
        }
        "benchmark-single" => {
            if args.len() != 4 {
                return Err("internal benchmark-single usage error".to_string());
            }
            let queries = args[3]
                .parse()
                .map_err(|_| "invalid benchmark query count".to_string())?;
            benchmark_corpus_inner(Path::new(&args[1]), Path::new(&args[2]), queries)?;
            Ok(true)
        }
        "compile" => {
            if args.len() < 3 {
                return Err("usage: layered-sat compile INPUT.cnf OUTPUT.lsat [branch-cap] [node-limit] [time-ms]".to_string());
            }
            let branch_cap = args
                .get(3)
                .and_then(|value| value.parse().ok())
                .unwrap_or(64usize)
                .min(64);
            let node_limit = args
                .get(4)
                .and_then(|value| value.parse().ok())
                .unwrap_or(100_000usize);
            let time_ms = args
                .get(5)
                .and_then(|value| value.parse().ok())
                .unwrap_or(100u64);
            let (vars, clauses) = parse_dimacs(Path::new(&args[1]))?;
            let start = Instant::now();
            let artifact = compile_safe_artifact(vars, &clauses, branch_cap, node_limit, time_ms);
            save_compiled_artifact(Path::new(&args[2]), &artifact)?;
            println!(
                "compiled vars={} core_vars={} removed={} seeds={} clauses={} core_clauses={} elapsed_ms={:.3}",
                vars,
                artifact.core_vars,
                vars - artifact.core_vars,
                artifact.seeds.len(),
                clauses.len(),
                artifact.core_clauses.len(),
                start.elapsed().as_secs_f64() * 1000.0
            );
            Ok(true)
        }
        "inspect" => {
            if args.len() != 2 {
                return Err("usage: layered-sat inspect MODEL.lsat".to_string());
            }
            let artifact = load_compiled_artifact(Path::new(&args[1]))?;
            println!(
                "original_vars={} core_vars={} removed={} removed_fraction={:.6} original_clauses={} core_clauses={} seeds={} bdd_nodes={}",
                artifact.original_vars,
                artifact.core_vars,
                artifact.original_vars - artifact.core_vars,
                (artifact.original_vars - artifact.core_vars) as f64
                    / artifact.original_vars.max(1) as f64,
                artifact.original_clauses.len(),
                artifact.core_clauses.len(),
                artifact.seeds.len(),
                artifact
                    .seeds
                    .iter()
                    .map(|seed| seed.manager.nodes.len())
                    .sum::<usize>()
            );
            print!("retained_original_variables=");
            for (index, variable) in artifact.core_to_original.iter().enumerate() {
                if index > 0 {
                    print!(",");
                }
                print!("{}", variable + 1);
            }
            println!();
            Ok(true)
        }
        "query" => {
            if args.len() < 2 {
                return Err("usage: layered-sat query MODEL.lsat [SIGNED-LITERAL ...]".to_string());
            }
            let artifact = load_compiled_artifact(Path::new(&args[1]))?;
            let assumptions = args[2..]
                .iter()
                .map(|value| {
                    let literal: isize = value
                        .parse()
                        .map_err(|_| format!("invalid signed literal: {value}"))?;
                    if literal == 0 {
                        return Err("literal zero is not an assumption".to_string());
                    }
                    Ok((literal.unsigned_abs() - 1, literal > 0))
                })
                .collect::<Result<Vec<_>, String>>()?;
            match query_compiled_artifact(&artifact, &assumptions)? {
                None => println!("s UNSATISFIABLE"),
                Some(assignment) => {
                    println!("s SATISFIABLE");
                    print!("v");
                    for (variable, value) in assignment.iter().enumerate() {
                        let literal = variable as isize + 1;
                        print!(" {}", if *value { literal } else { -literal });
                    }
                    println!(" 0");
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parallel_map<T: Sync, R: Send, F: Fn(&T) -> R + Sync>(items: &[T], function: F) -> Vec<R> {
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(items.len().max(1));
    let chunk_size = items.len().div_ceil(workers);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk_size.max(1))
            .map(|chunk| {
                let function = &function;
                scope.spawn(move || chunk.iter().map(function).collect::<Vec<_>>())
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("experiment worker panicked"))
            .collect()
    })
}

fn encode_cached_formula(record: &CachedHelperFormula) -> String {
    let clauses = record
        .clauses
        .iter()
        .map(|clause| {
            clause
                .0
                .iter()
                .map(|&(variable, sign)| format!("{variable}{}", if sign { '+' } else { '-' }))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        record.vars,
        record.ratio,
        record.family,
        record.seed,
        record.aligned_nodes,
        record.order_nodes,
        record.helper_nodes,
        clauses
    )
}

fn decode_cached_formula(line: &str) -> Option<CachedHelperFormula> {
    let mut fields = line.splitn(8, '|');
    let vars = fields.next()?.parse().ok()?;
    let ratio = fields.next()?.parse().ok()?;
    let family = fields.next()?.to_string();
    let seed = fields.next()?.parse().ok()?;
    let aligned_nodes = fields.next()?.parse().ok()?;
    let order_nodes = fields.next()?.parse().ok()?;
    let helper_nodes = fields.next()?.parse().ok()?;
    let clauses = fields
        .next()?
        .split(';')
        .filter(|clause| !clause.is_empty())
        .map(|clause| {
            Clause(
                clause
                    .split(',')
                    .map(|literal| {
                        let (variable, sign) = literal.split_at(literal.len() - 1);
                        Some((variable.parse().ok()?, sign == "+"))
                    })
                    .collect::<Option<Vec<_>>>()?,
            )
            .into()
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CachedHelperFormula {
        vars,
        ratio,
        family,
        seed,
        clauses,
        aligned_nodes,
        order_nodes,
        helper_nodes,
    })
}

fn helper_cache_path(vars: usize, trials: usize, budget: usize) -> PathBuf {
    PathBuf::from("target/experiment-cache")
        .join(format!("helper-v1-n{vars}-t{trials}-b{budget}.txt"))
}

fn load_or_build_helper_cache(
    vars: usize,
    trials: usize,
    budget: usize,
) -> (Vec<CachedHelperFormula>, bool, u128) {
    let start = Instant::now();
    let path = helper_cache_path(vars, trials, budget);
    if let Ok(contents) = fs::read_to_string(&path) {
        let records: Vec<_> = contents.lines().filter_map(decode_cached_formula).collect();
        if records.len() == 30 * trials {
            return (records, true, start.elapsed().as_micros());
        }
    }
    let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
    let mut tasks = Vec::new();
    for training_vars in sizes {
        for ratio in 2..=6 {
            for family in ["random", "banded"] {
                for training_seed in 1..=trials {
                    tasks.push((training_vars, ratio, family, training_seed));
                }
            }
        }
    }
    let records = parallel_map(&tasks, |&(formula_vars, ratio, family, training_seed)| {
        let seed = training_seed as u64
            + formula_vars as u64 * 1_000
            + ratio as u64 * 100
            + u64::from(family == "banded") * 50_000;
        let clauses = generate_formula(family, formula_vars, ratio, seed);
        let order = min_fill_order(formula_vars, &clauses);
        let aligned = eliminate_with_bdds_ordered(formula_vars, &clauses, &order, &order);
        let rule = ScentRule {
            length: 9,
            hops: 4,
            strongest_first: true,
        };
        let scent_order = apply_scent_rule(formula_vars, &clauses, &order, rule);
        let order_result =
            eliminate_with_bdds_ordered(formula_vars, &clauses, &order, &scent_order);
        let (expanded_vars, expanded, _, _) = scent_batch_expand(formula_vars, &clauses, budget, 4);
        let expanded_elimination = min_fill_order(expanded_vars, &expanded);
        let expanded_order =
            apply_scent_rule(expanded_vars, &expanded, &expanded_elimination, rule);
        let helper_result = eliminate_with_bdds_ordered(
            expanded_vars,
            &expanded,
            &expanded_elimination,
            &expanded_order,
        );
        CachedHelperFormula {
            vars: formula_vars,
            ratio,
            family: family.to_string(),
            seed,
            clauses,
            aligned_nodes: aligned.allocated_nodes,
            order_nodes: order_result.allocated_nodes,
            helper_nodes: helper_result.allocated_nodes,
        }
    });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create experiment cache directory");
    }
    let body = records
        .iter()
        .map(encode_cached_formula)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, body).expect("write experiment cache");
    (records, false, start.elapsed().as_micros())
}

fn main() {
    let args: Vec<_> = env::args().collect();
    match run_firmware_gate_cli(&args[1..]) {
        Ok(Some(true)) => return,
        Ok(Some(false)) => std::process::exit(1),
        Ok(None) => {}
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    }
    match run_artifact_cli(&args[1..]) {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    }
    let vars = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(16);
    let ratio: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let trials: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
    let family = args.get(4).map(String::as_str).unwrap_or("random");
    let order_name = args.get(5).map(String::as_str).unwrap_or("min-fill");
    let engine = args.get(6).map(String::as_str).unwrap_or("compare");
    let optional_budget = args.get(7).and_then(|value| value.parse().ok());
    let helper_budget = optional_budget.unwrap_or(vars / 2);
    let sift_passes = optional_budget.unwrap_or(4);
    let sift_trials = args
        .get(8)
        .and_then(|value| value.parse().ok())
        .unwrap_or(usize::MAX);
    assert!(vars > 0, "vars must be positive");
    if engine == "compare" {
        assert!(
            vars < usize::BITS as usize,
            "compare mode requires vars to fit a bit mask"
        );
    }

    if engine == "bdd-only" {
        println!(
            "family,order,vars,clauses,seed,sat,bdd_solver_us,bdd_allocated_nodes,witness_valid"
        );
    } else if engine == "expand-bdd" {
        println!(
            "family,order,vars,clauses,seed,helpers,expanded_vars,expanded_clauses,original_us,expanded_us,original_nodes,expanded_nodes,node_ratio,sat_equivalent,projected_witness_valid"
        );
    } else if engine == "greedy-expand-bdd" || engine == "aligned-greedy-expand" {
        println!(
            "family,order,vars,clauses,seed,budget,accepted,recursive_accepted,candidates_tested,beneficial_trials,expanded_vars,expanded_clauses,original_nodes,greedy_nodes,node_ratio,original_us,final_us,search_us,sat_equivalent,projected_witness_valid"
        );
    } else if engine == "feedback-expand-bdd" || engine == "shortlist-expand-bdd" {
        println!(
            "family,order,vars,clauses,seed,budget,accepted,recursive_accepted,candidates_tested,beneficial_trials,expanded_vars,expanded_clauses,original_nodes,feedback_nodes,node_ratio,original_us,final_us,search_us,sat_equivalent,projected_witness_valid"
        );
    } else if engine == "bdd-frontier-expand" {
        println!(
            "family,order,vars,clauses,seed,budget,accepted,recursive_accepted,candidates_tested,beneficial_trials,expanded_vars,expanded_clauses,original_nodes,frontier_nodes,node_ratio,original_us,final_us,search_us,sat_equivalent,projected_witness_valid"
        );
    } else if engine == "bdd-order-sweep" {
        println!(
            "family,elimination_order,bdd_order,vars,clauses,seed,sat,bdd_us,allocated_nodes,witness_valid"
        );
    } else if engine == "music-order-sweep" {
        println!(
            "family,elimination_order,musical_characteristic,vars,clauses,seed,sat,bdd_us,allocated_nodes,node_ratio_vs_aligned,witness_valid"
        );
    } else if engine == "learned-phrase" {
        println!(
            "family,vars,clauses,training_trials,test_seed,phrase_length,rule,training_mean_ratio,test_nodes,aligned_nodes,test_ratio,witness_valid"
        );
    } else if engine == "learned-scent" {
        println!(
            "family,vars,clauses,training_trials,test_seed,phrase_length,hops,direction,training_mean_ratio,test_nodes,aligned_nodes,test_ratio,witness_valid"
        );
    } else if engine == "scent-universal" {
        println!(
            "test_family,train_vars,test_vars,density,test_seed,variant,phrase_length,hops,direction,training_mean_ratio,test_nodes,aligned_nodes,test_ratio,witness_valid"
        );
    } else if engine == "scent-gate" {
        println!(
            "family,train_vars,test_vars,density,test_seed,selector,predicted_log_ratio,learned_threshold,oof_ratio,oof_applied,applied,training_samples,gate_us,aligned_nodes,scent_nodes,final_nodes,final_ratio,oracle_applied,witness_valid"
        );
    } else if engine == "scent-helper" {
        println!(
            "family,train_vars,test_vars,density,test_seed,selector,gate_applied,predicted_log_ratio,threshold,budget,accepted,candidates_scored,discovery_us,final_us,baseline_nodes,final_nodes,node_ratio,sat_equivalent,witness_valid"
        );
    } else if engine == "scent-helper-gate" {
        println!(
            "family,train_vars,test_vars,density,test_seed,selector,order_gate,helper_prediction,helper_threshold,helper_oof_ratio,helper_oof_applied,helper_applied,budget,accepted,feature_us,baseline_nodes,order_nodes,helper_nodes,final_nodes,final_ratio,witness_valid"
        );
    } else if engine == "helper-graph-memory" {
        println!(
            "family,train_vars,test_vars,density,test_seed,selector,order_gate,prediction,threshold,oof_ratio,oof_applied,helper_applied,budget,feature_us,baseline_nodes,order_nodes,helper_nodes,final_nodes,final_ratio,witness_valid"
        );
    } else if engine == "learned-message" {
        println!(
            "family,train_vars,test_vars,density,test_seed,selector,order_gate,self_weight,clause_weight,variable_weight,candidate_weight,memory_retention,prediction,threshold,oof_ratio,oof_applied,helper_applied,budget,feature_us,baseline_nodes,order_nodes,helper_nodes,final_nodes,final_ratio,witness_valid"
        );
    } else if engine == "math-tricks" {
        println!(
            "family,vars,input_clauses,seed,output_clauses,tautologies,subsumed,consensus_pairs,passes,preprocess_us,baseline_us,processed_us,baseline_nodes,processed_nodes,node_ratio,sat_equivalent,baseline_witness_valid,processed_witness_valid"
        );
    } else if engine == "incremental-pinch" {
        println!(
            "family,vars,clauses,seed,changed_clause,earliest_layer,reused_layers,recomputed_layers,checkpoint_factors,cache_build_us,incremental_us,full_us,speedup,incremental_new_nodes,full_nodes,node_work_ratio,sat_equivalent,incremental_witness_valid,full_witness_valid"
        );
    } else if engine == "checkpoint-compression" {
        println!(
            "family,vars,clauses,seed,stride,checkpoints,checkpoint_factors,memory_ratio_vs_dense,earliest_layer,restored_layer,replayed_layers,recomputed_layers,cache_build_us,incremental_us,full_us,speedup,new_nodes,full_nodes,node_work_ratio,sat_equivalent,witness_valid"
        );
    } else if engine == "branch-reuse" {
        println!(
            "family,vars,clauses,seed,branch_percent,branch_level,branch_variable,stride,checkpoint_factors,false_sat,true_sat,cache_false_us,incremental_true_us,fresh_false_us,fresh_true_us,pair_speedup,sibling_speedup,new_nodes,fresh_true_nodes,node_work_ratio,true_equivalent,false_witness_valid,true_witness_valid"
        );
    } else if engine == "joint-branch" {
        println!(
            "family,vars,clauses,test_seed,selector,learned_alpha,branch_level,branch_variable,branches,sat,reuse_us,fresh_us,time_speedup,reuse_nodes,fresh_nodes,node_ratio,valid"
        );
    } else if engine == "supervised-branch" {
        println!(
            "family,vars,clauses,test_seed,selector,branch_level,branch_variable,predicted_log_work,inference_us,branches,sat,reuse_us,fresh_us,time_speedup,reuse_nodes,oracle_nodes,work_vs_oracle,fresh_nodes,node_ratio,valid"
        );
    } else if engine == "branch-portfolio"
        || engine == "cheap-branch-portfolio"
        || engine == "three-action-portfolio"
        || engine == "portfolio-generalization"
    {
        println!(
            "family,vars,clauses,test_seed,selector,structure_prediction,policy,branch_level,branch_variable,branches,sat,decision_us,solve_us,total_us,direct_us,speed_vs_direct,work_nodes,direct_nodes,work_vs_direct,oracle_nodes,work_vs_oracle,valid"
        );
    } else if engine == "robust-reuse-portfolio"
        || engine == "external-reuse-portfolio"
        || engine == "external-ablation"
    {
        println!(
            "family,vars,clauses,test_seed,selector,predicted_log_ratio,support_distance,prediction_threshold,distance_threshold,policy,branches,sat,decision_us,solve_us,total_us,direct_us,speed_vs_direct,work_nodes,direct_nodes,work_vs_direct,valid"
        );
    } else if engine == "replay-metrics" {
        println!(
            "family,vars,clauses,test_seed,sat,first_sat,branches,direct_us,reuse_us,speedup,cache_build_us,sibling_us,cache_nodes,sibling_new_nodes,checkpoints,restored_layer,replayed_layers,direct_nodes,reuse_nodes,node_ratio,valid"
        );
    } else if engine == "direct-provenance-portfolio" {
        println!(
            "family,vars,clauses,test_seed,selector,predicted_log_ratio,policy,sat,decision_us,solve_us,total_us,direct_us,speed_vs_direct,nodes,direct_nodes,node_ratio,valid"
        );
    } else if engine == "direct-layout-ablation" {
        println!(
            "family,vars,clauses,test_seed,selector,solve_us,direct_us,speed_vs_direct,nodes,direct_nodes,node_ratio,sat,valid"
        );
    } else if engine == "kernel-scaling" {
        println!("family,vars,clauses,seed,kernel,solve_us,nodes,sat,valid");
    } else if engine == "structure-scaling" {
        println!("family,vars,clauses,seed,min_fill_width,estimated_work,solve_us,nodes,sat,valid");
    } else if engine == "helper-width-scaling" {
        println!(
            "family,original_vars,original_clauses,seed,variant,budget,accepted,expanded_vars,expanded_clauses,min_fill_width,estimated_work,solve_us,nodes,sat,sat_equivalent,witness_valid"
        );
    } else if engine == "exact-width-helper" {
        println!(
            "family,vars,clauses,seed,selector,candidates,helper_added,original_treewidth,final_treewidth,width_change,original_nodes,final_nodes,node_ratio,sat_equivalent,witness_valid"
        );
    } else if engine == "coordinated-width-helper" {
        println!(
            "family,vars,clauses,seed,selector,first_candidates,pairs_tested,helpers_added,original_treewidth,final_treewidth,width_change,original_nodes,final_nodes,node_ratio,sat_equivalent,witness_valid"
        );
    } else if engine == "shake-inverse-width" {
        println!(
            "family,vars,clauses,seed,selector,leaf_richness,gate_applied,inverse_depth,removed,core_vars,core_clauses,probes,inverse_forced,original_treewidth,core_treewidth,width_change,original_nodes,core_nodes,node_ratio,sat,sat_equivalent,reconstruction_valid"
        );
    } else if engine == "seed-branch-width" {
        println!(
            "family,vars,clauses,seed,boundary,interior,local_clauses,summary_clauses,recipe_entries,recipe_bits,compilation_trials,original_treewidth,core_treewidth,width_change,original_nodes,core_nodes,node_ratio,sat,sat_equivalent,reconstruction_valid"
        );
    } else if engine == "seed-bdd-width" {
        println!(
            "family,vars,clauses,seed,seed_order,boundary,interior,local_clauses,summary_clauses,seed_live_nodes,seed_allocated_nodes,compile_us,original_solve_us,core_solve_us,end_to_end_time_ratio,original_treewidth,core_treewidth,width_change,original_nodes,core_nodes,node_ratio,stored_charged_ratio,allocated_charged_ratio,sat,sat_equivalent,reconstruction_valid"
        );
    } else if engine == "seed-bdd-reuse" {
        println!(
            "family,vars,clauses,base_seed,update,interior,cold_allocated,warm_new_allocated,allocation_ratio,cold_us,warm_us,time_ratio,shared_manager_nodes,cache_cap,cache_policy,peak_shared_nodes,compactions,evictions,hot_roots,seed_live_nodes,original_treewidth,core_treewidth,width_change,original_nodes,core_nodes,node_ratio,sat_equivalent,reconstruction_valid"
        );
    } else if engine == "multi-seed-width" {
        println!(
            "family,vars,clauses,seed,selector,branch_cap,max_seeds,seeds,removed,boundary_sum,seed_live_nodes,seed_allocated_nodes,compile_us,final_vars,final_clauses,original_treewidth,final_treewidth,width_change,original_nodes,final_nodes,node_ratio,stored_charged_ratio,original_solve_us,final_solve_us,end_to_end_time_ratio,sat_equivalent,reconstruction_valid"
        );
    } else if engine == "incremental-payback" {
        println!(
            "family,vars,clauses,seed,query,cold_us,incremental_setup_us,incremental_query_us,incremental_cumulative_ratio,seed_compile_us,seed_solve_us,seed_cumulative_ratio,seeds,removed,seed_cache_nodes,cold_sat,incremental_agrees,seed_agrees,incremental_witness_valid,seed_witness_valid"
        );
    } else if engine == "assumption-payback" {
        println!(
            "family,vars,clauses,seed,query,assumed_original,assumed_value,cold_ns,incremental_setup_ns,incremental_query_ns,incremental_cumulative_ratio,seed_setup_ns,seed_query_ns,seed_cumulative_ratio,seeds,removed,core_vars,seed_live_nodes,cold_sat,incremental_agrees,seed_agrees,incremental_witness_valid,seed_witness_valid"
        );
    } else if engine == "deployment-gate" {
        println!(
            "family,vars,clauses,seed,selector,predicted_log_ratio,threshold,applied,actual_ratio,policy_ratio,seeds,removed,live_nodes,incremental_ns,seeded_ns,training_rows,oof_applied,oof_policy_ratio,sat_valid"
        );
    } else if engine == "crossover-gate" {
        println!(
            "family,vars,clauses,seed,selector,predicted_query_ratio,query_margin,predicted_setup_queries,setup_margin,predicted_crossover,query_horizon,applied,actual_query_ratio,actual_crossover,actual_ratio,policy_ratio,seeds,removed,incremental_setup_ns,seeded_setup_ns,incremental_query_ns,seeded_query_ns,training_rows,sat_valid"
        );
    } else if engine == "speculative-compile" {
        println!(
            "family,vars,clauses,seed,selector,predicted_query_ratio,query_margin,budget_ns,compile_ns,calibration_ns,aborted,deployed,seeds,removed,incremental_ns,seeded_ns,policy_ns,policy_ratio,actual_query_ratio,training_rows,sat_valid"
        );
    } else if engine == "offline-scaling" {
        println!(
            "family,vars,clauses,seed,horizon,seeds,removed,core_vars,seed_live_nodes,offline_compile_ns,incremental_setup_ns,incremental_query_ns,seeded_query_ns,online_query_ratio,amortized_ratio,incremental_qps,seeded_qps,reconstruction_ns_per_sample,reconstruction_samples,crossover_queries,sat_valid"
        );
    } else if engine == "batch-branch-discovery" {
        println!(
            "family,vars,clauses,seed,branch_cap,candidates,removed,removed_fraction,boundary_sum,local_clause_touches,discovery_us,target_met,integrity_valid"
        );
    } else if engine == "batch-seed-compile" {
        println!(
            "family,vars,clauses,seed,branch_cap,candidates,removed,removed_fraction,discovery_us,compile_us,seed_live_nodes,seed_allocated_nodes,core_vars,core_clauses,original_sat,core_sat,sat_equivalent,reconstruction_valid,target_met"
        );
    } else if engine == "batch-offline-service" {
        println!(
            "family,vars,clauses,seed,horizon,branch_cap,seeds,removed,removed_fraction,core_vars,discovery_ns,compile_ns,incremental_setup_ns,seeded_solver_setup_ns,incremental_query_ns,seeded_query_ns,online_query_ratio,amortized_ratio,crossover_queries,incremental_qps,seeded_qps,seed_live_nodes,reconstruction_ns,reconstruction_samples,sat_valid"
        );
    } else if engine == "renaming-control" {
        println!(
            "family,vars,clauses,seed,renaming,branch_cap,candidates,removed,removed_fraction,discovery_us,compile_us,seed_live_nodes,seed_allocated_nodes,core_vars,sat_equivalent,reconstruction_valid,target_met"
        );
    } else if engine == "safe-compiler-control" {
        println!(
            "family,layout,vars,clauses,seed,branch_cap,node_limit,time_limit_ms,candidates,accepted,node_rejected,time_rejected,removed,removed_fraction,discovery_us,compile_us,attempt_nodes,seed_live_nodes,core_vars,sat_equivalent,reconstruction_valid,target_met"
        );
    } else if engine == "bdd-sift" || engine == "bdd-sift-control" {
        println!(
            "family,elimination_order,start_order,vars,clauses,seed,passes,swaps_tested,swaps_accepted,start_nodes,final_nodes,node_ratio,start_live,final_live,live_ratio,search_us,final_us,total_vs_start_solves,witness_valid"
        );
    } else if engine == "flower-ring-states" {
        println!(
            "family,vars,clauses,seed,direction,ring,processed,residual_states,best_possible_c6_orbits,total_bdd_nodes"
        );
    } else if engine == "joint-predict" {
        println!(
            "family,vars,clauses,seed,selector,budget,accepted,recursive_accepted,candidates_scored,natural_nodes,aligned_nodes,final_nodes,final_vs_natural,final_vs_aligned,predict_us,final_us,initial_width,final_width,estimated_work_ratio,sat_equivalent,witness_valid"
        );
    } else if engine == "learned-joint" {
        println!(
            "family,vars,clauses,test_seed,selector,budget,accepted,training_samples,training_us,inference_us,final_us,natural_nodes,aligned_nodes,final_nodes,final_vs_natural,final_vs_aligned,sat_equivalent,witness_valid"
        );
    } else {
        assert_eq!(
            engine, "compare",
            "engine must be compare, bdd-only, expand-bdd, greedy-expand-bdd, aligned-greedy-expand, feedback-expand-bdd, shortlist-expand-bdd, bdd-frontier-expand, bdd-order-sweep, music-order-sweep, learned-phrase, learned-scent, scent-universal, scent-gate, scent-helper, scent-helper-gate, helper-graph-memory, learned-message, math-tricks, incremental-pinch, checkpoint-compression, branch-reuse, joint-branch, supervised-branch, branch-portfolio, cheap-branch-portfolio, three-action-portfolio, portfolio-generalization, robust-reuse-portfolio, external-reuse-portfolio, external-ablation, replay-metrics, direct-provenance-portfolio, direct-layout-ablation, kernel-scaling, structure-scaling, helper-width-scaling, exact-width-helper, coordinated-width-helper, shake-inverse-width, seed-branch-width, seed-bdd-width, seed-bdd-reuse, multi-seed-width, incremental-payback, assumption-payback, deployment-gate, crossover-gate, speculative-compile, offline-scaling, batch-branch-discovery, batch-seed-compile, bdd-sift, bdd-sift-control, flower-ring-states, joint-predict, or learned-joint"
        );
        println!(
            "family,order,vars,clauses,seed,sat,peak_boundary,peak_entries,stored_entries,peak_bdd_nodes,stored_bdd_nodes,bdd_ratio,layered_us,bdd_solver_us,bdd_allocated_nodes,bdd_agrees,brute_us,agrees"
        );
    }
    let mut histogram = HashMap::new();
    if engine == "learned-joint" {
        assert_eq!(
            family, "random",
            "learned-joint currently trains on random 3-SAT"
        );
        let training_start = Instant::now();
        let mut samples = Vec::new();
        let mut formula_training = Vec::new();
        for training_seed in 1..=trials {
            let formula = random_3sat(vars, vars * ratio, training_seed as u64);
            let order = min_fill_order(vars, &formula);
            let baseline = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            for (pair, frequency) in recurring_pair_candidates(&formula).into_iter().take(12) {
                let Some(candidate) = add_pair_helper(vars, &formula, pair) else {
                    continue;
                };
                let candidate_order = min_fill_order(vars + 1, &candidate);
                let candidate_result = eliminate_with_bdds_ordered(
                    vars + 1,
                    &candidate,
                    &candidate_order,
                    &candidate_order,
                );
                let features = helper_features(
                    vars,
                    &formula,
                    pair,
                    frequency,
                    &order,
                    &baseline.interaction_candidates,
                );
                let label = candidate_result.allocated_nodes as f64
                    / baseline.allocated_nodes.max(1) as f64
                    - 1.0;
                samples.push((features, label));
            }
            let (frequency_vars, frequency_formula, _) =
                expand_recurring_pairs(vars, &formula, helper_budget);
            let frequency_order = min_fill_order(frequency_vars, &frequency_formula);
            let frequency_result = eliminate_with_bdds_ordered(
                frequency_vars,
                &frequency_formula,
                &frequency_order,
                &frequency_order,
            );
            formula_training.push((
                formula_features(vars, &formula, &baseline),
                frequency_result.allocated_nodes as f64 / baseline.allocated_nodes.max(1) as f64
                    - 1.0,
            ));
        }
        let weights = fit_ridge(&samples, 0.1);
        let mut ranked_training: Vec<_> = samples
            .iter()
            .map(|(features, label)| (predict(&weights, features), *label))
            .collect();
        ranked_training.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut cumulative = 0.0;
        let mut best_cumulative = 0.0;
        let mut learned_threshold = f64::NEG_INFINITY;
        for (prediction, label) in ranked_training {
            cumulative += label;
            if cumulative < best_cumulative {
                best_cumulative = cumulative;
                learned_threshold = prediction + f64::EPSILON;
            }
        }
        let training_us = training_start.elapsed().as_micros();
        for offset in 1..=trials {
            let test_seed = trials + offset;
            let formula = random_3sat(vars, vars * ratio, test_seed as u64);
            let order = min_fill_order(vars, &formula);
            let natural = eliminate_with_bdds(vars, &formula, &order);
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            for selector in ["gated", "learned", "knn", "frequency", "oracle"] {
                let inference_start = Instant::now();
                let (expanded_vars, expanded, accepted) = match selector {
                    "gated" => {
                        let features = formula_features(vars, &formula, &aligned);
                        if formula_knn_prediction(&formula_training, &features, 5) < 0.0 {
                            expand_recurring_pairs(vars, &formula, helper_budget)
                        } else {
                            (vars, formula.clone(), 0)
                        }
                    }
                    "learned" => {
                        let (v, f, accepted, _) = learned_batch_expand(
                            vars,
                            &formula,
                            helper_budget,
                            &weights,
                            &aligned,
                            learned_threshold,
                        );
                        (v, f, accepted)
                    }
                    "knn" => {
                        let (v, f, accepted, _) =
                            knn_batch_expand(vars, &formula, helper_budget, &samples, &aligned, 7);
                        (v, f, accepted)
                    }
                    "frequency" => expand_recurring_pairs(vars, &formula, helper_budget),
                    _ => {
                        let oracle = greedy_expand(
                            vars,
                            &formula,
                            helper_budget,
                            12,
                            "min-fill",
                            test_seed as u64,
                            true,
                        );
                        (oracle.vars, oracle.clauses, oracle.accepted)
                    }
                };
                let inference_us = inference_start.elapsed().as_micros();
                let final_order = min_fill_order(expanded_vars, &expanded);
                let final_start = Instant::now();
                let result = eliminate_with_bdds_ordered(
                    expanded_vars,
                    &expanded,
                    &final_order,
                    &final_order,
                );
                let final_us = final_start.elapsed().as_micros();
                let equivalent = result.assignment.is_some() == natural.assignment.is_some();
                let valid = result.assignment.as_ref().is_none_or(|assignment| {
                    satisfies(&formula, &assignment[..vars]) && satisfies(&expanded, assignment)
                });
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{}",
                    family,
                    vars,
                    formula.len(),
                    test_seed,
                    selector,
                    helper_budget,
                    accepted,
                    samples.len(),
                    training_us,
                    inference_us,
                    final_us,
                    natural.allocated_nodes,
                    aligned.allocated_nodes,
                    result.allocated_nodes,
                    result.allocated_nodes as f64 / natural.allocated_nodes.max(1) as f64,
                    result.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64,
                    equivalent,
                    valid
                );
            }
        }
        return;
    }
    if engine == "learned-phrase" {
        let rules = phrase_rules();
        let mut log_ratios = vec![0.0f64; rules.len()];
        for training_seed in 1..=trials {
            let formula = generate_formula(family, vars, ratio, training_seed as u64);
            let order = min_fill_order(vars, &formula);
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            for (index, &rule) in rules.iter().enumerate() {
                let candidate_order = apply_phrase_rule(vars, &formula, &order, rule);
                let candidate =
                    eliminate_with_bdds_ordered(vars, &formula, &order, &candidate_order);
                let ratio =
                    candidate.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64;
                log_ratios[index] += ratio.ln();
            }
        }
        let (best_index, &best_log_sum) = log_ratios
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        let selected = rules[best_index];
        let training_mean = (best_log_sum / trials.max(1) as f64).exp();
        for test_index in 1..=trials {
            let test_seed = 10_000 + test_index as u64;
            let formula = generate_formula(family, vars, ratio, test_seed);
            let order = min_fill_order(vars, &formula);
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            let candidate_order = apply_phrase_rule(vars, &formula, &order, selected);
            let candidate = eliminate_with_bdds_ordered(vars, &formula, &order, &candidate_order);
            let valid = candidate
                .assignment
                .as_ref()
                .is_none_or(|assignment| satisfies(&formula, assignment));
            println!(
                "{},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                family,
                vars,
                formula.len(),
                trials,
                test_seed,
                selected.length,
                phrase_rule_name(selected),
                training_mean,
                candidate.allocated_nodes,
                aligned.allocated_nodes,
                candidate.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64,
                valid
            );
        }
        return;
    }
    if engine == "learned-scent" {
        let rules = scent_rules();
        let mut log_ratios = vec![0.0f64; rules.len()];
        for training_seed in 1..=trials {
            let formula = generate_formula(family, vars, ratio, training_seed as u64);
            let order = min_fill_order(vars, &formula);
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            for (index, &rule) in rules.iter().enumerate() {
                let candidate_order = apply_scent_rule(vars, &formula, &order, rule);
                let candidate =
                    eliminate_with_bdds_ordered(vars, &formula, &order, &candidate_order);
                let ratio =
                    candidate.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64;
                log_ratios[index] += ratio.ln();
            }
        }
        let (best_index, &best_log_sum) = log_ratios
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        let selected = rules[best_index];
        let training_mean = (best_log_sum / trials.max(1) as f64).exp();
        for test_index in 1..=trials {
            let test_seed = 10_000 + test_index as u64;
            let formula = generate_formula(family, vars, ratio, test_seed);
            let order = min_fill_order(vars, &formula);
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            let candidate_order = apply_scent_rule(vars, &formula, &order, selected);
            let candidate = eliminate_with_bdds_ordered(vars, &formula, &order, &candidate_order);
            let valid = candidate
                .assignment
                .as_ref()
                .is_none_or(|assignment| satisfies(&formula, assignment));
            let direction = if selected.hops == 0 {
                "no-op"
            } else if selected.strongest_first {
                "strongest-first"
            } else {
                "weakest-first"
            };
            println!(
                "{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                family,
                vars,
                formula.len(),
                trials,
                test_seed,
                selected.length,
                selected.hops,
                direction,
                training_mean,
                candidate.allocated_nodes,
                aligned.allocated_nodes,
                candidate.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64,
                valid
            );
        }
        return;
    }
    if engine == "scent-universal" {
        let rules = scent_rules();
        let mut log_ratios = vec![0.0f64; rules.len()];
        let mut training_formulas = 0usize;
        for training_ratio in 3..=5 {
            for training_seed in 1..=trials {
                let formula = random_3sat(vars, vars * training_ratio, training_seed as u64);
                let order = min_fill_order(vars, &formula);
                let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
                for (index, &rule) in rules.iter().enumerate() {
                    let candidate_order = apply_scent_rule(vars, &formula, &order, rule);
                    let candidate =
                        eliminate_with_bdds_ordered(vars, &formula, &order, &candidate_order);
                    let node_ratio =
                        candidate.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64;
                    log_ratios[index] += node_ratio.ln();
                }
                training_formulas += 1;
            }
        }
        let (best_index, &best_log_sum) = log_ratios
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        let selected = rules[best_index];
        let training_mean = (best_log_sum / training_formulas.max(1) as f64).exp();
        let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
        for &test_vars in &sizes {
            for test_ratio in 2..=6 {
                for test_family in ["random", "banded"] {
                    for test_index in 1..=trials {
                        let test_seed = 10_000
                            + test_index as u64
                            + test_vars as u64 * 1_000
                            + test_ratio as u64 * 100
                            + u64::from(test_family == "banded") * 50_000;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, test_seed);
                        let order = min_fill_order(test_vars, &formula);
                        let aligned =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &order);
                        let variants = [
                            ("full", selected, 0u8),
                            (
                                "no-diffusion",
                                ScentRule {
                                    hops: 0,
                                    ..selected
                                },
                                0,
                            ),
                            ("degree-only", selected, 1),
                            ("polarity-only", selected, 2),
                            ("random-control", selected, 3),
                        ];
                        for (variant_name, rule, signal_variant) in variants {
                            let candidate_order = apply_scent_rule_variant(
                                test_vars,
                                &formula,
                                &order,
                                rule,
                                signal_variant,
                            );
                            let candidate = eliminate_with_bdds_ordered(
                                test_vars,
                                &formula,
                                &order,
                                &candidate_order,
                            );
                            let valid = candidate
                                .assignment
                                .as_ref()
                                .is_none_or(|assignment| satisfies(&formula, assignment));
                            let direction = if selected.strongest_first {
                                "strongest-first"
                            } else {
                                "weakest-first"
                            };
                            println!(
                                "{},{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                                test_family,
                                vars,
                                test_vars,
                                test_ratio,
                                test_seed,
                                variant_name,
                                selected.length,
                                rule.hops,
                                direction,
                                training_mean,
                                candidate.allocated_nodes,
                                aligned.allocated_nodes,
                                candidate.allocated_nodes as f64
                                    / aligned.allocated_nodes.max(1) as f64,
                                valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "scent-gate" {
        let fixed_rule = ScentRule {
            length: 9,
            hops: 4,
            strongest_first: true,
        };
        let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
        let mut training = Vec::new();
        for &training_vars in &sizes {
            for training_ratio in 2..=6 {
                for training_family in ["random", "banded"] {
                    for training_seed in 1..=trials {
                        let seed = training_seed as u64
                            + training_vars as u64 * 1_000
                            + training_ratio as u64 * 100
                            + u64::from(training_family == "banded") * 50_000;
                        let formula =
                            generate_formula(training_family, training_vars, training_ratio, seed);
                        let order = min_fill_order(training_vars, &formula);
                        let aligned =
                            eliminate_with_bdds_ordered(training_vars, &formula, &order, &order);
                        let scent_order =
                            apply_scent_rule(training_vars, &formula, &order, fixed_rule);
                        let scent = eliminate_with_bdds_ordered(
                            training_vars,
                            &formula,
                            &order,
                            &scent_order,
                        );
                        training.push((
                            scent_gate_features(training_vars, &formula),
                            (scent.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64)
                                .ln(),
                        ));
                    }
                }
            }
        }
        let (learned_threshold, oof_ratio, oof_applied) = learn_scent_gate_threshold(&training, 11);
        for &test_vars in &sizes {
            for test_ratio in 2..=6 {
                for test_family in ["random", "banded"] {
                    for test_index in 1..=trials {
                        let test_seed = 100_000
                            + test_index as u64
                            + test_vars as u64 * 1_000
                            + test_ratio as u64 * 100
                            + u64::from(test_family == "banded") * 50_000;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, test_seed);
                        let order = min_fill_order(test_vars, &formula);
                        let aligned =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &order);
                        let scent_order = apply_scent_rule(test_vars, &formula, &order, fixed_rule);
                        let scent =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &scent_order);
                        let start = Instant::now();
                        let features = scent_gate_features(test_vars, &formula);
                        let prediction = scent_gate_predict(&training, &features, 11);
                        let gate_us = start.elapsed().as_micros();
                        let oracle_applied = scent.allocated_nodes < aligned.allocated_nodes;
                        for selector in ["always", "zero-gate", "oof-gate", "oracle", "off"] {
                            let applied = match selector {
                                "always" => true,
                                "zero-gate" => prediction < 0.0,
                                "oof-gate" => prediction < learned_threshold,
                                "oracle" => oracle_applied,
                                "off" => false,
                                _ => unreachable!(),
                            };
                            let final_result = if applied { &scent } else { &aligned };
                            let valid = final_result
                                .assignment
                                .as_ref()
                                .is_none_or(|assignment| satisfies(&formula, assignment));
                            println!(
                                "{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{:.6},{},{}",
                                test_family,
                                vars,
                                test_vars,
                                test_ratio,
                                test_seed,
                                selector,
                                prediction,
                                learned_threshold,
                                oof_ratio,
                                oof_applied,
                                applied,
                                training.len(),
                                gate_us,
                                aligned.allocated_nodes,
                                scent.allocated_nodes,
                                final_result.allocated_nodes,
                                final_result.allocated_nodes as f64
                                    / aligned.allocated_nodes.max(1) as f64,
                                oracle_applied,
                                valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "learned-message" {
        let fixed_rule = ScentRule {
            length: 9,
            hops: 4,
            strongest_first: true,
        };
        let budget = optional_budget.unwrap_or(3).min(6);
        let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
        let (cached, cache_hit, cache_us) = load_or_build_helper_cache(vars, trials, budget);
        eprintln!("training_cache_hit={cache_hit},training_cache_us={cache_us}");
        let order_training: Vec<_> = cached
            .iter()
            .map(|record| {
                (
                    scent_gate_features(record.vars, &record.clauses),
                    (record.order_nodes as f64 / record.aligned_nodes.max(1) as f64).ln(),
                )
            })
            .collect();
        let labeled_formulas: Vec<_> = cached
            .into_iter()
            .map(|record| {
                (
                    record.vars,
                    record.clauses,
                    (record.helper_nodes as f64 / record.order_nodes.max(1) as f64).ln(),
                )
            })
            .collect();
        let (order_threshold, _, _) = learn_scent_gate_threshold(&order_training, 11);
        let mut models = Vec::new();
        for mode in 0..=1u8 {
            let mut best: Option<(MessageParams, Vec<(Vec<f64>, f64)>, (f64, f64, usize))> = None;
            let mut screened: Vec<_> = message_parameter_candidates(mode == 1)
                .into_iter()
                .map(|params| {
                    let samples: Vec<_> = labeled_formulas
                        .iter()
                        .take(60)
                        .map(|(formula_vars, formula, label)| {
                            (
                                helper_graph_features_with_params(
                                    *formula_vars,
                                    formula,
                                    4,
                                    12,
                                    mode,
                                    params,
                                ),
                                *label,
                            )
                        })
                        .collect();
                    let score = learn_vector_threshold(&samples, 7).1;
                    (score, params)
                })
                .collect();
            screened.sort_by(|a, b| a.0.total_cmp(&b.0));
            screened.truncate(4);
            for (_, params) in screened {
                let samples: Vec<_> = labeled_formulas
                    .iter()
                    .map(|(formula_vars, formula, label)| {
                        (
                            helper_graph_features_with_params(
                                *formula_vars,
                                formula,
                                4,
                                12,
                                mode,
                                params,
                            ),
                            *label,
                        )
                    })
                    .collect();
                let threshold = learn_vector_threshold(&samples, 11);
                if best.as_ref().is_none_or(|current| {
                    threshold.1 < current.2.1
                        || (threshold.1 == current.2.1 && threshold.2 < current.2.2)
                }) {
                    best = Some((params, samples, threshold));
                }
            }
            let learned = best.unwrap();
            let fixed_samples: Vec<_> = labeled_formulas
                .iter()
                .map(|(formula_vars, formula, label)| {
                    (
                        helper_graph_features_with_params(
                            *formula_vars,
                            formula,
                            4,
                            12,
                            mode,
                            DEFAULT_MESSAGE_PARAMS,
                        ),
                        *label,
                    )
                })
                .collect();
            let fixed_threshold = learn_vector_threshold(&fixed_samples, 11);
            models.push((mode, DEFAULT_MESSAGE_PARAMS, fixed_samples, fixed_threshold));
            models.push((mode, learned.0, learned.1, learned.2));
        }
        for &test_vars in &sizes {
            for test_ratio in 2..=6 {
                for test_family in ["random", "banded"] {
                    for test_index in 1..=sift_trials.min(trials) {
                        let test_seed = 500_000
                            + test_index as u64
                            + test_vars as u64 * 1_000
                            + test_ratio as u64 * 100
                            + u64::from(test_family == "banded") * 50_000;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, test_seed);
                        let order = min_fill_order(test_vars, &formula);
                        let baseline =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &order);
                        let scent_order = apply_scent_rule(test_vars, &formula, &order, fixed_rule);
                        let order_result =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &scent_order);
                        let order_gate = scent_gate_predict(
                            &order_training,
                            &scent_gate_features(test_vars, &formula),
                            11,
                        ) < order_threshold;
                        let (expanded_vars, expanded, _, _) =
                            scent_batch_expand(test_vars, &formula, budget, 4);
                        let expanded_elimination = min_fill_order(expanded_vars, &expanded);
                        let expanded_order = apply_scent_rule(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            fixed_rule,
                        );
                        let helper_result = eliminate_with_bdds_ordered(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            &expanded_order,
                        );
                        let feature_start = Instant::now();
                        let evaluated: Vec<_> = models
                            .iter()
                            .map(|(mode, params, samples, threshold)| {
                                let features = helper_graph_features_with_params(
                                    test_vars, &formula, 4, 12, *mode, *params,
                                );
                                (vector_knn_predict(samples, &features, 11), *threshold)
                            })
                            .collect();
                        let feature_us = feature_start.elapsed().as_micros();
                        let beneficial =
                            helper_result.allocated_nodes < order_result.allocated_nodes;
                        for selector in [
                            "fixed-full",
                            "learned-full",
                            "fixed-ghost",
                            "learned-ghost",
                            "order-only",
                            "unconditional",
                            "oracle",
                        ] {
                            let model_index = match selector {
                                "fixed-full" => Some(0usize),
                                "learned-full" => Some(1usize),
                                "fixed-ghost" => Some(2usize),
                                "learned-ghost" => Some(3usize),
                                _ => None,
                            };
                            let (params, prediction, threshold) = if let Some(index) = model_index {
                                (models[index].1, evaluated[index].0, evaluated[index].1.0)
                            } else {
                                (DEFAULT_MESSAGE_PARAMS, 0.0, 0.0)
                            };
                            let helper_applied = order_gate
                                && match selector {
                                    "order-only" => false,
                                    "unconditional" => true,
                                    "oracle" => beneficial,
                                    _ => prediction < threshold,
                                };
                            let final_result = if !order_gate {
                                &baseline
                            } else if helper_applied {
                                &helper_result
                            } else {
                                &order_result
                            };
                            let threshold_info = model_index
                                .map(|index| models[index].3)
                                .unwrap_or((0.0, 1.0, 0));
                            let valid = final_result.assignment.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, &assignment[..test_vars])
                            });
                            println!(
                                "{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{},{:.6},{}",
                                test_family,
                                vars,
                                test_vars,
                                test_ratio,
                                test_seed,
                                selector,
                                order_gate,
                                params.self_weight,
                                params.clause_weight,
                                params.variable_weight,
                                params.candidate_weight,
                                params.memory_retention,
                                prediction,
                                threshold,
                                threshold_info.1,
                                threshold_info.2,
                                helper_applied,
                                budget,
                                feature_us,
                                baseline.allocated_nodes,
                                order_result.allocated_nodes,
                                helper_result.allocated_nodes,
                                final_result.allocated_nodes,
                                final_result.allocated_nodes as f64
                                    / baseline.allocated_nodes.max(1) as f64,
                                valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "helper-graph-memory" {
        let fixed_rule = ScentRule {
            length: 9,
            hops: 4,
            strongest_first: true,
        };
        let budget = optional_budget.unwrap_or(3).min(6);
        let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
        let mut order_training = Vec::new();
        let mut graph_training: [Vec<(Vec<f64>, f64)>; 3] = std::array::from_fn(|_| Vec::new());
        for &training_vars in &sizes {
            for training_ratio in 2..=6 {
                for training_family in ["random", "banded"] {
                    for training_seed in 1..=trials {
                        let seed = training_seed as u64
                            + training_vars as u64 * 1_000
                            + training_ratio as u64 * 100
                            + u64::from(training_family == "banded") * 50_000;
                        let formula =
                            generate_formula(training_family, training_vars, training_ratio, seed);
                        let order = min_fill_order(training_vars, &formula);
                        let aligned =
                            eliminate_with_bdds_ordered(training_vars, &formula, &order, &order);
                        let scent_order =
                            apply_scent_rule(training_vars, &formula, &order, fixed_rule);
                        let order_result = eliminate_with_bdds_ordered(
                            training_vars,
                            &formula,
                            &order,
                            &scent_order,
                        );
                        order_training.push((
                            scent_gate_features(training_vars, &formula),
                            (order_result.allocated_nodes as f64
                                / aligned.allocated_nodes.max(1) as f64)
                                .ln(),
                        ));
                        let (expanded_vars, expanded, _, _) =
                            scent_batch_expand(training_vars, &formula, budget, fixed_rule.hops);
                        let expanded_elimination = min_fill_order(expanded_vars, &expanded);
                        let expanded_order = apply_scent_rule(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            fixed_rule,
                        );
                        let helper_result = eliminate_with_bdds_ordered(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            &expanded_order,
                        );
                        let label = (helper_result.allocated_nodes as f64
                            / order_result.allocated_nodes.max(1) as f64)
                            .ln();
                        for mode in 0..3 {
                            graph_training[mode].push((
                                helper_graph_features(
                                    training_vars,
                                    &formula,
                                    fixed_rule.hops,
                                    12,
                                    mode as u8,
                                ),
                                label,
                            ));
                        }
                    }
                }
            }
        }
        let (order_threshold, _, _) = learn_scent_gate_threshold(&order_training, 11);
        let graph_thresholds: Vec<_> = graph_training
            .iter()
            .map(|training| learn_vector_threshold(training, 11))
            .collect();
        for &test_vars in &sizes {
            for test_ratio in 2..=6 {
                for test_family in ["random", "banded"] {
                    for test_index in 1..=trials {
                        let test_seed = 400_000
                            + test_index as u64
                            + test_vars as u64 * 1_000
                            + test_ratio as u64 * 100
                            + u64::from(test_family == "banded") * 50_000;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, test_seed);
                        let order = min_fill_order(test_vars, &formula);
                        let baseline =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &order);
                        let scent_order = apply_scent_rule(test_vars, &formula, &order, fixed_rule);
                        let order_result =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &scent_order);
                        let order_prediction = scent_gate_predict(
                            &order_training,
                            &scent_gate_features(test_vars, &formula),
                            11,
                        );
                        let order_gate = order_prediction < order_threshold;
                        let (expanded_vars, expanded, _, _) =
                            scent_batch_expand(test_vars, &formula, budget, fixed_rule.hops);
                        let expanded_elimination = min_fill_order(expanded_vars, &expanded);
                        let expanded_order = apply_scent_rule(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            fixed_rule,
                        );
                        let helper_result = eliminate_with_bdds_ordered(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            &expanded_order,
                        );
                        let feature_start = Instant::now();
                        let graph_features: Vec<_> = (0..3)
                            .map(|mode| {
                                helper_graph_features(
                                    test_vars,
                                    &formula,
                                    fixed_rule.hops,
                                    12,
                                    mode,
                                )
                            })
                            .collect();
                        let predictions: Vec<_> = (0..3)
                            .map(|mode| {
                                vector_knn_predict(&graph_training[mode], &graph_features[mode], 11)
                            })
                            .collect();
                        let feature_us = feature_start.elapsed().as_micros();
                        let beneficial =
                            helper_result.allocated_nodes < order_result.allocated_nodes;
                        for selector in [
                            "order-only",
                            "full-graph",
                            "ghost-memory",
                            "forget-pruned",
                            "unconditional",
                            "oracle",
                        ] {
                            let mode = match selector {
                                "full-graph" => Some(0usize),
                                "ghost-memory" => Some(1usize),
                                "forget-pruned" => Some(2usize),
                                _ => None,
                            };
                            let prediction = mode.map(|index| predictions[index]).unwrap_or(0.0);
                            let (threshold, oof_ratio, oof_applied) = mode
                                .map(|index| graph_thresholds[index])
                                .unwrap_or((0.0, 1.0, 0));
                            let helper_applied = order_gate
                                && match selector {
                                    "order-only" => false,
                                    "unconditional" => true,
                                    "oracle" => beneficial,
                                    _ => prediction < threshold,
                                };
                            let final_result = if !order_gate {
                                &baseline
                            } else if helper_applied {
                                &helper_result
                            } else {
                                &order_result
                            };
                            let valid = final_result.assignment.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, &assignment[..test_vars])
                            });
                            println!(
                                "{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{},{:.6},{}",
                                test_family,
                                vars,
                                test_vars,
                                test_ratio,
                                test_seed,
                                selector,
                                order_gate,
                                prediction,
                                threshold,
                                oof_ratio,
                                oof_applied,
                                helper_applied,
                                budget,
                                feature_us,
                                baseline.allocated_nodes,
                                order_result.allocated_nodes,
                                helper_result.allocated_nodes,
                                final_result.allocated_nodes,
                                final_result.allocated_nodes as f64
                                    / baseline.allocated_nodes.max(1) as f64,
                                valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "scent-helper-gate" {
        let fixed_rule = ScentRule {
            length: 9,
            hops: 4,
            strongest_first: true,
        };
        let budget = optional_budget.unwrap_or(3).min(6);
        let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
        let mut order_training = Vec::new();
        let mut helper_training = Vec::new();
        for &training_vars in &sizes {
            for training_ratio in 2..=6 {
                for training_family in ["random", "banded"] {
                    for training_seed in 1..=trials {
                        let seed = training_seed as u64
                            + training_vars as u64 * 1_000
                            + training_ratio as u64 * 100
                            + u64::from(training_family == "banded") * 50_000;
                        let formula =
                            generate_formula(training_family, training_vars, training_ratio, seed);
                        let order = min_fill_order(training_vars, &formula);
                        let aligned =
                            eliminate_with_bdds_ordered(training_vars, &formula, &order, &order);
                        let scent_order =
                            apply_scent_rule(training_vars, &formula, &order, fixed_rule);
                        let order_result = eliminate_with_bdds_ordered(
                            training_vars,
                            &formula,
                            &order,
                            &scent_order,
                        );
                        order_training.push((
                            scent_gate_features(training_vars, &formula),
                            (order_result.allocated_nodes as f64
                                / aligned.allocated_nodes.max(1) as f64)
                                .ln(),
                        ));
                        let (expanded_vars, expanded, _, _) =
                            scent_batch_expand(training_vars, &formula, budget, fixed_rule.hops);
                        let expanded_elimination = min_fill_order(expanded_vars, &expanded);
                        let expanded_order = apply_scent_rule(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            fixed_rule,
                        );
                        let helper_result = eliminate_with_bdds_ordered(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            &expanded_order,
                        );
                        helper_training.push((
                            helper_gate_features(training_vars, &formula, fixed_rule.hops, budget),
                            (helper_result.allocated_nodes as f64
                                / order_result.allocated_nodes.max(1) as f64)
                                .ln(),
                        ));
                    }
                }
            }
        }
        let (order_threshold, _, _) = learn_scent_gate_threshold(&order_training, 11);
        let (helper_threshold, helper_oof_ratio, helper_oof_applied) =
            learn_helper_gate_threshold(&helper_training, 11);
        for &test_vars in &sizes {
            for test_ratio in 2..=6 {
                for test_family in ["random", "banded"] {
                    for test_index in 1..=trials {
                        let test_seed = 300_000
                            + test_index as u64
                            + test_vars as u64 * 1_000
                            + test_ratio as u64 * 100
                            + u64::from(test_family == "banded") * 50_000;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, test_seed);
                        let order = min_fill_order(test_vars, &formula);
                        let baseline =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &order);
                        let scent_order = apply_scent_rule(test_vars, &formula, &order, fixed_rule);
                        let order_result =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &scent_order);
                        let order_prediction = scent_gate_predict(
                            &order_training,
                            &scent_gate_features(test_vars, &formula),
                            11,
                        );
                        let order_gate = order_prediction < order_threshold;
                        let feature_start = Instant::now();
                        let helper_features =
                            helper_gate_features(test_vars, &formula, fixed_rule.hops, budget);
                        let helper_prediction =
                            helper_gate_predict(&helper_training, &helper_features, 11);
                        let feature_us = feature_start.elapsed().as_micros();
                        let (expanded_vars, expanded, accepted, _) =
                            scent_batch_expand(test_vars, &formula, budget, fixed_rule.hops);
                        let expanded_elimination = min_fill_order(expanded_vars, &expanded);
                        let expanded_order = apply_scent_rule(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            fixed_rule,
                        );
                        let helper_result = eliminate_with_bdds_ordered(
                            expanded_vars,
                            &expanded,
                            &expanded_elimination,
                            &expanded_order,
                        );
                        let helper_beneficial =
                            helper_result.allocated_nodes < order_result.allocated_nodes;
                        for selector in [
                            "order-only",
                            "unconditional-helper",
                            "helper-gate",
                            "helper-oracle",
                        ] {
                            let helper_applied = order_gate
                                && match selector {
                                    "order-only" => false,
                                    "unconditional-helper" => true,
                                    "helper-gate" => helper_prediction < helper_threshold,
                                    "helper-oracle" => helper_beneficial,
                                    _ => unreachable!(),
                                };
                            let final_result = if !order_gate {
                                &baseline
                            } else if helper_applied {
                                &helper_result
                            } else {
                                &order_result
                            };
                            let valid = final_result.assignment.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, &assignment[..test_vars])
                            });
                            println!(
                                "{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{},{},{:.6},{}",
                                test_family,
                                vars,
                                test_vars,
                                test_ratio,
                                test_seed,
                                selector,
                                order_gate,
                                helper_prediction,
                                helper_threshold,
                                helper_oof_ratio,
                                helper_oof_applied,
                                helper_applied,
                                budget,
                                if helper_applied { accepted } else { 0 },
                                feature_us,
                                baseline.allocated_nodes,
                                order_result.allocated_nodes,
                                helper_result.allocated_nodes,
                                final_result.allocated_nodes,
                                final_result.allocated_nodes as f64
                                    / baseline.allocated_nodes.max(1) as f64,
                                valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "scent-helper" {
        let fixed_rule = ScentRule {
            length: 9,
            hops: 4,
            strongest_first: true,
        };
        let budget = optional_budget.unwrap_or(3).min(6);
        let sizes = [vars.saturating_sub(6).max(6), vars, vars + 6];
        let mut training = Vec::new();
        for &training_vars in &sizes {
            for training_ratio in 2..=6 {
                for training_family in ["random", "banded"] {
                    for training_seed in 1..=trials {
                        let seed = training_seed as u64
                            + training_vars as u64 * 1_000
                            + training_ratio as u64 * 100
                            + u64::from(training_family == "banded") * 50_000;
                        let formula =
                            generate_formula(training_family, training_vars, training_ratio, seed);
                        let order = min_fill_order(training_vars, &formula);
                        let aligned =
                            eliminate_with_bdds_ordered(training_vars, &formula, &order, &order);
                        let scent_order =
                            apply_scent_rule(training_vars, &formula, &order, fixed_rule);
                        let scent = eliminate_with_bdds_ordered(
                            training_vars,
                            &formula,
                            &order,
                            &scent_order,
                        );
                        training.push((
                            scent_gate_features(training_vars, &formula),
                            (scent.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64)
                                .ln(),
                        ));
                    }
                }
            }
        }
        let (threshold, _, _) = learn_scent_gate_threshold(&training, 11);
        for &test_vars in &sizes {
            for test_ratio in 2..=6 {
                for test_family in ["random", "banded"] {
                    for test_index in 1..=trials {
                        let test_seed = 200_000
                            + test_index as u64
                            + test_vars as u64 * 1_000
                            + test_ratio as u64 * 100
                            + u64::from(test_family == "banded") * 50_000;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, test_seed);
                        let order = min_fill_order(test_vars, &formula);
                        let baseline =
                            eliminate_with_bdds_ordered(test_vars, &formula, &order, &order);
                        let features = scent_gate_features(test_vars, &formula);
                        let prediction = scent_gate_predict(&training, &features, 11);
                        let gate_applied = prediction < threshold;
                        for selector in [
                            "order-only",
                            "frequency-helper",
                            "scent-helper",
                            "greedy-oracle",
                        ] {
                            let discovery_start = Instant::now();
                            let (expanded_vars, expanded, accepted, scored) = if !gate_applied
                                || selector == "order-only"
                            {
                                (test_vars, formula.clone(), 0, 0)
                            } else if selector == "frequency-helper" {
                                let (v, c, a) = expand_recurring_pairs(test_vars, &formula, budget);
                                (v, c, a, recurring_pair_candidates(&formula).len())
                            } else if selector == "scent-helper" {
                                scent_batch_expand(test_vars, &formula, budget, fixed_rule.hops)
                            } else {
                                let greedy = greedy_expand(
                                    test_vars, &formula, budget, 12, "min-fill", test_seed, true,
                                );
                                (
                                    greedy.vars,
                                    greedy.clauses,
                                    greedy.accepted,
                                    greedy.candidates_tested,
                                )
                            };
                            let discovery_us = discovery_start.elapsed().as_micros();
                            let expanded_elimination = min_fill_order(expanded_vars, &expanded);
                            let expanded_bdd_order = if gate_applied {
                                apply_scent_rule(
                                    expanded_vars,
                                    &expanded,
                                    &expanded_elimination,
                                    fixed_rule,
                                )
                            } else {
                                expanded_elimination.clone()
                            };
                            let final_start = Instant::now();
                            let final_result = eliminate_with_bdds_ordered(
                                expanded_vars,
                                &expanded,
                                &expanded_elimination,
                                &expanded_bdd_order,
                            );
                            let final_us = final_start.elapsed().as_micros();
                            let equivalent =
                                final_result.assignment.is_some() == baseline.assignment.is_some();
                            let valid = final_result.assignment.as_ref().is_none_or(|assignment| {
                                satisfies(&formula, &assignment[..test_vars])
                                    && satisfies(&expanded, assignment)
                            });
                            println!(
                                "{},{},{},{},{},{},{},{:.6},{:.6},{},{},{},{},{},{},{},{:.6},{},{}",
                                test_family,
                                vars,
                                test_vars,
                                test_ratio,
                                test_seed,
                                selector,
                                gate_applied,
                                prediction,
                                threshold,
                                budget,
                                accepted,
                                scored,
                                discovery_us,
                                final_us,
                                baseline.allocated_nodes,
                                final_result.allocated_nodes,
                                final_result.allocated_nodes as f64
                                    / baseline.allocated_nodes.max(1) as f64,
                                equivalent,
                                valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "coordinated-width-helper" {
        assert!(
            vars <= 12,
            "coordinated-width-helper supports at most 12 original variables"
        );
        let candidate_limit = optional_budget.unwrap_or(6);
        for seed in 1..=trials {
            let formula = generate_formula(family, vars, ratio, 110_000 + seed as u64);
            let original_treewidth = exact_treewidth(vars, &formula);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let first_candidates: Vec<_> = recurring_pair_candidates(&formula)
                .into_iter()
                .take(candidate_limit)
                .collect();
            let mut pairs_tested = 0usize;
            let mut oracle: Option<(usize, Vec<Clause>, usize)> = None;
            for &(first_pair, _) in &first_candidates {
                let Some(first_formula) = add_pair_helper(vars, &formula, first_pair) else {
                    continue;
                };
                let second_candidates: Vec<_> = recurring_pair_candidates(&first_formula)
                    .into_iter()
                    .take(candidate_limit)
                    .collect();
                for &(second_pair, _) in &second_candidates {
                    let Some(second_formula) =
                        add_pair_helper(vars + 1, &first_formula, second_pair)
                    else {
                        continue;
                    };
                    pairs_tested += 1;
                    let width = exact_treewidth(vars + 2, &second_formula);
                    if oracle.as_ref().is_none_or(|candidate| width < candidate.2) {
                        oracle = Some((vars + 2, second_formula, width));
                    }
                }
            }
            let (frequency_vars, frequency_formula, frequency_added) =
                expand_two_greedy(vars, &formula, candidate_limit, false);
            let (warp_vars, warp_formula, warp_added) =
                expand_two_greedy(vars, &formula, candidate_limit, true);
            let variants = [
                ("original", vars, formula.clone(), 0, original_treewidth),
                (
                    "frequency-two",
                    frequency_vars,
                    frequency_formula,
                    frequency_added,
                    0,
                ),
                ("warp-two", warp_vars, warp_formula, warp_added, 0),
                oracle.map_or_else(
                    || ("oracle-two", vars, formula.clone(), 0, original_treewidth),
                    |(oracle_vars, oracle_formula, width)| {
                        ("oracle-two", oracle_vars, oracle_formula, 2, width)
                    },
                ),
            ];
            for (selector, expanded_vars, expanded, added, known_width) in variants {
                let final_treewidth = if known_width > 0 {
                    known_width
                } else {
                    exact_treewidth(expanded_vars, &expanded)
                };
                let order = min_fill_order(expanded_vars, &expanded);
                let mut rank = vec![0usize; expanded_vars];
                for (level, &variable) in order.iter().enumerate() {
                    rank[variable] = level;
                }
                let mapped: Vec<_> = expanded
                    .iter()
                    .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                    .collect();
                let result = solve_tuple_natural(expanded_vars, &mapped);
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{}",
                    family,
                    vars,
                    formula.len(),
                    110_000 + seed as u64,
                    selector,
                    first_candidates.len(),
                    pairs_tested,
                    added,
                    original_treewidth,
                    final_treewidth,
                    final_treewidth as isize - original_treewidth as isize,
                    original_result.nodes,
                    result.nodes,
                    result.nodes as f64 / original_result.nodes.max(1) as f64,
                    result.assignment.is_some() == original_result.assignment.is_some(),
                    result
                        .assignment
                        .as_ref()
                        .is_none_or(|assignment| satisfies(&mapped, assignment))
                );
            }
        }
        return;
    }
    if engine == "shake-inverse-width" {
        assert!(
            vars <= 16,
            "shake-inverse-width supports at most 16 variables"
        );
        for seed in 1..=trials {
            let formula_seed = 120_000 + seed as u64;
            let formula = generate_formula(family, vars, ratio, formula_seed);
            let original_treewidth = exact_treewidth(vars, &formula);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let original_sat = original_result.assignment.is_some();

            let richness = leaf_richness(vars, &formula);
            for (selector, gated, inverse_depth, probe_limit) in [
                ("shake", false, 0, 0),
                ("gated-shake", true, 0, 0),
                ("gated-depth-two", true, 2, 4),
            ] {
                let gate_applied = !gated || richness > 0;
                let shaken = if gate_applied {
                    shake_formula(vars, &formula, inverse_depth, probe_limit)
                } else {
                    ShakenFormula {
                        vars,
                        clauses: formula.clone(),
                        core_to_original: (0..vars).collect(),
                        fixed: vec![None; vars],
                        removed: 0,
                        probes: 0,
                        inverse_forced: 0,
                        contradiction: false,
                    }
                };
                let core_treewidth = if shaken.contradiction {
                    0
                } else {
                    exact_treewidth(shaken.vars, &shaken.clauses)
                };
                let (core_nodes, core_assignment, core_rank) = if shaken.contradiction {
                    (0usize, None, Vec::new())
                } else {
                    let order = min_fill_order(shaken.vars, &shaken.clauses);
                    let mut rank = vec![0usize; shaken.vars];
                    for (level, &variable) in order.iter().enumerate() {
                        rank[variable] = level;
                    }
                    let mapped: Vec<_> = shaken
                        .clauses
                        .iter()
                        .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                        .collect();
                    let result = solve_tuple_natural(shaken.vars, &mapped);
                    (result.nodes, result.assignment, rank)
                };
                let core_sat = core_assignment.is_some();
                let reconstruction_valid = if let Some(core_assignment) = core_assignment.as_ref() {
                    let mut reconstructed = vec![false; vars];
                    for (variable, value) in shaken.fixed.iter().enumerate() {
                        if let Some(value) = value {
                            reconstructed[variable] = *value;
                        }
                    }
                    for (core_variable, &original_variable) in
                        shaken.core_to_original.iter().enumerate()
                    {
                        reconstructed[original_variable] =
                            core_assignment[core_rank[core_variable]];
                    }
                    satisfies(&formula, &reconstructed)
                } else {
                    !original_sat
                };
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{}",
                    family,
                    vars,
                    formula.len(),
                    formula_seed,
                    selector,
                    richness,
                    gate_applied,
                    inverse_depth,
                    shaken.removed,
                    shaken.vars,
                    shaken.clauses.len(),
                    shaken.probes,
                    shaken.inverse_forced,
                    original_treewidth,
                    core_treewidth,
                    core_treewidth as isize - original_treewidth as isize,
                    original_result.nodes,
                    core_nodes,
                    core_nodes as f64 / original_result.nodes.max(1) as f64,
                    core_sat,
                    core_sat == original_sat,
                    reconstruction_valid
                );
            }
        }
        return;
    }
    if engine == "seed-branch-width" {
        assert!(
            vars <= 16,
            "seed-branch-width supports at most 16 variables"
        );
        let max_interior = optional_budget.unwrap_or(8).min(12);
        for seed in 1..=trials {
            let formula_seed = 130_000 + seed as u64;
            let formula = generate_formula(family, vars, ratio, formula_seed);
            let original_treewidth = exact_treewidth(vars, &formula);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let original_sat = original_result.assignment.is_some();
            let seeded = seed_detachable_branch(vars, &formula, max_interior);
            let core_treewidth = exact_treewidth(seeded.vars, &seeded.clauses);
            let core_order = min_fill_order(seeded.vars, &seeded.clauses);
            let mut core_rank = vec![0usize; seeded.vars];
            for (level, &variable) in core_order.iter().enumerate() {
                core_rank[variable] = level;
            }
            let core_mapped: Vec<_> = seeded
                .clauses
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (core_rank[v], s)).collect()))
                .collect();
            let core_result = solve_tuple_natural(seeded.vars, &core_mapped);
            let core_sat = core_result.assignment.is_some();
            let reconstruction_valid = if let Some(core_assignment) = &core_result.assignment {
                let mut reconstructed = vec![false; vars];
                for (core_variable, &original_variable) in
                    seeded.core_to_original.iter().enumerate()
                {
                    reconstructed[original_variable] = core_assignment[core_rank[core_variable]];
                }
                let mut boundary_bits = 0usize;
                for (index, &variable) in seeded.boundary.iter().enumerate() {
                    if reconstructed[variable] {
                        boundary_bits |= 1usize << index;
                    }
                }
                if let Some(Some(values)) = seeded.witnesses.get(boundary_bits) {
                    for (index, &variable) in seeded.interior.iter().enumerate() {
                        reconstructed[variable] = values[index];
                    }
                    satisfies(&formula, &reconstructed)
                } else {
                    seeded.interior.is_empty() && satisfies(&formula, &reconstructed)
                }
            } else {
                !original_sat
            };
            let recipe_entries = seeded.witnesses.iter().flatten().count();
            let recipe_bits = recipe_entries * seeded.interior.len();
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{}",
                family,
                vars,
                formula.len(),
                formula_seed,
                seeded.boundary.len(),
                seeded.interior.len(),
                seeded.local_clauses,
                seeded.summary_clauses,
                recipe_entries,
                recipe_bits,
                seeded.compilation_trials,
                original_treewidth,
                core_treewidth,
                core_treewidth as isize - original_treewidth as isize,
                original_result.nodes,
                core_result.nodes,
                core_result.nodes as f64 / original_result.nodes.max(1) as f64,
                core_sat,
                core_sat == original_sat,
                reconstruction_valid
            );
        }
        return;
    }
    if engine == "seed-bdd-width" {
        assert!(vars <= 16, "seed-bdd-width supports at most 16 variables");
        let max_interior = optional_budget.unwrap_or(8).min(14);
        for seed in 1..=trials {
            let formula_seed = 140_000 + seed as u64;
            let formula = generate_formula(family, vars, ratio, formula_seed);
            let original_treewidth = exact_treewidth(vars, &formula);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_start = Instant::now();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let original_solve_us = original_start.elapsed().as_micros();
            let original_sat = original_result.assignment.is_some();
            let compile_start = Instant::now();
            let seeded = seed_detachable_branch_bdd(vars, &formula, max_interior, order_name);
            let compile_us = compile_start.elapsed().as_micros();
            let core_treewidth = exact_treewidth(seeded.vars, &seeded.clauses);
            let core_order = min_fill_order(seeded.vars, &seeded.clauses);
            let mut core_rank = vec![0usize; seeded.vars];
            for (level, &variable) in core_order.iter().enumerate() {
                core_rank[variable] = level;
            }
            let core_mapped: Vec<_> = seeded
                .clauses
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (core_rank[v], s)).collect()))
                .collect();
            let core_start = Instant::now();
            let core_result = solve_tuple_natural(seeded.vars, &core_mapped);
            let core_solve_us = core_start.elapsed().as_micros();
            let core_sat = core_result.assignment.is_some();
            let reconstruction_valid = if let Some(core_assignment) = &core_result.assignment {
                let mut reconstructed = vec![false; vars];
                for (core_variable, &original_variable) in
                    seeded.core_to_original.iter().enumerate()
                {
                    reconstructed[original_variable] = core_assignment[core_rank[core_variable]];
                }
                if let Some(values) = regrow_bdd_seed(&seeded, &reconstructed) {
                    for (index, &variable) in seeded.interior.iter().enumerate() {
                        reconstructed[variable] = values[index];
                    }
                    satisfies(&formula, &reconstructed)
                } else {
                    false
                }
            } else {
                !original_sat
            };
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{}",
                family,
                vars,
                formula.len(),
                formula_seed,
                order_name,
                seeded.boundary.len(),
                seeded.interior.len(),
                seeded.local_clauses,
                seeded.summary_clauses,
                seeded.live_nodes,
                seeded.allocated_nodes,
                compile_us,
                original_solve_us,
                core_solve_us,
                (compile_us + core_solve_us) as f64 / original_solve_us.max(1) as f64,
                original_treewidth,
                core_treewidth,
                core_treewidth as isize - original_treewidth as isize,
                original_result.nodes,
                core_result.nodes,
                core_result.nodes as f64 / original_result.nodes.max(1) as f64,
                (core_result.nodes + seeded.live_nodes) as f64
                    / original_result.nodes.max(1) as f64,
                (core_result.nodes + seeded.allocated_nodes) as f64
                    / original_result.nodes.max(1) as f64,
                core_sat,
                core_sat == original_sat,
                reconstruction_valid
            );
        }
        return;
    }
    if engine == "seed-bdd-reuse" {
        assert!(vars <= 16, "seed-bdd-reuse supports at most 16 variables");
        let max_interior = optional_budget.unwrap_or(8).min(14);
        let updates = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize)
            .max(1);
        let cache_cap = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(usize::MAX);
        let cache_policy = args.get(10).map(String::as_str).unwrap_or("roots");
        for seed in 1..=trials {
            let formula_seed = 150_000 + seed as u64;
            let base = generate_formula(family, vars, ratio, formula_seed);
            let mut shared_manager = BddManager::default();
            let mut cache_roots = Vec::new();
            let mut evictions = 0usize;
            let mut compactions = 0usize;
            let mut peak_shared_nodes = 0usize;
            let mut hot_roots = 0usize;
            for update in 0..updates {
                let mut formula = base.clone();
                if update > 0 && !formula.is_empty() {
                    let clause_index = (update - 1) % formula.len();
                    if !formula[clause_index].0.is_empty() {
                        let literal_index = (update - 1) % formula[clause_index].0.len();
                        formula[clause_index].0[literal_index].1 =
                            !formula[clause_index].0[literal_index].1;
                    }
                }
                let cold_start = Instant::now();
                let cold = seed_detachable_branch_bdd(vars, &formula, max_interior, "natural");
                let cold_us = cold_start.elapsed().as_micros();
                let warm_start = Instant::now();
                let warm = seed_detachable_branch_bdd_in(
                    vars,
                    &formula,
                    max_interior,
                    "natural",
                    &mut shared_manager,
                );
                let warm_us = warm_start.elapsed().as_micros();
                cache_roots.push(warm.cache_root);
                peak_shared_nodes = peak_shared_nodes.max(shared_manager.nodes.len());
                if shared_manager.nodes.len() > cache_cap {
                    let seed_root_count = cache_roots.len();
                    let mut retained_roots = cache_roots.clone();
                    if cache_policy == "hits" {
                        let mut hottest: Vec<_> = shared_manager
                            .node_hits
                            .iter()
                            .copied()
                            .enumerate()
                            .filter(|(index, hits)| {
                                *hits > 0 && !cache_roots.contains(&(index + 2))
                            })
                            .collect();
                        hottest.sort_by_key(|&(index, hits)| (std::cmp::Reverse(hits), index));
                        retained_roots
                            .extend(hottest.into_iter().take(128).map(|(index, _)| index + 2));
                    }
                    loop {
                        let (compacted, mapped_roots) =
                            compact_bdd_roots(&shared_manager, &retained_roots);
                        compactions += 1;
                        shared_manager = compacted;
                        retained_roots = mapped_roots;
                        if shared_manager.nodes.len() <= cache_cap {
                            break;
                        }
                        if retained_roots.len() > seed_root_count {
                            let hot_count = retained_roots.len() - seed_root_count;
                            retained_roots.truncate(retained_roots.len() - hot_count.div_ceil(2));
                        } else if seed_root_count > 1 {
                            retained_roots.remove(0);
                            evictions += 1;
                        } else {
                            break;
                        }
                    }
                    let retained_seed_count = seed_root_count.min(retained_roots.len());
                    cache_roots = retained_roots[..retained_seed_count].to_vec();
                    hot_roots = retained_roots.len() - retained_seed_count;
                }
                let original_treewidth = exact_treewidth(vars, &formula);
                let core_treewidth = exact_treewidth(warm.vars, &warm.clauses);
                let original_order = min_fill_order(vars, &formula);
                let mut original_rank = vec![0usize; vars];
                for (level, &variable) in original_order.iter().enumerate() {
                    original_rank[variable] = level;
                }
                let original_mapped: Vec<_> = formula
                    .iter()
                    .map(|clause| {
                        Clause(
                            clause
                                .0
                                .iter()
                                .map(|&(v, s)| (original_rank[v], s))
                                .collect(),
                        )
                    })
                    .collect();
                let original_result = solve_tuple_natural(vars, &original_mapped);
                let core_order = min_fill_order(warm.vars, &warm.clauses);
                let mut core_rank = vec![0usize; warm.vars];
                for (level, &variable) in core_order.iter().enumerate() {
                    core_rank[variable] = level;
                }
                let core_mapped: Vec<_> = warm
                    .clauses
                    .iter()
                    .map(|clause| {
                        Clause(clause.0.iter().map(|&(v, s)| (core_rank[v], s)).collect())
                    })
                    .collect();
                let core_result = solve_tuple_natural(warm.vars, &core_mapped);
                let equivalent =
                    core_result.assignment.is_some() == original_result.assignment.is_some();
                let valid = if let Some(core_assignment) = &core_result.assignment {
                    let mut reconstructed = vec![false; vars];
                    for (core_variable, &original_variable) in
                        warm.core_to_original.iter().enumerate()
                    {
                        reconstructed[original_variable] =
                            core_assignment[core_rank[core_variable]];
                    }
                    regrow_bdd_seed(&warm, &reconstructed).is_some_and(|values| {
                        for (index, &variable) in warm.interior.iter().enumerate() {
                            reconstructed[variable] = values[index];
                        }
                        satisfies(&formula, &reconstructed)
                    })
                } else {
                    original_result.assignment.is_none()
                };
                println!(
                    "{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{}",
                    family,
                    vars,
                    formula.len(),
                    formula_seed,
                    update,
                    warm.interior.len(),
                    cold.allocated_nodes,
                    warm.allocated_nodes,
                    warm.allocated_nodes as f64 / cold.allocated_nodes.max(1) as f64,
                    cold_us,
                    warm_us,
                    warm_us as f64 / cold_us.max(1) as f64,
                    shared_manager.nodes.len(),
                    cache_cap,
                    cache_policy,
                    peak_shared_nodes,
                    compactions,
                    evictions,
                    hot_roots,
                    warm.live_nodes,
                    original_treewidth,
                    core_treewidth,
                    core_treewidth as isize - original_treewidth as isize,
                    original_result.nodes,
                    core_result.nodes,
                    core_result.nodes as f64 / original_result.nodes.max(1) as f64,
                    equivalent,
                    valid
                );
            }
        }
        return;
    }
    if engine == "multi-seed-width" {
        assert!(vars <= 16, "multi-seed-width supports at most 16 variables");
        let branch_cap = optional_budget.unwrap_or(4).min(10);
        let max_seeds = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        for seed in 1..=trials {
            let formula_seed = 160_000 + seed as u64;
            let formula = generate_formula(family, vars, ratio, formula_seed);
            let original_treewidth = exact_treewidth(vars, &formula);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_start = Instant::now();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let original_solve_us = original_start.elapsed().as_micros();
            for (selector, cap, limit) in [
                ("original", 0usize, 0usize),
                ("single-giant", vars.saturating_sub(2), 1usize),
                ("multi-small", branch_cap, max_seeds),
            ] {
                let compile_start = Instant::now();
                let mut current_vars = vars;
                let mut current_clauses = formula.clone();
                let mut seeds = Vec::new();
                for _ in 0..limit {
                    let compiled =
                        seed_detachable_branch_bdd(current_vars, &current_clauses, cap, "natural");
                    if compiled.interior.is_empty() {
                        break;
                    }
                    current_vars = compiled.vars;
                    current_clauses = compiled.clauses.clone();
                    seeds.push(compiled);
                }
                let compile_us = compile_start.elapsed().as_micros();
                let final_treewidth = exact_treewidth(current_vars, &current_clauses);
                let final_order = min_fill_order(current_vars, &current_clauses);
                let mut final_rank = vec![0usize; current_vars];
                for (level, &variable) in final_order.iter().enumerate() {
                    final_rank[variable] = level;
                }
                let final_mapped: Vec<_> = current_clauses
                    .iter()
                    .map(|clause| {
                        Clause(clause.0.iter().map(|&(v, s)| (final_rank[v], s)).collect())
                    })
                    .collect();
                let final_start = Instant::now();
                let final_result = solve_tuple_natural(current_vars, &final_mapped);
                let final_solve_us = final_start.elapsed().as_micros();
                let reconstructed = final_result.assignment.as_ref().and_then(|assignment| {
                    let core_assignment: Vec<_> = (0..current_vars)
                        .map(|variable| assignment[final_rank[variable]])
                        .collect();
                    regrow_seed_chain(&seeds, &core_assignment)
                });
                let equivalent =
                    final_result.assignment.is_some() == original_result.assignment.is_some();
                let valid = reconstructed
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&formula, assignment))
                    && (reconstructed.is_some() == original_result.assignment.is_some());
                let removed: usize = seeds.iter().map(|seed| seed.interior.len()).sum();
                let boundary_sum: usize = seeds.iter().map(|seed| seed.boundary.len()).sum();
                let live_nodes: usize = seeds.iter().map(|seed| seed.live_nodes).sum();
                let allocated_nodes: usize = seeds.iter().map(|seed| seed.allocated_nodes).sum();
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{:.6},{},{}",
                    family,
                    vars,
                    formula.len(),
                    formula_seed,
                    selector,
                    cap,
                    limit,
                    seeds.len(),
                    removed,
                    boundary_sum,
                    live_nodes,
                    allocated_nodes,
                    compile_us,
                    current_vars,
                    current_clauses.len(),
                    original_treewidth,
                    final_treewidth,
                    final_treewidth as isize - original_treewidth as isize,
                    original_result.nodes,
                    final_result.nodes,
                    final_result.nodes as f64 / original_result.nodes.max(1) as f64,
                    (final_result.nodes + live_nodes) as f64 / original_result.nodes.max(1) as f64,
                    original_solve_us,
                    final_solve_us,
                    (compile_us + final_solve_us) as f64 / original_solve_us.max(1) as f64,
                    equivalent,
                    valid
                );
            }
        }
        return;
    }
    if engine == "incremental-payback" {
        let branch_cap = optional_budget.unwrap_or(8).min(12);
        let queries = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(32usize)
            .max(2);
        let max_seeds = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        for seed in 1..=trials {
            let formula_seed = 170_000 + seed as u64;
            let base = generate_formula(family, vars, ratio, formula_seed);
            let mut formulas = Vec::with_capacity(queries);
            formulas.push(base.clone());
            for query in 1..queries {
                let mut formula = base.clone();
                if !formula.is_empty() {
                    let clause_index = (query - 1) % formula.len();
                    if !formula[clause_index].0.is_empty() {
                        let literal_index = (query - 1) % formula[clause_index].0.len();
                        formula[clause_index].0[literal_index].1 =
                            !formula[clause_index].0[literal_index].1;
                    }
                }
                formulas.push(formula);
            }

            let setup_start = Instant::now();
            let mut incremental = Solver::new();
            let selector_count = queries - 1;
            for (clause_index, clause) in base.iter().enumerate() {
                let mut literals: Vec<_> = clause
                    .0
                    .iter()
                    .map(|&(variable, positive)| Lit::from_var(Var::from_index(variable), positive))
                    .collect();
                for query in 1..queries {
                    if (query - 1) % base.len() == clause_index {
                        literals.push(Lit::from_var(Var::from_index(vars + query - 1), true));
                    }
                }
                incremental.add_clause(&literals);
            }
            for query in 1..queries {
                let clause_index = (query - 1) % base.len();
                let mut literals: Vec<_> = formulas[query][clause_index]
                    .0
                    .iter()
                    .map(|&(variable, positive)| Lit::from_var(Var::from_index(variable), positive))
                    .collect();
                literals.push(Lit::from_var(Var::from_index(vars + query - 1), false));
                incremental.add_clause(&literals);
            }
            let setup_us = setup_start.elapsed().as_micros();
            let mut shared_seed_manager = BddManager::default();
            let mut cumulative_cold = 0u128;
            let mut cumulative_incremental = setup_us;
            let mut cumulative_seed = 0u128;
            for (query, formula) in formulas.iter().enumerate() {
                let cold_start = Instant::now();
                let cold_assignment = solve_with_varisat(vars, formula);
                let cold_us = cold_start.elapsed().as_micros();
                cumulative_cold += cold_us;

                let assumptions: Vec<_> = (0..selector_count)
                    .map(|selector| {
                        Lit::from_var(
                            Var::from_index(vars + selector),
                            query > 0 && selector == query - 1,
                        )
                    })
                    .collect();
                incremental.assume(&assumptions);
                let incremental_start = Instant::now();
                let incremental_sat = incremental.solve().expect("incremental Varisat solve");
                let incremental_us = incremental_start.elapsed().as_micros();
                cumulative_incremental += incremental_us;
                let mut incremental_assignment = vec![false; vars];
                if incremental_sat {
                    for literal in incremental.model().expect("incremental model") {
                        if literal.var().index() < vars {
                            incremental_assignment[literal.var().index()] = literal.is_positive();
                        }
                    }
                }

                let seed_compile_start = Instant::now();
                let mut current_vars = vars;
                let mut current_clauses = formula.clone();
                let mut seeds = Vec::new();
                for _ in 0..max_seeds {
                    let compiled = seed_detachable_branch_bdd_in(
                        current_vars,
                        &current_clauses,
                        branch_cap,
                        "natural",
                        &mut shared_seed_manager,
                    );
                    if compiled.interior.is_empty() {
                        break;
                    }
                    current_vars = compiled.vars;
                    current_clauses = compiled.clauses.clone();
                    seeds.push(compiled);
                }
                let seed_compile_us = seed_compile_start.elapsed().as_micros();
                let seed_solve_start = Instant::now();
                let core_assignment = solve_with_varisat(current_vars, &current_clauses);
                let seed_solve_us = seed_solve_start.elapsed().as_micros();
                cumulative_seed += seed_compile_us + seed_solve_us;
                let seed_assignment = core_assignment
                    .as_ref()
                    .and_then(|assignment| regrow_seed_chain(&seeds, assignment));
                let cold_sat = cold_assignment.is_some();
                let incremental_valid =
                    !incremental_sat || satisfies(formula, &incremental_assignment);
                let seed_valid = seed_assignment
                    .as_ref()
                    .is_none_or(|assignment| satisfies(formula, assignment));
                let removed: usize = seeds.iter().map(|seed| seed.interior.len()).sum();
                println!(
                    "{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{},{},{},{},{},{},{}",
                    family,
                    vars,
                    formula.len(),
                    formula_seed,
                    query,
                    cold_us,
                    setup_us,
                    incremental_us,
                    cumulative_incremental as f64 / cumulative_cold.max(1) as f64,
                    seed_compile_us,
                    seed_solve_us,
                    cumulative_seed as f64 / cumulative_cold.max(1) as f64,
                    seeds.len(),
                    removed,
                    shared_seed_manager.nodes.len(),
                    cold_sat,
                    incremental_sat == cold_sat,
                    seed_assignment.is_some() == cold_sat,
                    incremental_valid,
                    seed_valid
                );
            }
        }
        return;
    }
    if engine == "safe-compiler-control" {
        let branch_cap = optional_budget.unwrap_or(64).min(64);
        let node_limit = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(100_000usize);
        let time_limit_ms = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(100u64);
        let sizes: Vec<_> = [500usize, 1000]
            .into_iter()
            .filter(|&size| size <= vars)
            .collect();
        assert!(!sizes.is_empty(), "safe compiler requires vars >= 500");
        for &scale_vars in &sizes {
            for trial in 0..trials {
                let formula_seed = 1_700_000 + scale_vars as u64 * 100 + trial as u64;
                let original = generate_formula(family, scale_vars, ratio, formula_seed);
                for layout in ["original", "renamed"] {
                    let mut permutation: Vec<_> = (0..scale_vars).collect();
                    if layout == "renamed" {
                        Rng(formula_seed ^ 0x517c_c1b7).shuffle(&mut permutation);
                    }
                    let formula: Vec<_> = original
                        .iter()
                        .map(|clause| {
                            Clause(
                                clause
                                    .0
                                    .iter()
                                    .map(|&(variable, value)| (permutation[variable], value))
                                    .collect(),
                            )
                        })
                        .collect();
                    let discovery_start = Instant::now();
                    let candidates =
                        fast_detachable_branch_candidates(scale_vars, &formula, branch_cap);
                    let discovery_us = discovery_start.elapsed().as_micros();
                    let compile_start = Instant::now();
                    let mut seeds = Vec::new();
                    let mut node_rejected = 0usize;
                    let mut time_rejected = 0usize;
                    let mut attempt_nodes = 0usize;
                    for (interior, boundary) in candidates.iter().cloned() {
                        let attempt = try_seed_bdd_candidate(
                            scale_vars,
                            &formula,
                            interior,
                            boundary,
                            "min-fill",
                            node_limit,
                            std::time::Duration::from_millis(time_limit_ms),
                        );
                        attempt_nodes += attempt.nodes;
                        node_rejected += usize::from(attempt.node_exceeded);
                        time_rejected += usize::from(attempt.time_exceeded);
                        if let Some(seed) = attempt.seed {
                            seeds.push(seed);
                        }
                    }
                    let compile_us = compile_start.elapsed().as_micros();
                    let all_interior: BTreeSet<_> = seeds
                        .iter()
                        .flat_map(|seed| seed.interior.iter().copied())
                        .collect();
                    let mut core_clauses: Vec<_> = formula
                        .iter()
                        .filter(|clause| {
                            !clause
                                .0
                                .iter()
                                .any(|(variable, _)| all_interior.contains(variable))
                        })
                        .cloned()
                        .collect();
                    for seed in &seeds {
                        core_clauses.extend(seed.summary.iter().cloned());
                    }
                    let core_to_original: Vec<_> = (0..scale_vars)
                        .filter(|variable| !all_interior.contains(variable))
                        .collect();
                    let mut original_to_core = vec![usize::MAX; scale_vars];
                    for (core, &original) in core_to_original.iter().enumerate() {
                        original_to_core[original] = core;
                    }
                    for clause in &mut core_clauses {
                        for (variable, _) in &mut clause.0 {
                            *variable = original_to_core[*variable];
                        }
                    }
                    let original_assignment = solve_with_varisat(scale_vars, &formula);
                    let core_assignment = solve_with_varisat(core_to_original.len(), &core_clauses);
                    let mut reconstructed = core_assignment.as_ref().map(|core| {
                        let mut assignment = vec![false; scale_vars];
                        for (index, &original) in core_to_original.iter().enumerate() {
                            assignment[original] = core[index];
                        }
                        assignment
                    });
                    if let Some(assignment) = &mut reconstructed {
                        for seed in &seeds {
                            let Some(values) = regrow_bdd_seed(seed, assignment) else {
                                reconstructed = None;
                                break;
                            };
                            for (index, &variable) in seed.interior.iter().enumerate() {
                                assignment[variable] = values[index];
                            }
                        }
                    }
                    let removed = all_interior.len();
                    println!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{}",
                        family,
                        layout,
                        scale_vars,
                        formula.len(),
                        formula_seed,
                        branch_cap,
                        node_limit,
                        time_limit_ms,
                        candidates.len(),
                        seeds.len(),
                        node_rejected,
                        time_rejected,
                        removed,
                        removed as f64 / scale_vars as f64,
                        discovery_us,
                        compile_us,
                        attempt_nodes,
                        seeds.iter().map(|seed| seed.live_nodes).sum::<usize>(),
                        core_to_original.len(),
                        original_assignment.is_some() == core_assignment.is_some(),
                        original_assignment.is_none()
                            || reconstructed
                                .as_ref()
                                .is_some_and(|assignment| satisfies(&formula, assignment)),
                        removed * 10 >= scale_vars * 3
                    );
                }
            }
        }
        return;
    }
    if engine == "renaming-control" {
        let branch_cap = optional_budget.unwrap_or(64).min(64);
        let renamings = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(3usize);
        let sizes: Vec<_> = [500usize, 1000]
            .into_iter()
            .filter(|&size| size <= vars)
            .collect();
        assert!(!sizes.is_empty(), "renaming control requires vars >= 500");
        for &scale_vars in &sizes {
            for trial in 0..trials {
                let formula_seed = 1_500_000 + scale_vars as u64 * 100 + trial as u64;
                let original = generate_formula(family, scale_vars, ratio, formula_seed);
                for renaming in 0..=renamings {
                    let mut permutation: Vec<_> = (0..scale_vars).collect();
                    if renaming > 0 {
                        Rng(formula_seed ^ (renaming as u64).wrapping_mul(0x9e37_79b9))
                            .shuffle(&mut permutation);
                    }
                    let formula: Vec<_> = original
                        .iter()
                        .map(|clause| {
                            Clause(
                                clause
                                    .0
                                    .iter()
                                    .map(|&(variable, value)| (permutation[variable], value))
                                    .collect(),
                            )
                        })
                        .collect();
                    let discovery_start = Instant::now();
                    let candidates =
                        fast_detachable_branch_candidates(scale_vars, &formula, branch_cap);
                    let discovery_us = discovery_start.elapsed().as_micros();
                    let compile_start = Instant::now();
                    let mut seeds = Vec::new();
                    for (mut interior, boundary) in candidates.iter().cloned() {
                        let mut manager = BddManager::default();
                        seeds.push(seed_bdd_candidate_in(
                            scale_vars,
                            &formula,
                            &mut interior,
                            boundary,
                            "min-fill",
                            &mut manager,
                        ));
                    }
                    let all_interior: BTreeSet<_> = seeds
                        .iter()
                        .flat_map(|seed| seed.interior.iter().copied())
                        .collect();
                    let mut core_clauses: Vec<_> = formula
                        .iter()
                        .filter(|clause| {
                            !clause
                                .0
                                .iter()
                                .any(|(variable, _)| all_interior.contains(variable))
                        })
                        .cloned()
                        .collect();
                    for seed in &seeds {
                        core_clauses.extend(seed.summary.iter().cloned());
                    }
                    let core_to_original: Vec<_> = (0..scale_vars)
                        .filter(|variable| !all_interior.contains(variable))
                        .collect();
                    let mut original_to_core = vec![usize::MAX; scale_vars];
                    for (core, &original) in core_to_original.iter().enumerate() {
                        original_to_core[original] = core;
                    }
                    for clause in &mut core_clauses {
                        for (variable, _) in &mut clause.0 {
                            *variable = original_to_core[*variable];
                        }
                    }
                    let compile_us = compile_start.elapsed().as_micros();
                    let original_assignment = solve_with_varisat(scale_vars, &formula);
                    let core_assignment = solve_with_varisat(core_to_original.len(), &core_clauses);
                    let mut reconstructed = core_assignment.as_ref().map(|core| {
                        let mut assignment = vec![false; scale_vars];
                        for (index, &original) in core_to_original.iter().enumerate() {
                            assignment[original] = core[index];
                        }
                        assignment
                    });
                    if let Some(assignment) = &mut reconstructed {
                        for seed in &seeds {
                            let Some(values) = regrow_bdd_seed(seed, assignment) else {
                                reconstructed = None;
                                break;
                            };
                            for (index, &variable) in seed.interior.iter().enumerate() {
                                assignment[variable] = values[index];
                            }
                        }
                    }
                    let removed = all_interior.len();
                    println!(
                        "{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{}",
                        family,
                        scale_vars,
                        formula.len(),
                        formula_seed,
                        renaming,
                        branch_cap,
                        candidates.len(),
                        removed,
                        removed as f64 / scale_vars as f64,
                        discovery_us,
                        compile_us,
                        seeds.iter().map(|seed| seed.live_nodes).sum::<usize>(),
                        seeds.iter().map(|seed| seed.allocated_nodes).sum::<usize>(),
                        core_to_original.len(),
                        original_assignment.is_some() == core_assignment.is_some(),
                        reconstructed
                            .as_ref()
                            .is_some_and(|assignment| satisfies(&formula, assignment)),
                        removed * 10 >= scale_vars * 3
                    );
                }
            }
        }
        return;
    }
    if engine == "batch-offline-service" {
        let branch_cap = optional_budget.unwrap_or(64).min(64);
        let horizon = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(1_000_000usize)
            .max(2);
        let sizes: Vec<_> = [500usize, 1000]
            .into_iter()
            .filter(|&size| size <= vars)
            .collect();
        assert!(!sizes.is_empty(), "batch service requires vars >= 500");
        for &scale_vars in &sizes {
            for trial in 0..trials {
                let formula_seed = 1_300_000 + scale_vars as u64 * 100 + trial as u64;
                let mut formula = generate_formula(family, scale_vars, ratio, formula_seed);
                if order_name.contains("renamed") {
                    let mut permutation: Vec<_> = (0..scale_vars).collect();
                    Rng(formula_seed ^ 0x9e37_79b9).shuffle(&mut permutation);
                    for clause in &mut formula {
                        for (variable, _) in &mut clause.0 {
                            *variable = permutation[*variable];
                        }
                    }
                }
                let discovery_start = Instant::now();
                let candidates =
                    fast_detachable_branch_candidates(scale_vars, &formula, branch_cap);
                let discovery_ns = discovery_start.elapsed().as_nanos();
                let compile_start = Instant::now();
                let mut seeds = Vec::new();
                for (mut interior, boundary) in candidates {
                    let strategy = if order_name.contains("renamed") {
                        "min-fill"
                    } else {
                        "natural"
                    };
                    if order_name.starts_with("safe") {
                        let attempt = try_seed_bdd_candidate(
                            scale_vars,
                            &formula,
                            interior,
                            boundary,
                            strategy,
                            100_000,
                            std::time::Duration::from_millis(100),
                        );
                        if let Some(seed) = attempt.seed {
                            seeds.push(seed);
                        }
                    } else {
                        let mut manager = BddManager::default();
                        seeds.push(seed_bdd_candidate_in(
                            scale_vars,
                            &formula,
                            &mut interior,
                            boundary,
                            strategy,
                            &mut manager,
                        ));
                    }
                }
                let all_interior: BTreeSet<_> = seeds
                    .iter()
                    .flat_map(|seed| seed.interior.iter().copied())
                    .collect();
                let mut core_clauses: Vec<_> = formula
                    .iter()
                    .filter(|clause| {
                        !clause
                            .0
                            .iter()
                            .any(|(variable, _)| all_interior.contains(variable))
                    })
                    .cloned()
                    .collect();
                for seed in &seeds {
                    core_clauses.extend(seed.summary.iter().cloned());
                }
                let core_to_original: Vec<_> = (0..scale_vars)
                    .filter(|variable| !all_interior.contains(variable))
                    .collect();
                let mut original_to_core = vec![usize::MAX; scale_vars];
                for (core, &original) in core_to_original.iter().enumerate() {
                    original_to_core[original] = core;
                }
                for clause in &mut core_clauses {
                    for (variable, _) in &mut clause.0 {
                        *variable = original_to_core[*variable];
                    }
                }
                let compile_ns = compile_start.elapsed().as_nanos();
                let incremental_setup_start = Instant::now();
                let mut incremental = Solver::new();
                add_to_varisat(&mut incremental, &formula);
                let incremental_setup_ns = incremental_setup_start.elapsed().as_nanos();
                let seeded_setup_start = Instant::now();
                let mut seeded = Solver::new();
                add_to_varisat(&mut seeded, &core_clauses);
                let seeded_solver_setup_ns = seeded_setup_start.elapsed().as_nanos();
                let mut incremental_query_ns = 0u128;
                let mut seeded_query_ns = 0u128;
                let mut reconstruction_ns = 0u128;
                let mut reconstruction_samples = 0usize;
                let mut valid = !core_to_original.is_empty();
                for query in 0..128 + horizon {
                    let core_variable = query % core_to_original.len();
                    let original_variable = core_to_original[core_variable];
                    let value = (query / core_to_original.len() + query) % 2 == 0;
                    incremental.assume(&[Lit::from_var(Var::from_index(original_variable), value)]);
                    let start = Instant::now();
                    let incremental_sat = incremental.solve().expect("batch incremental solve");
                    if query >= 128 {
                        incremental_query_ns += start.elapsed().as_nanos();
                    }
                    seeded.assume(&[Lit::from_var(Var::from_index(core_variable), value)]);
                    let start = Instant::now();
                    let seeded_sat = seeded.solve().expect("batch seeded solve");
                    if query >= 128 {
                        seeded_query_ns += start.elapsed().as_nanos();
                    }
                    valid &= incremental_sat == seeded_sat;
                    if seeded_sat && query >= 128 && (query < 132 || query + 1 == 128 + horizon) {
                        let start = Instant::now();
                        let mut assignment = vec![false; scale_vars];
                        for literal in seeded.model().expect("batch seeded model") {
                            if literal.var().index() < core_to_original.len() {
                                assignment[core_to_original[literal.var().index()]] =
                                    literal.is_positive();
                            }
                        }
                        for seed in &seeds {
                            let Some(values) = regrow_bdd_seed(seed, &assignment) else {
                                valid = false;
                                break;
                            };
                            for (index, &variable) in seed.interior.iter().enumerate() {
                                assignment[variable] = values[index];
                            }
                        }
                        valid &= assignment[original_variable] == value
                            && satisfies(&formula, &assignment);
                        reconstruction_ns += start.elapsed().as_nanos();
                        reconstruction_samples += 1;
                    }
                }
                let incremental_total = incremental_setup_ns + incremental_query_ns;
                let seeded_total =
                    discovery_ns + compile_ns + seeded_solver_setup_ns + seeded_query_ns;
                let incremental_per = incremental_query_ns as f64 / horizon as f64;
                let seeded_per = seeded_query_ns as f64 / horizon as f64;
                let crossover = if seeded_per < incremental_per {
                    (discovery_ns + compile_ns + seeded_solver_setup_ns)
                        .saturating_sub(incremental_setup_ns) as f64
                        / (incremental_per - seeded_per)
                } else {
                    f64::INFINITY
                };
                println!(
                    "{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{:.6},{:.6},{:.1},{:.1},{:.1},{},{},{},{}",
                    family,
                    scale_vars,
                    formula.len(),
                    formula_seed,
                    horizon,
                    branch_cap,
                    seeds.len(),
                    all_interior.len(),
                    all_interior.len() as f64 / scale_vars as f64,
                    core_to_original.len(),
                    discovery_ns,
                    compile_ns,
                    incremental_setup_ns,
                    seeded_solver_setup_ns,
                    incremental_query_ns,
                    seeded_query_ns,
                    seeded_query_ns as f64 / incremental_query_ns.max(1) as f64,
                    seeded_total as f64 / incremental_total.max(1) as f64,
                    crossover,
                    horizon as f64 * 1e9 / incremental_query_ns.max(1) as f64,
                    horizon as f64 * 1e9 / seeded_query_ns.max(1) as f64,
                    seeds.iter().map(|seed| seed.live_nodes).sum::<usize>(),
                    reconstruction_ns,
                    reconstruction_samples,
                    valid
                );
            }
        }
        return;
    }
    if engine == "batch-seed-compile" {
        let branch_cap = optional_budget.unwrap_or(48).min(64);
        let sizes: Vec<_> = [500usize, 1000]
            .into_iter()
            .filter(|&size| size <= vars)
            .collect();
        assert!(!sizes.is_empty(), "batch compilation requires vars >= 500");
        for &scale_vars in &sizes {
            for trial in 0..trials {
                let formula_seed = 1_100_000 + scale_vars as u64 * 100 + trial as u64;
                let formula = generate_formula(family, scale_vars, ratio, formula_seed);
                let discovery_start = Instant::now();
                let candidates =
                    fast_detachable_branch_candidates(scale_vars, &formula, branch_cap);
                let discovery_us = discovery_start.elapsed().as_micros();
                let compile_start = Instant::now();
                let mut seeds = Vec::new();
                for (mut interior, boundary) in candidates.iter().cloned() {
                    let mut manager = BddManager::default();
                    seeds.push(seed_bdd_candidate_in(
                        scale_vars,
                        &formula,
                        &mut interior,
                        boundary,
                        "natural",
                        &mut manager,
                    ));
                }
                let all_interior: BTreeSet<_> = seeds
                    .iter()
                    .flat_map(|seed| seed.interior.iter().copied())
                    .collect();
                let mut core_clauses: Vec<_> = formula
                    .iter()
                    .filter(|clause| {
                        !clause
                            .0
                            .iter()
                            .any(|(variable, _)| all_interior.contains(variable))
                    })
                    .cloned()
                    .collect();
                for seed in &seeds {
                    core_clauses.extend(seed.summary.iter().cloned());
                }
                let core_to_original: Vec<_> = (0..scale_vars)
                    .filter(|variable| !all_interior.contains(variable))
                    .collect();
                let mut original_to_core = vec![usize::MAX; scale_vars];
                for (core, &original) in core_to_original.iter().enumerate() {
                    original_to_core[original] = core;
                }
                for clause in &mut core_clauses {
                    for (variable, _) in &mut clause.0 {
                        *variable = original_to_core[*variable];
                    }
                }
                let compile_us = compile_start.elapsed().as_micros();
                let original_assignment = solve_with_varisat(scale_vars, &formula);
                let core_assignment = solve_with_varisat(core_to_original.len(), &core_clauses);
                let mut reconstructed = core_assignment.as_ref().map(|core| {
                    let mut assignment = vec![false; scale_vars];
                    for (index, &original) in core_to_original.iter().enumerate() {
                        assignment[original] = core[index];
                    }
                    assignment
                });
                if let Some(assignment) = &mut reconstructed {
                    for seed in &seeds {
                        let Some(values) = regrow_bdd_seed(seed, assignment) else {
                            reconstructed = None;
                            break;
                        };
                        for (index, &variable) in seed.interior.iter().enumerate() {
                            assignment[variable] = values[index];
                        }
                    }
                }
                let removed = all_interior.len();
                println!(
                    "{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{},{},{},{}",
                    family,
                    scale_vars,
                    formula.len(),
                    formula_seed,
                    branch_cap,
                    seeds.len(),
                    removed,
                    removed as f64 / scale_vars as f64,
                    discovery_us,
                    compile_us,
                    seeds.iter().map(|seed| seed.live_nodes).sum::<usize>(),
                    seeds.iter().map(|seed| seed.allocated_nodes).sum::<usize>(),
                    core_to_original.len(),
                    core_clauses.len(),
                    original_assignment.is_some(),
                    core_assignment.is_some(),
                    original_assignment.is_some() == core_assignment.is_some(),
                    reconstructed
                        .as_ref()
                        .is_some_and(|assignment| satisfies(&formula, assignment)),
                    removed * 10 >= scale_vars * 3
                );
            }
        }
        return;
    }
    if engine == "batch-branch-discovery" {
        let branch_cap = optional_budget.unwrap_or(8).min(64);
        let sizes: Vec<_> = [100usize, 250, 500, 1000]
            .into_iter()
            .filter(|&size| size <= vars)
            .collect();
        assert!(!sizes.is_empty(), "batch discovery requires vars >= 100");
        for &scale_vars in &sizes {
            for trial in 0..trials {
                let formula_seed = 900_000 + scale_vars as u64 * 100 + trial as u64;
                let formula = generate_formula(family, scale_vars, ratio, formula_seed);
                let start = Instant::now();
                let candidates =
                    fast_detachable_branch_candidates(scale_vars, &formula, branch_cap);
                let discovery_us = start.elapsed().as_micros();
                let graph = primal_graph(scale_vars, &formula);
                let mut all_interior = BTreeSet::new();
                let mut integrity_valid = true;
                let mut local_clause_touches = 0usize;
                for (interior, boundary) in &candidates {
                    let interior_set: BTreeSet<_> = interior.iter().copied().collect();
                    let actual_boundary: BTreeSet<_> = interior
                        .iter()
                        .flat_map(|&variable| graph[variable].iter().copied())
                        .filter(|variable| !interior_set.contains(variable))
                        .collect();
                    integrity_valid &= actual_boundary.iter().copied().eq(boundary.iter().copied());
                    integrity_valid &= interior
                        .iter()
                        .all(|variable| all_interior.insert(*variable));
                    local_clause_touches += formula
                        .iter()
                        .filter(|clause| {
                            clause
                                .0
                                .iter()
                                .any(|(variable, _)| interior_set.contains(variable))
                        })
                        .count();
                }
                let removed = all_interior.len();
                println!(
                    "{},{},{},{},{},{},{},{:.6},{},{},{},{},{}",
                    family,
                    scale_vars,
                    formula.len(),
                    formula_seed,
                    branch_cap,
                    candidates.len(),
                    removed,
                    removed as f64 / scale_vars as f64,
                    candidates
                        .iter()
                        .map(|candidate| candidate.1.len())
                        .sum::<usize>(),
                    local_clause_touches,
                    discovery_us,
                    removed * 10 >= scale_vars * 3,
                    integrity_valid
                );
            }
        }
        return;
    }
    if engine == "offline-scaling" {
        let branch_cap = optional_budget.unwrap_or(8).min(12);
        let max_seeds = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        let sizes: Vec<_> = [100usize, 250, 500, 1000]
            .into_iter()
            .filter(|&size| size <= vars)
            .collect();
        assert!(!sizes.is_empty(), "offline-scaling requires vars >= 100");
        let horizons = [10_000usize, 25_000, 50_000, 100_000, 1_000_000];
        for &scale_vars in &sizes {
            for trial in 0..trials {
                let formula_seed = 790_000 + scale_vars as u64 * 100 + trial as u64;
                let formula = generate_formula(family, scale_vars, ratio, formula_seed);
                for &horizon in &horizons {
                    let measured = measure_assumption_service_warm(
                        scale_vars, &formula, branch_cap, max_seeds, 128, horizon,
                    );
                    let incremental_per_query =
                        measured.incremental_query_ns as f64 / horizon as f64;
                    let seeded_per_query = measured.seeded_query_ns as f64 / horizon as f64;
                    let crossover = if seeded_per_query < incremental_per_query {
                        measured
                            .seeded_setup_ns
                            .saturating_sub(measured.incremental_setup_ns)
                            as f64
                            / (incremental_per_query - seeded_per_query)
                    } else {
                        f64::INFINITY
                    };
                    println!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{:.1},{:.1},{:.1},{},{:.1},{}",
                        family,
                        scale_vars,
                        formula.len(),
                        formula_seed,
                        horizon,
                        measured.seeds,
                        measured.removed,
                        scale_vars.saturating_sub(measured.removed),
                        measured.live_nodes,
                        measured.seeded_setup_ns,
                        measured.incremental_setup_ns,
                        measured.incremental_query_ns,
                        measured.seeded_query_ns,
                        measured.seeded_query_ns as f64
                            / measured.incremental_query_ns.max(1) as f64,
                        measured.seeded_ns as f64 / measured.incremental_ns.max(1) as f64,
                        horizon as f64 * 1e9 / measured.incremental_query_ns.max(1) as f64,
                        horizon as f64 * 1e9 / measured.seeded_query_ns.max(1) as f64,
                        measured.reconstruction_ns as f64
                            / measured.reconstruction_samples.max(1) as f64,
                        measured.reconstruction_samples,
                        crossover,
                        measured.valid
                    );
                }
            }
        }
        return;
    }
    if engine == "speculative-compile" {
        let branch_cap = optional_budget.unwrap_or(8).min(12);
        let queries = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(4096usize)
            .max(2);
        let max_seeds = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        let warmup = 128usize;
        let calibration_queries = 128usize;
        let families = ["random", "banded", "stacked-flower"];
        let sizes = [vars.saturating_sub(10).max(10), vars];
        let densities = [2usize, 4usize];
        let mut query_training = Vec::new();
        for (family_index, training_family) in families.iter().enumerate() {
            for &training_vars in &sizes {
                for &training_ratio in &densities {
                    for trial in 0..trials {
                        let formula_seed = 590_000
                            + family_index as u64 * 10_000
                            + training_vars as u64 * 100
                            + training_ratio as u64 * 10
                            + trial as u64;
                        let formula = generate_formula(
                            training_family,
                            training_vars,
                            training_ratio,
                            formula_seed,
                        );
                        let features = deployment_features(training_vars, &formula, branch_cap);
                        let measured = measure_assumption_service_warm(
                            training_vars,
                            &formula,
                            branch_cap,
                            max_seeds,
                            warmup,
                            queries,
                        );
                        let query_ratio = measured.seeded_query_ns as f64
                            / measured.incremental_query_ns.max(1) as f64;
                        query_training.push((features, query_ratio.max(1e-12).ln()));
                    }
                }
            }
        }
        let query_margin = upper_error_margin(&query_training);
        for (family_index, test_family) in families.iter().enumerate() {
            for &test_vars in &sizes {
                for &test_ratio in &densities {
                    for trial in 0..trials {
                        let formula_seed = 690_000
                            + family_index as u64 * 10_000
                            + test_vars as u64 * 100
                            + test_ratio as u64 * 10
                            + trial as u64;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, formula_seed);
                        let features = deployment_features(test_vars, &formula, branch_cap);
                        let predicted_query_ratio =
                            (deployment_knn(&query_training, &features, None) + query_margin).exp();

                        let mut calibration_solver = Solver::new();
                        add_to_varisat(&mut calibration_solver, &formula);
                        let calibration_start = Instant::now();
                        for query in 0..calibration_queries {
                            calibration_solver.assume(&[Lit::from_var(
                                Var::from_index(query % test_vars),
                                query % 2 == 0,
                            )]);
                            calibration_solver
                                .solve()
                                .expect("speculative calibration solve");
                        }
                        let calibration_ns = calibration_start.elapsed().as_nanos();
                        let incremental_per_query =
                            calibration_ns as f64 / calibration_queries as f64;
                        let budget_ns = if predicted_query_ratio < 1.0 {
                            (queries as f64 * incremental_per_query * (1.0 - predicted_query_ratio))
                                as u128
                        } else {
                            0
                        };

                        let compile_start = Instant::now();
                        let mut current_vars = test_vars;
                        let mut current_clauses = formula.clone();
                        let mut seeds = 0usize;
                        let mut removed = 0usize;
                        let mut aborted = budget_ns == 0;
                        if !aborted {
                            for _ in 0..max_seeds {
                                let compiled = seed_detachable_branch_bdd(
                                    current_vars,
                                    &current_clauses,
                                    branch_cap,
                                    "natural",
                                );
                                if compiled.interior.is_empty() {
                                    break;
                                }
                                removed += compiled.interior.len();
                                seeds += 1;
                                current_vars = compiled.vars;
                                current_clauses = compiled.clauses;
                                if compile_start.elapsed().as_nanos() > budget_ns {
                                    aborted = true;
                                    break;
                                }
                            }
                        }
                        let compile_ns = compile_start.elapsed().as_nanos();
                        let deployed = !aborted && seeds > 0 && compile_ns <= budget_ns;
                        let measured = measure_assumption_service_warm(
                            test_vars, &formula, branch_cap, max_seeds, warmup, queries,
                        );
                        let actual_query_ratio = measured.seeded_query_ns as f64
                            / measured.incremental_query_ns.max(1) as f64;
                        for selector in ["off", "always", "speculative", "oracle"] {
                            let selected = match selector {
                                "off" => false,
                                "always" => true,
                                "speculative" => deployed,
                                "oracle" => measured.seeded_ns < measured.incremental_ns,
                                _ => unreachable!(),
                            };
                            let policy_ns = match selector {
                                "speculative" if !deployed => {
                                    measured.incremental_ns + calibration_ns + compile_ns
                                }
                                "speculative" => measured.seeded_ns + calibration_ns,
                                _ if selected => measured.seeded_ns,
                                _ => measured.incremental_ns,
                            };
                            println!(
                                "{},{},{},{},{},{:.6},{:.6},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{}",
                                test_family,
                                test_vars,
                                formula.len(),
                                formula_seed,
                                selector,
                                predicted_query_ratio,
                                query_margin,
                                budget_ns,
                                compile_ns,
                                calibration_ns,
                                aborted,
                                deployed,
                                seeds,
                                removed,
                                measured.incremental_ns,
                                measured.seeded_ns,
                                policy_ns,
                                policy_ns as f64 / measured.incremental_ns.max(1) as f64,
                                actual_query_ratio,
                                query_training.len(),
                                measured.valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "crossover-gate" {
        let branch_cap = optional_budget.unwrap_or(8).min(12);
        let queries = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(4096usize)
            .max(2);
        let max_seeds = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        let warmup = 128usize;
        let families = ["random", "banded", "stacked-flower"];
        let sizes = [vars.saturating_sub(10).max(10), vars];
        let densities = [2usize, 4usize];
        let mut query_training = Vec::new();
        let mut setup_training = Vec::new();
        let mut end_training = Vec::new();
        let mut end_ratios = Vec::new();
        for (family_index, training_family) in families.iter().enumerate() {
            for &training_vars in &sizes {
                for &training_ratio in &densities {
                    for trial in 0..trials {
                        let formula_seed = 390_000
                            + family_index as u64 * 10_000
                            + training_vars as u64 * 100
                            + training_ratio as u64 * 10
                            + trial as u64;
                        let formula = generate_formula(
                            training_family,
                            training_vars,
                            training_ratio,
                            formula_seed,
                        );
                        let features = deployment_features(training_vars, &formula, branch_cap);
                        let measured = measure_assumption_service_warm(
                            training_vars,
                            &formula,
                            branch_cap,
                            max_seeds,
                            warmup,
                            queries,
                        );
                        let query_ratio = measured.seeded_query_ns as f64
                            / measured.incremental_query_ns.max(1) as f64;
                        let incremental_per_query =
                            measured.incremental_query_ns as f64 / queries as f64;
                        let setup_queries = measured
                            .seeded_setup_ns
                            .saturating_sub(measured.incremental_setup_ns)
                            as f64
                            / incremental_per_query.max(1.0);
                        let end_ratio =
                            measured.seeded_ns as f64 / measured.incremental_ns.max(1) as f64;
                        query_training.push((features.clone(), query_ratio.max(1e-12).ln()));
                        setup_training.push((features.clone(), setup_queries.ln_1p()));
                        end_training.push((features, end_ratio.max(1e-12).ln()));
                        end_ratios.push(end_ratio);
                    }
                }
            }
        }
        let query_margin = upper_error_margin(&query_training);
        let setup_margin = upper_error_margin(&setup_training);
        let end_predictions: Vec<_> = end_training
            .iter()
            .enumerate()
            .map(|(index, item)| deployment_knn(&end_training, &item.0, Some(index)))
            .collect();
        let mut thresholds = end_predictions.clone();
        thresholds.extend([f64::NEG_INFINITY, f64::INFINITY]);
        let end_threshold = thresholds
            .into_iter()
            .min_by(|left, right| {
                let cost = |threshold: f64| {
                    end_predictions
                        .iter()
                        .zip(&end_ratios)
                        .map(
                            |(&prediction, &actual)| {
                                if prediction < threshold { actual } else { 1.0 }
                            },
                        )
                        .sum::<f64>()
                };
                cost(*left).total_cmp(&cost(*right))
            })
            .expect("crossover end threshold");
        for (family_index, test_family) in families.iter().enumerate() {
            for &test_vars in &sizes {
                for &test_ratio in &densities {
                    for trial in 0..trials {
                        let formula_seed = 490_000
                            + family_index as u64 * 10_000
                            + test_vars as u64 * 100
                            + test_ratio as u64 * 10
                            + trial as u64;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, formula_seed);
                        let features = deployment_features(test_vars, &formula, branch_cap);
                        let predicted_query_ratio =
                            (deployment_knn(&query_training, &features, None) + query_margin).exp();
                        let predicted_setup_queries =
                            (deployment_knn(&setup_training, &features, None) + setup_margin).exp()
                                - 1.0;
                        let predicted_crossover = if predicted_query_ratio < 1.0 {
                            predicted_setup_queries / (1.0 - predicted_query_ratio)
                        } else {
                            f64::INFINITY
                        };
                        let end_prediction = deployment_knn(&end_training, &features, None);
                        let measured = measure_assumption_service_warm(
                            test_vars, &formula, branch_cap, max_seeds, warmup, queries,
                        );
                        let actual_query_ratio = measured.seeded_query_ns as f64
                            / measured.incremental_query_ns.max(1) as f64;
                        let incremental_per_query =
                            measured.incremental_query_ns as f64 / queries as f64;
                        let seeded_per_query = measured.seeded_query_ns as f64 / queries as f64;
                        let actual_crossover = if seeded_per_query < incremental_per_query {
                            measured
                                .seeded_setup_ns
                                .saturating_sub(measured.incremental_setup_ns)
                                as f64
                                / (incremental_per_query - seeded_per_query)
                        } else {
                            f64::INFINITY
                        };
                        let actual_ratio =
                            measured.seeded_ns as f64 / measured.incremental_ns.max(1) as f64;
                        for selector in [
                            "off",
                            "always",
                            "end-to-end-gate",
                            "crossover-gate",
                            "oracle",
                        ] {
                            let applied = match selector {
                                "off" => false,
                                "always" => true,
                                "end-to-end-gate" => end_prediction < end_threshold,
                                "crossover-gate" => predicted_crossover < queries as f64,
                                "oracle" => actual_ratio < 1.0,
                                _ => unreachable!(),
                            };
                            println!(
                                "{},{},{},{},{},{:.6},{:.6},{:.3},{:.6},{:.3},{},{},{:.6},{:.3},{:.6},{:.6},{},{},{},{},{},{},{},{}",
                                test_family,
                                test_vars,
                                formula.len(),
                                formula_seed,
                                selector,
                                predicted_query_ratio,
                                query_margin,
                                predicted_setup_queries,
                                setup_margin,
                                predicted_crossover,
                                queries,
                                applied,
                                actual_query_ratio,
                                actual_crossover,
                                actual_ratio,
                                if applied { actual_ratio } else { 1.0 },
                                measured.seeds,
                                measured.removed,
                                measured.incremental_setup_ns,
                                measured.seeded_setup_ns,
                                measured.incremental_query_ns,
                                measured.seeded_query_ns,
                                query_training.len(),
                                measured.valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "deployment-gate" {
        let branch_cap = optional_budget.unwrap_or(8).min(12);
        let queries = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(4096usize)
            .max(2);
        let max_seeds = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        let families = ["random", "banded", "stacked-flower"];
        let sizes = [vars.saturating_sub(10).max(10), vars];
        let densities = [2usize, 4usize];
        let mut training = Vec::new();
        let mut training_ratios = Vec::new();
        for (family_index, training_family) in families.iter().enumerate() {
            for &training_vars in &sizes {
                for &training_ratio in &densities {
                    for trial in 0..trials {
                        let formula_seed = 190_000
                            + family_index as u64 * 10_000
                            + training_vars as u64 * 100
                            + training_ratio as u64 * 10
                            + trial as u64;
                        let formula = generate_formula(
                            training_family,
                            training_vars,
                            training_ratio,
                            formula_seed,
                        );
                        let features = deployment_features(training_vars, &formula, branch_cap);
                        let measurement = measure_assumption_service(
                            training_vars,
                            &formula,
                            branch_cap,
                            max_seeds,
                            queries,
                        );
                        let ratio =
                            measurement.seeded_ns as f64 / measurement.incremental_ns.max(1) as f64;
                        training.push((features, ratio.max(1e-12).ln()));
                        training_ratios.push(ratio);
                    }
                }
            }
        }
        let predictions: Vec<_> = training
            .iter()
            .enumerate()
            .map(|(index, item)| deployment_knn(&training, &item.0, Some(index)))
            .collect();
        let mut thresholds = predictions.clone();
        thresholds.push(f64::NEG_INFINITY);
        thresholds.push(f64::INFINITY);
        let (threshold, oof_policy_ratio) = thresholds
            .into_iter()
            .map(|candidate| {
                let cost = predictions
                    .iter()
                    .zip(&training_ratios)
                    .map(|(&prediction, &actual)| if prediction < candidate { actual } else { 1.0 })
                    .sum::<f64>()
                    / training.len().max(1) as f64;
                (candidate, cost)
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .expect("deployment threshold candidates");
        let oof_applied = predictions
            .iter()
            .filter(|&&prediction| prediction < threshold)
            .count();
        for (family_index, test_family) in families.iter().enumerate() {
            for &test_vars in &sizes {
                for &test_ratio in &densities {
                    for trial in 0..trials {
                        let formula_seed = 290_000
                            + family_index as u64 * 10_000
                            + test_vars as u64 * 100
                            + test_ratio as u64 * 10
                            + trial as u64;
                        let formula =
                            generate_formula(test_family, test_vars, test_ratio, formula_seed);
                        let features = deployment_features(test_vars, &formula, branch_cap);
                        let prediction = deployment_knn(&training, &features, None);
                        let measurement = measure_assumption_service(
                            test_vars, &formula, branch_cap, max_seeds, queries,
                        );
                        let actual =
                            measurement.seeded_ns as f64 / measurement.incremental_ns.max(1) as f64;
                        for selector in ["off", "always", "oof-gate", "oracle"] {
                            let applied = match selector {
                                "off" => false,
                                "always" => true,
                                "oof-gate" => prediction < threshold,
                                "oracle" => actual < 1.0,
                                _ => unreachable!(),
                            };
                            let policy = if applied { actual } else { 1.0 };
                            println!(
                                "{},{},{},{},{},{:.6},{:.6},{},{:.6},{:.6},{},{},{},{},{},{},{},{:.6},{}",
                                test_family,
                                test_vars,
                                formula.len(),
                                formula_seed,
                                selector,
                                prediction,
                                threshold,
                                applied,
                                actual,
                                policy,
                                measurement.seeds,
                                measurement.removed,
                                measurement.live_nodes,
                                measurement.incremental_ns,
                                measurement.seeded_ns,
                                training.len(),
                                oof_applied,
                                oof_policy_ratio,
                                measurement.valid
                            );
                        }
                    }
                }
            }
        }
        return;
    }
    if engine == "assumption-payback" {
        let branch_cap = optional_budget.unwrap_or(8).min(12);
        let queries = args
            .get(8)
            .and_then(|value| value.parse().ok())
            .unwrap_or(128usize)
            .max(2);
        let max_seeds = args
            .get(9)
            .and_then(|value| value.parse().ok())
            .unwrap_or(8usize);
        for seed in 1..=trials {
            let formula_seed = 180_000 + seed as u64;
            let formula = generate_formula(family, vars, ratio, formula_seed);
            let incremental_setup_start = Instant::now();
            let mut incremental = Solver::new();
            add_to_varisat(&mut incremental, &formula);
            let incremental_setup_us = incremental_setup_start.elapsed().as_nanos();

            let seed_setup_start = Instant::now();
            let mut current_vars = vars;
            let mut current_clauses = formula.clone();
            let mut current_to_original: Vec<_> = (0..vars).collect();
            let mut seeds = Vec::new();
            for _ in 0..max_seeds {
                let compiled = seed_detachable_branch_bdd(
                    current_vars,
                    &current_clauses,
                    branch_cap,
                    "natural",
                );
                if compiled.interior.is_empty() {
                    break;
                }
                current_to_original = compiled
                    .core_to_original
                    .iter()
                    .map(|&previous| current_to_original[previous])
                    .collect();
                current_vars = compiled.vars;
                current_clauses = compiled.clauses.clone();
                seeds.push(compiled);
            }
            assert!(
                current_vars > 0,
                "assumption benchmark requires a nonempty core"
            );
            let mut seed_solver = Solver::new();
            add_to_varisat(&mut seed_solver, &current_clauses);
            let seed_setup_us = seed_setup_start.elapsed().as_nanos();
            let removed: usize = seeds.iter().map(|seed| seed.interior.len()).sum();
            let live_nodes: usize = seeds.iter().map(|seed| seed.live_nodes).sum();
            let mut cumulative_cold = 0u128;
            let mut cumulative_incremental = incremental_setup_us;
            let mut cumulative_seed = seed_setup_us;
            for query in 0..queries {
                let core_variable = query % current_vars;
                let original_variable = current_to_original[core_variable];
                let value = (query / current_vars + query) % 2 == 0;
                let mut cold_formula = formula.clone();
                cold_formula.push(Clause(vec![(original_variable, value)]));
                let cold_start = Instant::now();
                let cold_assignment = solve_with_varisat(vars, &cold_formula);
                let cold_us = cold_start.elapsed().as_nanos();
                cumulative_cold += cold_us;

                incremental.assume(&[Lit::from_var(Var::from_index(original_variable), value)]);
                let incremental_start = Instant::now();
                let incremental_sat = incremental.solve().expect("incremental assumption solve");
                let incremental_query_us = incremental_start.elapsed().as_nanos();
                cumulative_incremental += incremental_query_us;
                let mut incremental_assignment = vec![false; vars];
                if incremental_sat {
                    for literal in incremental.model().expect("incremental assumption model") {
                        if literal.var().index() < vars {
                            incremental_assignment[literal.var().index()] = literal.is_positive();
                        }
                    }
                }

                seed_solver.assume(&[Lit::from_var(Var::from_index(core_variable), value)]);
                let seed_query_start = Instant::now();
                let seed_sat = seed_solver.solve().expect("seed core assumption solve");
                let seed_query_us = seed_query_start.elapsed().as_nanos();
                cumulative_seed += seed_query_us;
                let mut core_assignment = vec![false; current_vars];
                if seed_sat {
                    for literal in seed_solver.model().expect("seed assumption model") {
                        if literal.var().index() < current_vars {
                            core_assignment[literal.var().index()] = literal.is_positive();
                        }
                    }
                }
                let seed_assignment = seed_sat
                    .then(|| regrow_seed_chain(&seeds, &core_assignment))
                    .flatten();
                let cold_sat = cold_assignment.is_some();
                let incremental_valid = !incremental_sat
                    || (incremental_assignment[original_variable] == value
                        && satisfies(&formula, &incremental_assignment));
                let seed_valid = seed_assignment.as_ref().is_none_or(|assignment| {
                    assignment[original_variable] == value && satisfies(&formula, assignment)
                });
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{},{},{},{},{},{},{},{}",
                    family,
                    vars,
                    formula.len(),
                    formula_seed,
                    query,
                    original_variable,
                    value,
                    cold_us,
                    incremental_setup_us,
                    incremental_query_us,
                    cumulative_incremental as f64 / cumulative_cold.max(1) as f64,
                    seed_setup_us,
                    seed_query_us,
                    cumulative_seed as f64 / cumulative_cold.max(1) as f64,
                    seeds.len(),
                    removed,
                    current_vars,
                    live_nodes,
                    cold_sat,
                    incremental_sat == cold_sat,
                    seed_sat == cold_sat,
                    incremental_valid,
                    seed_valid
                );
            }
        }
        return;
    }
    if engine == "exact-width-helper" {
        assert!(
            vars <= 16,
            "exact-width-helper supports at most 16 original variables"
        );
        let candidate_limit = optional_budget.unwrap_or(12);
        for seed in 1..=trials {
            let formula = generate_formula(family, vars, ratio, 100_000 + seed as u64);
            let original_treewidth = exact_treewidth(vars, &formula);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let candidates: Vec<_> = recurring_pair_candidates(&formula)
                .into_iter()
                .take(candidate_limit)
                .filter_map(|(pair, frequency)| {
                    add_pair_helper(vars, &formula, pair).map(|expanded| {
                        let width = exact_treewidth(vars + 1, &expanded);
                        let osmotic = osmotic_helper_score(vars, &formula, pair, frequency);
                        (pair, frequency, osmotic, width, expanded)
                    })
                })
                .collect();
            let frequency_choice = candidates
                .iter()
                .max_by_key(|candidate| candidate.1)
                .map(|candidate| candidate.0);
            let osmotic_choice = candidates
                .iter()
                .max_by(|a, b| a.2.total_cmp(&b.2))
                .map(|candidate| candidate.0);
            let oracle_choice = candidates
                .iter()
                .min_by_key(|candidate| candidate.3)
                .map(|candidate| candidate.0);
            for (selector, choice) in [
                ("original", None),
                ("frequency", frequency_choice),
                ("osmosis", osmotic_choice),
                ("oracle", oracle_choice),
            ] {
                let selected =
                    choice.and_then(|pair| candidates.iter().find(|candidate| candidate.0 == pair));
                let (expanded_vars, expanded, final_treewidth) = selected.map_or_else(
                    || (vars, formula.clone(), original_treewidth),
                    |candidate| (vars + 1, candidate.4.clone(), candidate.3),
                );
                let order = min_fill_order(expanded_vars, &expanded);
                let mut rank = vec![0usize; expanded_vars];
                for (level, &variable) in order.iter().enumerate() {
                    rank[variable] = level;
                }
                let mapped: Vec<_> = expanded
                    .iter()
                    .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                    .collect();
                let result = solve_tuple_natural(expanded_vars, &mapped);
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{}",
                    family,
                    vars,
                    formula.len(),
                    100_000 + seed as u64,
                    selector,
                    candidates.len(),
                    selected.is_some(),
                    original_treewidth,
                    final_treewidth,
                    final_treewidth as isize - original_treewidth as isize,
                    original_result.nodes,
                    result.nodes,
                    result.nodes as f64 / original_result.nodes.max(1) as f64,
                    result.assignment.is_some() == original_result.assignment.is_some(),
                    result
                        .assignment
                        .as_ref()
                        .is_none_or(|assignment| satisfies(&mapped, assignment))
                );
            }
        }
        return;
    }
    if engine == "helper-width-scaling" {
        let budget = optional_budget.unwrap_or((vars / 10).max(1));
        for seed in 1..=trials {
            let formula = generate_formula(family, vars, ratio, 90_000 + seed as u64);
            let original_order = min_fill_order(vars, &formula);
            let mut original_rank = vec![0usize; vars];
            for (level, &variable) in original_order.iter().enumerate() {
                original_rank[variable] = level;
            }
            let original_mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(v, s)| (original_rank[v], s))
                            .collect(),
                    )
                })
                .collect();
            let original_result = solve_tuple_natural(vars, &original_mapped);
            let (frequency_vars, frequency_formula, frequency_accepted) =
                expand_recurring_pairs(vars, &formula, budget);
            let (scent_vars, scent_formula, scent_accepted, _) =
                scent_batch_expand(vars, &formula, budget, 4);
            for (variant, expanded_vars, expanded, accepted) in [
                ("original", vars, formula.clone(), 0),
                (
                    "frequency",
                    frequency_vars,
                    frequency_formula,
                    frequency_accepted,
                ),
                ("scent", scent_vars, scent_formula, scent_accepted),
            ] {
                let order = min_fill_order(expanded_vars, &expanded);
                let (width, estimated_work) = elimination_cost(expanded_vars, &expanded, &order);
                let mut rank = vec![0usize; expanded_vars];
                for (level, &variable) in order.iter().enumerate() {
                    rank[variable] = level;
                }
                let mapped: Vec<_> = expanded
                    .iter()
                    .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                    .collect();
                let start = Instant::now();
                let result = solve_tuple_natural(expanded_vars, &mapped);
                let solve_us = start.elapsed().as_micros();
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{:.3},{},{},{},{},{}",
                    family,
                    vars,
                    formula.len(),
                    90_000 + seed as u64,
                    variant,
                    budget,
                    accepted,
                    expanded_vars,
                    expanded.len(),
                    width,
                    estimated_work,
                    solve_us,
                    result.nodes,
                    result.assignment.is_some(),
                    result.assignment.is_some() == original_result.assignment.is_some(),
                    result
                        .assignment
                        .as_ref()
                        .is_none_or(|assignment| satisfies(&mapped, assignment))
                );
            }
        }
        return;
    }
    if engine == "structure-scaling" {
        for seed in 1..=trials {
            let formula = generate_formula(family, vars, ratio, 80_000 + seed as u64);
            let order = min_fill_order(vars, &formula);
            let (width, estimated_work) = elimination_cost(vars, &formula, &order);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let start = Instant::now();
            let result = solve_tuple_natural(vars, &mapped);
            let solve_us = start.elapsed().as_micros();
            let valid = result
                .assignment
                .as_ref()
                .is_none_or(|assignment| satisfies(&mapped, assignment));
            println!(
                "{},{},{},{},{},{:.3},{},{},{},{}",
                family,
                vars,
                mapped.len(),
                80_000 + seed as u64,
                width,
                estimated_work,
                solve_us,
                result.nodes,
                result.assignment.is_some(),
                valid
            );
        }
        return;
    }
    if engine == "kernel-scaling" {
        for test_family in ["random", "banded"] {
            for seed in 1..=trials {
                let formula = generate_formula(test_family, vars, ratio, 70_000 + seed as u64);
                let order = min_fill_order(vars, &formula);
                let mut rank = vec![0usize; vars];
                for (level, &variable) in order.iter().enumerate() {
                    rank[variable] = level;
                }
                let mapped: Vec<_> = formula
                    .iter()
                    .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                    .collect();
                let natural: Vec<_> = (0..vars).collect();
                let run_ordinary = || {
                    let start = Instant::now();
                    let result = eliminate_with_bdds(vars, &mapped, &natural);
                    (start.elapsed().as_micros(), result)
                };
                let run_specialized = || {
                    let start = Instant::now();
                    let result = solve_tuple_natural(vars, &mapped);
                    (start.elapsed().as_micros(), result)
                };
                let (ordinary_us, ordinary, specialized_us, specialized) = if seed % 2 == 0 {
                    let (specialized_us, specialized) = run_specialized();
                    let (ordinary_us, ordinary) = run_ordinary();
                    (ordinary_us, ordinary, specialized_us, specialized)
                } else {
                    let (ordinary_us, ordinary) = run_ordinary();
                    let (specialized_us, specialized) = run_specialized();
                    (ordinary_us, ordinary, specialized_us, specialized)
                };
                let agrees = ordinary.assignment.is_some() == specialized.assignment.is_some();
                for (kernel, solve_us, nodes, assignment) in [
                    (
                        "runtime-order",
                        ordinary_us,
                        ordinary.allocated_nodes,
                        ordinary.assignment.as_ref(),
                    ),
                    (
                        "natural-specialized",
                        specialized_us,
                        specialized.nodes,
                        specialized.assignment.as_ref(),
                    ),
                ] {
                    println!(
                        "{},{},{},{},{},{},{},{},{}",
                        test_family,
                        vars,
                        mapped.len(),
                        70_000 + seed as u64,
                        kernel,
                        solve_us,
                        nodes,
                        assignment.is_some(),
                        agrees
                            && assignment.is_none_or(|item| satisfies(&mapped, item))
                            && nodes == ordinary.allocated_nodes
                    );
                }
            }
        }
        return;
    }
    if engine == "direct-layout-ablation" {
        let cases: Vec<(String, usize, Vec<Clause>)> = if family == "external" {
            let directory = args
                .get(7)
                .map(String::as_str)
                .unwrap_or("benchmarks/satlib");
            let mut paths: Vec<_> = fs::read_dir(directory)
                .expect("read external benchmark directory")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "cnf"))
                .collect();
            paths.sort();
            paths
                .into_iter()
                .map(|path| {
                    let (case_vars, formula) = parse_dimacs(&path).unwrap();
                    (
                        path.file_name().unwrap().to_string_lossy().into_owned(),
                        case_vars,
                        formula,
                    )
                })
                .collect()
        } else {
            (1..=trials)
                .map(|seed| {
                    (
                        family.to_string(),
                        vars,
                        generate_formula(family, vars, ratio, 60_000 + seed as u64),
                    )
                })
                .collect()
        };
        let names = [
            "ordinary",
            "tuple-natural",
            "layout",
            "provenance",
            "checkpoints",
            "full",
        ];
        for (index, (case_name, case_vars, formula)) in cases.into_iter().enumerate() {
            let order = min_fill_order(case_vars, &formula);
            let mut rank = vec![0usize; case_vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let natural: Vec<_> = (0..case_vars).collect();
            let mut results: Vec<Option<(u128, DirectVariantResult)>> =
                (0..names.len()).map(|_| None).collect();
            for offset in 0..names.len() {
                let variant = (index + offset) % names.len();
                let start = Instant::now();
                let result = if variant == 0 {
                    let ordinary = eliminate_with_bdds(case_vars, &mapped, &natural);
                    DirectVariantResult {
                        assignment: ordinary.assignment,
                        nodes: ordinary.allocated_nodes,
                    }
                } else if variant == 1 {
                    solve_tuple_natural(case_vars, &mapped)
                } else {
                    solve_provenance_variant(
                        case_vars,
                        &mapped,
                        variant == 3 || variant == 5,
                        variant == 4 || variant == 5,
                        4,
                    )
                };
                results[variant] = Some((start.elapsed().as_micros(), result));
            }
            let direct_us = results[0].as_ref().unwrap().0;
            let direct_nodes = results[0].as_ref().unwrap().1.nodes;
            let direct_sat = results[0].as_ref().unwrap().1.assignment.is_some();
            for (variant, name) in names.iter().enumerate() {
                let (solve_us, result) = results[variant].as_ref().unwrap();
                let valid = result.assignment.is_some() == direct_sat
                    && result
                        .assignment
                        .as_ref()
                        .is_none_or(|assignment| satisfies(&mapped, assignment));
                println!(
                    "{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{}",
                    case_name,
                    case_vars,
                    mapped.len(),
                    index + 1,
                    name,
                    solve_us,
                    direct_us,
                    direct_us as f64 / (*solve_us).max(1) as f64,
                    result.nodes,
                    direct_nodes,
                    result.nodes as f64 / direct_nodes.max(1) as f64,
                    result.assignment.is_some(),
                    valid
                );
            }
        }
        return;
    }
    if engine == "direct-provenance-portfolio" {
        let training_trials = (trials / 2).max(2);
        let mut training = Vec::new();
        for training_vars in [24, 30, 36] {
            for training_ratio in [3, 4, 5] {
                for training_family in ["random", "banded"] {
                    for training_seed in 1..=training_trials {
                        let formula = generate_formula(
                            training_family,
                            training_vars,
                            training_ratio,
                            training_seed as u64,
                        );
                        let order = min_fill_order(training_vars, &formula);
                        let mut rank = vec![0usize; training_vars];
                        for (level, &variable) in order.iter().enumerate() {
                            rank[variable] = level;
                        }
                        let mapped: Vec<_> = formula
                            .iter()
                            .map(|clause| {
                                Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect())
                            })
                            .collect();
                        let natural: Vec<_> = (0..training_vars).collect();
                        let (direct_us, provenance_us) = if training_seed % 2 == 0 {
                            let provenance_start = Instant::now();
                            let _provenance =
                                build_incremental_bdd_cache_with_stride(training_vars, &mapped, 4);
                            let provenance_us = provenance_start.elapsed().as_micros();
                            let direct_start = Instant::now();
                            let _direct = eliminate_with_bdds(training_vars, &mapped, &natural);
                            (direct_start.elapsed().as_micros().max(1), provenance_us)
                        } else {
                            let direct_start = Instant::now();
                            let _direct = eliminate_with_bdds(training_vars, &mapped, &natural);
                            let direct_us = direct_start.elapsed().as_micros().max(1);
                            let provenance_start = Instant::now();
                            let _provenance =
                                build_incremental_bdd_cache_with_stride(training_vars, &mapped, 4);
                            (direct_us, provenance_start.elapsed().as_micros())
                        };
                        training.push((
                            cheap_structure_features(training_vars, &formula),
                            (provenance_us as f64 / direct_us as f64).max(1e-9).ln(),
                        ));
                    }
                }
            }
        }
        let cases: Vec<(String, usize, Vec<Clause>)> = if family == "external" {
            let directory = args
                .get(7)
                .map(String::as_str)
                .unwrap_or("benchmarks/satlib");
            let mut paths: Vec<_> = fs::read_dir(directory)
                .expect("read external benchmark directory")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "cnf"))
                .collect();
            paths.sort();
            paths
                .into_iter()
                .map(|path| {
                    let (case_vars, formula) = parse_dimacs(&path).unwrap();
                    (
                        path.file_name().unwrap().to_string_lossy().into_owned(),
                        case_vars,
                        formula,
                    )
                })
                .collect()
        } else {
            (1..=(trials - training_trials).max(1))
                .map(|seed| {
                    (
                        family.to_string(),
                        vars,
                        generate_formula(family, vars, ratio, 50_000 + seed as u64),
                    )
                })
                .collect()
        };
        for (index, (case_name, case_vars, formula)) in cases.into_iter().enumerate() {
            let order = min_fill_order(case_vars, &formula);
            let mut rank = vec![0usize; case_vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let natural: Vec<_> = (0..case_vars).collect();
            let (direct, direct_us, provenance, provenance_us) = if index % 2 == 0 {
                let provenance_start = Instant::now();
                let provenance = build_incremental_bdd_cache_with_stride(case_vars, &mapped, 4);
                let provenance_us = provenance_start.elapsed().as_micros();
                let direct_start = Instant::now();
                let direct = eliminate_with_bdds(case_vars, &mapped, &natural);
                (
                    direct,
                    direct_start.elapsed().as_micros(),
                    provenance,
                    provenance_us,
                )
            } else {
                let direct_start = Instant::now();
                let direct = eliminate_with_bdds(case_vars, &mapped, &natural);
                let direct_us = direct_start.elapsed().as_micros();
                let provenance_start = Instant::now();
                let provenance = build_incremental_bdd_cache_with_stride(case_vars, &mapped, 4);
                (
                    direct,
                    direct_us,
                    provenance,
                    provenance_start.elapsed().as_micros(),
                )
            };
            let decision_start = Instant::now();
            let prediction = scent_gate_predict(
                &training,
                &cheap_structure_features(case_vars, &formula),
                11,
            );
            let decision_us = decision_start.elapsed().as_micros();
            for selector in ["direct", "provenance", "cost-gate"] {
                let apply =
                    selector == "provenance" || (selector == "cost-gate" && prediction < -0.10);
                let policy = if apply { "provenance" } else { "direct" };
                let (sat, solve_us, nodes, valid) = if apply {
                    (
                        provenance.assignment.is_some(),
                        provenance_us,
                        provenance.manager.nodes.len(),
                        provenance
                            .assignment
                            .as_ref()
                            .is_none_or(|assignment| satisfies(&mapped, assignment))
                            && provenance.assignment.is_some() == direct.assignment.is_some(),
                    )
                } else {
                    (
                        direct.assignment.is_some(),
                        direct_us,
                        direct.allocated_nodes,
                        direct
                            .assignment
                            .as_ref()
                            .is_none_or(|assignment| satisfies(&mapped, assignment)),
                    )
                };
                let charged_decision = if selector == "cost-gate" {
                    decision_us
                } else {
                    0
                };
                let total_us = charged_decision + solve_us;
                println!(
                    "{},{},{},{},{},{:.6},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                    case_name,
                    case_vars,
                    mapped.len(),
                    index + 1,
                    selector,
                    prediction,
                    policy,
                    sat,
                    charged_decision,
                    solve_us,
                    total_us,
                    direct_us,
                    direct_us as f64 / total_us.max(1) as f64,
                    nodes,
                    direct.allocated_nodes,
                    nodes as f64 / direct.allocated_nodes.max(1) as f64,
                    valid
                );
            }
        }
        return;
    }
    if engine == "replay-metrics" {
        let cases: Vec<(String, usize, Vec<Clause>)> = if family == "external" {
            let directory = args
                .get(7)
                .map(String::as_str)
                .unwrap_or("benchmarks/satlib");
            let mut paths: Vec<_> = fs::read_dir(directory)
                .expect("read external benchmark directory")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "cnf"))
                .collect();
            paths.sort();
            paths
                .into_iter()
                .map(|path| {
                    let (case_vars, formula) = parse_dimacs(&path).unwrap();
                    (
                        path.file_name().unwrap().to_string_lossy().into_owned(),
                        case_vars,
                        formula,
                    )
                })
                .collect()
        } else {
            (1..=trials)
                .map(|seed| {
                    (
                        family.to_string(),
                        vars,
                        generate_formula(family, vars, ratio, seed as u64),
                    )
                })
                .collect()
        };
        for (index, (case_name, case_vars, formula)) in cases.into_iter().enumerate() {
            let order = min_fill_order(case_vars, &formula);
            let mut rank = vec![0usize; case_vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let natural: Vec<_> = (0..case_vars).collect();
            let direct_start = Instant::now();
            let direct = eliminate_with_bdds(case_vars, &mapped, &natural);
            let direct_us = direct_start.elapsed().as_micros();
            let reuse = evaluate_branch_choice(case_vars, &mapped, case_vars - 1, 4);
            println!(
                "{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{},{},{:.6},{}",
                case_name,
                case_vars,
                mapped.len(),
                index + 1,
                reuse.satisfiable,
                reuse.first_satisfiable,
                reuse.branches,
                direct_us,
                reuse.reuse_us,
                direct_us as f64 / reuse.reuse_us.max(1) as f64,
                reuse.cache_build_us,
                reuse.sibling_us,
                reuse.cache_nodes,
                reuse.sibling_new_nodes,
                reuse.checkpoint_count,
                reuse.restored_layer,
                reuse.replayed_layers,
                direct.allocated_nodes,
                reuse.reuse_nodes,
                reuse.reuse_nodes as f64 / direct.allocated_nodes.max(1) as f64,
                reuse.valid && reuse.satisfiable == direct.assignment.is_some()
            );
        }
        return;
    }
    if engine == "robust-reuse-portfolio"
        || engine == "external-reuse-portfolio"
        || engine == "external-ablation"
    {
        let ablation = engine == "external-ablation";
        let external = engine == "external-reuse-portfolio" || ablation;
        let training_trials = (trials / 2).max(2);
        let test_trials = (trials - training_trials).max(1);
        let mut records = Vec::new();
        let mut regime = 0usize;
        for training_vars in [24, 30, 36] {
            for training_ratio in [3, 4, 5] {
                for training_family in ["random", "banded"] {
                    for training_seed in 1..=training_trials {
                        let formula = generate_formula(
                            training_family,
                            training_vars,
                            training_ratio,
                            training_seed as u64,
                        );
                        let order = min_fill_order(training_vars, &formula);
                        let mut rank = vec![0usize; training_vars];
                        for (level, &variable) in order.iter().enumerate() {
                            rank[variable] = level;
                        }
                        let mapped: Vec<_> = formula
                            .iter()
                            .map(|clause| {
                                Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect())
                            })
                            .collect();
                        let natural: Vec<_> = (0..training_vars).collect();
                        let direct_start = Instant::now();
                        let _direct = eliminate_with_bdds(training_vars, &mapped, &natural);
                        let direct_us = direct_start.elapsed().as_micros().max(1);
                        let reuse =
                            evaluate_branch_choice(training_vars, &mapped, training_vars - 1, 4);
                        records.push((
                            cheap_structure_features(training_vars, &formula),
                            (reuse.reuse_us as f64 / direct_us as f64).max(1e-9).ln(),
                            regime,
                        ));
                    }
                    regime += 1;
                }
            }
        }
        let training: Vec<_> = records.iter().map(|item| (item.0, item.1)).collect();
        let (prediction_threshold, distance_threshold, _, _) = learn_regime_rejection(&records, 11);
        let cases: Vec<(String, usize, Vec<Clause>)> = if external {
            let directory = args
                .get(7)
                .map(String::as_str)
                .unwrap_or("benchmarks/satlib");
            let mut paths: Vec<_> = fs::read_dir(directory)
                .expect("read external benchmark directory")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "cnf"))
                .collect();
            paths.sort();
            paths
                .into_iter()
                .map(|path| {
                    let (case_vars, formula) =
                        parse_dimacs(&path).unwrap_or_else(|error| panic!("{error}"));
                    (
                        path.file_name().unwrap().to_string_lossy().into_owned(),
                        case_vars,
                        formula,
                    )
                })
                .collect()
        } else {
            (1..=test_trials)
                .map(|test_index| {
                    let test_seed = 40_000 + test_index as u64;
                    (
                        family.to_string(),
                        vars,
                        generate_formula(family, vars, ratio, test_seed),
                    )
                })
                .collect()
        };
        for (test_index, (case_name, vars, formula)) in cases.into_iter().enumerate() {
            let test_seed = 40_001 + test_index as u64;
            let family = case_name.as_str();
            let order = min_fill_order(vars, &formula);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let natural: Vec<_> = (0..vars).collect();
            let direct_start = Instant::now();
            let direct = eliminate_with_bdds(vars, &mapped, &natural);
            let direct_us = direct_start.elapsed().as_micros();
            let decision_start = Instant::now();
            let features = cheap_structure_features(vars, &formula);
            let prediction = scent_gate_predict(&training, &features, 11);
            let distance = support_distance(&training, &features);
            let decision_us = decision_start.elapsed().as_micros();
            let (reuse, forced) = if ablation && test_index % 2 == 0 {
                let forced = evaluate_forced_branch_pair(vars, &mapped, vars - 1, 4);
                let reuse = evaluate_branch_choice(vars, &mapped, vars - 1, 4);
                (reuse, Some(forced))
            } else {
                let reuse = evaluate_branch_choice(vars, &mapped, vars - 1, 4);
                let forced =
                    ablation.then(|| evaluate_forced_branch_pair(vars, &mapped, vars - 1, 4));
                (reuse, forced)
            };
            let portfolio_reuse =
                prediction < prediction_threshold && distance <= distance_threshold;
            let selectors: &[&str] = if ablation {
                &[
                    "direct",
                    "complete",
                    "no-gate",
                    "no-reuse",
                    "fresh-no-gate",
                    "no-early-stop",
                    "forced-no-gate",
                    "cost-gate",
                ]
            } else {
                &["direct", "reuse", "portfolio", "cost-gate"]
            };
            for &selector in selectors {
                let apply_reuse = selector == "reuse"
                    || selector == "no-gate"
                    || (selector == "cost-gate" && prediction < prediction_threshold)
                    || ((selector == "portfolio" || selector == "complete") && portfolio_reuse);
                let apply_fresh =
                    (selector == "no-reuse" && portfolio_reuse) || selector == "fresh-no-gate";
                let apply_forced = (selector == "no-early-stop" && portfolio_reuse)
                    || selector == "forced-no-gate";
                let policy = if apply_reuse {
                    "reuse"
                } else if apply_fresh {
                    "fresh-branch"
                } else if apply_forced {
                    "forced-pair"
                } else {
                    "direct"
                };
                let (branches, satisfiable, solve_us, work_nodes, valid) = if apply_reuse {
                    (
                        reuse.branches,
                        reuse.satisfiable,
                        reuse.reuse_us,
                        reuse.reuse_nodes,
                        reuse.valid && reuse.satisfiable == direct.assignment.is_some(),
                    )
                } else if apply_fresh {
                    (
                        reuse.branches,
                        reuse.satisfiable,
                        reuse.fresh_us,
                        reuse.fresh_nodes,
                        reuse.valid && reuse.satisfiable == direct.assignment.is_some(),
                    )
                } else if apply_forced {
                    let forced = forced.as_ref().unwrap();
                    (
                        forced.branches,
                        forced.satisfiable,
                        forced.reuse_us,
                        forced.reuse_nodes,
                        forced.valid && forced.satisfiable == direct.assignment.is_some(),
                    )
                } else {
                    (
                        1,
                        direct.assignment.is_some(),
                        direct_us,
                        direct.allocated_nodes,
                        direct
                            .assignment
                            .as_ref()
                            .is_none_or(|assignment| satisfies(&mapped, assignment)),
                    )
                };
                let charged_decision = if matches!(
                    selector,
                    "portfolio" | "complete" | "no-reuse" | "no-early-stop" | "cost-gate"
                ) {
                    decision_us
                } else {
                    0
                };
                let total_us = charged_decision + solve_us;
                println!(
                    "{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                    family,
                    vars,
                    mapped.len(),
                    test_seed,
                    selector,
                    prediction,
                    distance,
                    prediction_threshold,
                    distance_threshold,
                    policy,
                    branches,
                    satisfiable,
                    charged_decision,
                    solve_us,
                    total_us,
                    direct_us,
                    direct_us as f64 / total_us.max(1) as f64,
                    work_nodes,
                    direct.allocated_nodes,
                    work_nodes as f64 / direct.allocated_nodes.max(1) as f64,
                    valid
                );
            }
        }
        return;
    }
    if engine == "branch-portfolio"
        || engine == "cheap-branch-portfolio"
        || engine == "three-action-portfolio"
        || engine == "portfolio-generalization"
    {
        let generalization = engine == "portfolio-generalization";
        let three_action = engine == "three-action-portfolio" || generalization;
        let cheap_gate = engine != "branch-portfolio";
        let model_vars = if generalization { 30 } else { vars };
        let model_ratio = if generalization { 4 } else { ratio };
        let training_trials = (trials / 2).max(1);
        let test_trials = (trials - training_trials).max(1);
        let mut branch_training = Vec::new();
        let mut structure_training = Vec::new();
        for training_seed in 1..=training_trials {
            for training_family in ["random", "banded"] {
                let formula = generate_formula(
                    training_family,
                    model_vars,
                    model_ratio,
                    training_seed as u64,
                );
                let structure_features = if cheap_gate {
                    cheap_structure_features(model_vars, &formula)
                } else {
                    scent_gate_features(model_vars, &formula)
                };
                structure_training
                    .push((structure_features, f64::from(training_family == "banded")));
                if training_family == "random" {
                    let order = min_fill_order(model_vars, &formula);
                    let mut rank = vec![0usize; model_vars];
                    for (level, &variable) in order.iter().enumerate() {
                        rank[variable] = level;
                    }
                    let mapped: Vec<_> = formula
                        .iter()
                        .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                        .collect();
                    let feature_matrix = supervised_branch_feature_matrix(model_vars, &mapped);
                    for variable in 0..model_vars {
                        let result = evaluate_branch_choice(model_vars, &mapped, variable, 4);
                        branch_training.push((
                            feature_matrix[variable].clone(),
                            (result.reuse_nodes as f64 + 1.0).ln(),
                        ));
                    }
                }
            }
        }
        let mut benefit_training = Vec::new();
        let mut reuse_benefit_training = Vec::new();
        if cheap_gate {
            for training_seed in 1..=training_trials {
                for &training_family in if three_action {
                    &["random", "banded"][..]
                } else {
                    &["random"][..]
                } {
                    let formula = generate_formula(
                        training_family,
                        model_vars,
                        model_ratio,
                        training_seed as u64,
                    );
                    let order = min_fill_order(model_vars, &formula);
                    let mut rank = vec![0usize; model_vars];
                    for (level, &variable) in order.iter().enumerate() {
                        rank[variable] = level;
                    }
                    let mapped: Vec<_> = formula
                        .iter()
                        .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                        .collect();
                    let natural: Vec<_> = (0..model_vars).collect();
                    let direct_start = Instant::now();
                    let _direct = eliminate_with_bdds(model_vars, &mapped, &natural);
                    let direct_us = direct_start.elapsed().as_micros().max(1);
                    let inference_start = Instant::now();
                    let matrix = supervised_branch_feature_matrix(model_vars, &mapped);
                    let variable = (0..model_vars)
                        .min_by(|&a, &b| {
                            vector_knn_predict(&branch_training, &matrix[a], 15)
                                .total_cmp(&vector_knn_predict(&branch_training, &matrix[b], 15))
                        })
                        .unwrap();
                    let inference_us = inference_start.elapsed().as_micros();
                    let branch = evaluate_branch_choice(model_vars, &mapped, variable, 4);
                    let ratio = (inference_us + branch.reuse_us) as f64 / direct_us as f64;
                    benefit_training.push((
                        cheap_structure_features(model_vars, &formula),
                        ratio.max(1e-9).ln(),
                    ));
                    let reuse = evaluate_branch_choice(model_vars, &mapped, model_vars - 1, 4);
                    reuse_benefit_training.push((
                        cheap_structure_features(model_vars, &formula),
                        (reuse.reuse_us as f64 / direct_us as f64).max(1e-9).ln(),
                    ));
                }
            }
        }
        let (benefit_threshold, _, _) = if cheap_gate {
            learn_scent_gate_threshold(&benefit_training, 7)
        } else {
            (f64::NEG_INFINITY, 1.0, 0)
        };
        let (reuse_threshold, _, _) = if three_action {
            learn_scent_gate_threshold(&reuse_benefit_training, 7)
        } else {
            (f64::NEG_INFINITY, 1.0, 0)
        };
        let test_families: Vec<&str> = if generalization {
            vec![family]
        } else {
            vec!["random", "banded"]
        };
        for test_family in test_families {
            for test_index in 1..=test_trials {
                let test_seed = 30_000 + test_index as u64;
                let formula = generate_formula(test_family, vars, ratio, test_seed);
                let order = min_fill_order(vars, &formula);
                let mut rank = vec![0usize; vars];
                for (level, &variable) in order.iter().enumerate() {
                    rank[variable] = level;
                }
                let mapped: Vec<_> = formula
                    .iter()
                    .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                    .collect();
                let natural: Vec<_> = (0..vars).collect();
                let direct_start = Instant::now();
                let direct = eliminate_with_bdds(vars, &mapped, &natural);
                let direct_us = direct_start.elapsed().as_micros();

                let structure_start = Instant::now();
                let structure_features = if cheap_gate {
                    cheap_structure_features(vars, &formula)
                } else {
                    scent_gate_features(vars, &formula)
                };
                let structure_prediction = if three_action {
                    0.0
                } else {
                    scent_gate_predict(&structure_training, &structure_features, 7)
                };
                let benefit_prediction = if cheap_gate {
                    scent_gate_predict(&benefit_training, &structure_features, 7)
                } else {
                    0.0
                };
                let reuse_prediction = if three_action {
                    scent_gate_predict(&reuse_benefit_training, &structure_features, 7)
                } else {
                    0.0
                };
                let structure_us = structure_start.elapsed().as_micros();
                let branch_start = Instant::now();
                let feature_matrix = supervised_branch_feature_matrix(vars, &mapped);
                let predictions: Vec<_> = (0..vars)
                    .map(|variable| {
                        vector_knn_predict(&branch_training, &feature_matrix[variable], 15)
                    })
                    .collect();
                let branch_us = branch_start.elapsed().as_micros();
                let supervised_variable = (0..vars)
                    .min_by(|&a, &b| predictions[a].total_cmp(&predictions[b]))
                    .unwrap();
                let impact_scores = branching_scores(vars, &mapped, 0.0);
                let impact_variable = (0..vars)
                    .max_by(|&a, &b| impact_scores[a].total_cmp(&impact_scores[b]))
                    .unwrap();
                let results: Vec<_> = (0..vars)
                    .map(|variable| evaluate_branch_choice(vars, &mapped, variable, 4))
                    .collect();
                let oracle_variable = (0..vars)
                    .min_by_key(|&variable| results[variable].reuse_nodes)
                    .unwrap();
                let oracle_nodes = direct
                    .allocated_nodes
                    .min(results[oracle_variable].reuse_nodes);
                let portfolio_policy = if three_action {
                    let supervised_allowed = benefit_prediction < benefit_threshold;
                    let reuse_allowed = reuse_prediction < reuse_threshold;
                    if supervised_allowed
                        && (!reuse_allowed || benefit_prediction < reuse_prediction)
                    {
                        "supervised"
                    } else if reuse_allowed {
                        "reuse"
                    } else {
                        "direct"
                    }
                } else if cheap_gate {
                    if structure_prediction < 0.5 && benefit_prediction < benefit_threshold {
                        "supervised"
                    } else {
                        "direct"
                    }
                } else if structure_prediction < 0.25 {
                    "supervised"
                } else if structure_prediction > 0.75 {
                    "reuse"
                } else {
                    "direct"
                };
                for selector in [
                    "direct",
                    "impact",
                    "reuse",
                    "supervised",
                    "portfolio",
                    "oracle",
                ] {
                    let policy = match selector {
                        "portfolio" => portfolio_policy,
                        "oracle" => {
                            if direct.allocated_nodes <= results[oracle_variable].reuse_nodes {
                                "direct"
                            } else {
                                "oracle-branch"
                            }
                        }
                        other => other,
                    };
                    let variable = match policy {
                        "impact" => Some(impact_variable),
                        "reuse" => Some(vars - 1),
                        "supervised" => Some(supervised_variable),
                        "oracle-branch" => Some(oracle_variable),
                        _ => None,
                    };
                    let decision_us = match selector {
                        "supervised" => branch_us,
                        "portfolio" => {
                            structure_us + if policy == "supervised" { branch_us } else { 0 }
                        }
                        _ => 0,
                    };
                    let (branches, satisfiable, solve_us, work_nodes, valid) =
                        if let Some(variable) = variable {
                            let result = &results[variable];
                            (
                                result.branches,
                                result.satisfiable,
                                result.reuse_us,
                                result.reuse_nodes,
                                result.valid && result.satisfiable == direct.assignment.is_some(),
                            )
                        } else {
                            (
                                1,
                                direct.assignment.is_some(),
                                direct_us,
                                direct.allocated_nodes,
                                direct
                                    .assignment
                                    .as_ref()
                                    .is_none_or(|assignment| satisfies(&mapped, assignment)),
                            )
                        };
                    let total_us = decision_us + solve_us;
                    let branch_level = variable.map_or(usize::MAX, |item| item);
                    let branch_variable = variable.map_or(usize::MAX, |item| order[item]);
                    println!(
                        "{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{:.6},{}",
                        test_family,
                        vars,
                        mapped.len(),
                        test_seed,
                        selector,
                        if three_action {
                            benefit_prediction.min(reuse_prediction)
                        } else if cheap_gate {
                            benefit_prediction
                        } else {
                            structure_prediction
                        },
                        policy,
                        branch_level,
                        branch_variable,
                        branches,
                        satisfiable,
                        decision_us,
                        solve_us,
                        total_us,
                        direct_us,
                        direct_us as f64 / total_us.max(1) as f64,
                        work_nodes,
                        direct.allocated_nodes,
                        work_nodes as f64 / direct.allocated_nodes.max(1) as f64,
                        oracle_nodes,
                        work_nodes as f64 / oracle_nodes.max(1) as f64,
                        valid
                    );
                }
            }
        }
        return;
    }
    if engine == "supervised-branch" {
        let training_trials = (trials / 2).max(1);
        let test_trials = (trials - training_trials).max(1);
        let mut training = Vec::new();
        for training_seed in 1..=training_trials {
            let formula = generate_formula(family, vars, ratio, training_seed as u64);
            let order = min_fill_order(vars, &formula);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let feature_matrix = supervised_branch_feature_matrix(vars, &mapped);
            for variable in 0..vars {
                let result = evaluate_branch_choice(vars, &mapped, variable, 4);
                training.push((
                    feature_matrix[variable].clone(),
                    (result.reuse_nodes as f64 + 1.0).ln(),
                ));
            }
        }
        for test_index in 1..=test_trials {
            let test_seed = 20_000 + test_index as u64;
            let formula = generate_formula(family, vars, ratio, test_seed);
            let order = min_fill_order(vars, &formula);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let inference_start = Instant::now();
            let feature_matrix = supervised_branch_feature_matrix(vars, &mapped);
            let predictions: Vec<_> = (0..vars)
                .map(|variable| vector_knn_predict(&training, &feature_matrix[variable], 15))
                .collect();
            let inference_us = inference_start.elapsed().as_micros();
            let supervised_variable = (0..vars)
                .min_by(|&a, &b| predictions[a].total_cmp(&predictions[b]))
                .unwrap();
            let impact_scores = branching_scores(vars, &mapped, 0.0);
            let impact_variable = (0..vars)
                .max_by(|&a, &b| impact_scores[a].total_cmp(&impact_scores[b]))
                .unwrap();
            let random_variable = Rng(test_seed).below(vars);
            let results: Vec<_> = (0..vars)
                .map(|variable| evaluate_branch_choice(vars, &mapped, variable, 4))
                .collect();
            let oracle_variable = (0..vars)
                .min_by_key(|&variable| results[variable].reuse_nodes)
                .unwrap();
            let oracle_nodes = results[oracle_variable].reuse_nodes;
            for (selector, variable) in [
                ("supervised", supervised_variable),
                ("impact", impact_variable),
                ("reuse", vars - 1),
                ("random", random_variable),
                ("oracle", oracle_variable),
            ] {
                let result = &results[variable];
                println!(
                    "{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{:.6},{},{},{:.6},{},{:.6},{}",
                    family,
                    vars,
                    mapped.len(),
                    test_seed,
                    selector,
                    variable,
                    order[variable],
                    predictions[variable],
                    inference_us,
                    result.branches,
                    result.satisfiable,
                    result.reuse_us,
                    result.fresh_us,
                    result.fresh_us as f64 / result.reuse_us.max(1) as f64,
                    result.reuse_nodes,
                    oracle_nodes,
                    result.reuse_nodes as f64 / oracle_nodes.max(1) as f64,
                    result.fresh_nodes,
                    result.reuse_nodes as f64 / result.fresh_nodes.max(1) as f64,
                    result.valid
                );
            }
        }
        return;
    }
    if engine == "joint-branch" {
        let training_trials = (trials / 2).max(1);
        let test_trials = (trials - training_trials).max(1);
        let alphas = [0.0f64, 0.25, 0.5, 1.0, 2.0, 4.0];
        let mut training_work = vec![0usize; alphas.len()];
        for training_seed in 1..=training_trials {
            let formula = generate_formula(family, vars, ratio, training_seed as u64);
            let order = min_fill_order(vars, &formula);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            for (index, &alpha) in alphas.iter().enumerate() {
                let scores = branching_scores(vars, &mapped, alpha);
                let variable = (0..vars)
                    .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
                    .unwrap();
                training_work[index] +=
                    evaluate_branch_choice(vars, &mapped, variable, 4).reuse_nodes;
            }
        }
        let best_alpha_index = (0..alphas.len())
            .min_by_key(|&index| training_work[index])
            .unwrap();
        let learned_alpha = alphas[best_alpha_index];
        for test_index in 1..=test_trials {
            let test_seed = 10_000 + test_index as u64;
            let formula = generate_formula(family, vars, ratio, test_seed);
            let order = min_fill_order(vars, &formula);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let impact_scores = branching_scores(vars, &mapped, 0.0);
            let joint_scores = branching_scores(vars, &mapped, learned_alpha);
            let impact_variable = (0..vars)
                .max_by(|&a, &b| impact_scores[a].total_cmp(&impact_scores[b]))
                .unwrap();
            let joint_variable = (0..vars)
                .max_by(|&a, &b| joint_scores[a].total_cmp(&joint_scores[b]))
                .unwrap();
            let random_variable = Rng(test_seed).below(vars);
            let mut oracle_results: Vec<_> = (0..vars)
                .map(|variable| (variable, evaluate_branch_choice(vars, &mapped, variable, 4)))
                .collect();
            let oracle_index = (0..oracle_results.len())
                .min_by_key(|&index| oracle_results[index].1.reuse_nodes)
                .unwrap();
            let (oracle_variable, oracle_result) = oracle_results.swap_remove(oracle_index);
            let strategies = [
                ("impact", impact_variable),
                ("reuse", vars - 1),
                ("joint", joint_variable),
                ("random", random_variable),
            ];
            for (selector, variable) in strategies {
                let result = evaluate_branch_choice(vars, &mapped, variable, 4);
                println!(
                    "{},{},{},{},{},{:.2},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                    family,
                    vars,
                    mapped.len(),
                    test_seed,
                    selector,
                    learned_alpha,
                    variable,
                    order[variable],
                    result.branches,
                    result.satisfiable,
                    result.reuse_us,
                    result.fresh_us,
                    result.fresh_us as f64 / result.reuse_us.max(1) as f64,
                    result.reuse_nodes,
                    result.fresh_nodes,
                    result.reuse_nodes as f64 / result.fresh_nodes.max(1) as f64,
                    result.valid
                );
            }
            println!(
                "{},{},{},{},{},{:.2},{},{},{},{},{},{},{:.6},{},{},{:.6},{}",
                family,
                vars,
                mapped.len(),
                test_seed,
                "oracle",
                learned_alpha,
                oracle_variable,
                order[oracle_variable],
                oracle_result.branches,
                oracle_result.satisfiable,
                oracle_result.reuse_us,
                oracle_result.fresh_us,
                oracle_result.fresh_us as f64 / oracle_result.reuse_us.max(1) as f64,
                oracle_result.reuse_nodes,
                oracle_result.fresh_nodes,
                oracle_result.reuse_nodes as f64 / oracle_result.fresh_nodes.max(1) as f64,
                oracle_result.valid
            );
        }
        return;
    }
    for seed in 1..=trials {
        let formula = generate_formula(family, vars, ratio, seed as u64);
        let order = choose_order(order_name, vars, &formula, seed as u64);
        if engine == "branch-reuse" {
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(variable, sign)| (rank[variable], sign))
                            .collect(),
                    )
                })
                .collect();
            let natural: Vec<_> = (0..vars).collect();
            for branch_percent in [0usize, 25, 50, 75, 90] {
                let branch_level = (vars - 1) * branch_percent / 100;
                let branch_variable = order[branch_level];
                let mut false_formula = mapped.clone();
                false_formula.push(Clause(vec![(branch_level, false)]));
                let branch_clause = false_formula.len() - 1;
                let true_clause = Clause(vec![(branch_level, true)]);
                let mut true_formula = false_formula.clone();
                true_formula[branch_clause] = true_clause.clone();

                let cache_start = Instant::now();
                let mut cache = build_incremental_bdd_cache_with_stride(vars, &false_formula, 4);
                let cache_false_us = cache_start.elapsed().as_micros();
                let checkpoint_factors: usize = cache.checkpoints.iter().map(Vec::len).sum();
                let false_sat = cache.assignment.is_some();
                let false_valid = cache
                    .assignment
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&false_formula, assignment));

                let incremental_start = Instant::now();
                let (true_assignment, new_nodes, _, _) =
                    incremental_clause_update(&mut cache, branch_clause, &true_clause);
                let incremental_true_us = incremental_start.elapsed().as_micros();
                let true_valid = true_assignment
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&true_formula, assignment));

                let fresh_false_start = Instant::now();
                let fresh_false = eliminate_with_bdds(vars, &false_formula, &natural);
                let fresh_false_us = fresh_false_start.elapsed().as_micros();
                assert_eq!(false_sat, fresh_false.assignment.is_some());
                let fresh_true_start = Instant::now();
                let fresh_true = eliminate_with_bdds(vars, &true_formula, &natural);
                let fresh_true_us = fresh_true_start.elapsed().as_micros();
                let true_equivalent = true_assignment.is_some() == fresh_true.assignment.is_some();
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{:.6},{},{},{}",
                    family,
                    vars,
                    mapped.len(),
                    seed,
                    branch_percent,
                    branch_level,
                    branch_variable,
                    4,
                    checkpoint_factors,
                    false_sat,
                    true_assignment.is_some(),
                    cache_false_us,
                    incremental_true_us,
                    fresh_false_us,
                    fresh_true_us,
                    (fresh_false_us + fresh_true_us) as f64
                        / (cache_false_us + incremental_true_us).max(1) as f64,
                    fresh_true_us as f64 / incremental_true_us.max(1) as f64,
                    new_nodes,
                    fresh_true.allocated_nodes,
                    new_nodes as f64 / fresh_true.allocated_nodes.max(1) as f64,
                    true_equivalent,
                    false_valid,
                    true_valid
                );
            }
            continue;
        }
        if engine == "checkpoint-compression" {
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let changed_clause = seed % mapped.len();
            let mut replacement = mapped[changed_clause].clone();
            replacement.0[0].1 = !replacement.0[0].1;
            let mut updated = mapped.clone();
            updated[changed_clause] = replacement.clone();
            let natural: Vec<_> = (0..vars).collect();
            let full_start = Instant::now();
            let full = eliminate_with_bdds(vars, &updated, &natural);
            let full_us = full_start.elapsed().as_micros();
            let dense = build_incremental_bdd_cache_with_stride(vars, &mapped, 1);
            let dense_factors: usize = dense.checkpoints.iter().map(Vec::len).sum();
            for stride in [1usize, 2, 4, 8, 16] {
                let cache_start = Instant::now();
                let mut cache = build_incremental_bdd_cache_with_stride(vars, &mapped, stride);
                let cache_build_us = cache_start.elapsed().as_micros();
                let checkpoint_factors: usize = cache.checkpoints.iter().map(Vec::len).sum();
                let update_start = Instant::now();
                let (assignment, new_nodes, earliest, restored) =
                    incremental_clause_update(&mut cache, changed_clause, &replacement);
                let incremental_us = update_start.elapsed().as_micros();
                let valid = assignment
                    .as_ref()
                    .is_none_or(|candidate| satisfies(&updated, candidate));
                println!(
                    "{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{}",
                    family,
                    vars,
                    mapped.len(),
                    seed,
                    stride,
                    cache.checkpoints.len(),
                    checkpoint_factors,
                    checkpoint_factors as f64 / dense_factors.max(1) as f64,
                    earliest,
                    restored,
                    earliest - restored,
                    vars - earliest,
                    cache_build_us,
                    incremental_us,
                    full_us,
                    full_us as f64 / incremental_us.max(1) as f64,
                    new_nodes,
                    full.allocated_nodes,
                    new_nodes as f64 / full.allocated_nodes.max(1) as f64,
                    assignment.is_some() == full.assignment.is_some(),
                    valid
                );
            }
            continue;
        }
        if engine == "incremental-pinch" {
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mapped: Vec<_> = formula
                .iter()
                .map(|clause| {
                    Clause(
                        clause
                            .0
                            .iter()
                            .map(|&(variable, sign)| (rank[variable], sign))
                            .collect(),
                    )
                })
                .collect();
            let changed_clause = seed % mapped.len();
            let mut replacement = mapped[changed_clause].clone();
            replacement.0[0].1 = !replacement.0[0].1;
            let mut updated_mapped = mapped.clone();
            updated_mapped[changed_clause] = replacement.clone();
            let mut updated_original = formula.clone();
            updated_original[changed_clause].0[0].1 = !updated_original[changed_clause].0[0].1;
            let cache_start = Instant::now();
            let mut cache = build_incremental_bdd_cache(vars, &mapped);
            let cache_build_us = cache_start.elapsed().as_micros();
            let checkpoint_factors: usize = cache.checkpoints.iter().map(Vec::len).sum();
            let incremental_start = Instant::now();
            let (mapped_assignment, new_nodes, earliest, _) =
                incremental_clause_update(&mut cache, changed_clause, &replacement);
            let incremental_us = incremental_start.elapsed().as_micros();
            let full_start = Instant::now();
            let natural_order: Vec<_> = (0..vars).collect();
            let full = eliminate_with_bdds(vars, &updated_mapped, &natural_order);
            let full_us = full_start.elapsed().as_micros();
            let incremental_assignment = mapped_assignment.map(|assignment| {
                let mut original = vec![false; vars];
                for level in 0..vars {
                    original[order[level]] = assignment[level];
                }
                original
            });
            let full_assignment = full.assignment.as_ref().map(|assignment| {
                let mut original = vec![false; vars];
                for level in 0..vars {
                    original[order[level]] = assignment[level];
                }
                original
            });
            let equivalent = incremental_assignment.is_some() == full_assignment.is_some();
            let incremental_valid = incremental_assignment
                .as_ref()
                .is_none_or(|assignment| satisfies(&updated_original, assignment));
            let full_valid = full_assignment
                .as_ref()
                .is_none_or(|assignment| satisfies(&updated_original, assignment));
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{},{}",
                family,
                vars,
                formula.len(),
                seed,
                changed_clause,
                earliest,
                earliest,
                vars - earliest,
                checkpoint_factors,
                cache_build_us,
                incremental_us,
                full_us,
                full_us as f64 / incremental_us.max(1) as f64,
                new_nodes,
                full.allocated_nodes,
                new_nodes as f64 / full.allocated_nodes.max(1) as f64,
                equivalent,
                incremental_valid,
                full_valid
            );
            continue;
        }
        if engine == "math-tricks" {
            let preprocess_start = Instant::now();
            let (processed, stats) = mathematical_identity_preprocess(&formula);
            let preprocess_us = preprocess_start.elapsed().as_micros();
            let baseline_start = Instant::now();
            let baseline = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            let baseline_us = baseline_start.elapsed().as_micros();
            let processed_order = min_fill_order(vars, &processed);
            let processed_start = Instant::now();
            let processed_result =
                eliminate_with_bdds_ordered(vars, &processed, &processed_order, &processed_order);
            let processed_us = processed_start.elapsed().as_micros();
            let equivalent = baseline.assignment.is_some() == processed_result.assignment.is_some();
            let baseline_valid = baseline
                .assignment
                .as_ref()
                .is_none_or(|assignment| satisfies(&formula, assignment));
            let processed_valid = processed_result
                .assignment
                .as_ref()
                .is_none_or(|assignment| {
                    satisfies(&processed, assignment) && satisfies(&formula, assignment)
                });
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{}",
                family,
                vars,
                formula.len(),
                seed,
                processed.len(),
                stats.tautologies,
                stats.subsumed,
                stats.consensus_pairs,
                stats.passes,
                preprocess_us,
                baseline_us,
                processed_us,
                baseline.allocated_nodes,
                processed_result.allocated_nodes,
                processed_result.allocated_nodes as f64 / baseline.allocated_nodes.max(1) as f64,
                equivalent,
                baseline_valid,
                processed_valid
            );
            continue;
        }
        if engine == "joint-predict" {
            let natural = eliminate_with_bdds(vars, &formula, &order);
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            for selector in ["graph", "semantic", "frequency"] {
                let start = Instant::now();
                let (
                    expanded_vars,
                    expanded,
                    accepted,
                    recursive,
                    scored,
                    initial_width,
                    final_width,
                    work_ratio,
                ) = if selector == "graph" {
                    let predicted = predicted_joint_expand(vars, &formula, helper_budget, 24);
                    let work_ratio = predicted.final_estimated_work
                        / predicted.initial_estimated_work.max(f64::MIN_POSITIVE);
                    (
                        predicted.vars,
                        predicted.clauses,
                        predicted.accepted,
                        predicted.recursive_accepted,
                        predicted.candidates_scored,
                        predicted.initial_width,
                        predicted.final_width,
                        work_ratio,
                    )
                } else if selector == "semantic" {
                    let (expanded_vars, expanded, accepted, scored) = semantic_batch_expand(
                        vars,
                        &formula,
                        helper_budget,
                        &aligned.interaction_candidates,
                    );
                    let initial_order = min_fill_order(vars, &formula);
                    let expanded_order = min_fill_order(expanded_vars, &expanded);
                    let initial = elimination_cost(vars, &formula, &initial_order);
                    let final_cost = elimination_cost(expanded_vars, &expanded, &expanded_order);
                    (
                        expanded_vars,
                        expanded,
                        accepted,
                        0,
                        scored,
                        initial.0,
                        final_cost.0,
                        final_cost.1 / initial.1.max(f64::MIN_POSITIVE),
                    )
                } else {
                    let (expanded_vars, expanded, accepted) =
                        expand_recurring_pairs(vars, &formula, helper_budget);
                    let initial_order = min_fill_order(vars, &formula);
                    let expanded_order = min_fill_order(expanded_vars, &expanded);
                    let initial = elimination_cost(vars, &formula, &initial_order);
                    let final_cost = elimination_cost(expanded_vars, &expanded, &expanded_order);
                    (
                        expanded_vars,
                        expanded,
                        accepted,
                        0,
                        accepted,
                        initial.0,
                        final_cost.0,
                        final_cost.1 / initial.1.max(f64::MIN_POSITIVE),
                    )
                };
                let predict_us = start.elapsed().as_micros();
                let expanded_order = min_fill_order(expanded_vars, &expanded);
                let start = Instant::now();
                let result = eliminate_with_bdds_ordered(
                    expanded_vars,
                    &expanded,
                    &expanded_order,
                    &expanded_order,
                );
                let final_us = start.elapsed().as_micros();
                let equivalent = result.assignment.is_some() == natural.assignment.is_some();
                let valid = result.assignment.as_ref().is_none_or(|assignment| {
                    satisfies(&formula, &assignment[..vars]) && satisfies(&expanded, assignment)
                });
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{},{},{:.6},{},{}",
                    family,
                    vars,
                    formula.len(),
                    seed,
                    selector,
                    helper_budget,
                    accepted,
                    recursive,
                    scored,
                    natural.allocated_nodes,
                    aligned.allocated_nodes,
                    result.allocated_nodes,
                    result.allocated_nodes as f64 / natural.allocated_nodes.max(1) as f64,
                    result.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64,
                    predict_us,
                    final_us,
                    initial_width,
                    final_width,
                    work_ratio,
                    equivalent,
                    valid
                );
            }
            continue;
        }
        if engine == "flower-ring-states" {
            assert!(
                family == "flower" || family == "flower-planted" || family == "flower-symmetric",
                "flower-ring-states requires a flat flower family"
            );
            for (direction, outside_in) in [("inside-out", false), ("outside-in", true)] {
                let (total_nodes, profile) = flower_ring_state_profile(vars, &formula, outside_in);
                for point in profile {
                    println!(
                        "{},{},{},{},{},{},{},{},{},{}",
                        family,
                        vars,
                        formula.len(),
                        seed,
                        direction,
                        point.ring,
                        point.processed,
                        point.residual_states,
                        point.residual_states.div_ceil(6),
                        total_nodes
                    );
                }
            }
            continue;
        }
        if engine == "bdd-sift" || engine == "bdd-sift-control" {
            let natural: Vec<_> = (0..vars).collect();
            for (start_name, initial) in [("natural", natural), ("elimination", order.clone())] {
                let start = Instant::now();
                let baseline = eliminate_with_bdds_ordered(vars, &formula, &order, &initial);
                let baseline_us = start.elapsed().as_micros().max(1);
                let start = Instant::now();
                let sifted = sift_bdd_order(
                    vars,
                    &formula,
                    &order,
                    &initial,
                    sift_passes,
                    sift_trials,
                    engine == "bdd-sift",
                    seed as u64,
                );
                let search_us = start.elapsed().as_micros();
                let start = Instant::now();
                let final_check =
                    eliminate_with_bdds_ordered(vars, &formula, &order, &sifted.order);
                let final_us = start.elapsed().as_micros();
                assert_eq!(final_check.allocated_nodes, sifted.result.allocated_nodes);
                let valid = final_check
                    .assignment
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&formula, assignment));
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{},{:.3},{}",
                    family,
                    order_name,
                    start_name,
                    vars,
                    formula.len(),
                    seed,
                    sifted.passes,
                    sifted.swaps_tested,
                    sifted.swaps_accepted,
                    baseline.allocated_nodes,
                    sifted.result.allocated_nodes,
                    sifted.result.allocated_nodes as f64 / baseline.allocated_nodes.max(1) as f64,
                    baseline.live_nodes,
                    sifted.result.live_nodes,
                    sifted.result.live_nodes as f64 / baseline.live_nodes.max(1) as f64,
                    search_us,
                    final_us,
                    (search_us + final_us) as f64 / baseline_us as f64,
                    valid
                );
            }
            continue;
        }
        if engine == "bdd-order-sweep" {
            let natural: Vec<_> = (0..vars).collect();
            let mut reverse_elimination = order.clone();
            reverse_elimination.reverse();
            let configurations = [
                ("natural", natural),
                ("elimination", order.clone()),
                ("reverse-elimination", reverse_elimination),
                ("frequent-first", occurrence_order(vars, &formula, true)),
                ("rare-first", occurrence_order(vars, &formula, false)),
            ];
            for (bdd_order_name, bdd_order) in configurations {
                let start = Instant::now();
                let result = eliminate_with_bdds_ordered(vars, &formula, &order, &bdd_order);
                let elapsed = start.elapsed().as_micros();
                let valid = result
                    .assignment
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&formula, assignment));
                println!(
                    "{},{},{},{},{},{},{},{},{},{}",
                    family,
                    order_name,
                    bdd_order_name,
                    vars,
                    formula.len(),
                    seed,
                    result.assignment.is_some(),
                    elapsed,
                    result.allocated_nodes,
                    valid
                );
            }
            continue;
        }
        if engine == "music-order-sweep" {
            let aligned = eliminate_with_bdds_ordered(vars, &formula, &order, &order);
            let configurations = vec![
                ("aligned-melody", order.clone()),
                ("rhythm-2", metrical_order(&order, 2)),
                ("rhythm-3", metrical_order(&order, 3)),
                ("rhythm-4", metrical_order(&order, 4)),
                ("rhythm-5", metrical_order(&order, 5)),
                ("rhythm-7", metrical_order(&order, 7)),
                ("phrasing-4", phrase_order(&order, 4)),
                ("phrasing-8", phrase_order(&order, 8)),
                ("counterpoint", counterpoint_order(&order)),
                ("motif-polarity-degree", motif_order(vars, &formula, &order)),
                (
                    "dynamics-crescendo",
                    occurrence_order(vars, &formula, false),
                ),
                (
                    "dynamics-decrescendo",
                    occurrence_order(vars, &formula, true),
                ),
                (
                    "harmony-voice-leading",
                    harmonic_order(vars, &formula, &order),
                ),
                (
                    "tension-then-resolution",
                    tension_order(vars, &formula, true),
                ),
                (
                    "resolution-then-tension",
                    tension_order(vars, &formula, false),
                ),
            ];
            for (name, bdd_order) in configurations {
                let start = Instant::now();
                let result = eliminate_with_bdds_ordered(vars, &formula, &order, &bdd_order);
                let elapsed = start.elapsed().as_micros();
                let valid = result
                    .assignment
                    .as_ref()
                    .is_none_or(|a| satisfies(&formula, a));
                println!(
                    "{},{},{},{},{},{},{},{},{},{:.6},{}",
                    family,
                    order_name,
                    name,
                    vars,
                    formula.len(),
                    seed,
                    result.assignment.is_some(),
                    elapsed,
                    result.allocated_nodes,
                    result.allocated_nodes as f64 / aligned.allocated_nodes.max(1) as f64,
                    valid
                );
            }
            continue;
        }
        if engine == "bdd-frontier-expand" {
            let start = Instant::now();
            let original = eliminate_with_bdds(vars, &formula, &order);
            let original_us = start.elapsed().as_micros();
            let start = Instant::now();
            let frontier =
                bdd_frontier_expand(vars, &formula, helper_budget, 24, order_name, seed as u64);
            let search_us = start.elapsed().as_micros();
            let final_order =
                choose_order(order_name, frontier.vars, &frontier.clauses, seed as u64);
            let start = Instant::now();
            let final_check = eliminate_with_bdds(frontier.vars, &frontier.clauses, &final_order);
            let final_us = start.elapsed().as_micros();
            assert_eq!(final_check.allocated_nodes, frontier.result.allocated_nodes);
            let equivalent = original.assignment.is_some() == frontier.result.assignment.is_some();
            let projected_valid = frontier
                .result
                .assignment
                .as_ref()
                .is_none_or(|assignment| {
                    satisfies(&formula, &assignment[..vars])
                        && satisfies(&frontier.clauses, assignment)
                });
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{}",
                family,
                order_name,
                vars,
                formula.len(),
                seed,
                helper_budget,
                frontier.accepted,
                frontier.recursive_accepted,
                frontier.candidates_tested,
                frontier.beneficial_trials,
                frontier.vars,
                frontier.clauses.len(),
                original.allocated_nodes,
                frontier.result.allocated_nodes,
                frontier.result.allocated_nodes as f64 / original.allocated_nodes.max(1) as f64,
                original_us,
                final_us,
                search_us,
                equivalent,
                projected_valid
            );
            continue;
        }
        if engine == "feedback-expand-bdd" || engine == "shortlist-expand-bdd" {
            let start = Instant::now();
            let original = eliminate_with_bdds(vars, &formula, &order);
            let original_us = start.elapsed().as_micros();
            let start = Instant::now();
            let feedback = feedback_expand(
                vars,
                &formula,
                helper_budget,
                24,
                6,
                order_name,
                seed as u64,
                engine == "feedback-expand-bdd",
            );
            let search_us = start.elapsed().as_micros();
            let final_order =
                choose_order(order_name, feedback.vars, &feedback.clauses, seed as u64);
            let start = Instant::now();
            let final_check = eliminate_with_bdds(feedback.vars, &feedback.clauses, &final_order);
            let final_us = start.elapsed().as_micros();
            assert_eq!(final_check.allocated_nodes, feedback.result.allocated_nodes);
            let equivalent = original.assignment.is_some() == feedback.result.assignment.is_some();
            let projected_valid = feedback
                .result
                .assignment
                .as_ref()
                .is_none_or(|assignment| {
                    satisfies(&formula, &assignment[..vars])
                        && satisfies(&feedback.clauses, assignment)
                });
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{}",
                family,
                order_name,
                vars,
                formula.len(),
                seed,
                helper_budget,
                feedback.accepted,
                feedback.recursive_accepted,
                feedback.candidates_tested,
                feedback.beneficial_trials,
                feedback.vars,
                feedback.clauses.len(),
                original.allocated_nodes,
                feedback.result.allocated_nodes,
                feedback.result.allocated_nodes as f64 / original.allocated_nodes.max(1) as f64,
                original_us,
                final_us,
                search_us,
                equivalent,
                projected_valid
            );
            continue;
        }
        if engine == "greedy-expand-bdd" || engine == "aligned-greedy-expand" {
            let aligned = engine == "aligned-greedy-expand";
            let start = Instant::now();
            let original = solve_bdd_strategy(vars, &formula, order_name, seed as u64, aligned);
            let original_us = start.elapsed().as_micros();
            let start = Instant::now();
            let greedy = greedy_expand(
                vars,
                &formula,
                helper_budget,
                24,
                order_name,
                seed as u64,
                aligned,
            );
            let search_us = start.elapsed().as_micros();
            let start = Instant::now();
            let final_check = solve_bdd_strategy(
                greedy.vars,
                &greedy.clauses,
                order_name,
                seed as u64,
                aligned,
            );
            let final_us = start.elapsed().as_micros();
            assert_eq!(final_check.allocated_nodes, greedy.result.allocated_nodes);
            let equivalent = original.assignment.is_some() == greedy.result.assignment.is_some();
            let projected_valid = greedy.result.assignment.as_ref().is_none_or(|assignment| {
                satisfies(&formula, &assignment[..vars]) && satisfies(&greedy.clauses, assignment)
            });
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{}",
                family,
                order_name,
                vars,
                formula.len(),
                seed,
                helper_budget,
                greedy.accepted,
                greedy.recursive_accepted,
                greedy.candidates_tested,
                greedy.beneficial_trials,
                greedy.vars,
                greedy.clauses.len(),
                original.allocated_nodes,
                greedy.result.allocated_nodes,
                greedy.result.allocated_nodes as f64 / original.allocated_nodes.max(1) as f64,
                original_us,
                final_us,
                search_us,
                equivalent,
                projected_valid
            );
            continue;
        }
        if engine == "expand-bdd" {
            let start = Instant::now();
            let original = eliminate_with_bdds(vars, &formula, &order);
            let original_us = start.elapsed().as_micros();
            let (expanded_vars, expanded, helpers) =
                expand_recurring_pairs(vars, &formula, helper_budget);
            let expanded_order = choose_order(order_name, expanded_vars, &expanded, seed as u64);
            let start = Instant::now();
            let expanded_result = eliminate_with_bdds(expanded_vars, &expanded, &expanded_order);
            let expanded_us = start.elapsed().as_micros();
            let equivalent = original.assignment.is_some() == expanded_result.assignment.is_some();
            let projected_valid = expanded_result
                .assignment
                .as_ref()
                .is_none_or(|assignment| {
                    satisfies(&formula, &assignment[..vars]) && satisfies(&expanded, assignment)
                });
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{}",
                family,
                order_name,
                vars,
                formula.len(),
                seed,
                helpers,
                expanded_vars,
                expanded.len(),
                original_us,
                expanded_us,
                original.allocated_nodes,
                expanded_result.allocated_nodes,
                expanded_result.allocated_nodes as f64 / original.allocated_nodes.max(1) as f64,
                equivalent,
                projected_valid
            );
            continue;
        }
        if engine == "bdd-only" {
            let start = Instant::now();
            let result = eliminate_with_bdds(vars, &formula, &order);
            let elapsed = start.elapsed().as_micros();
            let valid = result
                .assignment
                .as_ref()
                .is_none_or(|a| satisfies(&formula, a));
            println!(
                "{},{},{},{},{},{},{},{},{}",
                family,
                order_name,
                vars,
                formula.len(),
                seed,
                result.assignment.is_some(),
                elapsed,
                result.allocated_nodes,
                valid
            );
            continue;
        }
        let start = Instant::now();
        let result = eliminate(vars, &formula, &order);
        let layered_us = start.elapsed().as_micros();
        let start = Instant::now();
        let bdd_result = eliminate_with_bdds(vars, &formula, &order);
        let bdd_solver_us = start.elapsed().as_micros();
        let bdd_valid = bdd_result
            .assignment
            .as_ref()
            .is_none_or(|a| satisfies(&formula, a));
        let bdd_agrees =
            bdd_result.assignment.is_some() == result.assignment.is_some() && bdd_valid;
        let start = Instant::now();
        let brute = (vars <= 24).then(|| brute_force(vars, &formula));
        let brute_us = start.elapsed().as_micros();
        let valid = result
            .assignment
            .as_ref()
            .is_none_or(|a| satisfies(&formula, a));
        let agrees = match brute {
            Some(brute) if result.assignment.is_some() == brute.is_some() && valid => "true",
            Some(_) => "false",
            None if valid => "unchecked",
            None => "invalid-witness",
        };
        *histogram.entry(result.peak_boundary).or_insert(0usize) += 1;
        let stored_entries: usize = result.layers.iter().map(|l| l.witness.len()).sum();
        let stored_bdd_nodes: usize = result.layers.iter().map(|l| l.bdd_nodes).sum();
        let bdd_ratio = stored_bdd_nodes as f64 / stored_entries as f64;
        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{}",
            family,
            order_name,
            vars,
            formula.len(),
            seed,
            result.assignment.is_some(),
            result.peak_boundary,
            result.peak_entries,
            stored_entries,
            result.peak_bdd_nodes,
            stored_bdd_nodes,
            bdd_ratio,
            layered_us,
            bdd_solver_us,
            bdd_result.allocated_nodes,
            bdd_agrees,
            brute_us,
            agrees
        );
    }
    if engine == "compare" {
        eprintln!("peak-boundary histogram: {histogram:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layered_solver_matches_brute_force() {
        for vars in 3..=10 {
            for seed in 1..=40 {
                let clauses = random_3sat(vars, vars * 4, seed);
                let result = eliminate(vars, &clauses, &min_degree_order(vars, &clauses));
                let brute = brute_force(vars, &clauses);
                assert_eq!(
                    result.assignment.is_some(),
                    brute.is_some(),
                    "vars={vars}, seed={seed}"
                );
                assert!(
                    result
                        .assignment
                        .as_ref()
                        .is_none_or(|a| satisfies(&clauses, a))
                );
            }
        }
    }

    #[test]
    fn reduced_bdd_merges_repeated_subfunctions() {
        assert_eq!(reduced_bdd_nodes(&[false, true, true, false], 2), 3); // xor
        assert_eq!(reduced_bdd_nodes(&[false, true, false, true], 2), 1); // x
        assert_eq!(reduced_bdd_nodes(&[true; 8], 3), 0); // constant
    }

    #[test]
    fn native_bdd_solver_matches_brute_force() {
        for vars in 3..=10 {
            for seed in 1..=40 {
                let clauses = random_3sat(vars, vars * 4, seed);
                let order = min_fill_order(vars, &clauses);
                let bdd = eliminate_with_bdds(vars, &clauses, &order);
                let brute = brute_force(vars, &clauses);
                assert_eq!(bdd.assignment.is_some(), brute.is_some());
                assert!(
                    bdd.assignment
                        .as_ref()
                        .is_none_or(|a| satisfies(&clauses, a))
                );
            }
        }
    }

    #[test]
    fn pair_expansion_preserves_sat_and_witnesses() {
        for vars in 4..=10 {
            for seed in 1..=30 {
                let clauses = random_3sat(vars, vars * 4, seed);
                let (expanded_vars, expanded, _) = expand_recurring_pairs(vars, &clauses, vars / 2);
                let original = brute_force(vars, &clauses);
                let result = eliminate_with_bdds(
                    expanded_vars,
                    &expanded,
                    &min_fill_order(expanded_vars, &expanded),
                );
                assert_eq!(original.is_some(), result.assignment.is_some());
                assert!(result.assignment.as_ref().is_none_or(|assignment| {
                    satisfies(&clauses, &assignment[..vars]) && satisfies(&expanded, assignment)
                }));
            }
        }
    }

    #[test]
    fn greedy_expansion_is_monotonic_and_equivalent() {
        for seed in 1..=20 {
            let vars = 10;
            let clauses = random_3sat(vars, vars * 6, seed);
            let baseline = eliminate_with_bdds(vars, &clauses, &min_fill_order(vars, &clauses));
            let greedy = greedy_expand(vars, &clauses, 3, 12, "min-fill", seed, false);
            assert!(greedy.result.allocated_nodes <= baseline.allocated_nodes);
            assert_eq!(
                baseline.assignment.is_some(),
                greedy.result.assignment.is_some()
            );
            assert!(greedy.result.assignment.as_ref().is_none_or(|assignment| {
                satisfies(&clauses, &assignment[..vars]) && satisfies(&greedy.clauses, assignment)
            }));
        }
    }

    #[test]
    fn feedback_expansion_is_monotonic_and_equivalent() {
        for seed in 1..=20 {
            let vars = 10;
            let clauses = random_3sat(vars, vars * 6, seed);
            let baseline = eliminate_with_bdds(vars, &clauses, &min_fill_order(vars, &clauses));
            let feedback = feedback_expand(vars, &clauses, 3, 12, 4, "min-fill", seed, true);
            assert!(feedback.result.allocated_nodes <= baseline.allocated_nodes);
            assert_eq!(
                baseline.assignment.is_some(),
                feedback.result.assignment.is_some()
            );
            assert!(
                feedback
                    .result
                    .assignment
                    .as_ref()
                    .is_none_or(|assignment| {
                        satisfies(&clauses, &assignment[..vars])
                            && satisfies(&feedback.clauses, assignment)
                    })
            );
        }
    }

    #[test]
    fn bdd_frontier_expansion_is_monotonic_and_equivalent() {
        for seed in 1..=15 {
            let vars = 10;
            let clauses = random_3sat(vars, vars * 6, seed);
            let baseline = eliminate_with_bdds(vars, &clauses, &min_fill_order(vars, &clauses));
            let frontier = bdd_frontier_expand(vars, &clauses, 3, 12, "min-fill", seed);
            assert!(frontier.result.allocated_nodes <= baseline.allocated_nodes);
            assert_eq!(
                baseline.assignment.is_some(),
                frontier.result.assignment.is_some()
            );
            assert!(
                frontier
                    .result
                    .assignment
                    .as_ref()
                    .is_none_or(|assignment| {
                        satisfies(&clauses, &assignment[..vars])
                            && satisfies(&frontier.clauses, assignment)
                    })
            );
        }
    }

    #[test]
    fn bdd_variable_order_preserves_answers_and_witnesses() {
        for seed in 1..=25 {
            let vars = 10;
            let clauses = random_3sat(vars, vars * 4, seed);
            let elimination = min_fill_order(vars, &clauses);
            let mut reversed = elimination.clone();
            reversed.reverse();
            for bdd_order in [elimination.clone(), reversed] {
                let result = eliminate_with_bdds_ordered(vars, &clauses, &elimination, &bdd_order);
                let brute = brute_force(vars, &clauses);
                assert_eq!(result.assignment.is_some(), brute.is_some());
                assert!(
                    result
                        .assignment
                        .as_ref()
                        .is_none_or(|assignment| satisfies(&clauses, assignment))
                );
            }
        }
    }

    #[test]
    fn bdd_sifting_is_monotonic_and_preserves_witnesses() {
        for seed in 1..=15 {
            let vars = 10;
            let clauses = random_3sat(vars, vars * 5, seed);
            let elimination = min_fill_order(vars, &clauses);
            let natural: Vec<_> = (0..vars).collect();
            let baseline = eliminate_with_bdds_ordered(vars, &clauses, &elimination, &natural);
            let sifted = sift_bdd_order(
                vars,
                &clauses,
                &elimination,
                &natural,
                3,
                usize::MAX,
                true,
                seed,
            );
            assert!(sifted.result.allocated_nodes <= baseline.allocated_nodes);
            assert_eq!(
                sifted.result.assignment.is_some(),
                brute_force(vars, &clauses).is_some()
            );
            assert!(
                sifted
                    .result
                    .assignment
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&clauses, assignment))
            );
        }
    }

    #[test]
    fn flower_geometries_match_brute_force() {
        for vars in 5..=10 {
            for seed in 1..=15 {
                for clauses in [
                    flower_3sat(vars, vars * 4, seed),
                    stacked_flower_3sat(vars, vars * 4, seed),
                ] {
                    let order = min_fill_order(vars, &clauses);
                    let result = eliminate_with_bdds(vars, &clauses, &order);
                    assert_eq!(
                        result.assignment.is_some(),
                        brute_force(vars, &clauses).is_some()
                    );
                    assert!(
                        result
                            .assignment
                            .as_ref()
                            .is_none_or(|assignment| satisfies(&clauses, assignment))
                    );
                }
            }
        }
    }

    #[test]
    fn planted_flowers_are_satisfiable_and_have_ring_profiles() {
        for vars in [7, 19, 25] {
            for seed in 1..=10 {
                let clauses = planted_flower_3sat(vars, vars * 4, seed);
                let planted = planted_assignment(vars, seed);
                assert!(satisfies(&clauses, &planted));
                for outside_in in [false, true] {
                    let (_, profile) = flower_ring_state_profile(vars, &clauses, outside_in);
                    assert!(!profile.is_empty());
                    assert_eq!(profile.last().unwrap().processed, vars);
                    assert!((1..=2).contains(&profile.last().unwrap().residual_states));
                }
            }
        }
    }

    #[test]
    fn symmetric_flower_is_rotation_invariant_and_satisfiable() {
        for vars in [7, 19, 37] {
            let clauses = symmetric_flower_3sat(vars, vars * 4);
            assert!(satisfies(&clauses, &vec![true; vars]));
            let coordinates = flower_coordinates(vars);
            let index: HashMap<_, _> = coordinates
                .iter()
                .copied()
                .enumerate()
                .map(|(variable, coordinate)| (coordinate, variable))
                .collect();
            let original: BTreeSet<_> = clauses.iter().map(|clause| clause.0.clone()).collect();
            let rotated: BTreeSet<_> = clauses
                .iter()
                .map(|clause| {
                    let mut literals: Vec<_> = clause
                        .0
                        .iter()
                        .map(|&(variable, sign)| {
                            let (q, r) = coordinates[variable];
                            (index[&(-r, q + r)], sign)
                        })
                        .collect();
                    literals.sort_unstable();
                    literals
                })
                .collect();
            assert_eq!(original, rotated);
        }
    }

    #[test]
    fn joint_predictor_improves_its_proxy_and_preserves_answers() {
        for seed in 1..=20 {
            let vars = 10;
            let clauses = random_3sat(vars, vars * 6, seed);
            let predicted = predicted_joint_expand(vars, &clauses, 4, 16);
            assert!(
                predicted.final_width < predicted.initial_width
                    || (predicted.final_width == predicted.initial_width
                        && predicted.final_estimated_work <= predicted.initial_estimated_work)
            );
            let order = min_fill_order(predicted.vars, &predicted.clauses);
            let result =
                eliminate_with_bdds_ordered(predicted.vars, &predicted.clauses, &order, &order);
            assert_eq!(
                result.assignment.is_some(),
                brute_force(vars, &clauses).is_some()
            );
            assert!(result.assignment.as_ref().is_none_or(|assignment| {
                satisfies(&clauses, &assignment[..vars])
                    && satisfies(&predicted.clauses, assignment)
            }));
        }
    }

    #[test]
    fn mathematical_identity_preprocessing_is_equivalent() {
        for seed in 1..=20 {
            for clauses in [random_3sat(8, 32, seed), identity_expanded_sat(8, 4, seed)] {
                let (processed, _) = mathematical_identity_preprocess(&clauses);
                for bits in 0..(1usize << 8) {
                    let assignment: Vec<_> = (0..8)
                        .map(|variable| ((bits >> variable) & 1) == 1)
                        .collect();
                    assert_eq!(
                        satisfies(&clauses, &assignment),
                        satisfies(&processed, &assignment)
                    );
                }
            }
        }
    }

    #[test]
    fn incremental_clause_updates_match_full_resolves() {
        for seed in 1..=20 {
            let vars = 10;
            let clauses = random_3sat(vars, 40, seed);
            let order = min_fill_order(vars, &clauses);
            let mut rank = vec![0usize; vars];
            for (level, &variable) in order.iter().enumerate() {
                rank[variable] = level;
            }
            let mut mapped: Vec<_> = clauses
                .iter()
                .map(|clause| Clause(clause.0.iter().map(|&(v, s)| (rank[v], s)).collect()))
                .collect();
            let changed = seed as usize % mapped.len();
            let mut replacement = mapped[changed].clone();
            replacement.0[0].1 = !replacement.0[0].1;
            let mut cache = build_incremental_bdd_cache(vars, &mapped);
            let (incremental, _, _, _) =
                incremental_clause_update(&mut cache, changed, &replacement);
            mapped[changed] = replacement;
            let natural: Vec<_> = (0..vars).collect();
            let full = eliminate_with_bdds(vars, &mapped, &natural);
            assert_eq!(incremental.is_some(), full.assignment.is_some());
            assert!(
                incremental
                    .as_ref()
                    .is_none_or(|assignment| satisfies(&mapped, assignment))
            );
        }
    }

    #[test]
    fn exact_treewidth_matches_path_and_clique() {
        let path = vec![
            Clause(vec![(0, true), (1, true)]),
            Clause(vec![(1, true), (2, true)]),
            Clause(vec![(2, true), (3, true)]),
        ];
        assert_eq!(exact_treewidth(4, &path), 1);
        assert_eq!(
            exact_weighted_treewidth(&[1, 1, 1, 1], &primal_graph(4, &path)),
            1
        );
        assert_eq!(greedy_clique_lower_bound(4, &path), 1);
        assert_eq!(minor_min_width_lower_bound(4, &path), 1);
        let mut clique = Vec::new();
        for left in 0..4 {
            for right in left + 1..4 {
                clique.push(Clause(vec![(left, true), (right, true)]));
            }
        }
        assert_eq!(exact_treewidth(4, &clique), 3);
        assert_eq!(
            exact_weighted_treewidth(&[1, 1, 1, 1], &primal_graph(4, &clique)),
            3
        );
        assert_eq!(greedy_clique_lower_bound(4, &clique), 3);
        assert_eq!(minor_min_width_lower_bound(4, &clique), 3);
    }

    #[test]
    fn shaking_and_inverse_probing_preserve_sat_and_reconstruct_witnesses() {
        for seed in 1..=40 {
            let vars = 9;
            let clauses = random_3sat(vars, vars * 4, seed);
            let expected = brute_force(vars, &clauses).is_some();
            for inverse in [false, true] {
                let shaken = shake_formula(vars, &clauses, usize::from(inverse), vars);
                if shaken.contradiction {
                    assert!(!expected);
                    continue;
                }
                let core = brute_force(shaken.vars, &shaken.clauses);
                assert_eq!(core.is_some(), expected);
                if let Some(core_assignment) = core {
                    let mut reconstructed = vec![false; vars];
                    for (variable, value) in shaken.fixed.iter().enumerate() {
                        if let Some(value) = value {
                            reconstructed[variable] = *value;
                        }
                    }
                    for (core_variable, &original_variable) in
                        shaken.core_to_original.iter().enumerate()
                    {
                        reconstructed[original_variable] = core_assignment[core_variable];
                    }
                    assert!(satisfies(&clauses, &reconstructed));
                }
            }
        }
    }

    #[test]
    fn depth_two_probe_finds_a_forcing_invisible_to_unit_propagation() {
        let clauses = vec![
            Clause(vec![(0, true), (1, true), (2, true)]),
            Clause(vec![(0, true), (1, true), (2, false)]),
            Clause(vec![(0, true), (1, false), (2, true)]),
            Clause(vec![(0, true), (1, false), (2, false)]),
            Clause(vec![(0, false), (3, true), (4, true)]),
            Clause(vec![(0, false), (3, false), (4, true)]),
            Clause(vec![(0, false), (3, true), (4, false)]),
        ];
        let shallow = shake_formula(5, &clauses, 1, 5);
        let deep = shake_formula(5, &clauses, 2, 5);
        assert_eq!(shallow.inverse_forced, 0);
        assert_eq!(deep.fixed[0], Some(true));
        assert!(deep.inverse_forced >= 1);
        assert!(deep.clauses.is_empty());
    }

    #[test]
    fn global_articulation_discovery_finds_small_branches_once() {
        // Triangle 0-1-2 is the core; articulation 2 owns the path 3-4-5.
        let clauses = vec![
            Clause(vec![(0, true), (1, true)]),
            Clause(vec![(1, true), (2, true)]),
            Clause(vec![(2, true), (0, true)]),
            Clause(vec![(2, true), (3, true)]),
            Clause(vec![(3, true), (4, true)]),
            Clause(vec![(4, true), (5, true)]),
        ];
        let graph = compact_primal_graph(6, &clauses);
        let candidates = global_small_separator_candidates(&graph, 3);
        assert!(
            candidates
                .iter()
                .any(|(interior, boundary)| interior == &[3, 4, 5] && boundary == &[2])
        );
        for (interior, boundary) in candidates {
            let interior: BTreeSet<_> = interior.into_iter().collect();
            let actual: BTreeSet<_> = interior
                .iter()
                .flat_map(|&v| graph[v].iter().copied())
                .filter(|v| !interior.contains(v))
                .collect();
            assert_eq!(actual.into_iter().collect::<Vec<_>>(), boundary);
        }
    }

    #[test]
    fn global_discovery_finds_two_vertex_separator() {
        let clauses = vec![
            Clause(vec![(0, true), (1, true)]),
            Clause(vec![(1, true), (2, true)]),
            Clause(vec![(2, true), (0, true)]),
            Clause(vec![(1, true), (3, true)]),
            Clause(vec![(2, true), (3, true)]),
            Clause(vec![(3, true), (4, true)]),
        ];
        let graph = compact_primal_graph(5, &clauses);
        let candidates = global_small_separator_candidates(&graph, 2);
        assert!(candidates.iter().any(|(interior, boundary)| {
            interior.iter().copied().collect::<BTreeSet<_>>() == BTreeSet::from([3, 4])
                && boundary == &[1, 2]
        }));
    }

    #[test]
    fn seeded_branches_preserve_sat_and_regrow_witnesses() {
        for seed in 1..=30 {
            let vars = 9;
            let clauses = banded_3sat(vars, vars * 2, seed, 3);
            let seeded = seed_detachable_branch(vars, &clauses, 6);
            let expected = brute_force(vars, &clauses).is_some();
            let core = brute_force(seeded.vars, &seeded.clauses);
            assert_eq!(core.is_some(), expected);
            if let Some(core_assignment) = core {
                let mut reconstructed = vec![false; vars];
                for (core, &original) in seeded.core_to_original.iter().enumerate() {
                    reconstructed[original] = core_assignment[core];
                }
                let mut bits = 0usize;
                for (index, &variable) in seeded.boundary.iter().enumerate() {
                    if reconstructed[variable] {
                        bits |= 1usize << index;
                    }
                }
                if !seeded.interior.is_empty() {
                    let witness = seeded.witnesses[bits].as_ref().unwrap();
                    for (index, &variable) in seeded.interior.iter().enumerate() {
                        reconstructed[variable] = witness[index];
                    }
                }
                assert!(satisfies(&clauses, &reconstructed));
            }
        }
    }

    #[test]
    fn shared_bdd_seeds_preserve_sat_and_regrow_witnesses() {
        for seed in 1..=30 {
            let vars = 9;
            let clauses = banded_3sat(vars, vars * 2, seed, 3);
            for strategy in [
                "natural",
                "min-degree",
                "min-fill",
                "boundary-min-degree",
                "boundary-min-fill",
            ] {
                let seeded = seed_detachable_branch_bdd(vars, &clauses, 8, strategy);
                let expected = brute_force(vars, &clauses).is_some();
                let core = brute_force(seeded.vars, &seeded.clauses);
                assert_eq!(core.is_some(), expected);
                if let Some(core_assignment) = core {
                    let mut reconstructed = vec![false; vars];
                    for (core, &original) in seeded.core_to_original.iter().enumerate() {
                        reconstructed[original] = core_assignment[core];
                    }
                    let values = regrow_bdd_seed(&seeded, &reconstructed).unwrap();
                    for (index, &variable) in seeded.interior.iter().enumerate() {
                        reconstructed[variable] = values[index];
                    }
                    assert!(satisfies(&clauses, &reconstructed));
                }
            }
        }
    }

    #[test]
    fn multiple_small_seeds_regrow_in_reverse() {
        for seed in 1..=20 {
            let vars = 10;
            let clauses = banded_3sat(vars, vars * 2, seed, 3);
            let mut current_vars = vars;
            let mut current_clauses = clauses.clone();
            let mut seeds = Vec::new();
            for _ in 0..6 {
                let compiled =
                    seed_detachable_branch_bdd(current_vars, &current_clauses, 4, "natural");
                if compiled.interior.is_empty() {
                    break;
                }
                current_vars = compiled.vars;
                current_clauses = compiled.clauses.clone();
                seeds.push(compiled);
            }
            let expected = brute_force(vars, &clauses).is_some();
            let core = brute_force(current_vars, &current_clauses);
            assert_eq!(core.is_some(), expected);
            if let Some(core_assignment) = core {
                let reconstructed = regrow_seed_chain(&seeds, &core_assignment).unwrap();
                assert!(satisfies(&clauses, &reconstructed));
            }
        }
    }

    #[test]
    fn bounded_seed_compiler_rejects_without_using_partial_summary() {
        let vars = 9;
        let clauses: Vec<_> = (0..8)
            .map(|variable| {
                Clause(vec![
                    (variable, true),
                    ((variable + 1) % 8, false),
                    (8, variable % 2 == 0),
                ])
            })
            .collect();
        let rejected = try_seed_bdd_candidate(
            vars,
            &clauses,
            (0..8).collect(),
            vec![8],
            "min-fill",
            1,
            std::time::Duration::from_secs(1),
        );
        assert!(rejected.seed.is_none());
        assert!(rejected.node_exceeded);

        let accepted = try_seed_bdd_candidate(
            vars,
            &clauses,
            (0..8).collect(),
            vec![8],
            "min-fill",
            100_000,
            std::time::Duration::from_secs(1),
        );
        assert!(accepted.seed.is_some());
    }

    #[test]
    fn compiled_artifact_round_trip_queries_and_reconstructs() {
        let vars = 24;
        let clauses = banded_3sat(vars, vars * 2, 44, 3);
        let artifact = compile_safe_artifact(vars, &clauses, 16, 100_000, 1_000);
        let path = std::env::temp_dir().join(format!(
            "layered-sat-round-trip-{}-{}.lsat",
            std::process::id(),
            artifact.core_vars
        ));
        save_compiled_artifact(&path, &artifact).unwrap();
        let loaded = load_compiled_artifact(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(loaded.original_vars, vars);
        assert_eq!(loaded.core_vars, artifact.core_vars);
        let assumed = loaded.core_to_original[0];
        let assignment = query_compiled_artifact(&loaded, &[(assumed, true)])
            .unwrap()
            .unwrap();
        assert!(assignment[assumed]);
        assert!(satisfies(&clauses, &assignment));
        let removed = loaded.seeds[0].interior[0];
        let reopened = query_compiled_artifact(&loaded, &[(removed, assignment[removed])])
            .unwrap()
            .unwrap();
        assert_eq!(reopened[removed], assignment[removed]);
        assert!(satisfies(&clauses, &reopened));
    }

    #[test]
    fn solver_aware_gates_preserve_answers_and_witnesses() {
        let vars = 18;
        let clauses = banded_3sat(vars, vars * 2, 91, 3);
        for gate in ["all", "balanced", "strict"] {
            let artifact =
                compile_safe_artifact_with_gate(vars, &clauses, 16, 100_000, 1_000, gate);
            for variable in [0, vars / 2, vars - 1] {
                for value in [false, true] {
                    let expected = {
                        let mut constrained = clauses.clone();
                        constrained.push(Clause(vec![(variable, value)]));
                        brute_force(vars, &constrained).is_some()
                    };
                    let actual = query_compiled_artifact(&artifact, &[(variable, value)]).unwrap();
                    assert_eq!(actual.is_some(), expected, "gate={gate}");
                    if let Some(assignment) = actual {
                        assert!(satisfies(&clauses, &assignment));
                        assert_eq!(assignment[variable], value);
                    }
                }
            }
        }
    }

    #[test]
    fn continuation_repairs_match_full_recompilation() {
        let vars = 12;
        let base_formula = banded_3sat(vars, vars * 3, 707, 3);
        let order: Vec<_> = (0..vars).collect();
        let base = compile_continuation(&base_formula, &order);
        let changes = [
            (Clause(vec![(3, true), (7, false), (10, true)]), true),
            (base_formula[5].clone(), false),
        ];
        for (changed, insertion) in changes {
            let mut updated = base_formula.clone();
            if insertion {
                updated.push(changed.clone());
            } else {
                updated.remove(5);
            }
            let repaired = repair_continuation(&base, &changed, insertion);
            let full = compile_continuation(&updated, &order);
            let mut repaired_scratch = ContinuationScratch::new(&repaired);
            let mut full_scratch = ContinuationScratch::new(&full);
            for variable in 0..vars {
                for value in [false, true] {
                    let mut assumptions = vec![None; vars];
                    assumptions[variable] = Some(value);
                    let repaired_answer =
                        query_continuation(&repaired, &assumptions, &mut repaired_scratch);
                    let full_answer = query_continuation(&full, &assumptions, &mut full_scratch);
                    assert_eq!(repaired_answer.is_some(), full_answer.is_some());
                    if let Some(assignment) = repaired_answer {
                        assert!(satisfies(&updated, &assignment));
                        assert_eq!(assignment[variable], value);
                    }
                }
            }
        }
    }

    #[test]
    fn cumulative_continuation_repairs_preserve_prefix_residuals() {
        let vars = 12;
        let mut formula = banded_3sat(vars, vars * 3, 808, 3);
        let order: Vec<_> = (0..vars).collect();
        let base = compile_continuation(&formula, &order);
        let inserted = Clause(vec![(8, true), (9, false), (11, true)]);
        formula.push(inserted.clone());
        let after_insert = repair_continuation(&base, &inserted, true);
        let deleted_index = formula
            .iter()
            .position(|clause| clause.0.iter().any(|&(variable, _)| variable < 8))
            .unwrap();
        let deleted = formula.remove(deleted_index);
        let cumulative = repair_continuation(&after_insert, &deleted, false);
        let full = compile_continuation(&formula, &order);
        let mut cumulative_scratch = ContinuationScratch::new(&cumulative);
        let mut full_scratch = ContinuationScratch::new(&full);
        for variable in 0..vars {
            for value in [false, true] {
                let mut assumptions = vec![None; vars];
                assumptions[variable] = Some(value);
                let cumulative_answer =
                    query_continuation(&cumulative, &assumptions, &mut cumulative_scratch);
                let full_answer = query_continuation(&full, &assumptions, &mut full_scratch);
                assert_eq!(cumulative_answer.is_some(), full_answer.is_some());
                if let Some(assignment) = cumulative_answer {
                    assert!(satisfies(&formula, &assignment));
                }
            }
        }
    }

    #[test]
    fn temporal_memory_queries_preserve_state_across_horizon() {
        let width = 3;
        let horizon = 5;
        let (vars, formula) = temporal_memory_formula(width, horizon);
        let order: Vec<_> = (0..vars).collect();
        let compiled = compile_continuation(&formula, &order);
        let summarized = compile_temporal_memory_continuation(width, horizon);
        let mut scratch = ContinuationScratch::new(&compiled);
        let mut summarized_scratch = ContinuationScratch::new(&summarized);

        let mut consistent = vec![None; vars];
        consistent[1] = Some(true);
        consistent[horizon * width + 1] = Some(true);
        let assignment = query_continuation(&compiled, &consistent, &mut scratch).unwrap();
        let summarized_assignment =
            query_continuation(&summarized, &consistent, &mut summarized_scratch).unwrap();
        let kernel_assignment = query_temporal_memory_kernel(width, horizon, &consistent).unwrap();
        assert!(satisfies(&formula, &assignment));
        assert!(satisfies(&formula, &summarized_assignment));
        assert!(satisfies(&formula, &kernel_assignment));
        assert!(assignment[1]);
        assert!(assignment[horizon * width + 1]);

        let mut conflicting = consistent;
        conflicting[horizon * width + 1] = Some(false);
        assert!(query_continuation(&compiled, &conflicting, &mut scratch).is_none());
        assert!(query_continuation(&summarized, &conflicting, &mut summarized_scratch).is_none());
        assert!(query_temporal_memory_kernel(width, horizon, &conflicting).is_none());
    }

    #[test]
    fn temporal_kernel_matches_generic_continuations_on_small_grid() {
        let mut rng = Rng(0x5eed_2024);
        for width in 1..=4 {
            for horizon in 1..=4 {
                let (vars, formula) = temporal_memory_formula(width, horizon);
                let order: Vec<_> = (0..vars).collect();
                let generic = compile_continuation(&formula, &order);
                let mut scratch = ContinuationScratch::new(&generic);
                for _ in 0..100 {
                    let mut assumptions = vec![None; vars];
                    for _ in 0..4 {
                        let variable = rng.below(vars);
                        assumptions[variable] = Some(rng.next() & 1 == 1);
                    }
                    let generic_answer = query_continuation(&generic, &assumptions, &mut scratch);
                    let kernel_answer = query_temporal_memory_kernel(width, horizon, &assumptions);
                    assert_eq!(generic_answer.is_some(), kernel_answer.is_some());
                    if let Some(assignment) = kernel_answer {
                        assert!(satisfies(&formula, &assignment));
                        assert!(assumptions.iter().enumerate().all(|(variable, required)| {
                            required.is_none_or(|value| assignment[variable] == value)
                        }));
                    }
                }
            }
        }
    }

    #[test]
    fn temporal_vocabulary_is_recognized_from_cnf_and_matches_varisat() {
        let mut rng = Rng(0xdec0_de24);
        for kind in ["copy", "negate", "permute", "xor", "circuit"] {
            let width = 4;
            let horizon = 7;
            let (vars, formula) = temporal_vocabulary_formula(kind, width, horizon).unwrap();
            let kernel = RecognizedTemporalKernel::recognize(&formula, width, horizon).unwrap();
            for _ in 0..100 {
                let mut assumptions = vec![None; vars];
                for _ in 0..5 {
                    assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                }
                let kernel_answer = kernel.query(&assumptions);
                let mut constrained = formula.clone();
                constrained.extend(assumptions.iter().enumerate().filter_map(
                    |(variable, value)| value.map(|value| Clause(vec![(variable, value)])),
                ));
                let varisat_answer = solve_with_varisat(vars, &constrained);
                assert_eq!(kernel_answer.is_some(), varisat_answer.is_some(), "{kind}");
                if let Some(assignment) = kernel_answer {
                    assert!(satisfies(&formula, &assignment), "{kind}");
                    assert!(assumptions.iter().enumerate().all(|(variable, required)| {
                        required.is_none_or(|value| assignment[variable] == value)
                    }));
                }
            }
        }
    }

    #[test]
    fn temporal_recognizer_rejects_a_changed_transition_template() {
        let width = 4;
        let horizon = 3;
        let (_, mut formula) = temporal_vocabulary_formula("copy", width, horizon).unwrap();
        let second_step_clause = 2 * width;
        formula[second_step_clause].0[0].1 = !formula[second_step_clause].0[0].1;
        assert!(RecognizedTemporalKernel::recognize(&formula, width, horizon).is_err());
    }

    #[test]
    fn exact_composition_recognizer_handles_functions_outside_fixed_vocabulary() {
        let mut rng = Rng(0xc0de_0517);
        for kind in ["majority3", "mux3", "mixed3", "cascade4"] {
            let width = 4;
            let horizon = 6;
            let (vars, formula) = temporal_composition_formula(kind, width, horizon).unwrap();
            assert!(RecognizedTemporalKernel::recognize(&formula, width, horizon).is_err());
            let kernel =
                RecognizedTemporalKernel::recognize_exact_composition(&formula, width, horizon)
                    .unwrap();
            for _ in 0..50 {
                let mut assumptions = vec![None; vars];
                for _ in 0..5 {
                    assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                }
                let kernel_answer = kernel.query(&assumptions);
                let mut constrained = formula.clone();
                constrained.extend(assumptions.iter().enumerate().filter_map(
                    |(variable, value)| value.map(|value| Clause(vec![(variable, value)])),
                ));
                let varisat_answer = solve_with_varisat(vars, &constrained);
                assert_eq!(kernel_answer.is_some(), varisat_answer.is_some(), "{kind}");
                if let Some(assignment) = kernel_answer {
                    assert!(satisfies(&formula, &assignment), "{kind}");
                }
            }
        }
    }

    #[test]
    fn exact_composition_recognizer_rejects_nondeterminism() {
        let width = 4;
        let horizon = 3;
        let (_, mut formula) = temporal_composition_formula("majority3", width, horizon).unwrap();
        let unconstrained_outputs: BTreeSet<_> = (1..=horizon).map(|time| time * width).collect();
        formula.retain(|clause| {
            !clause
                .0
                .iter()
                .any(|(variable, _)| unconstrained_outputs.contains(variable))
        });
        assert!(
            RecognizedTemporalKernel::recognize_exact_composition(&formula, width, horizon)
                .is_err()
        );
    }

    #[test]
    fn local_composition_recognizer_matches_exact_without_fixed_vocabulary() {
        let mut rng = Rng(0x10ca_1c0e);
        for kind in ["majority3", "mux3", "mixed3", "cascade4"] {
            let width = 7;
            let horizon = 9;
            let (vars, formula) = temporal_composition_formula(kind, width, horizon).unwrap();
            let local =
                RecognizedTemporalKernel::recognize_local_composition(&formula, width, horizon)
                    .unwrap();
            let exact =
                RecognizedTemporalKernel::recognize_exact_composition(&formula, width, horizon)
                    .unwrap();
            assert_eq!(local.jumps[0], exact.jumps[0], "{kind}");
            for _ in 0..50 {
                let mut assumptions = vec![None; vars];
                for _ in 0..6 {
                    assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                }
                assert_eq!(
                    local.query(&assumptions).is_some(),
                    exact.query(&assumptions).is_some(),
                    "{kind}"
                );
                if let Some(assignment) = local.query(&assumptions) {
                    assert!(satisfies(&formula, &assignment), "{kind}");
                }
            }
        }
    }

    #[test]
    fn local_composition_recognizer_rejects_cross_output_clauses() {
        let width = 4;
        let horizon = 2;
        let (_, mut formula) = temporal_composition_formula("mixed3", width, horizon).unwrap();
        formula[0].0.push((width + 1, true));
        formula[0].0.sort_unstable();
        assert!(
            RecognizedTemporalKernel::recognize_local_composition(&formula, width, horizon)
                .is_err()
        );
    }

    #[test]
    fn symbolic_transition_replays_without_exponential_state_table() {
        let width = 24;
        let horizon = 40;
        let (vars, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let transition = SymbolicTemporalTransition::recognize(&formula, width, horizon).unwrap();
        assert_eq!(transition.representation_entries(), width * (4 + 16));
        let mut assumptions = vec![None; vars];
        for (bit, value) in assumptions.iter_mut().take(width).enumerate() {
            *value = Some(bit % 3 == 0);
        }
        let assignment = transition.query(&assumptions).unwrap();
        assert!(satisfies(&formula, &assignment));
    }

    #[test]
    fn symbolic_transition_matches_varisat_with_observations() {
        let width = 7;
        let horizon = 12;
        let (vars, formula) = temporal_composition_formula("mux3", width, horizon).unwrap();
        let transition = SymbolicTemporalTransition::recognize(&formula, width, horizon).unwrap();
        let mut rng = Rng(0x5a8b_011c);
        for _ in 0..50 {
            let mut assumptions = vec![None; vars];
            for value in assumptions.iter_mut().take(width) {
                *value = Some(rng.next() & 1 == 1);
            }
            for _ in 0..4 {
                assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
            }
            let symbolic = transition.query(&assumptions);
            let mut constrained = formula.clone();
            constrained.extend(
                assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Clause(vec![(variable, value)]))
                    }),
            );
            let varisat = solve_with_varisat(vars, &constrained);
            assert_eq!(symbolic.is_some(), varisat.is_some());
            if let Some(assignment) = symbolic {
                assert!(satisfies(&formula, &assignment));
            }
        }
    }

    #[test]
    fn symbolic_preimage_finds_partial_initial_state_and_witness() {
        let width = 6;
        let horizon = 5;
        let (vars, formula) = temporal_composition_formula("mixed3", width, horizon).unwrap();
        let mut preimage = SymbolicPreimageTransition::recognize_ordered(
            &formula, width, horizon, 100_000, "natural",
        )
        .unwrap();
        let mut rng = Rng(0xbdd5_ea2c);
        for _ in 0..40 {
            let mut assumptions = vec![None; vars];
            for _ in 0..8 {
                assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
            }
            let answer = preimage.query(&assumptions);
            let mut constrained = formula.clone();
            constrained.extend(
                assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Clause(vec![(variable, value)]))
                    }),
            );
            let varisat = solve_with_varisat(vars, &constrained);
            assert_eq!(answer.is_some(), varisat.is_some());
            if let Some(assignment) = answer {
                assert!(satisfies(&formula, &assignment));
                assert!(assumptions.iter().enumerate().all(|(variable, required)| {
                    required.is_none_or(|value| assignment[variable] == value)
                }));
            }
        }
    }

    #[test]
    fn symbolic_preimage_node_gate_rejects_growth_exactly() {
        let width = 10;
        let horizon = 20;
        let (_, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        assert!(
            SymbolicPreimageTransition::recognize_ordered(&formula, width, horizon, 10, "natural")
                .is_err()
        );
    }

    #[test]
    fn asymmetric_preimages_preserve_answers_across_fixed_orders() {
        let mut rng = Rng(0xa51e_77ac);
        for kind in ["hub3", "tree3", "irregular3"] {
            let width = 7;
            let horizon = 6;
            let (vars, formula) = temporal_composition_formula(kind, width, horizon).unwrap();
            for order in ["natural", "reverse", "evenodd", "dependency"] {
                let mut preimage = SymbolicPreimageTransition::recognize_ordered(
                    &formula, width, horizon, 200_000, order,
                )
                .unwrap();
                for _ in 0..12 {
                    let mut assumptions = vec![None; vars];
                    for _ in 0..6 {
                        assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
                    }
                    let answer = preimage.query(&assumptions);
                    let mut constrained = formula.clone();
                    constrained.extend(assumptions.iter().enumerate().filter_map(
                        |(variable, value)| value.map(|value| Clause(vec![(variable, value)])),
                    ));
                    assert_eq!(
                        answer.is_some(),
                        solve_with_varisat(vars, &constrained).is_some(),
                        "{kind}/{order}"
                    );
                    if let Some(assignment) = answer {
                        assert!(satisfies(&formula, &assignment));
                    }
                }
            }
        }
    }

    #[test]
    fn symbolic_preimage_reuses_exact_frame_cycles() {
        let width = 6;
        let horizon = 200;
        let (vars, formula) = temporal_composition_formula("majority3", width, horizon).unwrap();
        let mut preimage = SymbolicPreimageTransition::recognize_ordered(
            &formula,
            width,
            horizon,
            200_000,
            "dependency",
        )
        .unwrap();
        assert!(preimage.cycle().is_some());
        assert!(preimage.compiled_frames() < horizon / 4);
        let mut assumptions = vec![None; vars];
        assumptions[0] = Some(true);
        assumptions[137 * width + 3] = Some(false);
        let answer = preimage.query(&assumptions);
        let mut constrained = formula.clone();
        constrained.extend(
            assumptions
                .iter()
                .enumerate()
                .filter_map(|(variable, value)| value.map(|value| Clause(vec![(variable, value)]))),
        );
        assert_eq!(
            answer.is_some(),
            solve_with_varisat(vars, &constrained).is_some()
        );
        if let Some(assignment) = answer {
            assert!(satisfies(&formula, &assignment));
        }
    }

    #[test]
    fn preimage_growth_guard_rejects_early_without_approximating() {
        let width = 9;
        let horizon = 137;
        let (_, explosive) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let error = SymbolicPreimageTransition::recognize_ordered(
            &explosive,
            width,
            horizon,
            200_000,
            "dependency-guard",
        )
        .err()
        .unwrap();
        assert!(error.contains("growth guard"));

        let (_, compact) = temporal_composition_formula("majority3", width, horizon).unwrap();
        let guarded = SymbolicPreimageTransition::recognize_ordered(
            &compact,
            width,
            horizon,
            200_000,
            "dependency-guard",
        )
        .unwrap();
        assert!(guarded.cycle().is_some());
    }

    #[test]
    fn hybrid_preimage_falls_back_exactly_on_bdd_growth() {
        let width = 9;
        let horizon = 137;
        let (vars, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let (mut hybrid, backend, reason) =
            HybridTemporalPreimage::recognize(&formula, width, horizon, 200_000).unwrap();
        assert_eq!(backend, "cdcl-fallback");
        assert!(reason.unwrap().contains("growth guard"));
        let mut assumptions = vec![None; vars];
        assumptions[3] = Some(true);
        assumptions[91 * width + 4] = Some(false);
        let answer = hybrid.query(&assumptions);
        let mut constrained = formula.clone();
        constrained.extend(
            assumptions
                .iter()
                .enumerate()
                .filter_map(|(variable, value)| value.map(|value| Clause(vec![(variable, value)]))),
        );
        assert_eq!(
            answer.is_some(),
            solve_with_varisat(vars, &constrained).is_some()
        );
        if let Some(assignment) = answer {
            assert!(satisfies(&formula, &assignment));
        }
    }

    #[test]
    fn checkpoint_cdcl_reuses_bdd_prefix_exactly() {
        let width = 9;
        let horizon = 137;
        let (vars, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let mut checkpoint = CheckpointCdclPreimage::recognize_with_encoding(
            &formula, width, horizon, 20, 200_000, "bdd",
        )
        .unwrap();
        assert_eq!(checkpoint.checkpoint, 20);
        assert!(checkpoint.bdd_nodes > 0);
        let mut rng = Rng(0xc1ac_0170);
        for _ in 0..3 {
            let mut assumptions = vec![None; vars];
            for _ in 0..7 {
                assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
            }
            let answer = checkpoint.query(&assumptions);
            let mut constrained = formula.clone();
            constrained.extend(
                assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Clause(vec![(variable, value)]))
                    }),
            );
            assert_eq!(
                answer.is_some(),
                solve_with_varisat(vars, &constrained).is_some()
            );
            if let Some(assignment) = answer {
                assert!(satisfies(&formula, &assignment));
            }
        }
    }

    #[test]
    fn aig_checkpoint_preserves_answers_and_witnesses() {
        let width = 5;
        let horizon = 23;
        let (vars, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let mut checkpoint = CheckpointCdclPreimage::recognize_with_encoding(
            &formula, width, horizon, 7, 50_000, "aig",
        )
        .unwrap();
        assert!(checkpoint.encoding_nodes > 0);
        assert_eq!(checkpoint.encoding_clauses, checkpoint.encoding_nodes * 3);
        let mut assumptions = vec![None; vars];
        assumptions[2] = Some(true);
        assumptions[width * 11 + 3] = Some(false);
        let answer = checkpoint.query(&assumptions);
        let mut constrained = formula.clone();
        constrained.extend(
            assumptions
                .iter()
                .enumerate()
                .filter_map(|(variable, value)| value.map(|value| Clause(vec![(variable, value)]))),
        );
        assert_eq!(
            answer.is_some(),
            solve_with_varisat(vars, &constrained).is_some()
        );
        if let Some(assignment) = answer {
            assert!(satisfies(&formula, &assignment));
        }
    }

    #[test]
    fn lazy_checkpoint_adds_observed_prefix_cones_exactly() {
        let width = 5;
        let horizon = 31;
        let checkpoint_frame = 9;
        let (vars, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let mut checkpoint = CheckpointCdclPreimage::recognize_with_encoding(
            &formula,
            width,
            horizon,
            checkpoint_frame,
            50_000,
            "lazy-bdd",
        )
        .unwrap();
        let initial_nodes = checkpoint.encoding_nodes;
        assert!(initial_nodes < checkpoint.bdd_nodes);
        let mut assumptions = vec![None; vars];
        assumptions[width * 3 + 1] = Some(true);
        assumptions[width * 7 + 4] = Some(false);
        assumptions[width * 19 + 2] = Some(true);
        let answer = checkpoint.query(&assumptions);
        assert!(checkpoint.encoding_nodes >= initial_nodes);
        let mut constrained = formula.clone();
        constrained.extend(
            assumptions
                .iter()
                .enumerate()
                .filter_map(|(variable, value)| value.map(|value| Clause(vec![(variable, value)]))),
        );
        assert_eq!(
            answer.is_some(),
            solve_with_varisat(vars, &constrained).is_some()
        );
        if let Some(assignment) = answer {
            assert!(satisfies(&formula, &assignment));
            assert!(assumptions.iter().enumerate().all(|(variable, required)| {
                required.is_none_or(|value| assignment[variable] == value)
            }));
        }
    }

    #[test]
    fn native_bdd_theory_reconciles_checkpoint_models_exactly() {
        let width = 5;
        let horizon = 31;
        let (vars, formula) = temporal_composition_formula("cascade4", width, horizon).unwrap();
        let mut engine =
            NativeBddTheoryPreimage::recognize(&formula, width, horizon, 9, 50_000).unwrap();
        let mut rng = Rng(0x07e0_f1e5);
        for _ in 0..5 {
            let mut assumptions = vec![None; vars];
            for _ in 0..6 {
                assumptions[rng.below(vars)] = Some(rng.next() & 1 == 1);
            }
            let answer = engine.query(&assumptions);
            let mut constrained = formula.clone();
            constrained.extend(
                assumptions
                    .iter()
                    .enumerate()
                    .filter_map(|(variable, value)| {
                        value.map(|value| Clause(vec![(variable, value)]))
                    }),
            );
            assert_eq!(
                answer.is_some(),
                solve_with_varisat(vars, &constrained).is_some()
            );
            if let Some(assignment) = answer {
                assert!(satisfies(&formula, &assignment));
                assert!(assumptions.iter().enumerate().all(|(variable, required)| {
                    required.is_none_or(|value| assignment[variable] == value)
                }));
            }
        }
    }

    #[test]
    fn cq_portfolio_gate_is_static_and_explainable() {
        let (_, cascade) = temporal_composition_formula("cascade4", 9, 137).unwrap();
        let dense = cq_portfolio_decision(&cascade, 9, 137, 50, 5.0).unwrap();
        assert!(dense.specialized);
        assert_eq!(dense.reason, "dense-transition");
        assert!(
            !cq_portfolio_decision(&cascade, 9, 137, 7, 5.0)
                .unwrap()
                .specialized
        );
        assert!(
            !cq_portfolio_decision(&cascade, 9, 137, 50, 10.0)
                .unwrap()
                .specialized
        );
        let (_, wide_cascade) = temporal_composition_formula("cascade4", 10, 137).unwrap();
        assert!(
            !cq_portfolio_decision(&wide_cascade, 10, 137, 50, 5.0)
                .unwrap()
                .specialized
        );

        let (_, hub) = temporal_composition_formula("hub3", 7, 137).unwrap();
        let narrow_hub = cq_portfolio_decision(&hub, 7, 137, 128, 5.0).unwrap();
        assert!(narrow_hub.specialized);
        assert_eq!(narrow_hub.reason, "narrow-hub");
        assert!(
            !cq_portfolio_decision(&hub, 7, 137, 127, 5.0)
                .unwrap()
                .specialized
        );

        let (_, tree_short) = temporal_composition_formula("tree3", 9, 137).unwrap();
        let fallback = cq_portfolio_decision(&tree_short, 9, 137, 50, 5.0).unwrap();
        assert!(!fallback.specialized);
        assert_eq!(fallback.reason, "cdcl-fallback");

        let (_, tree_long) = temporal_composition_formula("tree3", 11, 1_333).unwrap();
        let conservative_fallback = cq_portfolio_decision(&tree_long, 11, 1_333, 50, 5.0).unwrap();
        assert!(!conservative_fallback.specialized);
        assert_eq!(conservative_fallback.reason, "cdcl-fallback");

        let (_, watchdog) = temporal_composition_formula("watchdog4", 9, 137).unwrap();
        assert_eq!(
            cq_portfolio_decision(&watchdog, 9, 137, 50, 5.0)
                .unwrap()
                .reason,
            "dense-transition"
        );
        let (_, sensor_vote) = temporal_composition_formula("sensor-vote3", 8, 257).unwrap();
        assert_eq!(
            cq_portfolio_decision(&sensor_vote, 8, 257, 50, 5.0)
                .unwrap()
                .reason,
            "cdcl-fallback"
        );
    }

    #[test]
    fn ascii_aiger_import_preserves_transition_and_initial_state() {
        let path =
            std::env::temp_dir().join(format!("cq-sat-aiger-import-{}.aag", std::process::id()));
        fs::write(
            &path,
            "aag 5 0 4 1 1\n2 4\n4 6\n6 8\n8 10\n8\n10 2 5\nc\nsmall closed sequential model\n",
        )
        .unwrap();
        let model = parse_aag(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(model.latches.len(), 4);
        assert_eq!(model.outputs, vec![8]);
        let (variables, formula, initial) = aag_temporal_formula(&model, 3).unwrap();
        assert_eq!(variables, 16);
        assert_eq!(initial, vec![Some(false); 4]);
        assert!(
            cq_portfolio_decision(&formula, 4, 3, 8, 2.0)
                .unwrap()
                .specialized
        );

        let transition = SymbolicTemporalTransition::recognize(&formula, 4, 3).unwrap();
        let mut assumptions = vec![None; variables];
        assumptions[..4].copy_from_slice(&initial);
        let assignment = transition.query(&assumptions).unwrap();
        assert!(assignment.iter().all(|value| !value));
        assert!(satisfies(&formula, &assignment));
    }

    #[test]
    fn external_aiger_counter_reports_exact_unsafe_trace() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/aiger/counter-overflow-4.aag");
        let output = std::env::temp_dir().join(format!(
            "cq-sat-aiger-counter-{}-portfolio.csv",
            std::process::id()
        ));
        let safety = std::env::temp_dir().join(format!(
            "cq-sat-aiger-counter-{}-safety.txt",
            std::process::id()
        ));
        verify_cq_aiger(&input, 20, 10, 200_000, &output, &safety).unwrap();
        let portfolio = fs::read_to_string(&output).unwrap();
        let result = fs::read_to_string(&safety).unwrap();
        std::fs::remove_file(output).unwrap();
        std::fs::remove_file(safety).unwrap();
        assert!(portfolio.contains(",cdcl,cdcl-fallback,"));
        assert!(portfolio.contains(",true,true,ok\n"));
        assert!(result.starts_with("status=UNSAFE\n"));
        assert!(result.contains("bad_frame=15\n"));
        assert!(result.contains("15,1111\n"));
    }

    #[test]
    fn constant_false_aiger_property_reports_safe_without_solving() {
        let stem = format!("cq-sat-aiger-safe-{}", std::process::id());
        let input = std::env::temp_dir().join(format!("{stem}.aag"));
        let output = std::env::temp_dir().join(format!("{stem}.csv"));
        let safety = std::env::temp_dir().join(format!("{stem}.txt"));
        fs::write(&input, "aag 1 0 1 1 0\n2 2\n0\n").unwrap();
        verify_cq_aiger(&input, 20, 10, 200_000, &output, &safety).unwrap();
        let portfolio = fs::read_to_string(&output).unwrap();
        let result = fs::read_to_string(&safety).unwrap();
        std::fs::remove_file(input).unwrap();
        std::fs::remove_file(output).unwrap();
        std::fs::remove_file(safety).unwrap();
        assert!(portfolio.contains(",static,constant-false-output,"));
        assert!(portfolio.lines().all(|line| line.split(',').count() == 26));
        assert!(result.starts_with("status=SAFE\n"));
        assert!(result.contains("backend=static\n"));
    }

    #[test]
    fn primary_input_aiger_uses_exact_cdcl_and_emits_input_trace() {
        let stem = format!("cq-sat-aiger-input-{}", std::process::id());
        let input = std::env::temp_dir().join(format!("{stem}.aag"));
        let output = std::env::temp_dir().join(format!("{stem}.csv"));
        let safety = std::env::temp_dir().join(format!("{stem}.txt"));
        fs::write(&input, "aag 2 1 1 1 0\n2\n4 2\n4\n").unwrap();
        verify_cq_aiger(&input, 2, 1, 200_000, &output, &safety).unwrap();
        let portfolio = fs::read_to_string(&output).unwrap();
        let result = fs::read_to_string(&safety).unwrap();
        std::fs::remove_file(input).unwrap();
        std::fs::remove_file(output).unwrap();
        std::fs::remove_file(safety).unwrap();
        assert!(portfolio.contains(",cdcl,aiger-primary-inputs,"));
        assert!(portfolio.lines().all(|line| line.split(',').count() == 26));
        assert!(result.starts_with("status=UNSAFE\n"));
        assert!(result.contains("bad_frame=1\n"));
        assert!(result.contains("frame,latch_bits_low_to_high,input_bits_low_to_high\n"));
        assert!(result.contains("0,0,1\n"));
        assert!(result.contains("1,1,"));
    }

    #[test]
    fn independent_input_driven_aiger_models_match_known_safety_results() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/aiger");
        for (name, horizon, expected) in [
            ("petersons-algorithm-2-threads-1-core.aag", 20, "SAFE"),
            ("spi-bus-receive-e-08-bits.aag", 16, "UNSAFE"),
        ] {
            let stem = format!("cq-sat-aiger-known-{expected}-{}", std::process::id());
            let output = std::env::temp_dir().join(format!("{stem}.csv"));
            let safety = std::env::temp_dir().join(format!("{stem}.txt"));
            verify_cq_aiger(&root.join(name), horizon, 10, 200_000, &output, &safety).unwrap();
            let portfolio = fs::read_to_string(&output).unwrap();
            let result = fs::read_to_string(&safety).unwrap();
            std::fs::remove_file(output).unwrap();
            std::fs::remove_file(safety).unwrap();
            assert!(portfolio.contains(",cdcl,aiger-primary-inputs,"));
            assert!(portfolio.contains(",true,true,ok\n"));
            assert!(result.starts_with(&format!("status={expected}\n")));
            if expected == "UNSAFE" {
                assert!(result.contains("bad_frame=16\n"));
                assert!(result.contains("input_bits_low_to_high\n"));
            }
        }
    }

    #[test]
    fn firmware_safety_gate_distinguishes_safe_and_rejected_builds() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/products/infusion-pump/firmware");
        for (name, expected_safe, expected_status) in [
            ("safe-controller.aag", true, "status=SAFE\n"),
            ("door-interlock-regression.aag", false, "status=UNSAFE\n"),
        ] {
            let artifacts = std::env::temp_dir().join(format!(
                "cq-sat-firmware-gate-{name}-{}",
                std::process::id()
            ));
            let safe = firmware_safety_gate(&root.join(name), 8, &artifacts).unwrap();
            assert_eq!(safe, expected_safe);
            let report = fs::read_to_string(artifacts.join("safety-report.txt")).unwrap();
            assert!(report.starts_with(expected_status));
            assert!(artifacts.join("solver-metrics.csv").is_file());
            if !expected_safe {
                assert!(report.contains("bad_frame=1\n"));
                assert!(report.contains("0,0,10\n"));
                assert!(report.contains("1,1,01\n"));
            }
            std::fs::remove_dir_all(artifacts).unwrap();
        }
    }

    #[test]
    fn evidence_index_rejects_oversized_file_before_hashing() {
        let root =
            std::env::temp_dir().join(format!("cq-sat-oversized-evidence-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let oversized = root.join("oversized.bin");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(YOSYS_FILE_LIMIT_BYTES + 1)
            .unwrap();
        fs::write(
            root.join("evidence.sha256"),
            format!("{}  oversized.bin\n", "0".repeat(64)),
        )
        .unwrap();
        let index_digest = sha256_file(&root.join("evidence.sha256")).unwrap();
        assert!(
            validate_evidence_index(&root, &index_digest)
                .unwrap_err()
                .contains("evidence file exceeds")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rtl_safety_gate_synthesizes_and_names_exact_traces_when_yosys_is_available() {
        if Command::new("yosys").arg("-V").output().is_err() {
            eprintln!("skipping RTL integration test because Yosys is unavailable");
            return;
        }
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/products/infusion-pump/rtl");
        for (name, expected_safe, expected_status) in [
            ("safe-controller.sv", true, "status=SAFE\n"),
            ("door-interlock-regression.sv", false, "status=UNSAFE\n"),
        ] {
            let artifacts =
                std::env::temp_dir().join(format!("cq-sat-rtl-gate-{name}-{}", std::process::id()));
            let safe = firmware_rtl_safety_gate(
                &root.join(name),
                "infusion_pump_controller",
                8,
                &artifacts,
            )
            .unwrap();
            assert_eq!(safe, expected_safe);
            let report = fs::read_to_string(artifacts.join("safety-report.txt")).unwrap();
            assert!(report.starts_with(expected_status));
            assert!(report.contains("top=infusion_pump_controller\n"));
            assert!(report.contains("generated_model=model.aag\n"));
            assert!(artifacts.join("source.sv").is_file());
            assert!(artifacts.join("signal.map").is_file());
            assert!(artifacts.join("run-manifest.txt").is_file());
            if !expected_safe {
                assert!(report.contains("bad_frame=1\n"));
                assert!(report.contains("bad_output_name=bad\n"));
                assert!(
                    report.contains("named_frame,requested_motor_active,motor_request,door_open\n")
                );
                assert!(report.contains("0,0,1,0\n"));
                assert!(report.contains("1,1,0,1\n"));
            }
            std::fs::remove_dir_all(artifacts).unwrap();
        }
        assert!(
            firmware_rtl_safety_gate(
                &root.join("safe-controller.sv"),
                "bad; shell",
                8,
                &std::env::temp_dir().join("cq-sat-invalid-rtl-top")
            )
            .is_err()
        );
        #[cfg(unix)]
        {
            let real_artifacts =
                std::env::temp_dir().join(format!("cq-sat-real-artifacts-{}", std::process::id()));
            let linked_artifacts = std::env::temp_dir()
                .join(format!("cq-sat-linked-artifacts-{}", std::process::id()));
            fs::create_dir(&real_artifacts).unwrap();
            std::os::unix::fs::symlink(&real_artifacts, &linked_artifacts).unwrap();
            assert!(
                firmware_rtl_safety_gate(
                    &root.join("safe-controller.sv"),
                    "infusion_pump_controller",
                    8,
                    &linked_artifacts,
                )
                .unwrap_err()
                .contains("real directory")
            );
            fs::remove_file(linked_artifacts).unwrap();
            fs::remove_dir(real_artifacts).unwrap();
        }
        let oversized =
            std::env::temp_dir().join(format!("cq-sat-oversized-rtl-{}.sv", std::process::id()));
        let oversized_file = fs::File::create(&oversized).unwrap();
        oversized_file.set_len(10 * 1024 * 1024 + 1).unwrap();
        assert!(
            firmware_rtl_safety_gate(
                &oversized,
                "infusion_pump_controller",
                8,
                &std::env::temp_dir().join("cq-sat-oversized-rtl")
            )
            .is_err()
        );
        std::fs::remove_file(oversized).unwrap();
    }

    #[test]
    fn hierarchical_rtl_and_bounded_query_reuse_agree_with_cold_bmc() {
        if Command::new("yosys").arg("-V").output().is_err() {
            eprintln!("skipping hierarchical RTL test because Yosys is unavailable");
            return;
        }
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/products/infusion-pump/rtl");
        let artifacts =
            std::env::temp_dir().join(format!("cq-sat-hierarchical-rtl-{}", std::process::id()));
        assert!(
            firmware_rtl_safety_gate(
                &root.join("multimodule-controller.sv"),
                "infusion_pump_system",
                8,
                &artifacts,
            )
            .unwrap()
        );
        let script = fs::read_to_string(artifacts.join("synthesis.ys")).unwrap();
        assert!(script.contains("flatten\n"));
        assert!(script.contains("setundef -zero\n"));
        let benchmark = artifacts.join("query-reuse.csv");
        benchmark_aiger_query_reuse(&artifacts.join("model.aag"), &[4, 8], 2, &benchmark).unwrap();
        let results = fs::read_to_string(benchmark).unwrap();
        assert_eq!(results.lines().count(), 3);
        assert!(results.lines().skip(1).all(|row| row.ends_with(",true,ok")));
        assert!(results.lines().skip(1).all(|row| row.contains(",2,2,8,0,")));
        assert!(aiger_reuse_gate(15_000, 2));
        assert!(!aiger_reuse_gate(15_001, 2));
        assert!(!aiger_reuse_gate(1_000, 1));
        std::fs::remove_dir_all(artifacts).unwrap();
    }

    #[test]
    fn rtl_project_gate_stages_multiple_sources_and_rejects_ambiguous_inputs() {
        if Command::new("yosys").arg("-V").output().is_err() {
            eprintln!("skipping RTL project test because Yosys is unavailable");
            return;
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/products/infusion-pump/rtl/project");
        let sources = vec![root.join("pump-components.sv"), root.join("pump-system.sv")];
        let artifacts =
            std::env::temp_dir().join(format!("cq-sat-rtl-project-{}", std::process::id()));
        assert!(
            firmware_rtl_project_safety_gate(&sources, "infusion_pump_system", 8, &artifacts,)
                .unwrap()
        );
        assert!(artifacts.join("source-0000.sv").is_file());
        assert!(artifacts.join("source-0001.sv").is_file());
        assert!(!artifacts.join("source.sv").exists());
        let synthesis = fs::read_to_string(artifacts.join("synthesis.ys")).unwrap();
        assert!(synthesis.contains("source-0000.sv source-0001.sv"));
        assert!(!synthesis.contains(&root.to_string_lossy().to_string()));
        let manifest = fs::read_to_string(artifacts.join("run-manifest.txt")).unwrap();
        assert!(manifest.contains("status=SAFE\n"));
        assert!(manifest.contains("schema_version=4\n"));
        assert!(manifest.contains("firmware_cli_version=2\n"));
        assert!(manifest.contains("evidence_digest_algorithm=sha256\n"));
        assert!(manifest.contains("evidence_index=evidence.sha256\n"));
        assert!(manifest.contains("source_count=2\n"));
        assert!(manifest.contains("source_0="));
        assert!(manifest.contains("source_1="));
        assert!(manifest.contains(&format!("containment_platform={}\n", std::env::consts::OS)));
        assert!(manifest.contains("process_group_timeout_kill=true\n"));
        assert!(manifest.contains(&format!(
            "synthesis_memory_limit_kind={}\n",
            synthesis_memory_limit_kind()
        )));
        assert!(manifest.contains(&format!(
            "synthesis_memory_limit_bytes={}\n",
            synthesis_memory_limit_bytes()
        )));
        assert!(manifest.contains("synthesis_file_limit_bytes=536870912\n"));
        validate_rtl_artifact_bundle(&artifacts).unwrap();
        let source_snapshot = artifacts.join("source-0000.sv");
        let source_bytes = fs::read(&source_snapshot).unwrap();
        fs::write(&source_snapshot, b"tampered RTL").unwrap();
        assert!(
            validate_rtl_artifact_bundle(&artifacts)
                .unwrap_err()
                .contains("evidence SHA-256 mismatch")
        );
        fs::write(&source_snapshot, &source_bytes).unwrap();
        #[cfg(unix)]
        {
            let external = std::env::temp_dir().join(format!(
                "cq-sat-evidence-symlink-target-{}",
                std::process::id()
            ));
            fs::write(&external, &source_bytes).unwrap();
            fs::remove_file(&source_snapshot).unwrap();
            std::os::unix::fs::symlink(&external, &source_snapshot).unwrap();
            assert!(
                validate_rtl_artifact_bundle(&artifacts)
                    .unwrap_err()
                    .contains("not a regular file")
            );
            fs::remove_file(&source_snapshot).unwrap();
            fs::write(&source_snapshot, &source_bytes).unwrap();
            fs::remove_file(external).unwrap();
        }
        validate_rtl_artifact_bundle(&artifacts).unwrap();
        fs::write(
            artifacts.join("run-manifest.txt"),
            manifest.replacen("status=SAFE", "status=UNSAFE", 1),
        )
        .unwrap();
        assert!(
            validate_rtl_artifact_bundle(&artifacts)
                .unwrap_err()
                .contains("status disagrees")
        );
        fs::write(
            artifacts.join("run-manifest.txt"),
            format!("{manifest}unexpected_field=value\n"),
        )
        .unwrap();
        assert!(
            validate_rtl_artifact_bundle(&artifacts)
                .unwrap_err()
                .contains("fields or ordering")
        );
        fs::write(artifacts.join("run-manifest.txt"), &manifest).unwrap();
        assert!(
            firmware_rtl_safety_gate(
                &root.parent().unwrap().join("safe-controller.sv"),
                "infusion_pump_controller",
                8,
                &artifacts,
            )
            .unwrap()
        );
        assert!(artifacts.join("source.sv").is_file());
        assert!(!artifacts.join("source-0000.sv").exists());
        assert!(!artifacts.join("source-0001.sv").exists());
        std::fs::remove_dir_all(artifacts).unwrap();

        assert!(
            firmware_rtl_project_safety_gate(
                &[sources[0].clone(), sources[0].clone()],
                "infusion_pump_system",
                8,
                &std::env::temp_dir().join("cq-sat-duplicate-rtl-project"),
            )
            .unwrap_err()
            .contains("duplicate RTL source")
        );
        assert!(
            firmware_rtl_project_safety_gate(
                &vec![sources[0].clone(); 65],
                "infusion_pump_system",
                8,
                &std::env::temp_dir().join("cq-sat-too-many-rtl-sources"),
            )
            .unwrap_err()
            .contains("between 1 and 64")
        );
    }

    #[test]
    fn rtl_environment_assumptions_are_exact_named_all_frame_constraints() {
        if Command::new("yosys").arg("-V").output().is_err() {
            eprintln!("skipping RTL assumption test because Yosys is unavailable");
            return;
        }
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/products/infusion-pump/rtl");
        let source = root.join("door-interlock-regression.sv");
        let assumptions = root.join("door-closed.assumptions");
        let artifacts =
            std::env::temp_dir().join(format!("cq-sat-rtl-assumptions-{}", std::process::id()));
        assert!(
            firmware_rtl_project_safety_gate_with_assumptions(
                &[source],
                "infusion_pump_controller",
                8,
                &artifacts,
                Some(&assumptions),
                None,
            )
            .unwrap()
        );
        assert_eq!(
            fs::read_to_string(artifacts.join("assumptions.txt")).unwrap(),
            fs::read_to_string(&assumptions).unwrap()
        );
        let report = fs::read_to_string(artifacts.join("safety-report.txt")).unwrap();
        assert!(report.contains("assumption_count=1\n"));
        assert!(report.contains("assumption_0=door_open=0\n"));
        let manifest = fs::read_to_string(artifacts.join("run-manifest.txt")).unwrap();
        assert!(manifest.contains("assumption_count=1\n"));
        validate_rtl_artifact_bundle(&artifacts).unwrap();

        let unknown = std::env::temp_dir().join(format!(
            "cq-sat-unknown-assumption-{}.txt",
            std::process::id()
        ));
        fs::write(&unknown, "missing_input=0\n").unwrap();
        let (constraints, _) = parse_environment_assumptions(&unknown).unwrap();
        let model = parse_aag(&artifacts.join("model.aag")).unwrap();
        assert!(
            aag_bmc_encoding_with_constraints(&model, 8, &constraints)
                .err()
                .unwrap()
                .contains("matched 0 synthesized inputs")
        );
        std::fs::remove_file(unknown).unwrap();
        std::fs::remove_dir_all(artifacts).unwrap();
    }

    #[test]
    fn rtl_project_config_v1_is_strict_and_path_safe() {
        let scratch =
            std::env::temp_dir().join(format!("cq-sat-project-config-{}", std::process::id()));
        fs::create_dir_all(&scratch).unwrap();
        let config_path = scratch.join("cq-project.conf");
        fs::write(
            &config_path,
            "project_version=1\ntop=pump_top\nhorizon=32\nclock=clk:posedge\nreset=rst_n:deasserted-high\nsource=rtl/top.sv\nsource=rtl/memory.sv\ninclude_dir=rtl/include\nparameter=DEPTH:16\nassumptions=env.assumptions\n",
        )
        .unwrap();
        let parsed = parse_rtl_project_config(&config_path).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.top, "pump_top");
        assert_eq!(parsed.horizon, 32);
        assert_eq!(parsed.sources.len(), 2);
        assert_eq!(parsed.include_dirs, vec![PathBuf::from("rtl/include")]);
        assert_eq!(
            parsed.parameters,
            vec![("DEPTH".to_string(), "16".to_string())]
        );
        assert_eq!(parsed.clock, ("clk".to_string(), "posedge".to_string()));
        assert_eq!(
            parsed.reset,
            RtlResetPolicy::Deasserted {
                signal: "rst_n".to_string(),
                level: true
            }
        );
        fs::write(
            &config_path,
            "project_version=2\ntop=pump_top\nhorizon=32\nclock=clk:posedge\nreset=rst_n:active-low:2\nsource=rtl/top.sv\n",
        )
        .unwrap();
        let startup = parse_rtl_project_config(&config_path).unwrap();
        assert_eq!(startup.version, 2);
        assert_eq!(
            startup.reset,
            RtlResetPolicy::Startup {
                signal: "rst_n".to_string(),
                active_low: true,
                asserted_frames: 2,
            }
        );

        for (body, expected) in [
            (
                "project_version=1\ntop=x\nhorizon=1\nclock=c:posedge\nreset=none\nsource=../escape.sv\n",
                "without traversal",
            ),
            (
                "project_version=1\ntop=x\ntop=y\nhorizon=1\nclock=c:posedge\nreset=none\nsource=x.sv\n",
                "duplicate",
            ),
            (
                "project_version=1\ntop=x\nhorizon=1\nclock=c:posedge\nreset=none\nsource=x.sv\nshell=evil\n",
                "unknown",
            ),
            (
                "project_version=1\ntop=x\nhorizon=2\nclock=c:posedge\nreset=rst_n:active-low:1\nsource=x.sv\n",
                "require project_version=2",
            ),
        ] {
            fs::write(&config_path, body).unwrap();
            assert!(
                parse_rtl_project_config(&config_path)
                    .unwrap_err()
                    .contains(expected)
            );
        }
        fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn rtl_config_gate_snapshots_includes_applies_parameters_and_maps_memory() {
        if Command::new("yosys").arg("-V").output().is_err() {
            eprintln!("skipping RTL config test because Yosys is unavailable");
            return;
        }
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/products/infusion-pump/rtl/config-project");
        let artifacts =
            std::env::temp_dir().join(format!("cq-sat-config-project-{}", std::process::id()));
        assert!(
            firmware_rtl_config_safety_gate(&project.join("cq-project.conf"), &artifacts).unwrap()
        );
        validate_rtl_artifact_bundle(&artifacts).unwrap();
        assert_eq!(
            fs::read(artifacts.join("cq-project.conf")).unwrap(),
            fs::read(project.join("cq-project.conf")).unwrap()
        );
        assert_eq!(
            fs::read(artifacts.join("include-0000/pump-widths.svh")).unwrap(),
            fs::read(project.join("include/pump-widths.svh")).unwrap()
        );
        let script = fs::read_to_string(artifacts.join("synthesis.ys")).unwrap();
        assert!(script.contains("-Iinclude-0000"));
        assert!(script.contains("chparam -set DEPTH 8 infusion_pump_memory"));
        assert!(script.contains("select -assert-count 1 infusion_pump_memory/clk"));
        assert!(script.contains("memory_map"));
        let manifest = fs::read_to_string(artifacts.join("run-manifest.txt")).unwrap();
        assert!(manifest.contains("schema_version=4\n"));
        assert!(manifest.contains("firmware_cli_version=2\n"));
        assert!(manifest.contains("clock_policy=clk:posedge\n"));
        assert!(manifest.contains("reset_policy=rst_n:active-low:1\n"));
        assert!(manifest.contains("parameters=DEPTH:8\n"));
        let report = fs::read_to_string(artifacts.join("safety-report.txt")).unwrap();
        assert!(
            report.contains("assumption_0=rst_n=startup(asserted_frames=1,asserted_value=0)\n")
        );
        let config_snapshot = fs::read_to_string(artifacts.join("cq-project.conf")).unwrap();
        fs::write(
            artifacts.join("cq-project.conf"),
            config_snapshot.replace("parameter=DEPTH:8", "parameter=DEPTH:7"),
        )
        .unwrap();
        assert!(
            validate_rtl_artifact_bundle(&artifacts)
                .unwrap_err()
                .contains("evidence SHA-256 mismatch")
        );
        fs::remove_dir_all(artifacts).unwrap();
    }

    #[test]
    fn startup_reset_constraints_change_value_at_the_exact_frame_boundary() {
        let model = AagModel {
            max_variable: 2,
            inputs: vec![2],
            input_names: vec!["rst_n".to_string()],
            latches: vec![AagLatch {
                current: 4,
                next: 4,
                initial: Some(false),
            }],
            latch_names: vec!["state".to_string()],
            outputs: vec![4],
            output_names: vec!["bad".to_string()],
            ands: Vec::new(),
        };
        let encoding = aag_bmc_encoding_with_constraints(
            &model,
            3,
            &[AagInputConstraint {
                name: "rst_n".to_string(),
                pattern: AagInputConstraintPattern::StartupReset {
                    asserted_frames: 2,
                    asserted_value: false,
                },
            }],
        )
        .unwrap();
        for (frame, expected) in [(0, false), (1, false), (2, true), (3, true)] {
            let variable = frame * model.max_variable;
            assert!(
                encoding
                    .clauses
                    .iter()
                    .any(|clause| clause.0 == vec![(variable, expected)])
            );
        }
    }

    #[test]
    fn public_rtl_corpus_accepts_parameterless_safe_and_unsafe_bundles() {
        if Command::new("yosys").arg("-V").output().is_err() {
            eprintln!("skipping public RTL corpus test because Yosys is unavailable");
            return;
        }
        let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/rtl/yosys-simple");
        for (case, expected_safe) in [("always01-safe", true), ("always01-unsafe", false)] {
            let artifacts = std::env::temp_dir()
                .join(format!("cq-sat-public-rtl-{case}-{}", std::process::id()));
            assert_eq!(
                firmware_rtl_config_safety_gate(&corpus.join(format!("{case}.conf")), &artifacts,)
                    .unwrap(),
                expected_safe
            );
            validate_rtl_artifact_bundle(&artifacts).unwrap();
            let manifest = fs::read_to_string(artifacts.join("run-manifest.txt")).unwrap();
            assert!(manifest.contains("parameter_count=0\nparameters=none\n"));
            fs::remove_dir_all(artifacts).unwrap();
        }
    }

    fn fuzz_mutation(seed: &[u8], iteration: usize) -> Vec<u8> {
        let mut bytes = seed.to_vec();
        if bytes.is_empty() {
            bytes.push(0);
        }
        let mut rng =
            Rng(0x9e37_79b9_7f4a_7c15 ^ iteration as u64 ^ (seed.len() as u64).rotate_left(23));
        for _ in 0..=iteration % 8 {
            match rng.below(4) {
                0 => {
                    let index = rng.below(bytes.len());
                    bytes[index] ^= 1 << rng.below(8);
                }
                1 => bytes.truncate(rng.below(bytes.len() + 1)),
                2 if bytes.len() < 16_384 => {
                    let index = rng.below(bytes.len() + 1);
                    bytes.insert(index, rng.below(256) as u8);
                }
                _ => {
                    let index = rng.below(bytes.len());
                    bytes[index] = rng.below(256) as u8;
                }
            }
            if bytes.is_empty() {
                bytes.push(rng.below(256) as u8);
            }
        }
        bytes.truncate(16_384);
        bytes
    }

    fn corpus_files(path: &Path) -> Vec<Vec<u8>> {
        let mut paths = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|entry| entry.is_file())
            .collect::<Vec<_>>();
        paths.sort();
        paths.iter().map(|entry| fs::read(entry).unwrap()).collect()
    }

    #[test]
    fn parser_fuzz_regression_corpora_are_panic_free_and_bounded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let scratch =
            std::env::temp_dir().join(format!("cq-sat-parser-fuzz-{}", std::process::id()));
        fs::create_dir(&scratch).unwrap();

        let mut aiger_seeds = corpus_files(&root.join("tests/fuzz-corpus/aiger"));
        aiger_seeds.push(fs::read(root.join("examples/aiger/counter-overflow-4.aag")).unwrap());
        let aiger_input = scratch.join("mutated.aag");
        for iteration in 0..5_000 {
            let bytes = fuzz_mutation(&aiger_seeds[iteration % aiger_seeds.len()], iteration);
            fs::write(&aiger_input, bytes).unwrap();
            let _ = parse_aag(&aiger_input);
        }
        let oversized_aiger = scratch.join("oversized.aag");
        fs::File::create(&oversized_aiger)
            .unwrap()
            .set_len(AAG_INPUT_LIMIT_BYTES + 1)
            .unwrap();
        assert!(
            parse_aag(&oversized_aiger)
                .unwrap_err()
                .contains("exceeds safety limit")
        );

        let mut assumption_seeds = corpus_files(&root.join("tests/fuzz-corpus/assumptions"));
        assumption_seeds.push(
            fs::read(root.join("examples/products/infusion-pump/rtl/door-closed.assumptions"))
                .unwrap(),
        );
        let assumption_input = scratch.join("mutated.assumptions");
        for iteration in 0..5_000 {
            let bytes = fuzz_mutation(
                &assumption_seeds[iteration % assumption_seeds.len()],
                iteration ^ 0x5a5a,
            );
            fs::write(&assumption_input, bytes).unwrap();
            let _ = parse_environment_assumptions(&assumption_input);
        }

        let config_seeds = corpus_files(&root.join("tests/fuzz-corpus/project-config"));
        let config_input = scratch.join("mutated.conf");
        for iteration in 0..5_000 {
            let bytes = fuzz_mutation(
                &config_seeds[iteration % config_seeds.len()],
                iteration ^ 0x3c3c,
            );
            fs::write(&config_input, bytes).unwrap();
            let _ = parse_rtl_project_config(&config_input);
        }

        let cli_seeds = corpus_files(&root.join("tests/fuzz-corpus/cli"));
        let cli_artifacts = scratch.join("cli-artifacts");
        for iteration in 0..10_000 {
            let bytes = fuzz_mutation(&cli_seeds[iteration % cli_seeds.len()], iteration ^ 0xa5a5);
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines().take(32) {
                let mut args = line
                    .split('\t')
                    .take(70)
                    .map(|argument| argument.chars().take(1_024).collect::<String>())
                    .collect::<Vec<_>>();
                if args.first().map(String::as_str) == Some("firmware-safety-gate")
                    && args.len() == 4
                {
                    args[1] = scratch.join("missing.aag").to_string_lossy().to_string();
                    args[3] = cli_artifacts.to_string_lossy().to_string();
                }
                if args.first().map(String::as_str) == Some("firmware-cli-version")
                    && args.len() == 1
                {
                    args.push("fuzz-extra-argument".to_string());
                }
                let _ = run_firmware_gate_cli(&args);
            }
        }
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn firmware_cli_contract_version_is_machine_readable_and_strict() {
        assert_eq!(
            run_firmware_gate_cli(&["firmware-cli-version".to_string()]).unwrap(),
            Some(true)
        );
        assert!(
            run_firmware_gate_cli(&["firmware-cli-version".to_string(), "unexpected".to_string(),])
                .unwrap_err()
                .contains("usage:")
        );
    }

    #[cfg(unix)]
    #[test]
    fn contained_process_kills_descendants() {
        let limited_file = std::env::temp_dir().join(format!(
            "cq-sat-contained-output-{}.bin",
            std::process::id()
        ));
        let mut file_command = Command::new("sh");
        file_command
            .arg("-c")
            .arg("dd if=/dev/zero of=\"$1\" bs=2048 count=1 2>/dev/null")
            .arg("cq-sat-containment-test")
            .arg(&limited_file)
            .stderr(Stdio::null());
        configure_contained_process(&mut file_command, YOSYS_MEMORY_LIMIT_BYTES, 1024).unwrap();
        let mut file_child = file_command.spawn().unwrap();
        let file_status = wait_for_contained_process(
            &mut file_child,
            std::time::Duration::from_secs(5),
            "file-limit probe",
        )
        .unwrap()
        .expect("file-limit probe timed out");
        assert!(!file_status.success());
        assert!(fs::metadata(&limited_file).unwrap().len() <= 1024);
        std::fs::remove_file(limited_file).unwrap();

        let pid_file = std::env::temp_dir().join(format!(
            "cq-sat-contained-descendant-{}.pid",
            std::process::id()
        ));
        let mut tree_command = Command::new("sh");
        tree_command
            .arg("-c")
            .arg("sleep 30 & child=$!; printf '%s' \"$child\" > \"$1\"; wait")
            .arg("cq-sat-containment-test")
            .arg(&pid_file);
        configure_contained_process(
            &mut tree_command,
            YOSYS_MEMORY_LIMIT_BYTES,
            YOSYS_FILE_LIMIT_BYTES,
        )
        .unwrap();
        let mut tree_child = tree_command.spawn().unwrap();
        assert!(
            wait_for_contained_process(
                &mut tree_child,
                std::time::Duration::from_millis(250),
                "process-tree probe",
            )
            .unwrap()
            .is_none()
        );
        let descendant = fs::read_to_string(&pid_file)
            .unwrap()
            .parse::<i32>()
            .unwrap();
        let mut gone = false;
        for _ in 0..100 {
            // SAFETY: signal zero only queries whether the recorded child PID exists.
            if unsafe { libc::kill(descendant, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                gone = true;
                break;
            }
            thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(gone, "contained descendant survived process-group timeout");
        std::fs::remove_file(pid_file).unwrap();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn contained_process_enforces_address_space_limit() {
        let mut memory_command = Command::new("python3");
        memory_command
            .arg("-c")
            .arg("payload = bytearray(768 * 1024 * 1024); print(len(payload))");
        configure_contained_process(&mut memory_command, 512 * 1024 * 1024, 1024 * 1024).unwrap();
        let mut memory_child = memory_command.spawn().unwrap();
        let memory_status = wait_for_contained_process(
            &mut memory_child,
            std::time::Duration::from_secs(10),
            "memory-limit probe",
        )
        .unwrap()
        .expect("memory-limit probe timed out");
        assert!(!memory_status.success());
    }
}
