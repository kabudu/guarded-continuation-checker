use guarded_continuation_checker::revision_batch::{
    RevisionBatchComponent, RevisionBatchQuery, decode_revision_batch, encode_revision_batch,
    extract_revision_batch_certificates, produce_revision_batch, verify_revision_batch,
};
use guarded_continuation_checker::revision_local::{
    BoundedQuery, ComponentSide, InterfaceWire, WordInterfaceContract,
    encode_word_interface_contract, verify_revision_local_certificate,
};

#[test]
fn downstream_client_can_exchange_and_extract_a_shared_revision_batch() {
    let left = b"1 sort bitvec 1\n2 state 1 state\n3 zero 1\n4 init 1 2 3\n5 input 1 sensed\n6 next 1 2 5\n7 output 2 projected\n";
    let right = b"1 sort bitvec 1\n2 state 1 state\n3 zero 1\n4 init 1 2 3\n5 input 1 command\n6 next 1 2 5\n7 output 2 projected\n";
    let interface = encode_word_interface_contract(&WordInterfaceContract {
        wires: vec![InterfaceWire {
            from: ComponentSide::Left,
            output: 2,
            to_input: 5,
        }],
        external_inputs: None,
    })
    .unwrap();
    let components = [
        RevisionBatchComponent {
            source: left,
            outputs: &[2],
        },
        RevisionBatchComponent {
            source: right,
            outputs: &[2],
        },
    ];
    let queries = [RevisionBatchQuery {
        left_component: 0,
        right_component: 1,
        interface_source: interface.as_bytes(),
        query: BoundedQuery {
            horizon: 0,
            bad_side: ComponentSide::Left,
            bad_output: 2,
        },
    }];

    let (certificate, production) = produce_revision_batch(&components, &queries).unwrap();
    let encoded = encode_revision_batch(&certificate).unwrap();
    assert_eq!(decode_revision_batch(&encoded).unwrap(), certificate);
    assert_eq!(production.shared_sections, 2);
    assert_eq!(production.entries, 1);
    let verification = verify_revision_batch(&[left, right], &encoded).unwrap();
    assert_eq!(verification.shared_sections_verified, 2);
    assert_eq!(verification.entries_verified, 1);

    let extracted = extract_revision_batch_certificates(&encoded).unwrap();
    assert_eq!(extracted.len(), 1);
    verify_revision_local_certificate(left, right, interface.as_bytes(), &extracted[0]).unwrap();
}
