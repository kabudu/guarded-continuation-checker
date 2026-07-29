use guarded_continuation_checker::{
    compiled_mmio_branching_dag::verify_compiled_mmio_branching_dag_bytes,
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_explicit_transcript::verify_explicit_compiled_mmio_transcript,
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err(
            "usage: verify_branching_mmio_dag_baseline ROUTE ARTIFACT FIRMWARE_BIN SYMBOLS_TXT"
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
    match route {
        "dag" => {
            let verified = verify_compiled_mmio_branching_dag_bytes(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            println!("route=dag");
            println!("decoded_transitions={}", verified.decoded_transitions);
            println!("scalar_path_steps={}", verified.scalar_path_steps);
        }
        "explicit" => {
            let verified = verify_explicit_compiled_mmio_transcript(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            println!("route=explicit");
            println!(
                "decoded_transitions={}",
                verified.decoded_instruction_transitions
            );
            println!(
                "scalar_path_steps={}",
                verified.decoded_instruction_transitions
            );
        }
        _ => return Err("route must be dag or explicit".to_string()),
    }
    println!("artifact_bytes={}", artifact.len());
    println!("input_count=256");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("branching DAG consumer failed: {error}");
            ExitCode::FAILURE
        }
    }
}
