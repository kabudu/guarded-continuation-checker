use guarded_continuation_checker::btor2;

#[test]
fn firmware_trace_boundary_keeps_every_channel_independently_driven() {
    let bytes = include_bytes!(
        "../corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
    );
    let model = btor2::parse_component_bytes(bytes, &[45, 71]).unwrap();
    assert_eq!(model.inputs().len(), 35);
    assert!(model.constraints().is_empty());
    assert!(model.bad_properties().is_empty());
    let widths = model
        .inputs()
        .iter()
        .map(|input| model.nodes()[input].width)
        .collect::<Vec<_>>();
    assert_eq!(&widths[..3], &[6, 6, 6]);
    assert!(widths[3..33].iter().all(|width| *width == 16));
    assert_eq!(&widths[33..], &[6, 6]);
    assert!(widths.iter().all(|width| *width <= 64));
}
