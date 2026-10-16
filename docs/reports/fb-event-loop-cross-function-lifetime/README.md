# FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: census and design (paused for the user)

Design card with a mandatory pause. This report holds the census and the
proposal. [PREDICTION.md](PREDICTION.md) holds the BEFORE measurement and the
movement expected for each variant. **No product code was written.** The card
waits for the user's choice.

Base: local `main` `3f8f1a617` (round 31). Authority: the ledger entries
`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` (T1/T2) and
`W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS`; gateway rules 12n (closure and
pre-pass), 12t (holder paths, `#holds` removals), 10p (loop owner,
thread-entry census) and 10q (join before release); the
[FB-EVENT-LOOP-RETENTION](../fb-event-loop-retention/README.md) report and its
census; the Fluent Bit obligations OB-10 and OR-9
([lifetime inventory](../w45-fb-system-selection/lifetime-inventory.json)) and
C1, C2, C4, C5 ([tail-pause bindings](../w47-tail-pause-bindings/README.md) §5).

## User direction (2026-10-16, verbatim)

Asked to choose between T1 (a run-wide closure per loop member) and T2
(forwarding along call edges), the user answered:

> "Like rule 12'n but we need to provide the candidates of loop members. And we need to provide summary model. Not sure how we should handle on it."

The integrator read this as a hybrid of three parts. First, a run-wide
candidate set per loop member, computed the way rule 12n computes callback
sets. Second, a per-function summary model. Third, cross-function ordering
checked against those summaries.

## 1. Census

### 1.1 Method

The census script is [`scripts/loop_member_census.py`](scripts/loop_member_census.py).
Its output is [`artifacts/loop-member-census.json`](artifacts/loop-member-census.json).
It is a read-only lexical reading of three prepared trees: the corpus inputs'
`66910c10` source, the system route's `046accf6` export and the tail-pause
slice's `90978ac6` export. It never touches a Fluent Bit or Monkey checkout. It
reads every `.c` and `.h` file under `src/`, `plugins/` and `include/`: 622, 625
and 626 `.c` files. It reuses the previous card's function locator and blanking,
which read conditional groups over their first arm.

* **Loop member.** A record field typed `struct mk_event_loop *` (or a
  `void *` field named `evl`/`event_loop`). It is keyed like rule 12n's
  callback members by the innermost named record (`TAG.FIELD`), so
  `coll->instance->config->evl` is `flb_config.evl`. The engine's
  thread-local slot (`flb_engine_evl_set` / `_get`) is the key
  `tls.flb_engine_evl`.
* **Candidate set.** The set of `mk_event_loop_create` allocation sites that
  can reach the member. It is the least fixed point over the member's stores,
  where a store's right-hand side resolves to one of:
  * an allocation;
  * another member;
  * a formal, expanded over the actuals of every direct call;
  * a call's `return` operands;
  * `NULL`;
  * `open`: an address-taken function, no call read, an untyped root, or an
    unknown call.

  A set is **closed** only when no `open` origin reaches it.
* **Operations.** Each operation is attributed to the members and allocation
  sites its loop actual can name. Registrations are `mk_event_add`,
  `_timeout_create` and `_channel_create`. Removals are `mk_event_del`,
  `_timeout_destroy`, `_timeout_disable` and `_channel_destroy`. Waits are
  `mk_event_wait`, `_wait_2` and the two foreach macros. Destroys are
  `mk_event_loop_destroy`. The held object of a registration or removal is
  attributed to the `TAG.FIELD` interior it names. A local copy is followed
  when its writes name one interior.

The three trees give the same structure; only line offsets differ. Between
`046accf6` and `90978ac6` the only differing files are nine `in_tail` files.
Lines below are `046accf6` (= `90978ac6`) unless marked. A match is a site,
not a finding.

| Tree | Members | Closed | Allocation sites | Registrations | Removals | Waits | Destroys |
| --- | --- | --- | --- | --- | --- | --- | --- |
| corpus `66910c10` | 19 | 17 | 7 | 51 | 37 | 13 | 11 |
| system `046accf6` | 19 | 17 | 7 | 51 | 37 | 13 | 11 |
| slice `90978ac6` | 19 | 17 | 7 | 51 | 37 | 13 | 11 |

The previous card's census counted 37 registrations (`mk_event_add` and
`_timeout_create`); this one adds the 14 `_channel_create` sites. The previous
card's five "unresolved" `flb_engine_evl_get()` sites now resolve to
`tls.flb_engine_evl`. Its two "several writes" locals now resolve to the union
of their writes.

### 1.2 Candidate set per loop member

Allocation sites (`046accf6`; the corpus offsets are `flb_engine.c:1129` and
`flb_output_thread.c:505`):

* **E**, the engine loop: `flb_engine.c:1145`, in `flb_engine_start`.
* **I**, an input thread's loop: `flb_input_thread.c:255`, in
  `input_thread_instance_create`.
* **O**, an output worker's loop: `flb_output_thread.c:512`, in
  `flb_output_thread_pool_create`.
* **W**, a downstream worker's loop: `flb_downstream_worker.c:241`.
* **L**, the library context's loop: `flb_lib.c:190`, in `flb_create`.
* **G**, the logger's loop: `flb_log.c:1067`.
* **K**, the Kubernetes filter's AWS loop: `kubernetes.c:88`.

