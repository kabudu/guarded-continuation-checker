//! Canonical envelope primitives for revision-local component evidence.

use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

pub const REVISION_LOCAL_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_LOCAL_SECTION_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_INTERFACE_SECTION_BYTES: usize = 1024 * 1024;
pub const MAX_FINAL_SECTION_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_REVISION_LOCAL_CERTIFICATE_BYTES: usize = 50 * 1024 * 1024;

const MAGIC: &[u8; 8] = b"GCCRLCP1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceSection {
    Left,
    Right,
    Interface,
    Final,
    Envelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEvidence {
    pub source_sha256: [u8; 32],
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundEvidence {
    pub source_sha256: [u8; 32],
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionLocalCertificate {
    pub left: LocalEvidence,
    pub right: LocalEvidence,
    pub interface: BoundEvidence,
    pub final_evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionLocalError {
    pub section: EvidenceSection,
    pub message: String,
}

impl fmt::Display for RevisionLocalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} evidence: {}", self.section, self.message)
    }
}

impl Error for RevisionLocalError {}

fn reject(section: EvidenceSection, message: impl Into<String>) -> RevisionLocalError {
    RevisionLocalError {
        section,
        message: message.into(),
    }
}

pub fn source_digest(source: &[u8]) -> [u8; 32] {
    Sha256::digest(source).into()
}

pub fn evidence_digest(evidence: &[u8]) -> [u8; 32] {
    Sha256::digest(evidence).into()
}

pub fn verify_source_bindings(
    left_source: &[u8],
    right_source: &[u8],
    interface_source: &[u8],
    certificate: &RevisionLocalCertificate,
) -> Result<(), RevisionLocalError> {
    if source_digest(left_source) != certificate.left.source_sha256 {
        return Err(reject(EvidenceSection::Left, "source binding is invalid"));
    }
    if source_digest(right_source) != certificate.right.source_sha256 {
        return Err(reject(EvidenceSection::Right, "source binding is invalid"));
    }
    if source_digest(interface_source) != certificate.interface.source_sha256 {
        return Err(reject(
            EvidenceSection::Interface,
            "source binding is invalid",
        ));
    }
    Ok(())
}

pub fn unchanged_local_evidence(
    previous: &RevisionLocalCertificate,
    next: &RevisionLocalCertificate,
    section: EvidenceSection,
) -> Result<bool, RevisionLocalError> {
    match section {
        EvidenceSection::Left => Ok(previous.left == next.left),
        EvidenceSection::Right => Ok(previous.right == next.right),
        _ => Err(reject(
            EvidenceSection::Envelope,
            "reuse comparison requires a local component section",
        )),
    }
}

fn append_section(
    output: &mut Vec<u8>,
    section: EvidenceSection,
    digest: Option<&[u8; 32]>,
    bytes: &[u8],
    limit: usize,
) -> Result<(), RevisionLocalError> {
    if bytes.is_empty() {
        return Err(reject(section, "section is empty"));
    }
    if bytes.len() > limit {
        return Err(reject(section, "section exceeds byte limit"));
    }
    if let Some(digest) = digest {
        output.extend_from_slice(digest);
    }
    let length = u32::try_from(bytes.len())
        .map_err(|_| reject(section, "section length cannot be encoded"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

pub fn encode_revision_local_certificate(
    certificate: &RevisionLocalCertificate,
) -> Result<Vec<u8>, RevisionLocalError> {
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&REVISION_LOCAL_CERTIFICATE_VERSION.to_be_bytes());
    append_section(
        &mut output,
        EvidenceSection::Left,
        Some(&certificate.left.source_sha256),
        &certificate.left.evidence,
        MAX_LOCAL_SECTION_BYTES,
    )?;
    append_section(
        &mut output,
        EvidenceSection::Right,
        Some(&certificate.right.source_sha256),
        &certificate.right.evidence,
        MAX_LOCAL_SECTION_BYTES,
    )?;
    append_section(
        &mut output,
        EvidenceSection::Interface,
        Some(&certificate.interface.source_sha256),
        &certificate.interface.evidence,
        MAX_INTERFACE_SECTION_BYTES,
    )?;
    append_section(
        &mut output,
        EvidenceSection::Final,
        None,
        &certificate.final_evidence,
        MAX_FINAL_SECTION_BYTES,
    )?;
    if output.len() > MAX_REVISION_LOCAL_CERTIFICATE_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "certificate exceeds byte limit",
        ));
    }
    Ok(output)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn take(
        &mut self,
        count: usize,
        section: EvidenceSection,
    ) -> Result<&'a [u8], RevisionLocalError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject(section, "certificate is truncated"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self, section: EvidenceSection) -> Result<u32, RevisionLocalError> {
        let bytes: [u8; 4] = self.take(4, section)?.try_into().expect("fixed size");
        Ok(u32::from_be_bytes(bytes))
    }

    fn digest(&mut self, section: EvidenceSection) -> Result<[u8; 32], RevisionLocalError> {
        Ok(self.take(32, section)?.try_into().expect("fixed size"))
    }

    fn section(
        &mut self,
        section: EvidenceSection,
        limit: usize,
    ) -> Result<Vec<u8>, RevisionLocalError> {
        let length = usize::try_from(self.u32(section)?).expect("u32 fits usize");
        if length == 0 {
            return Err(reject(section, "section is empty"));
        }
        if length > limit {
            return Err(reject(section, "section exceeds byte limit"));
        }
        Ok(self.take(length, section)?.to_vec())
    }
}

