use std::fs;
use std::path::PathBuf;

use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_plant::ControllerPlantWiring;
use guarded_continuation_checker::controller_plant_aiger::{
    export_bounded_controller_plant_aag, export_bounded_controller_plant_multi_aag,
};
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
        .ok_or("usage: export_composed_witness_plant_family OUTPUT_DIR")?;
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }
    fs::create_dir(&output)?;
    let controller_bytes = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
    let controller = parse_ascii_aiger_transition(controller_bytes)?;
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let plants: [(&str, &[u8], &[u8]); 5] = [
        (
            "nominal",
            include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.v"),
            include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.aag"),
        ),
        (
            "sensor-stuck",
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/sensor-stuck/physical-plant.v"
            ),
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/sensor-stuck/physical-plant.aag"
            ),
        ),
        (
            "actuator-delay",
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-delay/physical-plant.v"
            ),
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-delay/physical-plant.aag"
            ),
        ),
        (
            "persistent-disturbance",
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/persistent-disturbance/physical-plant.v"
            ),
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/persistent-disturbance/physical-plant.aag"
            ),
        ),
        (
            "actuator-transport-lag",
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-transport-lag/physical-plant.v"
            ),
            include_bytes!(
                "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-transport-lag/physical-plant.aag"
            ),
        ),
    ];
    let mut manifest = format!(
        "format=composed-witness-changing-plants-v1\nexport_version=1\ncontroller_aag_sha256={}\nhorizon=32\nproperty_count=2\nproperty=15\nproperty=16\nmember_count=5\n",
        digest(controller_bytes),
    );
    for (name, source, bytes) in plants {
        let plant = parse_ascii_aiger_transition(bytes)?;
        let export = export_bounded_controller_plant_multi_aag(
            &controller,
            &plant,
            &wiring,
            0,
            0,
            &[15, 16],
            32,
        )?;
        let file = format!("{name}.aag");
        fs::write(output.join(&file), &export.bytes)?;
        let property_15 =
            export_bounded_controller_plant_aag(&controller, &plant, &wiring, 0, 0, 15, 32)?;
        let property_16 =
            export_bounded_controller_plant_aag(&controller, &plant, &wiring, 0, 0, 16, 32)?;
        let property_15_file = format!("{name}-property-15.aag");
        let property_16_file = format!("{name}-property-16.aag");
        fs::write(output.join(&property_15_file), &property_15.bytes)?;
        fs::write(output.join(&property_16_file), &property_16.bytes)?;
        manifest.push_str(&format!(
            "member={name},{file},{},{},{},{property_15_file},{},{property_16_file},{},safe,safe\n",
            digest(source),
            digest(bytes),
            digest(&export.bytes),
            digest(&property_15.bytes),
            digest(&property_16.bytes),
        ));
    }
    manifest.push_str("status=complete\n");
    fs::write(output.join("manifest-v1.txt"), manifest)?;
    fs::write(
        output.join("README.md"),
        "# Composed-witness model family v1\n\nThese deterministic AIGER exports pair one shared two-property model with the corresponding property-15 and property-16 single-property models for each changing plant. Frames 0 through 32 preserve the bounded query; the next state is absorbing and suppresses bad outputs. Four models form the original comparison package and `actuator-transport-lag` is the predeclared third-member replacement. The files are generated by `export_composed_witness_plant_family` and independently replayed by the integration tests. They are comparison fixtures, not external product evidence.\n",
    )?;
    Ok(())
}
