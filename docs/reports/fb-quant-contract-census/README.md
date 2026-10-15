# FB-QUANT-CONTRACT-CENSUS

Measurement and records only: no product code, contract, rule or Lean change.
The question (user, 2026-10-15): can the `quant live #…` forms decided for
librdkafka state Fluent Bit's contracts, deeply enough to audit it? This card
finds each candidate pattern in Fluent Bit, sizes what it blocks today on the
system and tail-pause slice routes (with the null property), fits it against
the decided syntax with a draft on a real function, and ranks the next
implementation cards. Every draft below is a **draft in this report**; none is
in a provider.

Base and measured tip: local `main` `d708641bc` (OP-CALLBACK-MEMBER-PATH-RELEASE
gate). Source read only from the prepared exports the routes analyse
(system: `046accf6`, `private/w45-fb/w46-external-callee-contracts/attempt-1`;
slice: the `90978ac6` export, `private/w47-tail/attempt-1/prep`); the Fluent Bit
and Monkey checkouts were not touched.

## Measurements (one BEFORE run per route)

Binary `7bf6e937…` built from `d708641bc` (`scripts/build.sh`), each route under
the shared heavy lock with `QCORE_NULL_PROPERTY` (`scripts/measure.sh before
private/fqc/bin/before`); `artifacts/runs.json` holds the identities.

| Route | Custody | Null property | Report sha256 | Wall (inside lock) |
| --- | --- | --- | --- | --- |
| corpus `66910c10` (arbiter, `--jobs 4`) | 936 / 9 / 3,889 | 1,113 / 1,435 / 2,355 / 18 Unsafe | `71387bd3…` | 728 s |
| system (8 files, 448 rows) | 93 / 4 / 351 | 84 / 125 / 245 | `d25e937f…` | 87 s |
| tail-pause slice (13 files, 404 rows) | 52 / 1 / 347 / 4 Pending | 71 / 103 / 232 | `0899ba80…` | 47 s |

All equal the round-28 figures; the corpus report digest equals main's
`71387bd3…` and `run.json`'s `binary_sha256` equals the built binary. The corpus's
only Fluent Bit unit is `flb_input_chunk.c` (64 rows, 13 Safe / 51 Unknown).

## Method

`scripts/classify.py` reads each route's report and null stream and classifies
every custody leaf (a row's diagnostic) and every null `candidate` check (the
checks that keep a null row Unknown; `moved` checks are already preconditions)
by the text of its origin line in the analysed export and, for custody, its
description. A leaf may fall in several patterns or none. Per pattern:

* **total**: non-Safe rows with at least one leaf of the pattern;
* **alone**: non-Safe rows whose *every* leaf is of the pattern (removing them
  would leave no leaf; it does not say the row would be Safe);
* **via callee**: rows with no leaf of the pattern that name, as an uncontracted
  callee, a function of the route whose row carries one (one step).

Rows are per compilation configuration (static and shared variants are
separate units, so most functions appear twice); the tables give distinct
functions, with rows in `artifacts/patterns.json`. The classification is
lexical and is a census of where the pattern stands, not a proof that any
contract would decide a row. Of the non-Safe leaves, 266 (system, of 3,586),
249 (slice, of 3,715) and 10 (corpus unit, of 430) fall in some pattern; the
rest are the routes' other blockers (`artifacts/runs.json`). Obligations:
`scripts/obligations.py` maps OR-1…OR-11 and OB-1…OB-10
(`w45-fb-system-selection/lifetime-inventory.json`, their cited `046accf6` line
ranges to the enclosing functions) and the tail-pause contracts C1…C5
(`w47-tail-pause-bindings` §5, by the functions it binds) to the patterns'
rows (`artifacts/obligations.json`). Source sites: `scripts/source_census.py`,
`artifacts/source-census.json`.

## Per-pattern table

Functions: total / alone / via callee. Null: functions with a candidate on the
pattern's lines / whose every candidate is there.