| Member | Stores (site: right-hand side) | Candidates | Closed | Registrations / removals / destroys |
| --- | --- | --- | --- | --- |
| `flb_config.evl` | `flb_engine.c:1150` `config->evl = evl` (alloc) | E | yes | 11 / 10 / 1 (`flb_config_exit`, `flb_config.c:742`) |
| `flb_input_thread_instance.evl` | `flb_input_thread.c:255` (alloc) | I | yes | 3 / 1 / 1 (`input_thread_instance_destroy`, `:206`) |
| `flb_input_collector.evl` | `flb_input.c:2251` `coll->evl = thi->evl`; `:2258` `coll->evl = config->evl` | E, I | yes | 4 / 2 / 0 |
| `flb_sched.evl` | `flb_scheduler.c:753` `sched->evl = evl` (formal of `flb_sched_create`: `config->evl` at `flb_engine.c:1288`, `thi->evl` at `flb_input_thread.c:340`, `th_ins->evl` at `flb_output_thread.c:213`) | E, I, O | yes | 2 / 3 / 0 |
| `flb_out_thread_instance.evl` | `flb_output_thread.c:525` (alloc via local) | O | yes | 3 / 4 / 3 |
| `tls.flb_engine_evl` | `flb_engine.c:1202`, `flb_input_thread.c:337`, `flb_output_thread.c:207`, `flb_downstream_worker.c:259`, `kubernetes.c:94`; `NULL` resets | E, I, O, W, K | yes | 5 / 0 / 0 |
| `flb_connection.evl` | `flb_connection.c:22` (formal of `flb_connection_setup`) | E, I, O, W, K | yes | 11 / 7 / 0 |
| `flb_dns_lookup_context.event_loop` | `flb_network.c:1123` (formal) | E, I, O, W, K | yes | 1 / 2 / 0 |
| `flb_ring_buffer.event_loop` (`void *`) | `flb_ring_buffer.c:159` (formal: `config->evl`); `:176` `NULL` | E | yes | 0 / 1 / 0 (+1 by formal) |
| `flb_lib_ctx.event_loop` | `flb_lib.c:190` (alloc) | L | yes | 0 / 1 / 1 (`flb_destroy`, `flb_lib.c:242`) |
| `flb_config.ch_evl` | `flb_lib.c:196` `= ctx->event_loop` | L | yes | 1 / 0 / 0 |
| `flb_log.evl` | `flb_log.c:1080` (alloc via local) | G | yes | 1 / 0 / 3 |
| `flb_downstream_worker.event_loop` | `flb_downstream_worker.c:241` (alloc); `:346` `NULL` | W | yes | 1 / 1 / 1 |
| `flb_in_tcp_config.event_loop`, `flb_in_udp_config.event_loop`, `flb_in_fw_config.event_loop` | `tcp.c:114`, `udp.c:115`, `fw.c:339` (formal: the worker's loop) | W | yes | 0 / 1 / 0 each (+1 registration by formal each) |
| `flb_kube.aws_pod_service_event_loop` | `kubernetes.c:88` (alloc); `:922` `NULL` | K | yes | 0 / 0 / 1 |
| `flb_http_server.event_loop`, `flb_http_server_options.event_loop` | `flb_http_server.c:183`; `prom_http.c:124`, `vivo_http.c:340`, `flb_hs.c:512` (`NULL`), `flb_http_server.c:686/835/900` | E, I, W | **open**: `flb_http_server_init` has no direct call in the tree | 1 / 2 / 0 |

Inverse view, from each allocation site to the members that can hold it:

* E: `flb_config.evl`, `flb_input_collector.evl`, `flb_sched.evl`,
  `flb_ring_buffer.event_loop`, `tls`, `flb_connection.evl`, the DNS and HTTP
  members.
* I: `flb_input_thread_instance.evl`, `flb_input_collector.evl`,
  `flb_sched.evl`, `tls`, `flb_connection.evl`, DNS, HTTP.
* O: `flb_out_thread_instance.evl`, `flb_sched.evl`, `tls`,
  `flb_connection.evl`, DNS.

The sets are over **allocation sites**, not run-time loops. Site I creates
one loop per threaded input instance, so two `thi->evl` of different instances
share the candidate I. A candidate set can prove that two loops differ: the
sets are disjoint. It cannot prove that two loops are the same. That needs the
member equalities of §2.3.

### 1.3 Held objects: where each is registered and where it is removed

From `held_keys` in the artifact. Every removal of a held key names the same
member as its registration, with two exceptions.

| Held key | Registered at (functions) | Removed at (functions) |
| --- | --- | --- |
| `flb_input_collector.event` | `flb_input_collector.evl` (`collector_start`, `flb_input_collector_resume`) | `flb_input_collector.evl` (`flb_input_collector_pause`); **`flb_config.evl`** (`flb_input_collector_destroy`) |
| `flb_sched_timer.event` | `flb_sched.evl` (`flb_sched_timer_cb_create`); the formal `evl` (`flb_sched_create`, frame timer); **`flb_config.evl`** (`schedule_request_now`) | `flb_sched.evl` (`flb_sched_timer_cb_disable`, `flb_sched_destroy`) |
| `flb_connection.event` | `flb_connection.evl` (10 functions); `tls` (`*_conn_add` ×5) | `flb_connection.evl` (6 functions) |
| `flb_in_{tcp,udp,fw}_config.listener_event` | the formal `event_loop` of `in_*_start_listener` | `X.event_loop` (`*_config_destroy`), stored from that same formal |
| `flb_config.event_thread_init`, `.notification_event`, `.event_flush`; `flb_me.event`; `flb_sp_task_window.event`; `flb_sched.event`; `flb_dns_lookup_context.response_event`; `flb_out_thread_instance.notification_event`; `flb_downstream_worker.control_event`; `flb_http_server.listener_event` | the same member | the same member |
| `flb_input_thread_instance.event`, `.event_local`; `flb_log.event`; `flb_config.ch_event` | their loop's member | never removed: their loop's destroy ends them |

The `flb_sched_timer.event` exception is only apparent. A request timer's
`config->evl` equals its `timer->sched->evl`: `config->sched` has one store
(`flb_engine.c:1293`), a scheduler created by
`flb_sched_create(config, config->evl)` (`:1288`). `sched->evl` has one store
per creation (`flb_scheduler.c:753`), and `timer->sched` has one store
(`:883`). The `flb_input_collector.event` exception is real; §1.5 reads it.

### 1.4 Teardown orderings, read by hand

Each line is one thread's order. "Joins" marks a cross-thread edge.

**R1. A threaded input instance.** Engine thread, `flb_engine_shutdown`
(`flb_engine.c:1665`):

* `flb_input_pause_all` runs, then outputs, then `flb_input_exit_all`
  (`:1686`). For each instance in reverse list order (`flb_input.c:2152`):
  1. `flb_input_instance_exit` (`:2159`). Its threaded arm (`:2132-2135`)
     calls `flb_input_thread_instance_exit` and returns, ignoring the result.
     That function (`flb_input_thread.c:580-611`):
     * runs `cb_pre_exit`;
     * writes `FLB_INPUT_THREAD_EXIT` to `ch_parent_events[1]` (`:601`). On
       `ret <= 0` it **returns -1 without joining** (`:602-605`);
     * otherwise runs `pthread_join(tid, NULL)` (`:607`). The join result is
       not checked.
  2. `flb_input_instance_destroy(ins)` (`flb_input.c:2160`). Its collector
     loop (`:1328-1332`) unlinks each collector and calls
     `flb_input_collector_destroy`, which removes `&coll->event` from
     `coll->instance->config->evl` (`:3146`/`:3150`) and runs
     `flb_free(coll)` (`:3153`). Then `flb_free(ins)` (`:1353`).

  Input thread, `input_thread` (`flb_input_thread.c:318`):

  * It sets `tls` to `thi->evl` (`:337`).
  * It registers its scheduler's frame timer and channel on `thi->evl`
    (`flb_sched_create`, `:340`) and its permanent timer (`:351`).
  * After the start signal it starts the collectors on `coll->evl = thi->evl`
    (`:405`, `collector_start`).
  * On `FLB_INPUT_THREAD_EXIT` (`:477`) it finishes the round, runs `cb_exit`
    and breaks (`:511`).
  * Teardown follows: `flb_bucket_queue_destroy` (`:516`), then
    `flb_sched_destroy(sched)` (`:517`, which removes the scheduler's timers
    and channel from `thi->evl`), then `input_thread_instance_destroy(thi)`
    (`:518`). That function destroys the notification channel, then
    `mk_event_loop_destroy(thi->evl)` (`:206`), closes the pipes, and frees
    `thi` (`:226`).
  * The collectors' registrations and `ins->event` (the instance channel,
    registered on `flb_input_event_loop_get(ins)` = `thi->evl`,
    `flb_input.c:1380`) are never removed. **The loop's destroy ends them.**
  * The early returns (`:343`, `:356`, `:374`, `:384`, `:398`) all precede
    `flb_input_thread_collectors_start`. They leave `thi->evl` and the
    scheduler undestroyed, with no collector registered.

  **Order.** On the joined path:

  1. the thread's loop destroy;
  2. the join;
  3. the engine's `flb_free(coll)` and `flb_free(ins)`.

  The removal through `config->evl` runs between 2 and 3 and acts on a loop
  the event is not registered in (§1.5).

