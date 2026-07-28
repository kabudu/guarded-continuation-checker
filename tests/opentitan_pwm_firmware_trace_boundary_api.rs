use guarded_continuation_checker::btor2;

#[test]
fn firmware_trace_boundary_keeps_every_channel_independently_driven() {
    let bytes = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
    );
    let model = btor2::parse_component_bytes(bytes, &[48, 74]).unwrap();
    assert_eq!(model.inputs().len(), 39);
    assert!(model.constraints().is_empty());
    assert!(model.bad_properties().is_empty());
    let widths = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].width)
        .collect::<Vec<_>>();
    assert_eq!(widths.iter().filter(|width| **width == 6).count(), 9);
    assert_eq!(widths.iter().filter(|width| **width == 4).count(), 30);
    assert!(widths.iter().all(|width| *width <= 64));
    let symbols = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].symbol.as_deref().unwrap())
        .collect::<Vec<_>>();
    for channel in 0..6 {
        for field in [
            "phase_delay",
            "duty_cycle_a",
            "duty_cycle_b",
            "blink_parameter_x",
            "blink_parameter_y",
        ] {
            assert!(symbols.contains(&format!("{field}_{channel}_i").as_str()));
        }
    }
}
