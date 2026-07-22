//! Canonical content-addressed batches for revision-local certificates.

use crate::btor2::NodeId;
use crate::revision_local::{
    BoundEvidence, BoundedAnswerSummary, BoundedQuery, EvidenceSection, LocalEvidence,
    RevisionLocalCertificate, RevisionLocalError, ValidatedLocalArtifact,
    encode_local_relation_certificate, evidence_digest, produce_local_relation,
    produce_revision_with_retained_components, source_digest, validate_local_artifact,
    verify_revision_with_retained_components,
};
use std::collections::{BTreeMap, BTreeSet};

pub const REVISION_BATCH_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_REVISION_BATCH_COMPONENTS: usize = 64;
pub const MAX_REVISION_BATCH_ENTRIES: usize = 256;
pub const MAX_REVISION_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"GCCRLB01";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionBatchSection {
    pub source_sha256: [u8; 32],
    pub evidence_sha256: [u8; 32],
    pub evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionBatchEntry {
    pub left_evidence_sha256: [u8; 32],
    pub right_evidence_sha256: [u8; 32],
    pub interface: BoundEvidence,
    pub final_evidence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionBatchCertificate {
    pub sections: Vec<RevisionBatchSection>,
    pub entries: Vec<RevisionBatchEntry>,
}

#[derive(Clone, Copy)]
pub struct RevisionBatchComponent<'a> {
    pub source: &'a [u8],
    pub outputs: &'a [NodeId],
}

#[derive(Clone)]
pub struct RevisionBatchQuery<'a> {
    pub left_component: usize,
    pub right_component: usize,
    pub interface_source: &'a [u8],
    pub query: BoundedQuery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionBatchProductionSummary {
    pub shared_sections: usize,
    pub entries: usize,
    pub candidate_valuations: usize,
    pub certificate_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionBatchVerificationSummary {
    pub shared_sections_verified: usize,
    pub entries_verified: usize,
    pub answers: Vec<BoundedAnswerSummary>,
}

fn reject(section: EvidenceSection, message: impl Into<String>) -> RevisionLocalError {
    RevisionLocalError {
        section,
        message: message.into(),
    }
}

fn append_len(output: &mut Vec<u8>, len: usize) -> Result<(), RevisionLocalError> {
    let len = u32::try_from(len)
        .map_err(|_| reject(EvidenceSection::Envelope, "batch length exceeds u32"))?;
    output.extend_from_slice(&len.to_be_bytes());
    Ok(())
}

fn encode_entry(entry: &RevisionBatchEntry) -> Result<Vec<u8>, RevisionLocalError> {
    if entry.interface.evidence.len() > crate::revision_local::MAX_INTERFACE_SECTION_BYTES
        || entry.final_evidence.len() > crate::revision_local::MAX_FINAL_SECTION_BYTES
    {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch entry section exceeds byte limit",
        ));
    }
    if source_digest(&entry.interface.evidence) != entry.interface.source_sha256 {
        return Err(reject(
            EvidenceSection::Interface,
            "batch interface source binding is invalid",
        ));
    }
    let mut output = Vec::new();
    output.extend_from_slice(&entry.left_evidence_sha256);
    output.extend_from_slice(&entry.right_evidence_sha256);
    output.extend_from_slice(&entry.interface.source_sha256);
    append_len(&mut output, entry.interface.evidence.len())?;
    output.extend_from_slice(&entry.interface.evidence);
    append_len(&mut output, entry.final_evidence.len())?;
    output.extend_from_slice(&entry.final_evidence);
    Ok(output)
}

