use guarded_continuation_checker::{
    compiled_mmio_branching_dag::{
        build_compiled_mmio_branching_dag, build_compiled_mmio_trace_family,
        encode_compiled_mmio_branching_dag, projected_compiled_mmio_trace_family_size,
        verify_compiled_mmio_branching_dag_bytes, verify_compiled_mmio_trace_family,
    },
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
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
    if arguments.len() != 5 {
        return Err(
            "usage: branching_mmio_dag_baseline FIRMWARE_BIN SYMBOLS_TXT DAG_OUT EXPLICIT_OUT"
                .to_string(),
        );
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let dag =
        build_compiled_mmio_branching_dag(&image, symbols).map_err(|error| error.to_string())?;
    let dag_bytes = encode_compiled_mmio_branching_dag(&dag).map_err(|error| error.to_string())?;
    let dag_verification = verify_compiled_mmio_branching_dag_bytes(&dag_bytes, &image, symbols)
        .map_err(|error| error.to_string())?;
    let trace_family = build_compiled_mmio_trace_family(&dag).map_err(|error| error.to_string())?;
    let trace_verification = verify_compiled_mmio_trace_family(&trace_family, &image, symbols)
        .map_err(|error| error.to_string())?;
    let trace_family_bytes = projected_compiled_mmio_trace_family_size(&trace_family)
        .map_err(|error| error.to_string())?;

    let explicit = build_explicit_compiled_mmio_transcript(&image, symbols)
        .map_err(|error| error.to_string())?;
    let explicit_bytes =
        encode_explicit_compiled_mmio_transcript(&explicit).map_err(|error| error.to_string())?;
    let explicit_verification =
        verify_explicit_compiled_mmio_transcript(&explicit_bytes, &image, symbols)
            .map_err(|error| error.to_string())?;

    for execution in &explicit.executions {
        let terminal = &dag.terminals
            [usize::from(dag.terminal_indices[usize::from(execution.input)])]
        .execution;
        if execution.execution != *terminal {
            return Err(format!("semantic mismatch at input {}", execution.input));
        }
    }
    fs::write(&arguments[3], &dag_bytes).map_err(|error| error.to_string())?;
    fs::write(&arguments[4], &explicit_bytes).map_err(|error| error.to_string())?;

    println!("branching_mmio_dag_baseline_version=1");
    println!("input_count=256");
    println!("semantic_agreement=true");
    println!("dag_artifact_bytes={}", dag_bytes.len());
    println!("dag_artifact_sha256={}", hex_digest(&dag_bytes));
    println!("dag_unique_nodes={}", dag.nodes.len());
    println!("dag_terminal_count={}", dag.terminals.len());
    println!(
        "dag_decoded_transitions={}",
        dag_verification.decoded_transitions
    );
    println!("dag_scalar_path_steps={}", dag.scalar_path_steps);
    println!("trace_family_count={}", trace_family.traces.len());
    println!("trace_family_projected_bytes={trace_family_bytes}");
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
        "artifact_reduction_times={:.6}",
        explicit_bytes.len() as f64 / dag_bytes.len() as f64
    );
    println!(
        "decoded_transition_reduction_times={:.6}",
        explicit_verification.decoded_instruction_transitions as f64
            / dag_verification.decoded_transitions as f64
    );
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("branching MMIO DAG baseline failed: {error}");
            ExitCode::FAILURE
        }
    }
}
