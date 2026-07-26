use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_predicate_certificate::{
        certify_compiled_mmio_predicate, decode_compiled_mmio_predicate_certificate,
        encode_compiled_mmio_predicate_certificate, verify_compiled_mmio_predicate_bytes,
    },
    compiled_mmio_predicate_portfolio::{
        preflight_compiled_mmio_predicate, produce_compiled_mmio_predicate_portfolio,
        verify_compiled_mmio_predicate_portfolio,
    },
};
use std::{env, fs, process::ExitCode};

fn hostile_codec_cases(bytes: &[u8]) -> Result<u64, String> {
    let mut cases = 0u64;
    for index in 0..bytes.len() {
        let mut changed = bytes.to_vec();
        changed[index] ^= 1;
        if decode_compiled_mmio_predicate_certificate(&changed).is_ok() {
            return Err(format!("accepted mutation at artifact byte {index}"));
        }
        cases += 1;
    }
    for length in 0..bytes.len() {
        if decode_compiled_mmio_predicate_certificate(&bytes[..length]).is_ok() {
            return Err(format!("accepted truncation at artifact byte {length}"));
        }
        cases += 1;
    }
    let mut extended = bytes.to_vec();
    extended.push(0);
    if decode_compiled_mmio_predicate_certificate(&extended).is_ok() {
        return Err("accepted artifact extension".to_string());
    }
    Ok(cases + 1)
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        return Err("usage: predicate_certificate FIRMWARE_BIN SYMBOLS_TXT".to_string());
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let first = certify_compiled_mmio_predicate(&image, symbols)
        .and_then(|certificate| encode_compiled_mmio_predicate_certificate(&certificate))
        .map_err(|error| error.to_string())?;
    let second = certify_compiled_mmio_predicate(&image, symbols)
        .and_then(|certificate| encode_compiled_mmio_predicate_certificate(&certificate))
        .map_err(|error| error.to_string())?;
    if first != second {
        return Err("clean certificate cycles differ".to_string());
    }
    let hostile_cases = hostile_codec_cases(&first)?;
    let verification = verify_compiled_mmio_predicate_bytes(&first, &image, symbols)
        .map_err(|error| error.to_string())?;
    let preflight =
        preflight_compiled_mmio_predicate(&image, symbols).map_err(|error| error.to_string())?;
    let evidence = produce_compiled_mmio_predicate_portfolio(&preflight, &image, symbols)
        .map_err(|error| error.to_string())?;
    let portfolio =
        verify_compiled_mmio_predicate_portfolio(&preflight, &evidence, &image, symbols)
            .map_err(|error| error.to_string())?;

    println!("route={:?}", portfolio.route);
    println!("artifact_bytes={}", verification.artifact_bytes);
    println!(
        "verifier_decoded_transitions={}",
        verification.decoded_transitions
    );
    println!(
        "verifier_lane_value_operations={}",
        verification.lane_value_operations
    );
    println!("clean_cycles_byte_identical=true");
    println!("hostile_codec_cases={hostile_cases}");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("predicate certificate failed: {error}");
            ExitCode::FAILURE
        }
    }
}
