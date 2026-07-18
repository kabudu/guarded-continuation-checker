//! Stable client API for CQ-SAT/GCC predicate certificate workflows.
//!
//! The verifier implementation remains in the separately versioned executable.
//! This library invokes it directly without a shell and validates its advertised
//! predicate CLI contract before exposing typed certificate operations.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Predicate CLI contract understood by this crate release.
pub const PREDICATE_API_VERSION: u32 = 1;

/// A supported predicate certificate encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateVersion {
    V1,
    V2,
}

impl CertificateVersion {
    fn producer_command(self) -> &'static str {
        match self {
            Self::V1 => "certify-aiger-predicate",
            Self::V2 => "certify-aiger-predicate-v2",
        }
    }

    fn verifier_command(self) -> &'static str {
        match self {
            Self::V1 => "verify-aiger-predicate-certificate",
            Self::V2 => "verify-aiger-predicate-certificate-v2",
        }
    }
}

/// The logical result carried by a successfully checked certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateResult {
    Avoidable,
    Unavoidable,
}

/// Machine-discovered limits and formats for a compatible executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateCapabilities {
    pub cli_version: u32,
    pub certificate_versions: Vec<CertificateVersion>,
    pub portfolio_certificate_version: CertificateVersion,
    pub proof_format: String,
    pub min_relevant_inputs: usize,
    pub max_relevant_inputs: usize,
    pub max_latches: usize,
    pub max_horizon: usize,
    pub max_certificate_v2_bytes: u64,
    pub max_proof_bytes: usize,
    pub max_total_proof_bytes: usize,
}

/// A stable API error. Logical certificate results are not errors.
#[derive(Debug)]
pub enum PredicateApiError {
    Io(std::io::Error),
    CommandFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    IncompatibleContract(String),
    InvalidResponse(String),
}

impl fmt::Display for PredicateApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "predicate tool I/O failed: {error}"),
            Self::CommandFailed { exit_code, stderr } => write!(
                formatter,
                "predicate tool exited with {}: {}",
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "no status".to_string()),
                stderr.trim()
            ),
            Self::IncompatibleContract(message) => {
                write!(formatter, "incompatible predicate contract: {message}")
            }
            Self::InvalidResponse(message) => {
                write!(formatter, "invalid predicate tool response: {message}")
            }
        }
    }
}

impl std::error::Error for PredicateApiError {}

impl From<std::io::Error> for PredicateApiError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Typed, shell-free client for one CQ-SAT/GCC executable.
#[derive(Clone, Debug)]
pub struct PredicateTool {
    executable: PathBuf,
    capabilities: PredicateCapabilities,
}

