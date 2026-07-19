//! Static exact portfolio for bounded BTOR2 reachability.

use crate::btor2::NodeId;
use crate::{btor2_motion, btor2_region, btor2_search};
use std::error::Error;
use std::fmt;

pub const BOUNDED_PORTFOLIO_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedBackend {
    MotionCurve,
    WordRegion,
    ExplicitSearch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundedCertificate {
    MotionCurve(btor2_motion::MotionCertificate),
    WordRegion(btor2_region::RegionCertificate),
    ExplicitSearch(btor2_search::SearchCertificate),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedSelectionReason {
    MotionCurveExactSafe,
    WordRegionExactSafe,
    SpecialisedInapplicableOrIntersecting,
}

impl BoundedSelectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MotionCurveExactSafe => "motion-curve-exact-safe",
            Self::WordRegionExactSafe => "word-region-exact-safe",
            Self::SpecialisedInapplicableOrIntersecting => {
                "specialised-inapplicable-or-intersecting"
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedProduction {
    pub certificate: BoundedCertificate,
    pub selection_reason: BoundedSelectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedSummary {
    pub backend: BoundedBackend,
    pub result: btor2_search::SearchResult,
    pub query_horizon: u32,
    pub bad_frame: Option<u32>,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedError(pub String);

impl fmt::Display for BoundedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for BoundedError {}

fn reject(message: impl Into<String>) -> BoundedError {
    BoundedError(message.into())
}

/// Applies the versioned source-structural portfolio with no timing or
/// per-formula training: coupled motion, then one-state word region, then the
/// unchanged explicit exact query.
pub fn produce(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<BoundedCertificate, BoundedError> {
    produce_with_observation(source, bad_property, horizon).map(|production| production.certificate)
}

pub fn produce_with_observation(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<BoundedProduction, BoundedError> {
    if let Some(certificate) = btor2_motion::try_produce_safe(source, bad_property, horizon)
        .map_err(|error| reject(format!("motion backend error: {error}")))?
    {
        return Ok(BoundedProduction {
            certificate: BoundedCertificate::MotionCurve(certificate),
            selection_reason: BoundedSelectionReason::MotionCurveExactSafe,
        });
    }
    if let Some(certificate) = btor2_region::try_produce_safe(source, bad_property, horizon)
        .map_err(|error| reject(format!("word-region backend error: {error}")))?
    {
        return Ok(BoundedProduction {
            certificate: BoundedCertificate::WordRegion(certificate),
            selection_reason: BoundedSelectionReason::WordRegionExactSafe,
        });
    }
    btor2_search::produce(source, bad_property, horizon)
        .map(|certificate| BoundedProduction {
            certificate: BoundedCertificate::ExplicitSearch(certificate),
            selection_reason: BoundedSelectionReason::SpecialisedInapplicableOrIntersecting,
        })
        .map_err(|error| reject(format!("explicit search fallback error: {error}")))
}

pub fn verify(
    source: &[u8],
    certificate: &BoundedCertificate,
) -> Result<BoundedSummary, BoundedError> {
    match certificate {
        BoundedCertificate::MotionCurve(certificate) => {
            let summary = btor2_motion::verify(source, certificate)
                .map_err(|error| reject(error.to_string()))?;
            Ok(BoundedSummary {
                backend: BoundedBackend::MotionCurve,
                result: btor2_search::SearchResult::Safe,
                query_horizon: summary.query_horizon,
                bad_frame: None,
                logical_reachable_states: summary.logical_reachable_states,
            })
        }
        BoundedCertificate::WordRegion(certificate) => {
            let summary = btor2_region::verify(source, certificate)
                .map_err(|error| reject(error.to_string()))?;
            Ok(BoundedSummary {
                backend: BoundedBackend::WordRegion,
                result: btor2_search::SearchResult::Safe,
                query_horizon: summary.query_horizon,
                bad_frame: None,
                logical_reachable_states: summary.logical_reachable_states,
            })
        }
        BoundedCertificate::ExplicitSearch(certificate) => {
            let summary = btor2_search::verify(source, certificate)
                .map_err(|error| reject(error.to_string()))?;
            Ok(BoundedSummary {
                backend: BoundedBackend::ExplicitSearch,
                result: summary.result,
                query_horizon: summary.query_horizon,
                bad_frame: summary.bad_frame,
                logical_reachable_states: summary.reachable_states as u64,
            })
        }
    }
}

pub fn encode(certificate: &BoundedCertificate) -> Result<String, BoundedError> {
    match certificate {
        BoundedCertificate::MotionCurve(certificate) => {
            btor2_motion::encode(certificate).map_err(|error| reject(error.to_string()))
        }
        BoundedCertificate::WordRegion(certificate) => {
            btor2_region::encode(certificate).map_err(|error| reject(error.to_string()))
        }
        BoundedCertificate::ExplicitSearch(certificate) => {
            btor2_search::encode(certificate).map_err(|error| reject(error.to_string()))
        }
    }
}

pub fn decode(bytes: &[u8]) -> Result<BoundedCertificate, BoundedError> {
    if bytes.starts_with(b"motion_certificate_version=") {
        return btor2_motion::decode(bytes)
            .map(BoundedCertificate::MotionCurve)
            .map_err(|error| reject(error.to_string()));
    }
    if bytes.starts_with(b"region_certificate_version=") {
        return btor2_region::decode(bytes)
            .map(BoundedCertificate::WordRegion)
            .map_err(|error| reject(error.to_string()));
    }
    if bytes.starts_with(b"search_certificate_version=") {
        return btor2_search::decode(bytes)
            .map(BoundedCertificate::ExplicitSearch)
            .map_err(|error| reject(error.to_string()));
    }
    Err(reject("unknown bounded BTOR2 certificate format"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATCHDOG: &[u8] = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
    const MOTION: &[u8] = include_bytes!("../examples/btor2/motion-envelope-v1.btor2");
    const SEMI_IMPLICIT: &[u8] =
        include_bytes!("../examples/btor2/semi-implicit-motion-rejected-v1.btor2");

    #[test]
    fn statically_selects_region_for_safe_and_exact_search_for_unsafe() {
        let production = produce_with_observation(WATCHDOG, 13, 2).unwrap();
        assert_eq!(
            production.selection_reason,
            BoundedSelectionReason::WordRegionExactSafe
        );
        let safe = production.certificate;
        assert!(matches!(safe, BoundedCertificate::WordRegion(_)));
        assert_eq!(
            verify(WATCHDOG, &safe).unwrap().result,
            btor2_search::SearchResult::Safe
        );

        let unsafe_certificate = produce(WATCHDOG, 13, 3).unwrap();
        assert!(matches!(
            unsafe_certificate,
            BoundedCertificate::ExplicitSearch(_)
        ));
        assert_eq!(
            verify(WATCHDOG, &unsafe_certificate).unwrap().result,
            btor2_search::SearchResult::Unsafe
        );
    }

    #[test]
    fn decodes_only_known_self_identifying_formats() {
        let certificate = produce(WATCHDOG, 13, 2).unwrap();
        let text = encode(&certificate).unwrap();
        assert_eq!(decode(text.as_bytes()).unwrap(), certificate);
        assert!(decode(b"certificate_version=1\n").is_err());
    }

    #[test]
    fn unsupported_region_shape_preserves_the_query_through_exact_search() {
        let source = b"1 sort bitvec 1\n2 sort bitvec 4\n3 input 1 reset\n4 one 2\n5 state 2 value\n6 init 2 5 4\n7 constd 2 2\n8 mul 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n11 constd 2 8\n12 eq 1 5 11\n13 bad 12 target\n";
        let certificate = produce(source, 13, 3).unwrap();
        assert!(matches!(certificate, BoundedCertificate::ExplicitSearch(_)));
        let summary = verify(source, &certificate).unwrap();
        assert_eq!(summary.result, btor2_search::SearchResult::Unsafe);
        assert_eq!(summary.bad_frame, Some(3));
    }

    #[test]
    fn selects_coupled_motion_without_changing_unsafe_fallback() {
        let production = produce_with_observation(MOTION, 21, 200).unwrap();
        assert_eq!(
            production.selection_reason,
            BoundedSelectionReason::MotionCurveExactSafe
        );
        let safe = production.certificate;
        assert!(matches!(safe, BoundedCertificate::MotionCurve(_)));
        assert_eq!(
            verify(MOTION, &safe).unwrap().result,
            btor2_search::SearchResult::Safe
        );

        let production = produce_with_observation(MOTION, 21, 201).unwrap();
        assert_eq!(
            production.selection_reason,
            BoundedSelectionReason::SpecialisedInapplicableOrIntersecting
        );
        let unsafe_certificate = production.certificate;
        assert!(matches!(
            unsafe_certificate,
            BoundedCertificate::ExplicitSearch(_)
        ));
        assert_eq!(
            verify(MOTION, &unsafe_certificate).unwrap().bad_frame,
            Some(201)
        );
    }

    #[test]
    fn semi_implicit_near_neighbour_uses_exact_search_for_both_answers() {
        for (horizon, result) in [
            (3, btor2_search::SearchResult::Safe),
            (4, btor2_search::SearchResult::Unsafe),
        ] {
            let certificate = produce(SEMI_IMPLICIT, 21, horizon).unwrap();
            assert!(matches!(certificate, BoundedCertificate::ExplicitSearch(_)));
            assert_eq!(verify(SEMI_IMPLICIT, &certificate).unwrap().result, result);
        }
    }
}
