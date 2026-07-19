//! Stable client APIs for GCC predicate and named event-contract workflows.
//!
//! The verifier implementation remains in the separately versioned executable.
//! This library invokes it directly without a shell and validates its advertised
//! versioned CLI contract before exposing typed certificate operations.

use std::fmt;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Predicate CLI contract understood by this crate release.
pub const PREDICATE_API_VERSION: u32 = 1;
pub const EVENT_CONTRACT_API_VERSION: u32 = 1;
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(300);
pub const DEFAULT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
pub const DEFAULT_FILE_LIMIT_BYTES: u64 = 32 * 1024 * 1024;
#[cfg(all(unix, not(target_os = "macos")))]
pub const DEFAULT_MEMORY_LIMIT_BYTES: Option<u64> = Some(2 * 1024 * 1024 * 1024);
#[cfg(any(not(unix), target_os = "macos"))]
pub const DEFAULT_MEMORY_LIMIT_BYTES: Option<u64> = None;
const MAX_OUTPUT_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const MIN_MEMORY_LIMIT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MEMORY_LIMIT_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_FILE_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;

/// Runtime bounds applied independently to every executable invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionPolicy {
    timeout: Duration,
    output_limit_bytes: usize,
    memory_limit_bytes: Option<u64>,
    file_limit_bytes: u64,
}

impl ExecutionPolicy {
    pub fn new(timeout: Duration, output_limit_bytes: usize) -> Result<Self, PredicateApiError> {
        if timeout.is_zero() {
            return Err(PredicateApiError::InvalidPolicy(
                "timeout must be greater than zero".to_string(),
            ));
        }
        if !(1..=MAX_OUTPUT_LIMIT_BYTES).contains(&output_limit_bytes) {
            return Err(PredicateApiError::InvalidPolicy(format!(
                "output limit must be in 1..={MAX_OUTPUT_LIMIT_BYTES} bytes"
            )));
        }
        Ok(Self {
            timeout,
            output_limit_bytes,
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
            file_limit_bytes: DEFAULT_FILE_LIMIT_BYTES,
        })
    }

    pub fn timeout(self) -> Duration {
        self.timeout
    }

    pub fn output_limit_bytes(self) -> usize {
        self.output_limit_bytes
    }

    pub fn memory_limit_bytes(self) -> Option<u64> {
        self.memory_limit_bytes
    }

    pub fn file_limit_bytes(self) -> u64 {
        self.file_limit_bytes
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn with_memory_limit(mut self, bytes: u64) -> Result<Self, PredicateApiError> {
        if !(MIN_MEMORY_LIMIT_BYTES..=MAX_MEMORY_LIMIT_BYTES).contains(&bytes) {
            return Err(PredicateApiError::InvalidPolicy(format!(
                "memory limit must be in {MIN_MEMORY_LIMIT_BYTES}..={MAX_MEMORY_LIMIT_BYTES} bytes"
            )));
        }
        self.memory_limit_bytes = Some(bytes);
        Ok(self)
    }

    #[cfg(any(not(unix), target_os = "macos"))]
    pub fn with_memory_limit(self, bytes: u64) -> Result<Self, PredicateApiError> {
        if !(MIN_MEMORY_LIMIT_BYTES..=MAX_MEMORY_LIMIT_BYTES).contains(&bytes) {
            return Err(PredicateApiError::InvalidPolicy(format!(
                "memory limit must be in {MIN_MEMORY_LIMIT_BYTES}..={MAX_MEMORY_LIMIT_BYTES} bytes"
            )));
        }
        Err(PredicateApiError::InvalidPolicy(
            "address-space limits are unavailable on this platform".to_string(),
        ))
    }

    pub fn with_file_limit(mut self, bytes: u64) -> Result<Self, PredicateApiError> {
        if !(1..=MAX_FILE_LIMIT_BYTES).contains(&bytes) {
            return Err(PredicateApiError::InvalidPolicy(format!(
                "file limit must be in 1..={MAX_FILE_LIMIT_BYTES} bytes"
            )));
        }
        self.file_limit_bytes = bytes;
        Ok(self)
    }
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_EXECUTION_TIMEOUT,
            output_limit_bytes: DEFAULT_OUTPUT_LIMIT_BYTES,
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
            file_limit_bytes: DEFAULT_FILE_LIMIT_BYTES,
        }
    }
}