pub fn decode_revision_local_certificate(
    bytes: &[u8],
) -> Result<RevisionLocalCertificate, RevisionLocalError> {
    if bytes.len() > MAX_REVISION_LOCAL_CERTIFICATE_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "certificate exceeds byte limit",
        ));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(MAGIC.len(), EvidenceSection::Envelope)? != MAGIC {
        return Err(reject(EvidenceSection::Envelope, "invalid magic"));
    }
    if decoder.u32(EvidenceSection::Envelope)? != REVISION_LOCAL_CERTIFICATE_VERSION {
        return Err(reject(EvidenceSection::Envelope, "unsupported version"));
    }
    let left_digest = decoder.digest(EvidenceSection::Left)?;
    let left = decoder.section(EvidenceSection::Left, MAX_LOCAL_SECTION_BYTES)?;
    let right_digest = decoder.digest(EvidenceSection::Right)?;
    let right = decoder.section(EvidenceSection::Right, MAX_LOCAL_SECTION_BYTES)?;
    let interface_digest = decoder.digest(EvidenceSection::Interface)?;
    let interface = decoder.section(EvidenceSection::Interface, MAX_INTERFACE_SECTION_BYTES)?;
    let final_evidence = decoder.section(EvidenceSection::Final, MAX_FINAL_SECTION_BYTES)?;
    if decoder.offset != bytes.len() {
        return Err(reject(
            EvidenceSection::Envelope,
            "certificate has trailing bytes",
        ));
    }
    let certificate = RevisionLocalCertificate {
        left: LocalEvidence {
            source_sha256: left_digest,
            evidence: left,
        },
        right: LocalEvidence {
            source_sha256: right_digest,
            evidence: right,
        },
        interface: BoundEvidence {
            source_sha256: interface_digest,
            evidence: interface,
        },
        final_evidence,
    };
    if encode_revision_local_certificate(&certificate)? != bytes {
        return Err(reject(EvidenceSection::Envelope, "noncanonical encoding"));
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> RevisionLocalCertificate {
        RevisionLocalCertificate {
            left: LocalEvidence {
                source_sha256: source_digest(b"left-v1"),
                evidence: b"left-proof".to_vec(),
            },
            right: LocalEvidence {
                source_sha256: source_digest(b"right-v1"),
                evidence: b"right-proof".to_vec(),
            },
            interface: BoundEvidence {
                source_sha256: source_digest(b"word-wire"),
                evidence: b"interface-proof".to_vec(),
            },
            final_evidence: b"safe-proof".to_vec(),
        }
    }

    #[test]
    fn canonical_round_trip_and_source_binding() {
        let certificate = fixture();
        let encoded = encode_revision_local_certificate(&certificate).unwrap();
        assert_eq!(
            decode_revision_local_certificate(&encoded).unwrap(),
            certificate
        );
        verify_source_bindings(b"left-v1", b"right-v1", b"word-wire", &certificate).unwrap();
    }

    #[test]
    fn changed_right_preserves_left_bytes() {
        let previous = fixture();
        let mut next = fixture();
        next.right.source_sha256 = source_digest(b"right-v2");
        next.right.evidence = b"right-v2-proof".to_vec();
        next.final_evidence = b"unsafe-witness".to_vec();
        assert!(unchanged_local_evidence(&previous, &next, EvidenceSection::Left).unwrap());
        assert!(!unchanged_local_evidence(&previous, &next, EvidenceSection::Right).unwrap());
    }

    #[test]
    fn source_drift_is_attributed_to_smallest_section() {
        let certificate = fixture();
        let error = verify_source_bindings(b"left-v1", b"right-v2", b"word-wire", &certificate)
            .unwrap_err();
        assert_eq!(error.section, EvidenceSection::Right);
    }

    #[test]
    fn every_truncation_and_trailing_byte_fail_closed() {
        let encoded = encode_revision_local_certificate(&fixture()).unwrap();
        for length in 0..encoded.len() {
            assert!(decode_revision_local_certificate(&encoded[..length]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        let error = decode_revision_local_certificate(&trailing).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Envelope);
    }

    #[test]
    fn section_length_attack_is_rejected_before_allocation() {
        let mut encoded = encode_revision_local_certificate(&fixture()).unwrap();
        let left_length_offset = MAGIC.len() + 4 + 32;
        encoded[left_length_offset..left_length_offset + 4]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        let error = decode_revision_local_certificate(&encoded).unwrap_err();
        assert_eq!(error.section, EvidenceSection::Left);
        assert_eq!(error.message, "section exceeds byte limit");
    }
}
