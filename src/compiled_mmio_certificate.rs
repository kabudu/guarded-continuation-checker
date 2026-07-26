//! Canonical certificate binding a bounded compiled-MMIO extraction to its
//! complete source, toolchain, image and symbol inputs.

use crate::riscv32imc::{
    CompiledMmioEvent, MAX_RV32_STEPS, Rv32Execution, Rv32SymbolLayout, execute_compiled_mmio,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

const MAGIC: &[u8; 8] = b"GCCMMI01";
pub const COMPILED_MMIO_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_BOUND_ARTIFACTS: usize = 64;
pub const MAX_BOUND_ARTIFACT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SYMBOL_TABLE_BYTES: usize = 1024 * 1024;
pub const MAX_TOOLCHAIN_IDENTITY_BYTES: usize = 16 * 1024;
pub const MAX_COMPILED_MMIO_CERTIFICATE_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct BoundArtifact<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub struct CompiledMmioCertificateInputs<'a> {
    pub upstream_sources: &'a [BoundArtifact<'a>],
    pub compatibility_sources: &'a [BoundArtifact<'a>],
    pub toolchain_identity: &'a [u8],
    pub image: &'a [u8],
    pub symbol_table: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioCertificate {
    pub version: u32,
    pub upstream_sources_sha256: [u8; 32],
    pub compatibility_sources_sha256: [u8; 32],
    pub toolchain_identity_sha256: [u8; 32],
    pub image_sha256: [u8; 32],
    pub symbol_table_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
    pub execution: Rv32Execution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioCertificateError(pub String);

impl fmt::Display for CompiledMmioCertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO certificate: {}", self.0)
    }
}

impl Error for CompiledMmioCertificateError {}

