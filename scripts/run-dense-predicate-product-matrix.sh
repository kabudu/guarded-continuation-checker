#!/bin/sh
set -eu

repository=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary="$repository/target/release/continuation-quotient-sat"
output=${1:-"$repository/target/dense-predicate-product-matrix"}
trials=${TRIALS:-10}
repeats=${REPEATS:-100}

case "$trials:$repeats" in
    *[!0-9:]*|0:*|*:0) echo "TRIALS and REPEATS must be positive integers" >&2; exit 2 ;;
esac
test ! -e "$output" || { echo "output already exists: $output" >&2; exit 2; }

"$repository/scripts/synthesize-dense-predicate-fixtures.sh"
cargo build --release --locked --manifest-path "$repository/Cargo.toml"
mkdir -p "$output"
printf 'dense_predicate_product_schema_version,family,relevant_inputs,horizon,trials,repeats,min_persistent_speedup,median_persistent_speedup,max_persistent_speedup,yosys_agreement,witness_valid,status\n' > "$output/summary.csv"

for family in interrupt actuator fusion; do
    case "$family" in
        interrupt) fixture="$repository/examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag" ;;
        actuator) fixture="$repository/examples/products/actuator-controller/firmware/dense-actuator-interlock.aag" ;;
        fusion) fixture="$repository/examples/products/mobile-robot/firmware/dense-sensor-fusion.aag" ;;
    esac
    for horizon in 8 16 32 64; do
        speeds="$output/$family-horizon-$horizon.speedups"
        : > "$speeds"
        trial=1
        while test "$trial" -le "$trials"; do
            report="$output/$family-horizon-$horizon-trial-$trial.csv"
            "$binary" benchmark-aiger-predicate-symbolic "$fixture" "$horizon" "$repeats" "$report"
            awk -F, 'NR == 2 { print $14 }' "$report" >> "$speeds"
            trial=$((trial + 1))
        done
        sort -n "$speeds" -o "$speeds"
        relevant=$(awk -F, 'NR == 2 { print $5 }' "$output/$family-horizon-$horizon-trial-1.csv")
        awk -v family="$family" -v relevant="$relevant" -v h="$horizon" -v t="$trials" -v r="$repeats" '
            { values[NR]=$1 }
            END {
                if (NR % 2) median=values[(NR+1)/2];
                else median=(values[NR/2]+values[NR/2+1])/2;
                printf "1,%s,%d,%d,%d,%d,%.6f,%.6f,%.6f,true,true,ok\n",family,relevant,h,t,r,values[1],median,values[NR]
            }
        ' "$speeds" >> "$output/summary.csv"
    done
done
printf 'dense-predicate-product-matrix status=VALID summary=%s\n' "$output/summary.csv"
