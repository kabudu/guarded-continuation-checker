use std::fs;
use std::path::PathBuf;

use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantWiring, compose_controller_plant_direct,
};
use guarded_continuation_checker::controller_plant_aiger::export_bounded_controller_plant_aag;
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export_washing_controller_certified_baseline OUTPUT_DIR")?;
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }
    fs::create_dir(&output)?;
    let controller_source = include_bytes!("../corpus/rtl/wmcontroller/upstream/Controller.v");
    let controller_bytes = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
    let plant_source = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.v");
    let plant_bytes = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.aag");
    let controller = parse_ascii_aiger_transition(controller_bytes)?;
    let plant = parse_ascii_aiger_transition(plant_bytes)?;
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let mut manifest = format!(
        "format=controller-plant-certified-baseline-v1\nexport_version=1\ncontroller_source_sha256={}\ncontroller_aag_sha256={}\nplant_source_sha256={}\nplant_aag_sha256={}\nhorizon=32\nmember_count=6\n",
        digest(controller_source),
        digest(controller_bytes),
        digest(plant_source),
        digest(plant_bytes)
    );
    for bad_output in 11..17 {
        let result =
            compose_controller_plant_direct(&controller, &plant, &wiring, 0, 0, bad_output, 32)?;
        let export = export_bounded_controller_plant_aag(
            &controller,
            &plant,
            &wiring,
            0,
            0,
            bad_output,
            32,
        )?;
        let name = format!("property-{bad_output}.aag");
        fs::write(output.join(&name), &export.bytes)?;
        let answer = format!("{:?}", result.answer).to_ascii_lowercase();
        let bad_frame = result
            .bad_frame
            .map_or_else(|| "none".to_string(), |frame| frame.to_string());
        manifest.push_str(&format!(
            "member={bad_output},{name},{answer},{bad_frame},{}\n",
            digest(&export.bytes)
        ));
    }
    manifest.push_str("status=complete\n");
    fs::write(output.join("manifest-v1.txt"), manifest)?;
    Ok(())
}
