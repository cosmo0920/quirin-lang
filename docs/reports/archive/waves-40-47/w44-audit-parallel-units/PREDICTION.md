# W44-AUDIT-PARALLEL-UNITS — prediction (frozen before any parallel run)

Written from the base profile only. The implementation was drafted while
that profile ran, but no run of it had been timed when this file was
committed; later measurements go in the README beside it, not here.

## Base profile (1ab56306, pinned route, one release run)

Measured with `scripts/profile_run.py` on a binary carrying
`scripts/profile-instrumentation.patch` (wall-time spans per stage and per
unit), `strace -f -e trace=execve` for every subprocess, and a 0.25 s RSS
sampler. The instrumented run's diagnostics are byte-identical to an
uninstrumented base run's; its `audit.log` differs only in the line naming the
output directory. Summary: `profile/base-pinned-summary.json`.

| Quantity | Value |
|---|---|
| Instrumented run wall | 522.1 s (host load average 5–7 from other work) |
| Uninstrumented base run (`/usr/bin/time -v`) | 590.5 s wall, 554.5 s user, 24.5 s system, 173,152 KiB peak RSS |
| Units | 185 |
| Sum of per-unit wall | 518.6 s (99.3 % of the run) |
| of which Clang subprocesses | 67.6 s (2,652 Clang runs; 70.7 s over the whole run) |
| of which in-process analysis | 451.0 s |
| Everything outside the unit loop | 3.5 s: ChunkIO library prelude 1.94 s, Monkey callback source 1.31 s, run inputs 0.14 s, serialization 0.08 s, ChunkIO postlude 0.02 s; the three summary profiles 0.002 s |
| Critical path (largest unit) | `librdkafka/rdkafka_admin.c`, 46.4 s (45.7 s in process, 0.65 s Clang) |
| Next units | `rdkafka.c` 20.1, `rdkafka_partition.c` 19.7, `rdkafka_cgrp.c` 19.1, `rdkafka_request.c` 17.9, `rdkafka_broker.c` 16.4, `rdkafka_mock.c` 15.8, `rdkafka_mock_handlers.c` 12.9, `flb_input_chunk.c` 12.6 (6.8 s of it Clang) |
| Peak RSS | 173 MiB audit process; 216 MiB process tree including Clang |

The work is in-process analysis, not Clang: Clang is 13 % of unit time. The
unit loop is 99.3 % of the run, and the only cross-unit dependencies are the
ChunkIO prelude (consumed by four ChunkIO units) and the ChunkIO postlude
(consuming five of them).

Unit time ranks closely with source size (Spearman 0.88; the largest file is
the slowest unit), so the implementation dispatches units largest first.

## Host

AMD Ryzen AI 7 350: 8 cores / 16 threads, four of the cores the compact
variant, laptop power limits. Other agents' work kept the load average at 5–7
during the base measurements.

## Prediction

Model: `T(N) ≈ S + max(C, U / E(N))`, with `S` ≈ 0.3 s truly serial (inputs,
serialization; the ChunkIO prelude and Monkey gate overlap unit work),
`U` = 518.6 s, `C` the critical unit's wall at that load, and `E(N)` the
effective parallel throughput in single-thread units. With largest-first
dispatch the critical unit starts first.

| Jobs | E(N) assumed | U / E(N) | C assumed | Predicted wall | Predicted peak RSS |
|---|---|---|---|---|---|
| 1 | 1 | 519 s | 46 s | 520–600 s (base) | 0.17 GiB |
| 4 | 3.6–3.9 | 133–144 s | 46–50 s | 135–150 s | 0.4–0.7 GiB |
| 8 | 6–7 | 74–86 s | 50–55 s | 75–90 s | 0.6–1.2 GiB |
| 16 | 8–10 (SMT, compact cores, power limit) | 52–65 s | 60–70 s (shares its core) | 60–75 s | 1–2.5 GiB |

At 16 jobs the critical unit, not the total, is expected to bound wall time;
a residual follow-up is warranted if it does. User time is predicted to rise
with N (SMT and frequency scaling), by 20–50 % at 16 jobs. Memory is
predicted to stay far below the host's 60 GiB at every N, so the default is
capped at 16 for scheduling reasons (no gain is expected beyond the
thread count) rather than memory.

The configured route (`run_current`) has the same per-unit shape; the user's
own data point (186 configurations, 558 Clang invocations, 42 s of Clang
preparation) likewise puts most of its time in process, and the same scaling
is predicted.
