//! Stable client APIs for GCC predicate and named event-contract workflows.
//!
//! The verifier implementation remains in the separately versioned executable.
//! This library invokes it directly without a shell and validates its advertised
//! versioned CLI contract before exposing typed certificate operations.

pub mod aiger_obligation;
pub mod btor2;
pub mod btor2_bitblast;
pub mod btor2_bounded;
pub mod btor2_braking;
pub mod btor2_component;
pub mod btor2_family;
pub mod btor2_family_orbit;
pub mod btor2_family_proof;
pub mod btor2_invariant_chain;
pub mod btor2_motion;
pub mod btor2_phase;
pub mod btor2_predicate_set;
pub mod btor2_region;
pub mod btor2_region_equivalence;
pub mod btor2_region_extract;
pub mod btor2_region_property;
pub mod btor2_search;
pub mod composed_witness;
pub mod controller_mtbdd;
pub mod controller_mtbdd_proof;
pub mod controller_plant;
pub mod controller_plant_aiger;
pub mod controller_plant_artifact;
pub mod controller_transducer;
pub mod dense_relation;
#[cfg(feature = "research-qatq-transport")]
pub mod qatq_transport;
pub mod revision_batch;
pub mod revision_impact;
pub mod revision_local;
pub mod source_model_attestation;
pub mod unsat_proof;

use std::collections::BTreeMap;
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
    DiscoverControllerMtbdd,
    CertifyControllerMtbddPlantBatch,
    VerifyControllerMtbddPlantBatch,
    DiscoverControllerProofMtbdd,
    CertifyControllerProofMtbddPlantBatch,
    VerifyControllerProofMtbddPlantBatch,
    DiscoverControllerPlantPortfolio,
    CertifyControllerPlantPortfolio,
    VerifyControllerPlantPortfolio,
    DiscoverControllerPlantResource,
    VerifyControllerPlantPortfolioResources,
    DiscoverControllerProofMtbddResource,
    VerifyControllerProofMtbddPlantResources,
    DiscoverControllerProofMtbddPortfolio,
    VerifyControllerProofMtbddPortfolioResources,
    VerifyControllerProofMtbddPortfolioResourcesAttested,
    DiscoverControllerSplitEvidence,
    CertifyControllerProofEvidence,
    CertifyBoundPlantResults,
    VerifyBoundPlantResultSet,
    DiscoverControllerSplitResource,
    VerifyBoundPlantResultSetResources,
    DiscoverControllerSplitObservability,
    VerifyBoundPlantResultSetResourcesObserved,
    DiscoverControllerSplitAllocationObservability,
    VerifyBoundPlantResultSetResourcesAllocationObserved,
    DiscoverControllerSplitCacheObservability,
    VerifyBoundPlantResultSetResourcesCacheObserved,
    DiscoverRevisionImpact,
    CertifyRevisionImpact,
    VerifyRevisionImpact,
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
            Self::DiscoverControllerMtbdd => "discover_controller_mtbdd",
            Self::CertifyControllerMtbddPlantBatch => "certify_controller_mtbdd_plant_batch",
            Self::VerifyControllerMtbddPlantBatch => "verify_controller_mtbdd_plant_batch",
            Self::DiscoverControllerProofMtbdd => "discover_controller_proof_mtbdd",
            Self::CertifyControllerProofMtbddPlantBatch => {
                "certify_controller_proof_mtbdd_plant_batch"
            }
            Self::VerifyControllerProofMtbddPlantBatch => {
                "verify_controller_proof_mtbdd_plant_batch"
            }
            Self::DiscoverControllerPlantPortfolio => "discover_controller_plant_portfolio",
            Self::CertifyControllerPlantPortfolio => "certify_controller_plant_portfolio",
            Self::VerifyControllerPlantPortfolio => "verify_controller_plant_portfolio",
            Self::DiscoverControllerPlantResource => "discover_controller_plant_resource",
            Self::VerifyControllerPlantPortfolioResources => {
                "verify_controller_plant_portfolio_resources"
            }
            Self::DiscoverControllerProofMtbddResource => {
                "discover_controller_proof_mtbdd_resource"
            }
            Self::VerifyControllerProofMtbddPlantResources => {
                "verify_controller_proof_mtbdd_plant_resources"
            }
            Self::DiscoverControllerProofMtbddPortfolio => {
                "discover_controller_proof_mtbdd_portfolio"
            }
            Self::VerifyControllerProofMtbddPortfolioResources => {
                "verify_controller_proof_mtbdd_portfolio_resources"
            }
            Self::VerifyControllerProofMtbddPortfolioResourcesAttested => {
                "verify_controller_proof_mtbdd_portfolio_resources_attested"
            }
            Self::DiscoverControllerSplitEvidence => "discover_controller_split_evidence",
            Self::CertifyControllerProofEvidence => "certify_controller_proof_evidence",
            Self::CertifyBoundPlantResults => "certify_bound_plant_results",
            Self::VerifyBoundPlantResultSet => "verify_bound_plant_result_set",
            Self::DiscoverControllerSplitResource => "discover_controller_split_resource",
            Self::VerifyBoundPlantResultSetResources => "verify_bound_plant_result_set_resources",
            Self::DiscoverControllerSplitObservability => "discover_controller_split_observability",
            Self::VerifyBoundPlantResultSetResourcesObserved => {
                "verify_bound_plant_result_set_resources_observed"
            }
            Self::DiscoverControllerSplitAllocationObservability => {
                "discover_controller_split_allocation_observability"
            }
            Self::VerifyBoundPlantResultSetResourcesAllocationObserved => {
                "verify_bound_plant_result_set_resources_allocation_observed"
            }
            Self::DiscoverControllerSplitCacheObservability => {
                "discover_controller_split_cache_observability"
            }
            Self::VerifyBoundPlantResultSetResourcesCacheObserved => {
                "verify_bound_plant_result_set_resources_cache_observed"
            }
            Self::DiscoverRevisionImpact => "discover_revision_impact",
            Self::CertifyRevisionImpact => "certify_revision_impact",
            Self::VerifyRevisionImpact => "verify_revision_impact",
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
    ResourceRefusal,
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
            Self::ResourceRefusal => "resource_refusal",
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

pub const INVOCATION_METRICS_AGGREGATE_SCHEMA_VERSION: u32 = 1;
pub const MAX_AGGREGATED_INVOCATIONS: usize = 1_000_000;

/// Bounded, canonical aggregation of process-client observations.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct InvocationMetricsAggregate {
    pub schema_version: u32,
    pub jobs: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_duration_ns: u128,
    pub maximum_duration_ns: u128,
    pub total_stdout_bytes: u128,
    pub total_stderr_bytes: u128,
    pub process_group_contained_jobs: u64,
    pub memory_limited_jobs: u64,
    pub operation_counts: BTreeMap<&'static str, u64>,
    pub failure_counts: BTreeMap<&'static str, u64>,
}

impl InvocationMetricsAggregate {
    pub const fn csv_header() -> &'static str {
        "schema_version,jobs,successes,failures,total_duration_ns,maximum_duration_ns,total_stdout_bytes,total_stderr_bytes,process_group_contained_jobs,memory_limited_jobs,operation_counts,failure_counts"
    }

    pub fn to_csv_row(&self) -> String {
        let counts = |values: &BTreeMap<&'static str, u64>| {
            if values.is_empty() {
                "none".to_string()
            } else {
                values
                    .iter()
                    .map(|(name, count)| format!("{name}={count}"))
                    .collect::<Vec<_>>()
                    .join(";")
            }
        };
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            self.schema_version,
            self.jobs,
            self.successes,
            self.failures,
            self.total_duration_ns,
            self.maximum_duration_ns,
            self.total_stdout_bytes,
            self.total_stderr_bytes,
            self.process_group_contained_jobs,
            self.memory_limited_jobs,
            counts(&self.operation_counts),
            counts(&self.failure_counts),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MetricsAggregationError {
    Empty,
    TooManyJobs,
    UnsupportedSchema(u32),
    Overflow,
}

impl fmt::Display for MetricsAggregationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "invocation metrics set is empty"),
            Self::TooManyJobs => write!(
                formatter,
                "invocation metrics set exceeds {MAX_AGGREGATED_INVOCATIONS} jobs"
            ),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported invocation metrics schema {version}")
            }
            Self::Overflow => write!(formatter, "invocation metrics aggregate overflows"),
        }
    }
}

impl std::error::Error for MetricsAggregationError {}

/// Aggregate process observations without dropping failed jobs.
pub fn aggregate_invocation_metrics<'a>(
    metrics: impl IntoIterator<Item = &'a InvocationMetrics>,
) -> Result<InvocationMetricsAggregate, MetricsAggregationError> {
    let mut aggregate = InvocationMetricsAggregate {
        schema_version: INVOCATION_METRICS_AGGREGATE_SCHEMA_VERSION,
        jobs: 0,
        successes: 0,
        failures: 0,
        total_duration_ns: 0,
        maximum_duration_ns: 0,
        total_stdout_bytes: 0,
        total_stderr_bytes: 0,
        process_group_contained_jobs: 0,
        memory_limited_jobs: 0,
        operation_counts: BTreeMap::new(),
        failure_counts: BTreeMap::new(),
    };
    for metric in metrics {
        if metric.schema_version != INVOCATION_METRICS_SCHEMA_VERSION {
            return Err(MetricsAggregationError::UnsupportedSchema(
                metric.schema_version,
            ));
        }
        if aggregate.jobs as usize == MAX_AGGREGATED_INVOCATIONS {
            return Err(MetricsAggregationError::TooManyJobs);
        }
        aggregate.jobs = aggregate
            .jobs
            .checked_add(1)
            .ok_or(MetricsAggregationError::Overflow)?;
        let duration = metric.duration.as_nanos();
        aggregate.total_duration_ns = aggregate
            .total_duration_ns
            .checked_add(duration)
            .ok_or(MetricsAggregationError::Overflow)?;
        aggregate.maximum_duration_ns = aggregate.maximum_duration_ns.max(duration);
        aggregate.total_stdout_bytes = aggregate
            .total_stdout_bytes
            .checked_add(metric.stdout_bytes as u128)
            .ok_or(MetricsAggregationError::Overflow)?;
        aggregate.total_stderr_bytes = aggregate
            .total_stderr_bytes
            .checked_add(metric.stderr_bytes as u128)
            .ok_or(MetricsAggregationError::Overflow)?;
        if metric.process_group_containment {
            aggregate.process_group_contained_jobs = aggregate
                .process_group_contained_jobs
                .checked_add(1)
                .ok_or(MetricsAggregationError::Overflow)?;
        }
        if metric.memory_limit_bytes.is_some() {
            aggregate.memory_limited_jobs = aggregate
                .memory_limited_jobs
                .checked_add(1)
                .ok_or(MetricsAggregationError::Overflow)?;
        }
        let operation_count = aggregate
            .operation_counts
            .entry(metric.operation.as_str())
            .or_default();
        *operation_count = operation_count
            .checked_add(1)
            .ok_or(MetricsAggregationError::Overflow)?;
        match metric.status {
            InvocationStatus::Success => {
                aggregate.successes = aggregate
                    .successes
                    .checked_add(1)
                    .ok_or(MetricsAggregationError::Overflow)?;
            }
            InvocationStatus::Failed(class) => {
                aggregate.failures = aggregate
                    .failures
                    .checked_add(1)
                    .ok_or(MetricsAggregationError::Overflow)?;
                let failure_count = aggregate.failure_counts.entry(class.as_str()).or_default();
                *failure_count = failure_count
                    .checked_add(1)
                    .ok_or(MetricsAggregationError::Overflow)?;
            }
        }
    }
    if aggregate.jobs == 0 {
        return Err(MetricsAggregationError::Empty);
    }
    if aggregate.successes.checked_add(aggregate.failures) != Some(aggregate.jobs) {
        return Err(MetricsAggregationError::Overflow);
    }
    Ok(aggregate)
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

/// Machine-discovered limits for controller MTBDD plant CLI v1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddCapabilities {
    pub cli_version: u32,
    pub mtbdd_version: u32,
    pub plant_artifact_version: u32,
    pub manifest_version: u32,
    pub max_manifest_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_members: usize,
    pub max_state_bits: usize,
    pub max_inputs: usize,
    pub max_outputs: usize,
    pub max_nodes: usize,
    pub max_terminals: usize,
    pub max_assignments: usize,
    pub max_horizon: usize,
}

/// Machine-discovered limits and semantics for revision-impact CLI v2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactCapabilities {
    pub cli_version: u32,
    pub impact_version: u32,
    pub query_manifest_version: u32,
    pub max_query_manifest_bytes: usize,
    pub max_input_bytes: usize,
    pub max_evidence_bytes: usize,
    pub max_bundle_bytes: usize,
    pub max_atoms: usize,
    pub max_combinations: usize,
    pub max_queries: usize,
}

/// Paths and projected output nodes bound into one revision-impact job.
#[derive(Clone, Debug)]
pub struct RevisionImpactFiles<'a> {
    pub left_old: &'a Path,
    pub left_new: &'a Path,
    pub left_outputs: &'a [u64],
    pub right_old: &'a Path,
    pub right_new: &'a Path,
    pub right_outputs: &'a [u64],
    pub interface_old: &'a Path,
    pub interface_new: &'a Path,
    pub queries: &'a Path,
}

/// Canonical summary returned by revision-impact production or verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactProcessSummary {
    pub impact_version: u32,
    pub atoms: usize,
    pub queries: usize,
    pub combinations: usize,
    pub reusable_observations: usize,
    pub invalidated_observations: usize,
    pub minimal_invalidating_sets: usize,
    pub minimal_semantic_change_sets: usize,
    pub evidence_members: usize,
    pub certificate_bytes: usize,
    pub parsed_evidence_bytes: usize,
    pub semantic_replays: usize,
    pub component_validations: usize,
    pub composed_pair_checks: usize,
    pub final_transition_checks: usize,
    pub result_comparisons: usize,
    pub elapsed_micros: usize,
    pub transitions: Vec<RevisionImpactQueryTransition>,
    pub semantic_change_sets: Vec<RevisionImpactSemanticChangeSet>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactQueryTransition {
    pub index: usize,
    pub horizon: u32,
    pub bad_side: revision_local::ComponentSide,
    pub bad_output: u64,
    pub old_result: revision_local::BoundedResult,
    pub new_result: revision_local::BoundedResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactSemanticChangeSet {
    pub query_index: usize,
    pub changed_mask: u16,
    pub baseline_result: revision_local::BoundedResult,
    pub changed_result: revision_local::BoundedResult,
}

/// Machine-discovered limits for proof-carrying controller MTBDD CLI v1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProofMtbddCapabilities {
    pub cli_version: u32,
    pub mtbdd_version: u32,
    pub equivalence_proof_version: u32,
    pub plant_artifact_version: u32,
    pub manifest_version: u32,
    pub max_manifest_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_equivalence_artifact_bytes: usize,
    pub max_unsat_proof_bytes: usize,
    pub max_members: usize,
    pub max_state_bits: usize,
    pub max_inputs: usize,
    pub max_outputs: usize,
    pub max_nodes: usize,
    pub max_terminals: usize,
    pub max_horizon: usize,
}

