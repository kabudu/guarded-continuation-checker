use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_quotient::{build_predicate_mmio_workflow, verify_predicate_mmio_workflow},
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        return Err("usage: predicate_transducer FIRMWARE_BIN SYMBOLS_TXT".to_string());
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;
    let workflow =
        build_predicate_mmio_workflow(&image, symbols).map_err(|error| error.to_string())?;
    let verification = verify_predicate_mmio_workflow(&workflow, &image, symbols)
        .map_err(|error| error.to_string())?;
    println!("valid_singletons={}", workflow.valid_behaviors.len());
    println!("predicate_first_input={}", workflow.invalid.first_input);
    println!("predicate_lane_count={}", workflow.invalid.lane_count);
    println!("return_value={}", workflow.invalid.return_value);
    println!("event_count={}", workflow.invalid.events.len());
    println!(
        "producer_decoded_transitions={}",
        workflow.producer_decoded_transitions
    );
    println!(
        "verifier_decoded_transitions={}",
        verification.decoded_transitions
    );
    println!(
        "workflow_cycle_decoded_transitions={}",
        workflow.producer_decoded_transitions + verification.decoded_transitions
    );
    println!(
        "producer_lane_value_operations={}",
        workflow.producer_lane_value_operations
    );
    println!(
        "verifier_lane_value_operations={}",
        verification.lane_value_operations
    );
    println!(
        "sparse_memory_bytes={}",
        workflow.invalid.sparse_memory_bytes
    );
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("predicate transducer failed: {error}");
            ExitCode::FAILURE
        }
    }
}
