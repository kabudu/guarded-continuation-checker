//! Research-only, exact QatQ transport for canonical GCC evidence bytes.
//!
//! The envelope is not a semantic certificate. Callers must recover the exact
//! canonical bytes and pass them to the existing independent verifier.

use qatq::{
    QatcDecodeLimits, encode_qatq_exact_bytes_container,
    for_each_qatq_exact_bytes_container_chunk_with_limits,
};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAGIC: &[u8; 8] = b"GCCQATQ1";
const VERSION: u16 = 1;
// Wire value 1 is retained because QatQ's byte API emits the same canonical
// little-endian opaque-u32 QATC bytes as the former GCC adapter.
const CODEC_QATQ_EXACT_U32_WORDS: u16 = 1;
const HEADER_LEN: usize = 104;
const SHA256_LEN: usize = 32;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Fail-closed resource policy for a QatQ transport envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QatqTransportPolicy {
    pub max_envelope_bytes: usize,
    pub max_decoded_bytes: usize,
    pub max_chunks: usize,
    pub max_encoded_chunk_bytes: usize,
    pub max_values_per_chunk: usize,
    pub max_expansion_ratio: u32,
}

impl Default for QatqTransportPolicy {
    fn default() -> Self {
        Self {
            max_envelope_bytes: 16 * 1024 * 1024,
            max_decoded_bytes: 64 * 1024 * 1024,
            max_chunks: 4_096,
            max_encoded_chunk_bytes: 8 * 1024 * 1024,
            max_values_per_chunk: 1_048_576,
            max_expansion_ratio: 1_024,
        }
    }
}

/// Metadata authenticated by the canonical GCC envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QatqTransportMetadata {
    pub canonical_bytes: usize,
    pub encoded_bytes: usize,
    pub max_values_per_chunk: usize,
    pub canonical_sha256: [u8; SHA256_LEN],
    pub encoded_sha256: [u8; SHA256_LEN],
}

#[derive(Debug)]
pub enum QatqTransportError {
    InvalidEnvelope(&'static str),
    LimitExceeded(&'static str),
    IntegrityMismatch(&'static str),
    Codec(String),
    Io(io::Error),
}

impl fmt::Display for QatqTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnvelope(reason) => {
                write!(formatter, "invalid QatQ transport envelope: {reason}")
            }
            Self::LimitExceeded(limit) => {
                write!(formatter, "QatQ transport limit exceeded: {limit}")
            }
            Self::IntegrityMismatch(field) => {
                write!(formatter, "QatQ transport integrity mismatch: {field}")
            }
            Self::Codec(error) => write!(formatter, "QatQ codec rejected transport: {error}"),
            Self::Io(error) => write!(formatter, "QatQ transport I/O error: {error}"),
        }
    }
}

impl std::error::Error for QatqTransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for QatqTransportError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Encode canonical GCC evidence without changing its semantic format.
pub fn encode_qatq_transport(
    canonical: &[u8],
    max_values_per_chunk: usize,
    policy: QatqTransportPolicy,
) -> Result<Vec<u8>, QatqTransportError> {
    validate_policy(policy)?;
    if canonical.len() > policy.max_decoded_bytes {
        return Err(QatqTransportError::LimitExceeded("decoded bytes"));
    }
    if max_values_per_chunk == 0 || max_values_per_chunk > policy.max_values_per_chunk {
        return Err(QatqTransportError::LimitExceeded("values per chunk"));
    }

    let encoded = encode_qatq_exact_bytes_container(canonical, max_values_per_chunk)
        .map_err(|error| QatqTransportError::Codec(error.to_string()))?;
    validate_encoded_lengths(canonical.len(), encoded.len(), policy)?;
    let envelope_len = HEADER_LEN
        .checked_add(encoded.len())
        .ok_or(QatqTransportError::LimitExceeded("envelope bytes"))?;
    if envelope_len > policy.max_envelope_bytes {
        return Err(QatqTransportError::LimitExceeded("envelope bytes"));
    }

    let canonical_len = u64::try_from(canonical.len())
        .map_err(|_| QatqTransportError::LimitExceeded("decoded bytes"))?;
    let encoded_len = u64::try_from(encoded.len())
        .map_err(|_| QatqTransportError::LimitExceeded("encoded bytes"))?;
    let chunk_values = u32::try_from(max_values_per_chunk)
        .map_err(|_| QatqTransportError::LimitExceeded("values per chunk"))?;
    let canonical_hash = sha256(canonical);
    let encoded_hash = sha256(&encoded);

    let mut envelope = Vec::new();
    envelope
        .try_reserve_exact(envelope_len)
        .map_err(|_| QatqTransportError::LimitExceeded("envelope allocation"))?;
    envelope.extend_from_slice(MAGIC);
    envelope.extend_from_slice(&VERSION.to_be_bytes());
    envelope.extend_from_slice(&CODEC_QATQ_EXACT_U32_WORDS.to_be_bytes());
    envelope.extend_from_slice(&(HEADER_LEN as u32).to_be_bytes());
    envelope.extend_from_slice(&canonical_len.to_be_bytes());
    envelope.extend_from_slice(&encoded_len.to_be_bytes());
    envelope.extend_from_slice(&chunk_values.to_be_bytes());
    envelope.extend_from_slice(&0_u32.to_be_bytes());
    envelope.extend_from_slice(&canonical_hash);
    envelope.extend_from_slice(&encoded_hash);
    envelope.extend_from_slice(&encoded);
    debug_assert_eq!(envelope.len(), envelope_len);
    Ok(envelope)
}

