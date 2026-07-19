use guarded_continuation_checker::btor2_phase::{self, PhaseSpec};

#[test]
fn downstream_api_produces_decodes_and_verifies_a_source_bound_phase_certificate() {
    let source = include_bytes!("../examples/btor2/watchdog-counter-v1.btor2");
    let certificate = btor2_phase::produce(
        source,
        &[
            PhaseSpec {
                input: true,
                length: 2,
            },
            PhaseSpec {
                input: false,
                length: 1_000_000_003,
            },
        ],
        13,
    )
    .unwrap();
    let encoded = btor2_phase::encode(&certificate).unwrap();
    let decoded = btor2_phase::decode(encoded.as_bytes()).unwrap();
    let summary = btor2_phase::verify(source, &decoded).unwrap();

    assert_eq!(summary.horizon, 1_000_000_005);
    assert_eq!(summary.phases, 2);
    assert_eq!(summary.final_state, 3);
    assert_eq!(summary.bad_property, 13);

    let saturating = include_bytes!("../examples/btor2/saturating-timer-rejected-v1.btor2");
    let replay = btor2_phase::produce_replay(
        saturating,
        &[PhaseSpec {
            input: false,
            length: 255,
        }],
        15,
    )
    .unwrap();
    let encoded = btor2_phase::encode_replay(&replay).unwrap();
    let decoded = btor2_phase::decode_replay(encoded.as_bytes()).unwrap();
    let summary = btor2_phase::verify_replay(saturating, &decoded).unwrap();
    assert_eq!(summary.final_state, 255);
}
