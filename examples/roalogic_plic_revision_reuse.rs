use guarded_continuation_checker::revision_local::{
    BoundedQuery, BoundedResult, ComponentSide, EvidenceSection,
    produce_revision_local_certificate, produce_revision_with_retained_left,
    unchanged_local_evidence, validate_local_artifact, verify_revision_local_certificate,
    verify_revision_with_retained_left,
};
use std::{env, fs, process};

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(format!(
            "usage: {} MONITOR.btor2 OLD_PLIC.btor2 NEW_PLIC.btor2 INTERFACE.txt",
            args[0]
        ));
    }
    let monitor = fs::read(&args[1]).map_err(|error| format!("read monitor: {error}"))?;
    let old_plic = fs::read(&args[2]).map_err(|error| format!("read old PLIC: {error}"))?;
    let new_plic = fs::read(&args[3]).map_err(|error| format!("read new PLIC: {error}"))?;
    let interface = fs::read(&args[4]).map_err(|error| format!("read interface: {error}"))?;

    println!(
        "schema_version,property,old_result,new_result,old_bad_frame,new_bad_frame,retained_bytes,retained_byte_identical,produced_local_sections,production_reused_local_sections,changed_candidate_valuations,decoded_local_sections,verified_local_sections,verification_reused_local_sections,composed_pair_checks,final_transition_checks,status"
    );
    for (property, bad_output, expected, expected_frame) in [
        ("repeated-pending", 7, BoundedResult::Unsafe, Some(2)),
        ("impossible", 8, BoundedResult::Safe, None),
    ] {
        let query = BoundedQuery {
            horizon: 2,
            bad_side: ComponentSide::Left,
            bad_output,
        };
        let (old, old_summary) = produce_revision_local_certificate(
            &monitor,
            &[7, 8],
            &old_plic,
            &[13],
            &interface,
            &query,
        )
        .map_err(|error| error.to_string())?;
        let retained = validate_local_artifact(&monitor, &old.left.evidence, EvidenceSection::Left)
            .map_err(|error| error.to_string())?;
        let (new, new_summary, production_work) =
            produce_revision_with_retained_left(&retained, &new_plic, &[13], &interface, &query)
                .map_err(|error| error.to_string())?;
        let old_checked = verify_revision_local_certificate(&monitor, &old_plic, &interface, &old)
            .map_err(|error| error.to_string())?;
        let (new_checked, work) =
            verify_revision_with_retained_left(&retained, &new_plic, &interface, &new)
                .map_err(|error| error.to_string())?;
        let identical = unchanged_local_evidence(&old, &new, EvidenceSection::Left)
            .map_err(|error| error.to_string())?;
        if !identical
            || old_summary.answer.result != expected
            || new_summary.answer.result != expected
            || old_checked.answer.result != expected
            || new_checked.answer.result != expected
            || old_checked.answer.bad_frame != expected_frame
            || new_checked.answer.bad_frame != expected_frame
            || work.decoded_local_sections != 1
            || work.semantically_verified_local_sections != 1
            || work.reused_local_sections != 1
            || production_work.produced_local_sections != 1
            || production_work.reused_local_sections != 1
        {
            return Err(format!("revision reuse acceptance failed for {property}"));
        }
        let result = match expected {
            BoundedResult::Safe => "SAFE",
            BoundedResult::Unsafe => "UNSAFE",
        };
        let frame = expected_frame.map_or_else(|| "none".to_string(), |value| value.to_string());
        println!(
            "1,{property},{result},{result},{frame},{frame},{},true,{},{},{},{},{},{},{},{},accepted",
            retained.encoded().len(),
            production_work.produced_local_sections,
            production_work.reused_local_sections,
            production_work.changed_candidate_valuations,
            work.decoded_local_sections,
            work.semantically_verified_local_sections,
            work.reused_local_sections,
            work.composed_pair_checks,
            work.final_transition_checks,
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(2);
    }
}
