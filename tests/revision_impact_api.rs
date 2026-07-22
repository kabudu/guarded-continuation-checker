use guarded_continuation_checker::revision_impact::{
    ImpactAtom, ImpactAtomKind, ImpactObservation, ImpactQuery, MinimalInvalidatingSet,
    RevisionImpactError, TwoComponentRevisionImpactInput, decode_revision_impact_certificate,
    encode_revision_impact_certificate, produce_revision_impact_certificate,
    produce_two_component_revision_impact, verify_revision_impact_with,
    verify_two_component_revision_impact,
};
use guarded_continuation_checker::revision_local::{
    BoundedQuery, BoundedResult, ComponentSide, InterfaceWire, WordInterfaceContract,
    encode_word_interface_contract,
};

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
                evidence_sha256: digest(10 + mask as u8 * 2 + query_index),
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
        Ok((
            observation.result,
            observation.reusable,
            observation.evidence_sha256,
        ))
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
            Ok((BoundedResult::Unsafe, true, digest(99)))
        } else {
            let observation = certificate.observations[mask as usize * 2 + query];
            Ok((
                observation.result,
                observation.reusable,
                observation.evidence_sha256,
            ))
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

#[test]
fn exact_revision_local_evidence_drives_every_counterfactual() {
    let left = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n";
    let right_old = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n";
    let right_new = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 and 2 3 5\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n";
    let interface = encode_word_interface_contract(&WordInterfaceContract {
        external_inputs: None,
        wires: vec![InterfaceWire {
            from: ComponentSide::Left,
            output: 7,
            to_input: 3,
        }],
    })
    .unwrap();
    let queries = [
        BoundedQuery {
            horizon: 0,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        },
        BoundedQuery {
            horizon: 1,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        },
    ];
    let input = TwoComponentRevisionImpactInput {
        left_old: left,
        left_new: left,
        left_outputs: &[7],
        right_old,
        right_new,
        right_outputs: &[7, 10],
        interface_old: interface.as_bytes(),
        interface_new: interface.as_bytes(),
        queries: &queries,
    };
    let bundle = produce_two_component_revision_impact(&input).unwrap();
    assert_eq!(bundle.impact.atoms.len(), 1);
    assert_eq!(bundle.impact.observations.len(), 4);
    assert_eq!(bundle.revision_evidence.len(), 4);
    assert_eq!(bundle.impact.observations[0].result, BoundedResult::Safe);
    assert_eq!(bundle.impact.observations[1].result, BoundedResult::Unsafe);
    assert_eq!(bundle.impact.observations[2].result, BoundedResult::Safe);
    assert_eq!(bundle.impact.observations[3].result, BoundedResult::Safe);
    let summary = verify_two_component_revision_impact(&input, &bundle).unwrap();
    assert_eq!(summary.combinations, 2);
    assert_eq!(summary.minimal_invalidating_sets, 2);

    let mut hostile_boundary = bundle.clone();
    hostile_boundary.impact.queries[0].name = "query-renamed".into();
    assert!(verify_two_component_revision_impact(&input, &hostile_boundary).is_err());

    let mut hostile = bundle;
    hostile.revision_evidence[3][20] ^= 1;
    assert!(verify_two_component_revision_impact(&input, &hostile).is_err());
}
