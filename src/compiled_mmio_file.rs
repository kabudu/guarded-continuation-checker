//! Strict file boundary for compiled-MMIO certificates.

use crate::compiled_mmio_certificate::{
    BoundArtifact, CompiledMmioCertificate, CompiledMmioCertificateInputs,
    MAX_BOUND_ARTIFACT_BYTES, MAX_BOUND_ARTIFACTS, MAX_COMPILED_MMIO_CERTIFICATE_BYTES,
    MAX_SYMBOL_TABLE_BYTES, MAX_TOOLCHAIN_IDENTITY_BYTES, certify_compiled_mmio,
    decode_compiled_mmio_certificate, encode_compiled_mmio_certificate, verify_compiled_mmio,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const COMPILED_MMIO_FILE_CLI_VERSION: u32 = 1;
pub const COMPILED_MMIO_INPUT_MANIFEST_VERSION: u32 = 1;
pub const MAX_COMPILED_MMIO_MANIFEST_BYTES: usize = 64 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestMember {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioInputManifest {
    pub upstream: Vec<ManifestMember>,
    pub compatibility: Vec<ManifestMember>,
    pub toolchain: PathBuf,
    pub image: PathBuf,
    pub symbols: PathBuf,
}

pub struct LoadedCompiledMmioInputs {
    manifest: CompiledMmioInputManifest,
    upstream: Vec<Vec<u8>>,
    compatibility: Vec<Vec<u8>>,
    toolchain: Vec<u8>,
    image: Vec<u8>,
    symbols: Vec<u8>,
}

impl LoadedCompiledMmioInputs {
    fn with_inputs<T>(&self, operation: impl FnOnce(CompiledMmioCertificateInputs<'_>) -> T) -> T {
        let upstream = self
            .manifest
            .upstream
            .iter()
            .zip(&self.upstream)
            .map(|(member, bytes)| BoundArtifact {
                name: &member.name,
                bytes,
            })
            .collect::<Vec<_>>();
        let compatibility = self
            .manifest
            .compatibility
            .iter()
            .zip(&self.compatibility)
            .map(|(member, bytes)| BoundArtifact {
                name: &member.name,
                bytes,
            })
            .collect::<Vec<_>>();
        operation(CompiledMmioCertificateInputs {
            upstream_sources: &upstream,
            compatibility_sources: &compatibility,
            toolchain_identity: &self.toolchain,
            image: &self.image,
            symbol_table: &self.symbols,
        })
    }

    pub fn certify(&self) -> Result<CompiledMmioCertificate, String> {
        self.with_inputs(certify_compiled_mmio)
            .map_err(|error| error.to_string())
    }

    pub fn verify(&self, certificate: &CompiledMmioCertificate) -> Result<(), String> {
        self.with_inputs(|inputs| verify_compiled_mmio(certificate, inputs))
            .map_err(|error| error.to_string())
    }

    pub fn image_len(&self) -> usize {
        self.image.len()
    }
}

fn canonical_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains('\\') {
        return Err("manifest path is not canonical".to_string());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("manifest path must be a canonical relative path".to_string());
    }
    Ok(path.to_path_buf())
}

fn parse_member(value: &str) -> Result<ManifestMember, String> {
    let (name, path) = value
        .split_once(',')
        .ok_or_else(|| "manifest member must use NAME,PATH syntax".to_string())?;
    if name.is_empty()
        || name.len() > 256
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'\\' && byte != b',')
    {
        return Err("manifest member name is not canonical".to_string());
    }
    Ok(ManifestMember {
        name: name.to_string(),
        path: canonical_relative_path(path)?,
    })
}

pub fn parse_compiled_mmio_manifest(bytes: &[u8]) -> Result<CompiledMmioInputManifest, String> {
    if bytes.is_empty() || bytes.len() > MAX_COMPILED_MMIO_MANIFEST_BYTES {
        return Err("compiled-MMIO manifest size is outside policy".to_string());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "compiled-MMIO manifest is not UTF-8".to_string())?;
    if text.contains('\r') || !text.ends_with('\n') {
        return Err("compiled-MMIO manifest must use canonical LF lines".to_string());
    }
    let mut lines = text.lines();
    if lines.next() != Some("gcc-compiled-mmio-input-manifest-v1") {
        return Err("compiled-MMIO manifest header mismatch".to_string());
    }
    fn count(lines: &mut std::str::Lines<'_>, label: &str) -> Result<usize, String> {
        let prefix = format!("{label}_count=");
        let line = lines
            .next()
            .ok_or_else(|| format!("missing {label} count"))?;
        let value = line
            .strip_prefix(&prefix)
            .ok_or_else(|| format!("invalid {label} count"))?
            .parse::<usize>()
            .map_err(|_| format!("invalid {label} count"))?;
        if value == 0 || value > MAX_BOUND_ARTIFACTS {
            return Err(format!("{label} count is outside policy"));
        }
        Ok(value)
    }
    fn members(
        lines: &mut std::str::Lines<'_>,
        label: &str,
        count: usize,
    ) -> Result<Vec<ManifestMember>, String> {
        let prefix = format!("{label}=");
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            let value = lines
                .next()
                .and_then(|line| line.strip_prefix(&prefix))
                .ok_or_else(|| format!("missing {label} member"))?;
            result.push(parse_member(value)?);
        }
        if !result.windows(2).all(|pair| pair[0].name < pair[1].name)
            || !result.windows(2).all(|pair| pair[0].path < pair[1].path)
        {
            return Err(format!(
                "{label} names and paths must both be strictly sorted and unique"
            ));
        }
        Ok(result)
    }
    let upstream_count = count(&mut lines, "upstream")?;
    let upstream = members(&mut lines, "upstream", upstream_count)?;
    let compatibility_count = count(&mut lines, "compatibility")?;
    let compatibility = members(&mut lines, "compatibility", compatibility_count)?;
    let field = |line: Option<&str>, label: &str| -> Result<PathBuf, String> {
        canonical_relative_path(
            line.and_then(|line| line.strip_prefix(&format!("{label}=")))
                .ok_or_else(|| format!("missing {label} path"))?,
        )
    };
    let toolchain = field(lines.next(), "toolchain")?;
    let image = field(lines.next(), "image")?;
    let symbols = field(lines.next(), "symbols")?;
    if lines.next() != Some("status=complete") || lines.next().is_some() {
        return Err("compiled-MMIO manifest has missing or trailing fields".to_string());
    }
    let mut all_paths = upstream
        .iter()
        .chain(&compatibility)
        .map(|member| &member.path)
        .collect::<Vec<_>>();
    all_paths.extend([&toolchain, &image, &symbols]);
    let mut sorted = all_paths.clone();
    sorted.sort();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("compiled-MMIO manifest contains aliased paths".to_string());
    }
    Ok(CompiledMmioInputManifest {
        upstream,
        compatibility,
        toolchain,
        image,
        symbols,
    })
}

