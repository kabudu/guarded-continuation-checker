use guarded_continuation_checker::revision_local::{
    BoundedQuery, BoundedResult, ComponentSide, EvidenceSection, InterfaceWire,
    WordInterfaceContract, encode_revision_local_certificate, encode_word_interface_contract,
    produce_revision_local_certificate, produce_revision_with_retained_components,
    produce_revision_with_retained_left, validate_local_artifact,
    verify_revision_local_certificate, verify_revision_with_retained_components,
    verify_revision_with_retained_left,
};
use std::{env, fs, process, time::Instant};

const PROPERTY_ROOTS: &[u64] = &[1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007];
const ENVIRONMENT_OUTPUTS: &[u64] = &[2, 3, 4, 9, 12];
const COMPONENT_OUTPUTS: &[u64] = PROPERTY_ROOTS;

fn query() -> BoundedQuery {
    BoundedQuery {
        horizon: 0,
        bad_side: ComponentSide::Left,
        bad_output: 2,
    }
}

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
    if args.len() != 4 {
        return Err(format!(
            "usage: {} ENVIRONMENT.btor2 BEFORE.btor2 AFTER.btor2",
            args[0]
        ));
    }
    let environment = fs::read(&args[1]).map_err(|error| format!("read environment: {error}"))?;
    let before = fs::read(&args[2]).map_err(|error| format!("read before: {error}"))?;
    let after = fs::read(&args[3]).map_err(|error| format!("read after: {error}"))?;
    let interfaces = PROPERTY_ROOTS
        .iter()
        .map(|root| interface(*root))
        .collect::<Result<Vec<_>, _>>()?;

    let full_started = Instant::now();
    let mut full = Vec::new();
    let mut full_candidate_valuations = 0usize;
    let mut artifact_bytes = 0usize;
    for source in [&before, &after] {
        for interface in &interfaces {
            let (certificate, summary) = produce_revision_local_certificate(
                &environment,
                ENVIRONMENT_OUTPUTS,
                source,
                COMPONENT_OUTPUTS,
                interface,
                &query(),
            )
            .map_err(|error| error.to_string())?;
            full_candidate_valuations +=
                summary.left.candidate_valuations + summary.right.candidate_valuations;
            artifact_bytes += summary.certificate_bytes;
            full.push(certificate);
        }
    }
    let full_produce_nanos = full_started.elapsed().as_nanos();

    let service_started = Instant::now();
    let (initial, initial_summary) = produce_revision_local_certificate(
        &environment,
        ENVIRONMENT_OUTPUTS,
        &before,
        COMPONENT_OUTPUTS,
        &interfaces[0],
        &query(),
    )
    .map_err(|error| error.to_string())?;
    let retained_environment =
        validate_local_artifact(&environment, &initial.left.evidence, EvidenceSection::Left)
            .map_err(|error| error.to_string())?;
    let retained_before =
        validate_local_artifact(&before, &initial.right.evidence, EvidenceSection::Right)
            .map_err(|error| error.to_string())?;
    let mut service = vec![initial];
    let mut produced_sections = 2usize;
    let mut reused_sections = 0usize;
    let mut service_candidate_valuations =
        initial_summary.left.candidate_valuations + initial_summary.right.candidate_valuations;
    for interface in &interfaces[1..] {
        let (certificate, _, work) = produce_revision_with_retained_components(
            &retained_environment,
            &retained_before,
            interface,
            &query(),
        )
        .map_err(|error| error.to_string())?;
        produced_sections += work.produced_local_sections;
        reused_sections += work.reused_local_sections;
        service_candidate_valuations += work.changed_candidate_valuations;
        service.push(certificate);
    }
    let (first_after, _, transition_work) = produce_revision_with_retained_left(
        &retained_environment,
        &after,
        COMPONENT_OUTPUTS,
        &interfaces[0],
        &query(),
    )
    .map_err(|error| error.to_string())?;
    produced_sections += transition_work.produced_local_sections;
    reused_sections += transition_work.reused_local_sections;
    service_candidate_valuations += transition_work.changed_candidate_valuations;
    let retained_after =
        validate_local_artifact(&after, &first_after.right.evidence, EvidenceSection::Right)
            .map_err(|error| error.to_string())?;
    service.push(first_after);
    for interface in &interfaces[1..] {
        let (certificate, _, work) = produce_revision_with_retained_components(
            &retained_environment,
            &retained_after,
            interface,
            &query(),
        )
        .map_err(|error| error.to_string())?;
        produced_sections += work.produced_local_sections;
        reused_sections += work.reused_local_sections;
        service_candidate_valuations += work.changed_candidate_valuations;
        service.push(certificate);
    }
    let service_produce_nanos = service_started.elapsed().as_nanos();

    for (index, (full_certificate, service_certificate)) in full.iter().zip(&service).enumerate() {
        if encode_revision_local_certificate(full_certificate).map_err(|e| e.to_string())?
            != encode_revision_local_certificate(service_certificate).map_err(|e| e.to_string())?
        {
            return Err(format!(
                "property {index} artifact differs from full rebuild"
            ));
        }
    }

    let full_verify_started = Instant::now();
    for (index, certificate) in full.iter().enumerate() {
        let source = if index < PROPERTY_ROOTS.len() {
            &before
        } else {
            &after
        };
        verify_revision_local_certificate(
            &environment,
            source,
            &interfaces[index % PROPERTY_ROOTS.len()],
            certificate,
        )
        .map_err(|error| error.to_string())?;
    }
    let full_verify_nanos = full_verify_started.elapsed().as_nanos();

    let service_verify_started = Instant::now();
    let mut verification_reused_sections = 0usize;
    for (index, certificate) in service[..PROPERTY_ROOTS.len()].iter().enumerate() {
        let (_, work) = verify_revision_with_retained_components(
            &retained_environment,
            &retained_before,
            &interfaces[index],
            certificate,
        )
        .map_err(|error| error.to_string())?;
        verification_reused_sections += work.reused_local_sections;
    }
    let (_, transition_verification) = verify_revision_with_retained_left(
        &retained_environment,
        &after,
        &interfaces[0],
        &service[PROPERTY_ROOTS.len()],
    )
    .map_err(|error| error.to_string())?;
    verification_reused_sections += transition_verification.reused_local_sections;
    for (offset, certificate) in service[(PROPERTY_ROOTS.len() + 1)..].iter().enumerate() {
        let (_, work) = verify_revision_with_retained_components(
            &retained_environment,
            &retained_after,
            &interfaces[offset + 1],
            certificate,
        )
        .map_err(|error| error.to_string())?;
        verification_reused_sections += work.reused_local_sections;
    }
    let service_verify_nanos = service_verify_started.elapsed().as_nanos();

    let mut before_safe = 0usize;
    let mut before_unsafe = 0usize;
    let mut after_safe = 0usize;
    let mut after_unsafe = 0usize;
    for (index, certificate) in service.iter().enumerate() {
        let before_revision = index < PROPERTY_ROOTS.len();
        let summary = if before_revision {
            verify_revision_with_retained_components(
                &retained_environment,
                &retained_before,
                &interfaces[index],
                certificate,
            )
            .map_err(|error| error.to_string())?
            .0
        } else if index == PROPERTY_ROOTS.len() {
            verify_revision_with_retained_left(
                &retained_environment,
                &after,
                &interfaces[0],
                certificate,
            )
            .map_err(|error| error.to_string())?
            .0
        } else {
            verify_revision_with_retained_components(
                &retained_environment,
                &retained_after,
                &interfaces[index - PROPERTY_ROOTS.len()],
                certificate,
            )
            .map_err(|error| error.to_string())?
            .0
        };
        match (before_revision, summary.answer.result) {
            (true, BoundedResult::Safe) => before_safe += 1,
            (true, BoundedResult::Unsafe) => before_unsafe += 1,
            (false, BoundedResult::Safe) => after_safe += 1,
            (false, BoundedResult::Unsafe) => after_unsafe += 1,
        }
    }
    if (before_safe, before_unsafe, after_safe, after_unsafe) != (7, 1, 5, 3) {
        return Err(format!(
            "unexpected answer classes: before={before_safe}/{before_unsafe} after={after_safe}/{after_unsafe}"
        ));
    }

    println!(
        "schema_version,revisions,properties_per_revision,total_queries,artifact_bytes,full_candidate_valuations,service_candidate_valuations,candidate_reduction_ratio,service_produced_sections,service_reused_sections,verification_reused_sections,full_produce_nanos,service_produce_nanos,produce_ratio,full_verify_nanos,service_verify_nanos,verify_ratio,artifacts_identical,before_safe,before_unsafe,after_safe,after_unsafe,status"
    );
    println!(
        "1,2,{},{},{artifact_bytes},{full_candidate_valuations},{service_candidate_valuations},{:.6},{produced_sections},{reused_sections},{verification_reused_sections},{full_produce_nanos},{service_produce_nanos},{:.6},{full_verify_nanos},{service_verify_nanos},{:.6},true,{before_safe},{before_unsafe},{after_safe},{after_unsafe},measured",
        PROPERTY_ROOTS.len(),
        PROPERTY_ROOTS.len() * 2,
        service_candidate_valuations as f64 / full_candidate_valuations as f64,
        service_produce_nanos as f64 / full_produce_nanos as f64,
        service_verify_nanos as f64 / full_verify_nanos as f64,
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(2);
    }
}
