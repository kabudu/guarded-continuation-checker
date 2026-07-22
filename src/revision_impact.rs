//! Canonical bounded certificates for revision-impact counterfactuals.
//!
//! This module deliberately separates certificate mechanics from model
//! semantics. [`verify_revision_impact_with`] requires an independent evaluator
//! for every admitted counterfactual observation.

use crate::revision_local::BoundedResult;
use std::{error::Error, fmt};

pub const REVISION_IMPACT_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_IMPACT_ATOMS: usize = 8;
pub const MAX_IMPACT_QUERIES: usize = 32;
pub const MAX_IMPACT_COMBINATIONS: usize = 256;
pub const MAX_IMPACT_DEPENDENCIES: usize = 64;
pub const MAX_MINIMAL_INVALIDATING_SETS: usize = 64;
pub const MAX_REVISION_IMPACT_CERTIFICATE_BYTES: usize = 64 * 1024 * 1024;

const MAGIC: &[u8; 8] = b"GCCRIM01";
const MAX_NAME_BYTES: usize = 64;

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
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MinimalInvalidatingSet {
    pub query_index: u8,
    pub changed_mask: u16,
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
    F: FnMut(u16, usize) -> Result<(BoundedResult, bool), RevisionImpactError>,
{
    validate_certificate(certificate)?;
    for observation in &certificate.observations {
        let actual = evaluator(observation.changed_mask, observation.query_index as usize)?;
        if actual != (observation.result, observation.reusable) {
            return Err(reject(format!(
                "independent observation mismatch at mask {} query {}",
                observation.changed_mask, observation.query_index
            )));
        }
    }
    Ok(summary(certificate))
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
        observations.push(ImpactObservation {
            changed_mask,
            query_index,
            result,
            reusable,
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

fn validate_inputs(
    atoms: &[ImpactAtom],
    queries: &[ImpactQuery],
    observations: &[ImpactObservation],
) -> Result<(), RevisionImpactError> {
    if atoms.len() < 2 || atoms.len() > MAX_IMPACT_ATOMS {
        return Err(reject("atom count must be between two and eight"));
    }
    if queries.is_empty() || queries.len() > MAX_IMPACT_QUERIES {
        return Err(reject("query count must be between one and 32"));
    }
    let mut dependency_total = 0usize;
    for (index, atom) in atoms.iter().enumerate() {
        validate_name(&atom.name)?;
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
