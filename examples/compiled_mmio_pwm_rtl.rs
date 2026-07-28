use guarded_continuation_checker::{
    compiled_mmio_certificate::parse_compiled_mmio_symbols,
    compiled_mmio_quotient::{build_predicate_mmio_workflow, verify_predicate_mmio_workflow},
    compiled_mmio_rtl_certificate::{
        decode_compiled_mmio_rtl_certificate, encode_compiled_mmio_rtl_certificate,
        produce_compiled_mmio_rtl_certificate, verify_compiled_mmio_rtl_certificate,
    },
    compiled_mmio_rtl_mapping::{
        PWM_RTL_MODEL_SHA256, extend_pwm_rtl_trace_one_phase_cycle, map_pwm_mmio_workflow,
        replay_pwm_rtl_trace,
    },
};
use std::{env, fs, io::Write, process::ExitCode};

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hostile_codec_cases(bytes: &[u8]) -> Result<u64, String> {
    let mut cases = 0u64;
    for index in 0..bytes.len() {
        let mut changed = bytes.to_vec();
        changed[index] ^= 1;
        if decode_compiled_mmio_rtl_certificate(&changed).is_ok() {
            return Err(format!("accepted mutation at artifact byte {index}"));
        }
        cases += 1;
    }
    for length in 0..bytes.len() {
        if decode_compiled_mmio_rtl_certificate(&bytes[..length]).is_ok() {
            return Err(format!("accepted truncation at artifact byte {length}"));
        }
        cases += 1;
    }
    let mut extended = bytes.to_vec();
    extended.push(0);
    if decode_compiled_mmio_rtl_certificate(&extended).is_ok() {
        return Err("accepted artifact extension".to_string());
    }
    Ok(cases + 1)
}

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if !matches!(arguments.len(), 4 | 5) {
        return Err(
            "usage: compiled_mmio_pwm_rtl FIRMWARE_BIN SYMBOLS_TXT MODEL_BTOR2 [CERTIFICATE_OUTPUT]"
                .to_string(),
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
    let first = produce_compiled_mmio_rtl_certificate(&image, symbols, &model)
        .map_err(|error| error.to_string())?;
    let second = produce_compiled_mmio_rtl_certificate(&image, symbols, &model)
        .map_err(|error| error.to_string())?;
    if first != second {
        return Err("clean composed certificate cycles differ".to_string());
    }
    let verification = verify_compiled_mmio_rtl_certificate(&first, &image, symbols, &model)
        .map_err(|error| error.to_string())?;
    let hostile_cases = hostile_codec_cases(&first)?;
    let mut observation_drift =
        decode_compiled_mmio_rtl_certificate(&first).map_err(|error| error.to_string())?;
    let last = observation_drift.members[0]
        .observations
        .last_mut()
        .ok_or_else(|| "composed certificate member has no observation".to_string())?;
    last.pwm ^= 1;
    let observation_drift = encode_compiled_mmio_rtl_certificate(&observation_drift)
        .map_err(|error| error.to_string())?;
    if verify_compiled_mmio_rtl_certificate(&observation_drift, &image, symbols, &model).is_ok() {
        return Err("accepted checksummed RTL observation drift".to_string());
    }
    let mut class_drift =
        decode_compiled_mmio_rtl_certificate(&first).map_err(|error| error.to_string())?;
    let channel_zero = class_drift.members[0].observations.clone();
    class_drift.members[0].observations = class_drift.members[1].observations.clone();
    class_drift.members[1].observations = channel_zero;
    let class_drift =
        encode_compiled_mmio_rtl_certificate(&class_drift).map_err(|error| error.to_string())?;
    if verify_compiled_mmio_rtl_certificate(&class_drift, &image, symbols, &model).is_ok() {
        return Err("accepted checksummed RTL class substitution".to_string());
    }
    let mut changed_model = model.clone();
    let model_middle = changed_model.len() / 2;
    changed_model[model_middle] ^= 1;
    if verify_compiled_mmio_rtl_certificate(&first, &image, symbols, &changed_model).is_ok() {
        return Err("accepted RTL model drift".to_string());
    }
    let mut changed_image = image.clone();
    changed_image[0] ^= 1;
    if verify_compiled_mmio_rtl_certificate(&first, &changed_image, symbols, &model).is_ok() {
        return Err("accepted firmware image drift".to_string());
    }
    let mut changed_symbols = symbols;
    changed_symbols.entry ^= 2;
    if verify_compiled_mmio_rtl_certificate(&first, &image, changed_symbols, &model).is_ok() {
        return Err("accepted firmware symbol drift".to_string());
    }
    if arguments.len() == 5 {
        let output = std::path::Path::new(&arguments[4]);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|error| error.to_string())?;
        file.write_all(&first).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    println!("composed_certificate_bytes={}", verification.artifact_bytes);
    println!(
        "composed_certificate_valid_rtl_members={}",
        verification.valid_rtl_members
    );
    println!(
        "composed_certificate_invalid_rtl_members={}",
        verification.invalid_rtl_members
    );
    println!(
        "composed_certificate_rtl_transitions={}",
        verification.rtl_transitions
    );
    println!(
        "composed_certificate_rtl_observations={}",
        verification.rtl_observations
    );
    println!("composed_certificate_clean_cycles_identical=true");
    println!("composed_certificate_hostile_codec_cases={hostile_cases}");
    println!("composed_certificate_source_drift_cases=3");
    println!("composed_certificate_semantic_drift_cases=2");
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
