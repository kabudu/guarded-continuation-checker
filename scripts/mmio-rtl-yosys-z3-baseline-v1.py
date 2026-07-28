#!/usr/bin/env python3
"""Independently translate angr MMIO rows and replay them with Yosys plus Z3."""

from __future__ import annotations

import re
import resource
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, replace
from pathlib import Path


CHANNELS = 6
EVENT_FRAMES = 16
BASE_FRAMES = EVENT_FRAMES + 1
CONTINUATION_FRAMES = 16
TRACE_FRAMES = BASE_FRAMES + CONTINUATION_FRAMES
OBSERVATIONS = TRACE_FRAMES + 1
READ = 1
WRITE = 2
OBSERVE_CHANNEL_0 = 3
REGWEN = 0x04
CFG = 0x08
ENABLE = 0x0C
INVERT = 0x10
PARAMETER_0 = 0x14
DUTY_CYCLE_0 = 0x2C
PHASE_TICKS_PER_BEAT = 4096
EXPECTED_Z3 = "Z3 version 4.16.0"
TOP = "opentitan_pwm_firmware_trace_harness"


@dataclass(frozen=True)
class Event:
    operation: int
    offset: int
    value: int


@dataclass(frozen=True)
class Behavior:
    inputs: str
    return_value: int
    events: tuple[Event, ...]


@dataclass(frozen=True)
class Inputs:
    enable_write: int = 0
    invert_write: int = 0
    parameter_write: int = 0
    duty_cycle_write: int = 0
    blink_parameter_write: int = 0
    channel_enable: int = 0
    channel_invert: int = 0
    blink_enable: int = 0
    heartbeat_enable: int = 0
    phase_delay: tuple[int, ...] = (0,) * CHANNELS
    duty_cycle_a: tuple[int, ...] = (0,) * CHANNELS
    duty_cycle_b: tuple[int, ...] = (0,) * CHANNELS
    blink_parameter_x: tuple[int, ...] = (0,) * CHANNELS
    blink_parameter_y: tuple[int, ...] = (0,) * CHANNELS


