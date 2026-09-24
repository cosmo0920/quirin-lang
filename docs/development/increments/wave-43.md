# Wave 43 records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-41-continuation.md) is preserved unchanged after reaching
> 36,333 bytes.

**`W43-LIBC-PRINTF-FAMILY` (2026-09-25).** Declared `printf`, `fprintf`, `snprintf` and `sprintf` in `providers/quirin-provider-libc/contracts/libc/stdio.qcore`: every declared parameter borrowed, a variadic data tail, and `resource_return_none()` for the `int` result, each a trusted assumption. The declared-ABI census reads all four as agreeing in the 168 units that declare them. The `va_list` members are refused by name, because the reader cannot read `__gnuc_va_list`; `dprintf` is never called. The prediction was frozen before any declaration was drafted. Arbiter, one member per stage: BEFORE reproduces the W43-LIBC-RESULT-ALIAS diagnostics byte for byte, and AFTER removes 2,229 leaves and adds 1,283, all attributed per member. Five functions move Unknown → Safe (`cmt_errno_print`, `ctr_errno_print`, `cio_errno_print`, `rd_kafka_mk_nodename`, `cJSON_Version`), neither ledger-forecast exchange fires, no unit moves, and the leak pin stays at 272. The forecast got the leaf exchange wrong: 1,179 declared-tail refusals (1,081 on librdkafka `RD_UT_*` macro lines, mostly in bodies whose typed facts are unreadable) and 100 nested-call refusals, against a forecast of 90. No function is blocked only by them, so the tail rule was not widened. Pure contract data; no Lean change. Follow-ups 299 → 301: `LIBC-PRINTF-FAMILY` retired; `LIBC-PRINTF-VA-LIST`, `LIBC-PRINTF-TAIL-ACTUALS` and `LIBC-PRINTF-NESTED-TAIL-CALLEES` registered. [Report](../../reports/w43-libc-printf-family/README.md).
