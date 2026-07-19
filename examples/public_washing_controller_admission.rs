use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_transducer::{
    encode_controller_transducer, produce_controller_transducer, verify_controller_transducer,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

const SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");

fn declared_input(pattern: usize) -> u64 {
    (0..11).fold(0u64, |value, bit| {
        value | (u64::from(pattern >> bit & 1 == 1) << (bit + 1))
    })
}

fn projected(outputs: u128) -> u128 {
    [2usize, 6, 7, 9]
        .iter()
        .enumerate()
        .fold(0, |value, (bit, output)| {
            value | (((outputs >> output) & 1) << bit)
        })
}

fn reachable_states(
    model: &guarded_continuation_checker::aiger_obligation::AigerTransition,
) -> Vec<usize> {
    let mut reached = BTreeSet::from([0usize]);
    loop {
        let before = reached.len();
        for state in reached.iter().copied().collect::<Vec<_>>() {
            for pattern in 0..(1usize << 11) {
                reached.insert(model.evaluate(state, declared_input(pattern)).unwrap().0);
            }
        }
        if reached.len() == before {
            return reached.into_iter().collect();
        }
    }
}

fn canonical_cells(signatures: &[Vec<(usize, u128)>], bit: usize, patterns: &[usize]) -> usize {
    let representative = patterns[0];
    if patterns
        .iter()
        .all(|&pattern| signatures[pattern] == signatures[representative])
    {
        return 1;
    }
    [false, true]
        .iter()
        .map(|&value| {
            let branch = patterns
                .iter()
                .copied()
                .filter(|pattern| (pattern >> bit & 1 == 1) == value)
                .collect::<Vec<_>>();
            canonical_cells(signatures, bit + 1, &branch)
        })
        .sum()
}

fn state_cells(
    signatures: &[Vec<(usize, u128)>],
    state_index: usize,
    bit: usize,
    patterns: &[usize],
) -> usize {
    let representative = patterns[0];
    if patterns
        .iter()
        .all(|&pattern| signatures[pattern][state_index] == signatures[representative][state_index])
    {
        return 1;
    }
    [false, true]
        .iter()
        .map(|&value| {
            let branch = patterns
                .iter()
                .copied()
                .filter(|pattern| (pattern >> bit & 1 == 1) == value)
                .collect::<Vec<_>>();
            state_cells(signatures, state_index, bit + 1, &branch)
        })
        .sum()
}

fn mtbdd_size(
    model: &guarded_continuation_checker::aiger_obligation::AigerTransition,
    order: &[usize],
) -> (usize, usize) {
    let mut terminal_ids = BTreeMap::new();
    let mut next_id = 0usize;
    let mut layer = (0..(1usize << order.len()))
        .map(|assignment| {
            let mut state = 0usize;
            let mut input_pattern = 0usize;
            for (position, &variable) in order.iter().enumerate() {
                let value = assignment >> (order.len() - position - 1) & 1;
                if variable < 6 {
                    state |= value << variable;
                } else {
                    input_pattern |= value << (variable - 6);
                }
            }
            let (target, outputs) = model
                .evaluate(state, declared_input(input_pattern))
                .unwrap();
            let outcome = (target as u128) | (projected(outputs) << 6);
            *terminal_ids.entry(outcome).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            })
        })
        .collect::<Vec<_>>();
    let terminals = next_id;
    let mut unique = BTreeMap::new();
    for level in (0..order.len()).rev() {
        layer = layer
            .chunks_exact(2)
            .map(|pair| {
                if pair[0] == pair[1] {
                    pair[0]
                } else {
                    *unique.entry((level, pair[0], pair[1])).or_insert_with(|| {
                        let id = next_id;
                        next_id += 1;
                        id
                    })
                }
            })
            .collect();
    }
    (terminals, unique.len())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = parse_ascii_aiger_transition(MODEL)?;
    let source_sha256: [u8; 32] = Sha256::digest(SOURCE).into();
    let relevant_inputs = (1..12).collect::<Vec<_>>();
    let observed_outputs = [2, 6, 7, 9];
    let reachable = reachable_states(&model);
    let signatures = (0..(1usize << 11))
        .map(|pattern| {
            reachable
                .iter()
                .map(|&state| {
                    let (target, outputs) = model.evaluate(state, declared_input(pattern)).unwrap();
                    (target, projected(outputs))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let patterns = (0..(1usize << 11)).collect::<Vec<_>>();
    let distinct_signatures = signatures.iter().collect::<BTreeSet<_>>().len();
    let canonical_cell_count = canonical_cells(&signatures, 0, &patterns);
    let state_conditioned_cells = (0..reachable.len())
        .map(|state| state_cells(&signatures, state, 0, &patterns))
        .sum::<usize>();
    let state_first = mtbdd_size(&model, &(0..17).collect::<Vec<_>>());
    let input_first = mtbdd_size(&model, &(6..17).chain(0..6).collect::<Vec<_>>());
    let interleaved_order = [6, 0, 7, 1, 8, 2, 9, 3, 10, 4, 11, 5, 12, 13, 14, 15, 16];
    let interleaved = mtbdd_size(&model, &interleaved_order);
    let source_hex = source_sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    println!(
        "schema_version,source_sha256,inputs,latches,outputs,ands,reachable_states,distinct_signatures,canonical_cells,state_conditioned_cells,state_first_mtbdd_nodes,input_first_mtbdd_nodes,interleaved_mtbdd_nodes,rows,proof_bytes,artifact_bytes,production_nanos,verification_nanos,status"
    );
    let started = Instant::now();
    let obligation = match produce_controller_transducer(
        &model,
        source_sha256,
        &relevant_inputs,
        &observed_outputs,
    ) {
        Ok(obligation) => obligation,
        Err(_) => {
            println!(
                "1,{source_hex},11,{},{},{},{},{},{canonical_cell_count},{state_conditioned_cells},{},{},{},0,0,0,0,0,rejected-cell-limit",
                model.latches.len(),
                observed_outputs.len(),
                model.ands.len(),
                reachable.len(),
                distinct_signatures,
                state_first.1,
                input_first.1,
                interleaved.1,
            );
            return Ok(());
        }
    };
    let production_nanos = started.elapsed().as_nanos();
    let encoded = encode_controller_transducer(&obligation)?;
    let started = Instant::now();
    let summary = verify_controller_transducer(&model, source_sha256, &obligation)?;
    let verification_nanos = started.elapsed().as_nanos();
    println!(
        "1,{source_hex},11,{},{},{},{},{},{canonical_cell_count},{state_conditioned_cells},{},{},{},{},{},{},{production_nanos},{verification_nanos},ok",
        model.latches.len(),
        observed_outputs.len(),
        model.ands.len(),
        reachable.len(),
        distinct_signatures,
        state_first.1,
        input_first.1,
        interleaved.1,
        summary.rows,
        summary.proof_bytes,
        encoded.len(),
    );
    Ok(())
}
