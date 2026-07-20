//! Canonical source-to-model attestation verification.
//!
//! This module verifies retained attestation evidence against the exact source,
//! synthesis recipe, and model bytes supplied by a caller. It does not execute
//! the synthesis tool and does not establish that the tool itself is correct.

use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

pub const SOURCE_MODEL_ATTESTATION_VERSION: u32 = 1;
pub const MAX_ATTESTATION_BYTES: usize = 64 * 1024;
pub const MAX_ATTESTATION_MEMBERS: usize = 4096;
pub const MAX_ATTESTATION_SUBJECT_BYTES: usize = 256 * 1024 * 1024;

const HEADER: &str = "schema_version,member,tool,tool_revision,source_sha256,recipe_sha256,model_sha256,regenerated_sha256,byte_match,status";

/// Exact bytes expected for one ordered attestation member.
#[derive(Clone, Copy, Debug)]
pub struct SourceModelBindingInput<'a> {
    pub source: &'a [u8],
    pub recipe: &'a [u8],
    pub model: &'a [u8],
}

/// Verified facts shared by every member in a canonical attestation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceModelAttestationSummary {
    pub version: u32,
    pub member_count: usize,
    pub tool: String,
    pub tool_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceModelAttestationError {
    EvidenceTooLarge,
    SubjectTooLarge {
        member: usize,
        subject: &'static str,
    },
    InvalidEncoding,
    NonCanonical(&'static str),
    InvalidField {
        member: usize,
        field: &'static str,
    },
    MemberCountMismatch {
        evidence: usize,
        supplied: usize,
    },
    DigestMismatch {
        member: usize,
        subject: &'static str,
    },
}

impl fmt::Display for SourceModelAttestationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceTooLarge => write!(f, "source-model attestation exceeds the byte limit"),
            Self::SubjectTooLarge { member, subject } => {
                write!(
                    f,
                    "attestation member {member} {subject} exceeds the byte limit"
                )
            }
            Self::InvalidEncoding => write!(f, "source-model attestation is not valid UTF-8"),
            Self::NonCanonical(reason) => {
                write!(f, "source-model attestation is not canonical: {reason}")
            }
            Self::InvalidField { member, field } => {
                write!(
                    f,
                    "attestation member {member} has an invalid {field} field"
                )
            }
            Self::MemberCountMismatch { evidence, supplied } => write!(
                f,
                "attestation member count {evidence} does not match supplied subject count {supplied}"
            ),
            Self::DigestMismatch { member, subject } => {
                write!(
                    f,
                    "attestation member {member} {subject} digest does not match"
                )
            }
        }
    }
}

impl Error for SourceModelAttestationError {}

