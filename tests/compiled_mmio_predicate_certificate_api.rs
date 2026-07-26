use guarded_continuation_checker::{
    compiled_mmio_predicate_certificate::{
        certify_compiled_mmio_predicate, decode_compiled_mmio_predicate_certificate,
        encode_compiled_mmio_predicate_certificate, verify_compiled_mmio_predicate_bytes,
    },
    compiled_mmio_predicate_portfolio::{
        CompiledMmioPredicateRoute, preflight_compiled_mmio_predicate,
        produce_compiled_mmio_predicate_portfolio, verify_compiled_mmio_predicate_portfolio,
    },
    riscv32imc::{RV32_IMAGE_BASE, Rv32SymbolLayout},
};

fn guarded_image() -> (Vec<u8>, Rv32SymbolLayout) {
    let mut image = vec![0; 0x110];
    let sltiu_a0_six = (6u32 << 20) | (10 << 15) | (3 << 12) | (10 << 7) | 0x13;
    let return_to_ra = (1u32 << 15) | 0x67;
    image[..4].copy_from_slice(&sltiu_a0_six.to_le_bytes());
    image[4..8].copy_from_slice(&return_to_ra.to_le_bytes());
    (
        image,
        Rv32SymbolLayout {
            entry: RV32_IMAGE_BASE,
            event_count: RV32_IMAGE_BASE + 0x100,
            events: RV32_IMAGE_BASE + 0x104,
        },
    )
}

#[test]
fn downstream_api_exchanges_and_independently_checks_predicate_evidence() {
    let (image, symbols) = guarded_image();
    let certificate = certify_compiled_mmio_predicate(&image, symbols).unwrap();
    let bytes = encode_compiled_mmio_predicate_certificate(&certificate).unwrap();
    assert_eq!(
        decode_compiled_mmio_predicate_certificate(&bytes).unwrap(),
        certificate
    );
    let verification = verify_compiled_mmio_predicate_bytes(&bytes, &image, symbols).unwrap();
    assert_eq!(verification.decoded_transitions, 14);

    let preflight = preflight_compiled_mmio_predicate(&image, symbols).unwrap();
    assert_eq!(preflight.route, CompiledMmioPredicateRoute::PredicateV1);
    let evidence = produce_compiled_mmio_predicate_portfolio(&preflight, &image, symbols).unwrap();
    let portfolio =
        verify_compiled_mmio_predicate_portfolio(&preflight, &evidence, &image, symbols).unwrap();
    assert_eq!(portfolio.route, CompiledMmioPredicateRoute::PredicateV1);
    assert!(portfolio.predicate_artifact_bytes > 0);
}