/// Parse and validate envelope metadata before decoding any QatQ chunk.
pub fn inspect_qatq_transport(
    envelope: &[u8],
    policy: QatqTransportPolicy,
) -> Result<QatqTransportMetadata, QatqTransportError> {
    validate_policy(policy)?;
    if envelope.len() > policy.max_envelope_bytes {
        return Err(QatqTransportError::LimitExceeded("envelope bytes"));
    }
    if envelope.len() < HEADER_LEN {
        return Err(QatqTransportError::InvalidEnvelope("truncated header"));
    }
    if &envelope[..8] != MAGIC {
        return Err(QatqTransportError::InvalidEnvelope("magic"));
    }
    if read_u16(envelope, 8)? != VERSION {
        return Err(QatqTransportError::InvalidEnvelope("version"));
    }
    if read_u16(envelope, 10)? != CODEC_QATQ_EXACT_U32_WORDS {
        return Err(QatqTransportError::InvalidEnvelope("codec"));
    }
    if read_u32(envelope, 12)? as usize != HEADER_LEN {
        return Err(QatqTransportError::InvalidEnvelope("header length"));
    }
    let canonical_bytes = usize_from_u64(read_u64(envelope, 16)?, "decoded bytes")?;
    let encoded_bytes = usize_from_u64(read_u64(envelope, 24)?, "encoded bytes")?;
    let max_values_per_chunk = read_u32(envelope, 32)? as usize;
    if read_u32(envelope, 36)? != 0 {
        return Err(QatqTransportError::InvalidEnvelope("reserved field"));
    }
    if max_values_per_chunk == 0 || max_values_per_chunk > policy.max_values_per_chunk {
        return Err(QatqTransportError::LimitExceeded("values per chunk"));
    }
    validate_encoded_lengths(canonical_bytes, encoded_bytes, policy)?;
    let exact_len = HEADER_LEN
        .checked_add(encoded_bytes)
        .ok_or(QatqTransportError::LimitExceeded("envelope bytes"))?;
    if exact_len != envelope.len() {
        return Err(QatqTransportError::InvalidEnvelope(
            "payload length or trailing bytes",
        ));
    }

    let canonical_sha256 = read_digest(envelope, 40)?;
    let encoded_sha256 = read_digest(envelope, 72)?;
    if sha256(&envelope[HEADER_LEN..]) != encoded_sha256 {
        return Err(QatqTransportError::IntegrityMismatch("encoded SHA-256"));
    }
    Ok(QatqTransportMetadata {
        canonical_bytes,
        encoded_bytes,
        max_values_per_chunk,
        canonical_sha256,
        encoded_sha256,
    })
}

/// Recover canonical bytes into an uncommitted writer while hashing them.
///
/// The caller must not expose or commit the writer's contents unless this
/// function returns `Ok`. Use [`decode_qatq_transport_file_create_new`] for an
/// atomic file boundary.
pub fn decode_qatq_transport_to_writer(
    envelope: &[u8],
    policy: QatqTransportPolicy,
    writer: &mut impl Write,
) -> Result<QatqTransportMetadata, QatqTransportError> {
    let metadata = inspect_qatq_transport(envelope, policy)?;
    let total_values = metadata
        .canonical_bytes
        .checked_add(3)
        .ok_or(QatqTransportError::LimitExceeded("decoded bytes"))?
        / 4;
    let limits = QatcDecodeLimits {
        max_total_values: total_values,
        max_chunks: policy.max_chunks,
        max_encoded_bytes: metadata.encoded_bytes,
        max_chunk_bytes: policy.max_encoded_chunk_bytes,
    };
    let mut digest = Sha256::new();
    let encoded = &envelope[HEADER_LEN..];
    let mut writer_error = None;
    let decoded = for_each_qatq_exact_bytes_container_chunk_with_limits(
        encoded,
        metadata.canonical_bytes,
        limits,
        metadata.max_values_per_chunk,
        |bytes| {
            if let Err(error) = writer.write_all(bytes) {
                writer_error = Some(error);
                return Err(qatq::QatqError::InvalidContainer);
            }
            digest.update(bytes);
            Ok(())
        },
    );
    if let Some(error) = writer_error {
        return Err(QatqTransportError::Io(error));
    }
    decoded.map_err(|error| QatqTransportError::Codec(error.to_string()))?;
    let actual: [u8; SHA256_LEN] = digest.finalize().into();
    if actual != metadata.canonical_sha256 {
        return Err(QatqTransportError::IntegrityMismatch("decoded SHA-256"));
    }
    Ok(metadata)
}

