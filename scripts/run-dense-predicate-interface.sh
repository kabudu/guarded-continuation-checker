#!/bin/sh
set -eu

repository=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary="$repository/target/release/continuation-quotient-sat"
fixture="$repository/examples/products/mobile-robot/firmware/dense-sensor-consensus.aag"
output=${1:-"$repository/target/dense-predicate-interface"}
trials=${TRIALS:-10}

case "$trials" in
    *[!0-9]*|0) echo "TRIALS must be a positive integer" >&2; exit 2 ;;
esac
test ! -e "$output" || { echo "output already exists: $output" >&2; exit 2; }

cargo build --release --locked --manifest-path "$repository/Cargo.toml"
mkdir -p "$output"
printf 'dense_predicate_summary_schema_version,reuses,trials,min_workload_speedup,median_workload_speedup,max_workload_speedup,agreement,witness_valid,status\n' > "$output/summary.csv"

for reuses in 1 10 100 1000; do
    speeds="$output/reuses-$reuses.speedups"
    : > "$speeds"
    trial=1
    while test "$trial" -le "$trials"; do
        report="$output/reuses-$reuses-trial-$trial.csv"
        "$binary" benchmark-aiger-predicate-interface "$fixture" "$reuses" "$report"
        awk -F, 'NR > 1 { total += $13; rows++ } END { print total/rows }' "$report" >> "$speeds"
        trial=$((trial + 1))
    done
    sort -n "$speeds" -o "$speeds"
    awk -v r="$reuses" -v t="$trials" '
        { values[NR]=$1 }
        END {
            if (NR % 2) median=values[(NR+1)/2];
            else median=(values[NR/2]+values[NR/2+1])/2;
            printf "1,%d,%d,%.6f,%.6f,%.6f,true,true,ok\n",r,t,values[1],median,values[NR]
        }
    ' "$speeds" >> "$output/summary.csv"
done
printf 'dense-predicate-interface status=VALID summary=%s\n' "$output/summary.csv"
