use std::collections::BTreeSet;

use guarded_continuation_checker::aiger_obligation::parse_ascii_aiger_transition;
use guarded_continuation_checker::controller_plant::{
    ControllerPlantWiring, compose_controller_plant_direct,
};
use guarded_continuation_checker::controller_plant_aiger::{
    CONTROLLER_PLANT_AIGER_EXPORT_VERSION, export_bounded_controller_plant_aag,
    export_bounded_controller_plant_multi_aag,
};

const CONTROLLER: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/generated/controller.aag");
const PLANT: &[u8] = include_bytes!("../corpus/rtl/wmcontroller/plant/physical-plant.aag");
const SENSOR_STUCK_PLANT: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/sensor-stuck/physical-plant.aag"
);
const ACTUATOR_DELAY_PLANT: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-delay/physical-plant.aag"
);
const PERSISTENT_DISTURBANCE_PLANT: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/persistent-disturbance/physical-plant.aag"
);
const ACTUATOR_TRANSPORT_LAG_PLANT: &[u8] = include_bytes!(
    "../corpus/rtl/wmcontroller/composed-witness-plants-v1/actuator-transport-lag/physical-plant.aag"
);

struct IndependentAag {
    inputs: Vec<usize>,
    latches: Vec<(usize, usize)>,
    outputs: Vec<usize>,
    ands: Vec<(usize, usize, usize)>,
    max_variable: usize,
}

fn number(token: Option<&str>) -> usize {
    token.unwrap().parse().unwrap()
}

fn parse_independently(bytes: &[u8]) -> IndependentAag {
    let text = std::str::from_utf8(bytes).unwrap();
    let mut lines = text.lines();
    let mut header = lines.next().unwrap().split_whitespace();
    assert_eq!(header.next(), Some("aag"));
    let max_variable = number(header.next());
    let input_count = number(header.next());
    let latch_count = number(header.next());
    let output_count = number(header.next());
    let and_count = number(header.next());
    let extension = header
        .map(|value| value.parse::<usize>().unwrap())
        .collect::<Vec<_>>();
    assert!(extension.len() <= 4);
    let bad_count = extension.first().copied().unwrap_or(0);
    assert!(extension.iter().skip(1).all(|&count| count == 0));
    assert!((output_count == 1 && bad_count == 0) || (output_count == 0 && bad_count > 0));
    assert_eq!(max_variable, input_count + latch_count + and_count);
    let inputs = (0..input_count)
        .map(|_| number(lines.next()))
        .collect::<Vec<_>>();
    let latches = (0..latch_count)
        .map(|_| {
            let mut fields = lines.next().unwrap().split_whitespace();
            let current = number(fields.next());
            let next = number(fields.next());
            assert_eq!(fields.next(), Some("0"));
            assert!(fields.next().is_none());
            (current, next)
        })
        .collect::<Vec<_>>();
    let mut outputs = (0..output_count)
        .map(|_| number(lines.next()))
        .collect::<Vec<_>>();
    outputs.extend((0..bad_count).map(|_| number(lines.next())));
    let ands = (0..and_count)
        .map(|_| {
            let mut fields = lines.next().unwrap().split_whitespace();
            let output = number(fields.next());
            let left = number(fields.next());
            let right = number(fields.next());
            assert!(fields.next().is_none());
            assert!(left / 2 < output / 2);
            assert!(right / 2 < output / 2);
            (output, left, right)
        })
        .collect::<Vec<_>>();
    assert!(lines.next().is_none());
    IndependentAag {
        inputs,
        latches,
        outputs,
        ands,
        max_variable,
    }
}

fn literal(literal: usize, values: &[bool]) -> bool {
    if literal < 2 {
        literal == 1
    } else {
        values[literal / 2] ^ (literal & 1 == 1)
    }
}

impl IndependentAag {
    fn evaluate(&self, state: u64, input: u64) -> (u64, Vec<bool>) {
        let mut values = vec![false; self.max_variable + 1];
        for (bit, &(current, _)) in self.latches.iter().enumerate() {
            values[current / 2] = state >> bit & 1 == 1;
        }
        for (bit, &declared) in self.inputs.iter().enumerate() {
            values[declared / 2] = input >> bit & 1 == 1;
        }
        for &(output, left, right) in &self.ands {
            values[output / 2] = literal(left, &values) && literal(right, &values);
        }
        let next = self
            .latches
            .iter()
            .enumerate()
            .fold(0, |state, (bit, &(_, next))| {
                state | (u64::from(literal(next, &values)) << bit)
            });
        (
            next,
            self.outputs
                .iter()
                .map(|&output| literal(output, &values))
                .collect(),
        )
    }