fn validate_structure(certificate: &RevisionBatchCertificate) -> Result<(), RevisionLocalError> {
    if certificate.sections.is_empty()
        || certificate.sections.len() > MAX_REVISION_BATCH_COMPONENTS
        || certificate.entries.is_empty()
        || certificate.entries.len() > MAX_REVISION_BATCH_ENTRIES
    {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch section or entry count is invalid",
        ));
    }
    let mut previous_section = None;
    let mut section_digests = BTreeSet::new();
    for section in &certificate.sections {
        if section.evidence.is_empty()
            || section.evidence.len() > crate::revision_local::MAX_LOCAL_SECTION_BYTES
            || evidence_digest(&section.evidence) != section.evidence_sha256
        {
            return Err(reject(
                EvidenceSection::Envelope,
                "batch shared section binding is invalid",
            ));
        }
        if previous_section.is_some_and(|previous| previous >= section.evidence_sha256)
            || !section_digests.insert(section.evidence_sha256)
        {
            return Err(reject(
                EvidenceSection::Envelope,
                "batch shared sections are not strictly digest ordered",
            ));
        }
        previous_section = Some(section.evidence_sha256);
    }
    let mut previous_entry: Option<Vec<u8>> = None;
    let mut referenced = BTreeSet::new();
    for entry in &certificate.entries {
        if !section_digests.contains(&entry.left_evidence_sha256)
            || !section_digests.contains(&entry.right_evidence_sha256)
        {
            return Err(reject(
                EvidenceSection::Envelope,
                "batch entry references an unknown shared section",
            ));
        }
        referenced.insert(entry.left_evidence_sha256);
        referenced.insert(entry.right_evidence_sha256);
        let encoded = encode_entry(entry)?;
        if previous_entry
            .as_ref()
            .is_some_and(|previous| previous >= &encoded)
        {
            return Err(reject(
                EvidenceSection::Envelope,
                "batch entries are not strictly canonical",
            ));
        }
        previous_entry = Some(encoded);
    }
    if referenced.len() != certificate.sections.len() {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch contains an unreferenced shared section",
        ));
    }
    Ok(())
}

pub fn encode_revision_batch(
    certificate: &RevisionBatchCertificate,
) -> Result<Vec<u8>, RevisionLocalError> {
    validate_structure(certificate)?;
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&REVISION_BATCH_CERTIFICATE_VERSION.to_be_bytes());
    append_len(&mut output, certificate.sections.len())?;
    for section in &certificate.sections {
        output.extend_from_slice(&section.source_sha256);
        output.extend_from_slice(&section.evidence_sha256);
        append_len(&mut output, section.evidence.len())?;
        output.extend_from_slice(&section.evidence);
    }
    append_len(&mut output, certificate.entries.len())?;
    for entry in &certificate.entries {
        output.extend_from_slice(&encode_entry(entry)?);
    }
    if output.len() > MAX_REVISION_BATCH_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch certificate exceeds byte limit",
        ));
    }
    Ok(output)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], RevisionLocalError> {
        let end = self
            .offset
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject(EvidenceSection::Envelope, "batch is truncated"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, RevisionLocalError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn digest(&mut self) -> Result<[u8; 32], RevisionLocalError> {
        Ok(self.take(32)?.try_into().expect("32 bytes"))
    }

    fn bytes(&mut self, max: usize) -> Result<Vec<u8>, RevisionLocalError> {
        let len = usize::try_from(self.u32()?).expect("u32 fits usize");
        if len > max {
            return Err(reject(
                EvidenceSection::Envelope,
                "batch section exceeds byte limit",
            ));
        }
        Ok(self.take(len)?.to_vec())
    }
}

pub fn decode_revision_batch(bytes: &[u8]) -> Result<RevisionBatchCertificate, RevisionLocalError> {
    if bytes.len() > MAX_REVISION_BATCH_BYTES {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch certificate exceeds byte limit",
        ));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(MAGIC.len())? != MAGIC {
        return Err(reject(EvidenceSection::Envelope, "invalid batch magic"));
    }
    if decoder.u32()? != REVISION_BATCH_CERTIFICATE_VERSION {
        return Err(reject(
            EvidenceSection::Envelope,
            "unsupported batch version",
        ));
    }
    let section_count = usize::try_from(decoder.u32()?).expect("u32 fits usize");
    if !(1..=MAX_REVISION_BATCH_COMPONENTS).contains(&section_count) {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch shared section count is invalid",
        ));
    }
    let mut sections = Vec::with_capacity(section_count);
    for _ in 0..section_count {
        sections.push(RevisionBatchSection {
            source_sha256: decoder.digest()?,
            evidence_sha256: decoder.digest()?,
            evidence: decoder.bytes(crate::revision_local::MAX_LOCAL_SECTION_BYTES)?,
        });
    }
    let entry_count = usize::try_from(decoder.u32()?).expect("u32 fits usize");
    if !(1..=MAX_REVISION_BATCH_ENTRIES).contains(&entry_count) {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch entry count is invalid",
        ));
    }
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        entries.push(RevisionBatchEntry {
            left_evidence_sha256: decoder.digest()?,
            right_evidence_sha256: decoder.digest()?,
            interface: BoundEvidence {
                source_sha256: decoder.digest()?,
                evidence: decoder.bytes(crate::revision_local::MAX_INTERFACE_SECTION_BYTES)?,
            },
            final_evidence: decoder.bytes(crate::revision_local::MAX_FINAL_SECTION_BYTES)?,
        });
    }
    if decoder.offset != bytes.len() {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch certificate has trailing bytes",
        ));
    }
    let certificate = RevisionBatchCertificate { sections, entries };
    validate_structure(&certificate)?;
    if encode_revision_batch(&certificate)? != bytes {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch certificate is not canonical",
        ));
    }
    Ok(certificate)
}