def parse_behaviors(path: Path) -> list[Behavior]:
    headers: dict[int, tuple[str, int, int]] = {}
    events: dict[int, list[Event]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("behavior="):
            match = re.fullmatch(
                r"behavior=(\d+),inputs=([^,]+),return=(\d+),events=(\d+)",
                line,
            )
            if match is None:
                raise RuntimeError("malformed angr behavior row")
            index = int(match.group(1))
            if index in headers:
                raise RuntimeError("duplicate angr behavior row")
            headers[index] = (
                match.group(2),
                int(match.group(3)),
                int(match.group(4)),
            )
            events[index] = []
        elif line.startswith("event="):
            match = re.fullmatch(
                r"event=(\d+),(\d+),(\d+),(\d+),(\d+)", line
            )
            if match is None:
                raise RuntimeError("malformed angr event row")
            behavior = int(match.group(1))
            event_index = int(match.group(2))
            if behavior not in events or event_index != len(events[behavior]):
                raise RuntimeError("angr event order is not canonical")
            events[behavior].append(
                Event(
                    int(match.group(3)),
                    int(match.group(4)),
                    int(match.group(5)),
                )
            )
    if sorted(headers) != list(range(7)):
        raise RuntimeError("angr behavior identifiers are not canonical")
    result = []
    for index in range(7):
        inputs, return_value, count = headers[index]
        if len(events[index]) != count:
            raise RuntimeError("angr event count differs from rows")
        result.append(
            Behavior(inputs, return_value, tuple(events[index]))
        )
    return result


def expected_valid(channel: int) -> tuple[Event, ...]:
    return (
        Event(READ, REGWEN, 1),
        Event(READ, CFG, 3),
        Event(READ, INVERT, 0),
        Event(WRITE, DUTY_CYCLE_0, 0x80004000),
        Event(WRITE, PARAMETER_0, 0),
        Event(WRITE, INVERT, 0),
        Event(READ, REGWEN, 1),
        Event(READ, ENABLE, 0),
        Event(WRITE, ENABLE, 1),
        Event(READ, REGWEN, 1),
        Event(READ, CFG, 3),
        Event(READ, INVERT, 0),
        Event(WRITE, DUTY_CYCLE_0 + 4 * channel, 0xA0006000),
        Event(WRITE, PARAMETER_0 + 4 * channel, 0x2000),
        Event(WRITE, INVERT, 0),
        Event(OBSERVE_CHANNEL_0, 0, 1),
    )


def validate_domain(behaviors: list[Behavior]) -> None:
    for channel in range(CHANNELS):
        behavior = behaviors[channel]
        if (
            behavior.inputs != str(channel)
            or behavior.return_value != 0
            or behavior.events != expected_valid(channel)
        ):
            raise RuntimeError(f"valid behavior {channel} is not canonical")
    expected_invalid = expected_valid(0)[:9] + (
        Event(OBSERVE_CHANNEL_0, 0, 1),
    )
    invalid = behaviors[6]
    if (
        invalid.inputs != "6-255"
        or invalid.return_value != 2
        or invalid.events != expected_invalid
    ):
        raise RuntimeError("invalid behavior is not canonical")


def normalize_beat(value: int, role: str) -> int:
    if value % PHASE_TICKS_PER_BEAT:
        raise RuntimeError(f"{role} is not exactly representable")
    result = value // PHASE_TICKS_PER_BEAT
    if result > 15:
        raise RuntimeError(f"{role} exceeds four bits")
    return result


def replace_index(values: tuple[int, ...], index: int, value: int) -> tuple[int, ...]:
    changed = list(values)
    changed[index] = value
    return tuple(changed)


def clear_writes(inputs: Inputs) -> Inputs:
    return replace(
        inputs,
        enable_write=0,
        invert_write=0,
        parameter_write=0,
        duty_cycle_write=0,
        blink_parameter_write=0,
    )


def apply_write(inputs: Inputs, event: Event, channel: int) -> Inputs:
    bit = 1 << channel
    if event.offset == ENABLE:
        if event.value & ~0x3F:
            raise RuntimeError("enable value exceeds six channels")
        return replace(
            inputs, channel_enable=event.value, enable_write=0x3F
        )
    if event.offset == INVERT:
        if event.value & ~0x3F:
            raise RuntimeError("invert value exceeds six channels")
        return replace(
            inputs, channel_invert=event.value, invert_write=0x3F
        )
    if event.offset == DUTY_CYCLE_0 + 4 * channel:
        return replace(
            inputs,
            duty_cycle_a=replace_index(
                inputs.duty_cycle_a,
                channel,
                normalize_beat(event.value & 0xFFFF, "duty cycle A"),
            ),
            duty_cycle_b=replace_index(
                inputs.duty_cycle_b,
                channel,
                normalize_beat(event.value >> 16, "duty cycle B"),
            ),
            duty_cycle_write=bit,
        )
    if event.offset == PARAMETER_0 + 4 * channel:
        blink = (inputs.blink_enable & ~bit) | (((event.value >> 31) & 1) * bit)
        heartbeat = (inputs.heartbeat_enable & ~bit) | (
            ((event.value >> 30) & 1) * bit
        )
        return replace(
            inputs,
            phase_delay=replace_index(
                inputs.phase_delay,
                channel,
                normalize_beat(event.value & 0xFFFF, "phase delay"),
            ),
            blink_enable=blink,
            heartbeat_enable=heartbeat,
            parameter_write=bit,
        )
    raise RuntimeError("write is not owned by expected channel")


def map_behavior(channel: int, behavior: Behavior) -> list[Inputs]:
    if behavior.events != expected_valid(channel):
        raise RuntimeError("behavior changed after domain validation")
    inputs = Inputs()
    frames = [inputs]
    for index, event in enumerate(behavior.events):
        inputs = clear_writes(inputs)
        if event.operation == WRITE:
            inputs = apply_write(inputs, event, 0 if index < 9 else channel)
        frames.append(inputs)
    quiescent = clear_writes(frames[-1])
    frames.extend([quiescent] * CONTINUATION_FRAMES)
    if len(frames) != TRACE_FRAMES:
        raise RuntimeError("mapped frame count is not canonical")
    return frames


def bit_vector(width: int, value: int) -> str:
    if value < 0 or value >= 1 << width:
        raise RuntimeError("input value exceeds declared width")
    return f"#b{value:0{width}b}"


def named_inputs(inputs: Inputs) -> dict[str, tuple[int, int]]:
    result = {
        "clk_i": (1, 0),
        "enable_write_i": (6, inputs.enable_write),
        "invert_write_i": (6, inputs.invert_write),
        "parameter_write_i": (6, inputs.parameter_write),
        "duty_cycle_write_i": (6, inputs.duty_cycle_write),
        "blink_parameter_write_i": (6, inputs.blink_parameter_write),
        "channel_enable_i": (6, inputs.channel_enable),
        "channel_invert_i": (6, inputs.channel_invert),
        "blink_enable_i": (6, inputs.blink_enable),
        "heartbeat_enable_i": (6, inputs.heartbeat_enable),
    }
    for channel in range(CHANNELS):
        result[f"phase_delay_{channel}_i"] = (4, inputs.phase_delay[channel])
        result[f"duty_cycle_a_{channel}_i"] = (
            4,
            inputs.duty_cycle_a[channel],
        )
        result[f"duty_cycle_b_{channel}_i"] = (
            4,
            inputs.duty_cycle_b[channel],
        )
        result[f"blink_parameter_x_{channel}_i"] = (
            4,
            inputs.blink_parameter_x[channel],
        )
        result[f"blink_parameter_y_{channel}_i"] = (
            4,
            inputs.blink_parameter_y[channel],
        )
    if len(result) != 40:
        raise RuntimeError("maintained mapping does not bind 40 source inputs")
    return result


def state_name(channel: int, frame: int) -> str:
    return f"c{channel}s{frame}"


def query_text(model: str, channel: int, frames: list[Inputs]) -> str:
    if len(frames) != TRACE_FRAMES:
        raise RuntimeError("SMT query continuation length is not canonical")
    lines = [model]
    state_type = f"|{TOP}_s|"
    for frame in range(OBSERVATIONS):
        lines.append(
            f"(declare-fun {state_name(channel, frame)} () {state_type})"
        )
    lines.append(f"(assert (|{TOP}_i| {state_name(channel, 0)}))")
    for frame in range(OBSERVATIONS):
        state = state_name(channel, frame)
        lines.append(f"(assert (|{TOP}_h| {state}))")
        lines.append(f"(assert (|{TOP}_a| {state}))")
        lines.append(f"(assert (|{TOP}_u| {state}))")
    for frame, inputs in enumerate(frames):
        state = state_name(channel, frame)
        for symbol, (width, value) in sorted(named_inputs(inputs).items()):
            function = f"|{TOP}_n {symbol}|"
            expected = (
                "false"
                if symbol == "clk_i"
                else bit_vector(width, value)
            )
            lines.append(f"(assert (= ({function} {state}) {expected}))")
        lines.append(
            f"(assert (|{TOP}_t| {state} {state_name(channel, frame + 1)}))"
        )
    lines.append("(check-sat)")
    for frame in range(OBSERVATIONS):
        state = state_name(channel, frame)
        lines.append(f'(echo "OBS,{frame}")')
        lines.append(
            f"(get-value ((|{TOP}_n step_o| {state}) "
            f"(|{TOP}_n pwm_o| {state})))"
        )
    return "\n".join(lines) + "\n"


def parse_value(token: str) -> int:
    if token.startswith("#b"):
        return int(token[2:], 2)
    if token.startswith("#x"):
        return int(token[2:], 16)
    raise RuntimeError(f"unexpected SMT value {token}")


def run_member(
    z3: Path, model: str, channel: int, frames: list[Inputs]
) -> list[tuple[int, int]]:
    query = query_text(model, channel, frames)
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=".smt2"
    ) as handle:
        handle.write(query)
        handle.flush()
        completed = subprocess.run(
            [str(z3), handle.name],
            check=False,
            capture_output=True,
            text=True,
            timeout=120,
        )
    if completed.returncode != 0 or completed.stderr:
        raise RuntimeError(
            f"Z3 failed for channel {channel}: "
            f"return={completed.returncode} stderr={completed.stderr!r}"
        )
    lines = [line.strip() for line in completed.stdout.splitlines()]
    if not lines or lines[0] != "sat":
        raise RuntimeError(f"Z3 did not return sat for channel {channel}")
    observations: list[tuple[int, int]] = []
    position = 1
    for frame in range(OBSERVATIONS):
        if position + 2 >= len(lines) or lines[position] != f"OBS,{frame}":
            raise RuntimeError("Z3 observation labels are not canonical")
        values = re.findall(
            r"(#[bx][0-9a-fA-F]+)\)",
            f"{lines[position + 1]} {lines[position + 2]}",
        )
        if len(values) != 2:
            raise RuntimeError(
                "unexpected Z3 observation rows "
                f"{lines[position + 1:position + 3]!r}"
            )
        observations.append(
            (parse_value(values[0]), parse_value(values[1]))
        )
        position += 3
    if position != len(lines):
        raise RuntimeError("Z3 returned trailing output")
    return observations