| Pattern | System custody | System null | Slice custody | Slice null | Corpus (`flb_input_chunk.c`) | Obligations touched |
| --- | --- | --- | --- | --- | --- | --- |
| P1 plugin callback members | 3 / 1 / 2 | 3 / 0 | 14 / 0 / 11 | 10 / **7** | 0 | C1, C2, C4, C5 |
| P2 return-code ownership | 2 / 0 / 1 | 0 / 0 | 0 | 0 | 0 | OR-3, OR-8, OR-10 (via `handle_output_event`) |
| P3 teardown member states | 36 / 3 / 11 | 3 / 0 | 24 / 1 / 6 | 1 / 0 | 4 / 0 / 3 | OB-10, OR-2…OR-6, OR-8…OR-11 |
| P4 fresh constructors, empty callback members | 11 / 0 / 7 | 0 | 9 / 3 / 3 | 0 | 0 | OR-2, OR-3, OR-8, OR-10, OR-11, C1 |
| P5a event-loop registrations | 16 / 1 / 8 | 3 / 3 | 22 / 4 / 8 | 5 / 0 | 2 / 0 / 1 | OB-10, OR-9, C1, C2, C4, C5 |
| P5b loop-owner obligation (rule 10p, already declared) | 3 / 0 / 0 | 0 | 6 / 4 / 0 | 0 | 0 | C1, C3 |

All five patterns together are every leaf of only 5 system functions
(`flb_engine_destroy_tasks`, `flb_output_pre_run`, `flb_sched_request_invalidate`,
`flb_sched_timer_cb_destroy`, `flb_sched_timer_cb_disable`) and 13 slice
functions (`flb_input`, `flb_output`, `flb_filter`, `flb_input_instance_exit`,
`flb_input_pause_all`, `flb_input_rate_gate_timer_schedule`,
`flb_input_test_pause_resume`, `flb_input_thread_collectors_start`,
`flb_tail_file_remove_all`, and the four `flb_tail_fs_{inotify,stat}_{pause,resume}`).
The other 180 of 185 system and 221 of 234 slice non-Safe functions keep
at least one other blocker. No obligation OB-1…OB-9, OR-1 or OR-7 function carries any
pattern leaf: their blockers are the reference, element-identity and
publication residuals already registered (`W45-FB-BACKLOG-*`,
`W45-FB-OR*`).

### P1: plugin callback members invoked through an instance

**Evidence.** Invocations through `ins->p->cb_*` / `p->cb_*`: 36 sites in 9
files of the tree, e.g. `flb_output.c:162` (`cb_pre_run`), `:669` (`cb_exit`),
`:1890` (`cb_init`), `:1910` (`cb_worker_init`) at `046accf6`; slice
`flb_input.c:3041`/`:3044` (`cb_pause_checked` / `cb_pause` in
`flb_input_plugin_pause`), `:3061`/`:3064` (resume), `:2046` (`cb_pre_run`),
`:2141` (`cb_exit`); collector callbacks `collector->cb_collect` at
`flb_input.c:3321` and `flb_input_thread.c:170`. Registration is by
designated initializer: 140 plugin descriptors (56 input, 50 output, 24
filter, 9 processor, 1 custom), e.g. `in_tail_plugin` at `tail.c:867`
(`.cb_pause = in_tail_pause`, `:874`) and `in_storage_backlog_plugin` at
`sb.c:811`; member counts `cb_init`/`cb_exit` 140, `cb_pre_run` 66,
`cb_flush` 50, `cb_pause`/`cb_resume` 32, `cb_pause_checked`/`cb_resume_checked`
5. The descriptor reaches an instance by `instance->p = plugin`
(`flb_input.c:490`, `flb_output.c:857`, `flb_filter.c:507`, `flb_processor.c:2188`,
`flb_custom.c:214`).

**Leaves.** Custody: `indirect memory access is outside the resource
projection` (system 10, slice 26) at the call and `unmodeled call, aggregate or
indirect access in branch condition` (8, 32) at the `if (p->cb_x)` test. Rule
11p reads a member reached through a pointer as an open set (vtable dispatch),
and rule 12n closes member invocations only for a declared callee. Alone:
`flb_output_pre_run` (system). Null: 34 slice candidates are `ins->p` / `p`
member accesses; they are every candidate of 7 slice functions
(`flb_input_plugin_pause`, `flb_input_plugin_resume`, `flb_input_pause`,
`flb_input_resume`, `flb_input_instance_pre_run`, `flb_input_instance_exit`,
`flb_input_thread_instance_pre_run`).