impl ControllerProofMtbddCapabilities {
    fn common(&self) -> ControllerMtbddCapabilities {
        ControllerMtbddCapabilities {
            cli_version: self.cli_version,
            mtbdd_version: self.mtbdd_version,
            plant_artifact_version: self.plant_artifact_version,
            manifest_version: self.manifest_version,
            max_manifest_bytes: self.max_manifest_bytes,
            max_artifact_bytes: self.max_artifact_bytes,
            max_members: self.max_members,
            max_state_bits: self.max_state_bits,
            max_inputs: self.max_inputs,
            max_outputs: self.max_outputs,
            max_nodes: self.max_nodes,
            max_terminals: self.max_terminals,
            max_assignments: 0,
            max_horizon: self.max_horizon,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitEvidenceCapabilities {
    pub cli_version: u32,
    pub controller_artifact_version: u32,
    pub plant_artifact_version: u32,
    pub manifest_version: u32,
    pub max_manifest_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_batches: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitArtifactSummary {
    pub artifact_version: u32,
    pub members: Option<usize>,
    pub mtbdd_nodes: Option<usize>,
    pub mtbdd_terminals: Option<usize>,
    pub artifact_bytes: usize,
    pub elapsed_micros: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitBatchSummary {
    pub index: usize,
    pub members: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub verification_micros: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitSetSummary {
    pub controller_admissions: usize,
    pub members: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub controller_evidence_bytes: usize,
    pub admission_micros: usize,
    pub elapsed_micros: usize,
    pub batches: Vec<ControllerSplitBatchSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitResourceCapabilities {
    pub cli_version: u32,
    pub policy_version: u32,
    pub controller_envelope_version: u32,
    pub plant_envelope_version: u32,
    pub controller_artifact_version: u32,
    pub plant_artifact_version: u32,
    pub manifest_version: u32,
    pub max_policy_bytes: usize,
    pub max_controller_artifact_bytes: usize,
    pub max_unsat_proof_bytes: usize,
    pub max_plant_artifact_bytes: usize,
    pub max_batches: usize,
    pub max_members_per_batch: usize,
    pub max_horizon: usize,
    pub max_product_states: usize,
    pub refusal_exit_code: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitResourceBatchSummary {
    pub index: usize,
    pub members: usize,
    pub maximum_member_horizon: usize,
    pub maximum_product_states: usize,
    pub transition_evaluation_bound: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub verification_micros: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitResourceSetSummary {
    pub controller_admissions: usize,
    pub members: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub controller_evidence_bytes: usize,
    pub controller_mtbdd_bytes: usize,
    pub equivalence_artifact_bytes: usize,
    pub unsat_proof_bytes: usize,
    pub total_plant_artifact_bytes: usize,
    pub total_transition_evaluation_bound: usize,
    pub admission_micros: usize,
    pub elapsed_micros: usize,
    pub batches: Vec<ControllerSplitResourceBatchSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitObservabilityCapabilities {
    pub cli_version: u32,
    pub phase_metrics_version: u32,
    pub resource: ControllerSplitResourceCapabilities,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitPhaseMetrics {
    pub version: u32,
    pub policy_and_input_micros: u128,
    pub controller_admission_micros: u128,
    pub complete_set_preflight_micros: u128,
    pub semantic_replay_micros: u128,
    pub total_micros: u128,
    pub controller_admissions: usize,
    pub manifest_loads: usize,
    pub plant_artifact_reads: usize,
    pub resource_assessments: usize,
    pub batch_verifications: usize,
    pub buffered_result_rows: usize,
    pub prepared_batches: usize,
    pub prepared_members: usize,
    pub controller_evidence_bytes: usize,
    pub total_plant_artifact_bytes: usize,
    pub total_transition_evaluation_bound: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitObservedSummary {
    pub verification: ControllerSplitResourceSetSummary,
    pub phases: ControllerSplitPhaseMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitAllocationObservabilityCapabilities {
    pub cli_version: u32,
    pub observability: ControllerSplitObservabilityCapabilities,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitAllocationMetrics {
    pub version: u32,
    pub allocation_calls: u64,
    pub allocated_bytes: u64,
    pub deallocation_calls: u64,
    pub deallocated_bytes: u64,
    pub reallocation_calls: u64,
    pub reallocated_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitAllocationObservedSummary {
    pub observed: ControllerSplitObservedSummary,
    pub allocations: ControllerSplitAllocationMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitCacheObservabilityCapabilities {
    pub cli_version: u32,
    pub allocation_observability: ControllerSplitAllocationObservabilityCapabilities,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitCacheMetrics {
    pub version: u32,
    pub lookups: usize,
    pub hits: usize,
    pub misses: usize,
    pub entries: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSplitCacheObservedSummary {
    pub observed: ControllerSplitAllocationObservedSummary,
    pub cache: ControllerSplitCacheMetrics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerMtbddAnswer {
    Safe,
    Unsafe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddMemberResult {
    pub index: usize,
    pub answer: ControllerMtbddAnswer,
    pub horizon: usize,
    pub bad_frame: Option<usize>,
    pub trace_steps: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerMtbddBatchSummary {
    pub artifact_version: u32,
    pub safe: usize,
    pub unsafe_count: usize,
    pub mtbdd_nodes: usize,
    pub mtbdd_terminals: usize,
    pub assignments_checked: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub elapsed_micros: usize,
    pub members: Vec<ControllerMtbddMemberResult>,
}

pub type ControllerMtbddApiError = PredicateApiError;
pub type ControllerMtbddOperationError = PredicateOperationError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantPortfolioCapabilities {
    pub cli_version: u32,
    pub artifact_version: u32,
    pub manifest_version: u32,
    pub max_manifest_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_members: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPlantPortfolioBackend {
    Mtbdd,
    DirectExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPlantPortfolioReason {
    MtbddAdmitted,
    BoundaryLimit,
    TerminalLimit,
    NodeLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantPortfolioBatchSummary {
    pub artifact_version: u32,
    pub backend: ControllerPlantPortfolioBackend,
    pub reason: ControllerPlantPortfolioReason,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub load_micros: usize,
    pub artifact_micros: usize,
    pub verification_micros: usize,
    pub write_micros: usize,
    pub elapsed_micros: usize,
    pub members: Vec<ControllerMtbddMemberResult>,
}

pub type ControllerPlantPortfolioApiError = PredicateApiError;
pub type ControllerPlantPortfolioOperationError = PredicateOperationError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantResourceCapabilities {
    pub cli_version: u32,
    pub policy_version: u32,
    pub envelope_version: u32,
    pub manifest_version: u32,
    pub portfolio_artifact_version: u32,
    pub max_policy_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_members: usize,
    pub max_horizon: usize,
    pub max_product_states: usize,
    pub refusal_exit_code: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProofMtbddResourceCapabilities {
    pub cli_version: u32,
    pub policy_version: u32,
    pub envelope_version: u32,
    pub manifest_version: u32,
    pub artifact_version: u32,
    pub max_policy_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_equivalence_artifact_bytes: usize,
    pub max_unsat_proof_bytes: usize,
    pub max_members: usize,
    pub max_horizon: usize,
    pub max_product_states: usize,
    pub refusal_exit_code: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProofMtbddResourceSummary {
    pub policy_version: u32,
    pub envelope_version: u32,
    pub artifact_version: u32,
    pub members: usize,
    pub maximum_member_horizon: usize,
    pub maximum_product_states: usize,
    pub transition_evaluation_bound: usize,
    pub equivalence_artifact_bytes: usize,
    pub unsat_proof_bytes: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub assignments_checked: usize,
    pub load_micros: usize,
    pub artifact_micros: usize,
    pub verification_micros: usize,
    pub elapsed_micros: usize,
    pub member_results: Vec<ControllerMtbddMemberResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProofMtbddPortfolioCapabilities {
    pub cli_version: u32,
    pub policy_version: u32,
    pub envelope_version: u32,
    pub artifact_version: u32,
    pub proof_artifact_version: u32,
    pub direct_artifact_version: u32,
    pub manifest_version: u32,
    pub source_model_attestation_version: u32,
    pub max_policy_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_equivalence_artifact_bytes: usize,
    pub max_unsat_proof_bytes: usize,
    pub max_members: usize,
    pub max_horizon: usize,
    pub max_product_states: usize,
    pub max_attestation_bytes: usize,
    pub refusal_exit_code: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProofMtbddPortfolioResourceSummary {
    pub policy_version: u32,
    pub envelope_version: u32,
    pub artifact_version: u32,
    pub backend: ControllerPlantPortfolioBackend,
    pub reason: ControllerPlantPortfolioReason,
    pub members: usize,
    pub maximum_member_horizon: usize,
    pub maximum_product_states: usize,
    pub transition_evaluation_bound: usize,
    pub equivalence_artifact_bytes: usize,
    pub unsat_proof_bytes: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub assignments_checked: usize,
    pub load_micros: usize,
    pub artifact_micros: usize,
    pub verification_micros: usize,
    pub elapsed_micros: usize,
    pub member_results: Vec<ControllerMtbddMemberResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerPlantResourceSummary {
    pub policy_version: u32,
    pub envelope_version: u32,
    pub artifact_version: u32,
    pub backend: ControllerPlantPortfolioBackend,
    pub members: usize,
    pub maximum_member_horizon: usize,
    pub maximum_product_states: usize,
    pub transition_evaluation_bound: usize,
    pub safe: usize,
    pub unsafe_count: usize,
    pub reachable_product_states: usize,
    pub explored_transitions: usize,
    pub artifact_bytes: usize,
    pub load_micros: usize,
    pub artifact_micros: usize,
    pub verification_micros: usize,
    pub elapsed_micros: usize,
    pub member_results: Vec<ControllerMtbddMemberResult>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPlantResourceRefusalReason {
    ArtifactBytes,
    ControllerArtifactBytes,
    PlantArtifactBytes,
    EquivalenceArtifactBytes,
    UnsatProofBytes,
    Batches,
    Members,
    MembersPerBatch,
    Horizon,
    ProductStates,
    TransitionEvaluations,
    TransitionsPerBatch,
    TotalPlantArtifactBytes,
    TotalMembers,
    TotalTransitionEvaluations,
}

impl ControllerPlantResourceRefusalReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ArtifactBytes => "artifact-bytes",
            Self::ControllerArtifactBytes => "controller-artifact-bytes",
            Self::PlantArtifactBytes => "plant-artifact-bytes",
            Self::EquivalenceArtifactBytes => "equivalence-artifact-bytes",
            Self::UnsatProofBytes => "unsat-proof-bytes",
            Self::Batches => "batches",
            Self::Members => "members",
            Self::MembersPerBatch => "members-per-batch",
            Self::Horizon => "horizon",
            Self::ProductStates => "product-states",
            Self::TransitionEvaluations => "transition-evaluations",
            Self::TransitionsPerBatch => "transitions-per-batch",
            Self::TotalPlantArtifactBytes => "total-plant-artifact-bytes",
            Self::TotalMembers => "total-members",
            Self::TotalTransitionEvaluations => "total-transition-evaluations",
        }
    }
}

pub type ControllerPlantResourceApiError = PredicateApiError;
pub type ControllerPlantResourceOperationError = PredicateOperationError;

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
    ResourceRefused {
        reason: ControllerPlantResourceRefusalReason,
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
            Self::ResourceRefused { .. } => FailureClass::ResourceRefusal,
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
            Self::ResourceRefused { reason } => write!(
                formatter,
                "controller plant verification refused by resource policy: {}",
                reason.as_str()
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

/// Typed, shell-free client for revision-impact CLI v2.
#[derive(Clone, Debug)]
pub struct RevisionImpactTool {
    executable: PathBuf,
    capabilities: RevisionImpactCapabilities,
    policy: ExecutionPolicy,
}

impl RevisionImpactTool {
    /// Discover and validate the executable contract with a file limit that
    /// admits the largest advertised v1 bundle.
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        let policy = ExecutionPolicy::default()
            .with_file_limit(revision_impact::MAX_REVISION_IMPACT_CERTIFICATE_BYTES as u64)?;
        Self::discover_with_policy(executable, policy)
    }

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
        command.arg("btor2-revision-impact-cli-version");
        let output = run_bounded(OperationKind::DiscoverRevisionImpact, command, policy)?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_revision_impact_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &RevisionImpactCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn certify(
        &self,
        files: &RevisionImpactFiles<'_>,
        artifact: &Path,
    ) -> Result<RevisionImpactProcessSummary, PredicateApiError> {
        self.certify_observed(files, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_observed(
        &self,
        files: &RevisionImpactFiles<'_>,
        artifact: &Path,
    ) -> Result<Observed<RevisionImpactProcessSummary>, PredicateOperationError> {
        self.invoke(
            OperationKind::CertifyRevisionImpact,
            "check-btor2-revision-impact",
            "CREATED",
            files,
            artifact,
        )
    }

    pub fn verify(
        &self,
        files: &RevisionImpactFiles<'_>,
        artifact: &Path,
    ) -> Result<RevisionImpactProcessSummary, PredicateApiError> {
        self.verify_observed(files, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        files: &RevisionImpactFiles<'_>,
        artifact: &Path,
    ) -> Result<Observed<RevisionImpactProcessSummary>, PredicateOperationError> {
        self.invoke(
            OperationKind::VerifyRevisionImpact,
            "verify-btor2-revision-impact",
            "VERIFIED",
            files,
            artifact,
        )
    }

    fn invoke(
        &self,
        operation: OperationKind,
        command_name: &str,
        expected_status: &str,
        files: &RevisionImpactFiles<'_>,
        artifact: &Path,
    ) -> Result<Observed<RevisionImpactProcessSummary>, PredicateOperationError> {
        let left_outputs =
            canonical_node_list(files.left_outputs).map_err(|error| PredicateOperationError {
                metrics: empty_metrics(
                    operation,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            })?;
        let right_outputs =
            canonical_node_list(files.right_outputs).map_err(|error| PredicateOperationError {
                metrics: empty_metrics(
                    operation,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            })?;
        let mut command = Command::new(&self.executable);
        command
            .arg(command_name)
            .arg(files.left_old)
            .arg(files.left_new)
            .arg(left_outputs)
            .arg(files.right_old)
            .arg(files.right_new)
            .arg(right_outputs)
            .arg(files.interface_old)
            .arg(files.interface_new)
            .arg(files.queries)
            .arg(artifact);
        let output = run_bounded(operation, command, self.policy)?;
        parse_revision_impact_summary(output, expected_status, &self.capabilities)
    }
}

/// Typed, shell-free client for controller MTBDD plant CLI v1.
#[derive(Clone, Debug)]
pub struct ControllerMtbddTool {
    executable: PathBuf,
    capabilities: ControllerMtbddCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerMtbddTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, ControllerMtbddApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

    pub fn discover_with_policy(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Self, ControllerMtbddApiError> {
        Self::discover_observed(executable, policy)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn discover_observed(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Observed<Self>, ControllerMtbddOperationError> {
        let executable = executable.into();
        let mut command = Command::new(&executable);
        command.arg("controller-mtbdd-cli-version");
        let output = run_bounded(OperationKind::DiscoverControllerMtbdd, command, policy)?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_controller_mtbdd_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerMtbddCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn certify(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<ControllerMtbddBatchSummary, ControllerMtbddApiError> {
        self.certify_observed(manifest, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_observed(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerMtbddBatchSummary>, ControllerMtbddOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("certify-controller-mtbdd-plant-batch")
            .arg(manifest)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::CertifyControllerMtbddPlantBatch,
            command,
            self.policy,
        )?;
        parse_controller_mtbdd_summary(
            output,
            "CREATED",
            Some(artifact),
            &self.capabilities,
            "controller-mtbdd-plant-batch",
            "controller-mtbdd-plant-member",
        )
    }

    pub fn verify(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<ControllerMtbddBatchSummary, ControllerMtbddApiError> {
        self.verify_observed(manifest, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerMtbddBatchSummary>, ControllerMtbddOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-mtbdd-plant-batch")
            .arg(manifest)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::VerifyControllerMtbddPlantBatch,
            command,
            self.policy,
        )?;
        parse_controller_mtbdd_summary(
            output,
            "VERIFIED",
            None,
            &self.capabilities,
            "controller-mtbdd-plant-batch",
            "controller-mtbdd-plant-member",
        )
    }
}

/// Typed, shell-free client for proof-carrying controller MTBDD plant CLI v1.
#[derive(Clone, Debug)]
pub struct ControllerProofMtbddTool {
    executable: PathBuf,
    capabilities: ControllerProofMtbddCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerProofMtbddTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, ControllerMtbddApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

    pub fn discover_with_policy(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Self, ControllerMtbddApiError> {
        Self::discover_observed(executable, policy)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn discover_observed(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Observed<Self>, ControllerMtbddOperationError> {
        let executable = executable.into();
        let mut command = Command::new(&executable);
        command.arg("controller-proof-mtbdd-cli-version");
        let output = run_bounded(OperationKind::DiscoverControllerProofMtbdd, command, policy)?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_controller_proof_mtbdd_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerProofMtbddCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn certify(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<ControllerMtbddBatchSummary, ControllerMtbddApiError> {
        self.certify_observed(manifest, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_observed(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerMtbddBatchSummary>, ControllerMtbddOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("certify-controller-proof-mtbdd-plant-batch")
            .arg(manifest)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::CertifyControllerProofMtbddPlantBatch,
            command,
            self.policy,
        )?;
        parse_controller_mtbdd_summary(
            output,
            "CREATED",
            Some(artifact),
            &self.capabilities.common(),
            "controller-proof-mtbdd-plant-batch",
            "controller-proof-mtbdd-plant-member",
        )
    }

    pub fn verify(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<ControllerMtbddBatchSummary, ControllerMtbddApiError> {
        self.verify_observed(manifest, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerMtbddBatchSummary>, ControllerMtbddOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-proof-mtbdd-plant-batch")
            .arg(manifest)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::VerifyControllerProofMtbddPlantBatch,
            command,
            self.policy,
        )?;
        parse_controller_mtbdd_summary(
            output,
            "VERIFIED",
            None,
            &self.capabilities.common(),
            "controller-proof-mtbdd-plant-batch",
            "controller-proof-mtbdd-plant-member",
        )
    }
}

/// Typed, shell-free client for split controller evidence and plant batches.
#[derive(Clone, Debug)]
pub struct ControllerSplitEvidenceTool {
    executable: PathBuf,
    capabilities: ControllerSplitEvidenceCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerSplitEvidenceTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-split-evidence-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerSplitEvidence,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_split_evidence_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerSplitEvidenceCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn certify_controller_evidence(
        &self,
        manifest: &Path,
        output: &Path,
    ) -> Result<ControllerSplitArtifactSummary, PredicateApiError> {
        self.certify_controller_evidence_observed(manifest, output)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_controller_evidence_observed(
        &self,
        manifest: &Path,
        output: &Path,
    ) -> Result<Observed<ControllerSplitArtifactSummary>, PredicateOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("certify-controller-proof-evidence-v1")
            .arg(manifest)
            .arg(output);
        let bounded = run_bounded(
            OperationKind::CertifyControllerProofEvidence,
            command,
            self.policy,
        )?;
        parse_controller_split_artifact_summary(
            bounded,
            "controller-split-evidence",
            false,
            output,
            &self.capabilities,
        )
    }

    pub fn certify_plant_results(
        &self,
        manifest: &Path,
        evidence: &Path,
        output: &Path,
    ) -> Result<ControllerSplitArtifactSummary, PredicateApiError> {
        self.certify_plant_results_observed(manifest, evidence, output)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_plant_results_observed(
        &self,
        manifest: &Path,
        evidence: &Path,
        output: &Path,
    ) -> Result<Observed<ControllerSplitArtifactSummary>, PredicateOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("certify-bound-plant-results-v1")
            .arg(manifest)
            .arg(evidence)
            .arg(output);
        let bounded = run_bounded(
            OperationKind::CertifyBoundPlantResults,
            command,
            self.policy,
        )?;
        parse_controller_split_artifact_summary(
            bounded,
            "controller-split-plant",
            true,
            output,
            &self.capabilities,
        )
    }

    pub fn verify_set(
        &self,
        evidence: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<ControllerSplitSetSummary, PredicateApiError> {
        self.verify_set_observed(evidence, batches)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_set_observed(
        &self,
        evidence: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<Observed<ControllerSplitSetSummary>, PredicateOperationError> {
        if batches.is_empty() || batches.len() > self.capabilities.max_batches {
            let error = PredicateApiError::InvalidPolicy(
                "split-evidence batch count is outside discovered limits".to_string(),
            );
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    OperationKind::VerifyBoundPlantResultSet,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-bound-plant-result-set-v1")
            .arg(evidence);
        for &(manifest, result) in batches {
            command.arg(manifest).arg(result);
        }
        let bounded = run_bounded(
            OperationKind::VerifyBoundPlantResultSet,
            command,
            self.policy,
        )?;
        parse_controller_split_set_summary(bounded, batches.len(), &self.capabilities)
    }
}

/// Typed, shell-free client for governed split-evidence verification v1.
#[derive(Clone, Debug)]
pub struct ControllerSplitResourceTool {
    executable: PathBuf,
    capabilities: ControllerSplitResourceCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerSplitResourceTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-split-resource-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerSplitResource,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_split_resource_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerSplitResourceCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify_set(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<ControllerSplitResourceSetSummary, PredicateApiError> {
        self.verify_set_observed(evidence, resource_policy, batches)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_set_observed(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<Observed<ControllerSplitResourceSetSummary>, PredicateOperationError> {
        if batches.is_empty() || batches.len() > self.capabilities.max_batches {
            let error = PredicateApiError::InvalidPolicy(
                "governed split-evidence batch count is outside discovered limits".to_string(),
            );
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    OperationKind::VerifyBoundPlantResultSetResources,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-bound-plant-result-set-with-resources-v1")
            .arg(evidence)
            .arg(resource_policy);
        for &(manifest, result) in batches {
            command.arg(manifest).arg(result);
        }
        let output = run_bounded(
            OperationKind::VerifyBoundPlantResultSetResources,
            command,
            self.policy,
        )?;
        parse_controller_split_resource_set_summary(output, batches.len(), &self.capabilities)
            .map_err(classify_controller_split_resource_refusal)
    }
}

/// Typed, shell-free client for governed split verification with phase metrics.
#[derive(Clone, Debug)]
pub struct ControllerSplitObservabilityTool {
    executable: PathBuf,
    capabilities: ControllerSplitObservabilityCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerSplitObservabilityTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-split-observability-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerSplitObservability,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_split_observability_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerSplitObservabilityCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify_set(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<ControllerSplitObservedSummary, PredicateApiError> {
        self.verify_set_observed(evidence, resource_policy, batches)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_set_observed(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<Observed<ControllerSplitObservedSummary>, PredicateOperationError> {
        if batches.is_empty() || batches.len() > self.capabilities.resource.max_batches {
            let error = PredicateApiError::InvalidPolicy(
                "observed split-evidence batch count is outside discovered limits".to_string(),
            );
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    OperationKind::VerifyBoundPlantResultSetResourcesObserved,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-bound-plant-result-set-with-resources-observed-v1")
            .arg(evidence)
            .arg(resource_policy);
        for &(manifest, result) in batches {
            command.arg(manifest).arg(result);
        }
        let output = run_bounded(
            OperationKind::VerifyBoundPlantResultSetResourcesObserved,
            command,
            self.policy,
        )?;
        parse_controller_split_observed_summary(output, batches.len(), &self.capabilities)
            .map_err(classify_controller_split_resource_refusal)
    }
}

/// Typed client for governed split verification with allocator accounting.
#[derive(Clone, Debug)]
pub struct ControllerSplitAllocationObservabilityTool {
    executable: PathBuf,
    capabilities: ControllerSplitAllocationObservabilityCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerSplitAllocationObservabilityTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-split-allocation-observability-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerSplitAllocationObservability,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_controller_split_allocation_observability_capabilities(&stdout)
            .map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerSplitAllocationObservabilityCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify_set(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<ControllerSplitAllocationObservedSummary, PredicateApiError> {
        self.verify_set_observed(evidence, resource_policy, batches)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_set_observed(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<Observed<ControllerSplitAllocationObservedSummary>, PredicateOperationError> {
        if batches.is_empty()
            || batches.len() > self.capabilities.observability.resource.max_batches
        {
            let error = PredicateApiError::InvalidPolicy(
                "allocation-observed split-evidence batch count is outside discovered limits"
                    .to_string(),
            );
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    OperationKind::VerifyBoundPlantResultSetResourcesAllocationObserved,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-bound-plant-result-set-with-resources-allocation-observed-v1")
            .arg(evidence)
            .arg(resource_policy);
        for &(manifest, result) in batches {
            command.arg(manifest).arg(result);
        }
        let output = run_bounded(
            OperationKind::VerifyBoundPlantResultSetResourcesAllocationObserved,
            command,
            self.policy,
        )?;
        parse_controller_split_allocation_observed_summary(
            output,
            batches.len(),
            &self.capabilities,
        )
        .map_err(classify_controller_split_resource_refusal)
    }
}

/// Typed client for governed split verification with integrity-preserving
/// semantic replay caching and allocator accounting.
#[derive(Clone, Debug)]
pub struct ControllerSplitCacheObservabilityTool {
    executable: PathBuf,
    capabilities: ControllerSplitCacheObservabilityCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerSplitCacheObservabilityTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-split-cache-observability-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerSplitCacheObservability,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities = parse_controller_split_cache_observability_capabilities(&stdout)
            .map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerSplitCacheObservabilityCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify_set(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<ControllerSplitCacheObservedSummary, PredicateApiError> {
        self.verify_set_observed(evidence, resource_policy, batches)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_set_observed(
        &self,
        evidence: &Path,
        resource_policy: &Path,
        batches: &[(&Path, &Path)],
    ) -> Result<Observed<ControllerSplitCacheObservedSummary>, PredicateOperationError> {
        if batches.is_empty()
            || batches.len()
                > self
                    .capabilities
                    .allocation_observability
                    .observability
                    .resource
                    .max_batches
        {
            let error = PredicateApiError::InvalidPolicy(
                "cache-observed split-evidence batch count is outside discovered limits"
                    .to_string(),
            );
            return Err(PredicateOperationError {
                metrics: empty_metrics(
                    OperationKind::VerifyBoundPlantResultSetResourcesCacheObserved,
                    self.policy,
                    InvocationStatus::Failed(error.failure_class()),
                ),
                error: Box::new(error),
            });
        }
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-bound-plant-result-set-with-resources-cache-observed-v1")
            .arg(evidence)
            .arg(resource_policy);
        for &(manifest, result) in batches {
            command.arg(manifest).arg(result);
        }
        let output = run_bounded(
            OperationKind::VerifyBoundPlantResultSetResourcesCacheObserved,
            command,
            self.policy,
        )?;
        parse_controller_split_cache_observed_summary(output, batches.len(), &self.capabilities)
            .map_err(classify_controller_split_resource_refusal)
    }
}

/// Typed, shell-free client for statically routed controller/plant portfolio v1.
#[derive(Clone, Debug)]
pub struct ControllerPlantPortfolioTool {
    executable: PathBuf,
    capabilities: ControllerPlantPortfolioCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerPlantPortfolioTool {
    pub fn discover(
        executable: impl Into<PathBuf>,
    ) -> Result<Self, ControllerPlantPortfolioApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

    pub fn discover_with_policy(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Self, ControllerPlantPortfolioApiError> {
        Self::discover_observed(executable, policy)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn discover_observed(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Observed<Self>, ControllerPlantPortfolioOperationError> {
        let executable = executable.into();
        let mut command = Command::new(&executable);
        command.arg("controller-plant-portfolio-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerPlantPortfolio,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_plant_portfolio_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerPlantPortfolioCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn certify(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<ControllerPlantPortfolioBatchSummary, ControllerPlantPortfolioApiError> {
        self.certify_observed(manifest, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn certify_observed(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<
        Observed<ControllerPlantPortfolioBatchSummary>,
        ControllerPlantPortfolioOperationError,
    > {
        let mut command = Command::new(&self.executable);
        command
            .arg("certify-controller-plant-portfolio")
            .arg(manifest)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::CertifyControllerPlantPortfolio,
            command,
            self.policy,
        )?;
        parse_controller_plant_portfolio_summary(
            output,
            "CREATED",
            Some(artifact),
            &self.capabilities,
        )
    }

    pub fn verify(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<ControllerPlantPortfolioBatchSummary, ControllerPlantPortfolioApiError> {
        self.verify_observed(manifest, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        manifest: &Path,
        artifact: &Path,
    ) -> Result<
        Observed<ControllerPlantPortfolioBatchSummary>,
        ControllerPlantPortfolioOperationError,
    > {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-plant-portfolio")
            .arg(manifest)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::VerifyControllerPlantPortfolio,
            command,
            self.policy,
        )?;
        parse_controller_plant_portfolio_summary(output, "VERIFIED", None, &self.capabilities)
    }
}

/// Typed, shell-free client for governed controller/plant verification v1.
#[derive(Clone, Debug)]
pub struct ControllerPlantResourceTool {
    executable: PathBuf,
    capabilities: ControllerPlantResourceCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerPlantResourceTool {
    pub fn discover(
        executable: impl Into<PathBuf>,
    ) -> Result<Self, ControllerPlantResourceApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

    pub fn discover_with_policy(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Self, ControllerPlantResourceApiError> {
        Self::discover_observed(executable, policy)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn discover_observed(
        executable: impl Into<PathBuf>,
        policy: ExecutionPolicy,
    ) -> Result<Observed<Self>, ControllerPlantResourceOperationError> {
        let executable = executable.into();
        let mut command = Command::new(&executable);
        command.arg("controller-plant-resource-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerPlantResource,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_plant_resource_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerPlantResourceCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
    ) -> Result<ControllerPlantResourceSummary, ControllerPlantResourceApiError> {
        self.verify_observed(manifest, resource_policy, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerPlantResourceSummary>, ControllerPlantResourceOperationError>
    {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-plant-portfolio-resources")
            .arg(manifest)
            .arg(resource_policy)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::VerifyControllerPlantPortfolioResources,
            command,
            self.policy,
        )?;
        parse_controller_plant_resource_summary(output, &self.capabilities)
            .map_err(classify_controller_plant_resource_refusal)
    }
}

/// Typed, shell-free client for governed proof-carrying MTBDD verification v1.
#[derive(Clone, Debug)]
pub struct ControllerProofMtbddResourceTool {
    executable: PathBuf,
    capabilities: ControllerProofMtbddResourceCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerProofMtbddResourceTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-proof-mtbdd-resource-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerProofMtbddResource,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_proof_mtbdd_resource_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerProofMtbddResourceCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
    ) -> Result<ControllerProofMtbddResourceSummary, PredicateApiError> {
        self.verify_observed(manifest, resource_policy, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerProofMtbddResourceSummary>, PredicateOperationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-proof-mtbdd-plant-resources")
            .arg(manifest)
            .arg(resource_policy)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::VerifyControllerProofMtbddPlantResources,
            command,
            self.policy,
        )?;
        parse_controller_proof_mtbdd_resource_summary(output, &self.capabilities)
            .map_err(classify_controller_proof_mtbdd_resource_refusal)
    }
}

/// Typed, shell-free client for the governed proof/direct controller portfolio.
#[derive(Clone, Debug)]
pub struct ControllerProofMtbddPortfolioTool {
    executable: PathBuf,
    capabilities: ControllerProofMtbddPortfolioCapabilities,
    policy: ExecutionPolicy,
}

impl ControllerProofMtbddPortfolioTool {
    pub fn discover(executable: impl Into<PathBuf>) -> Result<Self, PredicateApiError> {
        Self::discover_with_policy(executable, ExecutionPolicy::default())
    }

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
        command.arg("controller-proof-mtbdd-portfolio-cli-version");
        let output = run_bounded(
            OperationKind::DiscoverControllerProofMtbddPortfolio,
            command,
            policy,
        )?;
        let (stdout, mut metrics) = successful_stdout(output)?;
        let capabilities =
            parse_controller_proof_mtbdd_portfolio_capabilities(&stdout).map_err(|error| {
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

    pub fn capabilities(&self) -> &ControllerProofMtbddPortfolioCapabilities {
        &self.capabilities
    }

    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.policy
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
    ) -> Result<ControllerProofMtbddPortfolioResourceSummary, PredicateApiError> {
        self.verify_observed(manifest, resource_policy, artifact)
            .map(|observed| observed.value)
            .map_err(|failure| *failure.error)
    }

    pub fn verify_observed(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
    ) -> Result<Observed<ControllerProofMtbddPortfolioResourceSummary>, PredicateOperationError>
    {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-proof-mtbdd-portfolio-resources")
            .arg(manifest)
            .arg(resource_policy)
            .arg(artifact);
        let output = run_bounded(
            OperationKind::VerifyControllerProofMtbddPortfolioResources,
            command,
            self.policy,
        )?;
        parse_controller_proof_mtbdd_portfolio_resource_summary(output, &self.capabilities, false)
            .map_err(classify_controller_proof_mtbdd_resource_refusal)
    }

    pub fn verify_attested(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
        provenance_manifest: &Path,
        attestation: &Path,
    ) -> Result<ControllerProofMtbddPortfolioResourceSummary, PredicateApiError> {
        self.verify_attested_observed(
            manifest,
            resource_policy,
            artifact,
            provenance_manifest,
            attestation,
        )
        .map(|observed| observed.value)
        .map_err(|failure| *failure.error)
    }

    pub fn verify_attested_observed(
        &self,
        manifest: &Path,
        resource_policy: &Path,
        artifact: &Path,
        provenance_manifest: &Path,
        attestation: &Path,
    ) -> Result<Observed<ControllerProofMtbddPortfolioResourceSummary>, PredicateOperationError>
    {
        let mut command = Command::new(&self.executable);
        command
            .arg("verify-controller-proof-mtbdd-portfolio-resources-attested")
            .arg(manifest)
            .arg(resource_policy)
            .arg(artifact)
            .arg(provenance_manifest)
            .arg(attestation);
        let output = run_bounded(
            OperationKind::VerifyControllerProofMtbddPortfolioResourcesAttested,
            command,
            self.policy,
        )?;
        parse_controller_proof_mtbdd_portfolio_resource_summary(output, &self.capabilities, true)
            .map_err(classify_controller_proof_mtbdd_resource_refusal)
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

fn canonical_usize(value: &str, field: &str) -> Result<usize, PredicateApiError> {
    let parsed = value.parse::<usize>().map_err(|_| {
        PredicateApiError::InvalidResponse(format!("{field} is not an unsigned integer"))
    })?;
    if parsed.to_string() != value {
        return Err(PredicateApiError::InvalidResponse(format!(
            "{field} is noncanonical"
        )));
    }
    Ok(parsed)
}

fn canonical_u32(value: &str, field: &str) -> Result<u32, PredicateApiError> {
    let parsed = canonical_usize(value, field)?;
    u32::try_from(parsed).map_err(|_| {
        PredicateApiError::InvalidResponse(format!("{field} exceeds canonical u32 range"))
    })
}

fn canonical_u128(value: &str, field: &str) -> Result<u128, PredicateApiError> {
    let parsed = value.parse::<u128>().map_err(|_| {
        PredicateApiError::InvalidResponse(format!("{field} is not an unsigned integer"))
    })?;
    if parsed.to_string() != value {
        return Err(PredicateApiError::InvalidResponse(format!(
            "{field} is noncanonical"
        )));
    }
    Ok(parsed)
}

fn canonical_u64(value: &str, field: &str) -> Result<u64, PredicateApiError> {
    let parsed = value.parse::<u64>().map_err(|_| {
        PredicateApiError::InvalidResponse(format!("{field} is not an unsigned integer"))
    })?;
    if parsed.to_string() != value {
        return Err(PredicateApiError::InvalidResponse(format!(
            "{field} is noncanonical"
        )));
    }
    Ok(parsed)
}

fn token_value<'a>(token: &'a str, key: &str) -> Result<&'a str, PredicateApiError> {
    token
        .strip_prefix(&format!("{key}="))
        .ok_or_else(|| PredicateApiError::InvalidResponse(format!("expected response field {key}")))
}

fn parse_controller_proof_mtbdd_resource_capabilities(
    line: &str,
) -> Result<ControllerProofMtbddResourceCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller proof MTBDD resource capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_proof_mtbdd_resource_cli_version",
        "policy_version",
        "envelope_version",
        "manifest_version",
        "artifact_version",
        "max_policy_bytes",
        "max_artifact_bytes",
        "max_equivalence_artifact_bytes",
        "max_unsat_proof_bytes",
        "max_members",
        "max_horizon",
        "max_product_states",
        "refusal_exit",
        "verification",
        "exhaustive_replay",
        "accounting",
        "timing_calibration",
        "result_on_refusal",
        "refusal_schema",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller proof MTBDD resource capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[13..]
        != [
            "unsat-miter",
            "no",
            "conservative-static",
            "none",
            "none",
            "proof-reason-v1",
            "fail-closed",
        ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller proof MTBDD resource contract is unsupported".to_string(),
        ));
    }
    let versions = values[..5]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[5..12]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 5]))
        .collect::<Result<Vec<_>, _>>()?;
    let refusal_exit = canonical_usize(values[12], keys[12])?;
    if versions != [1, 1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller proof MTBDD resource version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) || refusal_exit != 3 {
        return Err(PredicateApiError::InvalidResponse(
            "controller proof MTBDD resource discovered limit must be positive".to_string(),
        ));
    }
    Ok(ControllerProofMtbddResourceCapabilities {
        cli_version: versions[0],
        policy_version: versions[1],
        envelope_version: versions[2],
        manifest_version: versions[3],
        artifact_version: versions[4],
        max_policy_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_equivalence_artifact_bytes: limits[2],
        max_unsat_proof_bytes: limits[3],
        max_members: limits[4],
        max_horizon: limits[5],
        max_product_states: limits[6],
        refusal_exit_code: 3,
    })
}

fn parse_controller_proof_mtbdd_portfolio_capabilities(
    line: &str,
) -> Result<ControllerProofMtbddPortfolioCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller proof MTBDD portfolio capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_proof_mtbdd_portfolio_cli_version",
        "policy_version",
        "envelope_version",
        "artifact_version",
        "proof_artifact_version",
        "direct_artifact_version",
        "manifest_version",
        "source_model_attestation_version",
        "max_policy_bytes",
        "max_artifact_bytes",
        "max_equivalence_artifact_bytes",
        "max_unsat_proof_bytes",
        "max_members",
        "max_horizon",
        "max_product_states",
        "max_attestation_bytes",
        "refusal_exit",
        "backends",
        "routing",
        "fallback",
        "proof_failure",
        "attested_verification",
        "accounting",
        "timing_calibration",
        "result_on_refusal",
        "refusal_schema",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller proof MTBDD portfolio capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[17..]
        != [
            "proof-mtbdd,direct-exact",
            "static",
            "exact",
            "fail-closed",
            "required",
            "conservative-static",
            "none",
            "none",
            "proof-reason-v1",
            "fail-closed",
        ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller proof MTBDD portfolio contract is unsupported".to_string(),
        ));
    }
    let versions = values[..8]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[8..16]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 8]))
        .collect::<Result<Vec<_>, _>>()?;
    let refusal_exit = canonical_usize(values[16], keys[16])?;
    if versions != [1, 1, 1, 1, 1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller proof MTBDD portfolio version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) || refusal_exit != 3 {
        return Err(PredicateApiError::InvalidResponse(
            "controller proof MTBDD portfolio discovered limit must be positive".to_string(),
        ));
    }
    Ok(ControllerProofMtbddPortfolioCapabilities {
        cli_version: versions[0],
        policy_version: versions[1],
        envelope_version: versions[2],
        artifact_version: versions[3],
        proof_artifact_version: versions[4],
        direct_artifact_version: versions[5],
        manifest_version: versions[6],
        source_model_attestation_version: versions[7],
        max_policy_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_equivalence_artifact_bytes: limits[2],
        max_unsat_proof_bytes: limits[3],
        max_members: limits[4],
        max_horizon: limits[5],
        max_product_states: limits[6],
        max_attestation_bytes: limits[7],
        refusal_exit_code: 3,
    })
}

fn classify_controller_proof_mtbdd_resource_refusal(
    mut failure: PredicateOperationError,
) -> PredicateOperationError {
    let reason = match failure.error.as_ref() {
        PredicateApiError::CommandFailed {
            exit_code: Some(3),
            stderr,
        } => stderr
            .trim_end_matches(['\r', '\n'])
            .strip_prefix("error: controller-proof-mtbdd-resource refusal=")
            .and_then(|value| value.strip_suffix(" result=none"))
            .and_then(|value| match value {
                "artifact-bytes" => Some(ControllerPlantResourceRefusalReason::ArtifactBytes),
                "equivalence-artifact-bytes" => {
                    Some(ControllerPlantResourceRefusalReason::EquivalenceArtifactBytes)
                }
                "unsat-proof-bytes" => Some(ControllerPlantResourceRefusalReason::UnsatProofBytes),
                "members" => Some(ControllerPlantResourceRefusalReason::Members),
                "horizon" => Some(ControllerPlantResourceRefusalReason::Horizon),
                "product-states" => Some(ControllerPlantResourceRefusalReason::ProductStates),
                "transition-evaluations" => {
                    Some(ControllerPlantResourceRefusalReason::TransitionEvaluations)
                }
                _ => None,
            }),
        _ => None,
    };
    if let Some(reason) = reason {
        let error = PredicateApiError::ResourceRefused { reason };
        failure.metrics.status = InvocationStatus::Failed(error.failure_class());
        failure.error = Box::new(error);
    }
    failure
}

fn parse_controller_plant_resource_capabilities(
    line: &str,
) -> Result<ControllerPlantResourceCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller plant resource capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_plant_resource_cli_version",
        "policy_version",
        "envelope_version",
        "manifest_version",
        "portfolio_artifact_version",
        "max_policy_bytes",
        "max_artifact_bytes",
        "max_members",
        "max_horizon",
        "max_product_states",
        "refusal_exit",
        "accounting",
        "timing_calibration",
        "result_on_refusal",
        "refusal_schema",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller plant resource capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[11..]
        != [
            "conservative-static",
            "none",
            "none",
            "reason-v1",
            "fail-closed",
        ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller plant resource accounting contract is unsupported".to_string(),
        ));
    }
    let versions = values[..5]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[5..10]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 5]))
        .collect::<Result<Vec<_>, _>>()?;
    let refusal_exit = canonical_usize(values[10], keys[10])?;
    if versions != [1, 1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller plant resource version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) || refusal_exit != 3 {
        return Err(PredicateApiError::InvalidResponse(
            "controller plant resource discovered limit must be positive".to_string(),
        ));
    }
    Ok(ControllerPlantResourceCapabilities {
        cli_version: versions[0],
        policy_version: versions[1],
        envelope_version: versions[2],
        manifest_version: versions[3],
        portfolio_artifact_version: versions[4],
        max_policy_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_members: limits[2],
        max_horizon: limits[3],
        max_product_states: limits[4],
        refusal_exit_code: 3,
    })
}

fn classify_controller_plant_resource_refusal(
    mut failure: PredicateOperationError,
) -> PredicateOperationError {
    let reason = match failure.error.as_ref() {
        PredicateApiError::CommandFailed {
            exit_code: Some(3),
            stderr,
        } => stderr
            .trim_end_matches(['\r', '\n'])
            .strip_prefix("error: controller-plant-resource refusal=")
            .and_then(|value| value.strip_suffix(" result=none"))
            .and_then(|value| match value {
                "artifact-bytes" => Some(ControllerPlantResourceRefusalReason::ArtifactBytes),
                "members" => Some(ControllerPlantResourceRefusalReason::Members),
                "horizon" => Some(ControllerPlantResourceRefusalReason::Horizon),
                "product-states" => Some(ControllerPlantResourceRefusalReason::ProductStates),
                "transition-evaluations" => {
                    Some(ControllerPlantResourceRefusalReason::TransitionEvaluations)
                }
                _ => None,
            }),
        _ => None,
    };
    if let Some(reason) = reason {
        let error = PredicateApiError::ResourceRefused { reason };
        failure.metrics.status = InvocationStatus::Failed(error.failure_class());
        failure.error = Box::new(error);
    }
    failure
}

fn parse_controller_plant_resource_summary(
    output: ManagedOutput,
    capabilities: &ControllerPlantResourceCapabilities,
) -> Result<Observed<ControllerPlantResourceSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerPlantResourceSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant resource response is not canonical LF text".to_string(),
            ));
        }
        let mut lines = stdout.lines();
        let first = lines.next().ok_or_else(|| {
            PredicateApiError::InvalidResponse(
                "controller plant resource summary line is missing".to_string(),
            )
        })?;
        let fields = first.split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-plant-resource",
            "status",
            "cli_version",
            "policy_version",
            "envelope_version",
            "artifact_version",
            "backend",
            "members",
            "maximum_member_horizon",
            "maximum_product_states",
            "transition_evaluation_bound",
            "safe",
            "unsafe",
            "reachable_product_states",
            "explored_transitions",
            "artifact_bytes",
            "load_micros",
            "artifact_micros",
            "verification_micros",
            "elapsed_micros",
        ];
        if fields.len() != keys.len()
            || fields[0] != keys[0]
            || token_value(fields[1], "status")? != "VERIFIED"
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant resource summary shape is invalid".to_string(),
            ));
        }
        let cli_version = canonical_u32(token_value(fields[2], keys[2])?, keys[2])?;
        let policy_version = canonical_u32(token_value(fields[3], keys[3])?, keys[3])?;
        let envelope_version = canonical_u32(token_value(fields[4], keys[4])?, keys[4])?;
        let artifact_version = canonical_u32(token_value(fields[5], keys[5])?, keys[5])?;
        if cli_version != capabilities.cli_version
            || policy_version != capabilities.policy_version
            || envelope_version != capabilities.envelope_version
            || artifact_version != capabilities.portfolio_artifact_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller plant resource response version changed".to_string(),
            ));
        }
        let backend = match token_value(fields[6], "backend")? {
            "MTBDD" => ControllerPlantPortfolioBackend::Mtbdd,
            "DIRECT_EXACT" => ControllerPlantPortfolioBackend::DirectExact,
            _ => {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant resource backend is invalid".to_string(),
                ));
            }
        };
        let numeric = fields[7..]
            .iter()
            .zip(&keys[7..])
            .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
            .collect::<Result<Vec<_>, _>>()?;
        let member_count = numeric[0];
        if member_count == 0
            || member_count > capabilities.max_members
            || numeric[1] > capabilities.max_horizon
            || numeric[2] > capabilities.max_product_states
            || numeric[8] > capabilities.max_artifact_bytes
            || numeric[4].checked_add(numeric[5]) != Some(member_count)
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant resource response exceeds discovered limits".to_string(),
            ));
        }
        let mut member_results = Vec::with_capacity(member_count);
        for expected_index in 0..member_count {
            let line = lines.next().ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller plant resource member line is missing".to_string(),
                )
            })?;
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                "controller-plant-resource-member",
                "index",
                "answer",
                "horizon",
                "bad_frame",
                "trace_steps",
                "reachable_product_states",
                "explored_transitions",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant resource member shape is invalid".to_string(),
                ));
            }
            let index = canonical_usize(token_value(fields[1], "index")?, "index")?;
            if index != expected_index {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant resource member ordering changed".to_string(),
                ));
            }
            let answer = match token_value(fields[2], "answer")? {
                "SAFE" => ControllerMtbddAnswer::Safe,
                "UNSAFE" => ControllerMtbddAnswer::Unsafe,
                _ => {
                    return Err(PredicateApiError::InvalidResponse(
                        "controller plant resource member answer is invalid".to_string(),
                    ));
                }
            };
            let horizon = canonical_usize(token_value(fields[3], "horizon")?, "horizon")?;
            let bad_frame = match token_value(fields[4], "bad_frame")? {
                "none" => None,
                value => Some(canonical_usize(value, "bad_frame")?),
            };
            let values = fields[5..]
                .iter()
                .zip(&keys[5..])
                .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
                .collect::<Result<Vec<_>, _>>()?;
            if horizon > capabilities.max_horizon
                || match (answer, bad_frame) {
                    (ControllerMtbddAnswer::Safe, None) => values[0] != 0,
                    (ControllerMtbddAnswer::Unsafe, Some(frame)) => {
                        frame > horizon || values[0] != frame.saturating_add(1)
                    }
                    _ => true,
                }
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant resource member result is inconsistent".to_string(),
                ));
            }
            member_results.push(ControllerMtbddMemberResult {
                index,
                answer,
                horizon,
                bad_frame,
                trace_steps: values[0],
                reachable_product_states: values[1],
                explored_transitions: values[2],
            });
        }
        let reachable_total = member_results.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.reachable_product_states)
        });
        let transition_total = member_results.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.explored_transitions)
        });
        if lines.next().is_some()
            || member_results
                .iter()
                .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Safe))
                .count()
                != numeric[4]
            || member_results
                .iter()
                .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Unsafe))
                .count()
                != numeric[5]
            || reachable_total != Some(numeric[6])
            || transition_total != Some(numeric[7])
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant resource member totals disagree".to_string(),
            ));
        }
        Ok(ControllerPlantResourceSummary {
            policy_version,
            envelope_version,
            artifact_version,
            backend,
            members: member_count,
            maximum_member_horizon: numeric[1],
            maximum_product_states: numeric[2],
            transition_evaluation_bound: numeric[3],
            safe: numeric[4],
            unsafe_count: numeric[5],
            reachable_product_states: numeric[6],
            explored_transitions: numeric[7],
            artifact_bytes: numeric[8],
            load_micros: numeric[9],
            artifact_micros: numeric[10],
            verification_micros: numeric[11],
            elapsed_micros: numeric[12],
            member_results,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_proof_mtbdd_resource_summary(
    output: ManagedOutput,
    capabilities: &ControllerProofMtbddResourceCapabilities,
) -> Result<Observed<ControllerProofMtbddResourceSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerProofMtbddResourceSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD resource response is not canonical LF text".to_string(),
            ));
        }
        let mut lines = stdout.lines();
        let first = lines.next().ok_or_else(|| {
            PredicateApiError::InvalidResponse(
                "controller proof MTBDD resource summary line is missing".to_string(),
            )
        })?;
        let fields = first.split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-proof-mtbdd-resource",
            "status",
            "cli_version",
            "policy_version",
            "envelope_version",
            "artifact_version",
            "members",
            "maximum_member_horizon",
            "maximum_product_states",
            "transition_evaluation_bound",
            "equivalence_artifact_bytes",
            "unsat_proof_bytes",
            "safe",
            "unsafe",
            "reachable_product_states",
            "explored_transitions",
            "artifact_bytes",
            "assignments_checked",
            "load_micros",
            "artifact_micros",
            "verification_micros",
            "elapsed_micros",
        ];
        if fields.len() != keys.len()
            || fields[0] != keys[0]
            || token_value(fields[1], "status")? != "VERIFIED"
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD resource summary shape is invalid".to_string(),
            ));
        }
        let cli_version = canonical_u32(token_value(fields[2], keys[2])?, keys[2])?;
        let policy_version = canonical_u32(token_value(fields[3], keys[3])?, keys[3])?;
        let envelope_version = canonical_u32(token_value(fields[4], keys[4])?, keys[4])?;
        let artifact_version = canonical_u32(token_value(fields[5], keys[5])?, keys[5])?;
        if cli_version != capabilities.cli_version
            || policy_version != capabilities.policy_version
            || envelope_version != capabilities.envelope_version
            || artifact_version != capabilities.artifact_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller proof MTBDD resource response version changed".to_string(),
            ));
        }
        let numeric = fields[6..]
            .iter()
            .zip(&keys[6..])
            .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
            .collect::<Result<Vec<_>, _>>()?;
        let member_count = numeric[0];
        if member_count == 0
            || member_count > capabilities.max_members
            || numeric[1] > capabilities.max_horizon
            || numeric[2] > capabilities.max_product_states
            || numeric[4] > capabilities.max_equivalence_artifact_bytes
            || numeric[5] > capabilities.max_unsat_proof_bytes
            || numeric[10] > capabilities.max_artifact_bytes
            || numeric[11] != 0
            || numeric[6].checked_add(numeric[7]) != Some(member_count)
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD resource response exceeds discovered limits".to_string(),
            ));
        }
        let mut member_results = Vec::with_capacity(member_count);
        for expected_index in 0..member_count {
            let line = lines.next().ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller proof MTBDD resource member line is missing".to_string(),
                )
            })?;
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                "controller-proof-mtbdd-resource-member",
                "index",
                "answer",
                "horizon",
                "bad_frame",
                "trace_steps",
                "reachable_product_states",
                "explored_transitions",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD resource member shape is invalid".to_string(),
                ));
            }
            let index = canonical_usize(token_value(fields[1], "index")?, "index")?;
            if index != expected_index {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD resource member ordering changed".to_string(),
                ));
            }
            let answer = match token_value(fields[2], "answer")? {
                "SAFE" => ControllerMtbddAnswer::Safe,
                "UNSAFE" => ControllerMtbddAnswer::Unsafe,
                _ => {
                    return Err(PredicateApiError::InvalidResponse(
                        "controller proof MTBDD resource member answer is invalid".to_string(),
                    ));
                }
            };
            let horizon = canonical_usize(token_value(fields[3], "horizon")?, "horizon")?;
            let bad_frame = match token_value(fields[4], "bad_frame")? {
                "none" => None,
                value => Some(canonical_usize(value, "bad_frame")?),
            };
            let values = fields[5..]
                .iter()
                .zip(&keys[5..])
                .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
                .collect::<Result<Vec<_>, _>>()?;
            if horizon > capabilities.max_horizon
                || match (answer, bad_frame) {
                    (ControllerMtbddAnswer::Safe, None) => values[0] != 0,
                    (ControllerMtbddAnswer::Unsafe, Some(frame)) => {
                        frame > horizon || values[0] != frame.saturating_add(1)
                    }
                    _ => true,
                }
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD resource member result is inconsistent".to_string(),
                ));
            }
            member_results.push(ControllerMtbddMemberResult {
                index,
                answer,
                horizon,
                bad_frame,
                trace_steps: values[0],
                reachable_product_states: values[1],
                explored_transitions: values[2],
            });
        }
        let safe = member_results
            .iter()
            .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Safe))
            .count();
        let unsafe_count = member_results.len() - safe;
        let reachable = member_results.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.reachable_product_states)
        });
        let explored = member_results.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.explored_transitions)
        });
        if lines.next().is_some()
            || safe != numeric[6]
            || unsafe_count != numeric[7]
            || reachable != Some(numeric[8])
            || explored != Some(numeric[9])
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD resource member totals disagree".to_string(),
            ));
        }
        Ok(ControllerProofMtbddResourceSummary {
            policy_version,
            envelope_version,
            artifact_version,
            members: member_count,
            maximum_member_horizon: numeric[1],
            maximum_product_states: numeric[2],
            transition_evaluation_bound: numeric[3],
            equivalence_artifact_bytes: numeric[4],
            unsat_proof_bytes: numeric[5],
            safe: numeric[6],
            unsafe_count: numeric[7],
            reachable_product_states: numeric[8],
            explored_transitions: numeric[9],
            artifact_bytes: numeric[10],
            assignments_checked: numeric[11],
            load_micros: numeric[12],
            artifact_micros: numeric[13],
            verification_micros: numeric[14],
            elapsed_micros: numeric[15],
            member_results,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_proof_mtbdd_portfolio_resource_summary(
    output: ManagedOutput,
    capabilities: &ControllerProofMtbddPortfolioCapabilities,
    require_attestation: bool,
) -> Result<Observed<ControllerProofMtbddPortfolioResourceSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerProofMtbddPortfolioResourceSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD portfolio response is not canonical LF text".to_string(),
            ));
        }
        let mut lines = stdout.lines();
        let first = lines.next().ok_or_else(|| {
            PredicateApiError::InvalidResponse(
                "controller proof MTBDD portfolio summary line is missing".to_string(),
            )
        })?;
        let fields = first.split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-proof-mtbdd-portfolio-resource",
            "status",
            "cli_version",
            "policy_version",
            "envelope_version",
            "artifact_version",
            "backend",
            "reason",
            "members",
            "maximum_member_horizon",
            "maximum_product_states",
            "transition_evaluation_bound",
            "equivalence_artifact_bytes",
            "unsat_proof_bytes",
            "safe",
            "unsafe",
            "reachable_product_states",
            "explored_transitions",
            "artifact_bytes",
            "assignments_checked",
            "load_micros",
            "artifact_micros",
            "verification_micros",
            "elapsed_micros",
        ];
        let attested = fields.len() == keys.len() + 5;
        if attested != require_attestation
            || (!attested && fields.len() != keys.len())
            || fields[0] != keys[0]
            || token_value(fields[1], "status")? != "VERIFIED"
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD portfolio summary shape is invalid".to_string(),
            ));
        }
        if attested {
            let attestation_keys = [
                "source_model_attestation_version",
                "source_model_members",
                "source_model_tool",
                "source_model_tool_revision",
                "provenance",
            ];
            let attestation_fields = &fields[keys.len()..];
            let version = canonical_u32(
                token_value(attestation_fields[0], attestation_keys[0])?,
                attestation_keys[0],
            )?;
            let members = canonical_usize(
                token_value(attestation_fields[1], attestation_keys[1])?,
                attestation_keys[1],
            )?;
            let tool = token_value(attestation_fields[2], attestation_keys[2])?;
            let revision = token_value(attestation_fields[3], attestation_keys[3])?;
            let provenance = token_value(attestation_fields[4], attestation_keys[4])?;
            if version != capabilities.source_model_attestation_version
                || members == 0
                || members > capabilities.max_members.saturating_add(1)
                || tool != "yosys"
                || revision.len() != 40
                || !revision
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || provenance != "BOUND"
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio attestation response is invalid".to_string(),
                ));
            }
        }
        let cli_version = canonical_u32(token_value(fields[2], keys[2])?, keys[2])?;
        let policy_version = canonical_u32(token_value(fields[3], keys[3])?, keys[3])?;
        let envelope_version = canonical_u32(token_value(fields[4], keys[4])?, keys[4])?;
        let artifact_version = canonical_u32(token_value(fields[5], keys[5])?, keys[5])?;
        if cli_version != capabilities.cli_version
            || policy_version != capabilities.policy_version
            || envelope_version != capabilities.envelope_version
            || artifact_version != capabilities.artifact_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller proof MTBDD portfolio response version changed".to_string(),
            ));
        }
        let backend = match token_value(fields[6], keys[6])? {
            "PROOF_MTBDD" => ControllerPlantPortfolioBackend::Mtbdd,
            "DIRECT_EXACT" => ControllerPlantPortfolioBackend::DirectExact,
            _ => {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio backend is invalid".to_string(),
                ));
            }
        };
        let reason = match token_value(fields[7], keys[7])? {
            "MTBDD_ADMITTED" => ControllerPlantPortfolioReason::MtbddAdmitted,
            "BOUNDARY_LIMIT" => ControllerPlantPortfolioReason::BoundaryLimit,
            "TERMINAL_LIMIT" => ControllerPlantPortfolioReason::TerminalLimit,
            "NODE_LIMIT" => ControllerPlantPortfolioReason::NodeLimit,
            _ => {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio reason is invalid".to_string(),
                ));
            }
        };
        if matches!(backend, ControllerPlantPortfolioBackend::Mtbdd)
            != matches!(reason, ControllerPlantPortfolioReason::MtbddAdmitted)
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD portfolio route is inconsistent".to_string(),
            ));
        }
        let numeric = fields[8..keys.len()]
            .iter()
            .zip(&keys[8..])
            .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
            .collect::<Result<Vec<_>, _>>()?;
        let member_count = numeric[0];
        if member_count == 0
            || member_count > capabilities.max_members
            || numeric[1] > capabilities.max_horizon
            || numeric[2] > capabilities.max_product_states
            || numeric[4] > capabilities.max_equivalence_artifact_bytes
            || numeric[5] > capabilities.max_unsat_proof_bytes
            || numeric[10] > capabilities.max_artifact_bytes
            || numeric[11] != 0
            || numeric[6].checked_add(numeric[7]) != Some(member_count)
            || (matches!(backend, ControllerPlantPortfolioBackend::DirectExact)
                && (numeric[4] != 0 || numeric[5] != 0))
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD portfolio response exceeds discovered limits".to_string(),
            ));
        }
        let mut member_results = Vec::with_capacity(member_count);
        for expected_index in 0..member_count {
            let line = lines.next().ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio member line is missing".to_string(),
                )
            })?;
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                "controller-proof-mtbdd-portfolio-resource-member",
                "index",
                "answer",
                "horizon",
                "bad_frame",
                "trace_steps",
                "reachable_product_states",
                "explored_transitions",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio member shape is invalid".to_string(),
                ));
            }
            let index = canonical_usize(token_value(fields[1], keys[1])?, keys[1])?;
            if index != expected_index {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio member ordering changed".to_string(),
                ));
            }
            let answer = match token_value(fields[2], keys[2])? {
                "SAFE" => ControllerMtbddAnswer::Safe,
                "UNSAFE" => ControllerMtbddAnswer::Unsafe,
                _ => {
                    return Err(PredicateApiError::InvalidResponse(
                        "controller proof MTBDD portfolio member answer is invalid".to_string(),
                    ));
                }
            };
            let horizon = canonical_usize(token_value(fields[3], keys[3])?, keys[3])?;
            let bad_frame = match token_value(fields[4], keys[4])? {
                "none" => None,
                value => Some(canonical_usize(value, keys[4])?),
            };
            let values = fields[5..]
                .iter()
                .zip(&keys[5..])
                .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
                .collect::<Result<Vec<_>, _>>()?;
            if horizon > capabilities.max_horizon
                || match (answer, bad_frame) {
                    (ControllerMtbddAnswer::Safe, None) => values[0] != 0,
                    (ControllerMtbddAnswer::Unsafe, Some(frame)) => {
                        frame > horizon || values[0] != frame.saturating_add(1)
                    }
                    _ => true,
                }
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller proof MTBDD portfolio member result is inconsistent".to_string(),
                ));
            }
            member_results.push(ControllerMtbddMemberResult {
                index,
                answer,
                horizon,
                bad_frame,
                trace_steps: values[0],
                reachable_product_states: values[1],
                explored_transitions: values[2],
            });
        }
        let safe = member_results
            .iter()
            .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Safe))
            .count();
        let unsafe_count = member_results.len() - safe;
        let reachable = member_results.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.reachable_product_states)
        });
        let explored = member_results.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.explored_transitions)
        });
        if lines.next().is_some()
            || safe != numeric[6]
            || unsafe_count != numeric[7]
            || reachable != Some(numeric[8])
            || explored != Some(numeric[9])
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller proof MTBDD portfolio member totals disagree".to_string(),
            ));
        }
        Ok(ControllerProofMtbddPortfolioResourceSummary {
            policy_version,
            envelope_version,
            artifact_version,
            backend,
            reason,
            members: member_count,
            maximum_member_horizon: numeric[1],
            maximum_product_states: numeric[2],
            transition_evaluation_bound: numeric[3],
            equivalence_artifact_bytes: numeric[4],
            unsat_proof_bytes: numeric[5],
            safe: numeric[6],
            unsafe_count: numeric[7],
            reachable_product_states: numeric[8],
            explored_transitions: numeric[9],
            artifact_bytes: numeric[10],
            assignments_checked: numeric[11],
            load_micros: numeric[12],
            artifact_micros: numeric[13],
            verification_micros: numeric[14],
            elapsed_micros: numeric[15],
            member_results,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn canonical_node_list(nodes: &[u64]) -> Result<String, PredicateApiError> {
    if nodes.is_empty()
        || nodes
            .windows(2)
            .any(|pair| pair[0] == 0 || pair[0] >= pair[1])
        || nodes.last() == Some(&0)
    {
        return Err(PredicateApiError::InvalidPolicy(
            "revision impact output nodes must be nonempty, nonzero, unique, and strictly increasing"
                .to_string(),
        ));
    }
    Ok(nodes
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(","))
}

fn parse_revision_impact_capabilities(
    line: &str,
) -> Result<RevisionImpactCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "revision impact capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "revision_impact_cli_version",
        "impact_version",
        "query_manifest_version",
        "max_query_manifest_bytes",
        "max_input_bytes",
        "max_evidence_bytes",
        "max_bundle_bytes",
        "max_atoms",
        "max_combinations",
        "max_queries",
        "semantics",
        "work_schema",
        "query_schema",
        "routing",
        "fallback",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "revision impact capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[10..]
        != [
            "exact-counterfactual-v1",
            "verification-v1",
            "transition-semantic-set-v1",
            "none",
            "none",
            "fail-closed",
        ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "revision impact semantic or fallback contract is unsupported".to_string(),
        ));
    }
    let versions = values[..3]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[3..10]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 3]))
        .collect::<Result<Vec<_>, _>>()?;
    if versions != [2, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "revision impact version tuple is unsupported".to_string(),
        ));
    }
    let expected = [
        16 * 1024,
        64 * 1024 * 1024,
        16 * 1024 * 1024,
        64 * 1024 * 1024,
        revision_impact::MAX_IMPACT_ATOMS,
        revision_impact::MAX_IMPACT_COMBINATIONS,
        revision_impact::MAX_IMPACT_QUERIES,
    ];
    if limits != expected {
        return Err(PredicateApiError::IncompatibleContract(
            "revision impact limit tuple is unsupported".to_string(),
        ));
    }
    Ok(RevisionImpactCapabilities {
        cli_version: versions[0],
        impact_version: versions[1],
        query_manifest_version: versions[2],
        max_query_manifest_bytes: limits[0],
        max_input_bytes: limits[1],
        max_evidence_bytes: limits[2],
        max_bundle_bytes: limits[3],
        max_atoms: limits[4],
        max_combinations: limits[5],
        max_queries: limits[6],
    })
}

