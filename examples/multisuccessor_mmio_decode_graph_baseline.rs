use guarded_continuation_checker::{
    compiled_mmio_branching_dag::{
        build_compiled_mmio_branching_dag, build_compiled_mmio_trace_family,
        encode_compiled_mmio_branching_dag, encode_compiled_mmio_trace_family,
        verify_compiled_mmio_branching_dag_bytes, verify_compiled_mmio_trace_family_bytes,
    },
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_decode_graph::{
        build_compiled_mmio_decode_graph, encode_compiled_mmio_decode_graph,
        verify_compiled_mmio_decode_graph_bytes,
    },
    compiled_mmio_explicit_transcript::{
        build_explicit_compiled_mmio_transcript, encode_explicit_compiled_mmio_transcript,
        verify_explicit_compiled_mmio_transcript,
    },
};
use sha2::{Digest, Sha256};
use std::{env, fs, process::ExitCode};

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 7 {
        return Err(
            "usage: multisuccessor_mmio_decode_graph_baseline FIRMWARE_BIN SYMBOLS_TXT GRAPH_OUT DAG_OUT TRACE_FAMILY_OUT EXPLICIT_OUT".to_string(),
        );
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let graph =
        build_compiled_mmio_decode_graph(&image, symbols).map_err(|error| error.to_string())?;
    let graph_bytes =
        encode_compiled_mmio_decode_graph(&graph).map_err(|error| error.to_string())?;
    let graph_verification = verify_compiled_mmio_decode_graph_bytes(&graph_bytes, &image, symbols)
        .map_err(|error| error.to_string())?;

    let dag =
        build_compiled_mmio_branching_dag(&image, symbols).map_err(|error| error.to_string())?;
    let dag_bytes = encode_compiled_mmio_branching_dag(&dag).map_err(|error| error.to_string())?;
    let dag_verification = verify_compiled_mmio_branching_dag_bytes(&dag_bytes, &image, symbols)
        .map_err(|error| error.to_string())?;
    let trace_family = build_compiled_mmio_trace_family(&dag).map_err(|error| error.to_string())?;
    let trace_family_bytes =
        encode_compiled_mmio_trace_family(&trace_family).map_err(|error| error.to_string())?;
    let trace_verification =
        verify_compiled_mmio_trace_family_bytes(&trace_family_bytes, &image, symbols)
            .map_err(|error| error.to_string())?;

    let explicit = build_explicit_compiled_mmio_transcript(&image, symbols)
        .map_err(|error| error.to_string())?;
    let explicit_bytes =
        encode_explicit_compiled_mmio_transcript(&explicit).map_err(|error| error.to_string())?;
    let explicit_verification =
        verify_explicit_compiled_mmio_transcript(&explicit_bytes, &image, symbols)
            .map_err(|error| error.to_string())?;

    for execution in &explicit.executions {
        let input = usize::from(execution.input);
        let graph_terminal = &graph.terminals[usize::from(graph.terminal_indices[input])].execution;
        let dag_terminal = &dag.terminals[usize::from(dag.terminal_indices[input])].execution;
        if execution.execution != *graph_terminal || execution.execution != *dag_terminal {
            return Err(format!("semantic mismatch at input {}", execution.input));
        }
    }

    fs::write(&arguments[3], &graph_bytes).map_err(|error| error.to_string())?;
    fs::write(&arguments[4], &dag_bytes).map_err(|error| error.to_string())?;
    fs::write(&arguments[5], &trace_family_bytes).map_err(|error| error.to_string())?;
    fs::write(&arguments[6], &explicit_bytes).map_err(|error| error.to_string())?;

    println!("multisuccessor_mmio_decode_graph_baseline_version=2");
    println!("input_count=256");
    println!("semantic_agreement=true");
    println!("graph_artifact_bytes={}", graph_bytes.len());
    println!("graph_artifact_sha256={}", hex_digest(&graph_bytes));
    println!("graph_unique_nodes={}", graph.nodes.len());
    println!("graph_edges={}", graph_verification.graph_edges);
    println!(
        "graph_unique_instruction_decodes={}",
        graph_verification.unique_instruction_decodes
    );
    println!("graph_scalar_path_steps={}", graph.scalar_path_steps);
    println!("dag_artifact_bytes={}", dag_bytes.len());
    println!("dag_artifact_sha256={}", hex_digest(&dag_bytes));
    println!("dag_unique_nodes={}", dag.nodes.len());
    println!(
        "dag_decoded_transitions={}",
        dag_verification.decoded_transitions
    );
    println!("trace_family_count={}", trace_family.traces.len());
    println!("trace_family_artifact_bytes={}", trace_family_bytes.len());
    println!(
        "trace_family_artifact_sha256={}",
        hex_digest(&trace_family_bytes)
    );
    println!(
        "trace_family_decoded_transitions={}",
        trace_verification.decoded_transitions
    );
    println!("explicit_artifact_bytes={}", explicit_bytes.len());
    println!("explicit_artifact_sha256={}", hex_digest(&explicit_bytes));
    println!(
        "explicit_decoded_transitions={}",
        explicit_verification.decoded_instruction_transitions
    );
    println!(
        "graph_to_trace_family_size_ratio={:.6}",
        graph_bytes.len() as f64 / trace_family_bytes.len() as f64
    );
    println!(
        "graph_to_trace_family_decode_ratio={:.6}",
        graph_verification.unique_instruction_decodes as f64
            / trace_verification.decoded_transitions as f64
    );
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("multi-successor MMIO decode graph baseline failed: {error}");
            ExitCode::FAILURE
        }
    }
}
