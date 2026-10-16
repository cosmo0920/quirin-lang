# RETENTION-UNMATCHED-REMOVAL-POISONS

A soundness fix to gateway rule 12t, on the user's decision of 2026-10-17
(D2, "Yes, small fix card now"). The defect was found by the census of
[FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME](../fb-event-loop-cross-function-lifetime/README.md)
(§1.5, §2.4) in the composition landed by
[FB-EVENT-LOOP-RETENTION-AND-REGISTRATION](../fb-event-loop-retention/README.md).
The prediction, frozen before any implementation code, is
[PREDICTION.md](PREDICTION.md).

Base: local `main` `3f8f1a617`, fast-forwarded to `a423fca5d` (a records-only
landing of the lifetime design card) before any commit of this card.

## The defect (`RETENTION-UNMATCHED-REMOVAL-CLEARS-STATUS`)

`mk_event_del` (`mk_event.c:132-153`) returns early only when the event's
status says unregistered. Otherwise `_mk_event_del` (`mk_event_epoll.c:156-186`)
issues `EPOLL_CTL_DEL` on the loop it was given, ignores a failure, unlinks the
priority link and clears the status (`MK_EVENT_NEW`). Through a loop that does
not hold the event, the `DEL` fails and the status is cleared anyway. A later
removal through the loop that does hold it returns at `:140`, so that loop
keeps the registration. The bodies are identical at 66910c10, 046accf6 and the
90978ac6 export ([`scripts/bodies.py`](scripts/bodies.py),
[`artifacts/bodies.json`](artifacts/bodies.json)).

Rule 12t's composition accepted "a removal through path A that matches no
record, then a removal through path B that matches it, then the release" as
ending the record (`QCore.RetentionPath.an_unmatched_removal_keeps_every_record`
with `a_removal_drops_exactly_its_record`). At run time B can still hold the
event at the release. The defect is recorded here rather than registered:
this card fixes it, so it would be registered and retired in the same change.
No row on any route had this sequence before the fix (the BEFORE census below),
so it was a gap in the rule, not in a published verdict.

## What changed

* **Poisoning** (`crates/qcore-c-gateway/src/retained_by.rs`: `RemovalKey`,
  `removal_key`, `may_name`, `remove`, `end_holder`, `Record`, `POISONED`,
  `poisoned_exit_description`, `Event::{Remove(RemovalKey), Destroy}`). A
  removal drops the exactly matching record only when that record is not
  poisoned. It then poisons every record still held whose object it may name.
  An undecided object names every record. A decided one names records of the
  same tracked identity and any record whose object is a caller's object:
  two caller objects, a union member or a caller object beside a tracked one
  are not known to be distinct. Only two distinct tracked identities are. A
  removal whose holder (or object) is not a spelling a record keys on used to
  drop nothing silently. Now it poisons what it may name: an undecided side
  names more, never less.
* **A poisoned record ends only when its holder ends** (`end_holder`): the
  loop's destroy, a released tracked holder, a declared reset or a registry's
  drop. If it is still held at an exit, it refuses as
  `retention-removal-poisoned`. A release of its object while it is held
  refuses as before (`retention-released-while-held`, never `Unsafe`).
* **Route G** (`legacy_retained_by.rs`, `legacy_callback_route.rs`): the
  removal and reset hooks, `holder_released` and the new `holder_ended` keep
  `AbstractState::poisoned` in step with `retained`. That field is a shared
  item added additively to `legacy_transfer.rs`, joined by union, and empty
  in every function that calls no removal. The exit report names a poisoned
  record's reason.
