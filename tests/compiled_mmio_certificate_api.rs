use guarded_continuation_checker::{
    compiled_mmio_certificate::{
        BoundArtifact, CompiledMmioCertificateInputs, certify_compiled_mmio,
        decode_compiled_mmio_certificate, encode_compiled_mmio_certificate, verify_compiled_mmio,
    },
    riscv32imc::RV32_IMAGE_BASE,
};

fn instruction(opcode: u32, rd: u32, rs1: u32, immediate: u32) -> [u8; 4] {
    (((immediate & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | opcode).to_le_bytes()
}

#[test]
fn downstream_client_can_bind_and_verify_a_compiled_mmio_extraction() {
    let mut image = vec![0; 0x110];
    image[..4].copy_from_slice(&instruction(0x13, 10, 0, 7));
    image[4..8].copy_from_slice(&instruction(0x67, 0, 1, 0));
    let symbols = format!(
        "{:08x} T gcc_firmware_entry\n{:08x} B gcc_mmio_event_count\n{:08x} B gcc_mmio_events\n",
        RV32_IMAGE_BASE,
        RV32_IMAGE_BASE + 0x100,
        RV32_IMAGE_BASE + 0x104
    );
    let upstream = [BoundArtifact {
        name: "upstream.c",
        bytes: b"upstream",
    }];
    let compatibility = [BoundArtifact {
        name: "compat.c",
        bytes: b"compatibility",
    }];
    let inputs = CompiledMmioCertificateInputs {
        upstream_sources: &upstream,
        compatibility_sources: &compatibility,
        toolchain_identity: b"clang=21.1.5\ntarget=riscv32-unknown-elf\n",
        image: &image,
        symbol_table: symbols.as_bytes(),
    };
    let certificate = certify_compiled_mmio(inputs).unwrap();
    assert_eq!(certificate.execution.return_value, 7);
    let bytes = encode_compiled_mmio_certificate(&certificate).unwrap();
    let decoded = decode_compiled_mmio_certificate(&bytes).unwrap();
    verify_compiled_mmio(&decoded, inputs).unwrap();
}