def main() -> None:
    if len(sys.argv) != 4:
        raise RuntimeError(
            "usage: mmio-rtl-yosys-z3-baseline-v1.py "
            "ANGR_ROWS MODEL_SMT2 Z3"
        )
    wall_start = time.monotonic()
    self_start = resource.getrusage(resource.RUSAGE_SELF)
    children_start = resource.getrusage(resource.RUSAGE_CHILDREN)
    angr_rows = Path(sys.argv[1])
    model_path = Path(sys.argv[2])
    z3 = Path(sys.argv[3])
    version = subprocess.run(
        [str(z3), "--version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout.strip()
    if not version.startswith(EXPECTED_Z3):
        raise RuntimeError(f"Z3 version differs from policy: {version}")
    behaviors = parse_behaviors(angr_rows)
    validate_domain(behaviors)
    model = model_path.read_text(encoding="utf-8")
    if f"(define-fun |{TOP}_t|" not in model:
        raise RuntimeError("SMT2 model does not expose expected transition")
    traces = []
    for channel in range(CHANNELS):
        frames = map_behavior(channel, behaviors[channel])
        observations = run_member(z3, model, channel, frames)
        traces.append(observations)
        encoded = ",".join(
            f"{step:x}:{pwm:02x}" for step, pwm in observations
        )
        print(
            f"maintained_rtl_trace={channel},transitions={len(frames)},"
            f"observations={encoded}"
        )
    classes = len({tuple(trace) for trace in traces})
    nonzero = sum(any(pwm != 0 for _, pwm in trace) for trace in traces)
    if classes != 2 or nonzero != CHANNELS:
        raise RuntimeError(
            f"maintained observation gate failed classes={classes} "
            f"nonzero={nonzero}"
        )
    print(f"maintained_valid_rtl_members={len(traces)}")
    print("maintained_invalid_rtl_members=0")
    print(f"maintained_rtl_transitions={CHANNELS * TRACE_FRAMES}")
    print(f"maintained_rtl_observations={CHANNELS * OBSERVATIONS}")
    print(f"maintained_phase_cycle_classes={classes}")
    print(f"maintained_nonzero_traces={nonzero}")
    print(f"z3_version={version}")
    self_end = resource.getrusage(resource.RUSAGE_SELF)
    children_end = resource.getrusage(resource.RUSAGE_CHILDREN)
    print(f"replay_wall_seconds={time.monotonic() - wall_start:.6f}")
    print(
        f"replay_user_seconds="
        f"{self_end.ru_utime - self_start.ru_utime + children_end.ru_utime - children_start.ru_utime:.6f}"
    )
    print(
        f"replay_system_seconds="
        f"{self_end.ru_stime - self_start.ru_stime + children_end.ru_stime - children_start.ru_stime:.6f}"
    )
    self_peak = self_end.ru_maxrss
    child_peak = children_end.ru_maxrss
    if sys.platform != "darwin":
        self_peak *= 1024
        child_peak *= 1024
    print(f"replay_peak_rss_bytes={max(self_peak, child_peak)}")
    print("status=complete")


if __name__ == "__main__":
    main()
