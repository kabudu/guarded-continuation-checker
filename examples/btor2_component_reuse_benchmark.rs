use guarded_continuation_checker::btor2_component::{
    self, ComponentBatchInput, ReusableBatchMember,
};
use std::hint::black_box;
use std::time::Instant;

const CONTROLLER: &[u8] = include_bytes!("btor2/components/braking-controller-v1.btor2");
const BASE_PLANT: &[u8] = include_bytes!("btor2/components/motion-plant-v1.btor2");
const FAST_PLANT: &[u8] = include_bytes!("btor2/components/fast-motion-plant-v1.btor2");
const CONTRACT: &[u8] = include_bytes!("btor2/components/braking-motion-contract-v1.txt");
const TRIALS: usize = 101;

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let selection = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    if !matches!(selection.as_str(), "all" | "admitted" | "mixed") {
        return Err("usage: btor2_component_reuse_benchmark [all|admitted|mixed]".into());
    }
    println!(
        "schema_version,cohort,members,trials,naive_artifact_bytes,reusable_artifact_bytes,artifact_ratio,naive_produce_median_nanos,reusable_produce_median_nanos,produce_ratio,naive_verify_median_nanos,reusable_verify_median_nanos,verify_ratio,break_even_verifications,reused_phase,exact_fallback,status"
    );
    let admitted_counts = [1usize, 2, 4, 8, 16, 32, 64];
    let mixed_counts = [4usize, 8, 16];
    for (cohort, counts) in [
        ("admitted", admitted_counts.as_slice()),
        ("mixed-25-percent-fallback", mixed_counts.as_slice()),
    ] {
        if (selection == "admitted" && cohort != "admitted")
            || (selection == "mixed" && cohort == "admitted")
        {
            continue;
        }
        for &count in counts {
            let inputs = (0..count)
                .map(|index| ComponentBatchInput {
                    plant_source: if (cohort != "admitted" && index % 4 == 3)
                        || index.is_multiple_of(2)
                    {
                        BASE_PLANT
                    } else {
                        FAST_PLANT
                    },
                    contract_source: CONTRACT,
                    horizon: if cohort != "admitted" && index % 4 == 3 {
                        256
                    } else if index.is_multiple_of(2) {
                        192 + (index as u32 % 64)
                    } else {
                        64 + (index as u32 % 64)
                    },
                })
                .collect::<Vec<_>>();
            let naive = btor2_component::produce_naive_component_batch(CONTROLLER, &inputs)?;
            let reusable = btor2_component::produce_reusable_component_batch(CONTROLLER, &inputs)?;
            let obligation_bytes =
                btor2_component::encode_controller_obligation(&reusable.controller_obligation)?
                    .len();
            let naive_artifact_bytes = obligation_bytes
                + naive
                    .members
                    .iter()
                    .map(btor2_component::encode)
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .map(String::len)
                    .sum::<usize>();
            let reusable_artifact_bytes =
                btor2_component::encode_reusable_component_batch(&reusable)?.len();
            let reused_phase = reusable
                .members
                .iter()
                .filter(|member| matches!(member, ReusableBatchMember::ReusedPhase(_)))
                .count();
            let exact_fallback = count - reused_phase;
            let mut naive_produce_times = Vec::with_capacity(TRIALS);
            let mut reusable_produce_times = Vec::with_capacity(TRIALS);
            let mut naive_times = Vec::with_capacity(TRIALS);
            let mut reusable_times = Vec::with_capacity(TRIALS);
            for trial in 0..TRIALS {
                if trial.is_multiple_of(2) {
                    let started = Instant::now();
                    black_box(btor2_component::produce_naive_component_batch(
                        CONTROLLER, &inputs,
                    )?);
                    naive_produce_times.push(started.elapsed().as_nanos());
                    let started = Instant::now();
                    black_box(btor2_component::produce_reusable_component_batch(
                        CONTROLLER, &inputs,
                    )?);
                    reusable_produce_times.push(started.elapsed().as_nanos());
                    let started = Instant::now();
                    black_box(btor2_component::verify_naive_component_batch(
                        CONTROLLER, &inputs, &naive,
                    )?);
                    naive_times.push(started.elapsed().as_nanos());
                    let started = Instant::now();
                    black_box(btor2_component::verify_reusable_component_batch(
                        CONTROLLER, &inputs, &reusable,
                    )?);
                    reusable_times.push(started.elapsed().as_nanos());
                } else {
                    let started = Instant::now();
                    black_box(btor2_component::produce_reusable_component_batch(
                        CONTROLLER, &inputs,
                    )?);
                    reusable_produce_times.push(started.elapsed().as_nanos());
                    let started = Instant::now();
                    black_box(btor2_component::produce_naive_component_batch(
                        CONTROLLER, &inputs,
                    )?);
                    naive_produce_times.push(started.elapsed().as_nanos());
                    let started = Instant::now();
                    black_box(btor2_component::verify_reusable_component_batch(
                        CONTROLLER, &inputs, &reusable,
                    )?);
                    reusable_times.push(started.elapsed().as_nanos());
                    let started = Instant::now();
                    black_box(btor2_component::verify_naive_component_batch(
                        CONTROLLER, &inputs, &naive,
                    )?);
                    naive_times.push(started.elapsed().as_nanos());
                }
            }
            let naive_produce_median = median(naive_produce_times);
            let reusable_produce_median = median(reusable_produce_times);
            let naive_median = median(naive_times);
            let reusable_median = median(reusable_times);
            let break_even = if reusable_median < naive_median {
                let extra_production = reusable_produce_median.saturating_sub(naive_produce_median);
                let saved_per_verification = naive_median - reusable_median;
                extra_production
                    .div_ceil(saved_per_verification)
                    .max(1)
                    .to_string()
            } else {
                "none".to_string()
            };
            println!(
                "2,{cohort},{count},{TRIALS},{naive_artifact_bytes},{reusable_artifact_bytes},{:.3},{naive_produce_median},{reusable_produce_median},{:.3},{naive_median},{reusable_median},{:.3},{break_even},{reused_phase},{exact_fallback},ok",
                reusable_artifact_bytes as f64 / naive_artifact_bytes as f64,
                reusable_produce_median as f64 / naive_produce_median as f64,
                reusable_median as f64 / naive_median as f64,
            );
        }
    }
    Ok(())
}
