//! Canonical bounded certificates for revision-impact counterfactuals.
//!
//! This module deliberately separates certificate mechanics from model
//! semantics. [`verify_revision_impact_with`] requires an independent evaluator
//! for every admitted counterfactual observation.

use crate::revision_local::{
    BoundedQuery, BoundedResult, RevisionLocalCertificate, decode_bounded_answer_certificate,
    decode_revision_local_certificate, encode_revision_local_certificate,
    produce_revision_local_certificate, source_digest, verify_revision_local_certificate,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const REVISION_IMPACT_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_IMPACT_ATOMS: usize = 8;
pub const MAX_IMPACT_QUERIES: usize = 32;
pub const MAX_IMPACT_COMBINATIONS: usize = 256;
pub const MAX_IMPACT_DEPENDENCIES: usize = 64;
pub const MAX_MINIMAL_INVALIDATING_SETS: usize = 64;
pub const MAX_REVISION_IMPACT_CERTIFICATE_BYTES: usize = 64 * 1024 * 1024;

const MAGIC: &[u8; 8] = b"GCCRIM01";
const BUNDLE_MAGIC: &[u8; 8] = b"GCCRIB01";
const MAX_NAME_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevisionImpactPolicy {
    pub max_input_bytes: usize,
    pub max_evidence_bytes: usize,
    pub max_bundle_bytes: usize,
    pub max_combinations: usize,
    pub max_queries: usize,
}

impl Default for RevisionImpactPolicy {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_evidence_bytes: 16 * 1024 * 1024,
            max_bundle_bytes: MAX_REVISION_IMPACT_CERTIFICATE_BYTES,
            max_combinations: MAX_IMPACT_COMBINATIONS,
            max_queries: MAX_IMPACT_QUERIES,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImpactAtomKind {
    Component,
    Interface,
    Property,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImpactAtom {
    pub name: String,
    pub kind: ImpactAtomKind,
    pub old_sha256: [u8; 32],
    pub new_sha256: [u8; 32],
    /// Earlier atom indices on which this atom directly depends.
    pub depends_on: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImpactQuery {
    pub name: String,
    /// Atom indices directly used by the query. Transitive dependencies count.
    pub support: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImpactObservation {
    pub changed_mask: u16,
    pub query_index: u8,
    pub result: BoundedResult,
    pub reusable: bool,
    pub evidence_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MinimalInvalidatingSet {
    pub query_index: u8,
    pub changed_mask: u16,
}

/// Inclusion-minimal changed-atom set whose bounded answer differs from the
/// unchanged baseline answer for one query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinimalSemanticChangeSet {
    pub query_index: u8,
    pub changed_mask: u16,
    pub baseline_result: BoundedResult,
    pub changed_result: BoundedResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactCertificate {
    pub atoms: Vec<ImpactAtom>,
    pub queries: Vec<ImpactQuery>,
    /// Complete mask-major table: every mask, then every query.
    pub observations: Vec<ImpactObservation>,
    pub minimal_invalidating_sets: Vec<MinimalInvalidatingSet>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactSummary {
    pub atoms: usize,
    pub queries: usize,
    pub combinations: usize,
    pub reusable_observations: usize,
    pub invalidated_observations: usize,
    pub minimal_invalidating_sets: usize,
    pub minimal_semantic_change_sets: usize,
}

/// Deterministic work independently observed while checking an aggregate
/// two-component revision-impact bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactVerificationWork {
    pub parsed_evidence_bytes: usize,
    pub semantic_replays: usize,
    pub component_validations: usize,
    pub composed_pair_checks: usize,
    pub final_transition_checks: usize,
    pub result_comparisons: usize,
}

/// Old and new sources for one exact two-component revision cohort.
pub struct TwoComponentRevisionImpactInput<'a> {
    pub left_old: &'a [u8],
    pub left_new: &'a [u8],
    pub left_outputs: &'a [u64],
    pub right_old: &'a [u8],
    pub right_new: &'a [u8],
    pub right_outputs: &'a [u64],
    pub interface_old: &'a [u8],
    pub interface_new: &'a [u8],
    pub queries: &'a [BoundedQuery],
}

/// Impact certificate plus one independently checkable semantic artifact per
/// mask-major observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoComponentRevisionImpactBundle {
    pub impact: RevisionImpactCertificate,
    pub revision_evidence: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionImpactError(pub String);

impl fmt::Display for RevisionImpactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "revision impact certificate: {}", self.0)
    }
}

impl Error for RevisionImpactError {}

fn reject(message: impl Into<String>) -> RevisionImpactError {
    RevisionImpactError(message.into())
}

/// Build a canonical certificate from a complete independently obtained table.
pub fn produce_revision_impact_certificate(
    atoms: Vec<ImpactAtom>,
    queries: Vec<ImpactQuery>,
    observations: Vec<ImpactObservation>,
) -> Result<RevisionImpactCertificate, RevisionImpactError> {
    validate_inputs(&atoms, &queries, &observations)?;
    let minimal_invalidating_sets = derive_minimal_sets(&atoms, &queries, &observations)?;
    let certificate = RevisionImpactCertificate {
        atoms,
        queries,
        observations,
        minimal_invalidating_sets,
    };
    validate_certificate(&certificate)?;
    Ok(certificate)
}

/// Verify structure, canonical minimality, and every semantic observation.
///
/// The evaluator is intentionally caller-supplied so a verifier can invoke an
/// existing exact backend without sharing the producer's implementation.
pub fn verify_revision_impact_with<F>(
    certificate: &RevisionImpactCertificate,
    mut evaluator: F,
) -> Result<RevisionImpactSummary, RevisionImpactError>
where
    F: FnMut(u16, usize) -> Result<(BoundedResult, bool, [u8; 32]), RevisionImpactError>,
{
    validate_certificate(certificate)?;
    for observation in &certificate.observations {
        let actual = evaluator(observation.changed_mask, observation.query_index as usize)?;
        if actual
            != (
                observation.result,
                observation.reusable,
                observation.evidence_sha256,
            )
        {
            return Err(reject(format!(
                "independent observation mismatch at mask {} query {}",
                observation.changed_mask, observation.query_index
            )));
        }
    }
    Ok(summary(certificate))
}

/// Derive the smallest atom combinations that actually change each query's
/// bounded answer. This is deliberately distinct from evidence invalidation:
/// a source change can make cached evidence non-reusable without changing the
/// query result.
pub fn derive_minimal_semantic_change_sets(
    certificate: &RevisionImpactCertificate,
) -> Result<Vec<MinimalSemanticChangeSet>, RevisionImpactError> {
    validate_certificate(certificate)?;
    Ok(derive_semantic_change_sets(certificate))
}

/// Produce every admitted old/new combination through the existing exact
/// revision-local producer and retain its canonical evidence for independent
/// checking.
pub fn produce_two_component_revision_impact(
    input: &TwoComponentRevisionImpactInput<'_>,
) -> Result<TwoComponentRevisionImpactBundle, RevisionImpactError> {
    produce_two_component_revision_impact_with_policy(input, RevisionImpactPolicy::default())
}

pub fn produce_two_component_revision_impact_with_policy(
    input: &TwoComponentRevisionImpactInput<'_>,
    policy: RevisionImpactPolicy,
) -> Result<TwoComponentRevisionImpactBundle, RevisionImpactError> {
    validate_policy(policy)?;
    validate_input_bytes(input, policy)?;
    if input.queries.is_empty() || input.queries.len() > policy.max_queries {
        return Err(reject("query count must be between one and 32"));
    }
    let roles = changed_roles(input);
    if roles.is_empty() {
        return Err(reject("revision does not change any bound source"));
    }
    let atoms = impact_atoms(input, &roles);
    let queries = impact_queries(atoms.len(), input.queries.len());
    let combinations = 1usize << atoms.len();
    if combinations > policy.max_combinations {
        return Err(reject("counterfactual combination count exceeds policy"));
    }
    let mut observations = Vec::with_capacity(combinations * queries.len());
    let mut revision_evidence = Vec::with_capacity(combinations * queries.len());
    for mask in 0..combinations {
        let selected = select_sources(input, &roles, mask as u16);
        for (query_index, query) in input.queries.iter().enumerate() {
            let (certificate, produced) = produce_revision_local_certificate(
                selected.left,
                input.left_outputs,
                selected.right,
                input.right_outputs,
                selected.interface,
                query,
            )
            .map_err(|error| reject(format!("exact producer rejected scenario: {error}")))?;
            let checked = verify_revision_local_certificate(
                selected.left,
                selected.right,
                selected.interface,
                &certificate,
            )
            .map_err(|error| reject(format!("independent checker rejected scenario: {error}")))?;
            if checked.answer.result != produced.answer.result
                || checked.answer.horizon != produced.answer.horizon
                || checked.answer.bad_frame != produced.answer.bad_frame
                || checked.answer.reachable_states != produced.answer.reachable_states
            {
                return Err(reject(
                    "producer and independent checker logical summaries differ",
                ));
            }
            let evidence = encode_revision_local_certificate(&certificate)
                .map_err(|error| reject(format!("encode scenario evidence: {error}")))?;
            if evidence.len() > policy.max_evidence_bytes {
                return Err(reject("scenario evidence exceeds policy"));
            }
            let retained_bytes = revision_evidence
                .iter()
                .try_fold(0usize, |total, bytes: &Vec<u8>| {
                    total.checked_add(bytes.len())
                })
                .and_then(|total| total.checked_add(evidence.len()))
                .ok_or_else(|| reject("revision evidence byte count overflow"))?;
            if retained_bytes > policy.max_bundle_bytes {
                return Err(reject("revision evidence total exceeds policy"));
            }
            observations.push(ImpactObservation {
                changed_mask: mask as u16,
                query_index: query_index as u8,
                result: checked.answer.result,
                reusable: mask == 0,
                evidence_sha256: Sha256::digest(&evidence).into(),
            });
            revision_evidence.push(evidence);
        }
    }
    let impact = produce_revision_impact_certificate(atoms, queries, observations)?;
    let bundle = TwoComponentRevisionImpactBundle {
        impact,
        revision_evidence,
    };
    encode_two_component_revision_impact_bundle(&bundle, policy)?;
    Ok(bundle)
}

/// Independently decode and verify every retained revision-local artifact,
/// then validate its result and digest against the impact certificate.
pub fn verify_two_component_revision_impact(
    input: &TwoComponentRevisionImpactInput<'_>,
    bundle: &TwoComponentRevisionImpactBundle,
) -> Result<RevisionImpactSummary, RevisionImpactError> {
    verify_two_component_revision_impact_with_policy(input, bundle, RevisionImpactPolicy::default())
}

pub fn verify_two_component_revision_impact_with_policy(
    input: &TwoComponentRevisionImpactInput<'_>,
    bundle: &TwoComponentRevisionImpactBundle,
    policy: RevisionImpactPolicy,
) -> Result<RevisionImpactSummary, RevisionImpactError> {
    verify_two_component_revision_impact_observed_with_policy(input, bundle, policy)
        .map(|(summary, _)| summary)
}

pub fn verify_two_component_revision_impact_observed(
    input: &TwoComponentRevisionImpactInput<'_>,
    bundle: &TwoComponentRevisionImpactBundle,
) -> Result<(RevisionImpactSummary, RevisionImpactVerificationWork), RevisionImpactError> {
    verify_two_component_revision_impact_observed_with_policy(
        input,
        bundle,
        RevisionImpactPolicy::default(),
    )
}

pub fn verify_two_component_revision_impact_observed_with_policy(
    input: &TwoComponentRevisionImpactInput<'_>,
    bundle: &TwoComponentRevisionImpactBundle,
    policy: RevisionImpactPolicy,
) -> Result<(RevisionImpactSummary, RevisionImpactVerificationWork), RevisionImpactError> {
    validate_policy(policy)?;
    validate_input_bytes(input, policy)?;
    encode_two_component_revision_impact_bundle(bundle, policy)?;
    let roles = changed_roles(input);
    if roles.is_empty() || bundle.revision_evidence.len() != bundle.impact.observations.len() {
        return Err(reject("revision evidence table is incomplete"));
    }
    let expected_atoms = impact_atoms(input, &roles);
    let combinations = 1usize << expected_atoms.len();
    if combinations > policy.max_combinations || input.queries.len() > policy.max_queries {
        return Err(reject("revision impact dimensions exceed policy"));
    }
    let evidence_total = bundle
        .revision_evidence
        .iter()
        .try_fold(0usize, |total, evidence| {
            if evidence.len() > policy.max_evidence_bytes {
                None
            } else {
                total.checked_add(evidence.len())
            }
        })
        .ok_or_else(|| reject("revision evidence exceeds policy"))?;
    if evidence_total > policy.max_bundle_bytes {
        return Err(reject("revision evidence total exceeds policy"));
    }
    let expected_queries = impact_queries(expected_atoms.len(), input.queries.len());
    if bundle.impact.atoms != expected_atoms || bundle.impact.queries != expected_queries {
        return Err(reject("impact boundary differs from supplied revision"));
    }
    let mut work = RevisionImpactVerificationWork {
        parsed_evidence_bytes: 0,
        semantic_replays: 0,
        component_validations: 0,
        composed_pair_checks: 0,
        final_transition_checks: 0,
        result_comparisons: 0,
    };
    let summary = verify_revision_impact_with(&bundle.impact, |mask, query_index| {
        let index = mask as usize * input.queries.len() + query_index;
        let evidence = bundle
            .revision_evidence
            .get(index)
            .ok_or_else(|| reject("revision evidence index is missing"))?;
        let certificate: RevisionLocalCertificate = decode_revision_local_certificate(evidence)
            .map_err(|error| reject(format!("decode scenario evidence: {error}")))?;
        let selected = select_sources(input, &roles, mask);
        let checked = verify_revision_local_certificate(
            selected.left,
            selected.right,
            selected.interface,
            &certificate,
        )
        .map_err(|error| reject(format!("independent checker rejected scenario: {error}")))?;
        work.parsed_evidence_bytes = work
            .parsed_evidence_bytes
            .checked_add(evidence.len())
            .ok_or_else(|| reject("verification evidence byte count overflow"))?;
        work.semantic_replays = work
            .semantic_replays
            .checked_add(1)
            .ok_or_else(|| reject("semantic replay count overflow"))?;
        work.component_validations = work
            .component_validations
            .checked_add(2)
            .ok_or_else(|| reject("component validation count overflow"))?;
        work.composed_pair_checks = work
            .composed_pair_checks
            .checked_add(
                checked
                    .left
                    .admissible_rows
                    .checked_mul(checked.right.admissible_rows)
                    .ok_or_else(|| reject("composed pair check count overflow"))?,
            )
            .ok_or_else(|| reject("composed pair check count overflow"))?;
        work.final_transition_checks = work
            .final_transition_checks
            .checked_add(checked.answer.transition_checks)
            .ok_or_else(|| reject("final transition check count overflow"))?;
        work.result_comparisons = work
            .result_comparisons
            .checked_add(1)
            .ok_or_else(|| reject("result comparison count overflow"))?;
        if checked.answer.horizon != input.queries[query_index].horizon
            || certificate_query(&certificate)? != input.queries[query_index]
        {
            return Err(reject(
                "scenario evidence query differs from supplied query",
            ));
        }
        Ok((
            checked.answer.result,
            mask == 0,
            Sha256::digest(evidence).into(),
        ))
    })?;
    if work.semantic_replays != bundle.impact.observations.len()
        || work.component_validations != bundle.impact.observations.len() * 2
        || work.result_comparisons != bundle.impact.observations.len()
        || work.parsed_evidence_bytes != evidence_total
    {
        return Err(reject("verification work accounting is incomplete"));
    }
    Ok((summary, work))
}

pub fn encode_two_component_revision_impact_bundle(
    bundle: &TwoComponentRevisionImpactBundle,
    policy: RevisionImpactPolicy,
) -> Result<Vec<u8>, RevisionImpactError> {
    validate_policy(policy)?;
    let impact = encode_revision_impact_certificate(&bundle.impact)?;
    let combinations = 1usize << bundle.impact.atoms.len();
    if combinations > policy.max_combinations || bundle.impact.queries.len() > policy.max_queries {
        return Err(reject("bundle dimensions exceed policy"));
    }
    if bundle.revision_evidence.len() != bundle.impact.observations.len() {
        return Err(reject("revision evidence table is incomplete"));
    }
    let mut total = BUNDLE_MAGIC.len() + 12 + impact.len();
    for evidence in &bundle.revision_evidence {
        if evidence.len() > policy.max_evidence_bytes {
            return Err(reject("scenario evidence exceeds policy"));
        }
        total = total
            .checked_add(4)
            .and_then(|value| value.checked_add(evidence.len()))
            .ok_or_else(|| reject("bundle byte count overflow"))?;
    }
    if total > policy.max_bundle_bytes {
        return Err(reject("bundle bytes exceed policy"));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| reject("bundle allocation failed"))?;
    output.extend_from_slice(BUNDLE_MAGIC);
    output.extend_from_slice(&REVISION_IMPACT_CERTIFICATE_VERSION.to_be_bytes());
    output.extend_from_slice(
        &u32::try_from(impact.len())
            .map_err(|_| reject("impact length overflow"))?
            .to_be_bytes(),
    );
    output.extend_from_slice(
        &u32::try_from(bundle.revision_evidence.len())
            .map_err(|_| reject("evidence count overflow"))?
            .to_be_bytes(),
    );
    output.extend_from_slice(&impact);
    for evidence in &bundle.revision_evidence {
        output.extend_from_slice(
            &u32::try_from(evidence.len())
                .map_err(|_| reject("evidence length overflow"))?
                .to_be_bytes(),
        );
        output.extend_from_slice(evidence);
    }
    debug_assert_eq!(output.len(), total);
    Ok(output)
}

pub fn decode_two_component_revision_impact_bundle(
    bytes: &[u8],
    policy: RevisionImpactPolicy,
) -> Result<TwoComponentRevisionImpactBundle, RevisionImpactError> {
    validate_policy(policy)?;
    if bytes.len() > policy.max_bundle_bytes {
        return Err(reject("bundle bytes exceed policy"));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(BUNDLE_MAGIC.len())? != BUNDLE_MAGIC {
        return Err(reject("invalid bundle magic"));
    }
    if decoder.u32()? != REVISION_IMPACT_CERTIFICATE_VERSION {
        return Err(reject("unsupported bundle version"));
    }
    let impact_len = decoder.bounded_u32(policy.max_bundle_bytes, "impact length")?;
    let evidence_count = decoder.bounded_u32(
        policy.max_combinations.saturating_mul(policy.max_queries),
        "evidence count",
    )?;
    let impact = decode_revision_impact_certificate(decoder.take(impact_len)?)?;
    let combinations = 1usize << impact.atoms.len();
    if combinations > policy.max_combinations || impact.queries.len() > policy.max_queries {
        return Err(reject("bundle dimensions exceed policy"));
    }
    if evidence_count != impact.observations.len() {
        return Err(reject("bundle evidence count is not complete"));
    }
    let mut revision_evidence = Vec::new();
    revision_evidence
        .try_reserve_exact(evidence_count)
        .map_err(|_| reject("evidence table allocation failed"))?;
    for _ in 0..evidence_count {
        let length = decoder.bounded_u32(policy.max_evidence_bytes, "evidence length")?;
        revision_evidence.push(decoder.take(length)?.to_vec());
    }
    if decoder.offset != bytes.len() {
        return Err(reject("bundle has trailing bytes"));
    }
    let bundle = TwoComponentRevisionImpactBundle {
        impact,
        revision_evidence,
    };
    if encode_two_component_revision_impact_bundle(&bundle, policy)? != bytes {
        return Err(reject("bundle encoding is not canonical"));
    }
    Ok(bundle)
}

pub fn encode_revision_impact_certificate(
    certificate: &RevisionImpactCertificate,
) -> Result<Vec<u8>, RevisionImpactError> {
    validate_certificate(certificate)?;
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&REVISION_IMPACT_CERTIFICATE_VERSION.to_be_bytes());
    append_u16(&mut output, certificate.atoms.len())?;
    append_u16(&mut output, certificate.queries.len())?;
    append_u32(&mut output, certificate.observations.len())?;
    append_u16(&mut output, certificate.minimal_invalidating_sets.len())?;
    output.extend_from_slice(&0_u16.to_be_bytes());
    for atom in &certificate.atoms {
        append_name(&mut output, &atom.name)?;
        output.push(match atom.kind {
            ImpactAtomKind::Component => 0,
            ImpactAtomKind::Interface => 1,
            ImpactAtomKind::Property => 2,
        });
        output.extend_from_slice(&atom.old_sha256);
        output.extend_from_slice(&atom.new_sha256);
        append_u16(&mut output, atom.depends_on.len())?;
        output.extend_from_slice(&atom.depends_on);
    }
    for query in &certificate.queries {
        append_name(&mut output, &query.name)?;
        append_u16(&mut output, query.support.len())?;
        output.extend_from_slice(&query.support);
    }
    for observation in &certificate.observations {
        output.extend_from_slice(&observation.changed_mask.to_be_bytes());
        output.push(observation.query_index);
        output.push(match observation.result {
            BoundedResult::Safe => 0,
            BoundedResult::Unsafe => 1,
        });
        output.push(u8::from(observation.reusable));
        output.extend_from_slice(&observation.evidence_sha256);
    }
    for set in &certificate.minimal_invalidating_sets {
        output.push(set.query_index);
        output.extend_from_slice(&set.changed_mask.to_be_bytes());
    }
    if output.len() > MAX_REVISION_IMPACT_CERTIFICATE_BYTES {
        return Err(reject("encoding exceeds byte limit"));
    }
    Ok(output)
}

pub fn decode_revision_impact_certificate(
    bytes: &[u8],
) -> Result<RevisionImpactCertificate, RevisionImpactError> {
    if bytes.len() > MAX_REVISION_IMPACT_CERTIFICATE_BYTES {
        return Err(reject("encoding exceeds byte limit"));
    }
    let mut decoder = Decoder { bytes, offset: 0 };
    if decoder.take(MAGIC.len())? != MAGIC {
        return Err(reject("invalid magic"));
    }
    if decoder.u32()? != REVISION_IMPACT_CERTIFICATE_VERSION {
        return Err(reject("unsupported version"));
    }
    let atom_count = decoder.bounded_u16(MAX_IMPACT_ATOMS, "atom count")?;
    let query_count = decoder.bounded_u16(MAX_IMPACT_QUERIES, "query count")?;
    let expected_combinations = 1usize
        .checked_shl(atom_count as u32)
        .ok_or_else(|| reject("combination count overflow"))?;
    let expected_observations = expected_combinations
        .checked_mul(query_count)
        .ok_or_else(|| reject("observation count overflow"))?;
    let observation_count = decoder.bounded_u32(
        MAX_IMPACT_COMBINATIONS * MAX_IMPACT_QUERIES,
        "observation count",
    )?;
    if observation_count != expected_observations {
        return Err(reject("observation count is not complete"));
    }
    let set_count = decoder.bounded_u16(
        MAX_MINIMAL_INVALIDATING_SETS,
        "minimal invalidating set count",
    )?;
    if decoder.u16()? != 0 {
        return Err(reject("reserved field is nonzero"));
    }
    let mut atoms = Vec::with_capacity(atom_count);
    for _ in 0..atom_count {
        let name = decoder.name()?;
        let kind = match decoder.byte()? {
            0 => ImpactAtomKind::Component,
            1 => ImpactAtomKind::Interface,
            2 => ImpactAtomKind::Property,
            _ => return Err(reject("invalid atom kind")),
        };
        let old_sha256 = decoder.digest()?;
        let new_sha256 = decoder.digest()?;
        let dependency_count = decoder.bounded_u16(MAX_IMPACT_DEPENDENCIES, "dependency count")?;
        let depends_on = decoder.take(dependency_count)?.to_vec();
        atoms.push(ImpactAtom {
            name,
            kind,
            old_sha256,
            new_sha256,
            depends_on,
        });
    }
    let mut queries = Vec::with_capacity(query_count);
    for _ in 0..query_count {
        let name = decoder.name()?;
        let support_count = decoder.bounded_u16(MAX_IMPACT_ATOMS, "query support count")?;
        let support = decoder.take(support_count)?.to_vec();
        queries.push(ImpactQuery { name, support });
    }
    let mut observations = Vec::with_capacity(observation_count);
    for _ in 0..observation_count {
        let changed_mask = decoder.u16()?;
        let query_index = decoder.byte()?;
        let result = match decoder.byte()? {
            0 => BoundedResult::Safe,
            1 => BoundedResult::Unsafe,
            _ => return Err(reject("invalid bounded result")),
        };
        let reusable = match decoder.byte()? {
            0 => false,
            1 => true,
            _ => return Err(reject("invalid reusable flag")),
        };
        let evidence_sha256 = decoder.digest()?;
        observations.push(ImpactObservation {
            changed_mask,
            query_index,
            result,
            reusable,
            evidence_sha256,
        });
    }
    let mut minimal_invalidating_sets = Vec::with_capacity(set_count);
    for _ in 0..set_count {
        minimal_invalidating_sets.push(MinimalInvalidatingSet {
            query_index: decoder.byte()?,
            changed_mask: decoder.u16()?,
        });
    }
    if decoder.offset != bytes.len() {
        return Err(reject("trailing bytes"));
    }
    let certificate = RevisionImpactCertificate {
        atoms,
        queries,
        observations,
        minimal_invalidating_sets,
    };
    validate_certificate(&certificate)?;
    if encode_revision_impact_certificate(&certificate)? != bytes {
        return Err(reject("encoding is not canonical"));
    }
    Ok(certificate)
}

fn validate_certificate(
    certificate: &RevisionImpactCertificate,
) -> Result<(), RevisionImpactError> {
    validate_inputs(
        &certificate.atoms,
        &certificate.queries,
        &certificate.observations,
    )?;
    let expected = derive_minimal_sets(
        &certificate.atoms,
        &certificate.queries,
        &certificate.observations,
    )?;
    if certificate.minimal_invalidating_sets != expected {
        return Err(reject(
            "minimal invalidating sets are incomplete or noncanonical",
        ));
    }
    Ok(())
}

fn validate_policy(policy: RevisionImpactPolicy) -> Result<(), RevisionImpactError> {
    if policy.max_input_bytes == 0
        || policy.max_evidence_bytes == 0
        || policy.max_bundle_bytes < BUNDLE_MAGIC.len() + 12
        || policy.max_bundle_bytes > MAX_REVISION_IMPACT_CERTIFICATE_BYTES
        || policy.max_combinations == 0
        || policy.max_combinations > MAX_IMPACT_COMBINATIONS
        || !policy.max_combinations.is_power_of_two()
        || policy.max_queries == 0
        || policy.max_queries > MAX_IMPACT_QUERIES
    {
        return Err(reject("invalid revision impact policy"));
    }
    Ok(())
}

fn validate_input_bytes(
    input: &TwoComponentRevisionImpactInput<'_>,
    policy: RevisionImpactPolicy,
) -> Result<(), RevisionImpactError> {
    let total = [
        input.left_old.len(),
        input.left_new.len(),
        input.right_old.len(),
        input.right_new.len(),
        input.interface_old.len(),
        input.interface_new.len(),
    ]
    .into_iter()
    .try_fold(0usize, |total, bytes| total.checked_add(bytes))
    .ok_or_else(|| reject("input byte count overflow"))?;
    if total > policy.max_input_bytes {
        return Err(reject("input bytes exceed policy"));
    }
    Ok(())
}

fn validate_inputs(
    atoms: &[ImpactAtom],
    queries: &[ImpactQuery],
    observations: &[ImpactObservation],
) -> Result<(), RevisionImpactError> {
    if atoms.is_empty() || atoms.len() > MAX_IMPACT_ATOMS {
        return Err(reject("atom count must be between one and eight"));
    }
    if queries.is_empty() || queries.len() > MAX_IMPACT_QUERIES {
        return Err(reject("query count must be between one and 32"));
    }
    let mut dependency_total = 0usize;
    for (index, atom) in atoms.iter().enumerate() {
        validate_name(&atom.name)?;
        if atom.old_sha256 == atom.new_sha256 {
            return Err(reject("impact atom does not change its bound source"));
        }
        if index > 0 && atoms[index - 1].name >= atom.name {
            return Err(reject("atom names must be unique and strictly ordered"));
        }
        validate_indices(&atom.depends_on, index, "atom dependency")?;
        dependency_total = dependency_total
            .checked_add(atom.depends_on.len())
            .ok_or_else(|| reject("dependency count overflow"))?;
    }
    if dependency_total > MAX_IMPACT_DEPENDENCIES {
        return Err(reject("dependency count exceeds limit"));
    }
    for (index, query) in queries.iter().enumerate() {
        validate_name(&query.name)?;
        if index > 0 && queries[index - 1].name >= query.name {
            return Err(reject("query names must be unique and strictly ordered"));
        }
        if query.support.is_empty() {
            return Err(reject("query support is empty"));
        }
        validate_indices(&query.support, atoms.len(), "query support")?;
    }
    let combinations = 1usize << atoms.len();
    let expected = combinations
        .checked_mul(queries.len())
        .ok_or_else(|| reject("observation count overflow"))?;
    if combinations > MAX_IMPACT_COMBINATIONS || observations.len() != expected {
        return Err(reject("observation table is not complete"));
    }
    let closures = queries
        .iter()
        .map(|query| support_closure(atoms, &query.support))
        .collect::<Vec<_>>();
    for mask in 0..combinations {
        for query_index in 0..queries.len() {
            let index = mask * queries.len() + query_index;
            let observation = observations[index];
            if observation.changed_mask as usize != mask
                || observation.query_index as usize != query_index
            {
                return Err(reject("observations must use complete mask-major order"));
            }
            if mask == 0 && !observation.reusable {
                return Err(reject("unchanged baseline must be reusable"));
            }
            if mask & closures[query_index] == 0 {
                let baseline = observations[query_index];
                if observation.result != baseline.result || !observation.reusable {
                    return Err(reject("out-of-support change affects query"));
                }
            }
        }
    }
    Ok(())
}

fn derive_minimal_sets(
    atoms: &[ImpactAtom],
    queries: &[ImpactQuery],
    observations: &[ImpactObservation],
) -> Result<Vec<MinimalInvalidatingSet>, RevisionImpactError> {
    let combinations = 1usize << atoms.len();
    let mut sets = Vec::new();
    for query_index in 0..queries.len() {
        for mask in 1..combinations {
            let observation = observations[mask * queries.len() + query_index];
            if observation.reusable {
                continue;
            }
            let has_invalid_proper_subset = (1..mask).any(|candidate| {
                candidate & mask == candidate
                    && !observations[candidate * queries.len() + query_index].reusable
            });
            if !has_invalid_proper_subset {
                sets.push(MinimalInvalidatingSet {
                    query_index: query_index as u8,
                    changed_mask: mask as u16,
                });
            }
        }
    }
    if sets.len() > MAX_MINIMAL_INVALIDATING_SETS {
        return Err(reject("minimal invalidating set count exceeds limit"));
    }
    Ok(sets)
}

fn derive_semantic_change_sets(
    certificate: &RevisionImpactCertificate,
) -> Vec<MinimalSemanticChangeSet> {
    let combinations = 1usize << certificate.atoms.len();
    let query_count = certificate.queries.len();
    let mut sets = Vec::new();
    for query_index in 0..query_count {
        let baseline_result = certificate.observations[query_index].result;
        for mask in 1..combinations {
            let changed_result = certificate.observations[mask * query_count + query_index].result;
            if changed_result == baseline_result {
                continue;
            }
            let has_changing_proper_subset = (1..mask).any(|candidate| {
                candidate & mask == candidate
                    && certificate.observations[candidate * query_count + query_index].result
                        != baseline_result
            });
            if !has_changing_proper_subset {
                sets.push(MinimalSemanticChangeSet {
                    query_index: query_index as u8,
                    changed_mask: mask as u16,
                    baseline_result,
                    changed_result,
                });
            }
        }
    }
    sets
}

fn support_closure(atoms: &[ImpactAtom], support: &[u8]) -> usize {
    let mut closure = 0usize;
    for index in support {
        closure |= 1usize << *index;
    }
    loop {
        let previous = closure;
        for (index, atom) in atoms.iter().enumerate() {
            if closure & (1usize << index) != 0 {
                for dependency in &atom.depends_on {
                    closure |= 1usize << *dependency;
                }
            }
        }
        if closure == previous {
            return closure;
        }
    }
}

#[derive(Clone, Copy)]
enum ChangedRole {
    Left,
    Right,
    Interface,
}

struct SelectedSources<'a> {
    left: &'a [u8],
    right: &'a [u8],
    interface: &'a [u8],
}

fn changed_roles(input: &TwoComponentRevisionImpactInput<'_>) -> Vec<ChangedRole> {
    let mut roles = Vec::new();
    if input.left_old != input.left_new {
        roles.push(ChangedRole::Left);
    }
    if input.right_old != input.right_new {
        roles.push(ChangedRole::Right);
    }
    if input.interface_old != input.interface_new {
        roles.push(ChangedRole::Interface);
    }
    roles
}

fn impact_atoms(
    input: &TwoComponentRevisionImpactInput<'_>,
    roles: &[ChangedRole],
) -> Vec<ImpactAtom> {
    roles
        .iter()
        .enumerate()
        .map(|(index, role)| {
            let (name, kind, old, new) = match role {
                ChangedRole::Left => (
                    "component-left",
                    ImpactAtomKind::Component,
                    input.left_old,
                    input.left_new,
                ),
                ChangedRole::Right => (
                    "component-right",
                    ImpactAtomKind::Component,
                    input.right_old,
                    input.right_new,
                ),
                ChangedRole::Interface => (
                    "interface",
                    ImpactAtomKind::Interface,
                    input.interface_old,
                    input.interface_new,
                ),
            };
            let depends_on = if matches!(role, ChangedRole::Interface) {
                (0..index).map(|dependency| dependency as u8).collect()
            } else {
                Vec::new()
            };
            ImpactAtom {
                name: name.to_string(),
                kind,
                old_sha256: source_digest(old),
                new_sha256: source_digest(new),
                depends_on,
            }
        })
        .collect()
}

fn impact_queries(atom_count: usize, query_count: usize) -> Vec<ImpactQuery> {
    let support = (0..atom_count).map(|index| index as u8).collect::<Vec<_>>();
    (0..query_count)
        .map(|index| ImpactQuery {
            name: format!("query-{index:02}"),
            support: support.clone(),
        })
        .collect()
}

fn select_sources<'a>(
    input: &TwoComponentRevisionImpactInput<'a>,
    roles: &[ChangedRole],
    mask: u16,
) -> SelectedSources<'a> {
    let mut selected = SelectedSources {
        left: input.left_old,
        right: input.right_old,
        interface: input.interface_old,
    };
    for (index, role) in roles.iter().enumerate() {
        if mask & (1_u16 << index) == 0 {
            continue;
        }
        match role {
            ChangedRole::Left => selected.left = input.left_new,
            ChangedRole::Right => selected.right = input.right_new,
            ChangedRole::Interface => selected.interface = input.interface_new,
        }
    }
    selected
}