pub const INVOCATION_METRICS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    Discover,
    DiscoverEventContract,
    CertifyV1,
    CertifyV2,
    CertifyEventContractV3,
    VerifyV1,
    VerifyV2,
    VerifyEventContractV3,
    EventContractPortfolio,
    VerifyEventContractPortfolioReport,
}

impl OperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discover => "discover",
            Self::DiscoverEventContract => "discover_event_contract",
            Self::CertifyV1 => "certify_v1",
            Self::CertifyV2 => "certify_v2",
            Self::CertifyEventContractV3 => "certify_event_contract_v3",
            Self::VerifyV1 => "verify_v1",
            Self::VerifyV2 => "verify_v2",
            Self::VerifyEventContractV3 => "verify_event_contract_v3",
            Self::EventContractPortfolio => "event_contract_portfolio",
            Self::VerifyEventContractPortfolioReport => "verify_event_contract_portfolio_report",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureClass {
    Policy,
    Io,
    Timeout,
    OutputLimit,
    ExitStatus,
    Compatibility,
    Response,
}

impl FailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Policy => "policy",
            Self::Io => "io",
            Self::Timeout => "timeout",
            Self::OutputLimit => "output_limit",
            Self::ExitStatus => "exit_status",
            Self::Compatibility => "compatibility",
            Self::Response => "response",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationStatus {
    Success,
    Failed(FailureClass),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvocationMetrics {
    pub schema_version: u32,
    pub operation: OperationKind,
    pub duration: Duration,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub timeout: Duration,
    pub output_limit_bytes: usize,
    pub memory_limit_bytes: Option<u64>,
    pub file_limit_bytes: u64,
    pub process_group_containment: bool,
    pub exit_code: Option<i32>,
    pub status: InvocationStatus,
}

impl InvocationMetrics {
    pub const fn csv_header() -> &'static str {
        "schema_version,operation,duration_ns,stdout_bytes,stderr_bytes,timeout_ms,output_limit_bytes,exit_code,status,failure_class"
    }

    pub fn to_csv_row(&self) -> String {
        let (status, failure_class) = match self.status {
            InvocationStatus::Success => ("ok", "-"),
            InvocationStatus::Failed(class) => ("error", class.as_str()),
        };
        format!(
            "{},{},{},{},{},{},{},{},{},{}",
            self.schema_version,
            self.operation.as_str(),
            self.duration.as_nanos(),
            self.stdout_bytes,
            self.stderr_bytes,
            self.timeout.as_millis(),
            self.output_limit_bytes,
            self.exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "-".to_string()),
            status,
            failure_class,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observed<T> {
    pub value: T,
    pub metrics: InvocationMetrics,
}

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

    fn certify_operation(self) -> OperationKind {
        match self {
            Self::V1 => OperationKind::CertifyV1,
            Self::V2 => OperationKind::CertifyV2,
        }
    }

    fn verify_operation(self) -> OperationKind {
        match self {
            Self::V1 => OperationKind::VerifyV1,
            Self::V2 => OperationKind::VerifyV2,
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

/// Machine-discovered limits and formats for event-contract CLI v1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventContractCapabilities {
    pub cli_version: u32,
    pub certificate_version: u32,
    pub portfolio_version: u32,
    pub semantics: String,
    pub proof_format: String,
    pub min_relevant_inputs: usize,
    pub max_relevant_inputs: usize,
    pub max_latches: usize,
    pub max_horizon: usize,
    pub max_contract_bytes: u64,
    pub max_certificate_bytes: u64,
    pub max_proof_bytes: usize,
    pub max_total_proof_bytes: usize,
}

/// Event-contract v1 uses the same two logical outcomes as predicate checks.
pub type EventContractResult = PredicateResult;
pub type EventContractApiError = PredicateApiError;
pub type EventContractOperationError = PredicateOperationError;

/// A stable API error. Logical certificate results are not errors.
#[derive(Debug)]
pub enum PredicateApiError {
    Io(std::io::Error),
    InvalidPolicy(String),
    TimedOut {
        timeout: Duration,
    },
    OutputLimitExceeded {
        stream: &'static str,
        limit_bytes: usize,
    },
    CommandFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    IncompatibleContract(String),
    InvalidResponse(String),
}

impl PredicateApiError {
    pub fn failure_class(&self) -> FailureClass {
        match self {
            Self::InvalidPolicy(_) => FailureClass::Policy,
            Self::Io(_) => FailureClass::Io,
            Self::TimedOut { .. } => FailureClass::Timeout,
            Self::OutputLimitExceeded { .. } => FailureClass::OutputLimit,
            Self::CommandFailed { .. } => FailureClass::ExitStatus,
            Self::IncompatibleContract(_) => FailureClass::Compatibility,
            Self::InvalidResponse(_) => FailureClass::Response,
        }
    }
}

#[derive(Debug)]
pub struct PredicateOperationError {
    pub error: Box<PredicateApiError>,
    pub metrics: InvocationMetrics,
}

impl fmt::Display for PredicateOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for PredicateOperationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}

impl fmt::Display for PredicateApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "predicate tool I/O failed: {error}"),
            Self::InvalidPolicy(message) => {
                write!(formatter, "invalid execution policy: {message}")
            }
            Self::TimedOut { timeout } => {
                write!(formatter, "predicate tool exceeded {timeout:?} deadline")
            }
            Self::OutputLimitExceeded {
                stream,
                limit_bytes,
            } => write!(
                formatter,
                "predicate tool {stream} exceeded {limit_bytes}-byte limit"
            ),
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
    policy: ExecutionPolicy,
}