    fn shortest_bad_and_convergence(&self, output: usize) -> (Option<usize>, usize) {
        let mut reached = BTreeSet::from([0]);
        for frame in 0..256 {
            let mut next = BTreeSet::new();
            for &state in &reached {
                for input in 0..(1u64 << self.inputs.len()) {
                    let (target, bads) = self.evaluate(state, input);
                    if bads[output] {
                        return (Some(frame), reached.len());
                    }
                    next.insert(target);
                }
            }
            if next == reached {
                return (None, reached.len());
            }
            reached = next;
        }
        panic!("independent exported-model replay did not converge");
    }
}

#[test]
fn six_exports_preserve_answers_and_shortest_bad_frames() {
    let controller = parse_ascii_aiger_transition(CONTROLLER).unwrap();
    let plant = parse_ascii_aiger_transition(PLANT).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let expected = [Some(4), Some(7), Some(15), Some(15), None, None];
    for (bad_output, expected_bad) in (11..17).zip(expected) {
        let source =
            compose_controller_plant_direct(&controller, &plant, &wiring, 0, 0, bad_output, 32)
                .unwrap();
        assert_eq!(source.bad_frame, expected_bad);
        let export =
            export_bounded_controller_plant_aag(&controller, &plant, &wiring, 0, 0, bad_output, 32)
                .unwrap();
        assert_eq!(export.version, CONTROLLER_PLANT_AIGER_EXPORT_VERSION);
        assert_eq!(export.external_plant_inputs, vec![0, 5, 6, 7]);
        let independent = parse_independently(&export.bytes);
        let (actual_bad, _) = independent.shortest_bad_and_convergence(0);
        assert_eq!(actual_bad, expected_bad, "plant output {bad_output}");
    }
}

#[test]
fn changing_plant_family_preserves_two_safe_properties() {
    let controller = parse_ascii_aiger_transition(CONTROLLER).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    for (name, bytes, expected) in [
        (
            "nominal",
            PLANT,
            [Some(4), Some(7), Some(15), Some(15), None, None],
        ),
        (
            "sensor-stuck",
            SENSOR_STUCK_PLANT,
            [Some(4), Some(7), Some(15), Some(15), None, None],
        ),
        (
            "actuator-delay",
            ACTUATOR_DELAY_PLANT,
            [Some(5), Some(11), Some(19), Some(19), None, None],
        ),
        (
            "persistent-disturbance",
            PERSISTENT_DISTURBANCE_PLANT,
            [Some(4), Some(7), Some(15), Some(15), None, None],
        ),
        (
            "actuator-transport-lag",
            ACTUATOR_TRANSPORT_LAG_PLANT,
            [Some(5), Some(8), Some(16), Some(16), None, None],
        ),
    ] {
        let plant = parse_ascii_aiger_transition(bytes).unwrap();
        let mut answers = Vec::new();
        for bad_output in 11..17 {
            let export = export_bounded_controller_plant_aag(
                &controller,
                &plant,
                &wiring,
                0,
                0,
                bad_output,
                32,
            )
            .unwrap();
            answers.push(
                parse_independently(&export.bytes)
                    .shortest_bad_and_convergence(0)
                    .0,
            );
        }
        assert_eq!(answers, expected, "plant {name}");
    }
}

#[test]
fn multi_property_export_shares_transition_logic_and_preserves_safe_answers() {
    let controller = parse_ascii_aiger_transition(CONTROLLER).unwrap();
    let plant = parse_ascii_aiger_transition(PLANT).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    let export = export_bounded_controller_plant_multi_aag(
        &controller,
        &plant,
        &wiring,
        0,
        0,
        &[15, 16],
        32,
    )
    .unwrap();
    assert_eq!(export.bad_plant_outputs, vec![15, 16]);
    let independent = parse_independently(&export.bytes);
    assert_eq!(independent.outputs.len(), 2);
    assert_eq!(independent.shortest_bad_and_convergence(0).0, None);
    assert_eq!(independent.shortest_bad_and_convergence(1).0, None);
    assert!(
        export_bounded_controller_plant_multi_aag(
            &controller,
            &plant,
            &wiring,
            0,
            0,
            &[15, 15],
            32,
        )
        .is_err()
    );
}

#[test]
fn exporter_rejects_nonzero_initial_state_and_oversized_horizon() {
    let controller = parse_ascii_aiger_transition(CONTROLLER).unwrap();
    let plant = parse_ascii_aiger_transition(PLANT).unwrap();
    let wiring = ControllerPlantWiring {
        controller_sensor_inputs: (1..12).collect(),
        controller_action_outputs: vec![2, 6, 7, 9],
        plant_sensor_outputs: (0..11).collect(),
        plant_action_inputs: vec![1, 2, 3, 4],
    };
    assert!(
        export_bounded_controller_plant_aag(&controller, &plant, &wiring, 1, 0, 11, 32).is_err()
    );
    assert!(
        export_bounded_controller_plant_aag(&controller, &plant, &wiring, 0, 0, 11, 63).is_err()
    );
}
