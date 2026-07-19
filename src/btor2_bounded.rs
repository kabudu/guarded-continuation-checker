//! Static exact portfolio for bounded BTOR2 reachability.

use crate::btor2::NodeId;
use crate::{btor2_region, btor2_search};
use std::error::Error;
use std::fmt;

pub const BOUNDED_PORTFOLIO_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedBackend {
    WordRegion,
    ExplicitSearch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundedCertificate {
    WordRegion(btor2_region::RegionCertificate),
    ExplicitSearch(btor2_search::SearchCertificate),
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

/// Applies one source-structural rule with no timing or per-formula training:
/// an exact word-region SAFE proof is preferred, otherwise the unchanged query
/// is sent to the explicit exact backend.
pub fn produce(
    source: &[u8],
    bad_property: NodeId,
    horizon: u32,
) -> Result<BoundedCertificate, BoundedError> {
    if let Some(certificate) = btor2_region::try_produce_safe(source, bad_property, horizon)
        .map_err(|error| reject(error.to_string()))?
    {
        return Ok(BoundedCertificate::WordRegion(certificate));
    }
    btor2_search::produce(source, bad_property, horizon)
        .map(BoundedCertificate::ExplicitSearch)
        .map_err(|error| reject(error.to_string()))
}

pub fn verify(
    source: &[u8],
    certificate: &BoundedCertificate,
) -> Result<BoundedSummary, BoundedError> {
    match certificate {
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
        BoundedCertificate::WordRegion(certificate) => {
            btor2_region::encode(certificate).map_err(|error| reject(error.to_string()))
        }
        BoundedCertificate::ExplicitSearch(certificate) => {
            btor2_search::encode(certificate).map_err(|error| reject(error.to_string()))
        }
    }
}

pub fn decode(bytes: &[u8]) -> Result<BoundedCertificate, BoundedError> {
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

    #[test]
    fn statically_selects_region_for_safe_and_exact_search_for_unsafe() {
        let safe = produce(WATCHDOG, 13, 2).unwrap();
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
}