fn certificate_query(
    certificate: &RevisionLocalCertificate,
) -> Result<BoundedQuery, RevisionImpactError> {
    decode_bounded_answer_certificate(&certificate.final_evidence)
        .map(|answer| answer.query)
        .map_err(|error| reject(format!("decode scenario query: {error}")))
}

fn validate_indices(indices: &[u8], upper: usize, label: &str) -> Result<(), RevisionImpactError> {
    if indices.iter().any(|index| *index as usize >= upper)
        || indices.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(reject(format!(
            "{label} indices must be in range, unique, and strictly ordered"
        )));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), RevisionImpactError> {
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(reject("name is empty, too long, or noncanonical"));
    }
    Ok(())
}

fn summary(certificate: &RevisionImpactCertificate) -> RevisionImpactSummary {
    let reusable_observations = certificate
        .observations
        .iter()
        .filter(|observation| observation.reusable)
        .count();
    RevisionImpactSummary {
        atoms: certificate.atoms.len(),
        queries: certificate.queries.len(),
        combinations: 1usize << certificate.atoms.len(),
        reusable_observations,
        invalidated_observations: certificate.observations.len() - reusable_observations,
        minimal_invalidating_sets: certificate.minimal_invalidating_sets.len(),
        minimal_semantic_change_sets: derive_semantic_change_sets(certificate).len(),
    }
}

