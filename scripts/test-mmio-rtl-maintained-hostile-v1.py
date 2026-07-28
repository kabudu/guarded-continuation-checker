#!/usr/bin/env python3
"""Exercise fail-closed boundaries of the maintained MMIO-to-RTL translator."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
from pathlib import Path


def load_module(path: Path):
    spec = importlib.util.spec_from_file_location("maintained_baseline", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load maintained baseline")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def must_refuse(name: str, operation) -> None:
    try:
        operation()
    except RuntimeError:
        print(f"hostile_control={name},status=refused")
        return
    raise RuntimeError(f"hostile control was accepted: {name}")


def main() -> None:
    if len(sys.argv) != 3:
        raise RuntimeError(
            "usage: test-mmio-rtl-maintained-hostile-v1.py "
            "BASELINE ANGR_ROWS"
        )
    baseline = load_module(Path(sys.argv[1]))
    original = Path(sys.argv[2]).read_text(encoding="utf-8")

    def parse_changed(text: str):
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", suffix=".txt"
        ) as handle:
            handle.write(text)
            handle.flush()
            return baseline.parse_behaviors(Path(handle.name))

    rows = original.splitlines()
    first = next(index for index, row in enumerate(rows) if row.startswith("event=0,0,"))
    swapped = rows.copy()
    swapped[first], swapped[first + 1] = swapped[first + 1], swapped[first]
    must_refuse("event-order", lambda: parse_changed("\n".join(swapped) + "\n"))

    changed_value = original.replace("event=0,12,2,44,2684379136", "event=0,12,2,44,2684379137", 1)
    must_refuse(
        "event-value",
        lambda: baseline.validate_domain(parse_changed(changed_value)),
    )
    must_refuse("mapping-width", lambda: baseline.bit_vector(4, 16))

    behaviors = baseline.parse_behaviors(Path(sys.argv[2]))
    baseline.validate_domain(behaviors)
    frames = baseline.map_behavior(0, behaviors[0])
    must_refuse(
        "continuation-length",
        lambda: baseline.query_text("", 0, frames[:-1]),
    )
    must_refuse(
        "nonrepresentable-phase",
        lambda: baseline.normalize_beat(1, "hostile phase"),
    )
    print("hostile_controls=5")
    print("status=complete")


if __name__ == "__main__":
    main()
