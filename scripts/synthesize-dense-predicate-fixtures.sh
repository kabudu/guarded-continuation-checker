#!/bin/sh
set -eu

repository=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

synthesise() {
    source=$1
    top=$2
    output=$3
    yosys -Q -q -p "read_verilog $source; hierarchy -check -top $top; proc; flatten; async2sync; opt; dffunmap; pmuxtree; simplemap; dffunmap; aigmap; setundef -zero; write_aiger -ascii -symbols $output"
}

synthesise \
    "$repository/examples/products/interrupt-controller/rtl/dense-interrupt-arbiter.v" \
    dense_interrupt_arbiter \
    "$repository/examples/products/interrupt-controller/firmware/dense-interrupt-arbiter.aag"
synthesise \
    "$repository/examples/products/actuator-controller/rtl/dense-actuator-interlock.v" \
    dense_actuator_interlock \
    "$repository/examples/products/actuator-controller/firmware/dense-actuator-interlock.aag"
synthesise \
    "$repository/examples/products/mobile-robot/rtl/dense-sensor-fusion.v" \
    dense_sensor_fusion \
    "$repository/examples/products/mobile-robot/firmware/dense-sensor-fusion.aag"
