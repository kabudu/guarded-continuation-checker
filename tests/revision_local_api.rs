use guarded_continuation_checker::revision_local::{
    BoundEvidence, EvidenceSection, LocalEvidence, RevisionLocalCertificate,
    decode_revision_local_certificate, encode_revision_local_certificate, source_digest,
    unchanged_local_evidence, verify_source_bindings,
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