impl PredicateTool {
    /// Discover and validate the executable's predicate CLI contract.
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        let executable = executable.into();
        let output = Command::new(&executable)
            .arg("predicate-cli-version")
            .output()?;
        let stdout = successful_stdout(output)?;
        let capabilities = parse_capabilities(&stdout)?;
        Ok(Self {
            executable,
            capabilities,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn capabilities(&self) -> &PredicateCapabilities {
        &self.capabilities
    }

    /// Produce a deterministic certificate without overwriting `certificate`.
    pub fn certify(
        &self,
        version: CertificateVersion,
        model: &Path,
        bad_output: usize,
        transcript: &Path,
        certificate: &Path,
    ) -> Result<PredicateResult, PredicateApiError> {
        self.require_version(version)?;
        let output = Command::new(&self.executable)
            .arg(version.producer_command())
            .arg(model)
            .arg(bad_output.to_string())
            .arg(transcript)
            .arg(certificate)
            .output()?;
        parse_result(&successful_stdout(output)?)
    }

    /// Verify a certificate against the selected model and return its result.
    pub fn verify(
        &self,
        version: CertificateVersion,
        model: &Path,
        certificate: &Path,
    ) -> Result<PredicateResult, PredicateApiError> {
        self.require_version(version)?;
        let output = Command::new(&self.executable)
            .arg(version.verifier_command())
            .arg(model)
            .arg(certificate)
            .output()?;
        parse_result(&successful_stdout(output)?)
    }

    fn require_version(&self, version: CertificateVersion) -> Result<(), PredicateApiError> {
        if !self.capabilities.certificate_versions.contains(&version) {
            return Err(PredicateApiError::IncompatibleContract(format!(
                "certificate version {version:?} is not advertised"
            )));
        }
        Ok(())
    }
}

fn successful_stdout(output: Output) -> Result<String, PredicateApiError> {
    if !output.status.success() {
        return Err(PredicateApiError::CommandFailed {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    String::from_utf8(output.stdout)
        .map_err(|_| PredicateApiError::InvalidResponse("stdout is not UTF-8".to_string()))
}

fn parse_result(stdout: &str) -> Result<PredicateResult, PredicateApiError> {
    let result = stdout
        .split_whitespace()
        .find_map(|field| field.strip_prefix("result="))
        .ok_or_else(|| PredicateApiError::InvalidResponse("result field is missing".to_string()))?;
    match result {
        "avoidable" => Ok(PredicateResult::Avoidable),
        "unavoidable" => Ok(PredicateResult::Unavoidable),
        _ => Err(PredicateApiError::InvalidResponse(format!(
            "unsupported result `{result}`"
        ))),
    }
}

fn parse_capabilities(line: &str) -> Result<PredicateCapabilities, PredicateApiError> {
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    if fields.len() != 11 || line.contains('\r') || !line.ends_with('\n') {
        return Err(PredicateApiError::InvalidResponse(
            "capability line shape is not canonical".to_string(),
        ));
    }
    let value = |index: usize, key: &str| -> Result<&str, PredicateApiError> {
        fields[index]
            .strip_prefix(&format!("{key}="))
            .ok_or_else(|| {
                PredicateApiError::InvalidResponse(format!("expected capability `{key}`"))
            })
    };
    let number = |index: usize, key: &str| -> Result<u64, PredicateApiError> {
        let text = value(index, key)?;
        if text.is_empty() || (text.len() > 1 && text.starts_with('0')) {
            return Err(PredicateApiError::InvalidResponse(format!(
                "capability `{key}` is not canonical decimal"
            )));
        }
        text.parse::<u64>().map_err(|_| {
            PredicateApiError::InvalidResponse(format!("capability `{key}` is invalid"))
        })
    };
    let cli_version = u32::try_from(number(0, "predicate_cli_version")?).map_err(|_| {
        PredicateApiError::InvalidResponse("predicate CLI version exceeds u32".to_string())
    })?;
    if cli_version != PREDICATE_API_VERSION {
        return Err(PredicateApiError::IncompatibleContract(format!(
            "expected CLI v{PREDICATE_API_VERSION}, found v{cli_version}"
        )));
    }
    if value(1, "certificate_versions")? != "1,2" {
        return Err(PredicateApiError::IncompatibleContract(
            "certificate versions must be 1,2".to_string(),
        ));
    }
    if number(2, "portfolio_certificate_version")? != 1 {
        return Err(PredicateApiError::IncompatibleContract(
            "portfolio certificate version must be 1".to_string(),
        ));
    }
    let proof_format = value(3, "proof_format")?.to_string();
    if proof_format != "varisat-native-0.2.2" {
        return Err(PredicateApiError::IncompatibleContract(format!(
            "unsupported proof format `{proof_format}`"
        )));
    }
    let as_usize = |index: usize, key: &str| -> Result<usize, PredicateApiError> {
        usize::try_from(number(index, key)?).map_err(|_| {
            PredicateApiError::InvalidResponse(format!("capability `{key}` exceeds usize"))
        })
    };
    Ok(PredicateCapabilities {
        cli_version,
        certificate_versions: vec![CertificateVersion::V1, CertificateVersion::V2],
        portfolio_certificate_version: CertificateVersion::V1,
        proof_format,
        min_relevant_inputs: as_usize(4, "min_relevant_inputs")?,
        max_relevant_inputs: as_usize(5, "max_relevant_inputs")?,
        max_latches: as_usize(6, "max_latches")?,
        max_horizon: as_usize(7, "max_horizon")?,
        max_certificate_v2_bytes: number(8, "max_certificate_v2_bytes")?,
        max_proof_bytes: as_usize(9, "max_proof_bytes")?,
        max_total_proof_bytes: as_usize(10, "max_total_proof_bytes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_parser_accepts_v1_and_rejects_drift() {
        let canonical = "predicate_cli_version=1 certificate_versions=1,2 portfolio_certificate_version=1 proof_format=varisat-native-0.2.2 min_relevant_inputs=9 max_relevant_inputs=16 max_latches=4 max_horizon=64 max_certificate_v2_bytes=16777216 max_proof_bytes=1048576 max_total_proof_bytes=8388608\n";
        let parsed = parse_capabilities(canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.certificate_versions.len(), 2);
        assert!(matches!(
            parse_capabilities(&canonical.replacen(
                "predicate_cli_version=1",
                "predicate_cli_version=2",
                1
            )),
            Err(PredicateApiError::IncompatibleContract(_))
        ));
        assert!(parse_capabilities(&canonical.replace(" max_latches=4", "")).is_err());
    }
}
