use guarded_continuation_checker::{
    compiled_mmio_branching_dag::verify_compiled_mmio_trace_family_bytes,
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_decode_graph::{
        decode_compiled_mmio_decode_graph, verify_compiled_mmio_decode_graph_bytes,
        verify_compiled_mmio_decode_graph_successor_index,
    },
};
use std::{env, fs, hint::black_box, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 6 {
        return Err(
            "usage: benchmark_mmio_replay_routes ROUTE ARTIFACT FIRMWARE_BIN SYMBOLS_TXT REPETITIONS"
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
    let repetitions = arguments[5]
        .to_str()
        .ok_or_else(|| "repetitions are not UTF-8".to_string())?
        .parse::<u32>()
        .map_err(|error| format!("invalid repetitions: {error}"))?;
    if repetitions == 0 || repetitions > 1_000 {
        return Err("repetitions must be between 1 and 1000".to_string());
    }

    let mut scalar_path_steps = 0u64;
    for _ in 0..repetitions {
        scalar_path_steps = match route {
            "graph" => {
                verify_compiled_mmio_decode_graph_bytes(
                    black_box(&artifact),
                    black_box(&image),
                    symbols,
                )
                .map_err(|error| error.to_string())?
                .scalar_path_steps
            }
            "graph-successor" => {
                let graph = decode_compiled_mmio_decode_graph(black_box(&artifact))
                    .map_err(|error| error.to_string())?;
                verify_compiled_mmio_decode_graph_successor_index(
                    black_box(&graph),
                    black_box(&image),
                    symbols,
                )
                .map_err(|error| error.to_string())?
                .scalar_path_steps
            }
            "trace-family" => {
                verify_compiled_mmio_trace_family_bytes(
                    black_box(&artifact),
                    black_box(&image),
                    symbols,
                )
                .map_err(|error| error.to_string())?
                .scalar_path_steps
            }
            _ => {
                return Err("route must be graph, graph-successor or trace-family".to_string());
            }
        };
        black_box(scalar_path_steps);
    }
    println!("route={route}");
    println!("repetitions={repetitions}");
    println!("scalar_path_steps={scalar_path_steps}");
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("MMIO replay benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}
