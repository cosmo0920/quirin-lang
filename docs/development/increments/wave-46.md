# Wave 46 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> records ([wave-45-continuation.md](wave-45-continuation.md), [wave-45.md](wave-45.md),
> [wave-45-history.md](wave-45-history.md)) hold the wave-45 paragraphs and stay
> unchanged. Wave 46 follows the [unsupported-coverage queue](../../plans/unsupported-coverage/README.md).

**`W46-EXTERNAL-CALLEE-CONTRACTS` (2026-09-27, base `85a35431`).** The external arm of P4 in the unsupported-coverage plan ([report](../../reports/w46-external-callee-contracts/README.md), gateway constraint 10j). Ranked the 385 external callees (2,277 sites) by the functions each alone would unblock: 33 functions, no callee above 2, and every top-30 libc callee but `fcntl` already declined for a named gap. Nine neutral contracts, each read at `66910c10` and `046accf6`: `fcntl` at `F_SETFD` only (libc, flexible, `resource_literal_argument(cmd, 2)`), `mk_utils_libc_error` (Monkey, `qcore-provider-libevent`), `rd_atomic32_get` (librdkafka), `cfl_sds_len` (cfl) and five mpack writer primitives, the eight non-libc pinned at both revisions (`pin --check` exit 0 at both). Corpus: functions 398 / 9 / 4,427 → **405 / 9 / 4,420**, units unchanged, leak 34, every leaf attributed, no correction, no new leak or violation; `rd_kafka_open_cb_generic` stays Unknown on its `&on` tail. Slice at `046accf6`: no verdict moves, one `fcntl(F_GETPIPE_SZ)` site now reads outside the premise. No Lean (existing effect kinds). Follow-ups 505 → 509, four amended, none retired.