impl PredicateTool {
    /// Discover and validate the executable's predicate CLI contract.
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

    /// Discover with explicit runtime bounds for discovery and later jobs.
    pub fn discover_with_policy(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Self, PredicateApiError> {
        Self::discover_observed(executable, policy)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn discover_observed(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Observed<Self>, PredicateOperationError> {
        let executable = executable.into();
        let mut command = Command::new(&executable);
        command.arg("predicate-cli-version");
        let output = run_bounded(OperationKind::Discover, command, policy)?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_capabilities(&stdout).map_err(|error| {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            PredicateOperationError {
                error: Box::new(error),
                metrics: metrics.clone(),
            }
        })?;
        Ok(Observed {
            value: Self {
                executable,
                capabilities,
                policy,
            },
            metrics,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn capabilities(&self) -> &PredicateCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    /// Return a handle with different validated bounds and unchanged capabilities.
    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
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
        self.certify_observed(version, model, bad_output, transcript, certificate)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_observed(
        &self,
        version: CertificateVersion,
        model: &Path,
        bad_output: usize,
        transcript: &Path,
        certificate: &Path,
    ) -> Result<Observed<PredicateResult>, PredicateOperationError> {
        self.require_version_observed(version, version.certify_operation())?;
        let mut command = Command::new(&self.executable);
        command
            .arg(version.producer_command())
            .arg(model)
            .arg(bad_output.to_string())
            .arg(transcript)
            .arg(certificate);
        let output = run_bounded(version.certify_operation(), command, self.policy)?;
        parse_observed_result(output)
    }

    /// Verify a certificate against the selected model and return its result.
    pub fn verify(
        &self,
        version: CertificateVersion,
        model: &Path,
        certificate: &Path,
    ) -> Result<PredicateResult, PredicateApiError> {
        self.verify_observed(version, model, certificate)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        version: CertificateVersion,
        model: &Path,
        certificate: &Path,
    ) -> Result<Observed<PredicateResult>, PredicateOperationError> {
        self.require_version_observed(version, version.verify_operation())?;
        let mut command = Command::new(&self.executable);
        command
            .arg(version.verifier_command())
            .arg(model)
            .arg(certificate);
        let output = run_bounded(version.verify_operation(), command, self.policy)?;
        parse_observed_result(output)
    }

    fn require_version_observed(
        &self,
        version: CertificateVersion,
        operation: OperationKind,
    ) -> Result<(), PredicateOperationError> {
        if !self.capabilities.certificate_versions.contains(&version) {
            let error = PredicateApiError::IncompatibleContract(format!(
                "certificate version {version:?} is not advertised"
            ));
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    operation,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        Ok(())
    }
}

/// Typed, shell-free client for event-contract certificate v3 and portfolio v1.
#[derive(Clone, Debug)]
pub struct EventContractTool {
    executable: PathBuf,
    capabilities: EventContractCapabilities,
    policy: ExecutionPolicy,
}

impl EventContractTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, EventContractApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

    pub fn discover_with_policy(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Self, EventContractApiError> {
        Self::discover_observed(executable, policy)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn discover_observed(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Observed<Self>, EventContractOperationError> {
        let executable = executable.into();
        let mut command = Command::new(&executable);
        command.arg("event-contract-cli-version");
        let output = run_bounded(OperationKind::DiscoverEventContract, command, policy)?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_event_contract_capabilities(&stdout).map_err(|error| {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            PredicateOperationError {
                error: Box::new(error),
                metrics: metrics.clone(),
            }
        })?;
        Ok(Observed {
            value: Self {
                executable,
                capabilities,
                policy,
            },
            metrics,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn capabilities(&self) -> &EventContractCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn certify_v3(
        &self,
        model: &Path,
        bad_output: usize,
        contract: &Path,
        certificate: &Path,
    ) -> Result<EventContractResult, EventContractApiError> {
        self.certify_v3_observed(model, bad_output, contract, certificate)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_v3_observed(
        &self,
        model: &Path,
        bad_output: usize,
        contract: &Path,
        certificate: &Path,
    ) -> Result<Observed<EventContractResult>, EventContractOperationError> {
        let operation = OperationKind::CertifyEventContractV3;
        self.require_contract(operation)?;
        let mut command = Command::new(&self.executable);
        command
            .arg("certify-aiger-event-contract-v3")
            .arg(model)
            .arg(bad_output.to_string())
            .arg(contract)
            .arg(certificate);
        parse_observed_result(run_bounded(operation, command, self.policy)?)
    }

    pub fn verify_v3(
        &self,
        model: &Path,
        contract: &Path,
        certificate: &Path,
    ) -> Result<EventContractResult, EventContractApiError> {
        self.verify_v3_observed(model, contract, certificate)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_v3_observed(
        &self,
        model: &Path,
        contract: &Path,
        certificate: &Path,
    ) -> Result<Observed<EventContractResult>, EventContractOperationError> {
        let operation = OperationKind::VerifyEventContractV3;
        self.require_contract(operation)?;
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-aiger-event-contract-certificate-v3")
            .arg(model)
            .arg(contract)
            .arg(certificate);
        parse_observed_result(run_bounded(operation, command, self.policy)?)
    }

    pub fn verify_portfolio(
        &self,
        model: &Path,
        bad_output: usize,
        contract: &Path,
        report: &Path,
        certificate: &Path,
    ) -> Result<EventContractResult, EventContractApiError> {
        self.verify_portfolio_observed(model, bad_output, contract, report, certificate)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_portfolio_observed(
        &self,
        model: &Path,
        bad_output: usize,
        contract: &Path,
        report: &Path,
        certificate: &Path,
    ) -> Result<Observed<EventContractResult>, EventContractOperationError> {
        let operation = OperationKind::EventContractPortfolio;
        self.require_contract(operation)?;
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-aiger-event-contract-portfolio")
            .arg(model)
            .arg(bad_output.to_string())
            .arg(contract)
            .arg(report)
            .arg(certificate);
        parse_observed_result(run_bounded(operation, command, self.policy)?)
    }

    pub fn verify_portfolio_report(
        &self,
        model: &Path,
        bad_output: usize,
        contract: &Path,
        report: &Path,
        certificate: &Path,
    ) -> Result<EventContractResult, EventContractApiError> {
        self.verify_portfolio_report_observed(model, bad_output, contract, report, certificate)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_portfolio_report_observed(
        &self,
        model: &Path,
        bad_output: usize,
        contract: &Path,
        report: &Path,
        certificate: &Path,
    ) -> Result<Observed<EventContractResult>, EventContractOperationError> {
        let operation = OperationKind::VerifyEventContractPortfolioReport;
        self.require_contract(operation)?;
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-aiger-event-contract-portfolio-report")
            .arg(model)
            .arg(bad_output.to_string())
            .arg(contract)
            .arg(report)
            .arg(certificate);
        parse_observed_result(run_bounded(operation, command, self.policy)?)
    }

    fn require_contract(
        &self,
        operation: OperationKind,
    ) -> Result<(), EventContractOperationError> {
        if self.capabilities.certificate_version != 3 || self.capabilities.portfolio_version != 1 {
            let error = PredicateApiError::IncompatibleContract(
                "event-contract certificate v3 and portfolio v1 are required".to_string(),
            );
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    operation,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ManagedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    metrics: InvocationMetrics,
}

fn empty_metrics(
    operation: OperationKind,
    policy: ExecutionPolicy,
    status: InvocationStatus,
) -> InvocationMetrics {
    InvocationMetrics {
        schema_version: INVOCATION_METRICS_SCHEMA_VERSION,
        operation,
        duration: Duration::ZERO,
        stdout_bytes: 0,
        stderr_bytes: 0,
        timeout: policy.timeout,
        output_limit_bytes: policy.output_limit_bytes,
        memory_limit_bytes: policy.memory_limit_bytes,
        file_limit_bytes: policy.file_limit_bytes,
        process_group_containment: cfg!(unix),
        exit_code: None,
        status,
    }
}

fn operation_failure(
    operation: OperationKind,
    policy: ExecutionPolicy,
    started: Instant,
    stdout_bytes: usize,
    stderr_bytes: usize,
    exit_code: Option<i32>,
    error: PredicateApiError,
) -> PredicateOperationError {
    PredicateOperationError {
        metrics: InvocationMetrics {
            schema_version: INVOCATION_METRICS_SCHEMA_VERSION,
            operation,
            duration: started.elapsed(),
            stdout_bytes,
            stderr_bytes,
            timeout: policy.timeout,
            output_limit_bytes: policy.output_limit_bytes,
            memory_limit_bytes: policy.memory_limit_bytes,
            file_limit_bytes: policy.file_limit_bytes,
            process_group_containment: cfg!(unix),
            exit_code,
            status: InvocationStatus::Failed(error.failure_class()),
        },
        error: Box::new(error),
    }
}

fn read_limited(
    reader: impl Read + Send + 'static,
    limit: usize,
) -> thread::JoinHandle<Result<Vec<u8>, std::io::Error>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

fn join_output(
    handle: thread::JoinHandle<Result<Vec<u8>, std::io::Error>>,
) -> Result<Vec<u8>, PredicateApiError> {
    handle
        .join()
        .map_err(|_| PredicateApiError::InvalidResponse("output reader stopped".to_string()))?
        .map_err(PredicateApiError::Io)
}

fn configure_process(
    command: &mut Command,
    policy: ExecutionPolicy,
) -> Result<(), PredicateApiError> {
    #[cfg(unix)]
    {
        let file_limit = libc::rlim_t::try_from(policy.file_limit_bytes).map_err(|_| {
            PredicateApiError::InvalidPolicy(
                "file limit is not representable on this platform".to_string(),
            )
        })?;
        #[cfg(not(target_os = "macos"))]
        let memory_limit = policy
            .memory_limit_bytes
            .map(libc::rlim_t::try_from)
            .transpose()
            .map_err(|_| {
                PredicateApiError::InvalidPolicy(
                    "memory limit is not representable on this platform".to_string(),
                )
            })?;
        // SAFETY: this closure runs after fork and before exec and only calls
        // async-signal-safe libc functions with values prepared above.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                #[cfg(not(target_os = "macos"))]
                if let Some(memory_limit) = memory_limit {
                    let memory = libc::rlimit {
                        rlim_cur: memory_limit,
                        rlim_max: memory_limit,
                    };
                    if libc::setrlimit(libc::RLIMIT_AS, &memory) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                let file = libc::rlimit {
                    rlim_cur: file_limit,
                    rlim_max: file_limit,
                };
                if libc::setrlimit(libc::RLIMIT_FSIZE, &file) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(not(unix))]
    let _ = (command, policy);
    Ok(())
}

fn stop_process(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        if let Ok(group) = i32::try_from(child.id()) {
            // SAFETY: the child creates a new session before exec; negating its
            // PID addresses that process group. ESRCH simply means it exited.
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
    }
    #[cfg(not(unix))]
    let _ = child.kill();
    let _ = child.wait();
}

fn run_bounded(
    operation: OperationKind,
    mut command: Command,
    policy: ExecutionPolicy,
) -> Result<ManagedOutput, PredicateOperationError> {
    let started = Instant::now();
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    configure_process(&mut command, policy)
        .map_err(|error| operation_failure(operation, policy, started, 0, 0, None, error))?;
    let mut child = command
        .spawn()
        .map_err(|error| operation_failure(operation, policy, started, 0, 0, None, error.into()))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        operation_failure(
            operation,
            policy,
            started,
            0,
            0,
            None,
            PredicateApiError::InvalidResponse("stdout pipe is missing".to_string()),
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        operation_failure(
            operation,
            policy,
            started,
            0,
            0,
            None,
            PredicateApiError::InvalidResponse("stderr pipe is missing".to_string()),
        )
    })?;
    let stdout_reader = read_limited(stdout, policy.output_limit_bytes);
    let stderr_reader = read_limited(stderr, policy.output_limit_bytes);
    let deadline = Instant::now() + policy.timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            operation_failure(operation, policy, started, 0, 0, None, error.into())
        })? {
            break status;
        }
        if Instant::now() >= deadline {
            stop_process(&mut child);
            let stdout_bytes = join_output(stdout_reader)
                .map(|bytes| bytes.len())
                .unwrap_or(0);
            let stderr_bytes = join_output(stderr_reader)
                .map(|bytes| bytes.len())
                .unwrap_or(0);
            return Err(operation_failure(
                operation,
                policy,
                started,
                stdout_bytes,
                stderr_bytes,
                None,
                PredicateApiError::TimedOut {
                    timeout: policy.timeout,
                },
            ));
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = join_output(stdout_reader).map_err(|error| {
        operation_failure(operation, policy, started, 0, 0, status.code(), error)
    })?;
    let stderr = join_output(stderr_reader).map_err(|error| {
        operation_failure(
            operation,
            policy,
            started,
            stdout.len(),
            0,
            status.code(),
            error,
        )
    })?;
    if stdout.len() > policy.output_limit_bytes {
        return Err(operation_failure(
            operation,
            policy,
            started,
            stdout.len(),
            stderr.len(),
            status.code(),
            PredicateApiError::OutputLimitExceeded {
                stream: "stdout",
                limit_bytes: policy.output_limit_bytes,
            },
        ));
    }
    if stderr.len() > policy.output_limit_bytes {
        return Err(operation_failure(
            operation,
            policy,
            started,
            stdout.len(),
            stderr.len(),
            status.code(),
            PredicateApiError::OutputLimitExceeded {
                stream: "stderr",
                limit_bytes: policy.output_limit_bytes,
            },
        ));
    }
    let metrics = InvocationMetrics {
        schema_version: INVOCATION_METRICS_SCHEMA_VERSION,
        operation,
        duration: started.elapsed(),
        stdout_bytes: stdout.len(),
        stderr_bytes: stderr.len(),
        timeout: policy.timeout,
        output_limit_bytes: policy.output_limit_bytes,
        memory_limit_bytes: policy.memory_limit_bytes,
        file_limit_bytes: policy.file_limit_bytes,
        process_group_containment: cfg!(unix),
        exit_code: status.code(),
        status: InvocationStatus::Success,
    };
    Ok(ManagedOutput {
        status,
        stdout,
        stderr,
        metrics,
    })
}

fn successful_stdout(
    mut output: ManagedOutput,
) -> Result<(String, InvocationMetrics), PredicateOperationError> {
    if !output.status.success() {
        let error = PredicateApiError::CommandFailed {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        output.metrics.status = InvocationStatus::Failed(error.failure_class());
        return Err(PredicateOperationError {
            error: Box::new(error),
            metrics: output.metrics,
        });
    }
    let stdout = String::from_utf8(output.stdout).map_err(|_| {
        let error = PredicateApiError::InvalidResponse("stdout is not UTF-8".to_string());
        output.metrics.status = InvocationStatus::Failed(error.failure_class());
        PredicateOperationError {
            error: Box::new(error),
            metrics: output.metrics.clone(),
        }
    })?;
    Ok((stdout, output.metrics))
}

fn parse_observed_result(
    output: ManagedOutput,
) -> Result<Observed<PredicateResult>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let value = parse_result(&stdout).map_err(|error| {
        metrics.status = InvocationStatus::Failed(error.failure_class());
        PredicateOperationError {
            error: Box::new(error),
            metrics: metrics.clone(),
        }
    })?;
    Ok(Observed { value, metrics })
}

fn parse_result(stdout: &str) -> Result<PredicateResult, PredicateApiError> {
    let results = stdout
        .split_whitespace()
        .filter_map(|field| field.strip_prefix("result="))
        .collect::<Vec<_>>();
    let Some(&result) = results.first() else {
        return Err(PredicateApiError::InvalidResponse(
            "result field is missing".to_string(),
        ));
    };
    if results.iter().any(|candidate| *candidate != result) {
        return Err(PredicateApiError::InvalidResponse(
            "subprocess result fields disagree".to_string(),
        ));
    }
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

fn parse_event_contract_capabilities(
    line: &str,
) -> Result<EventContractCapabilities, EventContractApiError> {
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    if fields.len() != 13 || line.contains('\r') || !line.ends_with('\n') {
        return Err(PredicateApiError::InvalidResponse(
            "event-contract capability line shape is not canonical".to_string(),
        ));
    }
    let value = |index: usize, key: &str| -> Result<&str, PredicateApiError> {
        fields[index]
            .strip_prefix(&format!("{key}="))
            .ok_or_else(|| {
                PredicateApiError::InvalidResponse(format!(
                    "expected event-contract capability `{key}`"
                ))
            })
    };
    let number = |index: usize, key: &str| -> Result<u64, PredicateApiError> {
        let text = value(index, key)?;
        if text.is_empty()
            || !text.bytes().all(|byte| byte.is_ascii_digit())
            || (text.len() > 1 && text.starts_with('0'))
        {
            return Err(PredicateApiError::InvalidResponse(format!(
                "event-contract capability `{key}` is not canonical decimal"
            )));
        }
        text.parse::<u64>().map_err(|_| {
            PredicateApiError::InvalidResponse(format!(
                "event-contract capability `{key}` is invalid"
            ))
        })
    };
    let as_usize = |index: usize, key: &str| -> Result<usize, PredicateApiError> {
        usize::try_from(number(index, key)?).map_err(|_| {
            PredicateApiError::InvalidResponse(format!(
                "event-contract capability `{key}` exceeds usize"
            ))
        })
    };
    let cli_version = u32::try_from(number(0, "event_contract_cli_version")?).map_err(|_| {
        PredicateApiError::InvalidResponse("event-contract CLI version exceeds u32".to_string())
    })?;
    if cli_version != EVENT_CONTRACT_API_VERSION {
        return Err(PredicateApiError::IncompatibleContract(format!(
            "expected event-contract CLI v{EVENT_CONTRACT_API_VERSION}, found v{cli_version}"
        )));
    }
    let certificate_version = u32::try_from(number(1, "certificate_version")?).map_err(|_| {
        PredicateApiError::InvalidResponse(
            "event-contract certificate version exceeds u32".to_string(),
        )
    })?;
    let portfolio_version = u32::try_from(number(2, "portfolio_version")?).map_err(|_| {
        PredicateApiError::InvalidResponse(
            "event-contract portfolio version exceeds u32".to_string(),
        )
    })?;
    if certificate_version != 3 || portfolio_version != 1 {
        return Err(PredicateApiError::IncompatibleContract(
            "event-contract certificate v3 and portfolio v1 are required".to_string(),
        ));
    }
    let semantics = value(3, "semantics")?.to_string();
    if semantics != "bounded-named-cnf-terminal-bad-avoidance" {
        return Err(PredicateApiError::IncompatibleContract(format!(
            "unsupported event-contract semantics `{semantics}`"
        )));
    }
    let proof_format = value(4, "proof_format")?.to_string();
    if proof_format != "varisat-native-0.2.2" {
        return Err(PredicateApiError::IncompatibleContract(format!(
            "unsupported proof format `{proof_format}`"
        )));
    }
    Ok(EventContractCapabilities {
        cli_version,
        certificate_version,
        portfolio_version,
        semantics,
        proof_format,
        min_relevant_inputs: as_usize(5, "min_relevant_inputs")?,
        max_relevant_inputs: as_usize(6, "max_relevant_inputs")?,
        max_latches: as_usize(7, "max_latches")?,
        max_horizon: as_usize(8, "max_horizon")?,
        max_contract_bytes: number(9, "max_contract_bytes")?,
        max_certificate_bytes: number(10, "max_certificate_bytes")?,
        max_proof_bytes: as_usize(11, "max_proof_bytes")?,
        max_total_proof_bytes: as_usize(12, "max_total_proof_bytes")?,
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

    #[test]
    fn event_contract_capability_parser_accepts_v1_and_rejects_drift() {
        let canonical = "event_contract_cli_version=1 certificate_version=3 portfolio_version=1 semantics=bounded-named-cnf-terminal-bad-avoidance proof_format=varisat-native-0.2.2 min_relevant_inputs=9 max_relevant_inputs=16 max_latches=4 max_horizon=64 max_contract_bytes=1048576 max_certificate_bytes=33554432 max_proof_bytes=1048576 max_total_proof_bytes=8388608\n";
        let parsed = parse_event_contract_capabilities(canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.certificate_version, 3);
        assert!(matches!(
            parse_event_contract_capabilities(&canonical.replacen(
                "event_contract_cli_version=1",
                "event_contract_cli_version=2",
                1
            )),
            Err(PredicateApiError::IncompatibleContract(_))
        ));
        assert!(
            parse_event_contract_capabilities(&canonical.replace(" max_latches=4", "")).is_err()
        );
    }

    #[test]
    fn result_parser_rejects_missing_ambiguous_and_unknown_answers() {
        assert_eq!(
            parse_result("status=VERIFIED result=avoidable\n").unwrap(),
            PredicateResult::Avoidable
        );
        assert!(parse_result("status=VERIFIED\n").is_err());
        assert_eq!(
            parse_result("result=avoidable result=avoidable\n").unwrap(),
            PredicateResult::Avoidable
        );
        assert!(parse_result("result=avoidable result=unavoidable\n").is_err());
        assert!(parse_result("result=maybe\n").is_err());
    }

    #[test]
    fn execution_policy_rejects_zero_and_excessive_bounds() {
        assert!(ExecutionPolicy::new(Duration::ZERO, 1).is_err());
        assert!(ExecutionPolicy::new(Duration::from_secs(1), 0).is_err());
        assert!(ExecutionPolicy::new(Duration::from_secs(1), MAX_OUTPUT_LIMIT_BYTES + 1).is_err());
        assert!(
            ExecutionPolicy::new(Duration::from_secs(1), 1024)
                .unwrap()
                .with_file_limit(0)
                .is_err()
        );
        assert_eq!(
            ExecutionPolicy::new(Duration::from_secs(2), 4096)
                .unwrap()
                .output_limit_bytes(),
            4096
        );
    }

    #[test]
    fn invocation_metrics_csv_schema_is_stable() {
        let metrics = InvocationMetrics {
            schema_version: 1,
            operation: OperationKind::VerifyV2,
            duration: Duration::from_nanos(123),
            stdout_bytes: 45,
            stderr_bytes: 6,
            timeout: Duration::from_millis(700),
            output_limit_bytes: 8192,
            memory_limit_bytes: Some(1024 * 1024 * 1024),
            file_limit_bytes: 32 * 1024 * 1024,
            process_group_containment: true,
            exit_code: Some(2),
            status: InvocationStatus::Failed(FailureClass::ExitStatus),
        };
        assert_eq!(
            InvocationMetrics::csv_header(),
            "schema_version,operation,duration_ns,stdout_bytes,stderr_bytes,timeout_ms,output_limit_bytes,exit_code,status,failure_class"
        );
        assert_eq!(
            metrics.to_csv_row(),
            "1,verify_v2,123,45,6,700,8192,2,error,exit_status"
        );
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_reports_deadline_and_output_classes() {
        let mut delayed = Command::new("sh");
        delayed.arg("-c").arg("sleep 5 & wait");
        let started = Instant::now();
        let failure = run_bounded(
            OperationKind::VerifyV2,
            delayed,
            ExecutionPolicy::new(Duration::from_millis(10), 1024).unwrap(),
        )
        .unwrap_err();
        assert!(matches!(*failure.error, PredicateApiError::TimedOut { .. }));
        assert_eq!(
            failure.metrics.status,
            InvocationStatus::Failed(FailureClass::Timeout)
        );
        assert!(started.elapsed() < Duration::from_secs(1));

        let mut verbose = Command::new("printf");
        verbose.arg("0123456789");
        let failure = run_bounded(
            OperationKind::CertifyV2,
            verbose,
            ExecutionPolicy::new(Duration::from_secs(1), 4).unwrap(),
        )
        .unwrap_err();
        assert!(matches!(
            *failure.error,
            PredicateApiError::OutputLimitExceeded {
                stream: "stdout",
                limit_bytes: 4,
            }
        ));
        assert_eq!(failure.metrics.operation, OperationKind::CertifyV2);
        assert_eq!(failure.metrics.stdout_bytes, 5);
        assert_eq!(
            failure.metrics.status,
            InvocationStatus::Failed(FailureClass::OutputLimit)
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn bounded_runner_applies_address_space_ceiling() {
        let bytes = 128 * 1024 * 1024;
        let policy = ExecutionPolicy::new(Duration::from_secs(1), 1024)
            .unwrap()
            .with_memory_limit(bytes)
            .unwrap();
        let mut command = Command::new("sh");
        command.arg("-c").arg("ulimit -v");
        let output = run_bounded(OperationKind::Discover, command, policy).unwrap();
        let reported_kib = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse::<u64>()
            .unwrap();
        assert!(reported_kib > 0);
        assert!(reported_kib <= bytes / 1024);
        assert_eq!(output.metrics.memory_limit_bytes, Some(bytes));
    }
}
