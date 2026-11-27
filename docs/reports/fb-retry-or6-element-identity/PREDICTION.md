# FB-RETRY-OR6-ELEMENT-IDENTITY: prediction

Frozen before any tip measurement. Base `76c1f9cfc` (r77).

**Disclosure.** The gateway code (gateway rule 91,
`legacy_carried_link.rs` and its hooks), the Lean module and the fixtures
were drafted in the worktree before this file was committed. The drafted
code was run only on the card's fixtures
(`crates/qcore-c-gateway/tests/held_formal_link_obligation.rs`, whose base
verdicts the probe commit `a82b33559` pins); nothing had been measured on a
native route. The base side of the opt-in current run was in flight.

## Census at base

[scripts/census.py](scripts/census.py) reads every `src/` and `plugins/`
function of the prepared tree (`private/fb-046accf6`, Fluent Bit 046accf6)
that hands one of its own formals, as written, to an element destroyer that
names a link member ([artifacts/census.json](artifacts/census.json)): 17
calls in 11 functions.

| Function | Destroyer | Expected at tip |
|---|---|---|
| `flb_sched_timer_cb_destroy` (`flb_scheduler.c`) | `flb_sched_timer_destroy(timer)`, unconditionally, then `return 0` | **Unknown -> Safe**: the summary carries `timer`'s `_head` obligation (an element destroyer of `timer` through `_head`) |
| `flb_output_task_queue_enqueue` (`flb_output.c`) | `flb_task_retry_destroy(retry)` on the allocation failure only (`return -1`); `retry` stored on `return 0` | **unchanged Unknown**: no mode holds at every exit, and in the exit-specific route the `return 0` class keeps the destroyer path (diverged) while `retry` is stored, so no class contract can state the clause (the unlink would be claimed where the body stores) |
| `flb_input_net_property_check`, `flb_input_plugin_property_check`, `flb_input_oauth2_jwt_property_check`, `flb_input_http_server_property_check`, `flb_output_net_property_check`, `flb_output_enable_multi_threading` | `flb_{input,output}_instance_destroy(ins)` on a failure path | unchanged: the enqueue shape (destroyed on one exit class, live on the other) |
| `flb_task_retry_reschedule`, `flb_engine_dispatch_retry` | `flb_task_retry_destroy(retry)` on a path | unchanged: the same shape, and other leaves |
| `check_purge_deleted_file`, `reconcile_file_state` (`in_tail`) | `flb_tail_file_remove(file)` on a path | unchanged: the same shape, and other leaves |

Callers: `flb_sched_event_handler` (Unknown) trades `call to uncontracted
function flb_sched_timer_cb_destroy` for a refusal at that call (its `timer`
is `(struct flb_sched_timer *) event`, neither initialised, published nor an
element); `flb_input_rate_gate_timer_cancel` (`flb_input.c`, another unit)
is unchanged (derived summaries stay in their unit).

Declared destroyers whose bodies are in the analysed units (each unlinks
`_head` with `mk_list_del` on its formal, refused at base as a plain unlink
of an unpublished object) keep their verdicts: rule 91's exit observation
only refutes a carried formal released without the carried destroyer, and
these rows already refuse.

## Mechanism

A contract stating `resource_unpublished(p, M)` at a bound position seeds
`p`'s entry with a `Ready(M)` record; rule 43's held arm, refused by rule
46 (3), admits a held object with that record for the destroyer's declared
member and spends it; the record is forgotten where rule 46 forgets a link
record and at a self-initialisation of the object; a carried formal is
observed `Released` only with a spent record. The W4 derivation offers the
clause at each bound formal handed directly to such a destroyer and attaches
it where the formal's mode is `Released`, so the summary is an element
destroyer's, checked at every call by rules 34, 43 and 46.

## Predicted moves

* system (both configurations): `flb_sched_timer_cb_destroy` Unknown ->
  Safe. Leaf trade in `flb_sched_event_handler`. No other status move.
* opt-in current (`--monkey-linked`, the route this host can run):
  `flb_sched_timer_cb_destroy` Unknown -> Safe; the same leaf trade; no other
  status move.
* slice, corpus: none (no function of either hands its formal to a
  member-naming destroyer on every exit; the corpus's Monkey drains pass
  elements).
* No Unknown -> Safe except the one named; no Safe -> Unknown; no leak,
  double release or Unsafe added or removed; the null property is
  unchanged except where the moved row's null reading follows its custody
  row.

```measure-prediction
route system
move flb_sched_timer_cb_destroy Unknown -> Safe
delta Safe +2
delta Unknown -2
route slice
none
route corpus
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have; the card records the refusal and measures the opt-in current
route base against tip.