pub fn produce_revision_batch(
    components: &[RevisionBatchComponent<'_>],
    queries: &[RevisionBatchQuery<'_>],
) -> Result<(RevisionBatchCertificate, RevisionBatchProductionSummary), RevisionLocalError> {
    if components.is_empty()
        || components.len() > MAX_REVISION_BATCH_COMPONENTS
        || queries.is_empty()
        || queries.len() > MAX_REVISION_BATCH_ENTRIES
    {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch production count is invalid",
        ));
    }
    let mut records = Vec::with_capacity(components.len());
    let mut candidate_valuations = 0usize;
    for (original_index, component) in components.iter().enumerate() {
        let relation = produce_local_relation(component.source, component.outputs)?;
        let evidence = encode_local_relation_certificate(&relation)?;
        let artifact = validate_local_artifact(component.source, &evidence, EvidenceSection::Left)?;
        candidate_valuations = candidate_valuations
            .checked_add(artifact.summary().candidate_valuations)
            .ok_or_else(|| reject(EvidenceSection::Envelope, "batch work count overflows"))?;
        records.push((
            RevisionBatchSection {
                source_sha256: source_digest(component.source),
                evidence_sha256: evidence_digest(&evidence),
                evidence,
            },
            artifact,
            original_index,
        ));
    }
    records.sort_by_key(|record| record.0.evidence_sha256);
    if records
        .windows(2)
        .any(|pair| pair[0].0.evidence_sha256 == pair[1].0.evidence_sha256)
    {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch production components are duplicated",
        ));
    }
    let mut original_to_sorted = vec![0usize; records.len()];
    for (sorted, record) in records.iter().enumerate() {
        original_to_sorted[record.2] = sorted;
    }
    let mut entries = Vec::with_capacity(queries.len());
    for request in queries {
        let left = *original_to_sorted
            .get(request.left_component)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Envelope,
                    "batch left component index is invalid",
                )
            })?;
        let right = *original_to_sorted
            .get(request.right_component)
            .ok_or_else(|| {
                reject(
                    EvidenceSection::Envelope,
                    "batch right component index is invalid",
                )
            })?;
        let (standalone, _, _) = produce_revision_with_retained_components(
            &records[left].1,
            &records[right].1,
            request.interface_source,
            &request.query,
        )?;
        entries.push(RevisionBatchEntry {
            left_evidence_sha256: records[left].0.evidence_sha256,
            right_evidence_sha256: records[right].0.evidence_sha256,
            interface: standalone.interface,
            final_evidence: standalone.final_evidence,
        });
    }
    let mut keyed_entries = entries
        .into_iter()
        .map(|entry| Ok((encode_entry(&entry)?, entry)))
        .collect::<Result<Vec<_>, RevisionLocalError>>()?;
    keyed_entries.sort_by(|left, right| left.0.cmp(&right.0));
    let entries = keyed_entries.into_iter().map(|(_, entry)| entry).collect();
    let sections = records.into_iter().map(|record| record.0).collect();
    let certificate = RevisionBatchCertificate { sections, entries };
    let certificate_bytes = encode_revision_batch(&certificate)?.len();
    Ok((
        certificate,
        RevisionBatchProductionSummary {
            shared_sections: components.len(),
            entries: queries.len(),
            candidate_valuations,
            certificate_bytes,
        },
    ))
}

