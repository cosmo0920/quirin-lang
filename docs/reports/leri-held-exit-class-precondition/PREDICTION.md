# LERI-HELD-FORMAL-LINK-OBLIGATION (exit-class precondition): prediction

Frozen before any tip measurement. Base `0956f4d21` (r80).

**Disclosure.** The gateway code (rule 91 (5), `legacy_carried_link.rs`
and its hooks) was drafted in the worktree before this file was committed
and run only on the card's fixtures (`link_member_premises.rs`, module
`carried_link_obligation`, whose base verdicts the probe commit `5ffea5579`
pins). The base side of the fluent-bit-system route was measured
(`private/m/sys-base`, 116 / 0 / 327 / 5); nothing was measured at tip.

## Census at base

The rows are FB-RETRY-OR6-ELEMENT-IDENTITY's census
([its census.json](../fb-retry-or6-element-identity/artifacts/census.json)):
every function of the prepared tree handing one of its own formals to a
member-naming element destroyer. Rule 91 discharged the one that destroys
on every exit; ten destroy on a failure path only. The system route reads
`flb_output.c`, `flb_task.c` and `flb_engine_dispatch.c`; it does not read
`flb_input.c` or `in_tail`.

| Function | Base leaves (system) | Expected at tip |
|---|---|---|
| `flb_output_task_queue_enqueue` | `unresolved resource identity retry` (twice), the rule 46 (3) held refusal at `flb_task_retry_destroy` | **Unknown -> Safe**: class `-1` is an element destroyer of `retry` through `_head`, class `0` consumes `retry` with `resource_links_valid(retry, _head)`, and the row reads each class's body under its class |
| `flb_output_net_property_check`, `flb_output_enable_multi_threading` | the held refusal on `ins`, plus uncontracted calls and an indirect access | unchanged Unknown (the other leaves); the `ins` leaves may go |
| `flb_task_retry_reschedule`, `flb_engine_dispatch_retry` | the held refusal on `retry` (and `task`), plus uncontracted calls, an indirect access, refuted callees | unchanged Unknown |
| `flb_input_*_property_check` (4), `check_purge_deleted_file`, `reconcile_file_state` | not on this route | not measured here |

Callers: `flb_output_task_singleplex_enqueue` (Unsafe at base on its own
`resource_user_exit` refutation, a counted-user clause) trades `call to
uncontracted function flb_output_task_queue_enqueue` for refusals at that
call: its `retry` is its own formal, which its declared contract does not
carry with `_head`. Its status stays Unsafe (the counted-user refutation is
untouched). No caller of the enqueue becomes Safe.

## Mechanism

A contract may state `resource_links_valid(p, MEMBER)` at a bound
position: the caller owes `p`'s `MEMBER` links valid on entry, and the
clause states nothing about the exit. The entry seeds an `Owed(MEMBER)`
record, admitted and forgotten as rule 91's `Ready`; a precondition-only
formal released through the carried destroyer reads `Mixed`. W4 attaches
the clause (after the destroyer clause's candidate failed) at each carried
formal bound but not released, checked by a fresh run; an exit-specific
summary publishes only when every class states the same entry obligations.
Each caller checks it before the call (its published object, its own
initialisation of the member, or its own formal carried with the member;
nothing else).

## Predicted moves

* system (both configurations): `flb_output_task_queue_enqueue` Unknown ->
  Safe. Leaf trade in `flb_output_task_singleplex_enqueue` (status
  unchanged). No other status move.
* opt-in current: none (its units are `lib/` and `flb_input_chunk.c`; none
  hands a formal to a member-naming destroyer).
* slice, corpus: none.
* No Unknown -> Safe except the one named; no Safe -> Unknown; no leak,
  double release or Unsafe added or removed; the null property unchanged
  except where the moved row's null reading follows its custody row.

```measure-prediction
route system
move flb_output_task_queue_enqueue Unknown -> Safe
delta Safe +2
delta Unknown -2
route slice
none
route corpus
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have; the card records the refusal.