**Fit.** The decided form names the invocation; the member path crosses a
pointer:

```qcore
// DRAFT. flb_input_plugin_pause (flb_input.c:3034-3052 at 90978ac6).
extern fn flb_input_plugin_pause(ins: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ins);
    ensures normal resource_callback_member_invoked(ins, flb_input_instance.p.cb_pause_checked, _, _);
    ensures normal resource_callback_member_invoked(ins, flb_input_instance.p.cb_pause, _, _);
    ensures normal resource_return_none();
};
```

Missing: (a) `p` is `struct flb_input_plugin *`, so `flb_input_instance.p.cb_pause`
is a dotted path through a pointer; the decided `TAG.FIELD` paths stay inside
one record (`rd_kafka_s.rk_conf.log_cb`): **a user decision on a pointer hop**;
(b) the in-body call `ins->p->cb_pause(...)` is decided only when 11p/12n close
a member read through a pointer over the run's set; (c) that set is closed only
when every plugin unit of the build is read (`MEMBER-PREPASS-OTHER-ROUTES`: the
pre-pass runs on the pinned route only, and the system and slice selections
hold one plugin unit each), under a premise that the registered plugins are
exactly the build's generated `flb_plugins.h` list (proxy plugins excluded);
(d) a contract per candidate (37 for pause alone; 140 for `cb_init`). The null
side needs none of this. Existing forms (rules 14x, 14z) fit:

```qcore
// DRAFT. bounds contract v1, host flb_input_new (flb_input.c:490 stores the
// plugin it dereferenced at :459/:461).
extern fn flb_input_new(config: ptr<mut, u8>, input: ptr<const, u8>, data: ptr<mut, u8>, public_only: i32) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires nonnull_member_stable(flb_input_instance, p);
};
extern fn flb_input_plugin_pause(ins: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires nonnull_member_scope(flb_input_instance, p, ins);
};
```

Risks the implementation must measure: the instance is `flb_calloc`ed, so `p`
is NULL until `:490` (rule 14x counts a `memset`-like whole-object write as an
unproved store; whether a fresh `calloc` object is exempt must be read, not
assumed), and rule 14z adds a `call-precondition` check at every direct call,
which can add candidates in callers.

### P2: return-code-dependent ownership (`FLB_OUTPUT_RETURN`)

**Evidence.** 242 `FLB_OUTPUT_RETURN` sites in 51 files (`FLB_RETRY` 117,
`FLB_ERROR` 50, `FLB_OK` 42, a variable 33). `cb_flush` returns `void`
(`flb_output.h:307`); `FLB_OUTPUT_RETURN(x)` is `flb_output_return_do(x);
return` (`:1443`), which calls `flb_output_return(ret, co)` (`:1307`): the code
is packed with the task and output ids by `FLB_TASK_SET` and written to the
engine or worker pipe, then the coroutine yields. The engine decodes it in
`handle_output_event` (`flb_engine.c:590`, `FLB_TASK_RET(key)`), finds the task
by id (`config->task_map[task_id].task`, `:617`) and per code: `FLB_OK` drops
the flush's task user with a release check (`flb_task_users_dec(task,
FLB_TRUE)`, `:703`); `FLB_RETRY` hands it to a retry (`flb_task_retry_create`,
`:746`, then `flb_task_users_dec(task, FLB_FALSE)`, `:797`; OR-3); `FLB_ERROR`
drops it. `FLB_OK`/`FLB_ERROR`/`FLB_RETRY` are macros (`flb_macros.h:30-32`).

**Leaves.** Only the two uncontracted calls on the consumer side:
`flb_output_flush_finished` and `handle_output_event` (system); no output
plugin is in either selection, so the 242 producers are not measured.

**Fit.** `quant live #P: #owner |-> #callback when #result OP C;` does not
express it: there is no `#result` (the callback returns `void` and the code is
an argument of `flb_output_return`), the object handed is a counted task user
found by id across a pipe and a yield, and `@C` resolves enumerators only
(rule 13l). The closest decided shape is the counted-user exit clause by arm:

