use guarded_continuation_checker::btor2;

#[test]
fn firmware_trace_boundary_keeps_every_channel_independently_driven() {
    let bytes = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
    );
    let model = btor2::parse_component_bytes(bytes, &[49, 75]).unwrap();
    assert_eq!(model.inputs().len(), 39);
    assert!(model.constraints().is_empty());
    assert!(model.bad_properties().is_empty());
    let widths = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].width)
        .collect::<Vec<_>>();
    assert!(widths[..7].iter().all(|width| *width == 6));
    assert!(widths[7..37].iter().all(|width| *width == 16));
    assert_eq!(&widths[37..], &[6, 6]);
    assert!(widths.iter().all(|width| *width <= 64));
}
