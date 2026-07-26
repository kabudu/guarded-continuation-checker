use guarded_continuation_checker::{
    compiled_mmio_certificate::decode_compiled_mmio_certificate,
    compiled_mmio_file::{load_compiled_mmio_inputs, parse_compiled_mmio_manifest},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn reject_mutation(
    root: &Path,
    manifest: &Path,
    certificate: &guarded_continuation_checker::compiled_mmio_certificate::CompiledMmioCertificate,
    path: &Path,
    exhaustive: bool,
) -> Result<usize, String> {
    let absolute = root.join(path);
    let original = fs::read(&absolute).map_err(|error| error.to_string())?;
    let indices = if exhaustive {
        (0..original.len()).collect::<Vec<_>>()
    } else {
        let mut values = vec![0, original.len() / 2, original.len() - 1];
        values.sort_unstable();
        values.dedup();
        values
    };
    for index in &indices {
        let mut changed = original.clone();
        changed[*index] ^= 1;
        fs::write(&absolute, &changed).map_err(|error| error.to_string())?;
        let refused = match load_compiled_mmio_inputs(root, manifest) {
            Ok(inputs) => inputs.verify(certificate).is_err(),
            Err(_) => true,
        };
        fs::write(&absolute, &original).map_err(|error| error.to_string())?;
        if !refused {
            return Err(format!(
                "mutation unexpectedly verified: {} byte {index}",
                path.display()
            ));
        }
    }
    Ok(indices.len())
}

fn run() -> Result<(), String> {
    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err("usage: hostile_compiled_mmio_files ROOT MANIFEST CERTIFICATE".to_string());
    }
    let root = PathBuf::from(&arguments[1]);
    let manifest_path = PathBuf::from(&arguments[2]);
    let manifest_bytes = fs::read(root.join(&manifest_path)).map_err(|error| error.to_string())?;
    let manifest = parse_compiled_mmio_manifest(&manifest_bytes)?;
    let certificate_bytes = fs::read(&arguments[3]).map_err(|error| error.to_string())?;
    let certificate =
        decode_compiled_mmio_certificate(&certificate_bytes).map_err(|error| error.to_string())?;

    let mut certificate_mutations = 0usize;
    for index in 0..certificate_bytes.len() {
        let mut changed = certificate_bytes.clone();
        changed[index] ^= 1;
        if decode_compiled_mmio_certificate(&changed).is_ok() {
            return Err(format!(
                "certificate mutation unexpectedly decoded at byte {index}"
            ));
        }
        certificate_mutations += 1;
    }

    let image_mutations =
        reject_mutation(&root, &manifest_path, &certificate, &manifest.image, true)?;
    let symbol_mutations =
        reject_mutation(&root, &manifest_path, &certificate, &manifest.symbols, true)?;
    let mut representative_mutations = reject_mutation(
        &root,
        &manifest_path,
        &certificate,
        &manifest.toolchain,
        false,
    )?;
    for member in manifest.upstream.iter().chain(&manifest.compatibility) {
        representative_mutations +=
            reject_mutation(&root, &manifest_path, &certificate, &member.path, false)?;
    }
    println!(
        "compiled_mmio_hostile_status=PASS certificate_mutations={certificate_mutations} image_mutations={image_mutations} symbol_mutations={symbol_mutations} representative_source_toolchain_mutations={representative_mutations}"
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("compiled-MMIO hostile cohort failed: {error}");
            ExitCode::FAILURE
        }
    }
}