fn parse_revision_impact_summary(
    output: ManagedOutput,
    expected_status: &str,
    capabilities: &RevisionImpactCapabilities,
) -> Result<Observed<RevisionImpactProcessSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<RevisionImpactProcessSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "revision impact response is not canonical LF text".to_string(),
            ));
        }
        let lines = stdout.lines().collect::<Vec<_>>();
        let fields = lines
            .first()
            .ok_or_else(|| {
                PredicateApiError::InvalidResponse("revision impact response is empty".to_string())
            })?
            .split(' ')
            .collect::<Vec<_>>();
        if fields.len() != 19 || fields.first().copied() != Some("btor2-revision-impact") {
            return Err(PredicateApiError::InvalidResponse(
                "revision impact summary shape is invalid".to_string(),
            ));
        }
        let keys = [
            "status",
            "impact_version",
            "atoms",
            "queries",
            "combinations",
            "reusable_observations",
            "invalidated_observations",
            "minimal_invalidating_sets",
            "minimal_semantic_change_sets",
            "evidence_members",
            "certificate_bytes",
            "parsed_evidence_bytes",
            "semantic_replays",
            "component_validations",
            "composed_pair_checks",
            "final_transition_checks",
            "result_comparisons",
            "elapsed_micros",
        ];
        let values = fields[1..19]
            .iter()
            .zip(keys)
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != expected_status {
            return Err(PredicateApiError::InvalidResponse(
                "revision impact status differs from requested operation".to_string(),
            ));
        }
        let impact_version = canonical_u32(values[1], keys[1])?;
        let numeric = values[2..]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_usize(value, keys[index + 2]))
            .collect::<Result<Vec<_>, _>>()?;
        if impact_version != capabilities.impact_version
            || numeric[0] == 0
            || numeric[0] > capabilities.max_atoms
            || numeric[1] == 0
            || numeric[1] > capabilities.max_queries
            || numeric[2] != 1usize << numeric[0]
            || numeric[2] > capabilities.max_combinations
            || numeric[3].checked_add(numeric[4]) != Some(numeric[2] * numeric[1])
            || numeric[7] != numeric[2] * numeric[1]
            || numeric[8] == 0
            || numeric[8] > capabilities.max_bundle_bytes
            || numeric[9] == 0
            || numeric[9] > capabilities.max_bundle_bytes
            || numeric[10] != numeric[7]
            || numeric[11] != numeric[7] * 2
            || numeric[14] != numeric[7]
        {
            return Err(PredicateApiError::InvalidResponse(
                "revision impact summary violates advertised dimensions".to_string(),
            ));
        }
        if lines.len() != numeric[1] + numeric[6] + 1 {
            return Err(PredicateApiError::InvalidResponse(
                "revision impact query transition count is incomplete".to_string(),
            ));
        }
        let query_keys = [
            "index",
            "horizon",
            "bad_side",
            "bad_output",
            "old_result",
            "new_result",
        ];
        let mut transitions = Vec::with_capacity(numeric[1]);
        let mut previous_key = None;
        for (index, line) in lines[1..=numeric[1]].iter().enumerate() {
            let fields = line.split(' ').collect::<Vec<_>>();
            if fields.len() != 7 || fields.first().copied() != Some("btor2-revision-impact-query") {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact query transition shape is invalid".to_string(),
                ));
            }
            let values = fields[1..]
                .iter()
                .zip(query_keys)
                .map(|(field, key)| token_value(field, key))
                .collect::<Result<Vec<_>, _>>()?;
            if canonical_usize(values[0], query_keys[0])? != index {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact query transition index is noncanonical".to_string(),
                ));
            }
            let horizon = canonical_u32(values[1], query_keys[1])?;
            let (bad_side, side_key) = match values[2] {
                "left" => (revision_local::ComponentSide::Left, 0_u8),
                "right" => (revision_local::ComponentSide::Right, 1_u8),
                _ => {
                    return Err(PredicateApiError::InvalidResponse(
                        "revision impact query side is invalid".to_string(),
                    ));
                }
            };
            let bad_output = canonical_u64(values[3], query_keys[3])?;
            if bad_output == 0 {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact query output is zero".to_string(),
                ));
            }
            let key = (horizon, side_key, bad_output);
            if previous_key.is_some_and(|previous| previous >= key) {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact query transitions are not strictly ordered".to_string(),
                ));
            }
            previous_key = Some(key);
            let parse_result = |value| match value {
                "SAFE" => Ok(revision_local::BoundedResult::Safe),
                "UNSAFE" => Ok(revision_local::BoundedResult::Unsafe),
                _ => Err(PredicateApiError::InvalidResponse(
                    "revision impact query result is invalid".to_string(),
                )),
            };
            transitions.push(RevisionImpactQueryTransition {
                index,
                horizon,
                bad_side,
                bad_output,
                old_result: parse_result(values[4])?,
                new_result: parse_result(values[5])?,
            });
        }
        let semantic_keys = [
            "query_index",
            "changed_mask",
            "baseline_result",
            "changed_result",
        ];
        let parse_result = |value| match value {
            "SAFE" => Ok(revision_local::BoundedResult::Safe),
            "UNSAFE" => Ok(revision_local::BoundedResult::Unsafe),
            _ => Err(PredicateApiError::InvalidResponse(
                "revision impact semantic-set result is invalid".to_string(),
            )),
        };
        let mut semantic_change_sets = Vec::with_capacity(numeric[6]);
        let mut previous_semantic_key = None;
        for line in &lines[numeric[1] + 1..] {
            let fields = line.split(' ').collect::<Vec<_>>();
            if fields.len() != 5
                || fields.first().copied() != Some("btor2-revision-impact-semantic-set")
            {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact semantic-set shape is invalid".to_string(),
                ));
            }
            let values = fields[1..]
                .iter()
                .zip(semantic_keys)
                .map(|(field, key)| token_value(field, key))
                .collect::<Result<Vec<_>, _>>()?;
            let query_index = canonical_usize(values[0], semantic_keys[0])?;
            let changed_mask_usize = canonical_usize(values[1], semantic_keys[1])?;
            if query_index >= numeric[1]
                || changed_mask_usize == 0
                || changed_mask_usize >= numeric[2]
            {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact semantic-set index or mask is out of range".to_string(),
                ));
            }
            let key = (query_index, changed_mask_usize);
            if previous_semantic_key.is_some_and(|previous| previous >= key) {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact semantic sets are not strictly ordered".to_string(),
                ));
            }
            previous_semantic_key = Some(key);
            let baseline_result = parse_result(values[2])?;
            let changed_result = parse_result(values[3])?;
            if baseline_result == changed_result {
                return Err(PredicateApiError::InvalidResponse(
                    "revision impact semantic set does not change the result".to_string(),
                ));
            }
            semantic_change_sets.push(RevisionImpactSemanticChangeSet {
                query_index,
                changed_mask: u16::try_from(changed_mask_usize).map_err(|_| {
                    PredicateApiError::InvalidResponse(
                        "revision impact semantic-set mask exceeds u16".to_string(),
                    )
                })?,
                baseline_result,
                changed_result,
            });
        }
        Ok(RevisionImpactProcessSummary {
            impact_version,
            atoms: numeric[0],
            queries: numeric[1],
            combinations: numeric[2],
            reusable_observations: numeric[3],
            invalidated_observations: numeric[4],
            minimal_invalidating_sets: numeric[5],
            minimal_semantic_change_sets: numeric[6],
            evidence_members: numeric[7],
            certificate_bytes: numeric[8],
            parsed_evidence_bytes: numeric[9],
            semantic_replays: numeric[10],
            component_validations: numeric[11],
            composed_pair_checks: numeric[12],
            final_transition_checks: numeric[13],
            result_comparisons: numeric[14],
            elapsed_micros: numeric[15],
            transitions,
            semantic_change_sets,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_plant_portfolio_capabilities(
    line: &str,
) -> Result<ControllerPlantPortfolioCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller plant portfolio capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_plant_portfolio_cli_version",
        "artifact_version",
        "manifest_version",
        "max_manifest_bytes",
        "max_artifact_bytes",
        "max_members",
        "backends",
        "routing",
        "fallback",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller plant portfolio capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[6..] != ["mtbdd,direct-exact", "static", "exact", "fail-closed"] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller plant portfolio routing contract is unsupported".to_string(),
        ));
    }
    let versions = values[..3]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[3..6]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 3]))
        .collect::<Result<Vec<_>, _>>()?;
    if versions != [1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller plant portfolio version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) {
        return Err(PredicateApiError::InvalidResponse(
            "controller plant portfolio discovered limit must be positive".to_string(),
        ));
    }
    Ok(ControllerPlantPortfolioCapabilities {
        cli_version: versions[0],
        artifact_version: versions[1],
        manifest_version: versions[2],
        max_manifest_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_members: limits[2],
    })
}