fn read_regular(
    root: &Path,
    relative: &Path,
    limit: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let root_metadata =
        fs::symlink_metadata(root).map_err(|error| format!("inspect input root: {error}"))?;
    if !root_metadata.file_type().is_dir() || root_metadata.file_type().is_symlink() {
        return Err("input root must be a non-symlink directory".to_string());
    }
    let mut checked = root.to_path_buf();
    for component in relative.components() {
        checked.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&checked)
            .map_err(|error| format!("inspect {label} {}: {error}", checked.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{label} path contains a symlink"));
        }
    }
    let metadata = fs::metadata(&checked).map_err(|error| format!("inspect {label}: {error}"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limit as u64 {
        return Err(format!("{label} size or file type is outside policy"));
    }
    fs::read(&checked).map_err(|error| format!("read {label}: {error}"))
}

pub fn load_compiled_mmio_inputs(
    root: &Path,
    manifest_path: &Path,
) -> Result<LoadedCompiledMmioInputs, String> {
    let manifest_bytes = read_regular(
        root,
        manifest_path,
        MAX_COMPILED_MMIO_MANIFEST_BYTES,
        "input manifest",
    )?;
    let manifest = parse_compiled_mmio_manifest(&manifest_bytes)?;
    let load_set = |members: &[ManifestMember], label: &str| -> Result<Vec<Vec<u8>>, String> {
        let mut total = 0usize;
        members
            .iter()
            .map(|member| {
                let bytes = read_regular(root, &member.path, MAX_BOUND_ARTIFACT_BYTES, label)?;
                total = total
                    .checked_add(bytes.len())
                    .ok_or_else(|| format!("{label} byte count overflow"))?;
                if total > MAX_BOUND_ARTIFACT_BYTES {
                    return Err(format!("{label} bytes exceed policy"));
                }
                Ok(bytes)
            })
            .collect()
    };
    let upstream = load_set(&manifest.upstream, "upstream source")?;
    let compatibility = load_set(&manifest.compatibility, "compatibility source")?;
    let toolchain = read_regular(
        root,
        &manifest.toolchain,
        MAX_TOOLCHAIN_IDENTITY_BYTES,
        "toolchain identity",
    )?;
    let image = read_regular(
        root,
        &manifest.image,
        MAX_BOUND_ARTIFACT_BYTES,
        "compiled image",
    )?;
    let symbols = read_regular(
        root,
        &manifest.symbols,
        MAX_SYMBOL_TABLE_BYTES,
        "symbol table",
    )?;
    Ok(LoadedCompiledMmioInputs {
        manifest,
        upstream,
        compatibility,
        toolchain,
        image,
        symbols,
    })
}

fn certificate_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn publish_verified_create_new(
    output: &Path,
    encoded: &[u8],
    loaded: &LoadedCompiledMmioInputs,
) -> Result<(), String> {
    publish_verified_create_new_with(output, loaded, |file| file.write_all(encoded))
}

fn publish_verified_create_new_with(
    output: &Path,
    loaded: &LoadedCompiledMmioInputs,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
) -> Result<(), String> {
    if fs::symlink_metadata(output).is_ok() {
        return Err("certificate output already exists".to_string());
    }
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "certificate output filename is invalid".to_string())?;
    let temporary = parent.join(format!(
        ".{name}.gcc-compiled-mmio-{}-{}.tmp",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("create temporary certificate: {error}"))?;
        write(&mut file)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("write temporary certificate: {error}"))?;
        drop(file);
        let disk = read_regular(
            parent,
            Path::new(
                temporary
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| "temporary filename is invalid".to_string())?,
            ),
            MAX_COMPILED_MMIO_CERTIFICATE_BYTES,
            "temporary certificate",
        )?;
        let decoded = decode_compiled_mmio_certificate(&disk).map_err(|error| error.to_string())?;
        loaded.verify(&decoded)?;
        fs::hard_link(&temporary, output)
            .map_err(|error| format!("publish certificate without replacement: {error}"))?;
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("sync certificate directory: {error}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(temporary);
    result
}

pub fn run_compiled_mmio_file_cli(args: &[String]) -> Result<bool, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(false);
    };
    if command == "compiled-mmio-cli-version" {
        if args.len() != 1 {
            return Err(
                "usage: guarded-continuation-checker compiled-mmio-cli-version".to_string(),
            );
        }
        println!(
            "compiled_mmio_cli_version={COMPILED_MMIO_FILE_CLI_VERSION} manifest_version={COMPILED_MMIO_INPUT_MANIFEST_VERSION} certificate_version=1 max_manifest_bytes={MAX_COMPILED_MMIO_MANIFEST_BYTES} max_certificate_bytes={MAX_COMPILED_MMIO_CERTIFICATE_BYTES} publication=atomic-create-new verification=independent-replay unsupported=fail-closed"
        );
        return Ok(true);
    }
    if command != "compiled-mmio-certify" && command != "compiled-mmio-verify" {
        return Ok(false);
    }
    if args.len() != 4 {
        return Err(format!(
            "usage: guarded-continuation-checker {command} ROOT MANIFEST {}",
            if command == "compiled-mmio-certify" {
                "OUTPUT.certificate"
            } else {
                "INPUT.certificate"
            }
        ));
    }
    let root = Path::new(&args[1]);
    let manifest_path = canonical_relative_path(&args[2])?;
    let loaded = load_compiled_mmio_inputs(root, &manifest_path)?;
    let certificate_path = Path::new(&args[3]);
    let started = std::time::Instant::now();
    let (certificate, encoded, status) = if command == "compiled-mmio-certify" {
        let certificate = loaded.certify()?;
        let encoded =
            encode_compiled_mmio_certificate(&certificate).map_err(|error| error.to_string())?;
        publish_verified_create_new(certificate_path, &encoded, &loaded)?;
        (certificate, encoded, "CREATED")
    } else {
        let encoded = read_regular(
            certificate_path.parent().unwrap_or_else(|| Path::new(".")),
            Path::new(
                certificate_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| "certificate filename is invalid".to_string())?,
            ),
            MAX_COMPILED_MMIO_CERTIFICATE_BYTES,
            "compiled-MMIO certificate",
        )?;
        let certificate =
            decode_compiled_mmio_certificate(&encoded).map_err(|error| error.to_string())?;
        loaded.verify(&certificate)?;
        (certificate, encoded, "VERIFIED")
    };
    println!(
        "compiled-mmio status={status} certificate_version={} certificate_bytes={} certificate_sha256={} image_bytes={} instructions={} events={} elapsed_micros={}",
        certificate.version,
        encoded.len(),
        certificate_sha256(&encoded),
        loaded.image_len(),
        certificate.execution.steps,
        certificate.execution.events.len(),
        started.elapsed().as_micros()
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riscv32imc::RV32_IMAGE_BASE;

    fn instruction(opcode: u32, rd: u32, rs1: u32, immediate: u32) -> [u8; 4] {
        (((immediate & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | opcode).to_le_bytes()
    }

    fn fixture_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "gcc-compiled-mmio-file-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&instruction(0x13, 10, 0, 7));
        image[4..8].copy_from_slice(&instruction(0x67, 0, 1, 0));
        let symbols = format!(
            "{:08x} T gcc_firmware_entry\n{:08x} B gcc_mmio_event_count\n{:08x} B gcc_mmio_events\n",
            RV32_IMAGE_BASE,
            RV32_IMAGE_BASE + 0x100,
            RV32_IMAGE_BASE + 0x104
        );
        fs::write(root.join("upstream.c"), b"upstream\n").unwrap();
        fs::write(root.join("compat.c"), b"compatibility\n").unwrap();
        fs::write(root.join("toolchain.txt"), b"clang=21.1.5\n").unwrap();
        fs::write(root.join("image.bin"), image).unwrap();
        fs::write(root.join("symbols.txt"), symbols).unwrap();
        fs::write(
            root.join("inputs.txt"),
            b"gcc-compiled-mmio-input-manifest-v1\nupstream_count=1\nupstream=upstream.c,upstream.c\ncompatibility_count=1\ncompatibility=compat.c,compat.c\ntoolchain=toolchain.txt\nimage=image.bin\nsymbols=symbols.txt\nstatus=complete\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn manifest_rejects_noncanonical_paths_and_aliases() {
        let valid = b"gcc-compiled-mmio-input-manifest-v1\nupstream_count=1\nupstream=a,a.c\ncompatibility_count=1\ncompatibility=b,b.c\ntoolchain=toolchain.txt\nimage=image.bin\nsymbols=symbols.txt\nstatus=complete\n";
        assert!(parse_compiled_mmio_manifest(valid).is_ok());
        for invalid in [
            valid.as_slice().replace(b"a.c", b"../x"),
            valid.as_slice().replace(b"a.c", b"/tmp"),
            valid.as_slice().replace(b"image.bin", b"a.c"),
            valid
                .as_slice()
                .replace(b"upstream_count=1", b"upstream_count=2"),
            [valid.as_slice(), b"extra=true\n"].concat(),
            vec![b'x'; MAX_COMPILED_MMIO_MANIFEST_BYTES + 1],
        ] {
            assert!(parse_compiled_mmio_manifest(&invalid).is_err());
        }
    }

    #[test]
    fn file_cycle_is_deterministic_no_clobber_and_independently_verified() {
        let root = fixture_root("cycle");
        let first = root.join("first.cert");
        let second = root.join("second.cert");
        let arguments = |command: &str, output: &Path| {
            vec![
                command.to_string(),
                root.display().to_string(),
                "inputs.txt".to_string(),
                output.display().to_string(),
            ]
        };
        assert!(run_compiled_mmio_file_cli(&arguments("compiled-mmio-certify", &first)).unwrap());
        assert!(run_compiled_mmio_file_cli(&arguments("compiled-mmio-certify", &second)).unwrap());
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
        assert!(run_compiled_mmio_file_cli(&arguments("compiled-mmio-verify", &first)).unwrap());
        assert!(run_compiled_mmio_file_cli(&arguments("compiled-mmio-certify", &first)).is_err());
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exhaustive_mutations_and_path_attacks_refuse() {
        let root = fixture_root("hostile");
        let loaded = load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).unwrap();
        let certificate = loaded.certify().unwrap();
        let encoded = encode_compiled_mmio_certificate(&certificate).unwrap();
        for index in 0..encoded.len() {
            let mut changed = encoded.clone();
            changed[index] ^= 1;
            assert!(decode_compiled_mmio_certificate(&changed).is_err());
        }
        for filename in [
            "upstream.c",
            "compat.c",
            "toolchain.txt",
            "image.bin",
            "symbols.txt",
        ] {
            let original = fs::read(root.join(filename)).unwrap();
            for index in 0..original.len() {
                let mut changed = original.clone();
                changed[index] ^= 1;
                fs::write(root.join(filename), &changed).unwrap();
                match load_compiled_mmio_inputs(&root, Path::new("inputs.txt")) {
                    Ok(changed_inputs) => assert!(changed_inputs.verify(&certificate).is_err()),
                    Err(_) => {}
                }
            }
            fs::write(root.join(filename), original).unwrap();
        }
        let original_manifest = fs::read(root.join("inputs.txt")).unwrap();
        for (from, to) in [
            (
                b"upstream=upstream.c,upstream.c".as_slice(),
                b"upstream=upstream.c,../escape.c".as_slice(),
            ),
            (
                b"compatibility=compat.c,compat.c".as_slice(),
                b"compatibility=compat.c,/tmp/evil.c".as_slice(),
            ),
            (
                b"image=image.bin".as_slice(),
                b"image=upstream.c".as_slice(),
            ),
        ] {
            let hostile = original_manifest.as_slice().replace(from, to);
            fs::write(root.join("inputs.txt"), hostile).unwrap();
            assert!(load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).is_err());
        }
        let renamed = original_manifest
            .as_slice()
            .replace(b"upstream=upstream.c,", b"upstream=renamed.c,");
        fs::write(root.join("inputs.txt"), renamed).unwrap();
        let renamed_inputs = load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).unwrap();
        assert!(renamed_inputs.verify(&certificate).is_err());
        fs::write(root.join("inputs.txt"), original_manifest).unwrap();
        let toolchain = fs::read(root.join("toolchain.txt")).unwrap();
        fs::write(
            root.join("toolchain.txt"),
            vec![b'x'; MAX_TOOLCHAIN_IDENTITY_BYTES + 1],
        )
        .unwrap();
        assert!(load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).is_err());
        fs::write(root.join("toolchain.txt"), toolchain).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("upstream.c", root.join("alias.c")).unwrap();
            let manifest = fs::read(root.join("inputs.txt"))
                .unwrap()
                .as_slice()
                .replace(b"upstream.c,upstream.c", b"upstream.c,alias.c");
            fs::write(root.join("inputs.txt"), manifest).unwrap();
            assert!(load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_partial_publication_leaves_no_visible_or_temporary_file() {
        let root = fixture_root("partial");
        let loaded = load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).unwrap();
        let output = root.join("partial.cert");
        let error = publish_verified_create_new_with(&output, &loaded, |file| {
            file.write_all(b"partial")?;
            Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "injected write failure",
            ))
        })
        .unwrap_err();
        assert!(error.contains("injected write failure"));
        assert!(!output.exists());
        assert!(fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("gcc-compiled-mmio")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    trait ReplaceBytes {
        fn replace(&self, from: &[u8], to: &[u8]) -> Vec<u8>;
    }

    impl ReplaceBytes for [u8] {
        fn replace(&self, from: &[u8], to: &[u8]) -> Vec<u8> {
            let offset = self
                .windows(from.len())
                .position(|value| value == from)
                .unwrap();
            [&self[..offset], to, &self[offset + from.len()..]].concat()
        }
    }
}
