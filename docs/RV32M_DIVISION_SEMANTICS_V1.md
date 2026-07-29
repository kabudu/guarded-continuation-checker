# RV32M division semantics v1

## Question

Can GCC add exact, fail-closed RV32M `DIV`, `DIVU`, `REM` and `REMU`
semantics to both bounded firmware executors without changing any memory,
control-flow, resource or unknown-value policy?

The OpenSBI UART 8250 qualification exposed `DIVU` as a concrete support
boundary. This is a separate executor experiment. It does not change that
qualification's retained negative result.

## Frozen semantics

Implement the RISC-V integer rules exactly:

| Instruction | Zero divisor | Signed overflow | Ordinary case |
| --- | --- | --- | --- |
| `DIV` | all-one quotient | dividend | quotient truncated toward zero |
| `DIVU` | all-one quotient | not applicable | unsigned quotient |
| `REM` | dividend | zero | signed remainder |
| `REMU` | dividend | not applicable | unsigned remainder |

Signed overflow is `0x80000000 / 0xffffffff`.

Both the scalar replay machine and predicate-lane machine must use the same
pure semantic functions. If either operand is runtime-unknown, the existing
machine must propagate unknownness exactly as it does for multiplication.

## Predeclared gates

The experiment succeeds only if:

1. all four instructions decode and execute through the existing RV32M
   decoder variants;
2. zero-divisor and signed-overflow results match the frozen table;
3. a deterministic cross-product of edge and ordinary operands agrees with
   independent mathematical reference functions;
4. scalar and predicate-lane execution agree for every tested pair;
5. register-read, register-write and unknownness observation remains governed
   by the existing `op_r` paths;
6. no host panic occurs for zero division or signed overflow;
7. existing unsupported instructions remain unsupported; and
8. formatting, strict clippy and the complete test suite pass.

No change is permitted to image size, step count, memory bounds, branch
policy, source binding, graph format or certificate format.

## Claim boundary

Passing would add exact support for four standard RV32M arithmetic
instructions. It would not establish complete RV32IMC emulation, arbitrary
OpenSBI support or production qualification. The unchanged OpenSBI UART cohort
must be rerun separately after this experiment passes.
