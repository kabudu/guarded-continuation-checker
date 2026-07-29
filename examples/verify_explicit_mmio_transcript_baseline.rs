use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_explicit_transcript::verify_explicit_compiled_mmio_transcript,
    compiled_mmio_predicate_certificate::verify_compiled_mmio_predicate_bytes,
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err(
            "usage: verify_explicit_mmio_transcript_baseline ROUTE ARTIFACT FIRMWARE_BIN SYMBOLS_TXT"
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
        "predicate" => {
            let result = verify_compiled_mmio_predicate_bytes(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            println!("route=predicate");
            println!("artifact_bytes={}", result.artifact_bytes);
            println!(
                "decoded_instruction_transitions={}",
                result.verifier_decoded_transitions
            );
        }
        "explicit" => {
            let result = verify_explicit_compiled_mmio_transcript(&artifact, &image, symbols)
                .map_err(|error| error.to_string())?;
            println!("route=explicit");
            println!("artifact_bytes={}", result.artifact_bytes);
            println!(
                "decoded_instruction_transitions={}",
                result.decoded_instruction_transitions
            );
        }
        _ => return Err("route must be predicate or explicit".to_string()),
    }
    println!("input_count=256");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("explicit transcript consumer failed: {error}");
            ExitCode::FAILURE
        }
    }
}
