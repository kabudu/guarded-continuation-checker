use guarded_continuation_checker::revision_batch::{
    RevisionBatchComponent, RevisionBatchQuery, encode_revision_batch,
    extract_revision_batch_certificates, produce_revision_batch, verify_revision_batch,
};
use guarded_continuation_checker::revision_local::{
    BoundedQuery, BoundedResult, ComponentSide, InterfaceWire, WordInterfaceContract,
    encode_revision_local_certificate, encode_word_interface_contract,
    produce_revision_local_certificate,
};
use std::{env, fs, io::Write, process, time::Instant};

const PROPERTY_ROOTS: &[u64] = &[1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007];
const ENVIRONMENT_OUTPUTS: &[u64] = &[2, 3, 4, 9, 12];

fn interface(property_root: u64) -> Result<Vec<u8>, String> {
    encode_word_interface_contract(&WordInterfaceContract {
        wires: vec![
            InterfaceWire {
                from: ComponentSide::Left,
                output: 3,
                to_input: 4,
            },
            InterfaceWire {
                from: ComponentSide::Left,
                output: 3,
                to_input: 6,
            },
            InterfaceWire {
                from: ComponentSide::Left,
                output: 3,
                to_input: 7,
            },
            InterfaceWire {
                from: ComponentSide::Left,
                output: 4,
                to_input: 8,
            },
            InterfaceWire {
                from: ComponentSide::Left,
                output: 9,
                to_input: 2,
            },
            InterfaceWire {
                from: ComponentSide::Left,
                output: 12,
                to_input: 5,
            },
            InterfaceWire {
                from: ComponentSide::Right,
                output: property_root,
                to_input: 2,
            },
        ],
        external_inputs: Some(Vec::new()),
    })
    .map(|text| text.into_bytes())
    .map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if !(4..=5).contains(&args.len()) {
        return Err(format!(
            "usage: {} ENVIRONMENT.btor2 BEFORE.btor2 AFTER.btor2 [OUTPUT.batch]",
            args[0]
        ));
    }
    let environment = fs::read(&args[1]).map_err(|error| format!("read environment: {error}"))?;
    let before = fs::read(&args[2]).map_err(|error| format!("read before: {error}"))?;
    let after = fs::read(&args[3]).map_err(|error| format!("read after: {error}"))?;
    let interfaces = PROPERTY_ROOTS
        .iter()
        .copied()
        .map(interface)
        .collect::<Result<Vec<_>, _>>()?;
    let query = BoundedQuery {
        horizon: 0,
        bad_side: ComponentSide::Left,
        bad_output: 2,
    };

    let mut standalone = Vec::new();
    for source in [&before, &after] {
        for interface in &interfaces {
            let (certificate, _) = produce_revision_local_certificate(
                &environment,
                ENVIRONMENT_OUTPUTS,
                source,
                PROPERTY_ROOTS,
                interface,
                &query,
            )
            .map_err(|error| error.to_string())?;
            standalone
                .push(encode_revision_local_certificate(&certificate).map_err(|e| e.to_string())?);
        }
    }
    standalone.sort();
    let standalone_bytes = standalone.iter().map(Vec::len).sum::<usize>();

    let components = [
        RevisionBatchComponent {
            source: &environment,
            outputs: ENVIRONMENT_OUTPUTS,
        },
        RevisionBatchComponent {
            source: &before,
            outputs: PROPERTY_ROOTS,
        },
        RevisionBatchComponent {
            source: &after,
            outputs: PROPERTY_ROOTS,
        },
    ];
    let mut requests = Vec::new();
    for right_component in [1usize, 2usize] {
        for interface_source in &interfaces {
            requests.push(RevisionBatchQuery {
                left_component: 0,
                right_component,
                interface_source,
                query: query.clone(),
            });
        }
    }

    let produce_started = Instant::now();
    let (batch, production) =
        produce_revision_batch(&components, &requests).map_err(|error| error.to_string())?;
    let batch_bytes = encode_revision_batch(&batch).map_err(|error| error.to_string())?;
    let produce_nanos = produce_started.elapsed().as_nanos();
    if let Some(output) = args.get(4) {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|error| format!("create batch output: {error}"))?;
        file.write_all(&batch_bytes)
            .map_err(|error| format!("write batch output: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("sync batch output: {error}"))?;
    }

    let verify_started = Instant::now();
    let verified = verify_revision_batch(&[&environment, &before, &after], &batch_bytes)
        .map_err(|error| error.to_string())?;
    let verify_nanos = verify_started.elapsed().as_nanos();
    let safe = verified
        .answers
        .iter()
        .filter(|answer| answer.result == BoundedResult::Safe)
        .count();
    let unsafe_count = verified.answers.len() - safe;
    if (safe, unsafe_count) != (12, 4) {
        return Err(format!(
            "unexpected answer distribution {safe}/{unsafe_count}"
        ));
    }

    let mut extracted = extract_revision_batch_certificates(&batch_bytes)
        .map_err(|error| error.to_string())?
        .iter()
        .map(encode_revision_local_certificate)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    extracted.sort();
    if extracted != standalone {
        return Err("batch extraction differs from standalone certificates".to_string());
    }

    println!(
        "schema_version,revisions,properties_per_revision,total_queries,shared_sections,standalone_bytes,batch_bytes,batch_to_standalone_ratio,bytes_saved,candidate_valuations,produce_nanos,verify_nanos,safe,unsafe,extraction_identical,status"
    );
    println!(
        "1,2,8,16,{},{standalone_bytes},{},{:.6},{},{},{produce_nanos},{verify_nanos},{safe},{unsafe_count},true,measured",
        production.shared_sections,
        batch_bytes.len(),
        batch_bytes.len() as f64 / standalone_bytes as f64,
        standalone_bytes - batch_bytes.len(),
        production.candidate_valuations,
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(2);
    }
}