```qcore
// DRAFT, does not load: @FLB_OK is a macro, and #task and #code are not
// parameters of handle_output_event (the task is config->task_map[id] and
// the code FLB_TASK_RET(val)).
alias #OK = @FLB_OK;
extern fn handle_output_event(ts: u64, config: ptr<mut, u8>, val: u64) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(config);
    ensures normal resource_user_dropped(#task, users) when #code == #OK;   // with release check
    ensures normal resource_user_dropped(#task, users) otherwise;           // RETRY hands the user to a retry first
};
```

Missing: **a user decision on `@` for object-like integer macros** (Clang sees
no enumerator); a binder for an object identified by a message (the pipe word's
task id); and the producer side (a `void` callback's code argument). The
result-dependent hand-offs inside the system slice proper are return-code
custody (C04.13) and counted-user exits, not callback hand-offs: OR-7's
`resource_user_exit(task, -1, restored)` already states `flb_output_task_flush`;
OB-5's `flb_input_chunk_map` NULL arms and OB-2's `-3` arm are
`W45-FB-BACKLOG-MAP-ARMS` and `W45-FB-BACKLOG-REFERENCE-VOCABULARY`.

### P3: post-call member states in destroy and teardown paths

**Evidence and leaves.** Mostly `call to uncontracted function` at teardown
calls whose bodies are in the route but in another unit: system
`flb_output_instance_destroy` 16 leaves, `flb_sched_timer_destroy` 9,
`flb_task_destroy` 9 (`flb_task.c:899-948`: frees routes, unlinks, destroys
`task->ic` with `del`, destroys every retry and `task->event_chunk`),
`flb_sched_request_destroy` 5, `flb_task_retry_clean` 4, `flb_task_queue_destroy`
4, `flb_task_users_release` 2, `flb_output_flush_destroy` 2,
`flb_event_chunk_destroy` 2, `flb_sched_request_invalidate` 2, `flb_output_exit`,
`flb_input_exit_all`, `flb_input_chunk_release_route`, `flb_coro_destroy`; slice
`flb_tail_config_destroy` 39 (the error paths of `in_tail_init`),
`flb_tail_file_remove` 17, `flb_input_instance_destroy` 9, and six more. 30
system leaves are `unresolved resource identity` of the argument
(`flb_task_retry_destroy(retry)` 7, `flb_input_chunk_destroy(ic, …)` 8): no
contract decides those. Alone: `flb_engine_destroy_tasks`,
`flb_sched_request_invalidate`, `flb_sched_timer_cb_destroy` (system),
`flb_tail_file_remove_all` (slice).

**Fit.** The object itself is `resource_released`; what the destroyer leaves
at members is rule 30's form:

```qcore
// DRAFT. flb_task_destroy (flb_task.c:899-948 at 046accf6).
extern fn flb_task_destroy(task: ptr<mut, u8>, del: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(task, nonnull);
    ensures normal resource_borrowed(del);
    quant live #task: flb_task.ic |-> #released;
    quant live #task: flb_task.event_chunk |-> #released;
    ensures normal resource_return_none();
};
```

Missing: rule 30 reads member-path states at callback invocations and refuses
a direct call (`member-path-direct`): **a direct call of a body-checked
destroyer needs the rule extended** (no new word); the chunk's on-disk deletion
depends on `del`, and a guard on a scalar parameter beside `quant live` (`when
#del != 0`) is **new surface** (C04.14.1 admits `when X OP K` on
`resource_retained_by` only); `flb_task_users_release` and the release check
are `W45-FB-USERS-RELEASE-CHECK`.

### P4: fresh constructors with empty callback members

**Evidence.** `flb_sched_timer_create` (`flb_scheduler.c:869-892`):
`flb_calloc` (so `cb` and `cb_coroutine_wrapper` are NULL), `MK_EVENT_ZERO`,
and `mk_list_add(&timer->_head, &sched->timers)`; the callback is stored later
by `flb_sched_timer_cb_create` (`:660`) and invoked by the scheduler
(`timer->cb(config, timer->data)`, `:614`, `:624`). `collector_create`
(`flb_input.c:2222-2270` at `90978ac6`) stores `coll->cb_collect = cb` and links
the collector into `ins->collectors`. Leaves: uncontracted constructors
(`flb_sched_timer_create` 4, `flb_sched_request_create` 2, `flb_coro_create`,
`flb_output_flush_create`, `collector_create` 3, `flb_output_new`,
`flb_filter_new`), `flb_input_new` awaiting review, and indirect accesses at
`MK_EVENT_ZERO(&timer->event)` / `timer->cb = cb`. Alone: `flb_input`,
`flb_output`, `flb_filter` (slice, `flb_lib.c`).

**Fit.** `quant live #result: TAG.FIELD |-> #absent;` is exactly the
`rd_kafka_op_new0` shape:

```qcore
// DRAFT. flb_sched_timer_create (flb_scheduler.c:869-892 at 046accf6).
extern fn flb_sched_timer_create(sched: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(sched);
    quant live #result: flb_sched_timer.cb |-> #absent;
    quant live #result: flb_sched_timer.cb_coroutine_wrapper |-> #absent;
    ensures normal resource_return_owned(nullable);   // not faithful: see below
};
```

Missing: every Fluent Bit constructor here **publishes its result** into a
holder's list before returning (`sched->timers`, `ins->collectors`,
`config->inputs`), so the caller does not own it alone; v1 has no
"result linked into a parameter's container" (C04.14 retains a parameter in a
holder, not the result in a parameter): `W45-FB-BACKLOG-PUBLICATION-BLOCKED`,
`W45-CONTAINER-ROOT-REACH`. The `#absent` state is useful only once that holds
and once the member's invocations (P1, P5a) are decided.

