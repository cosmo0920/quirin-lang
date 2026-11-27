# RETENTION-UNMATCHED-REMOVAL-POISONS: prediction

Frozen before any implementation code. Base: local `main` `3f8f1a617` (round
31), moved to `a423fca5d` during the BEFORE runs by a records-only commit (the
lifetime design card): the measured code is identical.

## The defect, read again

`mk_event_del` (`mk_event.c:132-153`) returns early only when the event's
status says unregistered; otherwise it calls `_mk_event_del`
(`mk_event_epoll.c:156-186`), which issues `EPOLL_CTL_DEL` on **the loop it was
given**, ignores a failure of that call, unlinks the priority link and runs
`MK_EVENT_NEW(event)`, clearing the status. Through a loop that does not hold
the event, the `DEL` fails (`ENOENT`) and the status is cleared anyway. A
second removal through the loop that does hold it then returns at `:140` and
leaves the registration in that loop's interest list. `mk_event_loop_destroy`
(`mk_event.c:91-96`, `_mk_event_loop_destroy` `mk_event_epoll.c:90-95`) closes
the epoll descriptor, which drops every registration of that loop, and frees
the loop. The bodies are identical at 66910c10, 046accf6 and the 90978ac6
export ([`scripts/bodies.py`](scripts/bodies.py),
[`artifacts/bodies.json`](artifacts/bodies.json)).

Rule 12t's composition (`retained_by::replay`, route G's `apply`,
`QCore.RetentionPath.an_unmatched_removal_keeps_every_record` with
`a_removal_drops_exactly_its_record`) accepts "removal through A (matches
nothing), removal through B (matches), release" as ending the record. So it
admits a release while the B loop still holds the event.

## The fix, as it will be implemented

1. **Poisoning.** A removal drops the one record its actuals key exactly
   (same held object, same holder) when that record is not poisoned. Every
   other record whose held object the removal may name is **poisoned**: a
   removal whose object is undecided may name every record; a decided object
   may name a record of the same tracked identity, and any record whose
   object is a caller's object (two caller objects, or a caller's object and a
   tracked identity, are not known to be distinct). A poisoned record stays,
   no later removal ends it, and it ends only when its holder ends (the loop's
   destroy: a release of a tracked holder, or a declared reset). At the exit
   it refuses under the new reason `retention-removal-poisoned`; a release of
   its object while it is held refuses as today (`retention-released-while-held`).
   A removal with an undecided holder no longer drops nothing silently: it
   poisons what it may name.
2. **Destroy ends records.** `mk_event_loop_create` (`resource_return_owned(nullable)`)
   and `mk_event_loop_destroy` (`resource_released(loop, nonnull)`) declared in
   `qcore-provider-libevent` `monkey/event_loop.qv`, pinned at the three
   revisions. A release of a tracked loop already drops what it held
   (`holder_released`); it now drops poisoned records too. A destroy at a path
   (`mk_event_loop_destroy(config->evl)`) is a release of a member actual: the
   access gate keeps every custody-taking position at a member actual refused
   (W44-INDIRECT-CALL-ARGUMENT-ACCESS premise 1), so on route G such a call
   still refuses before any record is read. The model (Rust replay and Lean)
   gains a destroy of any holder, path included, which ends every record at
   it; rule 12t's "do not read a destroy at a path as a holder release" is
   amended to say so, and that route G reaches it today only for a tracked
   loop.
3. **Guarded equalities (D3).** Not in this card: deriving
   `config->sched->evl == config->evl` needs the run-wide store census of the
   pre-pass, which is more than a small amount of code. Registered as a
   follow-up; within this card no two distinct paths are equal, which is the
   fail-closed reading.

## Expected movement

**No status move on any route**, custody or null. BEFORE (binary
`private/rup/bin/before`, built from the git archive of `3f8f1a617`, under the
shared heavy lock, with `QCORE_NULL_PROPERTY`):

| Route | Custody | Null | Report |
| --- | --- | --- | --- |
| corpus 66910c10 | 939 / 9 / 3,886 | 1,114 / 1,435 / 2,354 / 18 | `f2eb590a…` |
| system | 95 / 4 / 349 | 86 / 125 / 243 | `6e4c41ea…` |
| slice | 52 / 1 / 347 / 4 Pending | 69 / 121 / 216 | `61c29c85…` |

Equal to round 31's figures. The census ([`scripts/census.py`](scripts/census.py),
`private/rup/census-before.json`) names every row whose leaves the card can
change.

**Rows whose leaves change** (the declarations): every row with a
`call to uncontracted function mk_event_loop_create / mk_event_loop_destroy`
leaf trades it for the call's decision under the declaration, or for an older
rule's refusal at the actual (the access gate at a member actual such as
`config->evl`, `thi->evl`, `ctx->event_loop`; a nullable actual at the non-null
destroy):

* corpus (Monkey's own server, 6 rows): `mk_lib_worker`, `mk_start`,
  `mk_server_loop_balancer`, `mk_server_worker_loop`, `mk_exit_all`,
  `mk_server_create`; each has 8 to 49 other leaves. `mk_event_loop_create`
  and `mk_event_loop_destroy` in `mk_core/mk_event.c` (their own definitions)
  keep their leaves, since those name the backend's `_mk_event_loop_*`.
* system (both configurations): `flb_engine_start` (105 leaves).
* slice (both configurations): `input_thread_instance_create` (30),
  `input_thread_instance_destroy` (12), `flb_create` (12), `flb_destroy` (7).

A store of the created loop into a member (`config->evl = evl`,
`thi->evl = …`) is read by the rules that already read
`flb_bucket_queue_create`'s result stored the same way; a loop lost on an
error path would be a new leak, which is read by hand and classified if it
appears (not expected: each creating body tests the result and stores or
destroys it).

**Rows whose leaves the poisoning changes**: none expected. Every rule-12t leaf
on the three routes (corpus 35 rows: LZ4, librdkafka and Fluent Bit ring
buffer retentions; system 10 rows; slice none) is a retention or an exit with
no removal of a registration in the same body; a poisoned record needs a
removal in the body that meets a record, and the census of the
FB-EVENT-LOOP-RETENTION card found every Fluent Bit removal in a different
function from its registration. `mk_event_timeout_disable` (corpus Safe) holds
no record (its one call removes a cast actual): it stays Safe, now with its
removal poisoning nothing because nothing is held.

Any Safe → non-Safe move would be a correction and is kept and named.
