use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_explicit_transcript::{
        build_explicit_compiled_mmio_transcript, encode_explicit_compiled_mmio_transcript,
        verify_explicit_compiled_mmio_transcript,
    },
    compiled_mmio_predicate_certificate::{
        certify_compiled_mmio_predicate, encode_compiled_mmio_predicate_certificate,
        verify_compiled_mmio_predicate_bytes,
    },
};
use sha2::{Digest, Sha256};
use std::{env, fs, process::ExitCode};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err(
            "usage: explicit_mmio_transcript_baseline FIRMWARE_BIN SYMBOLS_TXT PREDICATE_OUT EXPLICIT_OUT"
                .to_string(),
        );
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let predicate =
        certify_compiled_mmio_predicate(&image, symbols).map_err(|error| error.to_string())?;
    let predicate_bytes = encode_compiled_mmio_predicate_certificate(&predicate)
        .map_err(|error| error.to_string())?;
    let predicate_verification =
        verify_compiled_mmio_predicate_bytes(&predicate_bytes, &image, symbols)
            .map_err(|error| error.to_string())?;

    let explicit = build_explicit_compiled_mmio_transcript(&image, symbols)
        .map_err(|error| error.to_string())?;
    let explicit_bytes =
        encode_explicit_compiled_mmio_transcript(&explicit).map_err(|error| error.to_string())?;
    let explicit_verification =
        verify_explicit_compiled_mmio_transcript(&explicit_bytes, &image, symbols)
            .map_err(|error| error.to_string())?;
    fs::write(&arguments[3], &predicate_bytes).map_err(|error| error.to_string())?;
    fs::write(&arguments[4], &explicit_bytes).map_err(|error| error.to_string())?;

    println!("explicit_transcript_baseline_version=1");
    println!("input_count={}", explicit.executions.len());
    println!("predicate_artifact_bytes={}", predicate_bytes.len());
    println!("predicate_artifact_sha256={}", digest(&predicate_bytes));
    println!(
        "predicate_producer_decoded_transitions={}",
        predicate_verification.producer_decoded_transitions
    );
    println!(
        "predicate_verifier_decoded_transitions={}",
        predicate_verification.verifier_decoded_transitions
    );
    println!(
        "predicate_lane_value_operations={}",
        predicate_verification.verifier_lane_value_operations
    );
    println!("explicit_artifact_bytes={}", explicit_bytes.len());
    println!("explicit_artifact_sha256={}", digest(&explicit_bytes));
    println!(
        "explicit_producer_decoded_transitions={}",
        explicit.decoded_instruction_transitions
    );
    println!(
        "explicit_verifier_decoded_transitions={}",
        explicit_verification.decoded_instruction_transitions
    );
    println!(
        "artifact_reduction_times={:.6}",
        explicit_bytes.len() as f64 / predicate_bytes.len() as f64
    );
    println!(
        "verifier_transition_reduction_times={:.6}",
        explicit_verification.decoded_instruction_transitions as f64
            / predicate_verification.verifier_decoded_transitions as f64
    );
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("explicit MMIO transcript baseline failed: {error}");
            ExitCode::FAILURE
        }
    }
}