**R2. A non-threaded input instance.** The same engine-thread sequence.
`collector_create` stored `coll->evl = config->evl` (`flb_input.c:2258`),
using its `config` formal. The removal in `flb_input_collector_destroy` names
`coll->instance->config->evl`. The two are equal when the setter's `config` is
the instance's `config`: every collector setter receives the `config` its
plugin's `cb_init` received, and `cb_init` is called as
`p->cb_init(ins, ins->config, …)` (`flb_input_thread.c:369`; the engine's init
path likewise). So the removal ends the registration on the loop that holds
it, and then `flb_free(coll)` runs. The engine loop is still live, because it
is destroyed later in R3. The same holds for `flb_input_collector_delete`
(`:3197-3216`): `flb_input_collector_pause` removes from `coll->evl` while
`running`, then `flb_free(coll)`. Its safety when `running == FALSE` rests on
the scalar member invariant "registered ⇒ `running`": `collector_start` and
`_resume` set `running` only after a successful registration (`:2374`,
`:3271`), and `_pause` clears it after the removal (`:3192`).

**R3. The engine loop.** The engine thread ends `flb_engine_start` with the
following steps:

* `flb_engine_shutdown` (R1/R2; then the engine channels on `config->evl`,
  `:1710-1722`).
* `mk_event_timeout_destroy(config->evl, &config->event_shutdown)`
  (`:1564`), then return.

