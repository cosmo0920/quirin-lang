# FB-RETRY-LIFECYCLE-CONTRACTS: the retry-lifecycle contracts, written down

Card FB-RETRY-LIFECYCLE-CONTRACTS (user decision 2026-11-27: write down the
contracts recently tested and observed for Fluent Bit's retry lifecycle, as a
census a human can review, and implement the contracts it shows are
statable). Base `56584db0b` (r82). No gateway rule, no Lean change, no rule
document. Commits: probe `827cac0ce` (the census at base),
[prediction](PREDICTION.md) `3fcd222ed`, semantic `a84fa54c7` (the module,
fixtures, synced inventory lock and pin), `45a17a70c` (the module selected in
`cycle19-resource-contracts.json`; the measured tip), census re-run
`24e49c1b6`, report `43071a9c8`, and `f9661e685` (the multi-module guard's
registration of `flb_output_name` beside its null-deref check; it changes no
analysis, so the measured tip stands).

## Headline

* **The census** ([CENSUS.md](CENSUS.md), [census.json](census.json),
  [scripts/census.py](scripts/census.py) over the fluent-bit-system route
  report): the 258 rows of the five lifecycle units (`flb_task.c`,
  `flb_output.c`, `flb_scheduler.c`, `flb_engine_dispatch.c`,
  `flb_engine.c`, both configurations), the 67 contracts their calls resolve
  in force at base (75 at the tip) with the test files that state or load each, every non-Safe row
  with its leaves by source line, the causes ranked, and each of the 200
  callees they call uncontracted classified in the reviewed
  [scripts/classification.json](scripts/classification.json): **8 (a)**
  statable now, **10 (b)** need vocabulary, **67 (c)** blocked by their own
  body, **115 (d)** out of scope (16 handed to FB-CHUNK-LIFECYCLE-CONTRACTS).
* **The class (a) contracts**, written in the new
  `providers/quirin-provider-fluent-bit/contracts/fluent_bit/retry_lifecycle.qv`
  (section 2), each proved on its callee's shape and refused on a violating
  body (section 3), synced and pinned at 046accf6.
