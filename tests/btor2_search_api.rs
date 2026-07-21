use guarded_continuation_checker::btor2_search::{self, SearchResult};

#[test]
fn downstream_api_produces_and_verifies_both_bounded_search_answers() {
    let source = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
    for (horizon, expected) in [(2, SearchResult::Safe), (3, SearchResult::Unsafe)] {
        let produced = btor2_search::produce(source, 13, horizon).unwrap();
        assert_eq!(produced.result, expected);
        let encoded = btor2_search::encode(&produced).unwrap();
        let decoded = btor2_search::decode(encoded.as_bytes()).unwrap();
        let summary = btor2_search::verify(source, &decoded).unwrap();
        assert_eq!(summary.result, expected);
        assert_eq!(summary.query_horizon, horizon);
    }
}

#[test]
fn downstream_api_preserves_a_distinct_terminal_input_for_reset_dependent_bad() {
    let source = b"1 sort bitvec 1\n2 sort bitvec 3\n3 input 1 reset\n4 zero 2\n5 state 2 count\n6 init 2 5 4\n7 one 2\n8 add 2 5 7\n9 ite 2 3 4 8\n10 next 2 5 9\n11 ite 2 3 4 5\n12 constd 2 2\n13 eq 1 11 12\n14 bad 13 reset_guarded\n";

    let produced = btor2_search::produce(source, 14, 2).unwrap();
    assert_eq!(produced.certificate_version, 2);
    assert_eq!(produced.result, SearchResult::Unsafe);
    assert_eq!(produced.bad_frame, Some(2));
    assert_eq!(produced.terminal_input, Some(false));

    let encoded = btor2_search::encode(&produced).unwrap();
    let decoded = btor2_search::decode(encoded.as_bytes()).unwrap();
    assert_eq!(
        btor2_search::verify(source, &decoded).unwrap().bad_frame,
        Some(2)
    );
}

#[test]
fn downstream_api_preserves_complete_multi_input_valuations() {
    let source = b"1 sort bitvec 1\n2 input 1 a\n3 input 1 b\n4 state 1 state\n5 zero 1\n6 init 1 4 5\n7 xor 1 2 3\n8 next 1 4 7\n9 and 1 4 2\n10 bad 9 state_and_a\n";

    let produced = btor2_search::produce(source, 10, 1).unwrap();
    assert_eq!(produced.certificate_version, 3);
    assert_eq!(produced.inputs, vec![2, 3]);
    assert_eq!(produced.result, SearchResult::Unsafe);
    assert_eq!(produced.witness_valuations, vec![1]);
    assert_eq!(produced.terminal_valuation, Some(1));

    let encoded = btor2_search::encode(&produced).unwrap();
    let decoded = btor2_search::decode(encoded.as_bytes()).unwrap();
    assert_eq!(
        btor2_search::verify(source, &decoded).unwrap().bad_frame,
        Some(1)
    );
}

#[test]
fn downstream_api_preserves_constraint_bound_admissible_valuations() {
    let source = b"1 sort bitvec 1\n2 input 1 a\n3 input 1 b\n4 state 1 state\n5 zero 1\n6 init 1 4 5\n7 xor 1 2 3\n8 next 1 4 7\n9 and 1 2 3\n10 not 1 9\n11 constraint 10\n12 bad 4 reached_one\n";

    let produced = btor2_search::produce(source, 12, 1).unwrap();
    assert_eq!(produced.certificate_version, 4);
    assert_eq!(produced.inputs, vec![2, 3]);
    assert_eq!(produced.constraints, vec![11]);
    assert_eq!(produced.result, SearchResult::Unsafe);
    assert_eq!(produced.witness_valuations, vec![1]);
    assert_eq!(produced.terminal_valuation, Some(0));

    let encoded = btor2_search::encode(&produced).unwrap();
    let decoded = btor2_search::decode(encoded.as_bytes()).unwrap();
    assert_eq!(
        btor2_search::verify(source, &decoded).unwrap().bad_frame,
        Some(1)
    );
}
