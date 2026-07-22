use guarded_continuation_checker::revision_impact::{
    ImpactAtom, ImpactAtomKind, ImpactObservation, ImpactQuery, MinimalInvalidatingSet,
    RevisionImpactError, decode_revision_impact_certificate, encode_revision_impact_certificate,
    produce_revision_impact_certificate, verify_revision_impact_with,
};
use guarded_continuation_checker::revision_local::BoundedResult;

fn digest(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn fixture() -> (Vec<ImpactAtom>, Vec<ImpactQuery>, Vec<ImpactObservation>) {
    let atoms = vec![
        ImpactAtom {
            name: "controller".into(),
            kind: ImpactAtomKind::Component,
            old_sha256: digest(1),
            new_sha256: digest(2),
            depends_on: vec![],
        },
        ImpactAtom {
            name: "interface".into(),
            kind: ImpactAtomKind::Interface,
            old_sha256: digest(3),
            new_sha256: digest(4),
            depends_on: vec![0],
        },
        ImpactAtom {
            name: "plant".into(),
            kind: ImpactAtomKind::Component,
            old_sha256: digest(5),
            new_sha256: digest(6),
            depends_on: vec![1],
        },
    ];
    let queries = vec![
        ImpactQuery {
            name: "controller-safe".into(),
            support: vec![0],
        },
        ImpactQuery {
            name: "plant-safe".into(),
            support: vec![2],
        },
    ];
    let mut observations = Vec::new();
    for mask in 0_u16..8 {
        for query_index in 0_u8..2 {
            let (result, reusable) = match query_index {
                0 => (
                    if mask & 1 == 0 {
                        BoundedResult::Safe
                    } else {
                        BoundedResult::Unsafe
                    },
                    mask & 1 == 0,
                ),
                _ => (
                    if mask & 4 == 0 {
                        BoundedResult::Unsafe
                    } else {
                        BoundedResult::Safe
                    },
                    mask & 6 == 0,
                ),
            };
            observations.push(ImpactObservation {
                changed_mask: mask,
                query_index,
                result,
                reusable,
            });
        }
    }
    (atoms, queries, observations)
}

#[test]
fn downstream_client_can_produce_encode_decode_and_independently_verify() {
    let (atoms, queries, observations) = fixture();
    let certificate =
        produce_revision_impact_certificate(atoms, queries, observations.clone()).unwrap();
    assert_eq!(
        certificate.minimal_invalidating_sets,
        vec![
            MinimalInvalidatingSet {
                query_index: 0,
                changed_mask: 1,
            },
            MinimalInvalidatingSet {
                query_index: 1,
                changed_mask: 2,
            },
            MinimalInvalidatingSet {
                query_index: 1,
                changed_mask: 4,
            },
        ]
    );
    let bytes = encode_revision_impact_certificate(&certificate).unwrap();
    assert_eq!(
        bytes,
        encode_revision_impact_certificate(&certificate).unwrap()
    );
    let decoded = decode_revision_impact_certificate(&bytes).unwrap();
    assert_eq!(decoded, certificate);
    let summary = verify_revision_impact_with(&decoded, |mask, query| {
        let observation = observations[mask as usize * 2 + query];
        Ok((observation.result, observation.reusable))
    })
    .unwrap();
    assert_eq!(summary.atoms, 3);
    assert_eq!(summary.queries, 2);
    assert_eq!(summary.combinations, 8);
    assert_eq!(summary.minimal_invalidating_sets, 3);
}

#[test]
fn independent_semantic_disagreement_fails_closed() {
    let (atoms, queries, observations) = fixture();
    let certificate = produce_revision_impact_certificate(atoms, queries, observations).unwrap();
    let error = verify_revision_impact_with(&certificate, |mask, query| {
        if mask == 4 && query == 1 {
            Ok((BoundedResult::Unsafe, true))
        } else {
            let observation = certificate.observations[mask as usize * 2 + query];
            Ok((observation.result, observation.reusable))
        }
    })
    .unwrap_err();
    assert!(error.0.contains("independent observation mismatch"));
}

#[test]
fn hostile_encoding_and_minimal_set_mutations_fail_closed() {
    let (atoms, queries, observations) = fixture();
    let certificate = produce_revision_impact_certificate(atoms, queries, observations).unwrap();
    let bytes = encode_revision_impact_certificate(&certificate).unwrap();
    for end in 0..bytes.len() {
        assert!(decode_revision_impact_certificate(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_revision_impact_certificate(&trailing).is_err());
    for index in 0..bytes.len() {
        let mut mutated = bytes.clone();
        mutated[index] ^= 1;
        let accepted = decode_revision_impact_certificate(&mutated);
        if let Ok(candidate) = accepted {
            assert_ne!(candidate, certificate);
        }
    }
    let mut missing = certificate.clone();
    missing.minimal_invalidating_sets.pop();
    assert!(encode_revision_impact_certificate(&missing).is_err());
    let mut extra = certificate;
    extra
        .minimal_invalidating_sets
        .push(MinimalInvalidatingSet {
            query_index: 1,
            changed_mask: 6,
        });
    assert!(encode_revision_impact_certificate(&extra).is_err());
}

#[test]
fn noncanonical_graphs_and_out_of_support_effects_are_rejected() {
    let (atoms, queries, mut observations) = fixture();
    observations[8].result = BoundedResult::Unsafe;
    let error = produce_revision_impact_certificate(atoms.clone(), queries.clone(), observations)
        .unwrap_err();
    assert!(error.0.contains("out-of-support"));

    let mut cyclic = atoms;
    cyclic[0].depends_on = vec![2];
    let (_, _, observations) = fixture();
    assert!(produce_revision_impact_certificate(cyclic, queries, observations).is_err());
}

#[test]
fn producer_errors_are_typed() {
    let (atoms, queries, mut observations) = fixture();
    observations.pop();
    let error: RevisionImpactError =
        produce_revision_impact_certificate(atoms, queries, observations).unwrap_err();
    assert!(error.0.contains("not complete"));
}