fn parse_controller_plant_portfolio_summary(
    output: ManagedOutput,
    expected_action: &str,
    expected_output: Option<&Path>,
    capabilities: &ControllerPlantPortfolioCapabilities,
) -> Result<Observed<ControllerPlantPortfolioBatchSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerPlantPortfolioBatchSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant portfolio response is not canonical LF text".to_string(),
            ));
        }
        let mut lines = stdout.lines();
        let first = lines.next().ok_or_else(|| {
            PredicateApiError::InvalidResponse(
                "controller plant portfolio summary line is missing".to_string(),
            )
        })?;
        let summary_text = if let Some(expected_output) = expected_output {
            first
                .split_once(" output=")
                .and_then(|(summary, output)| {
                    (output == expected_output.to_string_lossy()).then_some(summary)
                })
                .ok_or_else(|| {
                    PredicateApiError::InvalidResponse(
                        "controller plant portfolio output path disagrees".to_string(),
                    )
                })?
        } else if first.contains(" output=") {
            return Err(PredicateApiError::InvalidResponse(
                "verified controller plant portfolio unexpectedly names output".to_string(),
            ));
        } else {
            first
        };
        let fields = summary_text.split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-plant-portfolio",
            "status",
            "cli_version",
            "artifact_version",
            "backend",
            "reason",
            "members",
            "safe",
            "unsafe",
            "reachable_product_states",
            "explored_transitions",
            "artifact_bytes",
            "load_micros",
            "artifact_micros",
            "verification_micros",
            "write_micros",
            "elapsed_micros",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant portfolio summary shape is invalid".to_string(),
            ));
        }
        if token_value(fields[1], "status")? != expected_action {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant portfolio action disagrees".to_string(),
            ));
        }
        let cli_version = canonical_u32(token_value(fields[2], "cli_version")?, "cli_version")?;
        let artifact_version = canonical_u32(
            token_value(fields[3], "artifact_version")?,
            "artifact_version",
        )?;
        if cli_version != capabilities.cli_version
            || artifact_version != capabilities.artifact_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller plant portfolio response version changed".to_string(),
            ));
        }
        let backend = match token_value(fields[4], "backend")? {
            "MTBDD" => ControllerPlantPortfolioBackend::Mtbdd,
            "DIRECT_EXACT" => ControllerPlantPortfolioBackend::DirectExact,
            _ => {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant portfolio backend is invalid".to_string(),
                ));
            }
        };
        let reason = match token_value(fields[5], "reason")? {
            "mtbdd-admitted" => ControllerPlantPortfolioReason::MtbddAdmitted,
            "boundary-limit" => ControllerPlantPortfolioReason::BoundaryLimit,
            "terminal-limit" => ControllerPlantPortfolioReason::TerminalLimit,
            "node-limit" => ControllerPlantPortfolioReason::NodeLimit,
            _ => {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant portfolio reason is invalid".to_string(),
                ));
            }
        };
        if !matches!(
            (backend, reason),
            (
                ControllerPlantPortfolioBackend::Mtbdd,
                ControllerPlantPortfolioReason::MtbddAdmitted
            ) | (
                ControllerPlantPortfolioBackend::DirectExact,
                ControllerPlantPortfolioReason::BoundaryLimit
                    | ControllerPlantPortfolioReason::TerminalLimit
                    | ControllerPlantPortfolioReason::NodeLimit
            )
        ) {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant portfolio route and reason disagree".to_string(),
            ));
        }
        let numeric = fields[6..]
            .iter()
            .zip(&keys[6..])
            .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
            .collect::<Result<Vec<_>, _>>()?;
        let member_count = numeric[0];
        if member_count == 0
            || member_count > capabilities.max_members
            || numeric[5] > capabilities.max_artifact_bytes
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant portfolio response exceeds discovered limits".to_string(),
            ));
        }
        let mut members = Vec::with_capacity(member_count);
        for expected_index in 0..member_count {
            let line = lines.next().ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller plant portfolio member line is missing".to_string(),
                )
            })?;
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                "controller-plant-portfolio-member",
                "index",
                "answer",
                "horizon",
                "bad_frame",
                "trace_steps",
                "reachable_product_states",
                "explored_transitions",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant portfolio member shape is invalid".to_string(),
                ));
            }
            let index = canonical_usize(token_value(fields[1], "index")?, "index")?;
            if index != expected_index {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant portfolio member order is invalid".to_string(),
                ));
            }
            let answer = match token_value(fields[2], "answer")? {
                "SAFE" => ControllerMtbddAnswer::Safe,
                "UNSAFE" => ControllerMtbddAnswer::Unsafe,
                _ => {
                    return Err(PredicateApiError::InvalidResponse(
                        "controller plant portfolio member answer is invalid".to_string(),
                    ));
                }
            };
            let horizon = canonical_usize(token_value(fields[3], "horizon")?, "horizon")?;
            let bad = token_value(fields[4], "bad_frame")?;
            let bad_frame = if bad == "none" {
                None
            } else {
                Some(canonical_usize(bad, "bad_frame")?)
            };
            let trace_steps =
                canonical_usize(token_value(fields[5], "trace_steps")?, "trace_steps")?;
            if horizon > crate::controller_plant::MAX_COMPOSITION_HORIZON
                || match (answer, bad_frame) {
                    (ControllerMtbddAnswer::Safe, None) => trace_steps != 0,
                    (ControllerMtbddAnswer::Unsafe, Some(frame)) => {
                        frame > horizon || trace_steps != frame.saturating_add(1)
                    }
                    _ => true,
                }
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller plant portfolio member trace boundary is invalid".to_string(),
                ));
            }
            members.push(ControllerMtbddMemberResult {
                index,
                answer,
                horizon,
                bad_frame,
                trace_steps,
                reachable_product_states: canonical_usize(
                    token_value(fields[6], "reachable_product_states")?,
                    "reachable_product_states",
                )?,
                explored_transitions: canonical_usize(
                    token_value(fields[7], "explored_transitions")?,
                    "explored_transitions",
                )?,
            });
        }
        let reachable_total = members.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.reachable_product_states)
        });
        let transition_total = members.iter().try_fold(0usize, |total, member| {
            total.checked_add(member.explored_transitions)
        });
        if lines.next().is_some()
            || members
                .iter()
                .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Safe))
                .count()
                != numeric[1]
            || members
                .iter()
                .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Unsafe))
                .count()
                != numeric[2]
            || reachable_total != Some(numeric[3])
            || transition_total != Some(numeric[4])
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller plant portfolio member totals disagree".to_string(),
            ));
        }
        Ok(ControllerPlantPortfolioBatchSummary {
            artifact_version,
            backend,
            reason,
            safe: numeric[1],
            unsafe_count: numeric[2],
            reachable_product_states: numeric[3],
            explored_transitions: numeric[4],
            artifact_bytes: numeric[5],
            load_micros: numeric[6],
            artifact_micros: numeric[7],
            verification_micros: numeric[8],
            write_micros: numeric[9],
            elapsed_micros: numeric[10],
            members,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_split_evidence_capabilities(
    line: &str,
) -> Result<ControllerSplitEvidenceCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller split-evidence capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_split_evidence_cli_version",
        "controller_artifact_version",
        "plant_artifact_version",
        "manifest_version",
        "max_manifest_bytes",
        "max_artifact_bytes",
        "max_batches",
        "admission",
        "verification",
        "exhaustive_replay",
        "source_binding",
        "obligation_binding",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller split-evidence capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[7..]
        != [
            "once",
            "unsat-miter",
            "no",
            "sha256",
            "complete-ordered",
            "fail-closed",
        ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split-evidence trust contract is unsupported".to_string(),
        ));
    }
    let versions = values[..4]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[4..7]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 4]))
        .collect::<Result<Vec<_>, _>>()?;
    if versions != [1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split-evidence version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) {
        return Err(PredicateApiError::InvalidResponse(
            "controller split-evidence discovered limit must be positive".to_string(),
        ));
    }
    if limits[0] > 64 * 1024
        || limits[1] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_BYTES
        || limits[2] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split-evidence limits exceed client safety ceilings".to_string(),
        ));
    }
    Ok(ControllerSplitEvidenceCapabilities {
        cli_version: versions[0],
        controller_artifact_version: versions[1],
        plant_artifact_version: versions[2],
        manifest_version: versions[3],
        max_manifest_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_batches: limits[2],
    })
}

