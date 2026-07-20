use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;

const COUNTER: &[u8] = include_bytes!("../examples/aiger/counter-overflow-4.aag");

#[test]
fn downstream_parser_reads_a_pinned_public_transition_and_fails_closed() {
    let model = parse_ascii_aiger_transition(COUNTER).unwrap();
    assert_eq!(
        (model.inputs.len(), model.latches.len(), model.outputs.len()),
        (0, 4, 1)
    );
    assert_eq!(model.state_count(), 16);
    assert!(parse_ascii_aiger_transition(&COUNTER[..20]).is_err());

    let mut extended = COUNTER.to_vec();
    extended.splice(0..16, b"aag 16 0 4 1 12 1".iter().copied());
    assert!(parse_ascii_aiger_transition(&extended).is_err());

    let invalid_initializer = b"aag 1 0 1 0 0\n2 0 9\n";
    assert!(parse_ascii_aiger_transition(invalid_initializer).is_err());
}
