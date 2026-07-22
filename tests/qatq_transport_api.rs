#![cfg(feature = "research-qatq-transport")]

use guarded_continuation_checker::qatq_transport::{
    QatqTransportError, QatqTransportPolicy, decode_qatq_transport,
    decode_qatq_transport_file_create_new, decode_qatq_transport_to_writer, encode_qatq_transport,
    inspect_qatq_transport,
};
use sha2::{Digest, Sha256};
use std::{fs, io, path::PathBuf};

const HEADER_LEN: usize = 104;

fn fixture(length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| ((index.wrapping_mul(17) ^ (index / 7)) & 0xff) as u8)
        .collect()
}

fn envelope(bytes: &[u8], chunk_values: usize) -> Vec<u8> {
    encode_qatq_transport(bytes, chunk_values, QatqTransportPolicy::default()).unwrap()
}

fn resign_encoded(envelope: &mut [u8]) {
    let digest: [u8; 32] = Sha256::digest(&envelope[HEADER_LEN..]).into();
    envelope[72..104].copy_from_slice(&digest);
}

fn digest_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}

fn temporary_directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("gcc-qatq-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn exact_round_trip_is_deterministic_for_boundary_lengths() {
    for length in [0, 1, 2, 3, 4, 5, 31, 32, 33, 4_097] {
        let input = fixture(length);
        let first = envelope(&input, 64);
        let second = envelope(&input, 64);
        let third = envelope(&input, 64);
        assert_eq!(first, second);
        assert_eq!(second, third);
        assert_eq!(
            decode_qatq_transport(&first, QatqTransportPolicy::default()).unwrap(),
            input
        );
        let metadata = inspect_qatq_transport(&first, QatqTransportPolicy::default()).unwrap();
        assert_eq!(metadata.canonical_bytes, length);
        assert_eq!(metadata.encoded_bytes + HEADER_LEN, first.len());
        assert_eq!(metadata.max_values_per_chunk, 64);
    }
}

#[test]
fn portable_fixture_has_frozen_envelope_identity() {
    let input = include_bytes!("fuzz-corpus/aiger/duplicate-symbol.aag");
    let encoded = envelope(input, 64);
    assert_eq!(
        digest_hex(&encoded),
        "9be12addcf5044e300b9c54a00b9fbf476879befa0046262994a7ab87ba8efe0"
    );
    assert_eq!(
        decode_qatq_transport(&encoded, QatqTransportPolicy::default()).unwrap(),
        input
    );
}

#[test]
fn integrity_and_canonical_encoding_mutations_fail_closed() {
    let original = envelope(&fixture(1_025), 32);
    for index in [
        0,
        8,
        10,
        12,
        16,
        24,
        32,
        36,
        40,
        72,
        103,
        original.len() - 1,
    ] {
        let mut mutated = original.clone();
        mutated[index] ^= 1;
        assert!(
            decode_qatq_transport(&mutated, QatqTransportPolicy::default()).is_err(),
            "accepted mutation at {index}"
        );
    }

    let mut trailing = original.clone();
    trailing.push(0);
    assert!(decode_qatq_transport(&trailing, QatqTransportPolicy::default()).is_err());

    let mut truncated = original.clone();
    truncated.pop();
    assert!(decode_qatq_transport(&truncated, QatqTransportPolicy::default()).is_err());

    let mut corrupt_qatq = original.clone();
    corrupt_qatq[HEADER_LEN + 4] ^= 1;
    resign_encoded(&mut corrupt_qatq);
    assert!(decode_qatq_transport(&corrupt_qatq, QatqTransportPolicy::default()).is_err());

    let mut irregular_chunks = original.clone();
    irregular_chunks[HEADER_LEN + 16..HEADER_LEN + 20].copy_from_slice(&1_u32.to_be_bytes());
    resign_encoded(&mut irregular_chunks);
    assert!(decode_qatq_transport(&irregular_chunks, QatqTransportPolicy::default()).is_err());
}

#[test]
fn non_zero_final_word_padding_is_rejected() {
    let mut encoded = envelope(&[1, 2, 3], 16);
    encoded[16..24].copy_from_slice(&2_u64.to_be_bytes());
    let digest: [u8; 32] = Sha256::digest([1_u8, 2]).into();
    encoded[40..72].copy_from_slice(&digest);
    assert!(decode_qatq_transport(&encoded, QatqTransportPolicy::default()).is_err());
}