fn parse_controller_split_resource_capabilities(
    line: &str,
) -> Result<ControllerSplitResourceCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller split resource capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_split_resource_cli_version",
        "policy_version",
        "controller_envelope_version",
        "plant_envelope_version",
        "controller_artifact_version",
        "plant_artifact_version",
        "manifest_version",
        "max_policy_bytes",
        "max_controller_artifact_bytes",
        "max_unsat_proof_bytes",
        "max_plant_artifact_bytes",
        "max_batches",
        "max_members_per_batch",
        "max_horizon",
        "max_product_states",
        "refusal_exit",
        "admission",
        "verification",
        "exhaustive_replay",
        "accounting",
        "timing_calibration",
        "result_on_refusal",
        "refusal_schema",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller split resource capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[16..]
        != [
            "once",
            "unsat-miter",
            "no",
            "conservative-static-per-batch-and-total",
            "none",
            "none",
            "split-reason-v1",
            "fail-closed",
        ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split resource trust contract is unsupported".to_string(),
        ));
    }
    let versions = values[..7]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[7..15]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 7]))
        .collect::<Result<Vec<_>, _>>()?;
    let refusal_exit = canonical_usize(values[15], keys[15])?;
    if versions != [1, 1, 1, 1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split resource version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) || refusal_exit != 3 {
        return Err(PredicateApiError::InvalidResponse(
            "controller split resource discovered limits are invalid".to_string(),
        ));
    }
    if limits[0] > 4096
        || limits[1] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_BYTES
        || limits[2] > unsat_proof::MAX_UNSAT_PROOF_BYTES
        || limits[3] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_BYTES
        || limits[4] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
        || limits[5] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
        || limits[6] > controller_plant::MAX_COMPOSITION_HORIZON
        || limits[7] > controller_plant::MAX_PRODUCT_STATES
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split resource limits exceed client safety ceilings".to_string(),
        ));
    }
    Ok(ControllerSplitResourceCapabilities {
        cli_version: versions[0],
        policy_version: versions[1],
        controller_envelope_version: versions[2],
        plant_envelope_version: versions[3],
        controller_artifact_version: versions[4],
        plant_artifact_version: versions[5],
        manifest_version: versions[6],
        max_policy_bytes: limits[0],
        max_controller_artifact_bytes: limits[1],
        max_unsat_proof_bytes: limits[2],
        max_plant_artifact_bytes: limits[3],
        max_batches: limits[4],
        max_members_per_batch: limits[5],
        max_horizon: limits[6],
        max_product_states: limits[7],
        refusal_exit_code: 3,
    })
}

fn parse_controller_split_observability_capabilities(
    text: &str,
) -> Result<ControllerSplitObservabilityCapabilities, PredicateApiError> {
    if text.contains('\r') || !text.ends_with('\n') || text.lines().count() != 2 {
        return Err(PredicateApiError::InvalidResponse(
            "controller split observability capability response is not canonical".to_string(),
        ));
    }
    let lines = text.lines().collect::<Vec<_>>();
    let resource = parse_controller_split_resource_capabilities(&format!("{}\n", lines[0]))?;
    let fields = lines[1].split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_split_observability_cli_version",
        "base_cli_version",
        "phase_metrics_version",
        "phases",
        "counters",
        "timing_calibration",
        "partial_metrics_on_failure",
        "result_on_refusal",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller split observability capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    let cli_version = canonical_u32(values[0], keys[0])?;
    let base_cli_version = canonical_u32(values[1], keys[1])?;
    let phase_metrics_version = canonical_u32(values[2], keys[2])?;
    if cli_version != 1
        || base_cli_version != resource.cli_version
        || phase_metrics_version != 1
        || values[3]
            != "policy-and-input,controller-admission,complete-set-preflight,semantic-replay"
        || values[4]
            != "controller-admissions,manifest-loads,plant-artifact-reads,resource-assessments,batch-verifications,buffered-result-rows,prepared-batches,prepared-members,controller-evidence-bytes,total-plant-artifact-bytes,total-transition-evaluation-bound"
        || values[5..] != ["none", "none", "none", "fail-closed"]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split observability contract is unsupported".to_string(),
        ));
    }
    Ok(ControllerSplitObservabilityCapabilities {
        cli_version,
        phase_metrics_version,
        resource,
    })
}

fn parse_controller_split_allocation_observability_capabilities(
    text: &str,
) -> Result<ControllerSplitAllocationObservabilityCapabilities, PredicateApiError> {
    if text.contains('\r') || !text.ends_with('\n') || text.lines().count() != 3 {
        return Err(PredicateApiError::InvalidResponse(
            "controller split allocation capability response is not canonical".to_string(),
        ));
    }
    let lines = text.lines().collect::<Vec<_>>();
    let observability = parse_controller_split_observability_capabilities(&format!(
        "{}\n{}\n",
        lines[0], lines[1]
    ))?;
    let fields = lines[2].split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_split_allocation_observability_cli_version",
        "base_observability_cli_version",
        "allocator",
        "scope",
        "counters",
        "overflow",
        "timing_calibration",
        "partial_metrics_on_failure",
        "result_on_refusal",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller split allocation capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    let cli_version = canonical_u32(values[0], keys[0])?;
    let base_version = canonical_u32(values[1], keys[1])?;
    if cli_version != 1
        || base_version != observability.cli_version
        || values[2] != "system"
        || values[3] != "policy-through-replay"
        || values[4]
            != "allocation-calls,allocated-bytes,deallocation-calls,deallocated-bytes,reallocation-calls,reallocated-bytes"
        || values[5..] != ["fail-closed", "none", "none", "none", "fail-closed"]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split allocation observability contract is unsupported".to_string(),
        ));
    }
    Ok(ControllerSplitAllocationObservabilityCapabilities {
        cli_version,
        observability,
    })
}

