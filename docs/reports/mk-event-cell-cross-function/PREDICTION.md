# MK-EVENT-CELL-CROSS-FUNCTION prediction

Committed before any code edit, on base
`0ac169180` (origin/main after RETAINING-CALL-MINTED-NAME-ACTUALS). The
decision is recorded in [PROPOSAL.md](PROPOSAL.md) §7:

> **USER DECISION 2026-11-02** (verbatim, relayed by the coordinator): "B as
> recommended".

The base measurement is `baseline:0ac169180`. It was not yet published in the
main checkout's `private/baselines/` when this file was written. The rows
below were therefore relisted at `baseline:a9f7bebbe` (PROPOSAL §1). The
landing between the two bases (RETAINING-CALL-MINTED-NAME-ACTUALS, rule 38b)
measured no status move on any route.

## What this card changes

It adds the record-type cell protocol of decision B (`quant cell`, rule 38c,
§C04.6.3, grammar G05e) and its route-G reading:

- the loader and the canonical schema;
- the guard test that introduces a protocol record's cells on its live edge;
- the guard and cell stores;
- the close of a protocol cell (Q3);
- the exit and call-boundary consistency check that replaces
  `descriptor-cell-escapes` for a record that keeps its protocol.

**No production declaration carries a `quant cell` clause in this card.**
Every new hook is the identity in an environment with no protocol, which
is every profile of the three routes. Adopting protocols for `flb_sched`,
`flb_input_thread_instance` and `flb_config` needs a host declaration in
`quirin-provider-fluent-bit`, plus a store census of those records at the
pinned revisions. Neither exists, so that adoption is registered as an
amendment of `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` and not done here.

## Rows that could move

None. The rows that carry descriptor-cell leaves are unchanged, because no
protocol is in force:

| Route | Row (×units) | Leaf |
|---|---|---|
| corpus | `mk_server_worker_loop` | `descriptor-cell-undecided` :502 |
| system | `flb_sched_create` (×2) | `descriptor-cell-undecided` :783 |
| system | `flb_sched_destroy` (×2) | `descriptor-cell-guard-undecided` :856 |
| system | `flb_engine_shutdown` (×2) | `descriptor-cell-guard-undecided` :1711, :1718 |
| system | `schedule_request_now` (×2) | `descriptor-cell-undecided` :133 |
| slice | `collector_start` (×2) | `descriptor-cell-undecided` :2351 |
| slice | `input_instance_channel_events_init` (×2) | `descriptor-cell-escapes` :1401 (three cells) |
| slice | `input_thread_instance_destroy` (×2) | `descriptor-cell-guard-undecided` :197 |

- **No `Safe` move.** No row reads a protocol.
- **No `Unsafe` move.** Rule 38c never accuses.
- **No move away from `Safe` or `Pending`.** That includes
  `flb_sched_timer_cb_disable` (system, `Safe`).

Provider locks, contract digests and the bounds identity are unchanged,
because no provider file changes. The schema field is additive and omitted
when empty, so serialized contracts are byte-identical.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
