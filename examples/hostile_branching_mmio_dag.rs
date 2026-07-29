use guarded_continuation_checker::{
    compiled_mmio_branching_dag::{
        decode_compiled_mmio_branching_dag, verify_compiled_mmio_branching_dag_bytes,
    },
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
};
use std::{env, fs, process::ExitCode, time::Instant};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(
            "usage: hostile_branching_mmio_dag ARTIFACT FIRMWARE_BIN SYMBOLS_TXT".to_string(),
        );
    }
    let mut bytes = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let image = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;
    verify_compiled_mmio_branching_dag_bytes(&bytes, &image, symbols)
        .map_err(|error| error.to_string())?;

    let mutation_start = Instant::now();
    for index in 0..bytes.len() {
        bytes[index] ^= 1;
        if decode_compiled_mmio_branching_dag(&bytes).is_ok() {
            return Err(format!("accepted mutation at byte {index}"));
        }
        bytes[index] ^= 1;
    }
    let mutation_millis = mutation_start.elapsed().as_millis();

    let truncation_start = Instant::now();
    for length in 0..bytes.len() {
        if decode_compiled_mmio_branching_dag(&bytes[..length]).is_ok() {
            return Err(format!("accepted truncation at length {length}"));
        }
    }
    let truncation_millis = truncation_start.elapsed().as_millis();

    let mut extended = bytes.clone();
    extended.push(0);
    if decode_compiled_mmio_branching_dag(&extended).is_ok() {
        return Err("accepted extension".to_string());
    }
    let mut changed_image = image.clone();
    changed_image[0] ^= 1;
    if verify_compiled_mmio_branching_dag_bytes(&bytes, &changed_image, symbols).is_ok() {
        return Err("accepted image drift".to_string());
    }
    let mut changed_symbols = symbols;
    changed_symbols.entry ^= 2;
    if verify_compiled_mmio_branching_dag_bytes(&bytes, &image, changed_symbols).is_ok() {
        return Err("accepted symbol drift".to_string());
    }

    println!("branching_dag_hostile_version=1");
    println!("artifact_bytes={}", bytes.len());
    println!("single_bit_mutations={}", bytes.len());
    println!("mutation_millis={mutation_millis}");
    println!("truncations={}", bytes.len());
    println!("truncation_millis={truncation_millis}");
    println!("hostile_cases={}", bytes.len() * 2 + 3);
    println!("extension=refused");
    println!("image_drift=refused");
    println!("symbol_drift=refused");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("branching DAG hostile qualification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