* **Destroy ends records.** `mk_event_loop_create`
  (`resource_return_owned(nullable)`) and `mk_event_loop_destroy`
  (`resource_released(loop, nonnull)`) are declared in `qcore-provider-libevent`
  `monkey/event_loop.qv`. Each body was read at the three revisions: create
  frees what it made and returns NULL on any failure; destroy dereferences
  `loop->data`, closes the epoll descriptor (dropping every registration
  without touching the events' statuses) and frees the loop. Both are pinned at
  all three revisions ([`scripts/pin.sh`](scripts/pin.sh)). The comment on
  `mk_event_del` now states the wrong-loop reset.
  A tracked loop's destroy is a released holder, so it ends every record the
  loop holds, poisoned or not. **A destroy at a path is modelled, not reached.**
  The model's `Event::Destroy` / Lean `Event.destroy` ends the records at any
  holder. On route G, `mk_event_loop_destroy(config->evl)` releases a member
  actual, a custody-taking position the access gate keeps refused
  (W44-INDIRECT-CALL-ARGUMENT-ACCESS premise 1), so the call refuses before
  any record is read (`a_loop_destroy_at_a_path_stays_refused`). Registered:
  `RETENTION-PATH-LOOP-DESTROY-ADMISSION`.
* **Guarded equalities (D3) are not in this card.** Deriving
  `config->sched->evl == config->evl` from single stores needs the run-wide
  store census of the pre-pass, which is more than a small amount of code.
  Two distinct paths are never one loop here, the fail-closed reading.
  Registered: `RETENTION-DERIVED-LOOP-PATH-EQUALITIES`.

## Tests

`crates/qcore-c-gateway/tests/event_loop_retention.rs`, over the synthetic
`ev_*` fixtures (trusted test assumptions):

| Test | Shape | Outcome |
| --- | --- | --- |
| `an_unmatched_removal_poisons_the_record` | `ev_add(l,o); ev_del(m,o); ev_del(l,o); ev_free(o)` | refused (`retention-released-while-held`), not `Unsafe` |
| (same) | `ev_add(l,o); ev_del(l,o); ev_free(o)` | Safe |
| (same) | `ev_add(l,o); ev_loop_destroy(l); ev_free(o)` | Safe |
| (same) | `ev_add(l,o); ev_del(m,o); ev_loop_destroy(l); ev_free(o)` | Safe |
| (same) | poisoned, removed again, `l` destroyed, then `ev_free(o)` | Safe |
| `a_poisoned_path_record_names_its_reason_at_the_exit` | `coll->evl` registration removed through `config->evl`, then `coll->evl` | refused (`retention-removal-poisoned`) |
| (same) | removed through its own path | Safe |
| (same) | `&coll->timer` removed through `config->evl` first | refused (`retention-removal-poisoned`) |
| `a_loop_destroy_at_a_path_stays_refused` | `ev_add(coll->evl, …); ev_loop_destroy(coll->evl)` | refused, not `Unsafe` |

The landed `a_path_holder_still_holding_at_the_exit_is_refused` now expects
`elsewhere` and `one_of_two` to refuse as `retention-removal-poisoned`, still
not Safe. In `one_of_two`, removing `&coll->event` poisons the record of
`&coll->timer` (two caller objects are not known to be distinct), so the
remaining record refuses with the new reason. The shared vectors grow by nine
rows (17 trace rows, bound in Rust by `path_vectors_bind_the_table_and_the_replay`
and `the_shared_vectors_hold`, in Lean by `path_trace_vectors_hold`).
`compiler/tests/qcore_contract_corpus.rs` pins the two new declarations' clause
counts (3 each). The targeted run is [`scripts/targeted-tests.sh`](scripts/targeted-tests.sh)
(`private/rup/t2.log`, exit 0). The first run (`t1`, exit 101) failed only on
that landed test's old reason key.

## Lean: `QCore.RetentionPath`, corrected

Records carry a `poisoned` flag. `step`'s removal is now
`(rs.filter (!exact k ·)).map (poisonIf k.1)`, and `Event.destroy b` drops
every record at `b`.

* **Corrected, not extended.** `an_unmatched_removal_keeps_every_record` (an
  unmatched removal is the identity) is false in the new model and is replaced
  by `an_unmatched_removal_poisons_what_it_may_name`.
  `a_removal_drops_exactly_its_record` becomes
  `a_removal_drops_exactly_its_unpoisoned_record`.
  `a_path_record_ends_only_by_its_own_removal` is refuted by a destroy and
  becomes `a_path_record_ends_only_by_its_own_removal_or_its_destroy`.
  `a_path_record_without_its_removal_is_never_safe` becomes
  `a_path_record_without_its_removal_or_destroy_is_never_safe`.
* **New.**
  * `a_poisoned_record_ends_only_by_its_loop_destroy`: through any events none
    of which ends its holder, the poisoned record is still there.
  * `a_poisoned_record_without_its_destroy_is_never_safe`.
  * `the_old_sequence_no_longer_decides`: for any caller object and two
    distinct paths, retain at P, remove at Q, remove at P, then any events
    without P's destroy, `replay` is `false`.
  * `a_matched_removal_ends_an_unpoisoned_record`.
  * `a_destroy_ends_every_record_at_its_loop`.
  * The helper lemmas `step_keeps_a_poisoned_record`, `step_keeps_a_record`,
    `poisonIf_object` and `poisonIf_of_poisoned`.
* **Kept:** `an_unresolved_path_never_decides`,
  `a_safe_path_never_releases_a_held_identity`,
  `a_safe_path_leaves_no_outliving_holder_holding`, `run_append`,
  `release_refused_iff`, the decision and trace vectors.

[`proof/AuditRetentionPath.lean`](../../../proof/AuditRetentionPath.lean)
prints the axioms of all 20 theorems. The module count is unchanged: the card
amends the module and adds none.

## Measurements

BEFORE: binary `dac1482d…`, built from the git archive of `3f8f1a617`. AFTER:
binary `f743353a…`, built from the archive of the tip `6710d60a8` (later
commits change records only). Each run was taken under the shared heavy lock
with `QCORE_NULL_PROPERTY` ([`scripts/export.sh`](scripts/export.sh),
[`scripts/build.sh`](scripts/build.sh), [`scripts/measure.sh`](scripts/measure.sh)).
Each corpus `run.json` `binary_sha256` equals its binary. Identities are in
[`artifacts/runs.json`](artifacts/runs.json), row-level moves in
`artifacts/compare-{corpus,system,slice}.json` ([`scripts/compare.py`](scripts/compare.py),
[`scripts/show_moves.py`](scripts/show_moves.py)).

| Route | Custody before → after | Null before → after | Report |
| --- | --- | --- | --- |
| corpus `66910c10` | 939 / 9 / 3,886 → identical | 1,114 / 1,435 / 2,354 / 18 → identical | `f2eb590a…` → `33e139c0…` |
| system (8 files) | 95 / 4 / 349 → identical | 86 / 125 / 243 → identical | `6e4c41ea…` → `b1203de8…` |
| tail-pause slice | 52 / 1 / 347 / 4 Pending → identical | 69 / 121 / 216 → identical | `61c29c85…` → `c6b4b887…` |

BEFORE equals round 31's figures. **No status move and no null move on any
route; no correction.** No Safe row moves, so there is no Safe → non-Safe
change to name.

**Leaf trades, exactly the predicted rows.** Each trades its
`call to uncontracted function mk_event_loop_create / mk_event_loop_destroy`
leaf, and every row keeps many other leaves:

* **Corpus (6 rows).** `mk_lib_worker`, `mk_start` and `mk_exit_all` get the
  access gate's refusal at the member actual (`server->lib_evl`,
  `server->lib_evl_start`). `mk_server_loop_balancer` destroys its local loop
  (`mk_server.c:420`), and that call is now decided. It also gets
  `unmodeled expression may mutate a formal resource identity` in place of
  `unmodeled increment or decrement expression` at the event iteration over
  the now-tracked loop. `mk_server_worker_loop` gets the same two-for-two
  trade, plus `unresolved resource identity evl` at its destroy.
  `mk_server_create` gets six `unresolved resource identity server` leaves
  at the creates and destroys of `server->lib_evl` / `server->lib_evl_start`.