### P5: event-loop registrations

**Evidence.** Tree: 102 `flb_input_set_collector_*` sites in 57 files, 37
`mk_event_add` / `mk_event_timeout_create` sites, 22 `flb_sched_timer_*cb_create`
sites in 15 files. Slice: `in_tail_init` registers seven collectors
(`tail.c:410-481`), `tail_fs_inotify.c:432`/`:444`, `tail_fs_stat.c:206`/`:214`;
`collector_start` (`flb_input.c:2351` `mk_event_timeout_create`, `:2362`
`mk_event_add` of `&coll->event`). System: `sb.c:769`, `flb_scheduler.c:133`,
`:672`, `:783`, `flb_engine.c:325`, `:1168`. Leaves (P5a): uncontracted
`mk_event_timeout_create`/`_destroy`, `mk_event_add`/`_del`,
`mk_event_channel_create`/`_destroy`, `mk_event_wait`,
`flb_input_set_collector_*`, `flb_sched_timer_cb_create`,
`flb_input_collector_start`, `flb_input_pause`/`_resume`/`_pause_all`
(system 43, slice 43) and indirect accesses at member actuals such as
`mk_event_channel_create(config->evl, …)` (18, 20). Alone (P5a):
`flb_sched_timer_cb_disable` (system), `flb_input_pause_all`,
`flb_input_rate_gate_timer_schedule`, `flb_input_test_pause_resume`,
`flb_input_thread_collectors_start` (slice). P5b is rule 10p's declared
loop-owner obligation (`no derived thread on this route`, system 3, slice
16 leaves); it alone blocks the four `flb_tail_fs_*_pause/resume` and only the
thread-entry census route discharges it (`W47-LOOP-OWNER-AUDIT-ROUTE`).

**Fit.** The retention is C04.14's shape (b), the registration C04.19's:

```qcore
// DRAFT. Monkey's mk_event_add (qcore-provider-libevent owns Monkey facts).
extern fn mk_event_add(loop: ptr<mut, u8>, fd: i32, type: i32, mask: u32, data: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(data);
    ensures normal resource_retained_by(data, loop);
    ensures normal resource_return_none();
};
// DRAFT. collector_create (static, flb_input.c:2222-2270 at 90978ac6).
extern fn collector_create(type: i32, ins: ptr<mut, u8>, cb: fnptr, config: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_callback_member_registered(result, flb_input_collector.cb_collect, cb);
    ensures normal resource_return_interior(ins, nullable);   // linked into ins->collectors
};
```