#[test]
fn every_resource_policy_dimension_is_enforced() {
    let input = vec![0_u8; 16_384];
    let encoded = envelope(&input, 8);
    let base = QatqTransportPolicy::default();

    let policies = [
        (
            "envelope bytes",
            QatqTransportPolicy {
                max_envelope_bytes: encoded.len() - 1,
                ..base
            },
        ),
        (
            "decoded bytes",
            QatqTransportPolicy {
                max_decoded_bytes: input.len() - 1,
                ..base
            },
        ),
        (
            "chunks",
            QatqTransportPolicy {
                max_chunks: 1,
                ..base
            },
        ),
        (
            "chunk bytes",
            QatqTransportPolicy {
                max_encoded_chunk_bytes: 1,
                ..base
            },
        ),
        (
            "values per chunk",
            QatqTransportPolicy {
                max_values_per_chunk: 7,
                ..base
            },
        ),
    ];
    for (name, policy) in policies {
        assert!(
            decode_qatq_transport(&encoded, policy).is_err(),
            "accepted over-limit {name}"
        );
    }
    let compact = envelope(&input, 1_048_576);
    assert!(
        decode_qatq_transport(
            &compact,
            QatqTransportPolicy {
                max_expansion_ratio: 1,
                ..base
            }
        )
        .is_err(),
        "accepted over-limit expansion ratio"
    );

    let mut oversized_chunk = envelope(&fixture(1_024), 16);
    let first_qatq_value_count = HEADER_LEN + 32 + 4 + 8;
    oversized_chunk[first_qatq_value_count..first_qatq_value_count + 8]
        .copy_from_slice(&17_u64.to_be_bytes());
    resign_encoded(&mut oversized_chunk);
    assert!(
        decode_qatq_transport(&oversized_chunk, base).is_err(),
        "accepted a decoded chunk over the authenticated limit"
    );
}

#[test]
fn writer_failures_remain_typed_io_errors() {
    struct FailingWriter;
    impl io::Write for FailingWriter {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("injected failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let encoded = envelope(&fixture(128), 16);
    let error = decode_qatq_transport_to_writer(
        &encoded,
        QatqTransportPolicy::default(),
        &mut FailingWriter,
    )
    .unwrap_err();
    assert!(matches!(error, QatqTransportError::Io(_)));
}

#[test]
fn invalid_inputs_do_not_panic_or_silently_become_raw_bytes() {
    let original = envelope(&fixture(257), 16);
    for length in 0..HEADER_LEN {
        assert!(
            decode_qatq_transport(&original[..length], QatqTransportPolicy::default()).is_err()
        );
    }
    for index in 0..original.len() {
        let mut mutated = original.clone();
        mutated[index] ^= 0x80;
        let _ = decode_qatq_transport(&mutated, QatqTransportPolicy::default());
    }
    assert!(
        decode_qatq_transport(
            b"ordinary raw certificate bytes",
            QatqTransportPolicy::default()
        )
        .is_err()
    );
}

#[test]
fn atomic_file_decode_never_overwrites_and_cleans_temporary_files() {
    let directory = temporary_directory("atomic");
    let output = directory.join("canonical.batch");
    let sentinel = b"existing evidence must survive";
    fs::write(&output, sentinel).unwrap();
    let encoded = envelope(&fixture(4_097), 64);
    assert!(
        decode_qatq_transport_file_create_new(&encoded, &output, QatqTransportPolicy::default())
            .is_err()
    );
    assert_eq!(fs::read(&output).unwrap(), sentinel);

    fs::remove_file(&output).unwrap();
    let mut corrupt = encoded.clone();
    corrupt[40] ^= 1;
    assert!(
        decode_qatq_transport_file_create_new(&corrupt, &output, QatqTransportPolicy::default())
            .is_err()
    );
    assert!(!output.exists());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);

    decode_qatq_transport_file_create_new(&encoded, &output, QatqTransportPolicy::default())
        .unwrap();
    assert_eq!(fs::read(&output).unwrap(), fixture(4_097));
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(directory).unwrap();
}
