# Roa Logic PLIC gateway experiment v1

Status: candidate selected and negative boundary retained. Source import,
certificate v3, independent answers, and hosted reproduction remain open.

## Purpose

This experiment introduces a third public embedded RTL project and a different
control regime: interrupt request capture, edge/level selection, claim, and
completion. It tests whether GCC's word-level exact fallback can handle several
independent control inputs without hiding them in an authored abstraction.

## Pinned candidate

- project: `RoaLogic/plic`;
- upstream commit: `fc6abe4ee04073539b77abfb6d53ee19b25bbc75`;
- upstream module: `rtl/verilog/core/plic_gateway.sv`;
- upstream role: RISC-V Platform-Level Interrupt Controller gateway;
- licence: permissive source notice requiring retention of the copyright and
  disclaimer; and
- configuration: `MAX_PENDING_COUNT=3`.

The upstream source must remain byte-identical. GCC-owned wrappers, properties,
canonicalisation, and build recipes must be identified separately. No wrapper
claim may be described as an upstream product guarantee.

## Initial feasibility result

Pinned Yosys accepts and flattens the unmodified module after its standard
asynchronous-reset lowering. A minimal GCC-owned wrapper exposing reset,
source, edge/level, claim, and completion produces a valid BTOR2 model with:

- 97 nodes;
- five semantic one-bit inputs;
- seven state nodes;
- two bad properties;
- no constraints; and
- maximum word width two.

The current portfolio refuses the first property before producing an artifact:
`bounded search requires exactly one one-bit input`. This is the required
negative baseline. It demonstrates an actual semantic boundary rather than a
performance comparison selected after observing favourable results.

## Property boundary

The first wrapper properties cover only gateway protocol integrity:

1. pending may not appear before a source request has been observed; and
2. pending may not reappear while a claimed request awaits completion.

Both properties require independent review and comparison with a maintained
model checker. They are wrapper requirements, not statements copied from the
upstream PLIC specification. Later environments must include both edge and
level behavior plus claim/completion sequencing.

## Required result

The experiment proceeds through the predeclared
[bounded search v3 plan](BTOR2_BOUNDED_SEARCH_V3_PLAN.md). It must retain every
input valuation exactly, preserve shortest UNSAFE traces, prove SAFE by complete
successor construction, and use no PLIC-specific recogniser or route hint.

Success closes a multi-input interoperability and third-public-project
mechanism gap. It does not close independent partner acceptance, establish that
the complete PLIC is safe, or support an algorithmic novelty claim.

