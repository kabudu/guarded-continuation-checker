use guarded_continuation_checker::btor2;
use guarded_continuation_checker::btor2_region_equivalence::derive_btor2_region_equivalence;
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;

struct Fixture {
    bytes: &'static [u8],
    roots: &'static [u64],
    channels: usize,
    states: usize,
    classes: &'static [&'static [usize]],
}

#[test]
fn symbolic_firmware_class_inputs_admit_only_exact_structural_classes() {
    let fixtures = [
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-2.btor2"
            ),
            roots: &[9, 20],
            channels: 2,
            states: 17,
            classes: &[&[0], &[1]],
        },
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-4.btor2"
            ),
            roots: &[9, 29],
            channels: 4,
            states: 25,
            classes: &[&[0, 2], &[1], &[3]],
        },
        Fixture {
            bytes: include_bytes!(
                "../corpus/rtl/opentitan-pwm-channel-family/generated/symbolic-class-6.btor2"
            ),
            roots: &[9, 39],
            channels: 6,
            states: 33,
            classes: &[&[0, 2, 4], &[1], &[3, 5]],
        },
    ];
    for fixture in fixtures {
        let model = btor2::parse_component_bytes(fixture.bytes, fixture.roots).unwrap();
        assert_eq!(model.inputs().len(), 3);
        assert_eq!(model.states().len(), fixture.states);
        assert!(model.constraints().is_empty());
        assert!(model.bad_properties().is_empty());
        let summary = derive_btor2_region_equivalence(
            fixture.bytes,
            fixture.roots,
            fixture.channels,
            Btor2RegionPolicy::default(),
        )
        .unwrap();
        assert_eq!(
            summary
                .classes
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>(),
            fixture.classes
        );
    }
}