fn parse_controller_split_cache_observability_capabilities(
    text: &str,
) -> Result<ControllerSplitCacheObservabilityCapabilities, PredicateApiError> {
    if text.contains('\r') || !text.ends_with('\n') || text.lines().count() != 4 {
        return Err(PredicateApiError::InvalidResponse(
            "controller split cache capability response is not canonical".to_string(),
        ));
    }
    let lines = text.lines().collect::<Vec<_>>();
    let allocation_observability = parse_controller_split_allocation_observability_capabilities(
        &format!("{}\n{}\n{}\n", lines[0], lines[1], lines[2]),
    )?;
    let fields = lines[3].split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_split_cache_observability_cli_version",
        "base_allocation_observability_cli_version",
        "scope",
        "key",
        "counters",
        "integrity_preflight",
        "overflow",
        "timing_calibration",
        "partial_metrics_on_failure",
        "result_on_refusal",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller split cache capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    let cli_version = canonical_u32(values[0], keys[0])?;
    let base_version = canonical_u32(values[1], keys[1])?;
    if cli_version != 1
        || base_version != allocation_observability.cli_version
        || values[2] != "semantic-replay"
        || values[3] != "manifest-snapshot,resource-assessment,result-sha256"
        || values[4] != "lookups,hits,misses,entries"
        || values[5..]
            != [
                "required",
                "fail-closed",
                "none",
                "none",
                "none",
                "fail-closed",
            ]
    {
        return Err(PredicateApiError::IncompatibleContract(
            "controller split cache observability contract is unsupported".to_string(),
        ));
    }
    Ok(ControllerSplitCacheObservabilityCapabilities {
        cli_version,
        allocation_observability,
    })
}

fn parse_controller_mtbdd_capabilities(
    line: &str,
) -> Result<ControllerMtbddCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "controller MTBDD capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_mtbdd_cli_version",
        "mtbdd_version",
        "plant_artifact_version",
        "manifest_version",
        "max_manifest_bytes",
        "max_artifact_bytes",
        "max_members",
        "max_state_bits",
        "max_inputs",
        "max_outputs",
        "max_nodes",
        "max_terminals",
        "max_assignments",
        "max_horizon",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "controller MTBDD capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[14] != "fail-closed" {
        return Err(PredicateApiError::IncompatibleContract(
            "controller MTBDD unsupported-input policy is not fail-closed".to_string(),
        ));
    }
    let versions = values[..4]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = values[4..14]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 4]))
        .collect::<Result<Vec<_>, _>>()?;
    if versions != [1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "controller MTBDD version tuple is unsupported".to_string(),
        ));
    }
    if limits.contains(&0) {
        return Err(PredicateApiError::InvalidResponse(
            "controller MTBDD discovered limit must be positive".to_string(),
        ));
    }
    let capabilities = ControllerMtbddCapabilities {
        cli_version: versions[0],
        mtbdd_version: versions[1],
        plant_artifact_version: versions[2],
        manifest_version: versions[3],
        max_manifest_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_members: limits[2],
        max_state_bits: limits[3],
        max_inputs: limits[4],
        max_outputs: limits[5],
        max_nodes: limits[6],
        max_terminals: limits[7],
        max_assignments: limits[8],
        max_horizon: limits[9],
    };
    Ok(capabilities)
}

fn parse_controller_proof_mtbdd_capabilities(
    line: &str,
) -> Result<ControllerProofMtbddCapabilities, PredicateApiError> {
    if line.contains('\r') || !line.ends_with('\n') || line.lines().count() != 1 {
        return Err(PredicateApiError::InvalidResponse(
            "proof-carrying controller MTBDD capability line is not canonical".to_string(),
        ));
    }
    let fields = line.trim_end_matches('\n').split(' ').collect::<Vec<_>>();
    let keys = [
        "controller_proof_mtbdd_cli_version",
        "mtbdd_version",
        "equivalence_proof_version",
        "plant_artifact_version",
        "manifest_version",
        "max_manifest_bytes",
        "max_artifact_bytes",
        "max_equivalence_artifact_bytes",
        "max_unsat_proof_bytes",
        "max_members",
        "max_state_bits",
        "max_inputs",
        "max_outputs",
        "max_nodes",
        "max_terminals",
        "max_horizon",
        "verification",
        "exhaustive_replay",
        "unsupported",
    ];
    if fields.len() != keys.len() {
        return Err(PredicateApiError::InvalidResponse(
            "proof-carrying controller MTBDD capability field count is invalid".to_string(),
        ));
    }
    let values = fields
        .iter()
        .zip(keys)
        .map(|(field, key)| token_value(field, key))
        .collect::<Result<Vec<_>, _>>()?;
    if values[16] != "unsat-miter" || values[17] != "no" || values[18] != "fail-closed" {
        return Err(PredicateApiError::IncompatibleContract(
            "proof-carrying controller MTBDD verification contract changed".to_string(),
        ));
    }
    let versions = values[..5]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_u32(value, keys[index]))
        .collect::<Result<Vec<_>, _>>()?;
    if versions != [1, 1, 1, 1, 1] {
        return Err(PredicateApiError::IncompatibleContract(
            "proof-carrying controller MTBDD version tuple is unsupported".to_string(),
        ));
    }
    let limits = values[5..16]
        .iter()
        .enumerate()
        .map(|(index, value)| canonical_usize(value, keys[index + 5]))
        .collect::<Result<Vec<_>, _>>()?;
    if limits.contains(&0) {
        return Err(PredicateApiError::InvalidResponse(
            "proof-carrying controller MTBDD discovered limit must be positive".to_string(),
        ));
    }
    Ok(ControllerProofMtbddCapabilities {
        cli_version: versions[0],
        mtbdd_version: versions[1],
        equivalence_proof_version: versions[2],
        plant_artifact_version: versions[3],
        manifest_version: versions[4],
        max_manifest_bytes: limits[0],
        max_artifact_bytes: limits[1],
        max_equivalence_artifact_bytes: limits[2],
        max_unsat_proof_bytes: limits[3],
        max_members: limits[4],
        max_state_bits: limits[5],
        max_inputs: limits[6],
        max_outputs: limits[7],
        max_nodes: limits[8],
        max_terminals: limits[9],
        max_horizon: limits[10],
    })
}

fn parse_controller_split_artifact_summary(
    output: ManagedOutput,
    prefix: &str,
    has_members: bool,
    expected_output: &Path,
    capabilities: &ControllerSplitEvidenceCapabilities,
) -> Result<Observed<ControllerSplitArtifactSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerSplitArtifactSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') || stdout.lines().count() != 1 {
            return Err(PredicateApiError::InvalidResponse(
                "controller split artifact response is not canonical".to_string(),
            ));
        }
        let line = stdout.trim_end_matches('\n');
        let (summary, output_path) = line.rsplit_once(" output=").ok_or_else(|| {
            PredicateApiError::InvalidResponse(
                "controller split artifact output field is missing".to_string(),
            )
        })?;
        if output_path != expected_output.to_string_lossy() {
            return Err(PredicateApiError::InvalidResponse(
                "controller split artifact output disagrees".to_string(),
            ));
        }
        let fields = summary.split(' ').collect::<Vec<_>>();
        let keys = if has_members {
            vec![
                prefix,
                "status",
                "cli_version",
                "artifact_version",
                "members",
                "artifact_bytes",
                "elapsed_micros",
            ]
        } else {
            vec![
                prefix,
                "status",
                "cli_version",
                "artifact_version",
                "mtbdd_nodes",
                "mtbdd_terminals",
                "artifact_bytes",
                "elapsed_micros",
            ]
        };
        if fields.len() != keys.len() || fields[0] != prefix {
            return Err(PredicateApiError::InvalidResponse(
                "controller split artifact field count is invalid".to_string(),
            ));
        }
        let values = fields[1..]
            .iter()
            .zip(&keys[1..])
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != "CREATED"
            || canonical_u32(values[1], "cli_version")? != capabilities.cli_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split artifact creation contract changed".to_string(),
            ));
        }
        let artifact_version = canonical_u32(values[2], "artifact_version")?;
        let expected_version = if has_members {
            capabilities.plant_artifact_version
        } else {
            capabilities.controller_artifact_version
        };
        if artifact_version != expected_version {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split artifact version changed".to_string(),
            ));
        }
        let members = has_members
            .then(|| canonical_usize(values[3], "members"))
            .transpose()?;
        if matches!(members, Some(0))
            || members.is_some_and(|count| {
                count > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
            })
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split artifact member count is zero".to_string(),
            ));
        }
        let mtbdd_nodes = (!has_members)
            .then(|| canonical_usize(values[3], "mtbdd_nodes"))
            .transpose()?;
        let mtbdd_terminals = (!has_members)
            .then(|| canonical_usize(values[4], "mtbdd_terminals"))
            .transpose()?;
        if matches!(mtbdd_nodes, Some(0))
            || mtbdd_nodes.is_some_and(|count| count > controller_mtbdd::MAX_MTBDD_NODES)
            || matches!(mtbdd_terminals, Some(0))
            || mtbdd_terminals.is_some_and(|count| count > controller_mtbdd::MAX_MTBDD_TERMINALS)
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split evidence MTBDD dimensions are outside limits".to_string(),
            ));
        }
        let offset = if has_members { 4 } else { 5 };
        let artifact_bytes = canonical_usize(values[offset], "artifact_bytes")?;
        let elapsed_micros = canonical_usize(values[offset + 1], "elapsed_micros")?;
        if artifact_bytes == 0 || artifact_bytes > capabilities.max_artifact_bytes {
            return Err(PredicateApiError::InvalidResponse(
                "controller split artifact byte count is outside limits".to_string(),
            ));
        }
        Ok(ControllerSplitArtifactSummary {
            artifact_version,
            members,
            mtbdd_nodes,
            mtbdd_terminals,
            artifact_bytes,
            elapsed_micros,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_split_set_summary(
    output: ManagedOutput,
    expected_batches: usize,
    capabilities: &ControllerSplitEvidenceCapabilities,
) -> Result<Observed<ControllerSplitSetSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerSplitSetSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller split set response is not canonical LF text".to_string(),
            ));
        }
        let lines = stdout.lines().collect::<Vec<_>>();
        if lines.len() != expected_batches + 1 {
            return Err(PredicateApiError::InvalidResponse(
                "controller split set response line count is invalid".to_string(),
            ));
        }
        let mut batches = Vec::with_capacity(expected_batches);
        for (expected_index, line) in lines[..expected_batches].iter().enumerate() {
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                "controller-split-batch",
                "index",
                "status",
                "members",
                "safe",
                "unsafe",
                "reachable_product_states",
                "explored_transitions",
                "artifact_bytes",
                "verification_micros",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller split batch fields are invalid".to_string(),
                ));
            }
            let values = fields[1..]
                .iter()
                .zip(&keys[1..])
                .map(|(field, key)| token_value(field, key))
                .collect::<Result<Vec<_>, _>>()?;
            let index = canonical_usize(values[0], "index")?;
            if index != expected_index || values[1] != "VERIFIED" {
                return Err(PredicateApiError::InvalidResponse(
                    "controller split batch order or status is invalid".to_string(),
                ));
            }
            let numbers = values[2..]
                .iter()
                .enumerate()
                .map(|(index, value)| canonical_usize(value, keys[index + 3]))
                .collect::<Result<Vec<_>, _>>()?;
            if numbers[0] == 0
                || numbers[0] > controller_plant_artifact::MAX_CONTROLLER_PLANT_ARTIFACT_MEMBERS
                || numbers[1].checked_add(numbers[2]) != Some(numbers[0])
                || numbers[5] == 0
                || numbers[5] > capabilities.max_artifact_bytes
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller split batch dimensions are invalid".to_string(),
                ));
            }
            batches.push(ControllerSplitBatchSummary {
                index,
                members: numbers[0],
                safe: numbers[1],
                unsafe_count: numbers[2],
                reachable_product_states: numbers[3],
                explored_transitions: numbers[4],
                artifact_bytes: numbers[5],
                verification_micros: numbers[6],
            });
        }
        let fields = lines[expected_batches].split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-split-set",
            "status",
            "cli_version",
            "controller_admissions",
            "batches",
            "members",
            "safe",
            "unsafe",
            "reachable_product_states",
            "explored_transitions",
            "controller_evidence_bytes",
            "admission_micros",
            "elapsed_micros",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller split aggregate fields are invalid".to_string(),
            ));
        }
        let values = fields[1..]
            .iter()
            .zip(&keys[1..])
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != "VERIFIED"
            || canonical_u32(values[1], "cli_version")? != capabilities.cli_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split aggregate contract changed".to_string(),
            ));
        }
        let numbers = values[2..]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_usize(value, keys[index + 3]))
            .collect::<Result<Vec<_>, _>>()?;
        let summary = ControllerSplitSetSummary {
            controller_admissions: numbers[0],
            members: numbers[2],
            safe: numbers[3],
            unsafe_count: numbers[4],
            reachable_product_states: numbers[5],
            explored_transitions: numbers[6],
            controller_evidence_bytes: numbers[7],
            admission_micros: numbers[8],
            elapsed_micros: numbers[9],
            batches,
        };
        let batch_members = summary
            .batches
            .iter()
            .try_fold(0usize, |total, batch| total.checked_add(batch.members));
        let batch_safe = summary
            .batches
            .iter()
            .try_fold(0usize, |total, batch| total.checked_add(batch.safe));
        let batch_unsafe = summary
            .batches
            .iter()
            .try_fold(0usize, |total, batch| total.checked_add(batch.unsafe_count));
        let batch_reachable = summary.batches.iter().try_fold(0usize, |total, batch| {
            total.checked_add(batch.reachable_product_states)
        });
        let batch_transitions = summary.batches.iter().try_fold(0usize, |total, batch| {
            total.checked_add(batch.explored_transitions)
        });
        if summary.controller_admissions != 1
            || numbers[1] != expected_batches
            || Some(summary.members) != batch_members
            || Some(summary.safe) != batch_safe
            || Some(summary.unsafe_count) != batch_unsafe
            || Some(summary.reachable_product_states) != batch_reachable
            || Some(summary.explored_transitions) != batch_transitions
            || summary.controller_evidence_bytes == 0
            || summary.controller_evidence_bytes > capabilities.max_artifact_bytes
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split aggregate does not reconcile".to_string(),
            ));
        }
        Ok(summary)
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_split_resource_set_summary(
    output: ManagedOutput,
    expected_batches: usize,
    capabilities: &ControllerSplitResourceCapabilities,
) -> Result<Observed<ControllerSplitResourceSetSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerSplitResourceSetSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller split resource response is not canonical LF text".to_string(),
            ));
        }
        let lines = stdout.lines().collect::<Vec<_>>();
        if lines.len() != expected_batches + 1 {
            return Err(PredicateApiError::InvalidResponse(
                "controller split resource response line count is invalid".to_string(),
            ));
        }
        let mut batches = Vec::with_capacity(expected_batches);
        for (expected_index, line) in lines[..expected_batches].iter().enumerate() {
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                "controller-split-resource-batch",
                "index",
                "status",
                "policy_version",
                "envelope_version",
                "artifact_version",
                "members",
                "maximum_member_horizon",
                "maximum_product_states",
                "transition_evaluation_bound",
                "safe",
                "unsafe",
                "reachable_product_states",
                "explored_transitions",
                "artifact_bytes",
                "verification_micros",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller split resource batch fields are invalid".to_string(),
                ));
            }
            let values = fields[1..]
                .iter()
                .zip(&keys[1..])
                .map(|(field, key)| token_value(field, key))
                .collect::<Result<Vec<_>, _>>()?;
            let index = canonical_usize(values[0], "index")?;
            if index != expected_index || values[1] != "VERIFIED" {
                return Err(PredicateApiError::InvalidResponse(
                    "controller split resource batch order or status is invalid".to_string(),
                ));
            }
            let policy_version = canonical_u32(values[2], "policy_version")?;
            let envelope_version = canonical_u32(values[3], "envelope_version")?;
            let artifact_version = canonical_u32(values[4], "artifact_version")?;
            if policy_version != capabilities.policy_version
                || envelope_version != capabilities.plant_envelope_version
                || artifact_version != capabilities.plant_artifact_version
            {
                return Err(PredicateApiError::IncompatibleContract(
                    "controller split resource batch versions changed".to_string(),
                ));
            }
            let numbers = values[5..]
                .iter()
                .enumerate()
                .map(|(index, value)| canonical_usize(value, keys[index + 6]))
                .collect::<Result<Vec<_>, _>>()?;
            if numbers[0] == 0
                || numbers[0] > capabilities.max_members_per_batch
                || numbers[1] > capabilities.max_horizon
                || numbers[2] > capabilities.max_product_states
                || numbers[4].checked_add(numbers[5]) != Some(numbers[0])
                || numbers[8] == 0
                || numbers[8] > capabilities.max_plant_artifact_bytes
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller split resource batch dimensions are invalid".to_string(),
                ));
            }
            batches.push(ControllerSplitResourceBatchSummary {
                index,
                members: numbers[0],
                maximum_member_horizon: numbers[1],
                maximum_product_states: numbers[2],
                transition_evaluation_bound: numbers[3],
                safe: numbers[4],
                unsafe_count: numbers[5],
                reachable_product_states: numbers[6],
                explored_transitions: numbers[7],
                artifact_bytes: numbers[8],
                verification_micros: numbers[9],
            });
        }
        let fields = lines[expected_batches].split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-split-resource-set",
            "status",
            "cli_version",
            "policy_version",
            "controller_envelope_version",
            "plant_envelope_version",
            "controller_admissions",
            "batches",
            "members",
            "safe",
            "unsafe",
            "reachable_product_states",
            "explored_transitions",
            "controller_evidence_bytes",
            "controller_mtbdd_bytes",
            "equivalence_artifact_bytes",
            "unsat_proof_bytes",
            "total_plant_artifact_bytes",
            "total_transition_evaluation_bound",
            "admission_micros",
            "elapsed_micros",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller split resource aggregate fields are invalid".to_string(),
            ));
        }
        let values = fields[1..]
            .iter()
            .zip(&keys[1..])
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != "VERIFIED"
            || canonical_u32(values[1], keys[2])? != capabilities.cli_version
            || canonical_u32(values[2], keys[3])? != capabilities.policy_version
            || canonical_u32(values[3], keys[4])? != capabilities.controller_envelope_version
            || canonical_u32(values[4], keys[5])? != capabilities.plant_envelope_version
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split resource aggregate contract changed".to_string(),
            ));
        }
        let numbers = values[5..]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_usize(value, keys[index + 6]))
            .collect::<Result<Vec<_>, _>>()?;
        let summary = ControllerSplitResourceSetSummary {
            controller_admissions: numbers[0],
            members: numbers[2],
            safe: numbers[3],
            unsafe_count: numbers[4],
            reachable_product_states: numbers[5],
            explored_transitions: numbers[6],
            controller_evidence_bytes: numbers[7],
            controller_mtbdd_bytes: numbers[8],
            equivalence_artifact_bytes: numbers[9],
            unsat_proof_bytes: numbers[10],
            total_plant_artifact_bytes: numbers[11],
            total_transition_evaluation_bound: numbers[12],
            admission_micros: numbers[13],
            elapsed_micros: numbers[14],
            batches,
        };
        let batch_members = summary
            .batches
            .iter()
            .try_fold(0usize, |total, batch| total.checked_add(batch.members));
        let batch_safe = summary
            .batches
            .iter()
            .try_fold(0usize, |total, batch| total.checked_add(batch.safe));
        let batch_unsafe = summary
            .batches
            .iter()
            .try_fold(0usize, |total, batch| total.checked_add(batch.unsafe_count));
        let batch_reachable = summary.batches.iter().try_fold(0usize, |total, batch| {
            total.checked_add(batch.reachable_product_states)
        });
        let batch_explored = summary.batches.iter().try_fold(0usize, |total, batch| {
            total.checked_add(batch.explored_transitions)
        });
        let batch_artifact_bytes = summary.batches.iter().try_fold(0usize, |total, batch| {
            total.checked_add(batch.artifact_bytes)
        });
        let batch_transition_bound = summary.batches.iter().try_fold(0usize, |total, batch| {
            total.checked_add(batch.transition_evaluation_bound)
        });
        if summary.controller_admissions != 1
            || numbers[1] != expected_batches
            || Some(summary.members) != batch_members
            || Some(summary.safe) != batch_safe
            || Some(summary.unsafe_count) != batch_unsafe
            || Some(summary.reachable_product_states) != batch_reachable
            || Some(summary.explored_transitions) != batch_explored
            || Some(summary.total_plant_artifact_bytes) != batch_artifact_bytes
            || Some(summary.total_transition_evaluation_bound) != batch_transition_bound
            || summary.controller_evidence_bytes == 0
            || summary.controller_evidence_bytes > capabilities.max_controller_artifact_bytes
            || summary.controller_mtbdd_bytes == 0
            || summary.equivalence_artifact_bytes == 0
            || summary.unsat_proof_bytes == 0
            || summary.unsat_proof_bytes > capabilities.max_unsat_proof_bytes
            || summary.controller_mtbdd_bytes >= summary.controller_evidence_bytes
            || summary.equivalence_artifact_bytes >= summary.controller_evidence_bytes
            || summary
                .controller_mtbdd_bytes
                .checked_add(summary.equivalence_artifact_bytes)
                .is_none_or(|payload| payload >= summary.controller_evidence_bytes)
            || summary.unsat_proof_bytes > summary.equivalence_artifact_bytes
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split resource aggregate does not reconcile".to_string(),
            ));
        }
        Ok(summary)
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
    }
}

