use guarded_continuation_checker::revision_local::{
    BoundEvidence, BoundedQuery, BoundedResult, ComponentSide, EvidenceSection, InterfaceWire,
    LocalEvidence, RevisionLocalCertificate, WordInterfaceContract,
    compose_verified_local_relations, decode_bounded_answer_certificate,
    decode_local_relation_certificate, decode_revision_local_certificate,
    decode_word_interface_contract, encode_bounded_answer_certificate,
    encode_local_relation_certificate, encode_revision_local_certificate,
    encode_word_interface_contract, produce_bounded_answer, produce_local_relation,
    produce_revision_local_certificate, source_digest, unchanged_local_evidence,
    verify_bounded_answer, verify_local_relation, verify_local_relation_for_composition,
    verify_revision_local_certificate, verify_source_bindings,
};

#[test]
fn downstream_client_can_preserve_one_component_across_a_revision() {
    let left_source = b"left component v1";
    let right_v1 = b"right component v1";
    let right_v2 = b"right component v2";
    let interface = b"velocity:2 left->right";
    let previous = RevisionLocalCertificate {
        left: LocalEvidence {
            source_sha256: source_digest(left_source),
            evidence: b"complete left relation".to_vec(),
        },
        right: LocalEvidence {
            source_sha256: source_digest(right_v1),
            evidence: b"complete right-v1 relation".to_vec(),
        },
        interface: BoundEvidence {
            source_sha256: source_digest(interface),
            evidence: b"compatible word interface".to_vec(),
        },
        final_evidence: b"safe".to_vec(),
    };
    let mut next = previous.clone();
    next.right.source_sha256 = source_digest(right_v2);
    next.right.evidence = b"complete right-v2 relation".to_vec();
    next.final_evidence = b"unsafe at frame 2".to_vec();

    let bytes = encode_revision_local_certificate(&next).unwrap();
    let decoded = decode_revision_local_certificate(&bytes).unwrap();
    verify_source_bindings(left_source, right_v2, interface, &decoded).unwrap();
    assert!(unchanged_local_evidence(&previous, &decoded, EvidenceSection::Left).unwrap());
    assert!(!unchanged_local_evidence(&previous, &decoded, EvidenceSection::Right).unwrap());
}

#[test]
fn downstream_client_can_encode_and_verify_a_complete_word_relation() {
    let source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 ulte 1 3 9\n11 constraint 10\n12 zero 1\n13 bad 12 never\n";
    let produced = produce_local_relation(source, &[7]).unwrap();
    let encoded = encode_local_relation_certificate(&produced).unwrap();
    let decoded = decode_local_relation_certificate(&encoded).unwrap();
    let summary = verify_local_relation(source, &decoded, EvidenceSection::Left).unwrap();
    assert_eq!(summary.candidate_valuations, 16);
    assert_eq!(summary.admissible_rows, 12);
}

#[test]
fn downstream_client_can_reuse_validated_evidence_for_word_interface_composition() {
    let left_source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n";
    let right_source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 xor 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n";
    let left = produce_local_relation(left_source, &[7]).unwrap();
    let right = produce_local_relation(right_source, &[7]).unwrap();
    let contract = WordInterfaceContract {
        wires: vec![InterfaceWire {
            from: ComponentSide::Left,
            output: 7,
            to_input: 3,
        }],
    };
    let contract_text = encode_word_interface_contract(&contract).unwrap();
    let contract = decode_word_interface_contract(contract_text.as_bytes()).unwrap();
    let verified_left =
        verify_local_relation_for_composition(left_source, &left, EvidenceSection::Left).unwrap();
    let verified_right =
        verify_local_relation_for_composition(right_source, &right, EvidenceSection::Right)
            .unwrap();
    let composed =
        compose_verified_local_relations(&verified_left, &verified_right, &contract).unwrap();
    assert_eq!(composed.pair_checks, 256);
    assert_eq!(composed.pairs.len(), 64);
}

#[test]
fn downstream_client_can_verify_both_composed_bounded_answers() {
    let left_source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n";
    let right_source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n";
    let left = produce_local_relation(left_source, &[7]).unwrap();
    let right = produce_local_relation(right_source, &[7, 10]).unwrap();
    let left =
        verify_local_relation_for_composition(left_source, &left, EvidenceSection::Left).unwrap();
    let right = verify_local_relation_for_composition(right_source, &right, EvidenceSection::Right)
        .unwrap();
    let contract = WordInterfaceContract {
        wires: vec![InterfaceWire {
            from: ComponentSide::Left,
            output: 7,
            to_input: 3,
        }],
    };
    for (horizon, expected, bad_frame) in [
        (0, BoundedResult::Safe, None),
        (1, BoundedResult::Unsafe, Some(1)),
    ] {
        let query = BoundedQuery {
            horizon,
            bad_side: ComponentSide::Right,
            bad_output: 10,
        };
        let produced = produce_bounded_answer(&left, &right, &contract, &query).unwrap();
        let bytes = encode_bounded_answer_certificate(&produced).unwrap();
        let decoded = decode_bounded_answer_certificate(&bytes).unwrap();
        let summary = verify_bounded_answer(&left, &right, &contract, &decoded).unwrap();
        assert_eq!(summary.result, expected);
        assert_eq!(summary.bad_frame, bad_frame);
    }
}

#[test]
fn downstream_client_can_exchange_a_complete_revision_local_envelope() {
    let left_source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 command\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 zero 1\n10 bad 9 never\n";
    let right_source = b"1 sort bitvec 1\n2 sort bitvec 2\n3 input 2 sensed\n4 state 2 state\n5 zero 2\n6 init 2 4 5\n7 add 2 4 3\n8 next 2 4 7\n9 constd 2 2\n10 eq 1 4 9\n11 bad 10 reached_two\n";
    let interface = encode_word_interface_contract(&WordInterfaceContract {
        wires: vec![InterfaceWire {
            from: ComponentSide::Left,
            output: 7,
            to_input: 3,
        }],
    })
    .unwrap();
    let query = BoundedQuery {
        horizon: 1,
        bad_side: ComponentSide::Right,
        bad_output: 10,
    };
    let (produced, _) = produce_revision_local_certificate(
        left_source,
        &[7],
        right_source,
        &[7, 10],
        interface.as_bytes(),
        &query,
    )
    .unwrap();
    let bytes = encode_revision_local_certificate(&produced).unwrap();
    let decoded = decode_revision_local_certificate(&bytes).unwrap();
    let summary = verify_revision_local_certificate(
        left_source,
        right_source,
        interface.as_bytes(),
        &decoded,
    )
    .unwrap();
    assert_eq!(summary.answer.result, BoundedResult::Unsafe);
    assert_eq!(summary.answer.bad_frame, Some(1));
    assert_eq!(summary.certificate_bytes, bytes.len());
}
