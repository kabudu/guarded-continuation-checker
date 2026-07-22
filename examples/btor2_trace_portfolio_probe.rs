use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;

use guarded_continuation_checker::btor2_region_equivalence::{
    Btor2ChannelTraceQuery, admit_btor2_reachable_region_equivalence_artifact,
    evaluate_btor2_channel_trace_queries_exact, evaluate_btor2_channel_trace_queries_portfolio,
    produce_btor2_reachable_region_equivalence_artifact,
};
use guarded_continuation_checker::btor2_region_extract::Btor2RegionPolicy;

fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn equivalent_results(
    candidate: &[guarded_continuation_checker::btor2_region_equivalence::Btor2ChannelTraceResult],
    baseline: &[guarded_continuation_checker::btor2_region_equivalence::Btor2ChannelTraceResult],
) -> bool {
    candidate.len() == baseline.len()
        && candidate.iter().zip(baseline).all(|(left, right)| {
            left.query_id == right.query_id
                && left.channel_index == right.channel_index
                && left.matched == right.matched
                && left.earliest_frame == right.earliest_frame
        })
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args()
        .nth(1)
        .ok_or("usage: btor2_trace_portfolio_probe OUTPUT.csv")?;
    let bytes =
        include_bytes!("../corpus/rtl/opentitan-pwm-channel-family/generated/authentic-6.btor2");
    let roots = [5, 36];
    let channels = 6;
    let horizon = 4095;
    let trials = 5;
    let policy = Btor2RegionPolicy::default();
    let artifact = produce_btor2_reachable_region_equivalence_artifact(
        bytes, &roots, channels, horizon, policy,
    )?;
    let mut rows = vec!["schema_version,channels,horizon,predicates_per_channel,logical_queries,representative_evaluations,direct_singleton_evaluations,reused_queries,candidate_evaluation_bound,direct_evaluation_bound,work_reduction_percent,candidate_median_micros,direct_median_micros,speedup,exact_agreement,trials,selection,status".to_string()];
    for predicates_per_channel in [16usize, 256, 4096, 8192] {
        let queries = (0..channels)
            .flat_map(|channel| {
                (0..predicates_per_channel).map(move |predicate| {
                    let query_id = u32::try_from(channel * predicates_per_channel + predicate)
                        .expect("bounded query id");
                    let start_frame = u32::try_from(predicate % 4096).expect("bounded frame");
                    let value = u64::try_from(predicate / 4096).expect("bounded value");
                    Btor2ChannelTraceQuery::new(query_id, channel, start_frame, horizon, 1, value)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut candidate_times = Vec::with_capacity(trials);
        let mut direct_times = Vec::with_capacity(trials);
        let mut retained_candidate = None;
        let mut retained_direct = None;
        for _ in 0..trials {
            let candidate_start = Instant::now();
            let admission =
                admit_btor2_reachable_region_equivalence_artifact(bytes, &artifact, policy)?;
            let candidate = evaluate_btor2_channel_trace_queries_portfolio(&admission, &queries)?;
            candidate_times.push(candidate_start.elapsed().as_micros());

            let direct_start = Instant::now();
            let direct = evaluate_btor2_channel_trace_queries_exact(
                bytes, &roots, channels, horizon, &queries, policy,
            )?;
            direct_times.push(direct_start.elapsed().as_micros());
            if !equivalent_results(&candidate.results, &direct.results) {
                return Err("portfolio result disagrees with direct exact evaluation".into());
            }
            retained_candidate = Some(candidate);
            retained_direct = Some(direct);
        }
        let candidate = retained_candidate.expect("one trial");
        let direct = retained_direct.expect("one trial");
        let candidate_bound = candidate.metrics.representative_predicate_evaluations
            + candidate.metrics.exact_singleton_predicate_evaluations;
        let direct_bound = direct.metrics.direct_predicate_evaluation_bound;
        let candidate_micros = median(&mut candidate_times);
        let direct_micros = median(&mut direct_times);
        rows.push(format!(
            "1,{channels},{horizon},{predicates_per_channel},{},{},{},{},{candidate_bound},{direct_bound},{:.6},{candidate_micros},{direct_micros},{:.6},true,{trials},none,accepted",
            queries.len(),
            candidate.metrics.representative_predicate_evaluations,
            candidate.metrics.exact_singleton_predicate_evaluations,
            candidate.metrics.reused_logical_queries,
            (direct_bound - candidate_bound) as f64 * 100.0 / direct_bound as f64,
            direct_micros as f64 / candidate_micros as f64,
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(rows.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("btor2_trace_portfolio_probe_v1=PASS rows=4 output={output}");
    Ok(())
}