* **Measured** (fluent-bit-system route, base `56584db0b` against tip
  `45a17a70c`, each from its own `git archive` tree and release binary, over
  this host's 046accf6 preparation): custody **116 / 0 / 328 / 4 → 122 / 0 /
  322 / 4**; lifecycle rows 70 / 0 / 184 / 4 → 72 / 0 / 182 / 4. Three
  functions move Unknown → Safe in both configurations, each justified in
  section 4; the prediction said none (section 5). 54 lifecycle
  uncontracted-call leaves clear. The name accessors' borrowed results add
  299 `interior-borrow-ended` leaves in seven rows that stay Unknown
  (BORROW-ENDING-PRECISION). Combined custody+null Safe 76 → 80.

## 1. The census

`census.py` reads one route report and writes the census; `census.sh` runs it
over this host's preparation. A reviewer checks it without running anything:

* **Contracts in force**: each declaration a lifecycle row's
  `contract_dependencies.declarations` lists, its module, its clauses (each
  clipped to 110 characters), the functions using it, and the Rust test files
  that state it (`extern fn NAME(` in their contract text) or load its module
  (`include_str!`). "none" is printed where no test file does: 15 at base,
  among them the retry lifecycle's own `flb_task_retry_destroy`,
  `flb_task_users_inc`, `flb_task_users_dec`, `flb_task_retry_count`,
  `flb_task_running_count`, `flb_task_set_route_status`,
  `flb_output_thread_pool_flush` and `flb_output_flush_id_get` (the
  counted-user ones have generic analogues in `retry_lifetime.rs`'s `u_*`
  family; no test loads their production text). Registered as
  FB-RETRY-CONTRACT-FIXTURES.
* **Blocking reasons**: 188 non-Safe rows at base; causes ranked
  (`call to uncontracted function` 144 rows / 668 leaves including branch
  conditions, unresolved identities 102 / 440, the indirect-access gate 84 /
  414, unpublished unlinks 52 / 74, …), uncontracted callees ranked by rows,
  then every row with its leaves.
* **Classification** (each entry's reason cites the 046accf6 body): class (c)
  by rule when the callee's own row in a lifecycle unit is non-Safe (no
  summary, and a declaration would be checked against the same body); class
  (d) by location (the sibling's units, libc, cmetrics/cfl, the Monkey list
  header, and Fluent Bit bodies outside the system selection and outside the
  retry lifecycle, not read by this card); every (a) and (b) and each
  retry-specific (c) by hand.

The (a) set is the callees whose clause the body settles: four in audited
units whose own rows are Safe but publish no summary a caller reads
(`flb_sched_timer_cb_disable`, `map_free_task_id`,
`flb_task_from_fs_storage`, `flb_sched_ctx_init`), `flb_output_name`
(checked structurally by rule 13a in `flb_output.c`) and three bodies outside
the selection (`flb_output_thread_instance_get`, `flb_metrics_sum`,
`flb_input_name`), which stay declared assumptions at every call.
`flb_output_name` is also declared in `fluent_bit/null_deref.qv` (a bounds
view); the pair is registered as `null_deref_beside` in
`qcore_contract::contract_views::REGISTERED_RELATIONS`, as
`flb_input_chunk_get_name`'s is, so the multi-module guard checks the two
agree on signature.

The (b) and (c) owners, with lifecycle rows / leaves at the tip:
W45-FB-BACKLOG-REACHABLE-RELEASE (5 callees: `flb_sched_request_invalidate`,
`flb_task_clear_route_retry_context`, `flb_task_retry_clean`,
`flb_output_exit`, `flb_input_exit_all`; 14 / 20), W45-FB-USERS-RELEASE-CHECK
(`flb_task_users_release`; 4 / 4), FB-RETURN-LINKED-ELEMENT (new:
`flb_sched_timer_create`; 8 / 8), LERI-HELD-FORMAL-LINK-OBLIGATION
(`flb_output_task_queue_enqueue`; 2 / 2), MEMBER-EFFECT-STORES
(`flb_event_chunk_update`; 2 / 2), SLOT-MOVE-FIELD-SLOT
(`flb_config_task_map_grow`; 2 / 2), W45-FB-OR6-ELEMENT-IDENTITY (the
flush pair; 10 / 14), W45-FB-OR7-FLUSH-CREATE-OPAQUE
(`flb_output_flush_create`; 2 / 2) and FB-RETRY-LIFECYCLE-BODY-BLOCKERS
(new: the other 64 callees whose own body blocks; 146 / 220).

## 2. The contracts

`retry_lifecycle.qv`, each clause read off the 046accf6 body cited beside
it:

| Callee | Clauses | Body |
|---|---|---|
| `flb_sched_timer_cb_disable` | `resource_borrowed(timer)`, `resource_return_none()`, `quant live #timer: flb_sched_timer.event.fd \|-> {#stale(descriptor), #kept}` | `flb_scheduler.c:720-728`: `mk_event_timeout_destroy(timer->sched->evl, &timer->event)` (declared: the event's fd stale) when `timer_fd != -1`, then `timer_fd = -1` |
| `map_free_task_id` | borrowed `id`, `config`; `resource_user_neutral()` | `flb_task.c:75-78`: clears a non-owning map slot |
| `flb_task_from_fs_storage` | borrowed `task`; `resource_user_neutral()` | `flb_task.c:391-397`: two member reads |
| `flb_sched_ctx_init` | `resource_return_none()` | `flb_scheduler.c:36-40`: TLS key initialisation |
| `flb_output_thread_instance_get` | `resource_return_none()` (a pointer result carrying no custody, as `dlerror`) | `flb_output_thread.c:38-44`: the worker's own TLS instance |
| `flb_metrics_sum` | borrowed `id`, `val`, `metrics` | `flb_metrics.c:175-186`: a lookup and an integer add |
| `flb_output_name`, `flb_input_name` | borrowed `ins`; `resource_return_borrowed_from(ins, nullable)` | `flb_output.c:1321-1328`, `flb_input.c:1143-1150`: `ins->alias` or `ins->name` |

Not claimed: values, threads, and `flb_sched_timer_cb_disable`'s removal of
the event from the loop it is registered with (a loop no formal names;
omitting a removal only keeps a caller's reading conservative). The task
helpers state `resource_user_neutral()` as the module's other task queries
do; the rest carry no counted-user clause, which reads as neutral (UC-FRAME).

`providers/contracts.py sync` lists the module in the load path and the
crate's inventory lock (8 declarations: 7 in the functions table,
`flb_sched_timer_cb_disable` in the retaining table for its member state).
[scripts/pin.sh](scripts/pin.sh) pins the eight at 046accf6 (contracts.lock:
83 → 91 recorded; the three not-in-force entries `flb_config_set_program_name`,
`flb_tail_config_destroy`, `flb_tail_file_remove` are the same at base, so
`pin --check` exits 1 at both; `pin --verify-contracts` exits 0). The
66910c10 corpus tree is not on this host, so no identity is recorded there;
the routes this card measures read 046accf6. `cycle19-resource-contracts.json`
selects the module: the first tip (`a84fa54c7`, without it) measured
identical to base, row for row (`private/m/sys-tip`, discarded as evidence).

## 3. Fixtures

`crates/qcore-c-gateway/tests/retry_lifetime.rs`, module
`retry_lifecycle_contracts`, loads the production module with `include_str!`
and the callee declarations it rests on (restated from
`monkey/event_loop.qv`, `libc/threads.qv`, `fluent_bit/resource_memory.qv`):

| Test | Shape | Result |
|---|---|---|
| `the_timer_disable_declaration_holds_on_its_shape_and_its_caller_resolves` | the 046accf6 body and a caller | body Safe with no diagnostic; caller Safe |
| `a_timer_disable_body_that_frees_its_timer_is_refused` | body `flb_free(timer)` | not Safe |
| `the_task_helpers_and_the_name_accessor_hold_and_their_caller_resolves` | the three bodies and a caller | helpers Safe, no diagnostic; caller Safe; `flb_output_name`'s own row records only the W4 summary's disagreement with the derived result it cannot state (`SummaryConflict`), as its production row is Unknown already |
| `task_helpers_that_release_what_their_formals_reach_are_refused` | each helper frees a member | not Safe |
| `a_name_accessor_returning_a_fresh_allocation_is_not_read_as_borrowed` | accessor returns `flb_malloc(4)`, caller drops it | caller not Safe |
| `the_outside_bodies_hold_on_their_shapes_and_their_caller_resolves` | `pthread_getspecific` read, metric add, empty TLS init, and a cross-unit `flb_input_name` call | each Safe, caller Safe |
| `outside_bodies_that_violate_their_clauses_are_refused` | TLS init leaks, getter returns an allocation, sum frees `metrics` | each not Safe; the caller reads both declarations withdrawn |

`Safe` there is for the synthetic shape under the declarations, never a
statement about Fluent Bit. Caller tests are not evidence for a clause; each
clause rests on its body reading and its own-body fixture.

## 4. Measurement

**Routes.** The rows are system-route rows: the fluent-bit-system route
([scripts/system.sh](scripts/system.sh); the evaluation profile's command)
and the opt-in `current --monkey-linked` route
([scripts/opt-in-current.sh](scripts/opt-in-current.sh)), both from
[scripts/run.sh](scripts/run.sh) over this host's preparation
`private/fb-046accf6` (absolute figures are this preparation's). Base binary
`0a5a958a…`, tip `97760bd7…` (`private/m/run-{base,tip2}/binary.sha256`).
Pinned routes (`quirin-audit measure --declared`) cannot run on this host (no
published live-route inputs); nothing is fabricated for them.

**System** ([artifacts/system-totals-base.json](artifacts/system-totals-base.json),
[artifacts/system-totals-tip.json](artifacts/system-totals-tip.json),
[artifacts/system-moved.jsonl](artifacts/system-moved.jsonl),
[artifacts/system-changed.txt](artifacts/system-changed.txt); runs
`private/m/sys-{base,tip2}`, both exit 0; reports `3c45d081…`, `b44abdeb…`):

| | Safe | Pending | Unknown | Unsafe | combined custody+null Safe |
|---|---|---|---|---|---|
| base `56584db0b` | 116 | 0 | 328 | 4 | 76 |
| tip `45a17a70c` | **122** | 0 | **322** | 4 | **80** |

Every status move (both configurations of each), justified:

* `flb_output.c` `flb_output_flush_id_get` Unknown → Safe. Base leaves: the
  uncontracted `flb_output_thread_instance_get` and `th_ins`'s unresolved
  identity. At the tip `th_ins` is the result of a declared call carrying no
  custody, and the body increments and resets an integer member of it or of
  `ins`; its own declaration (`output_flush_id.qv`) holds. Safe relative to
  `flb_output_thread_instance_get`'s declaration, a declared assumption (its
  body is in `flb_output_thread.c`, outside the selection, read for this
  card).
* `flb_input_chunk.c` `flb_input_chunk_output_size_subtract` and
  `input_chunk_output_size_add` Unknown → Safe (the sibling's unit; no file
  of the sibling's is edited). Base leaves: the indirect access and
  `o_ins`'s unresolved identity, both at the nested `flb_output_name(o_ins)`
  inside `flb_warn`. At the tip that nested call is a declared borrowed
  result passed to `flb_log_print`'s borrowed variadic tail; the rest of each
  body is integer arithmetic on `o_ins->fs_chunks_size`. Safe relative to
  `flb_output_name`'s declaration, which rule 13a validates on its body in
  `flb_output.c`.

No Safe → Unknown, no Unsafe added or removed; the four Unsafe rows (OR-1's
and OR-7's provisional candidates) are unchanged, and are never upstream
defects. `flb_output_task_flush`'s undecided OR-7 `-1` clause now stops one
callee later, at the uncontracted `flb_processor_is_active`
(W45-FB-OR7-FLUSH-CREATE-OPAQUE, amended).

Leaf trades in rows that stay Unknown (all in
[artifacts/system-changed.txt](artifacts/system-changed.txt) and the census's
movement section): 54 lifecycle `call to uncontracted function` leaves for the
eight callees clear (27 per configuration; the prediction's table lists those
27 sites, its prose said 26) (plus the sibling units' sites); 299
`interior-borrow-ended` leaves appear where a name accessor's result is used
after a later call naming the instance (`metrics_append_input` 96,
`handle_output_event` 96, `flb_output_init_all` 64, `input_chunk_append_raw`
22, `update_logs_tag_records_metrics` 11, `record_retry_failure_metrics` 8,
`handle_dlq_if_available` 2): rule 12y's fail-closed ending, never Unsafe
(BORROW-ENDING-PRECISION, amended); 11 unresolved identities appear at
`flb_metrics_sum` actuals (`handle_output_event`'s `ins`,
`flb_input_chunk_map`'s `ic`) where the uncontracted call stood; and
`flb_storage_input_create` now reaches three `cio_stream_*` calls a nested
`flb_input_name` masked.

**Opt-in current** ([artifacts/current-totals-base.json](artifacts/current-totals-base.json),
[artifacts/current-totals-tip.json](artifacts/current-totals-tip.json),
[artifacts/current-moved.jsonl](artifacts/current-moved.jsonl),
[artifacts/current-changed.txt](artifacts/current-changed.txt); runs
`private/m/run-{base,tip2}`, both exit 0, about 58 minutes each; reports
`44da0b7b…`, `8d5b8e0d…`): 181 units, of the five lifecycle units none (its
only Fluent Bit `src/` unit is `flb_input_chunk.c`). **1205 / 2 / 3590 / 7 →
1209 / 2 / 3586 / 7**: the same two `flb_input_chunk.c` functions,
`flb_input_chunk_output_size_subtract` and `input_chunk_output_size_add`,
Unknown → Safe in both configurations, justified as on the system route;
leaf trades in nine more `flb_input_chunk.c` functions, as on the system
route. These two runs were not taken through `scripts/gate_slot.py` (an
oversight); they ran beside the sibling card's slotted audits and this card's
first gate attempt on a 32-CPU, 121 GiB host.

## 5. Prediction against measured

The [prediction](PREDICTION.md) block said `route system none`. Measured:
**6 unpredicted moves** (the three functions above, Unknown → Safe, both
configurations). The prediction missed them because it searched only for
rows whose leaves name the eight callees in a `call to uncontracted function`
leaf; these three rows reached them through a nested call or a result
identity instead. The predicted leaf clearances happened; the 299
`interior-borrow-ended` leaves were not predicted (the prediction allowed
for "a result pointer's identity, an access through it", not ended borrows).
On the opt-in route the prediction said no status move; measured 4 (the
two `flb_input_chunk.c` functions, both configurations), unpredicted for the
same reason. Every move is justified above.

## 6. Gate

GATE_PLACEHOLDER

## 7. Hand-offs, conflicts and follow-ups

* **To FB-CHUNK-LIFECYCLE-CONTRACTS**: 16 callees in its units
  (`flb_input_chunk_down`, `_flush`, `_get_tag`, `_release_lock`,
  `_set_up`, `_set_up_down`, `_set_limits`, `_get_direct_routes`,
  `_has_direct_routes`, `_ring_buffer_collector`,
  `flb_chunk_route_plugin_matches`, `sb_segregate_chunks`, and
  `flb_storage_*`; 38 lifecycle rows / 54 leaves). Its rows also move here:
  two Unknown → Safe and leaf trades in seven `flb_input_chunk.c` /
  `flb_storage.c` functions from `flb_output_name`, `flb_input_name` and
  `flb_metrics_sum`; its measurement's base should include this card if it
  lands first.
* **Files likely to conflict**: `cycle19-resource-contracts.json` (one line
  after `resource_string.qv`; the sibling may add its own),
  `providers/contract-load-path.json` (two lines),
  `providers/quirin-provider-fluent-bit/contract-inventory.lock` and
  `contracts.lock` (regenerate with `providers/contracts.py sync` and re-pin
  on a conflict, never by hand), and the tail of
  `crates/qcore-c-gateway/tests/retry_lifetime.rs` (a new module); one
  `REGISTERED_RELATIONS` entry in `crates/qcore-contract/src/contract_views.rs`
  (after `flb_input_chunk_get_name`'s) with the pinned relation counts in
  `crates/qcore-contract/tests/contract_views.rs` and
  `compiler/tests/contract_view_census.rs` (27 → 28; a sibling adding a
  relation adds one more); `docs/reports/INDEX.md` and `README.md`
  (regenerate with `scripts/build_docs_index.py`). No gateway hook point is
  touched.
* **Follow-ups** ([integration-amendments.json](integration-amendments.json)):
  registered FB-RETRY-LIFECYCLE-BODY-BLOCKERS, FB-RETURN-LINKED-ELEMENT and
  FB-RETRY-CONTRACT-FIXTURES;
  amended W45-FB-OR7-FLUSH-CREATE-OPAQUE, BORROW-ENDING-PRECISION,
  W45-FB-BACKLOG-REACHABLE-RELEASE, W45-FB-USERS-RELEASE-CHECK,
  LERI-HELD-FORMAL-LINK-OBLIGATION, MEMBER-EFFECT-STORES,
  SLOT-MOVE-FIELD-SLOT and W45-FB-OR6-ELEMENT-IDENTITY with the census's
  counts. The 87 class (d) Fluent Bit callees outside the retry lifecycle are
  the FB-SYSTEM-EXTENSIONS inventory's, not tracked here.
