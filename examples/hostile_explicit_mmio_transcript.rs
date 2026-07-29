use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_explicit_transcript::{
        decode_explicit_compiled_mmio_transcript, verify_explicit_compiled_mmio_transcript,
    },
};
use std::{
    env, fs,
    process::ExitCode,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Instant,
};

fn exhaustive_mutations(bytes: Arc<Vec<u8>>, workers: usize) -> Result<usize, String> {
    let next = Arc::new(AtomicUsize::new(0));
    let failure = Arc::new(Mutex::new(None));
    thread::scope(|scope| {
        for _ in 0..workers {
            let bytes = Arc::clone(&bytes);
            let next = Arc::clone(&next);
            let failure = Arc::clone(&failure);
            scope.spawn(move || {
                let mut changed = bytes.as_ref().clone();
                loop {
                    if failure.lock().expect("failure mutex poisoned").is_some() {
                        break;
                    }
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= changed.len() {
                        break;
                    }
                    changed[index] ^= 1;
                    if decode_explicit_compiled_mmio_transcript(&changed).is_ok() {
                        *failure.lock().expect("failure mutex poisoned") =
                            Some(format!("accepted mutation at byte {index}"));
                        break;
                    }
                    changed[index] ^= 1;
                }
            });
        }
    });
    if let Some(message) = failure.lock().map_err(|_| "failure mutex poisoned")?.take() {
        return Err(message);
    }
    Ok(bytes.len())
}

fn exhaustive_truncations(bytes: Arc<Vec<u8>>, workers: usize) -> Result<usize, String> {
    let next = Arc::new(AtomicUsize::new(0));
    let failure = Arc::new(Mutex::new(None));
    thread::scope(|scope| {
        for _ in 0..workers {
            let bytes = Arc::clone(&bytes);
            let next = Arc::clone(&next);
            let failure = Arc::clone(&failure);
            scope.spawn(move || {
                loop {
                    if failure.lock().expect("failure mutex poisoned").is_some() {
                        break;
                    }
                    let length = next.fetch_add(1, Ordering::Relaxed);
                    if length >= bytes.len() {
                        break;
                    }
                    if decode_explicit_compiled_mmio_transcript(&bytes[..length]).is_ok() {
                        *failure.lock().expect("failure mutex poisoned") =
                            Some(format!("accepted truncation at length {length}"));
                        break;
                    }
                }
            });
        }
    });
    if let Some(message) = failure.lock().map_err(|_| "failure mutex poisoned")?.take() {
        return Err(message);
    }
    Ok(bytes.len())
}

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err(
            "usage: hostile_explicit_mmio_transcript ARTIFACT FIRMWARE_BIN SYMBOLS_TXT WORKERS"
                .to_string(),
        );
    }
    let bytes = Arc::new(fs::read(&arguments[1]).map_err(|error| error.to_string())?);
    let image = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;
    let workers = arguments[4]
        .to_str()
        .ok_or_else(|| "worker count is not UTF-8".to_string())?
        .parse::<usize>()
        .map_err(|_| "worker count is not an integer".to_string())?;
    if !(1..=64).contains(&workers) {
        return Err("worker count must be between 1 and 64".to_string());
    }

    verify_explicit_compiled_mmio_transcript(&bytes, &image, symbols)
        .map_err(|error| error.to_string())?;

    let mutation_start = Instant::now();
    let mutations = exhaustive_mutations(Arc::clone(&bytes), workers)?;
    let mutation_millis = mutation_start.elapsed().as_millis();

    let truncation_start = Instant::now();
    let truncations = exhaustive_truncations(Arc::clone(&bytes), workers)?;
    let truncation_millis = truncation_start.elapsed().as_millis();

    let mut extended = bytes.as_ref().clone();
    extended.push(0);
    if decode_explicit_compiled_mmio_transcript(&extended).is_ok() {
        return Err("accepted one-byte extension".to_string());
    }
    let mut changed_image = image.clone();
    changed_image[0] ^= 1;
    if verify_explicit_compiled_mmio_transcript(&bytes, &changed_image, symbols).is_ok() {
        return Err("accepted firmware image drift".to_string());
    }
    let mut changed_symbols = symbols;
    changed_symbols.entry ^= 2;
    if verify_explicit_compiled_mmio_transcript(&bytes, &image, changed_symbols).is_ok() {
        return Err("accepted symbol drift".to_string());
    }

    println!("explicit_transcript_hostile_version=1");
    println!("artifact_bytes={}", bytes.len());
    println!("workers={workers}");
    println!("single_bit_mutations={mutations}");
    println!("mutation_millis={mutation_millis}");
    println!("truncations={truncations}");
    println!("truncation_millis={truncation_millis}");
    println!(
        "hostile_cases={}",
        mutations
            .checked_add(truncations)
            .and_then(|count| count.checked_add(3))
            .ok_or_else(|| "hostile case count overflow".to_string())?
    );
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
            eprintln!("hostile explicit transcript qualification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
