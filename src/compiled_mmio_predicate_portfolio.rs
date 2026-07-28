//! Static routing between finite-domain predicate evidence and the complete
//! exact compiled-MMIO reference.

use crate::compiled_mmio_predicate_certificate::{
    CompiledMmioPredicateCertificateError, CompiledMmioPredicateCertificateVerification,
    certify_compiled_mmio_predicate, encode_compiled_mmio_predicate_certificate,
    verify_compiled_mmio_predicate_bytes,
};
use crate::compiled_mmio_quotient::{
    ExactCompiledMmioReference, ExactCompiledMmioReferenceError,
    build_exact_compiled_mmio_reference, verify_exact_compiled_mmio_reference,
};
use crate::riscv32imc::Rv32SymbolLayout;
use crate::riscv32imc_predicate::execute_invalid_channel_predicate;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const COMPILED_MMIO_PREDICATE_PORTFOLIO_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompiledMmioPredicateRoute {
    PredicateV1,
    ExactV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicatePreflight {
    version: u32,
    image_bytes: u32,
    image_sha256: [u8; 32],
    symbols: Rv32SymbolLayout,
    route: CompiledMmioPredicateRoute,
    refusal: Option<String>,
    decoded_transitions: Option<u64>,
    lane_value_operations: Option<u64>,
}

impl CompiledMmioPredicatePreflight {
    pub fn route(&self) -> CompiledMmioPredicateRoute {
        self.route
    }

    pub fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }

    pub fn decoded_transitions(&self) -> Option<u64> {
        self.decoded_transitions
    }

    pub fn lane_value_operations(&self) -> Option<u64> {
        self.lane_value_operations
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledMmioPredicateEvidence {
    PredicateV1(Vec<u8>),
    ExactV1(ExactCompiledMmioReference),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicatePortfolioVerification {
    pub route: CompiledMmioPredicateRoute,
    pub preflight_decoded_transitions: Option<u64>,
    pub producer_decoded_transitions: u64,
    pub verifier_decoded_transitions: u64,
    pub total_decoded_transitions: Option<u64>,
    pub preflight_lane_value_operations: Option<u64>,
    pub producer_lane_value_operations: u64,
    pub verifier_lane_value_operations: u64,
    pub total_lane_value_operations: Option<u64>,
    pub predicate_artifact_bytes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicatePortfolioError(pub String);

impl fmt::Display for CompiledMmioPredicatePortfolioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO predicate portfolio: {}", self.0)
    }
}

impl Error for CompiledMmioPredicatePortfolioError {}

impl From<CompiledMmioPredicateCertificateError> for CompiledMmioPredicatePortfolioError {
    fn from(error: CompiledMmioPredicateCertificateError) -> Self {
        Self(error.to_string())
    }
}

impl From<ExactCompiledMmioReferenceError> for CompiledMmioPredicatePortfolioError {
    fn from(error: ExactCompiledMmioReferenceError) -> Self {
        Self(error.to_string())
    }
}

fn reject(message: impl Into<String>) -> CompiledMmioPredicatePortfolioError {
    CompiledMmioPredicatePortfolioError(message.into())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn check_identity(
    preflight: &CompiledMmioPredicatePreflight,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<(), CompiledMmioPredicatePortfolioError> {
    if preflight.version != COMPILED_MMIO_PREDICATE_PORTFOLIO_VERSION {
        return Err(reject("unsupported preflight version"));
    }
    if preflight.image_bytes as usize != image.len()
        || preflight.image_sha256 != digest(image)
        || preflight.symbols != symbols
    {
        return Err(reject("source changed after portfolio preflight"));
    }
    let canonical_lane_work = preflight.decoded_transitions.and_then(|work| {
        work.checked_mul(crate::riscv32imc_predicate::INVALID_PREDICATE_LANES as u64)
    });
    if preflight.lane_value_operations != canonical_lane_work {
        return Err(reject("preflight work counters are inconsistent"));
    }
    match (
        preflight.route,
        &preflight.refusal,
        preflight.decoded_transitions,
        preflight.lane_value_operations,
    ) {
        (CompiledMmioPredicateRoute::PredicateV1, None, Some(_), Some(_))
        | (CompiledMmioPredicateRoute::ExactV1, Some(_), None, None) => Ok(()),
        _ => Err(reject("preflight route and refusal are inconsistent")),
    }
}

/// Select a route from source structure and exact finite-domain control only.
///
/// This emits no terminal behavior or reusable evidence. A later production
/// failure on an admitted predicate route refuses rather than changing routes.
pub fn preflight_compiled_mmio_predicate(
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioPredicatePreflight, CompiledMmioPredicatePortfolioError> {
    let image_bytes =
        u32::try_from(image.len()).map_err(|_| reject("image byte count overflow"))?;
    let (route, refusal, decoded_transitions, lane_value_operations) =
        match execute_invalid_channel_predicate(image, symbols) {
            Ok(execution) => (
                CompiledMmioPredicateRoute::PredicateV1,
                None,
                Some(execution.symbolic_transitions),
                Some(execution.lane_value_operations),
            ),
            Err(error) => (
                CompiledMmioPredicateRoute::ExactV1,
                Some(error.to_string()),
                None,
                None,
            ),
        };
    Ok(CompiledMmioPredicatePreflight {
        version: COMPILED_MMIO_PREDICATE_PORTFOLIO_VERSION,
        image_bytes,
        image_sha256: digest(image),
        symbols,
        route,
        refusal,
        decoded_transitions,
        lane_value_operations,
    })
}

pub fn produce_compiled_mmio_predicate_portfolio(
    preflight: &CompiledMmioPredicatePreflight,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioPredicateEvidence, CompiledMmioPredicatePortfolioError> {
    check_identity(preflight, image, symbols)?;
    match preflight.route {
        CompiledMmioPredicateRoute::PredicateV1 => {
            let certificate = certify_compiled_mmio_predicate(image, symbols)?;
            Ok(CompiledMmioPredicateEvidence::PredicateV1(
                encode_compiled_mmio_predicate_certificate(&certificate)?,
            ))
        }
        CompiledMmioPredicateRoute::ExactV1 => Ok(CompiledMmioPredicateEvidence::ExactV1(
            build_exact_compiled_mmio_reference(image, symbols)?,
        )),
    }
}

pub fn verify_compiled_mmio_predicate_portfolio(
    preflight: &CompiledMmioPredicatePreflight,
    evidence: &CompiledMmioPredicateEvidence,
    image: &[u8],
    symbols: Rv32SymbolLayout,
) -> Result<CompiledMmioPredicatePortfolioVerification, CompiledMmioPredicatePortfolioError> {
    check_identity(preflight, image, symbols)?;
    match (preflight.route, evidence) {
        (
            CompiledMmioPredicateRoute::PredicateV1,
            CompiledMmioPredicateEvidence::PredicateV1(bytes),
        ) => {
            let CompiledMmioPredicateCertificateVerification {
                producer_decoded_transitions,
                producer_lane_value_operations,
                verifier_decoded_transitions,
                verifier_lane_value_operations,
                artifact_bytes,
            } = verify_compiled_mmio_predicate_bytes(bytes, image, symbols)?;
            let total_decoded_transitions = preflight
                .decoded_transitions
                .and_then(|work| work.checked_add(producer_decoded_transitions))
                .and_then(|work| work.checked_add(verifier_decoded_transitions))
                .ok_or_else(|| reject("complete decoded transition count overflow"))?;
            let total_lane_value_operations = preflight
                .lane_value_operations
                .and_then(|work| work.checked_add(producer_lane_value_operations))
                .and_then(|work| work.checked_add(verifier_lane_value_operations))
                .ok_or_else(|| reject("complete lane operation count overflow"))?;
            Ok(CompiledMmioPredicatePortfolioVerification {
                route: CompiledMmioPredicateRoute::PredicateV1,
                preflight_decoded_transitions: preflight.decoded_transitions,
                producer_decoded_transitions,
                verifier_decoded_transitions,
                total_decoded_transitions: Some(total_decoded_transitions),
                preflight_lane_value_operations: preflight.lane_value_operations,
                producer_lane_value_operations,
                verifier_lane_value_operations,
                total_lane_value_operations: Some(total_lane_value_operations),
                predicate_artifact_bytes: artifact_bytes,
            })
        }
        (
            CompiledMmioPredicateRoute::ExactV1,
            CompiledMmioPredicateEvidence::ExactV1(reference),
        ) => {
            verify_exact_compiled_mmio_reference(reference, image, symbols)?;
            Ok(CompiledMmioPredicatePortfolioVerification {
                route: CompiledMmioPredicateRoute::ExactV1,
                preflight_decoded_transitions: None,
                producer_decoded_transitions: reference.decoded_instruction_transitions,
                verifier_decoded_transitions: reference.decoded_instruction_transitions,
                total_decoded_transitions: None,
                preflight_lane_value_operations: None,
                producer_lane_value_operations: 0,
                verifier_lane_value_operations: 0,
                total_lane_value_operations: None,
                predicate_artifact_bytes: 0,
            })
        }
        _ => Err(reject(
            "evidence route differs from the predeclared portfolio route",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riscv32imc::RV32_IMAGE_BASE;

    fn guarded_image(threshold: u32) -> (Vec<u8>, Rv32SymbolLayout) {
        let mut image = vec![0; 0x110];
        let sltiu = (threshold << 20) | (10 << 15) | (3 << 12) | (10 << 7) | 0x13;
        let return_to_ra = (1u32 << 15) | 0x67;
        image[..4].copy_from_slice(&sltiu.to_le_bytes());
        image[4..8].copy_from_slice(&return_to_ra.to_le_bytes());
        (
            image,
            Rv32SymbolLayout {
                entry: RV32_IMAGE_BASE,
                event_count: RV32_IMAGE_BASE + 0x100,
                events: RV32_IMAGE_BASE + 0x104,
            },
        )
    }

    #[test]
    fn statically_selects_predicate_and_preserves_exact_fallback() {
        let (uniform_image, symbols) = guarded_image(6);
        let predicate = preflight_compiled_mmio_predicate(&uniform_image, symbols).unwrap();
        assert_eq!(predicate.route, CompiledMmioPredicateRoute::PredicateV1);
        let evidence =
            produce_compiled_mmio_predicate_portfolio(&predicate, &uniform_image, symbols).unwrap();
        let verified = verify_compiled_mmio_predicate_portfolio(
            &predicate,
            &evidence,
            &uniform_image,
            symbols,
        )
        .unwrap();
        assert_eq!(verified.route, CompiledMmioPredicateRoute::PredicateV1);
        assert_eq!(verified.preflight_decoded_transitions, Some(2));
        assert_eq!(verified.producer_decoded_transitions, 14);
        assert_eq!(verified.verifier_decoded_transitions, 14);
        assert_eq!(verified.total_decoded_transitions, Some(30));
        assert_eq!(verified.total_lane_value_operations, Some(1_500));

        let (divergent_image, symbols) = guarded_image(128);
        let exact = preflight_compiled_mmio_predicate(&divergent_image, symbols).unwrap();
        assert_eq!(exact.route, CompiledMmioPredicateRoute::ExactV1);
        let evidence =
            produce_compiled_mmio_predicate_portfolio(&exact, &divergent_image, symbols).unwrap();
        let verified =
            verify_compiled_mmio_predicate_portfolio(&exact, &evidence, &divergent_image, symbols)
                .unwrap();
        assert_eq!(verified.route, CompiledMmioPredicateRoute::ExactV1);
        assert_eq!(verified.preflight_decoded_transitions, None);
        assert_eq!(verified.total_decoded_transitions, None);
    }

    #[test]
    fn source_drift_and_forced_routes_fail_closed() {
        let (mut image, symbols) = guarded_image(6);
        let preflight = preflight_compiled_mmio_predicate(&image, symbols).unwrap();
        let evidence =
            produce_compiled_mmio_predicate_portfolio(&preflight, &image, symbols).unwrap();
        image[0] ^= 1;
        assert!(produce_compiled_mmio_predicate_portfolio(&preflight, &image, symbols).is_err());
        assert!(
            verify_compiled_mmio_predicate_portfolio(&preflight, &evidence, &image, symbols)
                .is_err()
        );

        let (image, symbols) = guarded_image(6);
        let mut forced = preflight_compiled_mmio_predicate(&image, symbols).unwrap();
        forced.route = CompiledMmioPredicateRoute::ExactV1;
        assert!(produce_compiled_mmio_predicate_portfolio(&forced, &image, symbols).is_err());
        let exact = CompiledMmioPredicateEvidence::ExactV1(
            build_exact_compiled_mmio_reference(&image, symbols).unwrap(),
        );
        let genuine = preflight_compiled_mmio_predicate(&image, symbols).unwrap();
        assert!(
            verify_compiled_mmio_predicate_portfolio(&genuine, &exact, &image, symbols).is_err()
        );
        let mut changed_work = genuine.clone();
        changed_work.decoded_transitions = Some(3);
        assert!(produce_compiled_mmio_predicate_portfolio(&changed_work, &image, symbols).is_err());
    }
}
