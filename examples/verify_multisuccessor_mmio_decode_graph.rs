use guarded_continuation_checker::{
    compiled_mmio_branching_dag::{
        verify_compiled_mmio_branching_dag_bytes, verify_compiled_mmio_trace_family_bytes,
    },
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_decode_graph::{
        decode_compiled_mmio_decode_graph, verify_compiled_mmio_decode_graph_bytes,
        verify_compiled_mmio_decode_graph_bytes_btree_baseline,
        verify_compiled_mmio_decode_graph_successor_index,
    },
    compiled_mmio_explicit_transcript::verify_explicit_compiled_mmio_transcript,
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err(
            "usage: verify_multisuccessor_mmio_decode_graph ROUTE ARTIFACT FIRMWARE_BIN SYMBOLS_TXT"
                .to_string(),
        );
    }
    let route = arguments[1]
        .to_str()
        .ok_or_else(|| "route is not UTF-8".to_string())?;
    let artifact = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let image = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[4]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let (decoded, scalar_steps) = match route {
        "graph" => {
            let verified = verify_compiled_mmio_decode_graph_bytes(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            println!("graph_edges={}", verified.graph_edges);
            (
                verified.unique_instruction_decodes,
                verified.scalar_path_steps,
            )
        }
        "graph-btree" => {
            let verified =
                verify_compiled_mmio_decode_graph_bytes_btree_baseline(&artifact, &image, symbols)
                    .map_err(|error| error.to_string())?;
            println!("graph_edges={}", verified.graph_edges);
            (
                verified.unique_instruction_decodes,
                verified.scalar_path_steps,
            )
        }
        "graph-successor" => {
            let graph =
                decode_compiled_mmio_decode_graph(&artifact).map_err(|error| error.to_string())?;
            let verified =
                verify_compiled_mmio_decode_graph_successor_index(&graph, &image, symbols)
                    .map_err(|error| error.to_string())?;
            println!("graph_edges={}", verified.graph_edges);
            (
                verified.unique_instruction_decodes,
                verified.scalar_path_steps,
            )
        }
        "dag" => {
            let verified = verify_compiled_mmio_branching_dag_bytes(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            (verified.decoded_transitions, verified.scalar_path_steps)
        }
        "trace-family" => {
            let verified = verify_compiled_mmio_trace_family_bytes(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            (verified.decoded_transitions, verified.scalar_path_steps)
        }
        "explicit" => {
            let verified = verify_explicit_compiled_mmio_transcript(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            (
                verified.decoded_instruction_transitions,
                verified.decoded_instruction_transitions,
            )
        }
        _ => {
            return Err(
                "route must be graph, graph-btree, graph-successor, dag, trace-family or explicit"
                    .to_string(),
            );
        }
    };
    println!("route={route}");
    println!("decoded_transitions={decoded}");
    println!("scalar_path_steps={scalar_steps}");
    println!("artifact_bytes={}", artifact.len());
    println!("input_count=256");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("multi-successor decode graph consumer failed: {error}");
            ExitCode::FAILURE
        }
    }
}