fn append_u16(output: &mut Vec<u8>, value: usize) -> Result<(), RevisionImpactError> {
    output.extend_from_slice(
        &u16::try_from(value)
            .map_err(|_| reject("u16 count overflow"))?
            .to_be_bytes(),
    );
    Ok(())
}

fn append_u32(output: &mut Vec<u8>, value: usize) -> Result<(), RevisionImpactError> {
    output.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| reject("u32 count overflow"))?
            .to_be_bytes(),
    );
    Ok(())
}

fn append_name(output: &mut Vec<u8>, name: &str) -> Result<(), RevisionImpactError> {
    validate_name(name)?;
    output.push(u8::try_from(name.len()).map_err(|_| reject("name length overflow"))?);
    output.extend_from_slice(name.as_bytes());
    Ok(())
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], RevisionImpactError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| reject("truncated encoding"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn byte(&mut self) -> Result<u8, RevisionImpactError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, RevisionImpactError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("bounded slice"),
        ))
    }

    fn u32(&mut self) -> Result<u32, RevisionImpactError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("bounded slice"),
        ))
    }

    fn bounded_u16(&mut self, limit: usize, label: &str) -> Result<usize, RevisionImpactError> {
        let value = self.u16()? as usize;
        if value > limit {
            return Err(reject(format!("{label} exceeds limit")));
        }
        Ok(value)
    }

    fn bounded_u32(&mut self, limit: usize, label: &str) -> Result<usize, RevisionImpactError> {
        let value =
            usize::try_from(self.u32()?).map_err(|_| reject(format!("{label} overflow")))?;
        if value > limit {
            return Err(reject(format!("{label} exceeds limit")));
        }
        Ok(value)
    }

    fn digest(&mut self) -> Result<[u8; 32], RevisionImpactError> {
        self.take(32)?
            .try_into()
            .map_err(|_| reject("truncated digest"))
    }

    fn name(&mut self) -> Result<String, RevisionImpactError> {
        let length = self.byte()? as usize;
        let bytes = self.take(length)?;
        let value = std::str::from_utf8(bytes).map_err(|_| reject("name is not UTF-8"))?;
        validate_name(value)?;
        Ok(value.to_string())
    }
}
