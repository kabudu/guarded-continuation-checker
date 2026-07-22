use guarded_continuation_checker::revision_local::{
    BoundedQuery, ComponentSide, EvidenceSection, encode_revision_local_certificate,
    produce_revision_local_certificate, produce_revision_with_retained_left,
    validate_local_artifact, verify_revision_local_certificate, verify_revision_with_retained_left,
};
use std::{env, fs, hint::black_box, process, time::Instant};

const TRIALS: usize = 21;

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(format!(
            "usage: {} PLIC.btor2 OLD_MONITOR.btor2 NEW_MONITOR.btor2 INTERFACE.txt",
            args[0]
        ));
    }
    let plic = fs::read(&args[1]).map_err(|error| format!("read PLIC: {error}"))?;
    let old_monitor = fs::read(&args[2]).map_err(|error| format!("read old monitor: {error}"))?;
    let new_monitor = fs::read(&args[3]).map_err(|error| format!("read new monitor: {error}"))?;
    let interface = fs::read(&args[4]).map_err(|error| format!("read interface: {error}"))?;
    let query = BoundedQuery {
        horizon: 2,
        bad_side: ComponentSide::Right,
        bad_output: 8,
    };
    let (previous, _) = produce_revision_local_certificate(
        &plic,
        &[13],
        &old_monitor,
        &[7, 8],
        &interface,
        &BoundedQuery {
            horizon: 2,
            bad_side: ComponentSide::Right,
            bad_output: 7,
        },
    )
    .map_err(|error| error.to_string())?;
    let retained = validate_local_artifact(&plic, &previous.left.evidence, EvidenceSection::Left)
        .map_err(|error| error.to_string())?;

    let (full_reference, full_summary) =
        produce_revision_local_certificate(&plic, &[13], &new_monitor, &[8, 9], &interface, &query)
            .map_err(|error| error.to_string())?;
    let (reuse_reference, reuse_summary, reuse_work) =
        produce_revision_with_retained_left(&retained, &new_monitor, &[8, 9], &interface, &query)
            .map_err(|error| error.to_string())?;
    let full_bytes =
        encode_revision_local_certificate(&full_reference).map_err(|error| error.to_string())?;
    let reuse_bytes =
        encode_revision_local_certificate(&reuse_reference).map_err(|error| error.to_string())?;
    if full_bytes != reuse_bytes || full_summary.answer != reuse_summary.answer {
        return Err("full and retained production artifacts differ".to_string());
    }
    verify_revision_local_certificate(&plic, &new_monitor, &interface, &full_reference)
        .map_err(|error| error.to_string())?;
    verify_revision_with_retained_left(&retained, &new_monitor, &interface, &reuse_reference)
        .map_err(|error| error.to_string())?;

    let mut full_produce = Vec::with_capacity(TRIALS);
    let mut reuse_produce = Vec::with_capacity(TRIALS);
    let mut full_verify = Vec::with_capacity(TRIALS);
    let mut reuse_verify = Vec::with_capacity(TRIALS);
    for trial in 0..TRIALS {
        if trial % 2 == 0 {
            let started = Instant::now();
            black_box(
                produce_revision_local_certificate(
                    &plic,
                    &[13],
                    &new_monitor,
                    &[8, 9],
                    &interface,
                    &query,
                )
                .map_err(|error| error.to_string())?,
            );
            full_produce.push(started.elapsed().as_nanos());
            let started = Instant::now();
            black_box(
                produce_revision_with_retained_left(
                    &retained,
                    &new_monitor,
                    &[8, 9],
                    &interface,
                    &query,
                )
                .map_err(|error| error.to_string())?,
            );
            reuse_produce.push(started.elapsed().as_nanos());
        } else {
            let started = Instant::now();
            black_box(
                produce_revision_with_retained_left(
                    &retained,
                    &new_monitor,
                    &[8, 9],
                    &interface,
                    &query,
                )
                .map_err(|error| error.to_string())?,
            );
            reuse_produce.push(started.elapsed().as_nanos());
            let started = Instant::now();
            black_box(
                produce_revision_local_certificate(
                    &plic,
                    &[13],
                    &new_monitor,
                    &[8, 9],
                    &interface,
                    &query,
                )
                .map_err(|error| error.to_string())?,
            );
            full_produce.push(started.elapsed().as_nanos());
        }

        let started = Instant::now();
        black_box(
            verify_revision_local_certificate(&plic, &new_monitor, &interface, &full_reference)
                .map_err(|error| error.to_string())?,
        );
        full_verify.push(started.elapsed().as_nanos());
        let started = Instant::now();
        black_box(
            verify_revision_with_retained_left(
                &retained,
                &new_monitor,
                &interface,
                &reuse_reference,
            )
            .map_err(|error| error.to_string())?,
        );
        reuse_verify.push(started.elapsed().as_nanos());
    }

    let full_produce = median(full_produce);
    let reuse_produce = median(reuse_produce);
    let full_verify = median(full_verify);
    let reuse_verify = median(reuse_verify);
    println!(
        "schema_version,trials,artifact_bytes,full_candidate_valuations,retained_candidate_valuations,retained_produced_sections,retained_reused_sections,full_produce_median_nanos,retained_produce_median_nanos,produce_ratio,full_verify_median_nanos,retained_verify_median_nanos,verify_ratio,answers_agree,artifacts_identical,status"
    );
    println!(
        "1,{TRIALS},{},{},{},{},{},{full_produce},{reuse_produce},{:.6},{full_verify},{reuse_verify},{:.6},true,true,measured",
        full_bytes.len(),
        full_summary.left.candidate_valuations + full_summary.right.candidate_valuations,
        reuse_work.changed_candidate_valuations,
        reuse_work.produced_local_sections,
        reuse_work.reused_local_sections,
        reuse_produce as f64 / full_produce as f64,
        reuse_verify as f64 / full_verify as f64,
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(2);
    }
}