fn reject(message: impl Into<String>) -> CompiledMmioCertificateError {
    CompiledMmioCertificateError(message.into())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn artifact_set_digest(
    label: &str,
    artifacts: &[BoundArtifact<'_>],
) -> Result<[u8; 32], CompiledMmioCertificateError> {
    if artifacts.is_empty() || artifacts.len() > MAX_BOUND_ARTIFACTS {
        return Err(reject(format!("{label} member count is outside policy")));
    }
    let mut previous = None;
    let mut total = 0usize;
    let mut hasher = Sha256::new();
    hasher.update(b"gcc-bound-artifact-set-v1\0");
    hasher.update((artifacts.len() as u32).to_le_bytes());
    for artifact in artifacts {
        if artifact.name.is_empty()
            || artifact.name.len() > 256
            || !artifact
                .name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b'\\')
        {
            return Err(reject(format!("{label} member name is not canonical")));
        }
        if previous.is_some_and(|name| name >= artifact.name) {
            return Err(reject(format!(
                "{label} members are duplicate or not strictly sorted"
            )));
        }
        total = total
            .checked_add(artifact.bytes.len())
            .ok_or_else(|| reject(format!("{label} byte count overflow")))?;
        if total > MAX_BOUND_ARTIFACT_BYTES {
            return Err(reject(format!("{label} bytes exceed policy")));
        }
        hasher.update((artifact.name.len() as u32).to_le_bytes());
        hasher.update(artifact.name.as_bytes());
        hasher.update((artifact.bytes.len() as u64).to_le_bytes());
        hasher.update(artifact.bytes);
        previous = Some(artifact.name);
    }
    Ok(hasher.finalize().into())
}

pub fn parse_compiled_mmio_symbols(
    bytes: &[u8],
) -> Result<Rv32SymbolLayout, CompiledMmioCertificateError> {
    if bytes.is_empty() || bytes.len() > MAX_SYMBOL_TABLE_BYTES {
        return Err(reject("symbol table size is outside policy"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| reject("symbol table is not UTF-8"))?;
    let mut entry = None;
    let mut event_count = None;
    let mut events = None;
    for line in text.lines() {
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
        if fields.next().is_some() {
            return Err(reject("symbol-table row has trailing fields"));
        }
        let destination = match name {
            "gcc_firmware_entry" => &mut entry,
            "gcc_mmio_event_count" => &mut event_count,
            "gcc_mmio_events" => &mut events,
            _ => continue,
        };
        let parsed = u32::from_str_radix(address, 16)
            .map_err(|_| reject(format!("invalid address for symbol {name}")))?;
        if destination.replace(parsed).is_some() {
            return Err(reject(format!("duplicate symbol {name}")));
        }
    }
    Ok(Rv32SymbolLayout {
        entry: entry.ok_or_else(|| reject("missing symbol gcc_firmware_entry"))?,
        event_count: event_count.ok_or_else(|| reject("missing symbol gcc_mmio_event_count"))?,
        events: events.ok_or_else(|| reject("missing symbol gcc_mmio_events"))?,
    })
}

struct InputIdentity {
    upstream: [u8; 32],
    compatibility: [u8; 32],
    toolchain: [u8; 32],
    image: [u8; 32],
    symbols: [u8; 32],
    layout: Rv32SymbolLayout,
}

fn identify(
    inputs: CompiledMmioCertificateInputs<'_>,
) -> Result<InputIdentity, CompiledMmioCertificateError> {
    if inputs.toolchain_identity.is_empty()
        || inputs.toolchain_identity.len() > MAX_TOOLCHAIN_IDENTITY_BYTES
    {
        return Err(reject("toolchain identity size is outside policy"));
    }
    Ok(InputIdentity {
        upstream: artifact_set_digest("upstream source", inputs.upstream_sources)?,
        compatibility: artifact_set_digest("compatibility source", inputs.compatibility_sources)?,
        toolchain: digest(inputs.toolchain_identity),
        image: digest(inputs.image),
        symbols: digest(inputs.symbol_table),
        layout: parse_compiled_mmio_symbols(inputs.symbol_table)?,
    })
}

pub fn certify_compiled_mmio(
    inputs: CompiledMmioCertificateInputs<'_>,
) -> Result<CompiledMmioCertificate, CompiledMmioCertificateError> {
    let identity = identify(inputs)?;
    let execution = execute_compiled_mmio(inputs.image, identity.layout)
        .map_err(|error| reject(error.to_string()))?;
    Ok(CompiledMmioCertificate {
        version: COMPILED_MMIO_CERTIFICATE_VERSION,
        upstream_sources_sha256: identity.upstream,
        compatibility_sources_sha256: identity.compatibility,
        toolchain_identity_sha256: identity.toolchain,
        image_sha256: identity.image,
        symbol_table_sha256: identity.symbols,
        symbols: identity.layout,
        execution,
    })
}

pub fn verify_compiled_mmio(
    certificate: &CompiledMmioCertificate,
    inputs: CompiledMmioCertificateInputs<'_>,
) -> Result<(), CompiledMmioCertificateError> {
    if certificate.version != COMPILED_MMIO_CERTIFICATE_VERSION {
        return Err(reject("unsupported certificate version"));
    }
    let identity = identify(inputs)?;
    if certificate.upstream_sources_sha256 != identity.upstream
        || certificate.compatibility_sources_sha256 != identity.compatibility
        || certificate.toolchain_identity_sha256 != identity.toolchain
        || certificate.image_sha256 != identity.image
        || certificate.symbol_table_sha256 != identity.symbols
        || certificate.symbols != identity.layout
    {
        return Err(reject("certificate input identity mismatch"));
    }
    let execution = execute_compiled_mmio(inputs.image, identity.layout)
        .map_err(|error| reject(error.to_string()))?;
    if certificate.execution != execution {
        return Err(reject(
            "certificate does not match independently reconstructed extraction",
        ));
    }
    Ok(())
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub fn encode_compiled_mmio_certificate(
    certificate: &CompiledMmioCertificate,
) -> Result<Vec<u8>, CompiledMmioCertificateError> {
    if certificate.version != COMPILED_MMIO_CERTIFICATE_VERSION
        || certificate.execution.steps > MAX_RV32_STEPS
        || certificate.execution.events.len() > 32
        || certificate.execution.events.len() != certificate.execution.event_program_locations.len()
    {
        return Err(reject("certificate fields are outside policy"));
    }
    let mut bytes = Vec::with_capacity(256 + certificate.execution.events.len() * 16);
    bytes.extend_from_slice(MAGIC);
    push_u32(&mut bytes, certificate.version);
    bytes.extend_from_slice(&certificate.upstream_sources_sha256);
    bytes.extend_from_slice(&certificate.compatibility_sources_sha256);
    bytes.extend_from_slice(&certificate.toolchain_identity_sha256);
    bytes.extend_from_slice(&certificate.image_sha256);
    bytes.extend_from_slice(&certificate.symbol_table_sha256);
    push_u32(&mut bytes, certificate.symbols.entry);
    push_u32(&mut bytes, certificate.symbols.event_count);
    push_u32(&mut bytes, certificate.symbols.events);
    push_u32(&mut bytes, certificate.execution.return_value);
    push_u64(&mut bytes, certificate.execution.steps);
    push_u32(&mut bytes, certificate.execution.events.len() as u32);
    for (event, location) in certificate
        .execution
        .events
        .iter()
        .zip(&certificate.execution.event_program_locations)
    {
        push_u32(&mut bytes, event.operation);
        push_u32(&mut bytes, event.offset);
        push_u32(&mut bytes, event.value);
        push_u32(&mut bytes, *location);
    }
    let checksum = digest(&bytes);
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_COMPILED_MMIO_CERTIFICATE_BYTES {
        return Err(reject("encoded certificate exceeds policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], CompiledMmioCertificateError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| reject("certificate offset overflow"))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| reject("truncated certificate"))?;
        self.position = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, CompiledMmioCertificateError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed width"),
        ))
    }

    fn u64(&mut self) -> Result<u64, CompiledMmioCertificateError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed width"),
        ))
    }

    fn digest(&mut self) -> Result<[u8; 32], CompiledMmioCertificateError> {
        Ok(self.take(32)?.try_into().expect("fixed width"))
    }
}

pub fn decode_compiled_mmio_certificate(
    bytes: &[u8],
) -> Result<CompiledMmioCertificate, CompiledMmioCertificateError> {
    if bytes.len() < 232 || bytes.len() > MAX_COMPILED_MMIO_CERTIFICATE_BYTES {
        return Err(reject("certificate size is outside policy"));
    }
    let content_len = bytes
        .len()
        .checked_sub(32)
        .ok_or_else(|| reject("certificate is truncated"))?;
    if digest(&bytes[..content_len]) != bytes[content_len..] {
        return Err(reject("certificate checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..content_len],
        position: 0,
    };
    if cursor.take(8)? != MAGIC {
        return Err(reject("certificate magic mismatch"));
    }
    let version = cursor.u32()?;
    if version != COMPILED_MMIO_CERTIFICATE_VERSION {
        return Err(reject("unsupported certificate version"));
    }
    let upstream_sources_sha256 = cursor.digest()?;
    let compatibility_sources_sha256 = cursor.digest()?;
    let toolchain_identity_sha256 = cursor.digest()?;
    let image_sha256 = cursor.digest()?;
    let symbol_table_sha256 = cursor.digest()?;
    let symbols = Rv32SymbolLayout {
        entry: cursor.u32()?,
        event_count: cursor.u32()?,
        events: cursor.u32()?,
    };
    let return_value = cursor.u32()?;
    let steps = cursor.u64()?;
    if steps > MAX_RV32_STEPS {
        return Err(reject("instruction count exceeds policy"));
    }
    let event_count = cursor.u32()? as usize;
    if event_count > 32 {
        return Err(reject("event count exceeds policy"));
    }
    let mut events = Vec::with_capacity(event_count);
    let mut event_program_locations = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        events.push(CompiledMmioEvent {
            operation: cursor.u32()?,
            offset: cursor.u32()?,
            value: cursor.u32()?,
        });
        event_program_locations.push(cursor.u32()?);
    }
    if cursor.position != content_len {
        return Err(reject("certificate has trailing content"));
    }
    let certificate = CompiledMmioCertificate {
        version,
        upstream_sources_sha256,
        compatibility_sources_sha256,
        toolchain_identity_sha256,
        image_sha256,
        symbol_table_sha256,
        symbols,
        execution: Rv32Execution {
            return_value,
            steps,
            events,
            event_program_locations,
        },
    };
    if encode_compiled_mmio_certificate(&certificate)? != bytes {
        return Err(reject("certificate encoding is not canonical"));
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riscv32imc::RV32_IMAGE_BASE;

    fn instruction(opcode: u32, rd: u32, rs1: u32, immediate: u32) -> [u8; 4] {
        (((immediate & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | opcode).to_le_bytes()
    }

    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let mut image = vec![0; 0x110];
        image[..4].copy_from_slice(&instruction(0x13, 10, 0, 7));
        image[4..8].copy_from_slice(&instruction(0x67, 0, 1, 0));
        let symbols = format!(
            "{:08x} T gcc_firmware_entry\n{:08x} B gcc_mmio_event_count\n{:08x} B gcc_mmio_events\n",
            RV32_IMAGE_BASE,
            RV32_IMAGE_BASE + 0x100,
            RV32_IMAGE_BASE + 0x104
        )
        .into_bytes();
        (image, symbols)
    }

    fn inputs<'a>(
        image: &'a [u8],
        symbols: &'a [u8],
        upstream: &'a [BoundArtifact<'a>],
        compatibility: &'a [BoundArtifact<'a>],
    ) -> CompiledMmioCertificateInputs<'a> {
        CompiledMmioCertificateInputs {
            upstream_sources: upstream,
            compatibility_sources: compatibility,
            toolchain_identity: b"clang=21.1.5\ntarget=riscv32-unknown-elf\n",
            image,
            symbol_table: symbols,
        }
    }

    #[test]
    fn round_trips_and_rejects_every_single_byte_mutation() {
        let (image, symbols) = fixture();
        let upstream = [BoundArtifact {
            name: "upstream.c",
            bytes: b"upstream",
        }];
        let compatibility = [BoundArtifact {
            name: "compat.c",
            bytes: b"compatibility",
        }];
        let certificate =
            certify_compiled_mmio(inputs(&image, &symbols, &upstream, &compatibility)).unwrap();
        verify_compiled_mmio(
            &certificate,
            inputs(&image, &symbols, &upstream, &compatibility),
        )
        .unwrap();
        let encoded = encode_compiled_mmio_certificate(&certificate).unwrap();
        assert_eq!(
            decode_compiled_mmio_certificate(&encoded).unwrap(),
            certificate
        );
        for index in 0..encoded.len() {
            let mut changed = encoded.clone();
            changed[index] ^= 1;
            assert!(decode_compiled_mmio_certificate(&changed).is_err());
        }
        assert!(decode_compiled_mmio_certificate(&encoded[..encoded.len() - 1]).is_err());
        let mut extended = encoded;
        extended.push(0);
        assert!(decode_compiled_mmio_certificate(&extended).is_err());
    }

    #[test]
    fn refuses_all_bound_input_substitutions() {
        let (image, symbols) = fixture();
        let upstream = [BoundArtifact {
            name: "upstream.c",
            bytes: b"upstream",
        }];
        let compatibility = [BoundArtifact {
            name: "compat.c",
            bytes: b"compatibility",
        }];
        let certificate =
            certify_compiled_mmio(inputs(&image, &symbols, &upstream, &compatibility)).unwrap();

        let changed_upstream = [BoundArtifact {
            name: "upstream.c",
            bytes: b"changed",
        }];
        assert!(
            verify_compiled_mmio(
                &certificate,
                inputs(&image, &symbols, &changed_upstream, &compatibility)
            )
            .is_err()
        );
        let changed_compatibility = [BoundArtifact {
            name: "compat.c",
            bytes: b"changed",
        }];
        assert!(
            verify_compiled_mmio(
                &certificate,
                inputs(&image, &symbols, &upstream, &changed_compatibility)
            )
            .is_err()
        );
        let mut changed_image = image.clone();
        changed_image[0] ^= 1;
        assert!(
            verify_compiled_mmio(
                &certificate,
                inputs(&changed_image, &symbols, &upstream, &compatibility)
            )
            .is_err()
        );
        let changed_toolchain = CompiledMmioCertificateInputs {
            upstream_sources: &upstream,
            compatibility_sources: &compatibility,
            toolchain_identity: b"clang=changed\n",
            image: &image,
            symbol_table: &symbols,
        };
        assert!(verify_compiled_mmio(&certificate, changed_toolchain).is_err());
        let mut changed_symbols = symbols.clone();
        changed_symbols[0] = b'9';
        assert!(
            verify_compiled_mmio(
                &certificate,
                inputs(&image, &changed_symbols, &upstream, &compatibility)
            )
            .is_err()
        );
    }

    #[test]
    fn refuses_duplicate_or_reordered_source_members() {
        let duplicated = [
            BoundArtifact {
                name: "same",
                bytes: b"a",
            },
            BoundArtifact {
                name: "same",
                bytes: b"b",
            },
        ];
        assert!(artifact_set_digest("source", &duplicated).is_err());
        let reordered = [
            BoundArtifact {
                name: "z",
                bytes: b"a",
            },
            BoundArtifact {
                name: "a",
                bytes: b"b",
            },
        ];
        assert!(artifact_set_digest("source", &reordered).is_err());
    }
}
