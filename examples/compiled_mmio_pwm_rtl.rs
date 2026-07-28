use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_quotient::{build_predicate_mmio_workflow, verify_predicate_mmio_workflow},
    compiled_mmio_rtl_mapping::{
        PWM_RTL_MODEL_SHA256, extend_pwm_rtl_trace_one_phase_cycle, map_pwm_mmio_workflow,
        replay_pwm_rtl_trace,
    },
};
use std::{env, fs, process::ExitCode};

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err(
            "usage: compiled_mmio_pwm_rtl FIRMWARE_BIN SYMBOLS_TXT MODEL_BTOR2".to_string(),
        );
    }
    let image = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let model = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let workflow =
        build_predicate_mmio_workflow(&image, symbols).map_err(|error| error.to_string())?;
    let verification = verify_predicate_mmio_workflow(&workflow, &image, symbols)
        .map_err(|error| error.to_string())?;
    let family = map_pwm_mmio_workflow(&workflow).map_err(|error| error.to_string())?;

    println!("compiled_mmio_pwm_rtl_version={}", family.version);
    println!("rtl_model_sha256={}", hex_digest(&PWM_RTL_MODEL_SHA256));
    println!("valid_rtl_members={}", family.traces.len());
    println!("invalid_rtl_members={}", family.invalid_rtl_members);
    println!(
        "firmware_verifier_decoded_transitions={}",
        verification.decoded_transitions
    );
    println!(
        "firmware_verifier_lane_value_operations={}",
        verification.lane_value_operations
    );
    for trace in &family.traces {
        let base = replay_pwm_rtl_trace(&model, trace).map_err(|error| error.to_string())?;
        let base_observations = base
            .observations
            .iter()
            .map(|observation| format!("{:x}:{:02x}", observation.step, observation.pwm))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "rtl_trace_base={},transitions={},observations={base_observations}",
            base.channel, base.transitions
        );
        let extended =
            extend_pwm_rtl_trace_one_phase_cycle(trace).map_err(|error| error.to_string())?;
        let replay = replay_pwm_rtl_trace(&model, &extended).map_err(|error| error.to_string())?;
        let phase_observations = replay
            .observations
            .iter()
            .map(|observation| format!("{:x}:{:02x}", observation.step, observation.pwm))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "rtl_trace_phase_cycle={},transitions={},observations={phase_observations}",
            replay.channel, replay.transitions
        );
    }
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("compiled MMIO to PWM RTL replay failed: {error}");
            ExitCode::FAILURE
        }
    }
}
