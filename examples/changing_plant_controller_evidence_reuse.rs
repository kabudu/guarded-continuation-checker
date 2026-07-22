use std::hint::black_box;
use std::time::Instant;

use guarded_continuation_checker::aiger_obligation::{
    AigerTransition, parse_ascii_aiger_transition,
};
use guarded_continuation_checker::controller_mtbdd::produce_controller_mtbdd;
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_artifact::{
    AdmittedControllerProofEvidence, ControllerPlantArtifactInput, admit_controller_proof_evidence,
    encode_bound_plant_results_artifact, encode_controller_proof_evidence_artifact,
    encode_controller_proof_mtbdd_plant_artifact,
    produce_bound_plant_results_with_admitted_controller,
    produce_controller_proof_evidence_artifact, produce_controller_proof_mtbdd_plant_artifact,
    verify_bound_plant_results_with_admitted_controller,
    verify_controller_proof_mtbdd_plant_artifact,
};
use sha2::{Digest, Sha256};

const CONTROLLER_SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
const CONTROLLER_MODEL: &[u8] =
    include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
const NOMINAL_SOURCE: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.v");
const NOMINAL_MODEL: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.aag");
const SENSOR_SOURCE: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/sensor-stuck/physical-plant.v"
);
const SENSOR_MODEL: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/sensor-stuck/physical-plant.aag"
);
const DELAY_SOURCE: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-delay/physical-plant.v"
);
const DELAY_MODEL: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-delay/physical-plant.aag"
);
const DISTURBANCE_SOURCE: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/persistent-disturbance/physical-plant.v"
);
const DISTURBANCE_MODEL: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/persistent-disturbance/physical-plant.aag"
);
const REPLACEMENT_SOURCE: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-transport-lag/physical-plant.v"
);
const REPLACEMENT_MODEL: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-transport-lag/physical-plant.aag"
);
const PROPERTIES: [usize; 2] = [15, 16];
const HORIZON: usize = 32;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex_digest(bytes: &[u8]) -> String {
    digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn inputs<'a>(
    plant: &'a AigerTransition,
    source: &[u8],
    wiring: &'a ControllerPlantWiring,
) -> Vec<ControllerPlantArtifactInput<'a>> {
    PROPERTIES
        .iter()
        .map(|&bad_plant_output| ControllerPlantArtifactInput {
            plant,
            plant_source_sha256: digest(source),
            wiring,
            initial_controller_state: 0,
            initial_plant_state: 0,
            bad_plant_output,
            horizon: HORIZON,
        })
        .collect()
}

