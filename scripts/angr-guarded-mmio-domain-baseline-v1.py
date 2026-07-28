#!/usr/bin/env python3
"""Derive the complete guarded-MMIO behavior domain with maintained angr."""

from __future__ import annotations

import sys
import resource
import time
from dataclasses import dataclass

import angr
import claripy


MAX_ACTIVE_STATES = 64
MAX_GLOBAL_STEPS = 10_000
MAX_EVENTS = 32
STOP_ADDRESS = 0x9000_0000
STACK_ADDRESS = 0x8FFF_F000


@dataclass(frozen=True)
class Behavior:
    inputs: tuple[int, ...]
    return_value: int
    events: tuple[tuple[int, int, int], ...]


def required_symbol(project: angr.Project, name: str) -> int:
    symbol = project.loader.find_symbol(name)
    if symbol is None:
        raise RuntimeError(f"missing symbol {name}")
    return symbol.rebased_addr


def one_value(
    state: angr.SimState,
    value: claripy.ast.BV,
    field: str,
    constraint: claripy.ast.Bool | None = None,
) -> int:
    extra_constraints = [] if constraint is None else [constraint]
    choices = state.solver.eval_upto(
        value, 2, extra_constraints=extra_constraints
    )
    if len(choices) != 1:
        raise RuntimeError(f"{field} is not uniquely determined")
    return choices[0]


def compact_inputs(inputs: tuple[int, ...]) -> str:
    ranges: list[str] = []
    start = inputs[0]
    end = start
    for value in inputs[1:]:
        if value == end + 1:
            end = value
            continue
        ranges.append(str(start) if start == end else f"{start}-{end}")
        start = value
        end = value
    ranges.append(str(start) if start == end else f"{start}-{end}")
    return ";".join(ranges)


def derive(elf: str) -> tuple[list[Behavior], int]:
    project = angr.Project(elf, auto_load_libs=False)
    entry = required_symbol(project, "gcc_firmware_entry")
    event_count_address = required_symbol(project, "gcc_mmio_event_count")
    events_address = required_symbol(project, "gcc_mmio_events")
    runtime_channel = claripy.BVS("runtime_channel", 32, explicit_name=True)
    state = project.factory.blank_state(
        addr=entry,
        add_options={
            angr.options.ZERO_FILL_UNCONSTRAINED_MEMORY,
            angr.options.ZERO_FILL_UNCONSTRAINED_REGISTERS,
        },
    )
    state.regs.a0 = runtime_channel
    state.regs.ra = STOP_ADDRESS
    state.regs.sp = STACK_ADDRESS
    state.solver.add(claripy.ULE(runtime_channel, 255))
    manager = project.factory.simgr(state)
    manager.stashes["returned"] = []
    steps = 0
    while manager.active:
        manager.move(
            from_stash="active",
            to_stash="returned",
            filter_func=lambda item: item.addr == STOP_ADDRESS,
        )
        if not manager.active:
            break
        if len(manager.active) > MAX_ACTIVE_STATES:
            raise RuntimeError("active symbolic state count exceeds policy")
        manager.step()
        steps += 1
        if steps > MAX_GLOBAL_STEPS:
            raise RuntimeError("symbolic execution step count exceeds policy")
    if manager.deadended or manager.errored or manager.unconstrained:
        raise RuntimeError(
            "symbolic execution did not return cleanly "
            f"deadended={len(manager.deadended)} "
            f"errored={len(manager.errored)} "
            f"unconstrained={len(manager.unconstrained)}"
        )
    if not manager.returned:
        raise RuntimeError("symbolic execution produced no return state")

    grouped: dict[
        tuple[int, tuple[tuple[int, int, int], ...]], list[int]
    ] = {}
    covered: set[int] = set()
    for returned in manager.returned:
        inputs = tuple(
            sorted(returned.solver.eval_upto(runtime_channel, 257, cast_to=int))
        )
        if not inputs or len(inputs) > 256:
            raise RuntimeError("returned path has invalid input membership")
        overlap = covered.intersection(inputs)
        if overlap:
            raise RuntimeError(f"returned paths overlap at input {min(overlap)}")
        covered.update(inputs)
        for input_value in inputs:
            constraint = runtime_channel == input_value
            return_value = one_value(
                returned, returned.regs.a0, "return value", constraint
            )
            count = one_value(
                returned,
                returned.memory.load(
                    event_count_address, 4, endness=project.arch.memory_endness
                ),
                "event count",
                constraint,
            )
            if count > MAX_EVENTS:
                raise RuntimeError("event count exceeds policy")
            events: list[tuple[int, int, int]] = []
            for index in range(count):
                fields = tuple(
                    one_value(
                        returned,
                        returned.memory.load(
                            events_address + index * 12 + field * 4,
                            4,
                            endness=project.arch.memory_endness,
                        ),
                        f"event {index} field {field}",
                        constraint,
                    )
                    for field in range(3)
                )
                events.append(fields)
            grouped.setdefault((return_value, tuple(events)), []).append(
                input_value
            )
    if covered != set(range(256)):
        missing = sorted(set(range(256)).difference(covered))
        raise RuntimeError(f"symbolic paths do not cover domain: {missing[:8]}")
    behaviors = [
        Behavior(tuple(sorted(inputs)), return_value, events)
        for (return_value, events), inputs in grouped.items()
    ]
    behaviors.sort(key=lambda behavior: behavior.inputs)
    return behaviors, steps


def main() -> None:
    if len(sys.argv) != 2:
        raise RuntimeError(
            "usage: angr-guarded-mmio-domain-baseline-v1.py FIRMWARE_ELF"
        )
    wall_start = time.monotonic()
    usage_start = resource.getrusage(resource.RUSAGE_SELF)
    behaviors, steps = derive(sys.argv[1])
    usage_end = resource.getrusage(resource.RUSAGE_SELF)
    wall_seconds = time.monotonic() - wall_start
    peak_rss_bytes = usage_end.ru_maxrss * 1024
    print(f"angr_version={angr.__version__}")
    print("input_domain=0-255")
    print(f"behavior_count={len(behaviors)}")
    print(f"symbolic_global_steps={steps}")
    for index, behavior in enumerate(behaviors):
        print(
            f"behavior={index},inputs={compact_inputs(behavior.inputs)},"
            f"return={behavior.return_value},events={len(behavior.events)}"
        )
        for event_index, (operation, offset, value) in enumerate(behavior.events):
            print(
                f"event={index},{event_index},{operation},{offset},{value}"
            )
    print("coverage=256")
    print("disjoint=true")
    print(f"analysis_wall_seconds={wall_seconds:.6f}")
    print(
        f"analysis_user_seconds="
        f"{usage_end.ru_utime - usage_start.ru_utime:.6f}"
    )
    print(
        f"analysis_system_seconds="
        f"{usage_end.ru_stime - usage_start.ru_stime:.6f}"
    )
    print(f"analysis_peak_rss_bytes={peak_rss_bytes}")
    print("status=complete")


if __name__ == "__main__":
    main()
