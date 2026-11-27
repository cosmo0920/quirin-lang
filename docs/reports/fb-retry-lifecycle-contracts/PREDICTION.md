# FB-RETRY-LIFECYCLE-CONTRACTS: prediction

Frozen before any tip measurement. Base `56584db0b` (r82); probe commit
`827cac0ce` (the census at base, [CENSUS.md](CENSUS.md)).

**Disclosure.** The contract module
(`providers/quirin-provider-fluent-bit/contracts/fluent_bit/retry_lifecycle.qv`)
and its fixtures (`crates/qcore-c-gateway/tests/retry_lifetime.rs`, module
`retry_lifecycle_contracts`) were drafted in the worktree before this file
was committed and run only on those fixtures (all pass); the provider lock
was synced and the eight declarations pinned at 046accf6 in the worktree.
Nothing had been measured at a tip on any route. The base side of the opt-in
current run was in flight.

## What the class (a) contracts reach

The census classifies 8 of the 200 uncontracted callees as statable now from
their 046accf6 bodies: `flb_sched_timer_cb_disable`, `map_free_task_id`,
`flb_task_from_fs_storage`, `flb_sched_ctx_init` (bodies in audited units,
whose own rows are Safe but publish no summary a caller reads),
`flb_output_thread_instance_get`, `flb_metrics_sum`, `flb_input_name` (bodies
outside the system selection: declared assumptions), and `flb_output_name`
(body in `flb_output.c`, checked structurally by rule 13a).

Every base row that calls one of them uncontracted carries other leaves too
([scripts/census.py](scripts/census.py)'s listing; `private/m/pred.py` over
the base report), so no row loses its last leaf:

| Row (both configurations) | Base leaves that clear | Other leaves left |
|---|---|---|
| `flb_task.c` `flb_task_destroy` | 909 `map_free_task_id` | 8 |
| `flb_scheduler.c` `flb_sched_timer_destroy` | 907 `flb_sched_timer_cb_disable` | 2 (the `mk_list_del` unlink, `timer`'s identity) |
| `flb_scheduler.c` `flb_sched_timer_invalidate` | 896 `flb_sched_timer_cb_disable` | 2 |
| `flb_scheduler.c` `flb_sched_event_handler` | 613 `flb_sched_timer_cb_disable` | 14 |
| `flb_output.c` `flb_output_flush_id_get` | 249 `flb_output_thread_instance_get` | 1 (`th_ins`'s identity) |
| `flb_output.c` `flb_output_flush_finished` | 734 `flb_output_thread_instance_get` | 5 |
| `flb_output.c` `flb_output_flush_prepare_destroy` | 230 `flb_output_thread_instance_get` | 6 |
| `flb_output.c` `flb_output_init_all` | 1569 `flb_output_name` | 72 |
| `flb_engine.c` `flb_engine_start` | 1296 `flb_sched_ctx_init` | 85 |
| `flb_engine.c` `handle_dlq_if_available` | 542 `flb_input_name`, 545 `flb_output_name` | 4 |
| `flb_engine.c` `handle_output_event` | 622, 623, 687 and nine `flb_metrics_sum` sites | 59 |
| `flb_engine_dispatch.c` `record_retry_failure_metrics` | 50, 51, 79, 80 | 4 (braced initialisers) |
| sibling units: `flb_input_chunk.c` (5 functions), `flb_storage.c` `metrics_append_input` | `flb_metrics_sum` / `flb_input_name` sites | 1 to 88 each |

A cleared leaf may be replaced by a leaf the call now exposes (a result
pointer's identity, an access through it); the census re-run reports each.

## Predicted moves

* **No status move** on the fluent-bit-system route: no lifecycle or sibling
  row loses its last leaf. The four rows whose own body is the declared one
  and is Safe at base (`flb_sched_timer_cb_disable`, `map_free_task_id`,
  `flb_task_from_fs_storage`, `flb_sched_ctx_init`, both configurations) stay
  Safe: each declaration is checked against that body and holds (the
  fixtures). `flb_output_name`'s own row stays Unknown.
* Leaves: the 26 lifecycle leaves above per configuration (52 in both) of
  `call to uncontracted function` for the eight callees clear, plus the
  sibling units' sites; no leaf is added except where a call result now
  resolves to an untracked pointer.
* No Unknown -> Safe, no Safe -> Unknown, no Unsafe added or removed.
* opt-in current (`--monkey-linked`): it reads none of the five lifecycle
  units; `flb_input_chunk.c`'s rows may trade the `flb_metrics_sum` /
  `flb_input_name` leaves; no status move.

```measure-prediction
route system
none
route slice
none
route corpus
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have; the card records the refusal and measures base against tip on
the fluent-bit-system route and the opt-in current route.