fn produce_local(
    admitted: &AdmittedControllerProofEvidence,
    members: &[ControllerPlantArtifactInput<'_>],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(encode_bound_plant_results_artifact(
        &produce_bound_plant_results_with_admitted_controller(admitted, members)?,
    )?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = parse_ascii_aiger_transition(CONTROLLER_MODEL)?;
    let plants = [
        parse_ascii_aiger_transition(NOMINAL_MODEL)?,
        parse_ascii_aiger_transition(SENSOR_MODEL)?,
        parse_ascii_aiger_transition(DELAY_MODEL)?,
        parse_ascii_aiger_transition(DISTURBANCE_MODEL)?,
        parse_ascii_aiger_transition(REPLACEMENT_MODEL)?,
    ];
    let sources = [
        NOMINAL_SOURCE,
        SENSOR_SOURCE,
        DELAY_SOURCE,
        DISTURBANCE_SOURCE,
        REPLACEMENT_SOURCE,
    ];
    let controller_digest = digest(CONTROLLER_SOURCE);
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let mtbdd = produce_controller_mtbdd(
        &controller,
        controller_digest,
        &(1..12).collect::<Vec<_>>(),
        &[2, 6, 7, 9],
    )?;
    let evidence_bytes = encode_controller_proof_evidence_artifact(
        &produce_controller_proof_evidence_artifact(&controller, controller_digest, &mtbdd)?,
    )?;

    let admission_started = Instant::now();
    let admitted = admit_controller_proof_evidence(
        &controller,
        controller_digest,
        black_box(&evidence_bytes),
    )?;
    let admission_nanos = admission_started.elapsed().as_nanos();
    let member_inputs = plants
        .iter()
        .zip(sources)
        .map(|(plant, source)| inputs(plant, source, &wiring))
        .collect::<Vec<_>>();

    let initial_started = Instant::now();
    let initial_locals = member_inputs[..4]
        .iter()
        .map(|members| produce_local(&admitted, members))
        .collect::<Result<Vec<_>, _>>()?;
    let initial_production_nanos = initial_started.elapsed().as_nanos();
    let initial_bytes: usize = initial_locals.iter().map(Vec::len).sum();
    let unchanged_before = [
        digest(&initial_locals[0]),
        digest(&initial_locals[1]),
        digest(&initial_locals[3]),
    ];
    let verify_started = Instant::now();
    for (index, bytes) in initial_locals.iter().enumerate() {
        let summary = black_box(verify_bound_plant_results_with_admitted_controller(
            &admitted,
            &member_inputs[index],
            bytes,
        )?);
        if summary.safe != 2 || summary.unsafe_count != 0 {
            return Err(format!("initial plant {index} did not retain two SAFE results").into());
        }
    }
    let initial_verification_nanos = verify_started.elapsed().as_nanos();

    let replacement_started = Instant::now();
    let replacement_bytes = produce_local(&admitted, &member_inputs[4])?;
    let replacement_production_nanos = replacement_started.elapsed().as_nanos();
    let replacement_verify_started = Instant::now();
    let replacement_summary = black_box(verify_bound_plant_results_with_admitted_controller(
        &admitted,
        &member_inputs[4],
        &replacement_bytes,
    )?);
    let replacement_verification_nanos = replacement_verify_started.elapsed().as_nanos();
    if replacement_summary.safe != 2 || replacement_summary.unsafe_count != 0 {
        return Err("replacement plant did not retain two SAFE results".into());
    }
    let replacement_repeat = produce_local(&admitted, &member_inputs[4])?;
    if replacement_repeat != replacement_bytes {
        return Err("replacement plant evidence is nondeterministic".into());
    }

    let initial_flat = member_inputs[..4]
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let mut replaced_flat = initial_flat.clone();
    replaced_flat.splice(4..6, member_inputs[4].iter().copied());
    let monolithic_initial = encode_controller_proof_mtbdd_plant_artifact(
        &produce_controller_proof_mtbdd_plant_artifact(
            &controller,
            controller_digest,
            &mtbdd,
            &initial_flat,
        )?,
    )?;
    let monolithic_replaced = encode_controller_proof_mtbdd_plant_artifact(
        &produce_controller_proof_mtbdd_plant_artifact(
            &controller,
            controller_digest,
            &mtbdd,
            &replaced_flat,
        )?,
    )?;
    let replaced_sources = [0usize, 1, 4, 3]
        .into_iter()
        .flat_map(|index| {
            let source = digest(sources[index]);
            [(&plants[index], source), (&plants[index], source)]
        })
        .collect::<Vec<_>>();
    verify_controller_proof_mtbdd_plant_artifact(
        &controller,
        controller_digest,
        &replaced_sources,
        &monolithic_replaced,
    )?;

    let unchanged_after = [0usize, 1, 3]
        .into_iter()
        .map(|index| produce_local(&admitted, &member_inputs[index]).map(|bytes| digest(&bytes)))
        .collect::<Result<Vec<_>, _>>()?;
    if unchanged_after != unchanged_before {
        return Err("unchanged plant evidence drifted".into());
    }
    let split_marginal_bytes =
        REPLACEMENT_SOURCE.len() + REPLACEMENT_MODEL.len() + replacement_bytes.len();
    let monolithic_marginal_bytes =
        REPLACEMENT_SOURCE.len() + REPLACEMENT_MODEL.len() + monolithic_replaced.len();
    println!(
        "schema_version,controller_evidence_bytes,controller_evidence_sha256,initial_plant_result_bytes,replacement_source_bytes,replacement_model_bytes,replacement_result_bytes,replacement_result_sha256,split_marginal_bytes,monolithic_initial_bytes,monolithic_replacement_bytes,monolithic_marginal_bytes,marginal_byte_ratio,admission_nanos,initial_production_nanos,initial_verification_nanos,replacement_production_nanos,replacement_verification_nanos,initial_safe,replacement_safe,deterministic,unchanged_members_identical,status"
    );
    println!(
        "1,{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},8,2,true,true,validated",
        evidence_bytes.len(),
        hex_digest(&evidence_bytes),
        initial_bytes,
        REPLACEMENT_SOURCE.len(),
        REPLACEMENT_MODEL.len(),
        replacement_bytes.len(),
        hex_digest(&replacement_bytes),
        split_marginal_bytes,
        monolithic_initial.len(),
        monolithic_replaced.len(),
        monolithic_marginal_bytes,
        split_marginal_bytes as f64 / monolithic_marginal_bytes as f64,
        admission_nanos,
        initial_production_nanos,
        initial_verification_nanos,
        replacement_production_nanos,
        replacement_verification_nanos,
    );
    Ok(())
}
