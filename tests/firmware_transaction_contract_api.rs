use guarded_continuation_checker::firmware_transaction_contract::{
    FirmwareTransactionContractInput, FirmwareTransactionEvent, compiled_pwm_schedule,
    decode_firmware_transaction_contract, encode_firmware_transaction_contract,
    produce_firmware_transaction_contract, verify_firmware_transaction_contract,
};
use guarded_continuation_checker::revision_impact::TwoComponentRevisionImpactInput;
use guarded_continuation_checker::revision_local::BoundedResult;
use guarded_continuation_checker::revision_local::{BoundedQuery, ComponentSide};
use guarded_continuation_checker::riscv32imc::CompiledMmioEvent;

const CORE_BEFORE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-before.btor2");
const CORE_AFTER: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/core-after.btor2");
const CHANNEL_BEFORE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-before.btor2");
const CHANNEL_AFTER: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/generated/channel-after.btor2");
const INTERFACE: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/interface.txt");
const CONTRACT: &[u8] = include_bytes!(
    "../corpus/rtl/opentitan-pwm-crosstalk-impact/firmware-transaction-contract-v1.txt"
);
const MAPPING: &[u8] =
    include_bytes!("../corpus/rtl/opentitan-pwm-crosstalk-impact/firmware-stimulus-mapping-v1.txt");
const EVENTS: &[FirmwareTransactionEvent] = &[
    FirmwareTransactionEvent::ConfigureChannel0,
    FirmwareTransactionEvent::EnableChannel0,
    FirmwareTransactionEvent::ConfigureChannel1,
    FirmwareTransactionEvent::ObserveChannel0,
];
const COMPILED_EVENTS: &[CompiledMmioEvent] = &[
    CompiledMmioEvent {
        operation: 1,
        offset: 4,
        value: 1,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 8,
        value: 3,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 16,
        value: 0,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 44,
        value: 2_147_500_032,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 20,
        value: 0,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 16,
        value: 0,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 4,
        value: 1,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 12,
        value: 0,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 12,
        value: 1,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 4,
        value: 1,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 8,
        value: 3,
    },
    CompiledMmioEvent {
        operation: 1,
        offset: 16,
        value: 0,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 48,
        value: 2_684_379_136,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 24,
        value: 8_192,
    },
    CompiledMmioEvent {
        operation: 2,
        offset: 16,
        value: 0,
    },
    CompiledMmioEvent {
        operation: 3,
        offset: 0,
        value: 1,
    },
];

fn queries() -> [BoundedQuery; 5] {
    [
        BoundedQuery {
            horizon: 0,
            bad_side: ComponentSide::Right,
            bad_output: 1004,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1000,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1001,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1002,
        },
        BoundedQuery {
            horizon: 4,
            bad_side: ComponentSide::Right,
            bad_output: 1003,
        },
    ]
}

fn input<'a>(
    contract: &'a [u8],
    mapping: &'a [u8],
    events: &'a [FirmwareTransactionEvent],
    queries: &'a [BoundedQuery],
) -> FirmwareTransactionContractInput<'a> {
    FirmwareTransactionContractInput {
        contract_source: contract,
        stimulus_mapping: mapping,
        events,
        revision: TwoComponentRevisionImpactInput {
            left_old: CORE_BEFORE,
            left_new: CORE_AFTER,
            left_outputs: &[1000, 1001, 1002, 1003],
            right_old: CHANNEL_BEFORE,
            right_new: CHANNEL_AFTER,
            right_outputs: &[1000, 1001, 1002, 1003, 1004],
            interface_old: INTERFACE,
            interface_new: INTERFACE,
            queries,
        },
    }
}