Missing: (a) **the loop holder**: every Fluent Bit loop is a member reached
through a formal (`config->evl`, `coll->evl`, `thi->evl`, `sched->evl`), which
rule 12t reads as `retention-undecided`, and the retained event is an interior
(`&coll->event`, `&timer->event`): loop-owner retention needs a holder whose
lifetime is the loop's (a word or a premise such as "the loop outlives every
event registered on it until `mk_event_del` / `mk_event_timeout_destroy`"),
**a user decision**; (b) the public setters return an id, not the collector,
so `flb_input_set_collector_*` registers through a wrapper that forwards its
formal into `collector_create`: registration forwarding is a new mechanism (no
new word); (c) the invocation `collector->cb_collect(...)` closes over the
actuals of 102 sites only with the pre-pass on these routes
(`MEMBER-PREPASS-OTHER-ROUTES`). This is `W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS`
(8 operations), measured here.

## Ranking

Yield = system and slice functions the pattern blocks (alone weighted over
total) and null functions, plus obligations; size = declarations, mechanisms
and user decisions.

| Rank | Card | Yield | Size | Decision |
| --- | --- | --- | --- | --- |
| 1 | FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE (P1, null) | slice null: 7 functions whose every candidate is `ins->p` (34 candidates); system null: 3 functions partly; C2, C4, C5 | 2 stable + about 10 scope declarations, existing rules 14x/14z | none |
| 2 | FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (P5a) | 38 functions total, 5 alone, 16 via callee; OB-10, OR-9, C1, C2, C4, C5 | about 12 operations in two crates, a loop holder, registration forwarding, the pre-pass on the current route | the loop holder |
| 3 | FB-TEARDOWN-DESTROY-DECLARATIONS (P3) | 60 functions total, 4 alone, 17 via callee, 4 corpus; OB-10, OR-2…OR-6, OR-8…OR-11 | about 20 body-checked destroyers; rule 30 at direct calls | a scalar-parameter guard beside `quant live` |
| 4 | P1 custody (plugin vtable dispatch) | 17 functions total, 1 alone | 140 descriptors, 37 to 280 candidate contracts, pre-pass, configuration premise | the pointer hop in a dotted path |
| 5 | P4 constructors | 20 functions total, 3 alone | needs constructor publication first | — |
| 6 | P2 return codes | 2 functions, 0 alone | macro constants, message-identified objects | `@` on integer macros |

Cards 1–3 are registered below. P1 custody and P4 wait on prerequisites
already tracked (`MEMBER-PREPASS-OTHER-ROUTES`, `CG22-OPEN-INDIRECT-CALLS`,
`W47-LOOP-OWNER-FIELD-KEYED-V-AND-F`, `W45-FB-BACKLOG-PUBLICATION-BLOCKED`);
P2 waits on the user's `@` decision and is not registered as a card.

## Ledger

`named_followups` 780 → 783 (`scripts/update_ledger.py`, idempotent):
`FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE`, `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION`
and `FB-TEARDOWN-DESTROY-DECLARATIONS` registered. `W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS`
and `MEMBER-PREPASS-OTHER-ROUTES` are unchanged and named as prerequisites.

## Gate

Records only: no code, contract, rule or Lean file changed, so the standing
gate is not run (the brief's scope). `scripts/gate.sh`, at the committed
records: the matrix regenerated with its current gate provenance
(`build_matrix.py . --gate docs/reports/op-callback-member-path-release/gate.json`;
the provenance string now names that gate by its repository path instead of
the landing worktree's absolute path; no number is re-measured) exit 0;
`build_docs_index.py` exit 0 and `--check` exit 0; the Python suite under
the heavy lock exit 0 (365 tests OK); the bold rule-id duplicate check
empty. Logs: `private/fqc/gate/`.

## Files

* `scripts/build.sh`, `scripts/measure.sh` (the BEFORE runs), `scripts/source_census.py`,
  `scripts/classify.py`, `scripts/obligations.py`, `scripts/summarize.py`,
  `scripts/census.sh` (classification driver), `scripts/update_ledger.py`,
  `scripts/gate.sh`.
* `artifacts/runs.json`, `artifacts/patterns.json`, `artifacts/obligations.json`,
  `artifacts/source-census.json`.

Private outputs: `private/fqc/` (runs, null streams, classified rows with every
leaf), `private/w47-tail/attempt-1/fqc-before/` (the slice run).
