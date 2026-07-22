#[cfg(not(feature = "research-qatq-transport"))]
fn main() {
    eprintln!("error: build with --features research-qatq-transport");
    std::process::exit(2);
}

#[cfg(feature = "research-qatq-transport")]
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

#[cfg(feature = "research-qatq-transport")]
fn run() -> Result<(), String> {
    use guarded_continuation_checker::qatq_transport::{
        QatqTransportPolicy, decode_qatq_transport, encode_qatq_transport, inspect_qatq_transport,
    };
    use sha2::{Digest, Sha256};
    use std::{env, fs, io::Write, time::Instant};

    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(format!(
            "usage: {} INPUT OUTPUT.csv TRIALS MAX_VALUES_PER_CHUNK",
            args[0]
        ));
    }
    let trials = args[3]
        .parse::<usize>()
        .map_err(|_| "TRIALS must be an integer".to_string())?;
    if !(5..=21).contains(&trials) {
        return Err("TRIALS must be between 5 and 21".to_string());
    }
    let chunk_values = args[4]
        .parse::<usize>()
        .map_err(|_| "MAX_VALUES_PER_CHUNK must be an integer".to_string())?;
    let input = fs::read(&args[1]).map_err(|error| format!("read input: {error}"))?;
    let policy = QatqTransportPolicy::default();

    let first =
        encode_qatq_transport(&input, chunk_values, policy).map_err(|error| error.to_string())?;
    for _ in 0..2 {
        let repeated = encode_qatq_transport(&input, chunk_values, policy)
            .map_err(|error| error.to_string())?;
        if repeated != first {
            return Err("QatQ transport encoding is not deterministic".to_string());
        }
    }
    let metadata = inspect_qatq_transport(&first, policy).map_err(|error| error.to_string())?;

    let mut encode_nanos = Vec::with_capacity(trials);
    let mut decode_nanos = Vec::with_capacity(trials);
    for _ in 0..trials {
        let started = Instant::now();
        let encoded = encode_qatq_transport(&input, chunk_values, policy)
            .map_err(|error| error.to_string())?;
        encode_nanos.push(started.elapsed().as_nanos());
        if encoded != first {
            return Err("timed encoding differs from canonical encoding".to_string());
        }

        let started = Instant::now();
        let decoded = decode_qatq_transport(&encoded, policy).map_err(|error| error.to_string())?;
        decode_nanos.push(started.elapsed().as_nanos());
        if decoded != input {
            return Err("timed decode differs from canonical input".to_string());
        }
    }
    encode_nanos.sort_unstable();
    decode_nanos.sort_unstable();
    let canonical_hash: [u8; 32] = Sha256::digest(&input).into();
    let envelope_hash: [u8; 32] = Sha256::digest(&first).into();

    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])
        .map_err(|error| format!("create output: {error}"))?;
    writeln!(
        output,
        "schema_version,os,arch,raw_bytes,envelope_bytes,qatq_payload_bytes,ratio_to_raw,max_values_per_chunk,trials,encode_min_ns,encode_median_ns,encode_max_ns,decode_min_ns,decode_median_ns,decode_max_ns,canonical_sha256,envelope_sha256,deterministic,bit_identical,status"
    )
    .map_err(|error| format!("write output: {error}"))?;
    writeln!(
        output,
        "1,{},{},{},{},{},{:.9},{},{},{},{},{},{},{},{},{},{},true,true,measured",
        env::consts::OS,
        env::consts::ARCH,
        input.len(),
        first.len(),
        metadata.encoded_bytes,
        first.len() as f64 / input.len().max(1) as f64,
        chunk_values,
        trials,
        encode_nanos[0],
        encode_nanos[trials / 2],
        encode_nanos[trials - 1],
        decode_nanos[0],
        decode_nanos[trials / 2],
        decode_nanos[trials - 1],
        hex(&canonical_hash),
        hex(&envelope_hash),
    )
    .map_err(|error| format!("write output: {error}"))?;
    output
        .sync_all()
        .map_err(|error| format!("sync output: {error}"))?;
    Ok(())
}

#[cfg(feature = "research-qatq-transport")]
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("write to string");
    }
    output
}