#[test]
fn valid_firmware_contract_preserves_the_complete_authentic_impact_matrix() {
    let queries = queries();
    let canonical_input = input(CONTRACT, MAPPING, EVENTS, &queries);
    let envelope = produce_firmware_transaction_contract(&canonical_input).unwrap();
    let first = encode_firmware_transaction_contract(&envelope).unwrap();
    let second = encode_firmware_transaction_contract(
        &produce_firmware_transaction_contract(&canonical_input).unwrap(),
    )
    .unwrap();
    assert_eq!(first, second);
    let decoded = decode_firmware_transaction_contract(&first).unwrap();
    assert_eq!(decoded, envelope);
    let summary = verify_firmware_transaction_contract(&canonical_input, &decoded).unwrap();
    assert_eq!(summary.events, 4);
    assert_eq!(summary.observation_ready_frame, 4);
    assert_eq!(
        (
            summary.impact.atoms,
            summary.impact.queries,
            summary.impact.combinations,
            summary.impact.minimal_semantic_change_sets,
        ),
        (2, 5, 4, 3)
    );
    let result =
        |mask: usize, query: usize| envelope.impact.impact.observations[mask * 5 + query].result;
    assert_eq!(result(0, 1), BoundedResult::Unsafe);
    assert_eq!(result(1, 1), BoundedResult::Safe);
    assert_eq!(result(0, 3), BoundedResult::Unsafe);
    assert_eq!(result(3, 3), BoundedResult::Safe);
}

#[test]
fn exact_compiled_mmio_stream_reaches_the_semantic_schedule() {
    assert_eq!(compiled_pwm_schedule(COMPILED_EVENTS).unwrap(), EVENTS);
    for index in 0..COMPILED_EVENTS.len() {
        let mut changed = COMPILED_EVENTS.to_vec();
        changed[index].value ^= 1;
        assert!(compiled_pwm_schedule(&changed).is_err());
    }
    assert!(compiled_pwm_schedule(&COMPILED_EVENTS[..15]).is_err());
    let mut extended = COMPILED_EVENTS.to_vec();
    extended.push(COMPILED_EVENTS[0]);
    assert!(compiled_pwm_schedule(&extended).is_err());
}

#[test]
fn invalid_schedules_refuse_without_producing_rtl_evidence() {
    let queries = queries();
    let invalid = [
        vec![
            FirmwareTransactionEvent::EnableChannel0,
            FirmwareTransactionEvent::ConfigureChannel0,
            FirmwareTransactionEvent::ConfigureChannel1,
            FirmwareTransactionEvent::ObserveChannel0,
        ],
        EVENTS[..3].to_vec(),
        vec![
            FirmwareTransactionEvent::ConfigureChannel0,
            FirmwareTransactionEvent::EnableChannel0,
            FirmwareTransactionEvent::DisableChannel0,
            FirmwareTransactionEvent::ObserveChannel0,
        ],
        vec![
            FirmwareTransactionEvent::ConfigureChannel0,
            FirmwareTransactionEvent::EnableChannel0,
            FirmwareTransactionEvent::ReconfigureChannel0,
            FirmwareTransactionEvent::ConfigureChannel1,
            FirmwareTransactionEvent::ObserveChannel0,
        ],
    ];
    for events in invalid {
        assert!(
            produce_firmware_transaction_contract(&input(CONTRACT, MAPPING, &events, &queries))
                .is_err()
        );
    }
}

#[test]
fn contract_mapping_revision_and_envelope_drift_fail_closed() {
    let queries = queries();
    let canonical_input = input(CONTRACT, MAPPING, EVENTS, &queries);
    let envelope = produce_firmware_transaction_contract(&canonical_input).unwrap();
    let encoded = encode_firmware_transaction_contract(&envelope).unwrap();

    let mut contract = CONTRACT.to_vec();
    contract.push(b'\n');
    assert!(
        verify_firmware_transaction_contract(
            &input(&contract, MAPPING, EVENTS, &queries),
            &envelope
        )
        .is_err()
    );
    let mut mapping = MAPPING.to_vec();
    mapping.push(b'\n');
    assert!(
        verify_firmware_transaction_contract(
            &input(CONTRACT, &mapping, EVENTS, &queries),
            &envelope
        )
        .is_err()
    );

    for offset in [0, 8, 44, encoded.len() / 2, encoded.len() - 1] {
        let mut changed = encoded.clone();
        changed[offset] ^= 1;
        assert!(decode_firmware_transaction_contract(&changed).is_err());
    }
    assert!(decode_firmware_transaction_contract(&encoded[..encoded.len() - 1]).is_err());
    let mut extended = encoded;
    extended.push(0);
    assert!(decode_firmware_transaction_contract(&extended).is_err());
}
