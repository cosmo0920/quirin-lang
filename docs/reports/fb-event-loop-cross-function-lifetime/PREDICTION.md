# FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: prediction

Frozen before any implementation code, at the card's pause point. The design
it predicts for is [README.md](README.md) §2–§3. The user has not chosen a
variant yet, so this file predicts each one.

## BEFORE (main `3f8f1a617`)

The binary is `2a91836f…`, built from the clean tree at `3f8f1a617`
([`scripts/build.sh`](scripts/build.sh)). Each route ran under the shared heavy
lock with `QCORE_NULL_PROPERTY` ([`scripts/measure.sh`](scripts/measure.sh)).
`run.json`'s `binary_sha256` equals the binary. Identities are in
[`artifacts/runs.json`](artifacts/runs.json).

| Route | Custody | Null (unconditional / conditional / Unknown / Unsafe) | Report |
| --- | --- | --- | --- |
| corpus `66910c10` | 939 / 9 / 3,886 | 1,114 / 1,435 / 2,354 / 18 | `f2eb590a…` |
| system `046accf6` (8 files) | 95 / 4 / 349 | 86 / 125 / 243 / 0 | `9d3fc333…` |
| tail-pause slice `90978ac6` | 52 / 1 / 347 / 4 Pending | 69 / 121 / 216 / 0 | `cfd3065a…` |

These equal round 31's figures.

[`scripts/lifetime_rows.py`](scripts/lifetime_rows.py) lists every row with a
lifetime leaf. Its outputs are
`artifacts/before-lifetime-rows-{corpus,system,slice}.json`. A lifetime leaf
is rule 12t's retention leaves, or a leaf naming a loop operation, a scheduler
timer or a collector operation.

| Route | Rows with a lifetime leaf | Of which every leaf is a lifetime leaf |
| --- | --- | --- |
| corpus | 43 | 12. Their holders: LZ4 streams and states ×10, `rd_kafka_queue_new`'s returned holder, and `flb_input_chunk`'s `global` ×2. **None is a loop.** |
| system | 41 | 6. These are `append_to_ring_buffer` (`global`) ×4 and `flb_sched_timer_coro_destroy` uncontracted ×2. **None is a loop operation.** |
| slice | 26 | 6. These are `flb_tail_fs_{inotify,stat}_{pause,resume}`, which are rule 10p's loop-owner refusals (C1), and `flb_input_thread_collectors_start` ×2 (`flb_input_collector_start` uncontracted). |

**The decisive reading.** On the system and slice routes, every function on
the registration and teardown chains already refuses earlier, at member
accesses and identities. The chains are R1–R6 of README §1.4:

* `collector_start`;
* `flb_input_collector_{start,pause,resume,delete,destroy}`;
* `flb_input_instance_destroy`, `flb_input_exit_all`;
* `flb_sched_timer_{cb_create,destroy,invalidate}`, `flb_sched_request_destroy`;
* `input_thread`, `input_thread_instance_{create,destroy}`.

Their earlier refusals are `indirect memory access is outside the resource
projection` and `unresolved resource identity`. These come before any
retention record is formed: `collector_start` has three access leaves and no
retention leaf. Every row on these chains also has other leaves. Only
`schedule_request_now` and `flb_sched_create` reach rule 12t at all
(`retention-undecided`).

## Per variant

All variants share the README §2.4 prerequisites. Their predicted effect
alone:

* **Custody:** no status move on any route.
* **Leaf trades:** `call to uncontracted function mk_event_loop_create` /
  `mk_event_loop_destroy` leave these rows:
  * system: `flb_engine_start` ×2 units;
  * slice: `input_thread_instance_create`, `input_thread_instance_destroy`,
    `flb_create`, `flb_destroy`, ×2 units each;
  * corpus: Monkey's `mk_lib_worker`, `mk_start`, `mk_server_loop_balancer`,
    `mk_server_worker_loop`, `mk_exit_all`, `mk_server_create`.

  Each such row keeps at least 6 other leaves. Monkey's own
  `mk_event_loop_create` / `_destroy` rows keep their `_mk_event_loop_*`
  leaves.
