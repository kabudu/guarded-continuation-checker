#!/bin/sh
set -eu

repository=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary="$repository/target/release/guarded-continuation-checker"
fixture="$repository/examples/products/infusion-pump/firmware/door-interlock-regression.aag"
output=${1:-"$repository/target/interface-quotient-scaling"}
trials=${TRIALS:-10}
repeats=${REPEATS:-100}

case "$trials:$repeats" in
    *[!0-9:]*|0:*|*:0) echo "TRIALS and REPEATS must be positive integers" >&2; exit 2 ;;
esac
test ! -e "$output" || { echo "output already exists: $output" >&2; exit 2; }

cargo build --release --locked --manifest-path "$repository/Cargo.toml"
mkdir -p "$output"
printf 'interface_quotient_scaling_schema_version,horizon,trials,repeats,reachable_targets,min_workload_speedup,median_workload_speedup,max_workload_speedup,agreement,witness_valid,status\n' > "$output/summary.csv"

for horizon in 8 16 32 64; do
    speeds="$output/horizon-$horizon.speedups"
    : > "$speeds"
    targets=0
    trial=1
    while test "$trial" -le "$trials"; do
        report="$output/horizon-$horizon-trial-$trial.csv"
        "$binary" benchmark-aiger-interface-quotient "$fixture" "$horizon" 8 "$repeats" "$report"
        "$binary" verify-aiger-interface-quotient "$fixture" "$report"
        tail -n 1 "$report" | awk -F, '{ print $23 }' >> "$speeds"
        current_targets=$(awk 'NR > 1 { rows++ } END { print rows+0 }' "$report")
        if test "$targets" -eq 0; then targets=$current_targets; fi
        test "$targets" -eq "$current_targets" || { echo "target count changed between trials" >&2; exit 1; }
        trial=$((trial + 1))
    done
    sort -n "$speeds" -o "$speeds"
    awk -v h="$horizon" -v t="$trials" -v r="$repeats" -v targets="$targets" '
        { values[NR]=$1 }
        END {
            if (NR % 2) median=values[(NR+1)/2];
            else median=(values[NR/2]+values[NR/2+1])/2;
            printf "1,%d,%d,%d,%d,%.6f,%.6f,%.6f,true,true,ok\n", h,t,r,targets,values[1],median,values[NR]
        }
    ' "$speeds" >> "$output/summary.csv"
done
printf 'interface-quotient-scaling status=VALID summary=%s\n' "$output/summary.csv"