fn parse_controller_split_observed_summary(
    output: ManagedOutput,
    expected_batches: usize,
    capabilities: &ControllerSplitObservabilityCapabilities,
) -> Result<Observed<ControllerSplitObservedSummary>, PredicateOperationError> {
    let status = output.status;
    let (stdout, mut invocation_metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerSplitObservedSummary, PredicateApiError> {
        if stdout.contains('\r')
            || !stdout.ends_with('\n')
            || stdout.lines().count() != expected_batches + 2
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split observed response line count is invalid".to_string(),
            ));
        }
        let lines = stdout.lines().collect::<Vec<_>>();
        let base_stdout = format!("{}\n", lines[..expected_batches + 1].join("\n"));
        let base_output = ManagedOutput {
            status,
            stdout: base_stdout.into_bytes(),
            stderr: Vec::new(),
            metrics: invocation_metrics.clone(),
        };
        let verification = parse_controller_split_resource_set_summary(
            base_output,
            expected_batches,
            &capabilities.resource,
        )
        .map_err(|failure| *failure.error)?
        .value;

        let fields = lines[expected_batches + 1].split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-split-resource-observability",
            "status",
            "cli_version",
            "phase_metrics_version",
            "policy_and_input_micros",
            "controller_admission_micros",
            "complete_set_preflight_micros",
            "semantic_replay_micros",
            "total_micros",
            "controller_admissions",
            "manifest_loads",
            "plant_artifact_reads",
            "resource_assessments",
            "batch_verifications",
            "buffered_result_rows",
            "prepared_batches",
            "prepared_members",
            "controller_evidence_bytes",
            "total_plant_artifact_bytes",
            "total_transition_evaluation_bound",
            "timing_calibration",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller split observed metrics fields are invalid".to_string(),
            ));
        }
        let values = fields[1..]
            .iter()
            .zip(&keys[1..])
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != "MEASURED"
            || canonical_u32(values[1], keys[2])? != capabilities.cli_version
            || canonical_u32(values[2], keys[3])? != capabilities.phase_metrics_version
            || values[19] != "none"
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split observed metrics contract changed".to_string(),
            ));
        }
        let timings = values[3..8]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_u128(value, keys[index + 4]))
            .collect::<Result<Vec<_>, _>>()?;
        let counts = values[8..19]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_usize(value, keys[index + 9]))
            .collect::<Result<Vec<_>, _>>()?;
        let expected_manifest_loads = expected_batches
            .checked_mul(2)
            .and_then(|count| count.checked_add(1));
        let expected_double_batches = expected_batches.checked_mul(2);
        let expected_rows = expected_batches.checked_add(1);
        let phase_sum = timings[..4]
            .iter()
            .try_fold(0u128, |total, value| total.checked_add(*value));
        if phase_sum.is_none_or(|sum| sum > timings[4])
            || u128::try_from(verification.elapsed_micros) != Ok(timings[4])
            || counts[0] != 1
            || Some(counts[1]) != expected_manifest_loads
            || Some(counts[2]) != expected_double_batches
            || Some(counts[3]) != expected_double_batches
            || counts[4] != expected_batches
            || Some(counts[5]) != expected_rows
            || counts[6] != expected_batches
            || counts[7] != verification.members
            || counts[8] != verification.controller_evidence_bytes
            || counts[9] != verification.total_plant_artifact_bytes
            || counts[10] != verification.total_transition_evaluation_bound
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split observed metrics do not reconcile".to_string(),
            ));
        }
        Ok(ControllerSplitObservedSummary {
            verification,
            phases: ControllerSplitPhaseMetrics {
                version: capabilities.phase_metrics_version,
                policy_and_input_micros: timings[0],
                controller_admission_micros: timings[1],
                complete_set_preflight_micros: timings[2],
                semantic_replay_micros: timings[3],
                total_micros: timings[4],
                controller_admissions: counts[0],
                manifest_loads: counts[1],
                plant_artifact_reads: counts[2],
                resource_assessments: counts[3],
                batch_verifications: counts[4],
                buffered_result_rows: counts[5],
                prepared_batches: counts[6],
                prepared_members: counts[7],
                controller_evidence_bytes: counts[8],
                total_plant_artifact_bytes: counts[9],
                total_transition_evaluation_bound: counts[10],
            },
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed {
            value,
            metrics: invocation_metrics,
        }),
        Err(error) => {
            invocation_metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics: invocation_metrics,
            })
        }
    }
}

fn parse_controller_split_allocation_observed_summary(
    output: ManagedOutput,
    expected_batches: usize,
    capabilities: &ControllerSplitAllocationObservabilityCapabilities,
) -> Result<Observed<ControllerSplitAllocationObservedSummary>, PredicateOperationError> {
    let status = output.status;
    let (stdout, mut invocation_metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerSplitAllocationObservedSummary, PredicateApiError> {
        if stdout.contains('\r')
            || !stdout.ends_with('\n')
            || stdout.lines().count() != expected_batches + 3
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split allocation response line count is invalid".to_string(),
            ));
        }
        let lines = stdout.lines().collect::<Vec<_>>();
        let observed_stdout = format!("{}\n", lines[..expected_batches + 2].join("\n"));
        let observed_output = ManagedOutput {
            status,
            stdout: observed_stdout.into_bytes(),
            stderr: Vec::new(),
            metrics: invocation_metrics.clone(),
        };
        let observed = parse_controller_split_observed_summary(
            observed_output,
            expected_batches,
            &capabilities.observability,
        )
        .map_err(|failure| *failure.error)?
        .value;

        let fields = lines[expected_batches + 2].split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-split-allocation-observability",
            "status",
            "cli_version",
            "allocator",
            "scope",
            "allocation_calls",
            "allocated_bytes",
            "deallocation_calls",
            "deallocated_bytes",
            "reallocation_calls",
            "reallocated_bytes",
            "overflow",
            "timing_calibration",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller split allocation metrics fields are invalid".to_string(),
            ));
        }
        let values = fields[1..]
            .iter()
            .zip(&keys[1..])
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != "MEASURED"
            || canonical_u32(values[1], keys[2])? != capabilities.cli_version
            || values[2] != "system"
            || values[3] != "policy-through-replay"
            || values[10] != "none"
            || values[11] != "none"
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split allocation metrics contract changed".to_string(),
            ));
        }
        let counts = values[4..10]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_u64(value, keys[index + 5]))
            .collect::<Result<Vec<_>, _>>()?;
        if counts[0] == 0 || counts[1] == 0 {
            return Err(PredicateApiError::InvalidResponse(
                "controller split allocation metrics are empty".to_string(),
            ));
        }
        Ok(ControllerSplitAllocationObservedSummary {
            observed,
            allocations: ControllerSplitAllocationMetrics {
                version: capabilities.cli_version,
                allocation_calls: counts[0],
                allocated_bytes: counts[1],
                deallocation_calls: counts[2],
                deallocated_bytes: counts[3],
                reallocation_calls: counts[4],
                reallocated_bytes: counts[5],
            },
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed {
            value,
            metrics: invocation_metrics,
        }),
        Err(error) => {
            invocation_metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics: invocation_metrics,
            })
        }
    }
}

fn parse_controller_split_cache_observed_summary(
    output: ManagedOutput,
    expected_batches: usize,
    capabilities: &ControllerSplitCacheObservabilityCapabilities,
) -> Result<Observed<ControllerSplitCacheObservedSummary>, PredicateOperationError> {
    let status = output.status;
    let (stdout, mut invocation_metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerSplitCacheObservedSummary, PredicateApiError> {
        if stdout.contains('\r')
            || !stdout.ends_with('\n')
            || stdout.lines().count() != expected_batches + 4
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split cache response line count is invalid".to_string(),
            ));
        }
        let lines = stdout.lines().collect::<Vec<_>>();
        let allocation_stdout = format!("{}\n", lines[..expected_batches + 3].join("\n"));
        let allocation_output = ManagedOutput {
            status,
            stdout: allocation_stdout.into_bytes(),
            stderr: Vec::new(),
            metrics: invocation_metrics.clone(),
        };
        let observed = parse_controller_split_allocation_observed_summary(
            allocation_output,
            expected_batches,
            &capabilities.allocation_observability,
        )
        .map_err(|failure| *failure.error)?
        .value;

        let fields = lines[expected_batches + 3].split(' ').collect::<Vec<_>>();
        let keys = [
            "controller-split-cache-observability",
            "status",
            "cli_version",
            "scope",
            "key",
            "lookups",
            "hits",
            "misses",
            "entries",
            "integrity_preflight",
            "overflow",
            "timing_calibration",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller split cache metrics fields are invalid".to_string(),
            ));
        }
        let values = fields[1..]
            .iter()
            .zip(&keys[1..])
            .map(|(field, key)| token_value(field, key))
            .collect::<Result<Vec<_>, _>>()?;
        if values[0] != "MEASURED"
            || canonical_u32(values[1], keys[2])? != capabilities.cli_version
            || values[2] != "semantic-replay"
            || values[3] != "manifest-snapshot,resource-assessment,result-sha256"
            || values[8] != "required"
            || values[9] != "none"
            || values[10] != "none"
        {
            return Err(PredicateApiError::IncompatibleContract(
                "controller split cache metrics contract changed".to_string(),
            ));
        }
        let counts = values[4..8]
            .iter()
            .enumerate()
            .map(|(index, value)| canonical_usize(value, keys[index + 5]))
            .collect::<Result<Vec<_>, _>>()?;
        let accounted = counts[1].checked_add(counts[2]);
        if counts[0] != expected_batches
            || accounted != Some(counts[0])
            || counts[3] != counts[2]
            || counts[3] > counts[0]
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller split cache metrics do not reconcile".to_string(),
            ));
        }
        Ok(ControllerSplitCacheObservedSummary {
            observed,
            cache: ControllerSplitCacheMetrics {
                version: capabilities.cli_version,
                lookups: counts[0],
                hits: counts[1],
                misses: counts[2],
                entries: counts[3],
            },
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed {
            value,
            metrics: invocation_metrics,
        }),
        Err(error) => {
            invocation_metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics: invocation_metrics,
            })
        }
    }
}

fn classify_controller_split_resource_refusal(
    mut failure: PredicateOperationError,
) -> PredicateOperationError {
    let reason = match failure.error.as_ref() {
        PredicateApiError::CommandFailed {
            exit_code: Some(3),
            stderr,
        } => stderr
            .trim_end_matches(['\r', '\n'])
            .strip_prefix("error: controller-split-resource refusal=")
            .and_then(|value| value.strip_suffix(" result=none"))
            .and_then(|value| match value {
                "controller-artifact-bytes" => {
                    Some(ControllerPlantResourceRefusalReason::ControllerArtifactBytes)
                }
                "unsat-proof-bytes" => Some(ControllerPlantResourceRefusalReason::UnsatProofBytes),
                "batches" => Some(ControllerPlantResourceRefusalReason::Batches),
                "plant-artifact-bytes" => {
                    Some(ControllerPlantResourceRefusalReason::PlantArtifactBytes)
                }
                "members-per-batch" => Some(ControllerPlantResourceRefusalReason::MembersPerBatch),
                "horizon" => Some(ControllerPlantResourceRefusalReason::Horizon),
                "product-states" => Some(ControllerPlantResourceRefusalReason::ProductStates),
                "transitions-per-batch" => {
                    Some(ControllerPlantResourceRefusalReason::TransitionsPerBatch)
                }
                "total-plant-artifact-bytes" => {
                    Some(ControllerPlantResourceRefusalReason::TotalPlantArtifactBytes)
                }
                "total-members" => Some(ControllerPlantResourceRefusalReason::TotalMembers),
                "total-transition-evaluations" => {
                    Some(ControllerPlantResourceRefusalReason::TotalTransitionEvaluations)
                }
                _ => None,
            }),
        _ => None,
    };
    if let Some(reason) = reason {
        let error = PredicateApiError::ResourceRefused { reason };
        failure.metrics.status = InvocationStatus::Failed(error.failure_class());
        failure.error = Box::new(error);
    }
    failure
}

