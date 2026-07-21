//! Proof-carrying batches of bounded BTOR2 predicates over one recurrence.

use crate::btor2::NodeId;
use crate::{btor2_bounded, btor2_region, btor2_search};
use std::error::Error;
use std::fmt;

pub const PREDICATE_SET_CERTIFICATE_VERSION: u32 = 1;
pub const PREDICATE_SET_PORTFOLIO_VERSION: u32 = 1;
pub const PREDICATE_SET_CLI_VERSION: u32 = 1;
pub const MAX_PREDICATE_SET_MEMBERS: usize = 64;
pub const MAX_PREDICATE_SET_CERTIFICATE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateMember {
    pub bad_property: NodeId,
    pub predicate: btor2_region::RegionPredicate,
    pub predicate_literal: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedRegionCertificate {
    pub source_sha256: String,
    pub query_horizon: u32,
    pub input: NodeId,
    pub state: NodeId,
    pub width: u32,
    pub family: btor2_region::RegionFamily,
    pub initial: u64,
    pub reset: u64,
    pub delta: u64,
    pub saturation: Option<u64>,
    pub max_index: u64,
    pub members: Vec<PredicateMember>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrdinaryPredicateSetCertificate {
    pub query_horizon: u32,
    pub members: Vec<btor2_bounded::BoundedCertificate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredicateSetCertificate {
    SharedRegion(SharedRegionCertificate),
    Ordinary(OrdinaryPredicateSetCertificate),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateSetRoute {
    SharedRegion,
    OrdinaryExact,
}

impl PredicateSetRoute {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SharedRegion => "shared-region",
            Self::OrdinaryExact => "ordinary-exact",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateSetSelectionReason {
    SharedEvidenceSmaller,
    SingletonUnsupportedOrIntersecting,
}

impl PredicateSetSelectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SharedEvidenceSmaller => "shared-evidence-smaller",
            Self::SingletonUnsupportedOrIntersecting => "singleton-unsupported-or-intersecting",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSetProduction {
    pub certificate: PredicateSetCertificate,
    pub selection_reason: PredicateSetSelectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSetMemberSummary {
    pub bad_property: NodeId,
    pub backend: btor2_bounded::BoundedBackend,
    pub result: btor2_search::SearchResult,
    pub bad_frame: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSetSummary {
    pub route: PredicateSetRoute,
    pub query_horizon: u32,
    pub members: Vec<PredicateSetMemberSummary>,
    pub safe: usize,
    pub unsafe_count: usize,
    pub logical_reachable_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSetError(pub String);

impl fmt::Display for PredicateSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for PredicateSetError {}

fn reject(message: impl Into<String>) -> PredicateSetError {
    PredicateSetError(message.into())
}

fn valid_properties(properties: &[NodeId]) -> bool {
    !properties.is_empty()
        && properties.len() <= MAX_PREDICATE_SET_MEMBERS
        && properties.windows(2).all(|pair| pair[0] < pair[1])
}

fn bounded_bad_property(certificate: &btor2_bounded::BoundedCertificate) -> NodeId {
    match certificate {
        btor2_bounded::BoundedCertificate::BrakingPhases(value) => value.bad_property,
        btor2_bounded::BoundedCertificate::MotionCurve(value) => value.bad_property,
        btor2_bounded::BoundedCertificate::WordRegion(value) => value.bad_property,
        btor2_bounded::BoundedCertificate::ExplicitSearch(value) => value.bad_property,
    }
}

fn bounded_horizon(certificate: &btor2_bounded::BoundedCertificate) -> u32 {
    match certificate {
        btor2_bounded::BoundedCertificate::BrakingPhases(value) => value.query_horizon,
        btor2_bounded::BoundedCertificate::MotionCurve(value) => value.query_horizon,
        btor2_bounded::BoundedCertificate::WordRegion(value) => value.query_horizon,
        btor2_bounded::BoundedCertificate::ExplicitSearch(value) => value.query_horizon,
    }
}

fn shared_candidate(
    source: &[u8],
    properties: &[NodeId],
    horizon: u32,
) -> Result<Option<(SharedRegionCertificate, usize)>, PredicateSetError> {
    if properties.len() < 2 {
        return Ok(None);
    }
    let Some(regions) = btor2_region::try_produce_safe_set(source, properties, horizon)
        .map_err(|error| reject(format!("word-region backend error: {error}")))?
    else {
        return Ok(None);
    };
    let mut separate_bytes = 0usize;
    for region in &regions {
        separate_bytes = separate_bytes
            .checked_add(
                btor2_region::encode(&region)
                    .map_err(|error| reject(error.to_string()))?
                    .len(),
            )
            .ok_or_else(|| reject("separate evidence byte count overflowed"))?;
    }
    let first = &regions[0];
    if regions.iter().any(|region| {
        region.source_sha256 != first.source_sha256
            || region.query_horizon != first.query_horizon
            || region.input != first.input
            || region.state != first.state
            || region.width != first.width
            || region.family != first.family
            || region.initial != first.initial
            || region.reset != first.reset
            || region.delta != first.delta
            || region.saturation != first.saturation
            || region.max_index != first.max_index
    }) {
        return Ok(None);
    }
    let shared = SharedRegionCertificate {
        source_sha256: first.source_sha256.clone(),
        query_horizon: first.query_horizon,
        input: first.input,
        state: first.state,
        width: first.width,
        family: first.family,
        initial: first.initial,
        reset: first.reset,
        delta: first.delta,
        saturation: first.saturation,
        max_index: first.max_index,
        members: regions
            .into_iter()
            .map(|region| PredicateMember {
                bad_property: region.bad_property,
                predicate: region.predicate,
                predicate_literal: region.predicate_literal,
            })
            .collect(),
    };
    let shared_bytes = encode_shared(&shared)?.len();
    if shared_bytes < separate_bytes {
        Ok(Some((shared, separate_bytes)))
    } else {
        Ok(None)
    }
}

pub fn produce(
    source: &[u8],
    properties: &[NodeId],
    horizon: u32,
) -> Result<PredicateSetProduction, PredicateSetError> {
    if !valid_properties(properties) {
        return Err(reject(
            "predicate set must contain 1..=64 strictly increasing property identifiers",
        ));
    }
    if let Some((certificate, _)) = shared_candidate(source, properties, horizon)? {
        return Ok(PredicateSetProduction {
            certificate: PredicateSetCertificate::SharedRegion(certificate),
            selection_reason: PredicateSetSelectionReason::SharedEvidenceSmaller,
        });
    }
    let members = properties
        .iter()
        .map(|property| {
            btor2_bounded::produce(source, *property, horizon)
                .map_err(|error| reject(format!("exact member {property} failed: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PredicateSetProduction {
        certificate: PredicateSetCertificate::Ordinary(OrdinaryPredicateSetCertificate {
            query_horizon: horizon,
            members,
        }),
        selection_reason: PredicateSetSelectionReason::SingletonUnsupportedOrIntersecting,
    })
}

fn shared_region_member(
    shared: &SharedRegionCertificate,
    member: &PredicateMember,
) -> btor2_region::RegionCertificate {
    btor2_region::RegionCertificate {
        source_sha256: shared.source_sha256.clone(),
        query_horizon: shared.query_horizon,
        bad_property: member.bad_property,
        input: shared.input,
        state: shared.state,
        width: shared.width,
        family: shared.family,
        initial: shared.initial,
        reset: shared.reset,
        delta: shared.delta,
        saturation: shared.saturation,
        predicate: member.predicate,
        predicate_literal: member.predicate_literal,
        max_index: shared.max_index,
    }
}

pub fn verify(
    source: &[u8],
    properties: &[NodeId],
    horizon: u32,
    certificate: &PredicateSetCertificate,
) -> Result<PredicateSetSummary, PredicateSetError> {
    if !valid_properties(properties) {
        return Err(reject(
            "predicate set must contain 1..=64 strictly increasing property identifiers",
        ));
    }
    match certificate {
        PredicateSetCertificate::SharedRegion(shared) => {
            if shared.query_horizon != horizon
                || shared.members.len() != properties.len()
                || shared
                    .members
                    .iter()
                    .map(|member| member.bad_property)
                    .ne(properties.iter().copied())
            {
                return Err(reject("shared certificate query binding mismatch"));
            }
            let Some((expected, _)) = shared_candidate(source, properties, horizon)? else {
                return Err(reject("shared certificate violates static selection gate"));
            };
            if &expected != shared {
                return Err(reject(
                    "shared certificate is not the canonical source claim",
                ));
            }
            let obligations = shared
                .members
                .iter()
                .map(|member| shared_region_member(shared, member))
                .collect::<Vec<_>>();
            let region_summary = btor2_region::verify_set(source, &obligations)
                .map_err(|error| reject(format!("shared member verification failed: {error}")))?;
            Ok(PredicateSetSummary {
                route: PredicateSetRoute::SharedRegion,
                query_horizon: horizon,
                members: properties
                    .iter()
                    .map(|property| PredicateSetMemberSummary {
                        bad_property: *property,
                        backend: btor2_bounded::BoundedBackend::WordRegion,
                        result: btor2_search::SearchResult::Safe,
                        bad_frame: None,
                    })
                    .collect(),
                safe: properties.len(),
                unsafe_count: 0,
                logical_reachable_states: region_summary.logical_reachable_states,
            })
        }
        PredicateSetCertificate::Ordinary(ordinary) => {
            if ordinary.query_horizon != horizon || ordinary.members.len() != properties.len() {
                return Err(reject("ordinary certificate query binding mismatch"));
            }
            if shared_candidate(source, properties, horizon)?.is_some() {
                return Err(reject(
                    "ordinary certificate violates static selection gate",
                ));
            }
            let mut summaries = Vec::with_capacity(properties.len());
            let mut safe = 0usize;
            let mut unsafe_count = 0usize;
            let mut logical_reachable_states = 0u64;
            for (property, certificate) in properties.iter().zip(&ordinary.members) {
                if bounded_bad_property(certificate) != *property
                    || bounded_horizon(certificate) != horizon
                {
                    return Err(reject("ordinary member query binding mismatch"));
                }
                let summary = btor2_bounded::verify(source, certificate)
                    .map_err(|error| reject(format!("exact member {property} failed: {error}")))?;
                logical_reachable_states = logical_reachable_states
                    .checked_add(summary.logical_reachable_states)
                    .ok_or_else(|| reject("ordinary logical state count overflowed"))?;
                match summary.result {
                    btor2_search::SearchResult::Safe => safe += 1,
                    btor2_search::SearchResult::Unsafe => unsafe_count += 1,
                }
                summaries.push(PredicateSetMemberSummary {
                    bad_property: *property,
                    backend: summary.backend,
                    result: summary.result,
                    bad_frame: summary.bad_frame,
                });
            }
            Ok(PredicateSetSummary {
                route: PredicateSetRoute::OrdinaryExact,
                query_horizon: horizon,
                members: summaries,
                safe,
                unsafe_count,
                logical_reachable_states,
            })
        }
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn family_name(value: btor2_region::RegionFamily) -> &'static str {
    match value {
        btor2_region::RegionFamily::ResetAdd => "reset_add",
        btor2_region::RegionFamily::ResetSaturatingAdd => "reset_saturating_add",
    }
}

fn predicate_name(value: btor2_region::RegionPredicate) -> &'static str {
    match value {
        btor2_region::RegionPredicate::Equal => "eq",
        btor2_region::RegionPredicate::UnsignedGreaterEqual => "ugte",
    }
}

fn encode_shared(certificate: &SharedRegionCertificate) -> Result<String, PredicateSetError> {
    if !valid_digest(&certificate.source_sha256)
        || certificate.members.len() < 2
        || certificate.members.len() > MAX_PREDICATE_SET_MEMBERS
        || !certificate
            .members
            .windows(2)
            .all(|pair| pair[0].bad_property < pair[1].bad_property)
    {
        return Err(reject("shared predicate-set certificate is not canonical"));
    }
    let saturation = certificate
        .saturation
        .map_or_else(|| "none".to_string(), |value| value.to_string());
    let mut text = format!(
        "predicate_set_certificate_version={PREDICATE_SET_CERTIFICATE_VERSION}\nroute=shared_region\nsource_sha256={}\nquery_horizon={}\ninput={}\nstate={}\nwidth={}\nfamily={}\ninitial={}\nreset={}\ndelta={}\nsaturation={saturation}\nmax_index={}\nmember_count={}\n",
        certificate.source_sha256,
        certificate.query_horizon,
        certificate.input,
        certificate.state,
        certificate.width,
        family_name(certificate.family),
        certificate.initial,
        certificate.reset,
        certificate.delta,
        certificate.max_index,
        certificate.members.len(),
    );
    for member in &certificate.members {
        text.push_str(&format!(
            "member={}:{}:{}\n",
            member.bad_property,
            predicate_name(member.predicate),
            member.predicate_literal
        ));
    }
    text.push_str("result=SAFE\nstatus=complete\n");
    if text.len() > MAX_PREDICATE_SET_CERTIFICATE_BYTES {
        return Err(reject("predicate-set certificate exceeds byte limit"));
    }
    Ok(text)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> Result<Vec<u8>, PredicateSetError> {
    if !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(reject("ordinary member hex is not canonical"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| reject("invalid member hex"))?;
            u8::from_str_radix(text, 16).map_err(|_| reject("invalid member hex"))
        })
        .collect()
}

pub fn encode(certificate: &PredicateSetCertificate) -> Result<String, PredicateSetError> {
    match certificate {
        PredicateSetCertificate::SharedRegion(shared) => encode_shared(shared),
        PredicateSetCertificate::Ordinary(ordinary) => {
            if ordinary.members.is_empty()
                || ordinary.members.len() > MAX_PREDICATE_SET_MEMBERS
                || ordinary
                    .members
                    .iter()
                    .any(|member| bounded_horizon(member) != ordinary.query_horizon)
                || !ordinary
                    .members
                    .windows(2)
                    .all(|pair| bounded_bad_property(&pair[0]) < bounded_bad_property(&pair[1]))
            {
                return Err(reject(
                    "ordinary predicate-set certificate is not canonical",
                ));
            }
            let mut text = format!(
                "predicate_set_certificate_version={PREDICATE_SET_CERTIFICATE_VERSION}\nroute=ordinary_exact\nquery_horizon={}\nmember_count={}\n",
                ordinary.query_horizon,
                ordinary.members.len()
            );
            for member in &ordinary.members {
                let encoded = btor2_bounded::encode(member)
                    .map_err(|error| reject(format!("ordinary member encoding failed: {error}")))?;
                text.push_str("certificate_hex=");
                text.push_str(&hex_encode(encoded.as_bytes()));
                text.push('\n');
                if text.len() > MAX_PREDICATE_SET_CERTIFICATE_BYTES {
                    return Err(reject("predicate-set certificate exceeds byte limit"));
                }
            }
            text.push_str("status=complete\n");
            if text.len() > MAX_PREDICATE_SET_CERTIFICATE_BYTES {
                return Err(reject("predicate-set certificate exceeds byte limit"));
            }
            Ok(text)
        }
    }
}

fn canonical_text(bytes: &[u8]) -> Result<&str, PredicateSetError> {
    if bytes.len() > MAX_PREDICATE_SET_CERTIFICATE_BYTES {
        return Err(reject("predicate-set certificate exceeds byte limit"));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| reject("predicate-set certificate is not UTF-8"))?;
    if bytes.contains(&0) || text.contains('\r') || !text.ends_with('\n') {
        return Err(reject(
            "predicate-set certificate must be canonical LF text without NUL",
        ));
    }
    Ok(text)
}

fn take<'a>(lines: &mut std::str::Lines<'a>, key: &str) -> Result<&'a str, PredicateSetError> {
    lines
        .next()
        .and_then(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| reject(format!("expected {key}")))
}

fn number<T: std::str::FromStr + fmt::Display>(
    value: &str,
    key: &str,
) -> Result<T, PredicateSetError> {
    let parsed = value
        .parse::<T>()
        .map_err(|_| reject(format!("invalid {key}")))?;
    if parsed.to_string() != value {
        return Err(reject(format!("noncanonical {key}")));
    }
    Ok(parsed)
}

pub fn decode(bytes: &[u8]) -> Result<PredicateSetCertificate, PredicateSetError> {
    let text = canonical_text(bytes)?;
    let mut lines = text.lines();
    let version: u32 = number(
        take(&mut lines, "predicate_set_certificate_version")?,
        "predicate set certificate version",
    )?;
    if version != PREDICATE_SET_CERTIFICATE_VERSION {
        return Err(reject("unsupported predicate-set certificate version"));
    }
    let certificate =
        match take(&mut lines, "route")? {
            "shared_region" => {
                let source_sha256 = take(&mut lines, "source_sha256")?.to_string();
                if !valid_digest(&source_sha256) {
                    return Err(reject("shared source digest is not canonical"));
                }
                let query_horizon = number(take(&mut lines, "query_horizon")?, "query horizon")?;
                let input = number(take(&mut lines, "input")?, "input")?;
                let state = number(take(&mut lines, "state")?, "state")?;
                let width = number(take(&mut lines, "width")?, "width")?;
                let family = match take(&mut lines, "family")? {
                    "reset_add" => btor2_region::RegionFamily::ResetAdd,
                    "reset_saturating_add" => btor2_region::RegionFamily::ResetSaturatingAdd,
                    _ => return Err(reject("unknown shared recurrence family")),
                };
                let initial = number(take(&mut lines, "initial")?, "initial")?;
                let reset = number(take(&mut lines, "reset")?, "reset")?;
                let delta = number(take(&mut lines, "delta")?, "delta")?;
                let saturation = match take(&mut lines, "saturation")? {
                    "none" => None,
                    value => Some(number(value, "saturation")?),
                };
                let max_index = number(take(&mut lines, "max_index")?, "max index")?;
                let count: usize = number(take(&mut lines, "member_count")?, "member count")?;
                if !(2..=MAX_PREDICATE_SET_MEMBERS).contains(&count) {
                    return Err(reject("shared member count is outside limit"));
                }
                let mut members = Vec::with_capacity(count);
                for _ in 0..count {
                    let fields = take(&mut lines, "member")?.split(':').collect::<Vec<_>>();
                    if fields.len() != 3 {
                        return Err(reject("shared member field is malformed"));
                    }
                    members.push(PredicateMember {
                        bad_property: number(fields[0], "bad property")?,
                        predicate: match fields[1] {
                            "eq" => btor2_region::RegionPredicate::Equal,
                            "ugte" => btor2_region::RegionPredicate::UnsignedGreaterEqual,
                            _ => return Err(reject("unknown shared predicate")),
                        },
                        predicate_literal: number(fields[2], "predicate literal")?,
                    });
                }
                if take(&mut lines, "result")? != "SAFE" {
                    return Err(reject("shared result must be SAFE"));
                }
                PredicateSetCertificate::SharedRegion(SharedRegionCertificate {
                    source_sha256,
                    query_horizon,
                    input,
                    state,
                    width,
                    family,
                    initial,
                    reset,
                    delta,
                    saturation,
                    max_index,
                    members,
                })
            }
            "ordinary_exact" => {
                let query_horizon = number(take(&mut lines, "query_horizon")?, "query horizon")?;
                let count: usize = number(take(&mut lines, "member_count")?, "member count")?;
                if !(1..=MAX_PREDICATE_SET_MEMBERS).contains(&count) {
                    return Err(reject("ordinary member count is outside limit"));
                }
                let mut members = Vec::with_capacity(count);
                for _ in 0..count {
                    let decoded = hex_decode(take(&mut lines, "certificate_hex")?)?;
                    if decoded.len() > btor2_search::MAX_SEARCH_CERTIFICATE_BYTES {
                        return Err(reject("ordinary member certificate exceeds byte limit"));
                    }
                    members.push(btor2_bounded::decode(&decoded).map_err(|error| {
                        reject(format!("ordinary member decode failed: {error}"))
                    })?);
                }
                PredicateSetCertificate::Ordinary(OrdinaryPredicateSetCertificate {
                    query_horizon,
                    members,
                })
            }
            _ => return Err(reject("unknown predicate-set route")),
        };
    if take(&mut lines, "status")? != "complete" || lines.next().is_some() {
        return Err(reject(
            "predicate-set certificate is incomplete or has trailing fields",
        ));
    }
    if encode(&certificate)? != text {
        return Err(reject("predicate-set certificate is not canonical"));
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_PREDICATES: &[u8] = b"1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 reset\n4 zero 2\n5 state 2 count\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n11 constd 2 5\n12 ugte 1 5 11\n13 bad 12 bark\n14 constd 2 9\n15 ugte 1 5 14\n16 bad 15 bite\n";

    #[test]
    fn shares_one_recurrence_for_two_safe_predicates() {
        let production = produce(TWO_PREDICATES, &[13, 16], 4).unwrap();
        assert_eq!(
            production.selection_reason,
            PredicateSetSelectionReason::SharedEvidenceSmaller
        );
        let encoded = encode(&production.certificate).unwrap();
        let separate = [13, 16]
            .iter()
            .map(|property| {
                btor2_region::encode(
                    &btor2_region::try_produce_safe(TWO_PREDICATES, *property, 4)
                        .unwrap()
                        .unwrap(),
                )
                .unwrap()
                .len()
            })
            .sum::<usize>();
        assert!(encoded.len() < separate);
        let decoded = decode(encoded.as_bytes()).unwrap();
        let summary = verify(TWO_PREDICATES, &[13, 16], 4, &decoded).unwrap();
        assert_eq!(summary.route, PredicateSetRoute::SharedRegion);
        assert_eq!(summary.safe, 2);
        assert_eq!(summary.unsafe_count, 0);
    }

    #[test]
    fn mixed_answers_preserve_every_query_through_exact_fallback() {
        let production = produce(TWO_PREDICATES, &[13, 16], 5).unwrap();
        assert_eq!(
            production.selection_reason,
            PredicateSetSelectionReason::SingletonUnsupportedOrIntersecting
        );
        let encoded = encode(&production.certificate).unwrap();
        let decoded = decode(encoded.as_bytes()).unwrap();
        let summary = verify(TWO_PREDICATES, &[13, 16], 5, &decoded).unwrap();
        assert_eq!(summary.route, PredicateSetRoute::OrdinaryExact);
        assert_eq!(summary.safe, 1);
        assert_eq!(summary.unsafe_count, 1);
        assert_eq!(summary.members[0].bad_frame, Some(5));
        assert_eq!(summary.members[1].bad_frame, None);
    }

    #[test]
    fn rejects_query_omission_reordering_and_forced_downgrade() {
        let shared = produce(TWO_PREDICATES, &[13, 16], 4).unwrap().certificate;
        assert!(verify(TWO_PREDICATES, &[13], 4, &shared).is_err());
        assert!(verify(TWO_PREDICATES, &[16, 13], 4, &shared).is_err());

        let ordinary = PredicateSetCertificate::Ordinary(OrdinaryPredicateSetCertificate {
            query_horizon: 4,
            members: [13, 16]
                .iter()
                .map(|property| btor2_bounded::produce(TWO_PREDICATES, *property, 4).unwrap())
                .collect(),
        });
        assert!(verify(TWO_PREDICATES, &[13, 16], 4, &ordinary).is_err());
    }

    #[test]
    fn mutations_and_truncations_fail_closed() {
        let encoded = encode(&produce(TWO_PREDICATES, &[13, 16], 4).unwrap().certificate)
            .unwrap()
            .into_bytes();
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        for index in 0..encoded.len() {
            let mut mutated = encoded.clone();
            mutated[index] = if mutated[index] == b'!' { b'?' } else { b'!' };
            if let Ok(certificate) = decode(&mutated) {
                assert!(verify(TWO_PREDICATES, &[13, 16], 4, &certificate).is_err());
            }
        }
    }

    #[test]
    fn admits_the_member_limit_and_rejects_work_beyond_it() {
        let mut source = String::from(
            "1 sort bitvec 1\n2 sort bitvec 8\n3 input 1 reset\n4 zero 2\n5 state 2 count\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n",
        );
        let mut properties = Vec::new();
        let mut id = 11u64;
        for literal in 100..164 {
            source.push_str(&format!(
                "{id} constd 2 {literal}\n{} ugte 1 5 {id}\n{} bad {} property_{literal}\n",
                id + 1,
                id + 2,
                id + 1
            ));
            properties.push(id + 2);
            id += 3;
        }
        let production = produce(source.as_bytes(), &properties, 10).unwrap();
        let summary = verify(source.as_bytes(), &properties, 10, &production.certificate).unwrap();
        assert_eq!(summary.safe, MAX_PREDICATE_SET_MEMBERS);
        properties.push(id);
        assert!(produce(source.as_bytes(), &properties, 10).is_err());
    }

    #[test]
    fn returns_no_partial_batch_when_exact_fallback_exceeds_its_bound() {
        let error = produce(TWO_PREDICATES, &[13, 16], 1_000_000_000).unwrap_err();
        assert!(error.to_string().contains("exact member 13 failed"));
    }
}