The main thread then continues:

* `flb_stop` joins the engine thread, and this join is checked
  (`flb_lib.c:1077-1080`).
* `flb_destroy` (`flb_lib.c:229`): `mk_event_del` of the library channel on
  `ctx->event_loop`, then `mk_event_loop_destroy(ctx->event_loop)` (`:242`),
  then `flb_config_exit` (`:249`).
* `flb_config_exit` (`flb_config.c:580`):
  * `flb_log_destroy`, which joins the log worker and then destroys `log->evl`
    (`flb_log.c:1345-1350`);
  * `event_flush` removed from `config->evl` (`:681-683`);
  * `flb_sched_destroy(config->sched)` (`:687`), which removes every request
    and timer from `sched->evl` = `config->evl`;
  * `mk_event_loop_destroy(config->evl)` (`:742`).

**Order.** Every registration on E that is removed at all is removed before
E's destroy (`:742`), and the destroy follows the checked join of E's owner
thread.

**R4. An output worker.** Worker thread, end of `output_thread`:

* `cb_worker_exit`, then the local channel;
* the upstream destroy lists;
* `flb_sched_destroy(sched)` (`flb_output_thread.c:404`);
* the parent and notification channels;
* `mk_event_loop_destroy(th_ins->evl)` (`:432`).

Engine thread, `flb_output_thread_pool_destroy` (`:618-652`):

* writes the stop message (`:639`). On `n < 0` it runs
  **`flb_free(th_ins)` without a join** (`:643`);
* otherwise `pthread_join` (unchecked, `:646`), then `flb_free(th_ins)`
  (`:647`).

**R5. A downstream worker** (`in_tcp`, `in_udp`, `in_forward`, HTTP). All of
it runs on the worker thread (`flb_downstream_worker.c:312-347`):

1. `runtime->cb_exit`, for example `in_tcp_worker_exit` → `tcp_config_destroy`,
   which removes the listener from `ctx->event_loop` (`tcp_config.c:173`).
   That is the worker's loop: `in_tcp_start_listener` stored its formal
   `event_loop` = `flb_downstream_worker_event_loop_get(worker)`.
2. The control channel.
3. `mk_event_loop_destroy(worker->event_loop)`.

Everything is ordered on one thread. The edge runs through a callback member
(`runtime->cb_exit`).

**R6. Scheduler request timers (OR-9).** On the engine thread:

1. `schedule_request_now` registers `&timer->event` on `config->evl`
   (`flb_scheduler.c:133`).
2. `flb_task_retry_destroy` (`flb_task.c:274`) runs
   `flb_sched_request_invalidate` → `flb_sched_request_destroy` →
   `flb_sched_timer_invalidate`. That calls `flb_sched_timer_cb_disable`,
   which removes the event from `timer->sched->evl` when `timer_fd != -1`
   (`:723`), and moves the timer to `timers_drop` (`:901`).
3. The timer is freed only by `flb_sched_timer_cleanup` (`:916-934`), after
   the engine round.

The frame timer has `timer_fd == -1`: `flb_sched_create` stores the fd in
`sched->frame_fd`, not in `timer->timer_fd` (`:783-791`). It is removed
explicitly in `flb_sched_destroy`'s timer loop before
`flb_sched_timer_destroy`.

### 1.5 The collector-destroy reading: confirmed as text; the normal order is benign

**Confirmed at all three revisions.** `flb_input_collector_destroy`
(`66910c10` `flb_input.c:3120-3136`; `046accf6` and `90978ac6` `:3140-3156`)
removes `&coll->event` from `coll->instance->config->evl`, never from
`coll->evl`. A threaded instance's collectors were registered on
`coll->evl = thi->evl` (`:2251`, `collector_start` `:2351`/`:2362`). The
candidate sets agree: the removal's member `flb_config.evl` = {E}, and the
registration's member `flb_input_collector.evl` = {E, I}. The census flags
`flb_input_collector.event` as the one held key whose terminal removal names
another member (§1.3). (`flb_input_collector_pause` removes from `coll->evl`.)

What the removal does, read from Monkey's epoll backend (`mk_event.c:132-153`,
`mk_event_epoll.c:156-186` and `:341-353`, identical at the three revisions):

* **On the joined path (R1: loop destroyed, then join, then destroy), it is
  benign.** The event's status still says registered: the thread loop's
  destroy does not reset event statuses. So `_mk_event_del` calls `epoll_ctl(DEL)`
  on the **engine's** epoll fd and fails, an error the code ignores. It unlinks
  the priority link and resets the status. A time collector then has its timerfd
  closed by `mk_event_timeout_destroy`; that is the collector's own descriptor,
  still open. The registration it should end was already ended by
  `mk_event_loop_destroy(thi->evl)`, which closed the epoll fd before the join
  returned. Had the removal named `coll->evl`, it would have read the freed loop
  (`loop->data`). So naming `config->evl` is what keeps this path free of a
  freed-loop read.
