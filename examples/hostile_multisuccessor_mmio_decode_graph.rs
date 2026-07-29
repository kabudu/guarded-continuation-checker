use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_decode_graph::{
        decode_compiled_mmio_decode_graph, verify_compiled_mmio_decode_graph,
        verify_compiled_mmio_decode_graph_bytes, verify_compiled_mmio_decode_graph_successor_index,
    },
};
use std::{env, fs, process::ExitCode, time::Instant};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(
            "usage: hostile_multisuccessor_mmio_decode_graph ARTIFACT FIRMWARE_BIN SYMBOLS_TXT"
                .to_string(),
        );
    }
    let mut bytes = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let image = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;
    verify_compiled_mmio_decode_graph_bytes(&bytes, &image, symbols)
        .map_err(|error| error.to_string())?;

    let mutation_start = Instant::now();
    for index in 0..bytes.len() {
        bytes[index] ^= 1;
        if decode_compiled_mmio_decode_graph(&bytes).is_ok() {
            return Err(format!("accepted mutation at byte {index}"));
        }
        bytes[index] ^= 1;
    }
    let mutation_millis = mutation_start.elapsed().as_millis();

    let truncation_start = Instant::now();
    for length in 0..bytes.len() {
        if decode_compiled_mmio_decode_graph(&bytes[..length]).is_ok() {
            return Err(format!("accepted truncation at length {length}"));
        }
    }
    let truncation_millis = truncation_start.elapsed().as_millis();

    let mut extended = bytes.clone();
    extended.push(0);
    if decode_compiled_mmio_decode_graph(&extended).is_ok() {
        return Err("accepted extension".to_string());
    }
    let mut changed_image = image.clone();
    changed_image[0] ^= 1;
    if verify_compiled_mmio_decode_graph_bytes(&bytes, &changed_image, symbols).is_ok() {
        return Err("accepted image drift".to_string());
    }
    let mut changed_symbols = symbols;
    changed_symbols.entry ^= 2;
    if verify_compiled_mmio_decode_graph_bytes(&bytes, &image, changed_symbols).is_ok() {
        return Err("accepted symbol drift".to_string());
    }

    let graph = decode_compiled_mmio_decode_graph(&bytes).map_err(|error| error.to_string())?;
    verify_compiled_mmio_decode_graph_successor_index(&graph, &image, symbols)
        .map_err(|error| error.to_string())?;
    let branch_index = graph
        .nodes
        .iter()
        .position(|node| node.next_program_counters.len() > 1)
        .ok_or_else(|| "graph has no multi-successor node".to_string())?;

    let mut missing_edge = graph.clone();
    missing_edge.nodes[branch_index].next_program_counters.pop();
    if verify_compiled_mmio_decode_graph(&missing_edge, &image, symbols).is_ok() {
        return Err("accepted missing edge".to_string());
    }
    if verify_compiled_mmio_decode_graph_successor_index(&missing_edge, &image, symbols).is_ok() {
        return Err("successor replay accepted missing edge".to_string());
    }
    let mut additional_edge = graph.clone();
    additional_edge.nodes[branch_index]
        .next_program_counters
        .push(u32::MAX);
    if verify_compiled_mmio_decode_graph(&additional_edge, &image, symbols).is_ok() {
        return Err("accepted additional edge".to_string());
    }
    if verify_compiled_mmio_decode_graph_successor_index(&additional_edge, &image, symbols).is_ok()
    {
        return Err("successor replay accepted additional edge".to_string());
    }
    let mut duplicate_edge = graph.clone();
    let edge = duplicate_edge.nodes[branch_index].next_program_counters[0];
    duplicate_edge.nodes[branch_index]
        .next_program_counters
        .insert(0, edge);
    if verify_compiled_mmio_decode_graph(&duplicate_edge, &image, symbols).is_ok() {
        return Err("accepted duplicate edge".to_string());
    }
    if verify_compiled_mmio_decode_graph_successor_index(&duplicate_edge, &image, symbols).is_ok() {
        return Err("successor replay accepted duplicate edge".to_string());
    }
    let mut missing_node = graph.clone();
    missing_node.nodes.remove(branch_index);
    if verify_compiled_mmio_decode_graph(&missing_node, &image, symbols).is_ok() {
        return Err("accepted missing node".to_string());
    }
    if verify_compiled_mmio_decode_graph_successor_index(&missing_node, &image, symbols).is_ok() {
        return Err("successor replay accepted missing node".to_string());
    }
    let mut terminal = graph.clone();
    terminal.terminals[0].execution.return_value ^= 1;
    if verify_compiled_mmio_decode_graph(&terminal, &image, symbols).is_ok() {
        return Err("accepted terminal drift".to_string());
    }
    if verify_compiled_mmio_decode_graph_successor_index(&terminal, &image, symbols).is_ok() {
        return Err("successor replay accepted terminal drift".to_string());
    }

    println!("multisuccessor_decode_graph_hostile_version=2");
    println!("artifact_bytes={}", bytes.len());
    println!("single_bit_mutations={}", bytes.len());
    println!("mutation_millis={mutation_millis}");
    println!("truncations={}", bytes.len());
    println!("truncation_millis={truncation_millis}");
    println!("structural_mutations=5");
    println!("hostile_cases={}", bytes.len() * 2 + 8);
    println!("extension=refused");
    println!("image_drift=refused");
    println!("symbol_drift=refused");
    println!("missing_edge=refused");
    println!("additional_edge=refused");
    println!("duplicate_edge=refused");
    println!("missing_node=refused");
    println!("terminal_drift=refused");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("multi-successor decode graph hostile qualification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
