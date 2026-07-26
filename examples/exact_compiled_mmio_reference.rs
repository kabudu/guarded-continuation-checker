use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_quotient::{
        EXACT_COMPILED_MMIO_INPUTS, build_exact_compiled_mmio_reference,
        verify_exact_compiled_mmio_reference,
    },
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        return Err("usage: exact_compiled_mmio_reference FIRMWARE_BIN SYMBOLS_TXT".to_string());
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbols = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let layout = parse_compiled_mmio_symbols(&symbols).map_err(|error| error.to_string())?;
    let reference =
        build_exact_compiled_mmio_reference(&image, layout).map_err(|error| error.to_string())?;
    verify_exact_compiled_mmio_reference(&reference, &image, layout)
        .map_err(|error| error.to_string())?;

    println!(
        "exact_compiled_mmio_reference_version={}",
        reference.version
    );
    println!("input_count={EXACT_COMPILED_MMIO_INPUTS}");
    println!("class_count={}", reference.classes.len());
    println!(
        "decoded_instruction_transitions={}",
        reference.decoded_instruction_transitions
    );
    for (class_index, class) in reference.classes.iter().enumerate() {
        let members = class
            .members
            .iter()
            .map(|word| format!("{word:016x}"))
            .collect::<Vec<_>>()
            .join("");
        println!(
            "class={class_index},representative={},member_count={},return_value={},event_count={},members={members}",
            class.representative,
            class.member_count(),
            class.behavior.return_value,
            class.behavior.events.len(),
        );
        for (event_index, event) in class.behavior.events.iter().enumerate() {
            println!(
                "class_event={class_index},{event_index},{},{},{}",
                event.operation, event.offset, event.value
            );
        }
    }
    for execution in &reference.executions {
        let class = &reference.classes[usize::from(execution.class_index)];
        println!(
            "input_behavior={},{},{}",
            execution.input,
            class.behavior.return_value,
            class.behavior.events.len()
        );
        for (event_index, event) in class.behavior.events.iter().enumerate() {
            println!(
                "input_event={},{event_index},{},{},{}",
                execution.input, event.operation, event.offset, event.value
            );
        }
        let locations = execution
            .event_program_locations
            .iter()
            .map(|location| format!("{location:08x}"))
            .collect::<Vec<_>>()
            .join(":");
        println!(
            "execution={},class={},steps={},locations={locations}",
            execution.input, execution.class_index, execution.steps
        );
    }
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("exact compiled-MMIO reference failed: {error}");
            ExitCode::FAILURE
        }
    }
}