/// Verify canonical CSV evidence against ordered source, recipe, and model bytes.
pub fn verify_source_model_attestation(
    evidence: &[u8],
    subjects: &[SourceModelBindingInput<'_>],
) -> Result<SourceModelAttestationSummary, SourceModelAttestationError> {
    if evidence.len() > MAX_ATTESTATION_BYTES {
        return Err(SourceModelAttestationError::EvidenceTooLarge);
    }
    if subjects.len() > MAX_ATTESTATION_MEMBERS {
        return Err(SourceModelAttestationError::NonCanonical(
            "too many members",
        ));
    }
    for (member, subject) in subjects.iter().enumerate() {
        for (name, bytes) in [
            ("source", subject.source),
            ("recipe", subject.recipe),
            ("model", subject.model),
        ] {
            if bytes.len() > MAX_ATTESTATION_SUBJECT_BYTES {
                return Err(SourceModelAttestationError::SubjectTooLarge {
                    member,
                    subject: name,
                });
            }
        }
    }

    let text =
        std::str::from_utf8(evidence).map_err(|_| SourceModelAttestationError::InvalidEncoding)?;
    if text.contains(['\r', '\0']) {
        return Err(SourceModelAttestationError::NonCanonical(
            "CR and NUL bytes are forbidden",
        ));
    }
    if !text.ends_with('\n') {
        return Err(SourceModelAttestationError::NonCanonical(
            "a final newline is required",
        ));
    }
    let mut lines = text[..text.len() - 1].split('\n');
    if lines.next() != Some(HEADER) {
        return Err(SourceModelAttestationError::NonCanonical(
            "header does not match version 1",
        ));
    }

    let rows: Vec<&str> = lines.collect();
    if rows.is_empty() || rows.len() > MAX_ATTESTATION_MEMBERS {
        return Err(SourceModelAttestationError::NonCanonical(
            "member count is outside the supported range",
        ));
    }
    if rows.len() != subjects.len() {
        return Err(SourceModelAttestationError::MemberCountMismatch {
            evidence: rows.len(),
            supplied: subjects.len(),
        });
    }

    let mut common_tool: Option<&str> = None;
    let mut common_revision: Option<&str> = None;
    for (member, (row, subject)) in rows.iter().zip(subjects).enumerate() {
        if row.is_empty() {
            return Err(SourceModelAttestationError::NonCanonical("empty row"));
        }
        let fields: Vec<&str> = row.split(',').collect();
        if fields.len() != 10 {
            return Err(SourceModelAttestationError::InvalidField {
                member,
                field: "column count",
            });
        }
        if fields[0] != SOURCE_MODEL_ATTESTATION_VERSION.to_string() {
            return Err(invalid(member, "schema_version"));
        }
        if fields[1] != member.to_string() {
            return Err(invalid(member, "member"));
        }
        if fields[2] != "yosys" {
            return Err(invalid(member, "tool"));
        }
        if !is_lower_hex(fields[3], 40) {
            return Err(invalid(member, "tool_revision"));
        }
        if common_tool
            .replace(fields[2])
            .is_some_and(|tool| tool != fields[2])
        {
            return Err(invalid(member, "tool"));
        }
        if common_revision
            .replace(fields[3])
            .is_some_and(|revision| revision != fields[3])
        {
            return Err(invalid(member, "tool_revision"));
        }
        for (index, name) in [
            (4, "source_sha256"),
            (5, "recipe_sha256"),
            (6, "model_sha256"),
            (7, "regenerated_sha256"),
        ] {
            if !is_lower_hex(fields[index], 64) {
                return Err(invalid(member, name));
            }
        }
        if fields[8] != "true" || fields[9] != "attested" || fields[6] != fields[7] {
            return Err(invalid(member, "attestation result"));
        }

        verify_digest(member, "source", subject.source, fields[4])?;
        verify_digest(member, "recipe", subject.recipe, fields[5])?;
        verify_digest(member, "model", subject.model, fields[6])?;
    }

    Ok(SourceModelAttestationSummary {
        version: SOURCE_MODEL_ATTESTATION_VERSION,
        member_count: rows.len(),
        tool: common_tool.expect("non-empty rows").to_string(),
        tool_revision: common_revision.expect("non-empty rows").to_string(),
    })
}

fn invalid(member: usize, field: &'static str) -> SourceModelAttestationError {
    SourceModelAttestationError::InvalidField { member, field }
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn verify_digest(
    member: usize,
    subject: &'static str,
    bytes: &[u8],
    expected: &str,
) -> Result<(), SourceModelAttestationError> {
    let actual = digest_hex(bytes);
    if actual != expected {
        return Err(SourceModelAttestationError::DigestMismatch { member, subject });
    }
    Ok(())
}

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(bytes: &[u8]) -> String {
        digest_hex(bytes)
    }

    fn evidence(source: &[u8], recipe: &[u8], model: &[u8]) -> Vec<u8> {
        format!(
            "{HEADER}\n1,0,yosys,0123456789abcdef0123456789abcdef01234567,{},{},{},{},true,attested\n",
            digest(source),
            digest(recipe),
            digest(model),
            digest(model)
        )
        .into_bytes()
    }

    #[test]
    fn accepts_canonical_evidence_bound_to_exact_bytes() {
        let source = b"module controller; endmodule\n";
        let recipe = b"read_verilog controller.v\n";
        let model = b"aag 0 0 0 0 0\n";
        let summary = verify_source_model_attestation(
            &evidence(source, recipe, model),
            &[SourceModelBindingInput {
                source,
                recipe,
                model,
            }],
        )
        .unwrap();
        assert_eq!(summary.version, 1);
        assert_eq!(summary.member_count, 1);
        assert_eq!(summary.tool, "yosys");
    }

    #[test]
    fn rejects_model_substitution() {
        let evidence = evidence(b"source", b"recipe", b"model");
        let error = verify_source_model_attestation(
            &evidence,
            &[SourceModelBindingInput {
                source: b"source",
                recipe: b"recipe",
                model: b"other model",
            }],
        )
        .unwrap_err();
        assert_eq!(
            error,
            SourceModelAttestationError::DigestMismatch {
                member: 0,
                subject: "model"
            }
        );
    }

    #[test]
    fn rejects_noncanonical_and_false_evidence() {
        let source = b"source";
        let recipe = b"recipe";
        let model = b"model";
        let input = [SourceModelBindingInput {
            source,
            recipe,
            model,
        }];
        let canonical = evidence(source, recipe, model);

        let crlf = String::from_utf8(canonical.clone())
            .unwrap()
            .replace('\n', "\r\n");
        assert!(matches!(
            verify_source_model_attestation(crlf.as_bytes(), &input),
            Err(SourceModelAttestationError::NonCanonical(_))
        ));

        let false_result = String::from_utf8(canonical)
            .unwrap()
            .replace(",true,attested", ",false,mismatch");
        assert!(matches!(
            verify_source_model_attestation(false_result.as_bytes(), &input),
            Err(SourceModelAttestationError::InvalidField {
                field: "attestation result",
                ..
            })
        ));
    }

    #[test]
    fn rejects_truncation_and_extra_members() {
        let source = b"source";
        let recipe = b"recipe";
        let model = b"model";
        let mut truncated = evidence(source, recipe, model);
        truncated.pop();
        assert!(matches!(
            verify_source_model_attestation(
                &truncated,
                &[SourceModelBindingInput {
                    source,
                    recipe,
                    model
                }]
            ),
            Err(SourceModelAttestationError::NonCanonical(_))
        ));

        assert_eq!(
            verify_source_model_attestation(&evidence(source, recipe, model), &[]).unwrap_err(),
            SourceModelAttestationError::MemberCountMismatch {
                evidence: 1,
                supplied: 0
            }
        );
    }
}
