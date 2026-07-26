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
    pub version: u32,
    pub image_bytes: u32,
    pub image_sha256: [u8; 32],
    pub symbols: Rv32SymbolLayout,
    pub route: CompiledMmioPredicateRoute,
    pub refusal: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledMmioPredicateEvidence {
    PredicateV1(Vec<u8>),
    ExactV1(ExactCompiledMmioReference),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMmioPredicatePortfolioVerification {
    pub route: CompiledMmioPredicateRoute,
    pub decoded_transitions: u64,
    pub lane_value_operations: u64,
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
    match (preflight.route, &preflight.refusal) {
        (CompiledMmioPredicateRoute::PredicateV1, None)
        | (CompiledMmioPredicateRoute::ExactV1, Some(_)) => Ok(()),
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
    let (route, refusal) = match execute_invalid_channel_predicate(image, symbols) {
        Ok(_) => (CompiledMmioPredicateRoute::PredicateV1, None),
        Err(error) => (CompiledMmioPredicateRoute::ExactV1, Some(error.to_string())),
    };
    Ok(CompiledMmioPredicatePreflight {
        version: COMPILED_MMIO_PREDICATE_PORTFOLIO_VERSION,
        image_bytes,
        image_sha256: digest(image),
        symbols,
        route,
        refusal,
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
                decoded_transitions,
                lane_value_operations,
                artifact_bytes,
            } = verify_compiled_mmio_predicate_bytes(bytes, image, symbols)?;
            Ok(CompiledMmioPredicatePortfolioVerification {
                route: CompiledMmioPredicateRoute::PredicateV1,
                decoded_transitions,
                lane_value_operations,
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
                decoded_transitions: reference.decoded_instruction_transitions,
                lane_value_operations: 0,
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

        let (divergent_image, symbols) = guarded_image(128);
        let exact = preflight_compiled_mmio_predicate(&divergent_image, symbols).unwrap();
        assert_eq!(exact.route, CompiledMmioPredicateRoute::ExactV1);
        let evidence =
            produce_compiled_mmio_predicate_portfolio(&exact, &divergent_image, symbols).unwrap();
        let verified =
            verify_compiled_mmio_predicate_portfolio(&exact, &evidence, &divergent_image, symbols)
                .unwrap();
        assert_eq!(verified.route, CompiledMmioPredicateRoute::ExactV1);
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
    }
}
