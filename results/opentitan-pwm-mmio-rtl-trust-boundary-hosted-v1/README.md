# OpenTitan PWM MMIO-to-RTL hosted trust-boundary result v1

The predeclared hosted reproduction passed on GitHub-hosted Ubuntu 24.04
x86-64:

- run:
  [30438790163](https://github.com/kabudu/guarded-continuation-checker/actions/runs/30438790163);
- revision: `f44bc350d21a8747dbbe91b419bb61b7f86722ef`;
- artifact ID: `8719172826`;
- uploaded artifact SHA-256:
  `4a742aef526b7225547fbfdba38aac5abe553232925642c5d83ed1a7e282c0d9`;
  and
- result: passed every predeclared hosted gate.

Both clean cycles recovered the frozen complete semantic answer, refused all
four GCC and ten maintained-route hostile changes, and passed five
fresh-process trials per route. Their Linux consumer executables,
certificates, semantic summaries and identity manifests are byte-identical.

## Measured result

| Cycle | GCC median wall | Maintained median wall | GCC maximum RSS | Maintained minimum RSS |
| --- | ---: | ---: | ---: | ---: |
| 1 | 0.06 s | 4.95 s | 8,761,344 bytes | 67,084,288 bytes |
| 2 | 0.06 s | 4.96 s | 8,945,664 bytes | 67,076,096 bytes |

The least favorable hosted ratio is 82.50 times in median warm-consumer wall
time and 7.50 times in peak RSS. The unchanged twofold thresholds pass with
substantial margin.

The payload tradeoff also reproduces: GCC transfers 110,831 bytes and the
maintained route transfers 31,679 bytes, so GCC transfers about 3.50 times
more. The hosted release consumer is 63,449,344 bytes and the freshly built
angr image reports 715,271,127 bytes.

Setup and producer costs remain separate and material. Across the two cycles:

- input preparation takes 46.92 and 12.11 seconds;
- the clean GCC consumer build takes 41.22 and 41.28 seconds and peaks at
  about 1.48 to 1.49 GB RSS;
- the GCC producer build takes 18.21 and 18.09 seconds and peaks at about
  1.26 GB RSS;
- the fresh maintained image build takes 28.65 and 25.12 seconds; and
- certificate production takes 19.24 and 18.92 seconds.

The first input-preparation cycle includes cold toolchain work; the second
benefits from the runner's warm process and filesystem state. Neither setup
observation is included in the warm-consumer ratios.

## Retained identities

- normalized semantic SHA-256:
  `e7a87b007d82f2c7cee41d4b005066c4ba94f8c690df5f0e38f423ff65907abf`;
- certificate SHA-256:
  `90dfeff77ee5eeb32f0c578d3e9a37429bcf2b8665d79dea05983f83ab9c8cd3`;
- Linux consumer SHA-256:
  `8fc7f7b8bea932f85a8a4f23c5387988bb9a88622aae7ac0c5a85a493363e716`;
- identity-manifest SHA-256:
  `5adcd2f9fe8e6e9f3024477169112e3b246f54db7095e2baab0ab51b66f07002`;
  and
- semantic-summary file SHA-256:
  `84482e14e111bab7bb4e9955506e11d529c2bb3b287040ade375c6ef653a482e`.

The certificate SHA-256 matches the retained Darwin result. The Linux ELF
consumer is intentionally not required to match the Darwin Mach-O executable.

This closes bounded Ubuntu platform reproduction for the frozen
trust-boundary comparison. It does not close compatibility history, external
independent assessment, production readiness, novelty or general performance.