* **On the unjoined path (`flb_input.c:2159` ignores the -1 of
  `flb_input_thread.c:602-605`), the removal cannot end the registration.** If
  the exit message is not written, the input thread keeps running on `thi->evl`.
  `flb_input_collector_destroy` then resets the event's status through the
  wrong loop and frees `coll` while `thi->evl`'s interest list still holds
  `&coll->event`; `flb_free(ins)` follows. A later dispatch on the thread would
  read freed memory. This is **a reading on an error path, not a finding**: it
  is not validated, no reproduction or trace exists, and the write fails only
  if `write` on a blocking pipe returns `<= 0`.
* **An fd-number corner, not excluded.** For an FD collector, the plugin's
  `cb_exit` on the thread closed the descriptor. A `DEL` of that number on the
  engine's epoll fd would remove another registration only if the number were
  reused and registered on E between the close and the removal. The census
  cannot exclude that. It is a reading, not a finding.

The modelling consequence matters for every variant. A removal through a loop
that does not hold the event is **not** a no-op on the event: it clears the
event's registered status. A later removal through the right loop then
returns early (`mk_event.c:140`) and leaves that loop holding the event. Rule
12t's composition (`QCore.RetentionPath.an_unmatched_removal_keeps_every_record`,
then `a_removal_drops_exactly_its_record`) would accept this sequence: a
removal at path A that matches no record, then a removal at path B that matches
one, then a release. In reality the second removal does nothing when A and B
are different loops. Section 2.4 makes an unmatched removal poison the record,
and §6 registers the correction.

## 2. What any summary model needs (common to every variant)

### 2.1 What is checked

The analysis works on a small abstract machine:

* **State.** `Reg`, a set of records `(o, ℓ)` (interior `o` registered in the
  run-time loop `ℓ`); `Live`, the live loops; `Freed`, the freed objects.
* **Steps.**
  * `reg(o, ℓ)` and `rem(o, ℓ)` require `ℓ ∈ Live`;
  * `destroy(ℓ)` drops every record at `ℓ`;
  * `free(x)` requires that no record `(o, ℓ)` with `o` an interior of `x`
    remains;
  * a dispatch from `ℓ` touches only objects in `Reg` at `ℓ`.
* **Obligations.**
  * **P1 (release):** never free an object a live loop holds.
  * **P2 (loop use):** never operate on a destroyed loop.

  A registration that reaches its loop's destroy unremoved is not a
  violation. In R1 that is how every threaded collector ends.

### 2.2 Candidate sets: what they decide and what they cannot

Following rule 12n, the run's sets come from the pre-pass. A unit decides
only over a closed set; an open or empty set never decides. The sets give
three things:

1. **The closed world of operations.** Every registration, removal and
   destroy that can touch a member is known. A held key with a registration
   whose loop or held object the census cannot key opens that key, and every
   release of its record then refuses.
2. **Disjointness.** A removal or destroy whose member's set is disjoint from
   the record's member's set provably does not end it.
3. **Destroy coverage.** Only a destroy of a member in the set can end the
   record.

They do **not** give equality (§1.2), so ending a record always needs a
must-equality.

### 2.3 Member equalities (derived, guarded)

`p ≡ q` for two loop paths relative to one record `x`. Example:
`x.evl ≡ x.instance.config.evl` when `¬threaded(x.instance)`. Equalities are
derived from the run's stores:

* every store into the left member establishes it;
* no store into a member on the right path follows;
* the guard is an immutable flag tested through a known predicate.

The derivations in this census:

* `flb_input_collector.evl`: two stores, both in `collector_create`, under
  `flb_input_is_threaded(ins)` and its negation, with `coll->instance = ins`
  (`:2246`). `flb_input_thread_instance.evl` has one store; `flb_config.evl`
  has one store, before any registration.
* `flb_sched_timer`'s `sched.evl` against `config.evl`: the one stores of
  §1.3.

Two premises are needed, and both are already rule 10p's or derivable.
P-INSTANCE-CONFIG says the setter's `config` is `ins->config`; rule 10p's
P-INSTANCE family, derivable from the `cb_init` dispatch sites. The threaded
flag is immutable after init (rule 10p's premise). A path pair with no
derivation is not equal. A removal through it ends nothing.

### 2.4 Prerequisites shared by all variants

All three variants share these prerequisites. They are product work, listed
here for the decision.

* **Loop create and destroy declarations.** Declare `mk_event_loop_create`
  (a fresh loop, null on failure) and `mk_event_loop_destroy` (consumes the
  loop and ends every record at it) in `qcore-provider-libevent`. Today
  neither is declared.
* **Destroy at a path.** Rule 12t says: "Do not … read a destroy at a path as
  a holder release". It is amended so that a destroy of the loop at path `p`
  ends the records whose holder is `p` (or `≡ p`), when no store into `p`'s
  member intervenes.
* **Removal poisoning (the §1.5 correction).** A removal through a loop the
  record's holder is not proved equal to **poisons** the record: it stays, no
  later removal ends it, and only its loop's destroy does. This changes the
  decision of rule 12t's composition only in the A-then-B case. It is
  fail-closed (it can only turn a decided row into a refused one). Whether
  any current row has such a sequence was not checked here; the
  implementation measures it, and any move is a named correction.
* **The closed world of releases of a held record type.** A record type whose
  interior is registered (for example `flb_input_collector`) must be released
  only through a pointer of its own type. Premise `P-LOOP-HELD-RELEASE-TYPED`
  is checked like rule 12n's opening rules: a conversion of `R *` to another
  pointer type, a `memcpy` of an `R`, or an address-taken `R` in a generic
  container opens the key.

## 3. The summary-model variants

Each variant composes with the candidate sets of §2.2 and the equalities of
§2.3.

### Variant S1: derived per-function effect summaries (T2 with type-keyed lifting)

**Model.** Every analysed function gets a derived summary, computed bottom-up
over the call graph (an SCC iterates from "unknown"; rule 12n's derivation
edges order the callbacks). The summary has four parts:

* **may-`Reg`.** The registrations it leaves live, as `(object path, loop
  path)` relative to its formals or `#result`. Rule 12t's "never forward a
  retention into a summary" is replaced by "forward a resolved one".
* **must-`Rem` / must-`Destroy`.** The removals and loop destroys on every
  normal path.
* **`requires`.** For each release in the body that the body cannot justify:
  `ended(x.E @ x.M)` for each footprint member `M` of the released record,
  with the equality guard it lacks.
* **lifted records.** When a caller cannot name a forwarded record's root (a
  list-iteration or id lookup, as in `flb_input_collector_start` and
  `get_collector`), the record becomes the run-wide footprint of its held key:
  "some `R` has `E` registered at its `M`". It is then no longer tracked per
  path. It is not dropped.

