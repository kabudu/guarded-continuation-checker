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
