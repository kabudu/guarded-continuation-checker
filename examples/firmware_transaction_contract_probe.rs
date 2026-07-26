use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

use guarded_continuation_checker::firmware_transaction_contract::{
    FirmwareTransactionContractInput, FirmwareTransactionEvent,
    encode_firmware_transaction_contract, produce_firmware_transaction_contract,
    verify_firmware_transaction_contract,
};
use guarded_continuation_checker::revision_impact::TwoComponentRevisionImpactInput;
use guarded_continuation_checker::revision_local::{BoundedQuery, ComponentSide};
use sha2::{Digest, Sha256};

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
const VALID: &[FirmwareTransactionEvent] = &[
    FirmwareTransactionEvent::ConfigureChannel0,
    FirmwareTransactionEvent::EnableChannel0,
    FirmwareTransactionEvent::ConfigureChannel1,
    FirmwareTransactionEvent::ObserveChannel0,
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
    events: &'a [FirmwareTransactionEvent],
    queries: &'a [BoundedQuery],
) -> FirmwareTransactionContractInput<'a> {
    FirmwareTransactionContractInput {
        contract_source: CONTRACT,
        stimulus_mapping: MAPPING,
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

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: firmware_transaction_contract_probe OUTPUT.csv")?;
    let queries = queries();
    let valid_input = input(VALID, &queries);
    let envelope = produce_firmware_transaction_contract(&valid_input)?;
    let encoded = encode_firmware_transaction_contract(&envelope)?;
    let summary = verify_firmware_transaction_contract(&valid_input, &envelope)?;
    let digest = Sha256::digest(&encoded)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut rows = vec![format!(
        "valid,accepted,true,{},{},{},{},{digest}",
        encoded.len(),
        summary.events,
        summary.impact.combinations * summary.impact.queries,
        summary.impact.minimal_semantic_change_sets
    )];

    let invalid = [
        (
            "reordered",
            vec![
                FirmwareTransactionEvent::EnableChannel0,
                FirmwareTransactionEvent::ConfigureChannel0,
                FirmwareTransactionEvent::ConfigureChannel1,
                FirmwareTransactionEvent::ObserveChannel0,
            ],
        ),
        ("omitted", VALID[..3].to_vec()),
        (
            "disabled_before_observation",
            vec![
                FirmwareTransactionEvent::ConfigureChannel0,
                FirmwareTransactionEvent::EnableChannel0,
                FirmwareTransactionEvent::DisableChannel0,
                FirmwareTransactionEvent::ObserveChannel0,
            ],
        ),
        (
            "reconfigured_active_channel",
            vec![
                FirmwareTransactionEvent::ConfigureChannel0,
                FirmwareTransactionEvent::EnableChannel0,
                FirmwareTransactionEvent::ReconfigureChannel0,
                FirmwareTransactionEvent::ConfigureChannel1,
                FirmwareTransactionEvent::ObserveChannel0,
            ],
        ),
    ];
    for (name, events) in invalid {
        if produce_firmware_transaction_contract(&input(&events, &queries)).is_ok() {
            return Err(format!("invalid schedule {name} was accepted").into());
        }
        rows.push(format!("{name},refused,false,0,0,0,0,none"));
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(
        b"schedule,status,verified,envelope_bytes,events,impact_observations,minimal_semantic_change_sets,sha256\n",
    )?;
    for row in &rows {
        writeln!(file, "{row}")?;
    }
    file.sync_all()?;
    println!(
        "firmware_transaction_contract_probe=PASS rows={} output={output}",
        rows.len()
    );
    Ok(())
}