**Check.** A release `free(x : R)` is admitted when, for each `M` in
`footprint(R.E)`, a dominating must-`Rem(x.E, p)` with `p ≡ x.M` exists, or a
must-`Destroy(q)` with `q ≡ x.M`, or the obligation is put in the summary's
`requires`. A run-wide **fold** (demote-only, like `callback_member_fold`)
checks that every caller of a function with `requires` either discharges it
or forwards it. A root (a thread entry, a callback whose candidate set is not
closed, an unread caller) that forwards it demotes every row in the chain to
`Unknown`, naming the root.

**Soundness argument.** The abstract `Reg` over-approximates the concrete one.

* Forwarded may-registrations are kept.
* Lifting replaces a path record by the type-keyed footprint, which covers
  every object of `R` (closed world: §2.2(1), §2.4).
* `must`-effects under-approximate: they are on every path.
* Endings need a derived equality.
* A poisoned record ends only at its loop's destroy.
* The fold only demotes.

Under these conditions, the theorem is that an admitted release frees no
object with a record in a live loop. An unresolved path, an open candidate set
or an unproven equality never decides. Nothing is `Unsafe`: a release with a
possibly live record refuses, as rule 12t does.

**What it cannot decide.** A record ended by another thread's loop destroy
(R1, R4). The engine thread's summary has no effect of the input thread. The
threaded arm of `flb_input_exit_all` keeps its `requires` and refuses. Order
across event-loop rounds (OR-9's "before the current batch ends") is also
beyond it, as are list-membership invariants ("every timer in `timers_drop`
was disabled"; OB-10's and OR-9's "list membership").

**Syntax.** None. Summaries are derived, never declared. Reports need two
leaf names: `lifetime-precondition-undischarged` (naming the root and the
footprint) and `retention-removal-poisoned`.

**Worked examples.**

1. **The non-threaded collector (R2).**
   * `collector_start(coll, config)` gets the summary
     `Reg(coll.event @ coll.evl)`. Its exit would no longer refuse (today the row refuses earlier, on three member accesses; PREDICTION.md).
   * `flb_input_collector_start(coll_id, in)` cannot name `coll` (a list
     lookup), so the record is lifted to `footprint(flb_input_collector.event)
     = {x.evl}`.
   * `flb_input_collector_destroy(coll)`: `Rem(coll.event @
     coll.instance.config.evl)` then `free(coll)`. The equality `coll.evl ≡
     coll.instance.config.evl` holds only when `¬threaded(coll.instance)`, so
     the summary is
     `requires ended(coll.event @ coll.evl) when threaded(coll.instance)`.
   * `flb_input_instance_destroy(ins)` forwards it for every collector of
     `ins.collectors`.
   * `flb_input_exit_all` discharges the non-threaded arm: the guard is
     false, and the `flb_input_is_threaded` test in `flb_input_instance_exit`
     is rule 10p's hop F. The threaded arm keeps it. The fold demotes the
     chain, with the reason "threaded arm: no ending of `x.event @ x.evl`".
2. **A scheduler timer (R6).**
   * `flb_sched_timer_cb_create(sched, …)` creates `timer`, which is linked
     into `sched->timers` and optionally returned through `out_timer`. It
     registers `&timer->event` at `sched.evl`, which is lifted to
     `footprint(flb_sched_timer.event)`. The census's three registrations
     give `{x.sched.evl}`, using the §1.3 equality for `schedule_request_now`.
   * `flb_sched_timer_destroy(timer)` runs `flb_sched_timer_cb_disable`. That
     removal is conditional on the scalar `timer_fd != -1`. Without scalar
     member guards the summary is cautious: `requires ended(timer.event @
     timer.sched.evl)`. `flb_sched_destroy`'s timer loop discharges it with
     its explicit removal (`:843-844`). `flb_sched_timer_cleanup` cannot
     discharge it: membership in `timers_drop` is not tracked.

**Lean.** A new module `QCore.LoopLifetimeSummary` over the §2.1 machine.
Proposed theorems:

* `summary_composition_overapproximates` (the substitution of actuals);
* `lifting_preserves_coverage` (under a closed footprint);
* `an_admitted_release_frees_no_live_registration`;
* `an_unproven_equality_never_ends_a_record`;
* `a_poisoned_record_ends_only_by_its_loop_destroy`;
* `the_fold_only_demotes`;
* the shared decision and trace vectors.

It is bound to `QCore.RetentionPath`: the S1 step restricted to one body is
12t's step, corrected for poisoning.

### Variant S2: per-thread loop lifetimes from the thread-entry census (rules 10p/10q)

**Model.**

* Each allocation site gets an **owner**: the unique thread context that waits
  on it. Owners derive through rule 10p's census and hop S:
  * `input_thread` waits on `thi->evl` (I);
  * `output_thread` waits on O;
  * `flb_engine_start` on `flb_lib_worker`'s thread waits on E;
  * the downstream worker waits on W;
  * the logger waits on G.

  A site with no unique waiting context has no owner.
* Each owner's **exit summary** lists, for every exit path of its entry, the
  held keys that may remain registered in a loop it owns while that loop stays
  live on that path.
  * `input_thread`'s main path destroys I after `flb_sched_destroy`, so it
    leaves none.
  * Its early returns leave I live, but with only scheduler records and no
    collector.
* A **join** of the owner thread that rule 10q admits gives the joiner a fact
  for every held key: no record of that key remains live in a loop the thread
  owned. Rule 10q admits a join only when it is on every path and its zero
  result is checked.

**Check.** A release `free(x : R)` in the joiner is admitted when every
footprint member `x.M` is `≡` a loop owned by a thread whose admitted join
dominates the release. The guarded equality of §2.3 maps `x.evl` to
`x.instance.thi.evl`. Alone, S2 decides only cross-thread endings; same-thread
endings are S1's.

**Soundness argument.**

* Ownership is derived, and a site with two waiting contexts has no owner.
* The exit summary is per path.
* Rule 10q's join already proves that every effect of the joined thread
  happens before the joiner's continuation (`QCore.CondvarJoin`).
* An unchecked or missing join, or an unresolved spawner, never discharges.
* A thread whose exit path may leave a key live in a live loop never
  discharges that key.

**What it decides on Fluent Bit at these revisions: nothing.**

* `flb_input_thread_instance_exit` joins without checking the result (`:607`)
  and has a path with no join (`:602-605`).
* `flb_input_instance_exit` drops the result (`flb_input.c:2134`).
* `flb_output_thread_pool_destroy` frees without a join on one path (`:643`)
  and joins unchecked (`:646`).

Rule 10q refuses each of these. The only checked join is `flb_stop`'s second
(`flb_lib.c:1077`). It covers E: in R3 everything on E is removed before E's
destroy on the main thread, which S1 decides without S2. S2's value here is a
precise refusal: "join of `input_thread` unchecked / absent on a path". It
would decide a revision that checks those joins.

**Syntax.** None (derived).

**Worked example.** R1 for a threaded `in_tail` instance:

* The footprint `{x.evl}` maps to `x.instance.thi.evl`, whose owner is
  `input_thread` (the waits at `:420-421`).
* `input_thread`'s exit summary leaves no `flb_input_collector.event` record
  live in I on any path. On the early paths no collector is started, and on
  the main path I is destroyed.
* The joiner's join at `:607` is unchecked, and `:602-605` returns without it.
  So it is not discharged.
* With a checked join on every path, `flb_input_exit_all`'s threaded arm would
  discharge S1's `requires`, and the chain of §3/S1 example 1 would stand.

**Lean.** A new module `QCore.LoopThreadLifetime`:

* `a_checked_join_ends_the_owned_registrations` (from
  `QCore.CondvarJoin`'s release verdict and `QCore.LoopOwner`'s owner
  derivation);
* `an_unchecked_or_missing_join_never_discharges`;
* `an_ownerless_site_never_discharges`;
* `a_path_leaving_a_live_record_never_discharges`.

### Variant S3: declared lifetime protocols at record types (contract surface)

**Model.** Instead of deriving footprints and equalities (§2.3), a provider
module **declares** them for a record type, and the pre-pass checks each
declaration against every registration and store site in the run, in rule
15w's discipline. A declaration that a site contradicts is withdrawn. One that
cannot be checked is a declared assumption (rule 13i) and carries its premise
into every verdict that rests on it. Summaries are then S1's, read from
declarations where present.

**Syntax (proposal; needs the user's decision).** It is built on the decided
sigils (`#` binders and value keywords, bare clause-kind word after `quant`,
dotted `TAG.FIELD`, `|->`):

```qcore
// footprint: an flb_input_collector's event, whenever registered, is held by the loop at its own evl
quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;

// guarded member equality, using decided scalar guards extended to members (not yet decided)
quant live #coll: flb_input_collector.evl |-> flb_input_collector.instance.config.evl
    when #coll.instance.is_threaded == 0;
```

`registered` would be a new clause-kind word. The guard on a member
(`#coll.instance.is_threaded`) extends decision 2, which allows only integer
**parameters**. Both need the user's decision.

**Soundness argument.** These are the S1 rules. A declared footprint or
equality is used only when its body check stands (`Validated`). Otherwise it
is a named declared assumption, never silently. A withdrawn declaration falls
back to S1's derivation, which may refuse.

**Worked example.** The R2 chain with the two declarations above. The check
reads `collector_create`'s two stores and `collector_start`/`_resume`'s two
registrations, and validates both clauses. The verdicts are then S1's, with
`contract_dependencies` naming the two clauses.

**Lean.** `QCore.DeclaredLoopProtocol`:

* `a_validated_protocol_equals_the_derived_one`;
* `a_withdrawn_protocol_never_decides`;
* `a_declared_protocol_is_named_in_every_dependent_verdict`.

**Assessment.** S3 adds surface and assumptions where S1 derives the same
facts from the census. It earns its place only where a body is not in the run:
a plugin compiled elsewhere, or a library boundary.

## 4. Obligations each variant would move

These are read against R1–R6. "Moves" means rows the obligation maps to may
trade leaves or move. **No obligation is predicted to discharge** under any
variant at these revisions. The per-row prediction is in
[PREDICTION.md](PREDICTION.md).

| Obligation | S1 | S2 | S3 |
| --- | --- | --- | --- |
| OB-10 (teardown touches only live outputs) | leaves only (`flb_engine_start`'s registrations forward); list membership after unlink still missing | none (`flb_output_thread_pool_destroy`'s joins refused) | as S1 |
| OR-9 (retry invalidation, round ordering) | the timer footprint decides `flb_sched_destroy`'s and `_cb_disable`'s side; `timers_drop` membership and round ordering stay undecided | none | as S1 |
| C1 (collector access by the loop owner) | none (C1 is rule 10p's thread derivation, not a lifetime) | owner derivation is shared with 10p; no new discharge | none |
| C2 (two completion meanings) | none | none | none |
| C4 (condvar publication) | none | none | none |
| C5 (join before release) | none | **refines the reason**: the joins are unchecked or absent on a path (`flb_input_thread.c:602-607`, `flb_output_thread.c:640-646`) | none |

## 5. Recommendation

Implement **S1** with the §2.4 prerequisites first. Use the candidate sets as
the pre-pass, the equalities derived, and no new syntax. Add **S2** as S1's
join edge in a second card. S2 alone decides nothing at these revisions, but
it turns the threaded-arm refusals into the precise C5 reason, and it decides
a revision whose joins are checked. Keep **S3** in reserve for bodies outside
the run.

The order is fail-closed throughout. A row S1 frees is a registering function
whose only blocker was the exit's `retention-undecided`. Every releaser's
`Safe` needs its callers' discharge through the fold. The threaded chain stays
`Unknown`, with the R1 reason named.

**Expected yield, stated plainly** (PREDICTION.md, measured BEFORE at
`3f8f1a617`): no status move on any route under any variant. On the system
and slice routes, every row of the R1–R6 chains already refuses earlier, at
member accesses (`indirect memory access is outside the resource projection`)
and identities (`unresolved resource identity`). For example, `collector_start`
never reaches rule 12t. The variants change leaves only: the loop
create/destroy leaves, and `retention-undecided` at `schedule_request_now` /
`flb_sched_create`. The design's value at these revisions is that the lifetime
reasoning is correct and ready when those blockers clear. If the user wants
measurable movement first, the access and identity blockers on these chains
are the prior card.

Decisions requested from the user:

1. **D1.** Variant: S1 now (+ S2 later), or another combination.
2. **D2.** Amend rule 12t's "do not read a destroy at a path as a holder
   release" as in §2.4, and its unmatched-removal composition to poisoning.
3. **D3.** Whether guarded equalities may use immutable flags through known
   predicates (`flb_input_is_threaded`), derived only. This is the guard S1
   needs; it is not a new spelling.
4. **D4.** If S3 is wanted at all, the `quant registered` clause-kind word and
   member guards.
5. **D5.** S1's scope: loop holders only (recommended), or forwarding of every
   rule-12t retention. The second reaches 10 LZ4 / `rd_kafka_queue_new`
   corpus rows. Whether they become `Safe` depends on the root rule
   (PREDICTION.md).
6. **D6.** Order: this lifetime card first, or the member-access and identity
   blockers on the R1–R6 chains first, so that the lifetime model can be
   measured.

## 6. Follow-ons

None of these is a finding.

* **`RETENTION-UNMATCHED-REMOVAL-CLEARS-STATUS`** (§1.5, §2.4). The
  composition of rule 12t and `QCore.RetentionPath` accepts a removal at an
  unmatched path followed by a matched one. Monkey's `_mk_event_del` resets
  the event's status on the first call, so the second call does nothing when
  the loops differ. The fix is to poison the record. To be registered in the
  ledger when the user's decision lands; it is in scope of any variant.
* **Unjoined teardown paths (readings, not validated).**
  * `flb_input_instance_exit` drops the -1 of `flb_input_thread_instance_exit`
    (no join, `flb_input_thread.c:602-605`), and the engine then frees the
    collectors and the instance while the thread may run (§1.5).
  * `flb_output_thread_pool_destroy` frees `th_ins` without a join when the
    stop write fails (`flb_output_thread.c:640-644`).
  * `flb_input_thread_instance_init` destroys `thi` after storing it in
    `ins->thi` when the spawn fails (`flb_input_thread.c:626`, `:632`), so a
    later exit reads a freed `thi`.

  Each is a candidate for C5's census (rule 10q). None is presented as an
  upstream defect.
* **The open HTTP-server members** (`flb_http_server_init` has no direct
  call in the tree). Any variant keeps them open.
