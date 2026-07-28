//! Exact translation from the retained OpenTitan PWM MMIO schedule to the
//! source-attested per-channel RTL input boundary.
//!
//! This module is deliberately fixture-specific. It refuses any event,
//! register value, order or channel ownership outside the frozen firmware
//! contract instead of guessing how a general bus transaction should map.

use crate::{
    btor2::{self, NodeKind, WordValues},
    compiled_mmio_quotient::{ExactCompiledMmioBehavior, PredicateMmioWorkflow},
    riscv32imc::CompiledMmioEvent,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::{error::Error, fmt};

pub const PWM_RTL_MAPPING_VERSION: u32 = 1;
pub const PWM_RTL_CHANNELS: usize = 6;
pub const PWM_RTL_EVENT_FRAMES: usize = 16;
pub const PWM_RTL_TRACE_FRAMES: usize = PWM_RTL_EVENT_FRAMES + 1;
pub const PWM_RTL_PHASE_CYCLE_FRAMES: usize = 16;
pub const PWM_RTL_EXTENDED_TRACE_FRAMES: usize = PWM_RTL_TRACE_FRAMES + PWM_RTL_PHASE_CYCLE_FRAMES;
pub const PWM_RTL_MODEL_SHA256: [u8; 32] = [
    0x15, 0x9a, 0xdf, 0x69, 0xab, 0x63, 0x6d, 0x95, 0x19, 0x5b, 0x2a, 0x65, 0xdd, 0x5d, 0x7a, 0xfd,
    0x46, 0xf0, 0x5b, 0x3b, 0xad, 0x83, 0x26, 0x65, 0x9f, 0xe0, 0x79, 0x43, 0xcd, 0x42, 0x5f, 0x7b,
];
pub const PWM_RTL_STEP_ROOT: u64 = 48;
pub const PWM_RTL_OUTPUT_ROOT: u64 = 74;

const READ: u32 = 1;
const WRITE: u32 = 2;
const OBSERVE_CHANNEL_0: u32 = 3;
const REGWEN: u32 = 0x04;
const CFG: u32 = 0x08;
const ENABLE: u32 = 0x0c;
const INVERT: u32 = 0x10;
const PARAMETER_0: u32 = 0x14;
const DUTY_CYCLE_0: u32 = 0x2c;
const PHASE_TICKS_PER_BEAT: u32 = 4096;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PwmRtlInputs {
    pub enable_write: u8,
    pub invert_write: u8,
    pub parameter_write: u8,
    pub duty_cycle_write: u8,
    pub blink_parameter_write: u8,
    pub channel_enable: u8,
    pub channel_invert: u8,
    pub blink_enable: u8,
    pub heartbeat_enable: u8,
    pub phase_delay: [u8; PWM_RTL_CHANNELS],
    pub duty_cycle_a: [u8; PWM_RTL_CHANNELS],
    pub duty_cycle_b: [u8; PWM_RTL_CHANNELS],
    pub blink_parameter_x: [u8; PWM_RTL_CHANNELS],
    pub blink_parameter_y: [u8; PWM_RTL_CHANNELS],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PwmRtlTrace {
    pub version: u32,
    pub channel: u8,
    pub frames: Vec<PwmRtlInputs>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PwmRtlTraceFamily {
    pub version: u32,
    pub traces: Vec<PwmRtlTrace>,
    pub invalid_rtl_members: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PwmRtlObservation {
    pub step: u8,
    pub pwm: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PwmRtlReplay {
    pub version: u32,
    pub channel: u8,
    pub observations: Vec<PwmRtlObservation>,
    pub transitions: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PwmRtlMappingError(pub String);

impl fmt::Display for PwmRtlMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO to PWM RTL mapping: {}", self.0)
    }
}

impl Error for PwmRtlMappingError {}

fn reject(message: impl Into<String>) -> PwmRtlMappingError {
    PwmRtlMappingError(message.into())
}

fn expect(event: CompiledMmioEvent, operation: u32, offset: u32, value: u32) -> bool {
    event.operation == operation && event.offset == offset && event.value == value
}

fn normalized_beat(value: u32, role: &str) -> Result<u8, PwmRtlMappingError> {
    if value % PHASE_TICKS_PER_BEAT != 0 {
        return Err(reject(format!("{role} is not an exact beat-domain value")));
    }
    let beat = value / PHASE_TICKS_PER_BEAT;
    u8::try_from(beat)
        .ok()
        .filter(|beat| *beat <= 0x0f)
        .ok_or_else(|| reject(format!("{role} exceeds the four-bit RTL domain")))
}

fn apply_write(
    inputs: &mut PwmRtlInputs,
    event: CompiledMmioEvent,
    channel: usize,
) -> Result<(), PwmRtlMappingError> {
    let channel_bit = 1u8 << channel;
    match event.offset {
        ENABLE => {
            if event.value & !0x3f != 0 {
                return Err(reject("enable write exceeds the six-channel domain"));
            }
            inputs.channel_enable = event.value as u8;
            inputs.enable_write = 0x3f;
        }
        INVERT => {
            if event.value & !0x3f != 0 {
                return Err(reject("invert write exceeds the six-channel domain"));
            }
            inputs.channel_invert = event.value as u8;
            inputs.invert_write = 0x3f;
        }
        offset if offset == DUTY_CYCLE_0 + 4 * channel as u32 => {
            inputs.duty_cycle_a[channel] = normalized_beat(event.value & 0xffff, "duty cycle A")?;
            inputs.duty_cycle_b[channel] = normalized_beat(event.value >> 16, "duty cycle B")?;
            inputs.duty_cycle_write = channel_bit;
        }
        offset if offset == PARAMETER_0 + 4 * channel as u32 => {
            inputs.phase_delay[channel] = normalized_beat(event.value & 0xffff, "phase delay")?;
            inputs.blink_enable = (inputs.blink_enable & !channel_bit)
                | ((((event.value >> 31) as u8) & 1) * channel_bit);
            inputs.heartbeat_enable = (inputs.heartbeat_enable & !channel_bit)
                | ((((event.value >> 30) as u8) & 1) * channel_bit);
            inputs.parameter_write = channel_bit;
        }
        _ => return Err(reject("write is not owned by the expected channel")),
    }
    Ok(())
}

fn named_inputs(inputs: &PwmRtlInputs) -> BTreeMap<String, u64> {
    let mut named = BTreeMap::from([
        (
            "parameter_write_i".to_string(),
            u64::from(inputs.parameter_write),
        ),
        (
            "duty_cycle_write_i".to_string(),
            u64::from(inputs.duty_cycle_write),
        ),
        (
            "blink_parameter_write_i".to_string(),
            u64::from(inputs.blink_parameter_write),
        ),
        (
            "channel_enable_i".to_string(),
            u64::from(inputs.channel_enable),
        ),
        (
            "channel_invert_i".to_string(),
            u64::from(inputs.channel_invert),
        ),
        ("blink_enable_i".to_string(), u64::from(inputs.blink_enable)),
        (
            "heartbeat_enable_i".to_string(),
            u64::from(inputs.heartbeat_enable),
        ),
        ("clk_i".to_string(), 0),
        ("enable_write_i".to_string(), u64::from(inputs.enable_write)),
        ("invert_write_i".to_string(), u64::from(inputs.invert_write)),
    ]);
    for channel in 0..PWM_RTL_CHANNELS {
        named.insert(
            format!("phase_delay_{channel}_i"),
            u64::from(inputs.phase_delay[channel]),
        );
        named.insert(
            format!("duty_cycle_a_{channel}_i"),
            u64::from(inputs.duty_cycle_a[channel]),
        );
        named.insert(
            format!("duty_cycle_b_{channel}_i"),
            u64::from(inputs.duty_cycle_b[channel]),
        );
        named.insert(
            format!("blink_parameter_x_{channel}_i"),
            u64::from(inputs.blink_parameter_x[channel]),
        );
        named.insert(
            format!("blink_parameter_y_{channel}_i"),
            u64::from(inputs.blink_parameter_y[channel]),
        );
    }
    named
}

fn bind_inputs(
    model: &btor2::Btor2Model,
    inputs: &PwmRtlInputs,
) -> Result<WordValues, PwmRtlMappingError> {
    let mut named = named_inputs(inputs);
    let mut bound = WordValues::new();
    let source_inputs = model
        .nodes()
        .values()
        .filter(|node| node.kind == NodeKind::Input)
        .collect::<Vec<_>>();
    if source_inputs.len() != named.len()
        || source_inputs.iter().any(|node| {
            node.symbol
                .as_ref()
                .is_none_or(|symbol| !named.contains_key(symbol))
        })
    {
        return Err(reject(
            "RTL model input count differs from the mapping boundary",
        ));
    }
    for input in model.inputs() {
        let node = model
            .nodes()
            .get(input)
            .ok_or_else(|| reject("RTL model input node is missing"))?;
        if node.kind != NodeKind::Input {
            return Err(reject("RTL model input list contains a non-input node"));
        }
        let symbol = node
            .symbol
            .as_ref()
            .ok_or_else(|| reject("RTL model input is unnamed"))?;
        let value = named
            .remove(symbol)
            .ok_or_else(|| reject(format!("unexpected RTL input {symbol}")))?;
        let expected_width = if symbol == "clk_i" {
            1
        } else if symbol.starts_with("phase_delay_")
            || symbol.starts_with("duty_cycle_a_")
            || symbol.starts_with("duty_cycle_b_")
            || symbol.starts_with("blink_parameter_x_")
            || symbol.starts_with("blink_parameter_y_")
        {
            4
        } else {
            6
        };
        if node.width != expected_width {
            return Err(reject(format!("RTL input {symbol} has the wrong width")));
        }
        bound.insert(*input, value);
    }
    if named.len() != 1 || named.remove("clk_i") != Some(0) {
        return Err(reject("RTL model omits mapping inputs"));
    }
    Ok(bound)
}

fn observe(
    model: &btor2::Btor2Model,
    state: &WordValues,
    inputs: &WordValues,
) -> Result<PwmRtlObservation, PwmRtlMappingError> {
    let step = model
        .evaluate(PWM_RTL_STEP_ROOT, state, inputs)
        .map_err(|error| reject(format!("RTL step observation failed: {error}")))?;
    let pwm = model
        .evaluate(PWM_RTL_OUTPUT_ROOT, state, inputs)
        .map_err(|error| reject(format!("RTL PWM observation failed: {error}")))?;
    Ok(PwmRtlObservation {
        step: u8::try_from(step).map_err(|_| reject("RTL step output exceeds eight bits"))?,
        pwm: u8::try_from(pwm).map_err(|_| reject("RTL PWM output exceeds eight bits"))?,
    })
}

fn validate_quiescent_extension(trace: &PwmRtlTrace) -> Result<(), PwmRtlMappingError> {
    if trace.frames.len() != PWM_RTL_EXTENDED_TRACE_FRAMES {
        return Ok(());
    }
    let mut expected = trace.frames[PWM_RTL_TRACE_FRAMES - 1];
    expected.enable_write = 0;
    expected.invert_write = 0;
    expected.parameter_write = 0;
    expected.duty_cycle_write = 0;
    expected.blink_parameter_write = 0;
    if trace.frames[PWM_RTL_TRACE_FRAMES..]
        .iter()
        .any(|frame| *frame != expected)
    {
        return Err(reject(
            "phase-cycle continuation contains a write or valuation drift",
        ));
    }
    Ok(())
}

/// Independently parse and replay one complete translated trace against the
/// exact source-attested BTOR2 model.
pub fn replay_pwm_rtl_trace(
    model_bytes: &[u8],
    trace: &PwmRtlTrace,
) -> Result<PwmRtlReplay, PwmRtlMappingError> {
    if Sha256::digest(model_bytes).as_slice() != PWM_RTL_MODEL_SHA256 {
        return Err(reject(
            "RTL model identity differs from the pinned source boundary",
        ));
    }
    if trace.version != PWM_RTL_MAPPING_VERSION
        || usize::from(trace.channel) >= PWM_RTL_CHANNELS
        || !matches!(
            trace.frames.len(),
            PWM_RTL_TRACE_FRAMES | PWM_RTL_EXTENDED_TRACE_FRAMES
        )
    {
        return Err(reject("RTL trace shape is outside the mapping policy"));
    }
    validate_quiescent_extension(trace)?;
    let model =
        btor2::parse_component_bytes(model_bytes, &[PWM_RTL_STEP_ROOT, PWM_RTL_OUTPUT_ROOT])
            .map_err(|error| reject(format!("RTL model parse failed: {error}")))?;
    let first_inputs = bind_inputs(&model, &trace.frames[0])?;
    let mut state = model
        .initial_state()
        .map_err(|error| reject(format!("RTL initial state failed: {error}")))?;
    let mut observations = Vec::with_capacity(trace.frames.len() + 1);
    observations.push(observe(&model, &state, &first_inputs)?);
    for frame in &trace.frames {
        let inputs = bind_inputs(&model, frame)?;
        state = model
            .step(&state, &inputs)
            .map_err(|error| reject(format!("RTL transition failed: {error}")))?;
        observations.push(observe(&model, &state, &inputs)?);
    }
    Ok(PwmRtlReplay {
        version: PWM_RTL_MAPPING_VERSION,
        channel: trace.channel,
        observations,
        transitions: trace.frames.len() as u32,
    })
}

/// Translate one canonical valid singleton behavior.
///
/// Frame zero is the complete initial input valuation. Each subsequent frame
/// applies exactly one firmware event, with write-enable pulses asserted only
/// for the corresponding write. Read and observation frames retain values and
/// assert no write domains.
pub fn map_pwm_mmio_behavior(
    channel: u8,
    behavior: &ExactCompiledMmioBehavior,
) -> Result<PwmRtlTrace, PwmRtlMappingError> {
    let channel_index = usize::from(channel);
    if channel_index >= PWM_RTL_CHANNELS {
        return Err(reject("channel is outside the retained RTL domain"));
    }
    if behavior.return_value != 0 || behavior.events.len() != PWM_RTL_EVENT_FRAMES {
        return Err(reject("valid behavior shape is not canonical"));
    }

    let selected_duty = DUTY_CYCLE_0 + 4 * u32::from(channel);
    let selected_parameter = PARAMETER_0 + 4 * u32::from(channel);
    let expected = [
        (READ, REGWEN, 1),
        (READ, CFG, 3),
        (READ, INVERT, 0),
        (WRITE, DUTY_CYCLE_0, 0x8000_4000),
        (WRITE, PARAMETER_0, 0),
        (WRITE, INVERT, 0),
        (READ, REGWEN, 1),
        (READ, ENABLE, 0),
        (WRITE, ENABLE, 1),
        (READ, REGWEN, 1),
        (READ, CFG, 3),
        (READ, INVERT, 0),
        (WRITE, selected_duty, 0xa000_6000),
        (WRITE, selected_parameter, 0x0000_2000),
        (WRITE, INVERT, 0),
        (OBSERVE_CHANNEL_0, 0, 1),
    ];

    let mut inputs = PwmRtlInputs::default();
    let mut frames = Vec::with_capacity(PWM_RTL_TRACE_FRAMES);
    frames.push(inputs);
    for (index, (event, (operation, offset, value))) in
        behavior.events.iter().copied().zip(expected).enumerate()
    {
        if !expect(event, operation, offset, value) {
            return Err(reject(format!(
                "event {index} differs from the canonical schedule"
            )));
        }
        inputs.enable_write = 0;
        inputs.invert_write = 0;
        inputs.parameter_write = 0;
        inputs.duty_cycle_write = 0;
        inputs.blink_parameter_write = 0;
        if event.operation == WRITE {
            let owner = if index < 9 { 0 } else { channel_index };
            apply_write(&mut inputs, event, owner)?;
        }
        frames.push(inputs);
    }

    Ok(PwmRtlTrace {
        version: PWM_RTL_MAPPING_VERSION,
        channel,
        frames,
    })
}

/// Translate all six verified singleton behaviors and bind the invalid
/// predicate to zero RTL members.
pub fn map_pwm_mmio_workflow(
    workflow: &PredicateMmioWorkflow,
) -> Result<PwmRtlTraceFamily, PwmRtlMappingError> {
    if workflow.valid_behaviors.len() != PWM_RTL_CHANNELS {
        return Err(reject("valid singleton count is not canonical"));
    }
    if workflow.invalid.first_input != 6
        || workflow.invalid.lane_count != 250
        || workflow.invalid.return_value != 2
        || workflow.invalid.events.len() != 10
    {
        return Err(reject(
            "invalid predicate cannot be bound to zero RTL members",
        ));
    }
    let invalid_prefix = [
        (READ, REGWEN, 1),
        (READ, CFG, 3),
        (READ, INVERT, 0),
        (WRITE, DUTY_CYCLE_0, 0x8000_4000),
        (WRITE, PARAMETER_0, 0),
        (WRITE, INVERT, 0),
        (READ, REGWEN, 1),
        (READ, ENABLE, 0),
        (WRITE, ENABLE, 1),
        (OBSERVE_CHANNEL_0, 0, 1),
    ];
    if workflow
        .invalid
        .events
        .iter()
        .copied()
        .zip(invalid_prefix)
        .any(|(event, (operation, offset, value))| !expect(event, operation, offset, value))
    {
        return Err(reject(
            "invalid predicate event prefix is not the canonical no-RTL path",
        ));
    }
    let traces = workflow
        .valid_behaviors
        .iter()
        .enumerate()
        .map(|(channel, behavior)| map_pwm_mmio_behavior(channel as u8, behavior))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PwmRtlTraceFamily {
        version: PWM_RTL_MAPPING_VERSION,
        traces,
        invalid_rtl_members: 0,
    })
}

/// Append one complete source-derived four-bit phase cycle without any
/// additional firmware write.
pub fn extend_pwm_rtl_trace_one_phase_cycle(
    trace: &PwmRtlTrace,
) -> Result<PwmRtlTrace, PwmRtlMappingError> {
    if trace.version != PWM_RTL_MAPPING_VERSION
        || usize::from(trace.channel) >= PWM_RTL_CHANNELS
        || trace.frames.len() != PWM_RTL_TRACE_FRAMES
    {
        return Err(reject("base RTL trace shape is outside the mapping policy"));
    }
    let mut quiescent = *trace
        .frames
        .last()
        .ok_or_else(|| reject("base RTL trace is empty"))?;
    quiescent.enable_write = 0;
    quiescent.invert_write = 0;
    quiescent.parameter_write = 0;
    quiescent.duty_cycle_write = 0;
    quiescent.blink_parameter_write = 0;
    let mut extended = trace.clone();
    extended
        .frames
        .extend(std::iter::repeat_n(quiescent, PWM_RTL_PHASE_CYCLE_FRAMES));
    Ok(extended)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn behavior(channel: u8) -> ExactCompiledMmioBehavior {
        ExactCompiledMmioBehavior {
            return_value: 0,
            events: vec![
                CompiledMmioEvent {
                    operation: READ,
                    offset: REGWEN,
                    value: 1,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: CFG,
                    value: 3,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: INVERT,
                    value: 0,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: DUTY_CYCLE_0,
                    value: 0x8000_4000,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: PARAMETER_0,
                    value: 0,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: INVERT,
                    value: 0,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: REGWEN,
                    value: 1,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: ENABLE,
                    value: 0,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: ENABLE,
                    value: 1,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: REGWEN,
                    value: 1,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: CFG,
                    value: 3,
                },
                CompiledMmioEvent {
                    operation: READ,
                    offset: INVERT,
                    value: 0,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: DUTY_CYCLE_0 + 4 * u32::from(channel),
                    value: 0xa000_6000,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: PARAMETER_0 + 4 * u32::from(channel),
                    value: 0x2000,
                },
                CompiledMmioEvent {
                    operation: WRITE,
                    offset: INVERT,
                    value: 0,
                },
                CompiledMmioEvent {
                    operation: OBSERVE_CHANNEL_0,
                    offset: 0,
                    value: 1,
                },
            ],
        }
    }

    #[test]
    fn maps_every_channel_without_cross_channel_aliasing() {
        for channel in 0..6 {
            let trace = map_pwm_mmio_behavior(channel, &behavior(channel)).unwrap();
            assert_eq!(trace.frames.len(), PWM_RTL_TRACE_FRAMES);
            let final_frame = trace.frames.last().unwrap();
            assert_eq!(final_frame.channel_enable, 1);
            assert_eq!(
                final_frame.duty_cycle_a[0],
                if channel == 0 { 6 } else { 4 }
            );
            assert_eq!(
                final_frame.duty_cycle_b[0],
                if channel == 0 { 10 } else { 8 }
            );
            assert_eq!(final_frame.duty_cycle_a[usize::from(channel)], 6);
            assert_eq!(final_frame.duty_cycle_b[usize::from(channel)], 10);
            assert_eq!(final_frame.phase_delay[usize::from(channel)], 2);
            assert_eq!(final_frame.duty_cycle_write, 0);
            assert_eq!(final_frame.parameter_write, 0);
        }
    }

    #[test]
    fn refuses_value_order_and_channel_drift() {
        let mut value_drift = behavior(2);
        value_drift.events[12].value += 1;
        assert!(map_pwm_mmio_behavior(2, &value_drift).is_err());

        let mut order_drift = behavior(2);
        order_drift.events.swap(12, 13);
        assert!(map_pwm_mmio_behavior(2, &order_drift).is_err());

        let mut channel_drift = behavior(2);
        channel_drift.events[12].offset += 4;
        assert!(map_pwm_mmio_behavior(2, &channel_drift).is_err());
    }

    #[test]
    fn refuses_every_event_field_mutation_and_model_drift() {
        let original = behavior(4);
        for event in 0..original.events.len() {
            for field in 0..3 {
                let mut changed = original.clone();
                match field {
                    0 => changed.events[event].operation ^= 0x80,
                    1 => changed.events[event].offset ^= 0x80,
                    2 => changed.events[event].value ^= 0x80,
                    _ => unreachable!(),
                }
                assert!(
                    map_pwm_mmio_behavior(4, &changed).is_err(),
                    "accepted event {event} field {field} mutation"
                );
            }
        }

        let model = include_bytes!(
            "../corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
        );
        let trace = map_pwm_mmio_behavior(4, &original).unwrap();
        let mut changed_model = model.to_vec();
        let middle = changed_model.len() / 2;
        changed_model[middle] ^= 1;
        assert!(replay_pwm_rtl_trace(&changed_model, &trace).is_err());
    }

    #[test]
    fn independently_replays_every_mapped_channel() {
        let model = include_bytes!(
            "../corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
        );
        for channel in 0..6 {
            let trace = map_pwm_mmio_behavior(channel, &behavior(channel)).unwrap();
            let replay = replay_pwm_rtl_trace(model, &trace).unwrap();
            assert_eq!(replay.channel, channel);
            assert_eq!(replay.transitions, PWM_RTL_TRACE_FRAMES as u32);
            assert_eq!(replay.observations.len(), PWM_RTL_TRACE_FRAMES + 1);
            assert_eq!(
                replay.observations[0],
                PwmRtlObservation { step: 0, pwm: 0 }
            );
            assert_eq!(replay.observations.last().unwrap().step, 15);
        }
    }

    #[test]
    fn one_phase_cycle_is_fixed_quiescent_and_discriminating() {
        let model = include_bytes!(
            "../corpus/rtl/opentitan-pwm-channel-family/generated/firmware-trace-6.btor2"
        );
        let replays = (0..6)
            .map(|channel| {
                let base = map_pwm_mmio_behavior(channel, &behavior(channel)).unwrap();
                let extended = extend_pwm_rtl_trace_one_phase_cycle(&base).unwrap();
                assert_eq!(extended.frames.len(), PWM_RTL_EXTENDED_TRACE_FRAMES);
                assert!(extended.frames[PWM_RTL_TRACE_FRAMES..].iter().all(|frame| {
                    frame.enable_write == 0
                        && frame.invert_write == 0
                        && frame.parameter_write == 0
                        && frame.duty_cycle_write == 0
                        && frame.blink_parameter_write == 0
                }));
                replay_pwm_rtl_trace(model, &extended).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(replays.iter().all(|replay| {
            replay
                .observations
                .iter()
                .any(|observation| observation.pwm != 0)
        }));
        assert_ne!(replays[0].observations, replays[1].observations);
        assert!(
            replays[1..]
                .windows(2)
                .all(|pair| pair[0].observations == pair[1].observations)
        );

        let base = map_pwm_mmio_behavior(1, &behavior(1)).unwrap();
        let mut changed = extend_pwm_rtl_trace_one_phase_cycle(&base).unwrap();
        changed.frames[PWM_RTL_TRACE_FRAMES].parameter_write = 1;
        assert!(replay_pwm_rtl_trace(model, &changed).is_err());
        assert!(extend_pwm_rtl_trace_one_phase_cycle(&changed).is_err());
    }
}