fn parse_controller_mtbdd_summary(
    output: ManagedOutput,
    expected_action: &str,
    expected_output: Option<&Path>,
    capabilities: &ControllerMtbddCapabilities,
    summary_prefix: &str,
    member_prefix: &str,
) -> Result<Observed<ControllerMtbddBatchSummary>, PredicateOperationError> {
    let (stdout, mut metrics) = successful_stdout(output)?;
    let parsed = (|| -> Result<ControllerMtbddBatchSummary, PredicateApiError> {
        if stdout.contains('\r') || !stdout.ends_with('\n') {
            return Err(PredicateApiError::InvalidResponse(
                "controller MTBDD response is not canonical LF text".to_string(),
            ));
        }
        let mut lines = stdout.lines();
        let first = lines.next().ok_or_else(|| {
            PredicateApiError::InvalidResponse(
                "controller MTBDD summary line is missing".to_string(),
            )
        })?;
        let summary_text = if let Some(expected_output) = expected_output {
            first
                .split_once(" output=")
                .and_then(|(summary, output)| {
                    (output == expected_output.to_string_lossy()).then_some(summary)
                })
                .ok_or_else(|| {
                    PredicateApiError::InvalidResponse(
                        "controller MTBDD created response output disagrees".to_string(),
                    )
                })?
        } else if first.contains(" output=") {
            return Err(PredicateApiError::InvalidResponse(
                "controller MTBDD verified response unexpectedly names output".to_string(),
            ));
        } else {
            first
        };
        let fields = summary_text.split(' ').collect::<Vec<_>>();
        let keys = [
            summary_prefix,
            "status",
            "cli_version",
            "artifact_version",
            "members",
            "safe",
            "unsafe",
            "mtbdd_nodes",
            "mtbdd_terminals",
            "assignments_checked",
            "reachable_product_states",
            "explored_transitions",
            "artifact_bytes",
            "elapsed_micros",
        ];
        if fields.len() != keys.len() || fields[0] != keys[0] {
            return Err(PredicateApiError::InvalidResponse(
                "controller MTBDD summary shape is invalid".to_string(),
            ));
        }
        if token_value(fields[1], keys[1])? != expected_action {
            return Err(PredicateApiError::InvalidResponse(
                "controller MTBDD response action disagrees".to_string(),
            ));
        }
        let numeric = fields[2..]
            .iter()
            .zip(&keys[2..])
            .map(|(field, key)| canonical_usize(token_value(field, key)?, key))
            .collect::<Result<Vec<_>, _>>()?;
        if numeric[0] != capabilities.cli_version as usize {
            return Err(PredicateApiError::IncompatibleContract(
                "controller MTBDD response CLI version changed".to_string(),
            ));
        }
        if numeric[1] != capabilities.plant_artifact_version as usize {
            return Err(PredicateApiError::IncompatibleContract(
                "controller MTBDD response artifact version changed".to_string(),
            ));
        }
        let member_count = numeric[2];
        if member_count == 0
            || member_count > capabilities.max_members
            || numeric[5] > capabilities.max_nodes
            || numeric[6] > capabilities.max_terminals
            || numeric[7] > capabilities.max_assignments
            || numeric[10] > capabilities.max_artifact_bytes
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller MTBDD response exceeds discovered limits".to_string(),
            ));
        }
        let mut members = Vec::with_capacity(member_count);
        for expected_index in 0..member_count {
            let line = lines.next().ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller MTBDD member line is missing".to_string(),
                )
            })?;
            let fields = line.split(' ').collect::<Vec<_>>();
            let keys = [
                member_prefix,
                "index",
                "answer",
                "horizon",
                "bad_frame",
                "trace_steps",
                "reachable_product_states",
                "explored_transitions",
            ];
            if fields.len() != keys.len() || fields[0] != keys[0] {
                return Err(PredicateApiError::InvalidResponse(
                    "controller MTBDD member shape is invalid".to_string(),
                ));
            }
            let index = canonical_usize(token_value(fields[1], "index")?, "index")?;
            if index != expected_index {
                return Err(PredicateApiError::InvalidResponse(
                    "controller MTBDD member order is invalid".to_string(),
                ));
            }
            let answer = match token_value(fields[2], "answer")? {
                "SAFE" => ControllerMtbddAnswer::Safe,
                "UNSAFE" => ControllerMtbddAnswer::Unsafe,
                _ => {
                    return Err(PredicateApiError::InvalidResponse(
                        "controller MTBDD member answer is invalid".to_string(),
                    ));
                }
            };
            let bad = token_value(fields[4], "bad_frame")?;
            let bad_frame = if bad == "none" {
                None
            } else {
                Some(canonical_usize(bad, "bad_frame")?)
            };
            if matches!(answer, ControllerMtbddAnswer::Safe) != bad_frame.is_none() {
                return Err(PredicateApiError::InvalidResponse(
                    "controller MTBDD answer and bad frame disagree".to_string(),
                ));
            }
            let horizon = canonical_usize(token_value(fields[3], "horizon")?, "horizon")?;
            let trace_steps =
                canonical_usize(token_value(fields[5], "trace_steps")?, "trace_steps")?;
            if horizon > capabilities.max_horizon
                || match (answer, bad_frame) {
                    (ControllerMtbddAnswer::Safe, None) => trace_steps != 0,
                    (ControllerMtbddAnswer::Unsafe, Some(frame)) => {
                        frame > horizon || trace_steps != frame.saturating_add(1)
                    }
                    _ => true,
                }
            {
                return Err(PredicateApiError::InvalidResponse(
                    "controller MTBDD member trace boundary is invalid".to_string(),
                ));
            }
            members.push(ControllerMtbddMemberResult {
                index,
                answer,
                horizon,
                bad_frame,
                trace_steps,
                reachable_product_states: canonical_usize(
                    token_value(fields[6], "reachable_product_states")?,
                    "reachable_product_states",
                )?,
                explored_transitions: canonical_usize(
                    token_value(fields[7], "explored_transitions")?,
                    "explored_transitions",
                )?,
            });
        }
        let reachable_total = members
            .iter()
            .try_fold(0usize, |total, member| {
                total.checked_add(member.reachable_product_states)
            })
            .ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller MTBDD reachable-state total overflows".to_string(),
                )
            })?;
        let transition_total = members
            .iter()
            .try_fold(0usize, |total, member| {
                total.checked_add(member.explored_transitions)
            })
            .ok_or_else(|| {
                PredicateApiError::InvalidResponse(
                    "controller MTBDD transition total overflows".to_string(),
                )
            })?;
        if lines.next().is_some()
            || members
                .iter()
                .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Safe))
                .count()
                != numeric[3]
            || members
                .iter()
                .filter(|member| matches!(member.answer, ControllerMtbddAnswer::Unsafe))
                .count()
                != numeric[4]
            || reachable_total != numeric[8]
            || transition_total != numeric[9]
        {
            return Err(PredicateApiError::InvalidResponse(
                "controller MTBDD member totals disagree".to_string(),
            ));
        }
        Ok(ControllerMtbddBatchSummary {
            artifact_version: numeric[1] as u32,
            safe: numeric[3],
            unsafe_count: numeric[4],
            mtbdd_nodes: numeric[5],
            mtbdd_terminals: numeric[6],
            assignments_checked: numeric[7],
            reachable_product_states: numeric[8],
            explored_transitions: numeric[9],
            artifact_bytes: numeric[10],
            elapsed_micros: numeric[11],
            members,
        })
    })();
    match parsed {
        Ok(value) => Ok(Observed { value, metrics }),
        Err(error) => {
            metrics.status = InvocationStatus::Failed(error.failure_class());
            Err(PredicateOperationError {
                error: Box::new(error),
                metrics,
            })
        }
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
    fn revision_impact_capability_parser_is_strict_and_fail_closed() {
        let canonical = "revision_impact_cli_version=2 impact_version=1 query_manifest_version=1 max_query_manifest_bytes=16384 max_input_bytes=67108864 max_evidence_bytes=16777216 max_bundle_bytes=67108864 max_atoms=8 max_combinations=256 max_queries=32 semantics=exact-counterfactual-v1 work_schema=verification-v1 query_schema=transition-semantic-set-v1 routing=none fallback=none unsupported=fail-closed\n";
        let parsed = parse_revision_impact_capabilities(canonical).unwrap();
        assert_eq!(parsed.cli_version, 2);
        assert_eq!(parsed.max_bundle_bytes, 64 * 1024 * 1024);
        for hostile in [
            canonical.replace("impact_version=1", "impact_version=2"),
            canonical.replace("max_atoms=8", "max_atoms=08"),
            canonical.replace("max_queries=32", "max_queries=33"),
            canonical.replace("routing=none", "routing=heuristic"),
            canonical.replace("work_schema=verification-v1", "work_schema=timing-v1"),
            canonical.replace(
                "query_schema=transition-semantic-set-v1",
                "query_schema=summary-v1",
            ),
            canonical.replace("fallback=none", "fallback=exact"),
            canonical.replace("unsupported=fail-closed", "unsupported=ignore"),
            canonical.replace('\n', "\r\n"),
            canonical.trim_end().to_string(),
            format!("{canonical}unexpected=1\n"),
        ] {
            assert!(parse_revision_impact_capabilities(&hostile).is_err());
        }
        assert!(canonical_node_list(&[]).is_err());
        assert!(canonical_node_list(&[0]).is_err());
        assert!(canonical_node_list(&[7, 7]).is_err());
        assert!(canonical_node_list(&[10, 7]).is_err());
        assert_eq!(canonical_node_list(&[7, 10]).unwrap(), "7,10");
    }

    #[test]
    fn controller_mtbdd_capability_parser_is_strict_and_fail_closed() {
        let canonical = "controller_mtbdd_cli_version=1 mtbdd_version=1 plant_artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=33554432 max_members=64 max_state_bits=20 max_inputs=32 max_outputs=8 max_nodes=1048576 max_terminals=1048576 max_assignments=1048576 max_horizon=4096 unsupported=fail-closed\n";
        let parsed = parse_controller_mtbdd_capabilities(canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.max_outputs, 8);
        assert!(
            parse_controller_mtbdd_capabilities(&canonical.replacen(
                "controller_mtbdd_cli_version=1",
                "controller_mtbdd_cli_version=4294967297",
                1
            ))
            .is_err()
        );
        assert!(
            parse_controller_mtbdd_capabilities(
                &canonical.replace("max_outputs=8", "max_outputs=08")
            )
            .is_err()
        );
        assert!(
            parse_controller_mtbdd_capabilities(
                &canonical.replace("max_members=64", "max_members=0")
            )
            .is_err()
        );
        assert!(
            parse_controller_mtbdd_capabilities(
                &canonical.replace("unsupported=fail-closed", "unsupported=best-effort")
            )
            .is_err()
        );
        assert!(parse_controller_mtbdd_capabilities(&canonical.replace('\n', "\r\n")).is_err());
    }

    #[test]
    fn controller_proof_mtbdd_capability_parser_is_strict() {
        let canonical = "controller_proof_mtbdd_cli_version=1 mtbdd_version=1 equivalence_proof_version=1 plant_artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=16777216 max_equivalence_artifact_bytes=2097152 max_unsat_proof_bytes=1048576 max_members=64 max_state_bits=6 max_inputs=12 max_outputs=8 max_nodes=512 max_terminals=1024 max_horizon=1024 verification=unsat-miter exhaustive_replay=no unsupported=fail-closed\n";
        let parsed = parse_controller_proof_mtbdd_capabilities(canonical).unwrap();
        assert_eq!(parsed.equivalence_proof_version, 1);
        assert_eq!(parsed.max_equivalence_artifact_bytes, 2_097_152);
        assert_eq!(parsed.max_unsat_proof_bytes, 1_048_576);
        for hostile in [
            canonical.replace("verification=unsat-miter", "verification=trusted"),
            canonical.replace("exhaustive_replay=no", "exhaustive_replay=yes"),
            canonical.replace(
                "max_equivalence_artifact_bytes=2097152",
                "max_equivalence_artifact_bytes=0",
            ),
            canonical.replace("max_unsat_proof_bytes=1048576", "max_unsat_proof_bytes=0"),
            canonical.replace("unsupported=fail-closed", "unsupported=best-effort"),
            canonical.replace('\n', "\r\n"),
        ] {
            assert!(parse_controller_proof_mtbdd_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_split_evidence_capability_parser_is_strict() {
        let canonical = "controller_split_evidence_cli_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=16777216 max_batches=64 admission=once verification=unsat-miter exhaustive_replay=no source_binding=sha256 obligation_binding=complete-ordered unsupported=fail-closed\n";
        let parsed = parse_controller_split_evidence_capabilities(canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.max_batches, 64);
        for hostile in [
            canonical.replace("admission=once", "admission=per-batch"),
            canonical.replace("verification=unsat-miter", "verification=trusted"),
            canonical.replace("exhaustive_replay=no", "exhaustive_replay=yes"),
            canonical.replace("source_binding=sha256", "source_binding=path"),
            canonical.replace(
                "obligation_binding=complete-ordered",
                "obligation_binding=partial",
            ),
            canonical.replace("max_batches=64", "max_batches=0"),
            canonical.replace("max_batches=64", "max_batches=064"),
            canonical.replace("max_batches=64", "max_batches=65"),
            canonical.replace("max_artifact_bytes=16777216", "max_artifact_bytes=16777217"),
            canonical.replace("unsupported=fail-closed", "unsupported=best-effort"),
            canonical.replace('\n', "\r\n"),
        ] {
            assert!(parse_controller_split_evidence_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_split_resource_capability_parser_is_strict() {
        let canonical = "controller_split_resource_cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_policy_bytes=4096 max_controller_artifact_bytes=16777216 max_unsat_proof_bytes=1048576 max_plant_artifact_bytes=16777216 max_batches=64 max_members_per_batch=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 admission=once verification=unsat-miter exhaustive_replay=no accounting=conservative-static-per-batch-and-total timing_calibration=none result_on_refusal=none refusal_schema=split-reason-v1 unsupported=fail-closed\n";
        let parsed = parse_controller_split_resource_capabilities(canonical).unwrap();
        assert_eq!(parsed.policy_version, 1);
        assert_eq!(parsed.max_batches, 64);
        for hostile in [
            canonical.replace("admission=once", "admission=per-batch"),
            canonical.replace("verification=unsat-miter", "verification=trusted"),
            canonical.replace(
                "accounting=conservative-static-per-batch-and-total",
                "accounting=measured",
            ),
            canonical.replace("timing_calibration=none", "timing_calibration=per-formula"),
            canonical.replace("result_on_refusal=none", "result_on_refusal=safe"),
            canonical.replace("refusal_exit=3", "refusal_exit=2"),
            canonical.replace("max_batches=64", "max_batches=65"),
            canonical.replace("max_policy_bytes=4096", "max_policy_bytes=0"),
            canonical.replace("refusal_schema=split-reason-v1", "refusal_schema=free-text"),
            canonical.replace('\n', "\r\n"),
        ] {
            assert!(parse_controller_split_resource_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_split_observability_capability_parser_is_strict() {
        let resource = "controller_split_resource_cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_policy_bytes=4096 max_controller_artifact_bytes=16777216 max_unsat_proof_bytes=1048576 max_plant_artifact_bytes=16777216 max_batches=64 max_members_per_batch=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 admission=once verification=unsat-miter exhaustive_replay=no accounting=conservative-static-per-batch-and-total timing_calibration=none result_on_refusal=none refusal_schema=split-reason-v1 unsupported=fail-closed";
        let observability = "controller_split_observability_cli_version=1 base_cli_version=1 phase_metrics_version=1 phases=policy-and-input,controller-admission,complete-set-preflight,semantic-replay counters=controller-admissions,manifest-loads,plant-artifact-reads,resource-assessments,batch-verifications,buffered-result-rows,prepared-batches,prepared-members,controller-evidence-bytes,total-plant-artifact-bytes,total-transition-evaluation-bound timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed";
        let canonical = format!("{resource}\n{observability}\n");
        let parsed = parse_controller_split_observability_capabilities(&canonical).unwrap();
        assert_eq!(parsed.resource.cli_version, 1);
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.phase_metrics_version, 1);
        for hostile in [
            canonical.replace("phase_metrics_version=1", "phase_metrics_version=2"),
            canonical.replace("semantic-replay", "trusted-replay"),
            canonical.replace("manifest-loads", "manifest-hints"),
            canonical.replace(
                "partial_metrics_on_failure=none",
                "partial_metrics_on_failure=yes",
            ),
            canonical.replace("timing_calibration=none", "timing_calibration=per-formula"),
            canonical.replace('\n', "\r\n"),
            format!("{resource}\n"),
            format!("{observability}\n{resource}\n"),
        ] {
            assert!(parse_controller_split_observability_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_split_allocation_capability_parser_is_strict() {
        let resource = "controller_split_resource_cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_policy_bytes=4096 max_controller_artifact_bytes=16777216 max_unsat_proof_bytes=1048576 max_plant_artifact_bytes=16777216 max_batches=64 max_members_per_batch=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 admission=once verification=unsat-miter exhaustive_replay=no accounting=conservative-static-per-batch-and-total timing_calibration=none result_on_refusal=none refusal_schema=split-reason-v1 unsupported=fail-closed";
        let observability = "controller_split_observability_cli_version=1 base_cli_version=1 phase_metrics_version=1 phases=policy-and-input,controller-admission,complete-set-preflight,semantic-replay counters=controller-admissions,manifest-loads,plant-artifact-reads,resource-assessments,batch-verifications,buffered-result-rows,prepared-batches,prepared-members,controller-evidence-bytes,total-plant-artifact-bytes,total-transition-evaluation-bound timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed";
        let allocation = "controller_split_allocation_observability_cli_version=1 base_observability_cli_version=1 allocator=system scope=policy-through-replay counters=allocation-calls,allocated-bytes,deallocation-calls,deallocated-bytes,reallocation-calls,reallocated-bytes overflow=fail-closed timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed";
        let canonical = format!("{resource}\n{observability}\n{allocation}\n");
        let parsed =
            parse_controller_split_allocation_observability_capabilities(&canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.observability.cli_version, 1);
        for hostile in [
            canonical.replace("allocator=system", "allocator=jemalloc"),
            canonical.replace("scope=policy-through-replay", "scope=whole-process"),
            canonical.replace("allocated-bytes", "live-bytes"),
            canonical.replace("overflow=fail-closed", "overflow=saturate"),
            canonical.replace(
                "base_observability_cli_version=1",
                "base_observability_cli_version=2",
            ),
            canonical.replace('\n', "\r\n"),
            format!("{resource}\n{observability}\n"),
        ] {
            assert!(
                parse_controller_split_allocation_observability_capabilities(&hostile).is_err()
            );
        }
    }

    #[test]
    fn controller_split_cache_capability_parser_is_strict() {
        let resource = "controller_split_resource_cli_version=1 policy_version=1 controller_envelope_version=1 plant_envelope_version=1 controller_artifact_version=1 plant_artifact_version=1 manifest_version=1 max_policy_bytes=4096 max_controller_artifact_bytes=16777216 max_unsat_proof_bytes=1048576 max_plant_artifact_bytes=16777216 max_batches=64 max_members_per_batch=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 admission=once verification=unsat-miter exhaustive_replay=no accounting=conservative-static-per-batch-and-total timing_calibration=none result_on_refusal=none refusal_schema=split-reason-v1 unsupported=fail-closed";
        let observability = "controller_split_observability_cli_version=1 base_cli_version=1 phase_metrics_version=1 phases=policy-and-input,controller-admission,complete-set-preflight,semantic-replay counters=controller-admissions,manifest-loads,plant-artifact-reads,resource-assessments,batch-verifications,buffered-result-rows,prepared-batches,prepared-members,controller-evidence-bytes,total-plant-artifact-bytes,total-transition-evaluation-bound timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed";
        let allocation = "controller_split_allocation_observability_cli_version=1 base_observability_cli_version=1 allocator=system scope=policy-through-replay counters=allocation-calls,allocated-bytes,deallocation-calls,deallocated-bytes,reallocation-calls,reallocated-bytes overflow=fail-closed timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed";
        let cache = "controller_split_cache_observability_cli_version=1 base_allocation_observability_cli_version=1 scope=semantic-replay key=manifest-snapshot,resource-assessment,result-sha256 counters=lookups,hits,misses,entries integrity_preflight=required overflow=fail-closed timing_calibration=none partial_metrics_on_failure=none result_on_refusal=none unsupported=fail-closed";
        let canonical = format!("{resource}\n{observability}\n{allocation}\n{cache}\n");
        let parsed = parse_controller_split_cache_observability_capabilities(&canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.allocation_observability.cli_version, 1);
        for hostile in [
            canonical.replace("scope=semantic-replay", "scope=preflight"),
            canonical.replace("manifest-snapshot", "manifest-path"),
            canonical.replace(
                "integrity_preflight=required",
                "integrity_preflight=optional",
            ),
            canonical.replace("hits,misses", "misses,hits"),
            canonical.replace("overflow=fail-closed", "overflow=saturate"),
            canonical.replace(
                "base_allocation_observability_cli_version=1",
                "base_allocation_observability_cli_version=2",
            ),
            canonical.replace('\n', "\r\n"),
            format!("{resource}\n{observability}\n{allocation}\n"),
        ] {
            assert!(parse_controller_split_cache_observability_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_plant_portfolio_capability_parser_is_strict() {
        let canonical = "controller_plant_portfolio_cli_version=1 artifact_version=1 manifest_version=1 max_manifest_bytes=65536 max_artifact_bytes=16777216 max_members=64 backends=mtbdd,direct-exact routing=static fallback=exact unsupported=fail-closed\n";
        let parsed = parse_controller_plant_portfolio_capabilities(canonical).unwrap();
        assert_eq!(parsed.cli_version, 1);
        assert_eq!(parsed.max_members, 64);
        assert!(
            parse_controller_plant_portfolio_capabilities(
                &canonical.replace("routing=static", "routing=timed")
            )
            .is_err()
        );
        assert!(
            parse_controller_plant_portfolio_capabilities(
                &canonical.replace("max_members=64", "max_members=064")
            )
            .is_err()
        );
        assert!(
            parse_controller_plant_portfolio_capabilities(&canonical.replace('\n', "\r\n"))
                .is_err()
        );
    }

    #[test]
    fn controller_plant_resource_capability_parser_is_strict() {
        let canonical = "controller_plant_resource_cli_version=1 policy_version=1 envelope_version=1 manifest_version=1 portfolio_artifact_version=1 max_policy_bytes=4096 max_artifact_bytes=16777216 max_members=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 accounting=conservative-static timing_calibration=none result_on_refusal=none refusal_schema=reason-v1 unsupported=fail-closed\n";
        let parsed = parse_controller_plant_resource_capabilities(canonical).unwrap();
        assert_eq!(parsed.envelope_version, 1);
        assert_eq!(parsed.max_product_states, 4096);
        for hostile in [
            canonical.replace("accounting=conservative-static", "accounting=measured"),
            canonical.replace("timing_calibration=none", "timing_calibration=per-formula"),
            canonical.replace("result_on_refusal=none", "result_on_refusal=safe"),
            canonical.replace("refusal_exit=3", "refusal_exit=2"),
            canonical.replace("refusal_schema=reason-v1", "refusal_schema=free-text"),
            canonical.replace("max_policy_bytes=4096", "max_policy_bytes=0"),
            canonical.replace("max_members=64", "max_members=064"),
            canonical.replace('\n', "\r\n"),
        ] {
            assert!(parse_controller_plant_resource_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_proof_mtbdd_resource_capability_parser_is_strict() {
        let canonical = "controller_proof_mtbdd_resource_cli_version=1 policy_version=1 envelope_version=1 manifest_version=1 artifact_version=1 max_policy_bytes=4096 max_artifact_bytes=16777216 max_equivalence_artifact_bytes=2097152 max_unsat_proof_bytes=1048576 max_members=64 max_horizon=1024 max_product_states=4096 refusal_exit=3 verification=unsat-miter exhaustive_replay=no accounting=conservative-static timing_calibration=none result_on_refusal=none refusal_schema=proof-reason-v1 unsupported=fail-closed\n";
        let parsed = parse_controller_proof_mtbdd_resource_capabilities(canonical).unwrap();
        assert_eq!(parsed.max_equivalence_artifact_bytes, 2_097_152);
        assert_eq!(parsed.max_unsat_proof_bytes, 1_048_576);
        for hostile in [
            canonical.replace("verification=unsat-miter", "verification=trusted"),
            canonical.replace("exhaustive_replay=no", "exhaustive_replay=yes"),
            canonical.replace("accounting=conservative-static", "accounting=measured"),
            canonical.replace("result_on_refusal=none", "result_on_refusal=safe"),
            canonical.replace("refusal_schema=proof-reason-v1", "refusal_schema=free-text"),
            canonical.replace("max_unsat_proof_bytes=1048576", "max_unsat_proof_bytes=0"),
            canonical.replace("max_members=64", "max_members=064"),
            canonical.replace('\n', "\r\n"),
        ] {
            assert!(parse_controller_proof_mtbdd_resource_capabilities(&hostile).is_err());
        }
    }

    #[test]
    fn controller_proof_mtbdd_portfolio_capability_parser_is_strict() {
        let canonical = "controller_proof_mtbdd_portfolio_cli_version=1 policy_version=1 envelope_version=1 artifact_version=1 proof_artifact_version=1 direct_artifact_version=1 manifest_version=1 source_model_attestation_version=1 max_policy_bytes=4096 max_artifact_bytes=16777216 max_equivalence_artifact_bytes=2097152 max_unsat_proof_bytes=1048576 max_members=64 max_horizon=1024 max_product_states=4096 max_attestation_bytes=65536 refusal_exit=3 backends=proof-mtbdd,direct-exact routing=static fallback=exact proof_failure=fail-closed attested_verification=required accounting=conservative-static timing_calibration=none result_on_refusal=none refusal_schema=proof-reason-v1 unsupported=fail-closed\n";
        let parsed = parse_controller_proof_mtbdd_portfolio_capabilities(canonical).unwrap();
        assert_eq!(parsed.artifact_version, 1);
        assert_eq!(parsed.max_unsat_proof_bytes, 1_048_576);
        for hostile in [
            canonical.replace("backends=proof-mtbdd,direct-exact", "backends=direct-exact"),
            canonical.replace("routing=static", "routing=timed"),
            canonical.replace("fallback=exact", "fallback=heuristic"),
            canonical.replace("proof_failure=fail-closed", "proof_failure=fallback"),
            canonical.replace(
                "attested_verification=required",
                "attested_verification=optional",
            ),
            canonical.replace("result_on_refusal=none", "result_on_refusal=safe"),
            canonical.replace("max_members=64", "max_members=064"),
            canonical.replace('\n', "\r\n"),
        ] {
            assert!(parse_controller_proof_mtbdd_portfolio_capabilities(&hostile).is_err());
        }
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

        let successful = InvocationMetrics {
            schema_version: 1,
            operation: OperationKind::Discover,
            duration: Duration::from_nanos(7),
            stdout_bytes: 10,
            stderr_bytes: 0,
            timeout: Duration::from_millis(700),
            output_limit_bytes: 8192,
            memory_limit_bytes: None,
            file_limit_bytes: 32 * 1024 * 1024,
            process_group_containment: true,
            exit_code: Some(0),
            status: InvocationStatus::Success,
        };
        let aggregate = aggregate_invocation_metrics([&metrics, &successful]).unwrap();
        assert_eq!(aggregate.jobs, 2);
        assert_eq!(aggregate.successes, 1);
        assert_eq!(aggregate.failures, 1);
        assert_eq!(aggregate.total_duration_ns, 130);
        assert_eq!(aggregate.maximum_duration_ns, 123);
        assert_eq!(aggregate.total_stdout_bytes, 55);
        assert_eq!(aggregate.total_stderr_bytes, 6);
        assert_eq!(aggregate.process_group_contained_jobs, 2);
        assert_eq!(aggregate.memory_limited_jobs, 1);
        assert_eq!(aggregate.operation_counts["discover"], 1);
        assert_eq!(aggregate.operation_counts["verify_v2"], 1);
        assert_eq!(aggregate.failure_counts["exit_status"], 1);
        assert_eq!(
            InvocationMetricsAggregate::csv_header(),
            "schema_version,jobs,successes,failures,total_duration_ns,maximum_duration_ns,total_stdout_bytes,total_stderr_bytes,process_group_contained_jobs,memory_limited_jobs,operation_counts,failure_counts"
        );
        assert_eq!(
            aggregate.to_csv_row(),
            "1,2,1,1,130,123,55,6,2,1,discover=1;verify_v2=1,exit_status=1"
        );

        let mut incompatible = successful.clone();
        incompatible.schema_version = 2;
        assert_eq!(
            aggregate_invocation_metrics([&incompatible]),
            Err(MetricsAggregationError::UnsupportedSchema(2))
        );
        assert_eq!(
            aggregate_invocation_metrics(std::iter::empty()),
            Err(MetricsAggregationError::Empty)
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
