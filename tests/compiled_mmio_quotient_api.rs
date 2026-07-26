use guarded_continuation_checker::{
    compiled_mmio_quotient::{
        EXACT_COMPILED_MMIO_INPUTS, EXACT_COMPILED_MMIO_REFERENCE_VERSION,
        build_exact_compiled_mmio_reference, verify_exact_compiled_mmio_reference,
    },
    riscv32imc::{RV32_IMAGE_BASE, Rv32SymbolLayout, execute_compiled_mmio_with_a0},
};

fn parity_image() -> (Vec<u8>, Rv32SymbolLayout) {
    let mut image = vec![0; 0x110];
    let andi_a0_one = (1u32 << 20) | (10 << 15) | (7 << 12) | (10 << 7) | 0x13;
    let return_to_ra = (1u32 << 15) | 0x67;
    image[..4].copy_from_slice(&andi_a0_one.to_le_bytes());
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
fn downstream_client_can_build_and_verify_the_complete_exact_reference() {
    let (image, symbols) = parity_image();
    assert_eq!(
        execute_compiled_mmio_with_a0(&image, symbols, 19)
            .unwrap()
            .return_value,
        1
    );

    let reference = build_exact_compiled_mmio_reference(&image, symbols).unwrap();
    assert_eq!(reference.version, EXACT_COMPILED_MMIO_REFERENCE_VERSION);
    assert_eq!(reference.executions.len(), EXACT_COMPILED_MMIO_INPUTS);
    assert_eq!(reference.classes.len(), 2);
    assert_eq!(reference.classes[0].member_count(), 128);
    assert_eq!(reference.classes[1].member_count(), 128);
    verify_exact_compiled_mmio_reference(&reference, &image, symbols).unwrap();
}