/// Recover canonical bytes in memory after all limits and integrity checks pass.
pub fn decode_qatq_transport(
    envelope: &[u8],
    policy: QatqTransportPolicy,
) -> Result<Vec<u8>, QatqTransportError> {
    let metadata = inspect_qatq_transport(envelope, policy)?;
    let mut canonical = Vec::new();
    canonical
        .try_reserve_exact(metadata.canonical_bytes)
        .map_err(|_| QatqTransportError::LimitExceeded("decoded allocation"))?;
    decode_qatq_transport_to_writer(envelope, policy, &mut canonical)?;
    Ok(canonical)
}

/// Decode to a same-directory temporary file and publish only with create-new
/// semantics. Existing files and symlinks are never overwritten.
pub fn decode_qatq_transport_file_create_new(
    envelope: &[u8],
    output: &Path,
    policy: QatqTransportPolicy,
) -> Result<QatqTransportMetadata, QatqTransportError> {
    if fs::symlink_metadata(output).is_ok() {
        return Err(QatqTransportError::InvalidEnvelope("output already exists"));
    }
    let temporary = temporary_path(output)?;
    let result = (|| {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let mut writer = BufWriter::new(file);
        let metadata = decode_qatq_transport_to_writer(envelope, policy, &mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        fs::hard_link(&temporary, output)?;
        Ok(metadata)
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn temporary_path(output: &Path) -> Result<PathBuf, QatqTransportError> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(QatqTransportError::InvalidEnvelope("output filename"))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{name}.gcc-qatq-{}-{sequence}.tmp",
        std::process::id()
    )))
}

fn validate_policy(policy: QatqTransportPolicy) -> Result<(), QatqTransportError> {
    if policy.max_envelope_bytes < HEADER_LEN
        || policy.max_decoded_bytes == 0
        || policy.max_chunks == 0
        || policy.max_encoded_chunk_bytes == 0
        || policy.max_values_per_chunk == 0
        || policy.max_expansion_ratio == 0
    {
        return Err(QatqTransportError::InvalidEnvelope(
            "invalid resource policy",
        ));
    }
    Ok(())
}

fn validate_encoded_lengths(
    canonical_bytes: usize,
    encoded_bytes: usize,
    policy: QatqTransportPolicy,
) -> Result<(), QatqTransportError> {
    if canonical_bytes > policy.max_decoded_bytes {
        return Err(QatqTransportError::LimitExceeded("decoded bytes"));
    }
    let max_encoded = policy
        .max_envelope_bytes
        .checked_sub(HEADER_LEN)
        .ok_or(QatqTransportError::LimitExceeded("encoded bytes"))?;
    if encoded_bytes == 0 || encoded_bytes > max_encoded {
        return Err(QatqTransportError::LimitExceeded("encoded bytes"));
    }
    let permitted = encoded_bytes
        .checked_mul(policy.max_expansion_ratio as usize)
        .ok_or(QatqTransportError::LimitExceeded("expansion ratio"))?;
    if canonical_bytes > permitted {
        return Err(QatqTransportError::LimitExceeded("expansion ratio"));
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, QatqTransportError> {
    let end = offset + 2;
    Ok(u16::from_be_bytes(bytes[offset..end].try_into().map_err(
        |_| QatqTransportError::InvalidEnvelope("truncated integer"),
    )?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, QatqTransportError> {
    let end = offset + 4;
    Ok(u32::from_be_bytes(bytes[offset..end].try_into().map_err(
        |_| QatqTransportError::InvalidEnvelope("truncated integer"),
    )?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, QatqTransportError> {
    let end = offset + 8;
    Ok(u64::from_be_bytes(bytes[offset..end].try_into().map_err(
        |_| QatqTransportError::InvalidEnvelope("truncated integer"),
    )?))
}

fn read_digest(bytes: &[u8], offset: usize) -> Result<[u8; SHA256_LEN], QatqTransportError> {
    let end = offset + SHA256_LEN;
    bytes[offset..end]
        .try_into()
        .map_err(|_| QatqTransportError::InvalidEnvelope("truncated digest"))
}

fn usize_from_u64(value: u64, field: &'static str) -> Result<usize, QatqTransportError> {
    usize::try_from(value).map_err(|_| QatqTransportError::LimitExceeded(field))
}

fn sha256(bytes: &[u8]) -> [u8; SHA256_LEN] {
    Sha256::digest(bytes).into()
}