pub fn verify_revision_batch(
    component_sources: &[&[u8]],
    certificate_bytes: &[u8],
) -> Result<RevisionBatchVerificationSummary, RevisionLocalError> {
    let certificate = decode_revision_batch(certificate_bytes)?;
    let mut sources = BTreeMap::new();
    for source in component_sources {
        if sources.insert(source_digest(source), *source).is_some() {
            return Err(reject(
                EvidenceSection::Envelope,
                "batch verifier source digest is duplicated",
            ));
        }
    }
    let required_sources = certificate
        .sections
        .iter()
        .map(|section| section.source_sha256)
        .collect::<BTreeSet<_>>();
    if sources.keys().copied().collect::<BTreeSet<_>>() != required_sources {
        return Err(reject(
            EvidenceSection::Envelope,
            "batch verifier sources do not exactly match shared sections",
        ));
    }
    let mut artifacts: BTreeMap<[u8; 32], ValidatedLocalArtifact> = BTreeMap::new();
    for section in &certificate.sections {
        let source = sources.get(&section.source_sha256).ok_or_else(|| {
            reject(
                EvidenceSection::Envelope,
                "batch verifier is missing a component source",
            )
        })?;
        let artifact = validate_local_artifact(source, &section.evidence, EvidenceSection::Left)?;
        artifacts.insert(section.evidence_sha256, artifact);
    }
    let mut answers = Vec::with_capacity(certificate.entries.len());
    for entry in &certificate.entries {
        let left = &artifacts[&entry.left_evidence_sha256];
        let right = &artifacts[&entry.right_evidence_sha256];
        let standalone = RevisionLocalCertificate {
            left: LocalEvidence {
                source_sha256: *left.source_sha256(),
                evidence: left.encoded().to_vec(),
            },
            right: LocalEvidence {
                source_sha256: *right.source_sha256(),
                evidence: right.encoded().to_vec(),
            },
            interface: entry.interface.clone(),
            final_evidence: entry.final_evidence.clone(),
        };
        let (summary, _) = verify_revision_with_retained_components(
            left,
            right,
            &entry.interface.evidence,
            &standalone,
        )?;
        answers.push(summary.answer);
    }
    Ok(RevisionBatchVerificationSummary {
        shared_sections_verified: certificate.sections.len(),
        entries_verified: certificate.entries.len(),
        answers,
    })
}