* **Poisoning:** predicted to move no row. No measured route row is known to
  hold an A-then-B removal; the implementation measures it.

### S1, derived per-function effect summaries, loop-scoped (recommended)

* **Corpus:** no row moves. No corpus retention row has a loop holder.
* **System:** no status move. `schedule_request_now` and `flb_sched_create`
  (×2 units each) lose their `retention-undecided` leaf, because the
  registration is forwarded. They keep 4 and 26 other leaves respectively. The
  releasers `flb_sched_timer_destroy`, `flb_sched_timer_invalidate` and
  `flb_sched_request_destroy` keep their access and identity leaves, and their
  release check is not reached.
* **Slice:** no status move and no leaf change beyond the prerequisites. No
  slice row reaches a retention record.
* **Null property:** identical on every route. Lifetime summaries add no null
  knowledge.
* **Safe → non-Safe:** none predicted. A releaser's fold demotion could act
  only on a `Safe` row with a forwarded `requires`, and none exists.

### S1, generic (option D5: forwarding retentions of every holder, not only loops)

This option is not recommended for the first card. Its prediction differs only
on the corpus. The 12 corpus rows and 4 system rows whose only leaf is
`retention-undecided` reach the forwarding:

* 9 LZ4 functions retaining `src` in a stream or state formal;
* `rd_kafka_queue_new`;
* `flb_input_chunk_ring_buffer_enqueue` / `_log_routing` (`global`).

Rows with a formal holder forward the record to their callers. Whether they
become `Safe` depends on the rule for roots. The cautious rule, that a root
forwarding a caller object's retention into a caller's holder refuses as
today, predicts **no move**. The rule "a forwarded retention is a derived
`resource_retained_by` postcondition" predicts **up to +10 corpus `Safe`**:
the 9 LZ4 rows and `rd_kafka_queue_new`. Each such row would be read by hand,
and each would rest on a derived retention that its callers must honour. The
`global` rows stay refused under both rules. A non-static object retained in
`global` has no ending in the run.

### S2, per-thread loop lifetimes (with S1, or alone)

No status move and no leaf change on any route beyond S1's. Every join on the
threaded teardown paths is unchecked or absent on a path:

* `flb_input_thread.c:602-607`;
* `flb_output_thread.c:640-646`.

Rule 10q therefore admits none of them. The rows on those chains already
refuse earlier (above). S2's change is to the reason a threaded arm refuses,
once those rows reach it. That is not measurable at these revisions.

### S3, declared lifetime protocols

S3 predicts as S1. It adds two validated declarations per held key at most
(`flb_input_collector.event`, `flb_sched_timer.event`) and no row move.

## Obligations

No obligation discharges under any variant.

* **OB-10 and OR-9.** Their rows (`flb_engine_start`,
  `flb_sched_event_handler`, `flb_sched_request_destroy`,
  `flb_sched_timer_cleanup`, `flb_sched_timer_invalidate`,
  `flb_task_retry_destroy`, `flb_output_instance_destroy`, the
  `in_storage_backlog` rows) keep their access, identity and list-membership
  blockers.
* **C1 and C2.** These are rule 10p's thread derivation. The four slice rows
  that refuse only for C1 keep that refusal under every variant: lifetime
  summaries do not derive threads.
* **C4.** Untouched.
* **C5.** Not discharged. Under S2 the reason becomes the unchecked or absent
  join, once reached.

## What would move rows

This is outside every variant and is recorded for the decision, not as this
card's scope. The chains' first blockers are the access gate and identity
resolution at member paths of formals and list-iterated records (`coll->…`,
`timer->…`, `thi->…`). A lifetime model becomes measurable on these routes
only after those blockers clear for the chain's rows. Until then, a correct
lifetime model shows up only as the leaf trades above.
