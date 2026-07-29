use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_rtl_certificate::{
        decode_compiled_mmio_rtl_certificate, verify_compiled_mmio_rtl_certificate,
    },
};
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err(
            "usage: verify_compiled_mmio_pwm_rtl CERTIFICATE FIRMWARE_BIN SYMBOLS_TXT MODEL_BTOR2"
                .to_string(),
        );
    }
    let certificate_bytes = fs::read(&arguments[1]).map_err(|error| error.to_string())?;
    let image = fs::read(&arguments[2]).map_err(|error| error.to_string())?;
    let symbol_bytes = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let model = fs::read(&arguments[4]).map_err(|error| error.to_string())?;
    let symbols = parse_compiled_mmio_symbols(&symbol_bytes).map_err(|error| error.to_string())?;

    let verification =
        verify_compiled_mmio_rtl_certificate(&certificate_bytes, &image, symbols, &model)
            .map_err(|error| error.to_string())?;
    let certificate = decode_compiled_mmio_rtl_certificate(&certificate_bytes)
        .map_err(|error| error.to_string())?;

    println!("trust_boundary_consumer_version=1");
    println!("route=gcc-certificate");
    println!("firmware_behaviors=7");
    println!("valid_rtl_members={}", verification.valid_rtl_members);
    println!("invalid_rtl_members={}", verification.invalid_rtl_members);
    println!("rtl_transitions={}", verification.rtl_transitions);
    println!("rtl_observations={}", verification.rtl_observations);
    let phase_cycle_classes = certificate
        .members
        .iter()
        .map(|member| {
            member
                .observations
                .iter()
                .map(|observation| (observation.step, observation.pwm))
                .collect::<Vec<_>>()
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let nonzero_traces = certificate
        .members
        .iter()
        .filter(|member| {
            member
                .observations
                .iter()
                .any(|observation| observation.pwm != 0)
        })
        .count();
    println!("phase_cycle_classes={phase_cycle_classes}");
    println!("nonzero_traces={nonzero_traces}");
    for member in certificate.members {
        let observations = member
            .observations
            .iter()
            .map(|observation| format!("{:x}:{:02x}", observation.step, observation.pwm))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "rtl_trace_phase_cycle={},transitions={},observations={observations}",
            member.channel, member.transitions
        );
    }
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("compiled MMIO PWM RTL verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
