# OpenTitan PWM MMIO-to-RTL trust-boundary result v1

The retained positive result is identity v3, measured on Darwin arm64. Cycles
5 and 6 independently:

- verify the same O2 certificate and regenerate the same maintained answer;
- recover seven firmware behaviors, six valid RTL members, zero invalid
  members, 198 transitions, 204 observations, two phase-cycle classes and six
  non-zero traces;
- reproduce semantic SHA-256
  `e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`;
- refuse four GCC and ten maintained-route hostile changes;
- pass five fresh-process trials per route; and
- produce byte-identical consumer executable, certificate, semantic summary
  and identity-manifest bytes.

## Measured result

| Cycle | GCC median wall | Maintained median wall | GCC maximum RSS | Maintained minimum RSS |
| --- | ---: | ---: | ---: | ---: |
| 5 | 0.04 s | 2.89 s | 9,388,032 bytes | 68,075,520 bytes |
| 6 | 0.04 s | 2.86 s | 11,436,032 bytes | 67,928,064 bytes |

Across both cycles, the least favorable observed ratios are 71.50 times lower
median wall time and 5.94 times lower peak RSS for GCC verification.

The GCC transfer payload is 110,831 bytes. The maintained payload is 31,679
bytes, so GCC transfers about 3.50 times more. The GCC consumer executable is
763,056 bytes. Fresh maintained angr image observations are 192,391,187 and
192,388,264 bytes; their variation is retained as setup evidence and is not an
identity field.

Clean GCC consumer builds take 21.64 and 21.33 seconds and peak at about 1.88
GiB. Maintained angr image setup takes 27.85 and 28.51 seconds and peaks near
50 MiB in the timing process. GCC certificate production takes 10.56 and 10.94
seconds. These setup and producer observations are not included in the warm
consumer advantage.

## Files

- `identity-manifest-v3.txt`: deterministic source, tool-policy, payload,
  executable, certificate and semantic identities.
- `semantic-summary.txt`: the complete common bounded answer.
- `cycle-5-resources.csv` and `cycle-6-resources.csv`: complete setup,
  producer and fresh-process consumer observations.
- `cycle-5-resource-manifest.txt` and
  `cycle-6-resource-manifest.txt`: per-cycle threshold decisions.
- `cycle-5-payloads.txt` and `cycle-6-payloads.txt`: transfer and setup-size
  observations, including the varying fresh image size.
- `gcc-hostile.txt` and `maintained-hostile.txt`: fail-closed input controls.

This establishes a bounded deployment advantage for certificate verification.
It is not a universal performance, production-readiness or novelty claim.