* **System (2 rows).** `flb_engine_start` (both configurations) gets
  `unresolved resource identity evl` at line 19898.
* **Slice (8 rows).** `input_thread_instance_create` gets
  `unresolved resource identity thi`. `input_thread_instance_destroy` and
  `flb_destroy` get the access gate at `thi->evl` / `ctx->event_loop`.
  `flb_create` loses its leaf with no replacement and stays Unknown on its
  other 11.

Both configurations are counted in each case.

**The poisoning moves nothing measured, as predicted.** No report on any
route contains `retention-removal-poisoned`. The rule-12t leaves (corpus 35
rows: LZ4, librdkafka and the Fluent Bit ring buffer; system 10; slice none)
are identical before and after ([`scripts/census.py`](scripts/census.py),
[`artifacts/census-before.json`](artifacts/census-before.json),
[`artifacts/census-after.json`](artifacts/census-after.json)). None of them
has a registration and a removal in one body.
`mk_core/mk_event.c::mk_event_timeout_disable` stays **Safe**. Its leaves are
empty and `contract_dependencies` is `mk_event_del`. Its removal names a cast
actual (undecided object), which poisons nothing because nothing is held.

## Ledger

`named_followups` 792 → 794 ([`scripts/update_ledger.py`](scripts/update_ledger.py),
idempotent):

* registered `RETENTION-DERIVED-LOOP-PATH-EQUALITIES` (D3);
* registered `RETENTION-PATH-LOOP-DESTROY-ADMISSION` (11 loop destroys, all at
  member paths, none in a body that registers at the same path);
* `RETENTION-UNMATCHED-REMOVAL-CLEARS-STATUS` recorded above and fixed by this
  card, not registered.

## Rule ids

Rule 12t is amended in place: the correction paragraph, the "Do not" list and
the Lean list. Contract specification C04.14.3 is amended in place: a
"Removal poisoning" paragraph and its Lean sentence. No new rule id; `32`
stays free. No grammar change: the declarations use existing clauses.

## Gate

See [`gate.json`](gate.json) and [`gate/`](gate/). The standing gate and the
provider gate ran on the tip under one exclusive flock of the shared heavy
lock ([`scripts/gate-whole.sh`](scripts/gate-whole.sh),
[`scripts/gate.sh`](scripts/gate.sh), [`scripts/finish-gate.sh`](scripts/finish-gate.sh),
[`scripts/write_gate_json.py`](scripts/write_gate_json.py)). Logs are in
`private/rup/gate-g1` of this worktree.