pub fn extract_revision_batch_certificates(
    certificate_bytes: &[u8],
) -> Result<Vec<RevisionLocalCertificate>, RevisionLocalError> {
    let certificate = decode_revision_batch(certificate_bytes)?;
    let sections = certificate
        .sections
        .iter()
        .map(|section| (section.evidence_sha256, section))
        .collect::<BTreeMap<_, _>>();
    certificate
        .entries
        .into_iter()
        .map(|entry| {
            let left = sections[&entry.left_evidence_sha256];
            let right = sections[&entry.right_evidence_sha256];
            Ok(RevisionLocalCertificate {
                left: LocalEvidence {
                    source_sha256: left.source_sha256,
                    evidence: left.evidence.clone(),
                },
                right: LocalEvidence {
                    source_sha256: right.source_sha256,
                    evidence: right.evidence.clone(),
                },
                interface: entry.interface,
                final_evidence: entry.final_evidence,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revision_local::{
        ComponentSide, InterfaceWire, WordInterfaceContract, encode_word_interface_contract,
    };

    const LEFT: &[u8] = b"1 sort bitvec 1\n2 state 1 state\n3 zero 1\n4 init 1 2 3\n5 input 1 sensed\n6 next 1 2 5\n7 output 2 state_out\n";
    const RIGHT: &[u8] = b"1 sort bitvec 1\n2 state 1 state\n3 zero 1\n4 init 1 2 3\n5 input 1 command\n6 next 1 2 5\n7 output 2 state_out\n";

    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let first = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Left,
                output: 2,
                to_input: 5,
            }],
            external_inputs: None,
        })
        .unwrap()
        .into_bytes();
        let second = encode_word_interface_contract(&WordInterfaceContract {
            wires: vec![InterfaceWire {
                from: ComponentSide::Right,
                output: 2,
                to_input: 5,
            }],
            external_inputs: None,
        })
        .unwrap()
        .into_bytes();
        (first, second)
    }

    #[test]
    fn batch_shares_sections_and_verifies_every_query() {
        let (first, second) = fixture();
        let components = [
            RevisionBatchComponent {
                source: LEFT,
                outputs: &[2],
            },
            RevisionBatchComponent {
                source: RIGHT,
                outputs: &[2],
            },
        ];
        let queries = [
            RevisionBatchQuery {
                left_component: 0,
                right_component: 1,
                interface_source: &first,
                query: BoundedQuery {
                    horizon: 1,
                    bad_side: ComponentSide::Left,
                    bad_output: 2,
                },
            },
            RevisionBatchQuery {
                left_component: 0,
                right_component: 1,
                interface_source: &second,
                query: BoundedQuery {
                    horizon: 1,
                    bad_side: ComponentSide::Right,
                    bad_output: 2,
                },
            },
        ];
        let (certificate, production) = produce_revision_batch(&components, &queries).unwrap();
        let encoded = encode_revision_batch(&certificate).unwrap();
        assert_eq!(decode_revision_batch(&encoded).unwrap(), certificate);
        assert_eq!(production.shared_sections, 2);
        assert_eq!(production.entries, 2);
        let verified = verify_revision_batch(&[LEFT, RIGHT], &encoded).unwrap();
        assert_eq!(verified.shared_sections_verified, 2);
        assert_eq!(verified.entries_verified, 2);
        assert_eq!(verified.answers.len(), 2);
        let extracted = extract_revision_batch_certificates(&encoded).unwrap();
        assert_eq!(extracted.len(), 2);
        for standalone in extracted {
            crate::revision_local::verify_revision_local_certificate(
                LEFT,
                RIGHT,
                &standalone.interface.evidence,
                &standalone,
            )
            .unwrap();
        }
    }

    #[test]
    fn every_truncation_and_common_structure_attack_fails_closed() {
        let (first, second) = fixture();
        let components = [
            RevisionBatchComponent {
                source: LEFT,
                outputs: &[2],
            },
            RevisionBatchComponent {
                source: RIGHT,
                outputs: &[2],
            },
        ];
        let queries = [
            RevisionBatchQuery {
                left_component: 0,
                right_component: 1,
                interface_source: &first,
                query: BoundedQuery {
                    horizon: 0,
                    bad_side: ComponentSide::Left,
                    bad_output: 2,
                },
            },
            RevisionBatchQuery {
                left_component: 0,
                right_component: 1,
                interface_source: &second,
                query: BoundedQuery {
                    horizon: 0,
                    bad_side: ComponentSide::Right,
                    bad_output: 2,
                },
            },
        ];
        let (certificate, _) = produce_revision_batch(&components, &queries).unwrap();
        let encoded = encode_revision_batch(&certificate).unwrap();
        for length in 0..encoded.len() {
            assert!(decode_revision_batch(&encoded[..length]).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_revision_batch(&trailing).is_err());

        let mut excessive_count = encoded.clone();
        excessive_count[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_revision_batch(&excessive_count).is_err());
        let mut excessive_section = encoded.clone();
        excessive_section[80..84].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_revision_batch(&excessive_section).is_err());
        let mut corrupted_evidence = encoded.clone();
        corrupted_evidence[84] ^= 1;
        assert!(decode_revision_batch(&corrupted_evidence).is_err());

        let mut reordered = certificate.clone();
        reordered.sections.swap(0, 1);
        assert!(encode_revision_batch(&reordered).is_err());
        let mut reordered_entries = certificate.clone();
        reordered_entries.entries.swap(0, 1);
        assert!(encode_revision_batch(&reordered_entries).is_err());
        let mut duplicated = certificate.clone();
        duplicated.entries.push(duplicated.entries[0].clone());
        assert!(encode_revision_batch(&duplicated).is_err());
        let mut unreferenced = certificate.clone();
        let only = unreferenced.sections[0].evidence_sha256;
        unreferenced.entries.retain(|entry| {
            entry.left_evidence_sha256 == only && entry.right_evidence_sha256 == only
        });
        assert!(encode_revision_batch(&unreferenced).is_err());
        assert!(verify_revision_batch(&[LEFT], &encoded).is_err());
        assert!(verify_revision_batch(&[LEFT, RIGHT, b"unreferenced source"], &encoded).is_err());

        let mut substituted = certificate.clone();
        substituted.entries[0].right_evidence_sha256 = substituted.entries[0].left_evidence_sha256;
        let mut keyed = substituted
            .entries
            .into_iter()
            .map(|entry| (encode_entry(&entry).unwrap(), entry))
            .collect::<Vec<_>>();
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        substituted.entries = keyed.into_iter().map(|(_, entry)| entry).collect();
        let substituted = encode_revision_batch(&substituted).unwrap();
        assert!(verify_revision_batch(&[LEFT, RIGHT], &substituted).is_err());
    }
}
