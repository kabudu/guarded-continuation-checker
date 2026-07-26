use guarded_continuation_checker::compiled_mmio_certificate::{
    BoundArtifact, CompiledMmioCertificateInputs, certify_compiled_mmio,
    decode_compiled_mmio_certificate, encode_compiled_mmio_certificate, verify_compiled_mmio,
};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

const UPSTREAM: &[&str] = &["LICENSE", "PROVENANCE.md", "upstream/dif_pwm.c"];
const COMPATIBILITY: &[&str] = &[
    "compat/assert.h",
    "compat/firmware_caller.c",
    "compat/hw/top/pwm_regs.h",
    "compat/sw/device/lib/base/bitfield.h",
    "compat/sw/device/lib/dif/dif_base.h",
    "compat/sw/device/lib/dif/dif_pwm.h",
];

fn load_set(root: &Path, names: &[&str]) -> Result<Vec<Vec<u8>>, String> {
    names
        .iter()
        .map(|name| fs::read(root.join(name)).map_err(|error| format!("{name}: {error}")))
        .collect()
}

fn bind<'a>(names: &'a [&'a str], bytes: &'a [Vec<u8>]) -> Vec<BoundArtifact<'a>> {
    names
        .iter()
        .zip(bytes)
        .map(|(name, bytes)| BoundArtifact { name, bytes })
        .collect()
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn run() -> Result<(), String> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 5 {
        return Err(
            "usage: certify_compiled_mmio PROFILE_DIR CORPUS_ROOT TOOLCHAIN_ID OUTPUT".to_string(),
        );
    }
    let profile = PathBuf::from(&arguments[1]);
    let corpus = PathBuf::from(&arguments[2]);
    let toolchain = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let image = fs::read(profile.join("firmware.bin")).map_err(|error| error.to_string())?;
    let symbols =
        fs::read(profile.join("firmware.symbols.txt")).map_err(|error| error.to_string())?;
    let upstream_bytes = load_set(&corpus, UPSTREAM)?;
    let compatibility_bytes = load_set(&corpus, COMPATIBILITY)?;
    let upstream = bind(UPSTREAM, &upstream_bytes);
    let compatibility = bind(COMPATIBILITY, &compatibility_bytes);
    let inputs = CompiledMmioCertificateInputs {
        upstream_sources: &upstream,
        compatibility_sources: &compatibility,
        toolchain_identity: &toolchain,
        image: &image,
        symbol_table: &symbols,
    };
    let certificate = certify_compiled_mmio(inputs).map_err(|error| error.to_string())?;
    let encoded =
        encode_compiled_mmio_certificate(&certificate).map_err(|error| error.to_string())?;
    let decoded = decode_compiled_mmio_certificate(&encoded).map_err(|error| error.to_string())?;
    verify_compiled_mmio(&decoded, inputs).map_err(|error| error.to_string())?;

    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&arguments[4])
        .map_err(|error| error.to_string())?;
    output
        .write_all(&encoded)
        .map_err(|error| error.to_string())?;
    output.sync_all().map_err(|error| error.to_string())?;

    println!("certificate_version={}", certificate.version);
    println!("certificate_bytes={}", encoded.len());
    println!("certificate_sha256={}", hex(Sha256::digest(&encoded)));
    println!("image_sha256={}", hex(Sha256::digest(&image)));
    println!("instruction_count={}", certificate.execution.steps);
    println!("event_count={}", certificate.execution.events.len());
    println!("status=complete");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("compiled MMIO certification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
