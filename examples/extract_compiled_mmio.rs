use guarded_continuation_checker::riscv32imc::{Rv32SymbolLayout, execute_compiled_mmio};
use std::{collections::BTreeMap, env, fs, process::ExitCode};

fn symbol(contents: &str, name: &str) -> Result<u32, String> {
    let mut matches = contents.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let address = fields.next()?;
        let _kind = fields.next()?;
        (fields.next()? == name).then_some(address)
    });
    let address = matches
        .next()
        .ok_or_else(|| format!("missing symbol {name}"))?;
    if matches.next().is_some() {
        return Err(format!("duplicate symbol {name}"));
    }
    u32::from_str_radix(address, 16).map_err(|_| format!("invalid address for symbol {name}"))
}

fn symbol_table(contents: &str) -> Result<BTreeMap<u32, String>, String> {
    let mut symbols = BTreeMap::new();
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(address) = fields.next() else {
            continue;
        };
        let Some(_kind) = fields.next() else {
            continue;
        };
        let Some(name) = fields.next() else {
            continue;
        };
        let address = u32::from_str_radix(address, 16)
            .map_err(|_| format!("invalid address for symbol {name}"))?;
        symbols.insert(address, name.to_string());
    }
    Ok(symbols)
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        return Err("usage: extract_compiled_mmio FIRMWARE_BIN SYMBOLS_TXT".to_string());
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbols = fs::read_to_string(&arguments[2]).map_err(|error| error.to_string())?;
    let table = symbol_table(&symbols)?;
    let execution = execute_compiled_mmio(
        &image,
        Rv32SymbolLayout {
            entry: symbol(&symbols, "gcc_firmware_entry")?,
            event_count: symbol(&symbols, "gcc_mmio_event_count")?,
            events: symbol(&symbols, "gcc_mmio_events")?,
        },
    )
    .map_err(|error| error.to_string())?;
    println!("firmware_result={}", execution.return_value);
    println!("instruction_count={}", execution.steps);
    println!("event_count={}", execution.events.len());
    for (index, event) in execution.events.iter().enumerate() {
        let location = execution.event_program_locations[index];
        let source = table
            .range(..=location)
            .next_back()
            .map(|(_, name)| name.as_str())
            .unwrap_or("unknown");
        println!(
            "event={index},{},{},{},0x{location:08x},{source}",
            event.operation, event.offset, event.value,
        );
    }
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("compiled MMIO extraction failed: {error}");
            ExitCode::FAILURE
        }
    }
}
