//! Strict file boundary for compiled-MMIO certificates.

use crate::compiled_mmio_certificate::{
    BoundArtifact, CompiledMmioCertificate, CompiledMmioCertificateInputs,
    MAX_BOUND_ARTIFACT_BYTES, MAX_BOUND_ARTIFACTS, MAX_COMPILED_MMIO_CERTIFICATE_BYTES,
    MAX_SYMBOL_TABLE_BYTES, MAX_TOOLCHAIN_IDENTITY_BYTES, certify_compiled_mmio,
    decode_compiled_mmio_certificate, encode_compiled_mmio_certificate, verify_compiled_mmio,
};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::{
    collections::BTreeSet,
    ffi::{CString, OsStr, OsString},
    io::{Read, Seek, SeekFrom},
    os::unix::{
        ffi::OsStrExt,
        fs::MetadataExt,
        io::{AsRawFd, FromRawFd},
    },
};
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

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectSnapshot {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
fn snapshot(metadata: &fs::Metadata) -> ObjectSnapshot {
    ObjectSnapshot {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
fn c_string(value: &OsStr, label: &str) -> Result<CString, String> {
    CString::new(value.as_bytes()).map_err(|_| format!("{label} contains NUL"))
}

#[cfg(unix)]
fn open_root(path: &Path) -> Result<fs::File, String> {
    let path = c_string(path.as_os_str(), "input root path")?;
    // SAFETY: `path` is a live NUL-terminated C string. The returned descriptor
    // is checked before ownership is transferred to `File`.
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "open input root without following symlinks: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: `descriptor` is newly owned after a successful `open`.
    Ok(unsafe { fs::File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn open_relative(
    directory: &fs::File,
    name: &OsStr,
    directory_only: bool,
) -> Result<fs::File, String> {
    let name = c_string(name, "input path component")?;
    let mut flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    if directory_only {
        flags |= libc::O_DIRECTORY;
    } else {
        flags |= libc::O_NONBLOCK;
    }
    // SAFETY: `directory` owns a valid descriptor and `name` is a live
    // NUL-terminated C string. `openat` returns a new descriptor on success.
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(format!(
            "open input component without following symlinks: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: `descriptor` is newly owned after a successful `openat`.
    Ok(unsafe { fs::File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
struct RaceResistantInputRoot {
    root: fs::File,
    root_snapshot: ObjectSnapshot,
    traversed_directories: Vec<(fs::File, ObjectSnapshot)>,
    opened_files: BTreeSet<(u64, u64)>,
}

#[cfg(unix)]
impl RaceResistantInputRoot {
    fn open(path: &Path) -> Result<Self, String> {
        let root = open_root(path)?;
        let metadata = root
            .metadata()
            .map_err(|error| format!("inspect opened input root: {error}"))?;
        if !metadata.is_dir() {
            return Err("input root is not a directory".to_string());
        }
        Ok(Self {
            root_snapshot: snapshot(&metadata),
            root,
            traversed_directories: Vec::new(),
            opened_files: BTreeSet::new(),
        })
    }

    fn read_regular(
        &mut self,
        relative: &Path,
        limit: usize,
        label: &str,
    ) -> Result<Vec<u8>, String> {
        self.read_regular_with_hook(relative, limit, label, |_| {})
    }

    fn read_regular_with_hook(
        &mut self,
        relative: &Path,
        limit: usize,
        label: &str,
        hook: impl FnOnce(&fs::File),
    ) -> Result<Vec<u8>, String> {
        if relative.as_os_str().is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("{label} path is not canonical and relative"));
        }
        let components = relative
            .components()
            .map(|component| component.as_os_str())
            .collect::<Vec<_>>();
        let (filename, parents) = components
            .split_last()
            .ok_or_else(|| format!("{label} path is empty"))?;
        let mut directory = self
            .root
            .try_clone()
            .map_err(|error| format!("clone input root descriptor: {error}"))?;
        for component in parents {
            let next = open_relative(&directory, component, true)
                .map_err(|error| format!("open {label} parent: {error}"))?;
            let metadata = next
                .metadata()
                .map_err(|error| format!("inspect {label} parent: {error}"))?;
            if !metadata.is_dir() {
                return Err(format!("{label} parent is not a directory"));
            }
            self.traversed_directories.push((
                next.try_clone().map_err(|error| error.to_string())?,
                snapshot(&metadata),
            ));
            directory = next;
        }
        let mut file = open_relative(&directory, filename, false)
            .map_err(|error| format!("open {label}: {error}"))?;
        let before = file
            .metadata()
            .map_err(|error| format!("inspect opened {label}: {error}"))?;
        if !before.is_file() || before.len() == 0 || before.len() > limit as u64 {
            return Err(format!("{label} size or file type is outside policy"));
        }
        if !self.opened_files.insert((before.dev(), before.ino())) {
            return Err(format!("{label} aliases another manifest input"));
        }
        hook(&file);
        let expected = before.len() as usize;
        let mut bytes = Vec::with_capacity(expected.saturating_add(1));
        Read::by_ref(&mut file)
            .take(expected as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("read opened {label}: {error}"))?;
        let after = file
            .metadata()
            .map_err(|error| format!("reinspect opened {label}: {error}"))?;
        if bytes.len() != expected || snapshot(&before) != snapshot(&after) {
            return Err(format!("{label} changed while it was being read"));
        }
        Ok(bytes)
    }

    fn finish(self) -> Result<(), String> {
        let root_after = self
            .root
            .metadata()
            .map_err(|error| format!("reinspect input root: {error}"))?;
        if snapshot(&root_after) != self.root_snapshot {
            return Err("input root changed during acquisition".to_string());
        }
        for (directory, before) in self.traversed_directories {
            let after = directory
                .metadata()
                .map_err(|error| format!("reinspect input directory: {error}"))?;
            if snapshot(&after) != before {
                return Err("input directory changed during acquisition".to_string());
            }
        }
        Ok(())
    }
}

#[cfg(not(unix))]
struct RaceResistantInputRoot;

#[cfg(not(unix))]
impl RaceResistantInputRoot {
    fn open(_path: &Path) -> Result<Self, String> {
        Err(
            "race-resistant compiled-MMIO input acquisition is unsupported on this platform"
                .to_string(),
        )
    }
}

fn read_standalone_regular(
    root: &Path,
    relative: &Path,
    limit: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    #[cfg(unix)]
    {
        let mut opened = RaceResistantInputRoot::open(root)?;
        let bytes = opened.read_regular(relative, limit, label)?;
        opened.finish()?;
        Ok(bytes)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative, limit, label);
        RaceResistantInputRoot::open(root)?;
        unreachable!()
    }
}

#[cfg(unix)]
pub fn load_compiled_mmio_inputs(
    root: &Path,
    manifest_path: &Path,
) -> Result<LoadedCompiledMmioInputs, String> {
    let mut opened_root = RaceResistantInputRoot::open(root)?;
    let manifest_bytes = opened_root.read_regular(
        manifest_path,
        MAX_COMPILED_MMIO_MANIFEST_BYTES,
        "input manifest",
    )?;
    let manifest = parse_compiled_mmio_manifest(&manifest_bytes)?;
    let mut load_set = |members: &[ManifestMember], label: &str| -> Result<Vec<Vec<u8>>, String> {
        let mut total = 0usize;
        members
            .iter()
            .map(|member| {
                let bytes =
                    opened_root.read_regular(&member.path, MAX_BOUND_ARTIFACT_BYTES, label)?;
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
    let toolchain = opened_root.read_regular(
        &manifest.toolchain,
        MAX_TOOLCHAIN_IDENTITY_BYTES,
        "toolchain identity",
    )?;
    let image =
        opened_root.read_regular(&manifest.image, MAX_BOUND_ARTIFACT_BYTES, "compiled image")?;
    let symbols =
        opened_root.read_regular(&manifest.symbols, MAX_SYMBOL_TABLE_BYTES, "symbol table")?;
    opened_root.finish()?;
    Ok(LoadedCompiledMmioInputs {
        manifest,
        upstream,
        compatibility,
        toolchain,
        image,
        symbols,
    })
}

#[cfg(not(unix))]
pub fn load_compiled_mmio_inputs(
    root: &Path,
    _manifest_path: &Path,
) -> Result<LoadedCompiledMmioInputs, String> {
    RaceResistantInputRoot::open(root)?;
    unreachable!()
}

fn certificate_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(unix)]
struct RaceResistantOutput {
    directory: fs::File,
    ancestors: Vec<(fs::File, OsString, (u64, u64))>,
    final_name: CString,
}

#[cfg(unix)]
impl RaceResistantOutput {
    fn open(output: &Path) -> Result<Self, String> {
        let filename = output
            .file_name()
            .ok_or_else(|| "certificate output path is empty".to_string())?;
        let declared_parent = output.parent().unwrap_or_else(|| Path::new("."));
        let canonical_parent = fs::canonicalize(declared_parent)
            .map_err(|error| format!("resolve certificate output parent: {error}"))?;
        let resolved_output = canonical_parent.join(filename);
        let components = resolved_output.components().collect::<Vec<_>>();
        let (filename, parent_components) = components
            .split_last()
            .ok_or_else(|| "certificate output path is empty".to_string())?;
        let Component::Normal(filename) = filename else {
            return Err("certificate output path is not canonical".to_string());
        };
        let absolute = resolved_output.is_absolute();
        let mut parents = parent_components;
        if absolute {
            let Some((Component::RootDir, remaining)) = parents.split_first() else {
                return Err("certificate output path is not canonical".to_string());
            };
            parents = remaining;
        }
        if parents
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err("certificate output path is not canonical".to_string());
        }
        let mut directory = open_root(if absolute {
            Path::new("/")
        } else {
            Path::new(".")
        })
        .map_err(|error| format!("open certificate output root: {error}"))?;
        let mut ancestors = Vec::new();
        for component in parents {
            let Component::Normal(name) = component else {
                unreachable!("parent components were validated");
            };
            let next = open_relative(&directory, name, true)
                .map_err(|error| format!("open certificate output parent: {error}"))?;
            let metadata = next
                .metadata()
                .map_err(|error| format!("inspect certificate output parent: {error}"))?;
            ancestors.push((
                directory
                    .try_clone()
                    .map_err(|error| format!("retain certificate output ancestor: {error}"))?,
                name.to_os_string(),
                (metadata.dev(), metadata.ino()),
            ));
            directory = next;
        }
        Ok(Self {
            directory,
            ancestors,
            final_name: c_string(filename, "certificate output filename")?,
        })
    }

    fn create_temporary(&self, name: &CString) -> Result<fs::File, String> {
        let flags = libc::O_RDWR
            | libc::O_CREAT
            | libc::O_EXCL
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK;
        // SAFETY: the directory descriptor and NUL-terminated name are valid.
        // `O_CREAT` supplies the required mode argument and the returned
        // descriptor is checked before ownership transfer.
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                flags,
                0o600 as libc::c_uint,
            )
        };
        if descriptor < 0 {
            return Err(format!(
                "create temporary certificate: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: `descriptor` is newly owned after successful `openat`.
        Ok(unsafe { fs::File::from_raw_fd(descriptor) })
    }

    fn publish(&self, temporary: &CString) -> Result<(), String> {
        // SAFETY: both names are live NUL-terminated strings and both
        // directory descriptors are valid for the duration of `linkat`.
        let result = unsafe {
            libc::linkat(
                self.directory.as_raw_fd(),
                temporary.as_ptr(),
                self.directory.as_raw_fd(),
                self.final_name.as_ptr(),
                0,
            )
        };
        if result != 0 {
            return Err(format!(
                "publish certificate without replacement: {}",
                std::io::Error::last_os_error()
            ));
        }
        self.directory
            .sync_all()
            .map_err(|error| format!("sync certificate directory: {error}"))
    }

    fn cleanup(&self, temporary: &CString) -> Result<(), String> {
        // SAFETY: the directory descriptor and NUL-terminated name remain
        // valid for the duration of `unlinkat`.
        let result = unsafe { libc::unlinkat(self.directory.as_raw_fd(), temporary.as_ptr(), 0) };
        if result != 0 {
            return Err(format!(
                "remove temporary certificate: {}",
                std::io::Error::last_os_error()
            ));
        }
        self.directory
            .sync_all()
            .map_err(|error| format!("sync certificate cleanup: {error}"))
    }

    fn finish(&self) -> Result<(), String> {
        for (parent, child_name, before_identity) in &self.ancestors {
            let child = open_relative(parent, child_name, true)
                .map_err(|error| format!("reopen certificate output ancestor: {error}"))?;
            let after = child
                .metadata()
                .map_err(|error| format!("reinspect certificate output ancestor: {error}"))?;
            if (after.dev(), after.ino()) != *before_identity {
                return Err("certificate output ancestor changed during publication".to_string());
            }
        }
        Ok(())
    }
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
    #[cfg(unix)]
    {
        let target = RaceResistantOutput::open(output)?;
        let temporary = CString::new(format!(
            ".gcc-compiled-mmio-{}-{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
        .map_err(|_| "temporary certificate filename contains NUL".to_string())?;
        let result = (|| {
            let mut file = target.create_temporary(&temporary)?;
            write(&mut file)
                .and_then(|_| file.sync_all())
                .map_err(|error| format!("write temporary certificate: {error}"))?;
            let before = file
                .metadata()
                .map_err(|error| format!("inspect temporary certificate: {error}"))?;
            if !before.is_file()
                || before.len() == 0
                || before.len() > MAX_COMPILED_MMIO_CERTIFICATE_BYTES as u64
            {
                return Err("temporary certificate size or file type is outside policy".to_string());
            }
            file.seek(SeekFrom::Start(0))
                .map_err(|error| format!("rewind temporary certificate: {error}"))?;
            let mut disk = Vec::with_capacity(before.len() as usize + 1);
            Read::by_ref(&mut file)
                .take(MAX_COMPILED_MMIO_CERTIFICATE_BYTES as u64 + 1)
                .read_to_end(&mut disk)
                .map_err(|error| format!("reload temporary certificate: {error}"))?;
            let after = file
                .metadata()
                .map_err(|error| format!("reinspect temporary certificate: {error}"))?;
            if disk.len() != before.len() as usize || snapshot(&before) != snapshot(&after) {
                return Err("temporary certificate changed during verification".to_string());
            }
            let decoded =
                decode_compiled_mmio_certificate(&disk).map_err(|error| error.to_string())?;
            loaded.verify(&decoded)?;
            target.publish(&temporary)?;
            target.cleanup(&temporary)?;
            target.finish()
        })();
        if result.is_err() {
            let _ = target.cleanup(&temporary);
        }
        result
    }
    #[cfg(not(unix))]
    {
        let _ = (output, loaded, write);
        Err(
            "race-resistant compiled-MMIO certificate publication is unsupported on this platform"
                .to_string(),
        )
    }
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
        let encoded = read_standalone_regular(
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
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    };

    fn instruction(opcode: u32, rd: u32, rs1: u32, immediate: u32) -> [u8; 4] {
        (((immediate & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | opcode).to_le_bytes()
    }

    fn fixture_image(return_value: u32) -> Vec<u8> {
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&instruction(0x13, 10, 0, return_value));
        image[4..8].copy_from_slice(&instruction(0x67, 0, 1, 0));
        image
    }

    fn fixture_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "gcc-compiled-mmio-file-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let image = fixture_image(7);
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
                if let Ok(changed_inputs) =
                    load_compiled_mmio_inputs(&root, Path::new("inputs.txt"))
                {
                    assert!(changed_inputs.verify(&certificate).is_err());
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

    #[cfg(unix)]
    #[test]
    fn descriptor_publication_cannot_be_redirected_after_parent_acquisition() {
        let root = fixture_root("output-parent-replacement");
        let container = root.join("container");
        let declared = container.join("declared");
        let retained = container.join("retained");
        fs::create_dir(&container).unwrap();
        fs::create_dir(&declared).unwrap();
        let target = RaceResistantOutput::open(&declared.join("result.cert")).unwrap();

        fs::rename(&declared, &retained).unwrap();
        fs::create_dir(&declared).unwrap();
        let temporary = CString::new(".controlled.tmp").unwrap();
        let mut file = target.create_temporary(&temporary).unwrap();
        file.write_all(b"descriptor-bound").unwrap();
        file.sync_all().unwrap();
        target.publish(&temporary).unwrap();
        target.cleanup(&temporary).unwrap();

        assert_eq!(
            fs::read(retained.join("result.cert")).unwrap(),
            b"descriptor-bound"
        );
        assert!(!declared.join("result.cert").exists());
        assert!(target.finish().unwrap_err().contains("ancestor changed"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_publication_refuses_temporary_final_and_symlink_collisions() {
        let root = fixture_root("output-collisions");
        let output = root.join("result.cert");
        let target = RaceResistantOutput::open(&output).unwrap();

        let occupied_temporary = CString::new(".occupied.tmp").unwrap();
        fs::write(root.join(".occupied.tmp"), b"occupied").unwrap();
        assert!(
            target
                .create_temporary(&occupied_temporary)
                .unwrap_err()
                .to_lowercase()
                .contains("exists")
        );

        let candidate = CString::new(".candidate.tmp").unwrap();
        let mut file = target.create_temporary(&candidate).unwrap();
        file.write_all(b"candidate").unwrap();
        file.sync_all().unwrap();
        fs::write(&output, b"sentinel").unwrap();
        assert!(
            target
                .publish(&candidate)
                .unwrap_err()
                .to_lowercase()
                .contains("exists")
        );
        assert_eq!(fs::read(&output).unwrap(), b"sentinel");
        target.cleanup(&candidate).unwrap();

        let link_output = root.join("link.cert");
        std::os::unix::fs::symlink("upstream.c", &link_output).unwrap();
        let link_target = RaceResistantOutput::open(&link_output).unwrap();
        let link_candidate = CString::new(".link-candidate.tmp").unwrap();
        let mut file = link_target.create_temporary(&link_candidate).unwrap();
        file.write_all(b"candidate").unwrap();
        file.sync_all().unwrap();
        assert!(
            link_target
                .publish(&link_candidate)
                .unwrap_err()
                .to_lowercase()
                .contains("exists")
        );
        assert_eq!(
            fs::read_link(&link_output).unwrap(),
            Path::new("upstream.c")
        );
        link_target.cleanup(&link_candidate).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_loader_detects_in_place_and_directory_entry_changes() {
        let root = fixture_root("descriptor-races");
        let image_path = root.join("image.bin");
        let original = fs::read(&image_path).unwrap();
        let changed = fixture_image(8);

        let mut opened = RaceResistantInputRoot::open(&root).unwrap();
        let error = opened
            .read_regular_with_hook(
                Path::new("image.bin"),
                MAX_BOUND_ARTIFACT_BYTES,
                "compiled image",
                |_| fs::write(&image_path, &changed).unwrap(),
            )
            .unwrap_err();
        assert!(error.contains("changed while it was being read"));
        fs::write(&image_path, &original).unwrap();

        let mut opened = RaceResistantInputRoot::open(&root).unwrap();
        let error = opened
            .read_regular_with_hook(
                Path::new("image.bin"),
                MAX_BOUND_ARTIFACT_BYTES,
                "compiled image",
                |_| fs::write(&image_path, &original[..original.len() / 2]).unwrap(),
            )
            .unwrap_err();
        assert!(error.contains("changed while it was being read"));
        fs::write(&image_path, &original).unwrap();

        let mut opened = RaceResistantInputRoot::open(&root).unwrap();
        let error = opened
            .read_regular_with_hook(
                Path::new("image.bin"),
                MAX_BOUND_ARTIFACT_BYTES,
                "compiled image",
                |_| {
                    let mut extended = original.clone();
                    extended.extend_from_slice(b"extended");
                    fs::write(&image_path, extended).unwrap();
                },
            )
            .unwrap_err();
        assert!(error.contains("changed while it was being read"));
        fs::write(&image_path, &original).unwrap();

        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/value.bin"), b"original").unwrap();
        let mut opened = RaceResistantInputRoot::open(&root).unwrap();
        assert_eq!(
            opened
                .read_regular(
                    Path::new("nested/value.bin"),
                    MAX_BOUND_ARTIFACT_BYTES,
                    "nested input"
                )
                .unwrap(),
            b"original"
        );
        fs::rename(root.join("nested/value.bin"), root.join("nested/value.old")).unwrap();
        fs::write(root.join("nested/value.bin"), b"replacement").unwrap();
        assert!(
            opened
                .finish()
                .unwrap_err()
                .contains("input directory changed")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_loader_refuses_final_entry_replacement_after_open() {
        let root = fixture_root("rename-after-open");
        let image_path = root.join("image.bin");
        let replacement = fixture_image(8);
        let old_path = root.join("image.old");
        let mut opened = RaceResistantInputRoot::open(&root).unwrap();
        let error = opened
            .read_regular_with_hook(
                Path::new("image.bin"),
                MAX_BOUND_ARTIFACT_BYTES,
                "compiled image",
                |_| {
                    fs::rename(&image_path, &old_path).unwrap();
                    fs::write(&image_path, &replacement).unwrap();
                },
            )
            .unwrap_err();
        assert!(error.contains("changed while it was being read"));
        assert!(opened.finish().unwrap_err().contains("input root changed"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn hard_link_aliases_refuse_even_under_distinct_paths() {
        let root = fixture_root("hard-link");
        fs::hard_link(root.join("upstream.c"), root.join("alias.c")).unwrap();
        let manifest = fs::read(root.join("inputs.txt"))
            .unwrap()
            .as_slice()
            .replace(
                b"compatibility=compat.c,compat.c",
                b"compatibility=compat.c,alias.c",
            );
        fs::write(root.join("inputs.txt"), manifest).unwrap();
        assert!(
            load_compiled_mmio_inputs(&root, Path::new("inputs.txt"))
                .err()
                .unwrap()
                .contains("aliases another manifest input")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sustained_rewrite_race_never_returns_a_mixed_snapshot() {
        let root = fixture_root("sustained-race");
        let image_path = root.join("image.bin");
        let first_image = fixture_image(7);
        let second_image = fixture_image(8);
        let first_inputs = load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).unwrap();
        let first_certificate = first_inputs.certify().unwrap();
        fs::write(&image_path, &second_image).unwrap();
        let second_inputs = load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).unwrap();
        let second_certificate = second_inputs.certify().unwrap();
        fs::write(&image_path, &first_image).unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker_path = image_path.clone();
        let worker_first = first_image.clone();
        let worker_second = second_image.clone();
        let worker = std::thread::spawn(move || {
            while worker_running.load(AtomicOrdering::Relaxed) {
                fs::write(&worker_path, &worker_first).unwrap();
                fs::write(&worker_path, &worker_second).unwrap();
            }
        });
        let mut refused = 0usize;
        for _ in 0..500 {
            match load_compiled_mmio_inputs(&root, Path::new("inputs.txt")) {
                Ok(inputs) => {
                    let certificate = inputs.certify().unwrap();
                    assert!(
                        certificate == first_certificate || certificate == second_certificate,
                        "loader returned a mixed compiled-MMIO snapshot"
                    );
                }
                Err(_) => refused += 1,
            }
        }
        running.store(false, AtomicOrdering::Relaxed);
        worker.join().unwrap();
        assert!(refused > 0);
        fs::write(image_path, first_image).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sustained_regular_and_symlink_replacement_never_verifies_the_link() {
        let root = fixture_root("sustained-symlink-race");
        let image_path = root.join("image.bin");
        let regular_candidate = root.join("candidate.regular");
        let symlink_candidate = root.join("candidate.symlink");
        let image = fixture_image(7);
        let baseline_inputs = load_compiled_mmio_inputs(&root, Path::new("inputs.txt")).unwrap();
        let baseline = baseline_inputs.certify().unwrap();
        fs::write(&regular_candidate, &image).unwrap();
        std::os::unix::fs::symlink("upstream.c", &symlink_candidate).unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker_image = image.clone();
        let worker_root = root.clone();
        let worker = std::thread::spawn(move || {
            let image_path = worker_root.join("image.bin");
            let regular = worker_root.join("candidate.regular");
            let symlink = worker_root.join("candidate.symlink");
            while worker_running.load(AtomicOrdering::Relaxed) {
                fs::rename(&regular, &image_path).unwrap();
                fs::write(&regular, &worker_image).unwrap();
                fs::rename(&symlink, &image_path).unwrap();
                std::os::unix::fs::symlink("upstream.c", &symlink).unwrap();
            }
        });
        let mut refused = 0usize;
        for _ in 0..500 {
            match load_compiled_mmio_inputs(&root, Path::new("inputs.txt")) {
                Ok(inputs) => assert_eq!(inputs.certify().unwrap(), baseline),
                Err(_) => refused += 1,
            }
        }
        running.store(false, AtomicOrdering::Relaxed);
        worker.join().unwrap();
        assert!(refused > 0);
        let _ = fs::remove_file(&image_path);
        fs::write(&image_path, image).unwrap();
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
