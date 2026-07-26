#!/usr/bin/env python3
"""Recover GCC's bounded MMIO event memory with maintained angr."""

import sys

import angr


MAX_BLOCKS = 10_000
STOP_ADDRESS = 0x9000_0000
STACK_ADDRESS = 0x8FFF_F000


def required_symbol(project: angr.Project, name: str) -> int:
    symbol = project.loader.find_symbol(name)
    if symbol is None:
        raise RuntimeError(f"missing symbol {name}")
    return symbol.rebased_addr


def main() -> None:
    if len(sys.argv) != 2:
        raise RuntimeError("usage: angr-compiled-mmio-baseline-v1.py FIRMWARE_ELF")
    project = angr.Project(sys.argv[1], auto_load_libs=False)
    entry = required_symbol(project, "gcc_firmware_entry")
    event_count = required_symbol(project, "gcc_mmio_event_count")
    events = required_symbol(project, "gcc_mmio_events")
    state = project.factory.blank_state(
        addr=entry,
        add_options={
            angr.options.ZERO_FILL_UNCONSTRAINED_MEMORY,
            angr.options.ZERO_FILL_UNCONSTRAINED_REGISTERS,
        },
    )
    state.regs.ra = STOP_ADDRESS
    state.regs.sp = STACK_ADDRESS
    manager = project.factory.simgr(state)
    blocks = 0
    while manager.active and not any(
        item.addr == STOP_ADDRESS for item in manager.active
    ):
        manager.step()
        blocks += 1
        if blocks > MAX_BLOCKS or len(manager.active) != 1:
            address = hex(manager.active[0].addr) if manager.active else "none"
            raise RuntimeError(
                "baseline outside policy "
                f"blocks={blocks} active={len(manager.active)} address={address}"
            )
    matching = [item for item in manager.active if item.addr == STOP_ADDRESS]
    if len(matching) != 1:
        raise RuntimeError("baseline did not reach one exact return state")
    state = matching[0]
    count = state.solver.eval(
        state.memory.load(event_count, 4, endness=project.arch.memory_endness)
    )
    if count > 32:
        raise RuntimeError("baseline event count exceeds policy")

    print(f"angr_version={angr.__version__}")
    print(f"architecture={project.arch.name}")
    print(f"event_count={count}")
    for index in range(count):
        values = [
            state.solver.eval(
                state.memory.load(
                    events + index * 12 + field * 4,
                    4,
                    endness=project.arch.memory_endness,
                )
            )
            for field in range(3)
        ]
        print(f"event={index},{values[0]},{values[1]},{values[2]}")
    print(f"angr_blocks={blocks}")
    print("status=complete")


if __name__ == "__main__":
    main()
