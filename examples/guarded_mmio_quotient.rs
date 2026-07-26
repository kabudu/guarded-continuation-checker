use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_quotient::{build_guarded_mmio_quotient, verify_guarded_mmio_quotient},
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        return Err("usage: guarded_mmio_quotient FIRMWARE_BIN SYMBOLS_TXT".to_string());
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;
    let quotient =
        build_guarded_mmio_quotient(&image, symbols).map_err(|error| error.to_string())?;
    let verification = verify_guarded_mmio_quotient(&quotient, &image, symbols)
        .map_err(|error| error.to_string())?;

    println!("guarded_mmio_quotient_version={}", quotient.version);
    println!("valid_class_count={}", quotient.valid_behaviors.len());
    println!(
        "invalid_class_members={}",
        quotient.invalid_prefix_steps.len()
    );
    println!("invalid_prefix_steps={}", quotient.invalid_prefix_steps[0]);
    println!(
        "shared_continuation_steps={}",
        quotient.shared_continuation_steps
    );
    println!(
        "producer_decoded_instruction_transitions={}",
        quotient.producer_decoded_instruction_transitions
    );
    println!(
        "verifier_decoded_instruction_transitions={}",
        verification.decoded_instruction_transitions
    );
    println!(
        "quotient_cycle_decoded_instruction_transitions={}",
        quotient.producer_decoded_instruction_transitions
            + verification.decoded_instruction_transitions
    );
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("guarded MMIO quotient failed: {error}");
            ExitCode::FAILURE
        }
    }
}
