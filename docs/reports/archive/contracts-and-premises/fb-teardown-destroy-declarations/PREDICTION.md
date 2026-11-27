# FB-TEARDOWN-DESTROY-DECLARATIONS: census and frozen prediction

Frozen before any implementation code. Base: local `main` `8b360766c`
(the census itself was read on `cc3e79559`, binary `f25317e5…`; the routes'
custody and null figures there equal the round-30 figures: corpus 938 / 9 /
3,887, null 1,113 / 1,435 / 2,355 / 18; system and slice below). Sources read
only from the prepared exports (`66910c10` corpus tree, `046accf6` system
clone, `90978ac6` export); `scripts/bodies.py` hashes every body read,
`scripts/destroyers.py` lists every destroyer leaf of a route
(`artifacts/destroyers-*-before.json`).

## Decisions in force

* Decision 2 (2026-10-16, "Yes, allow it"): a `when` guard may test a plain
  integer parameter; a known constant actual decides, an unknown value gives
  the cautious join of both cases.
* Rule 30's values (`#released`, `#moved`, `#stale`, `#kept`, `{V, #kept}`)
  are unchanged. No new word, value or form is needed (no pause): the card
  reads the existing states at a **direct call** and admits the decided
  scalar guard beside them. Two placement extensions inside the decided
  spellings, both stated in the spec amendment: the binder may be a formal
  the declaration releases (`resource_released(P, …)`, the destroyer's own
  object), and a parameter-guarded state stands with or without a hand-off.

## Census: the destroyers, their member effects and their callers

Bodies are byte-identical at all three trees unless noted. "Paths" are the
pointer members of the formal's object whose object the body releases
(directly, through `flb_sds_destroy` / a `*_destroy` callee, or through a
followed static helper), per guard value. No destroyer's custody depends on
a scalar parameter: `flb_task_destroy`'s `del` (like
`flb_input_chunk_destroy`'s, already declared) selects only whether
`cio_chunk_close` unlinks the backing file; `task->ic` is destroyed on every
path. The scalar guard therefore has **no Fluent Bit use** in this census;
it is implemented and tested on fixtures.

| Destroyer (route) | Object | Member paths released (guard) | Callers' actuals (leaves before) | Declared? |
| --- | --- | --- | --- | --- |
| `flb_sched_timer_destroy` (system) | `timer` nonnull | none (`event` embedded) | `timer` ×9: locals of `flb_sched_create`, `_request_create`, `_retry_now`, `_timer_cb_create`, `_timer_cleanup`, `_destroy`; formal of `flb_sched_timer_cb_destroy` (18) | yes, plain |
| `flb_sched_request_destroy` (system) | `req` nullable (`if (!req) return 0`) | `timer` handed to `sched->timers_drop` by `flb_sched_timer_invalidate`: `#moved` | `request`/`req`: list entries of `_request_invalidate`, `_destroy`, `_event_handler` (10) | yes, `timer |-> #moved` |
| `flb_task_destroy` (system, corpus) | `task` nonnull | `ic` (`if != NULL`, any `del`), `event_chunk` (`if != NULL`): `#released`; `routes`, `retries` are lists | `task` ×7 (locals of `flb_task_create`, `_retry_reschedule`, `flb_engine_dispatch`; list entry of `flb_engine_destroy_tasks`), `ic->task` / `old_input_chunk->task` (corpus `flb_input_chunk.c`) (18) | yes, two paths, no guard |
| `flb_task_queue_destroy` (system) | `queue` nonnull | none (two lists of entries) | `ins->singleplex_queue`, `instance->singleplex_queue` ×3 (8) | yes, plain |
| `flb_event_chunk_destroy` (system) | `evc` nullable | `tag` (`if`): `#released` | `evc` (`flb_task_create`), `task->event_chunk` (`flb_task_destroy`) (4) | yes, one path |
| `flb_output_flush_destroy` (system, header `static inline`) | `out_flush` nonnull | `coro`: `#released` | `out_flush` ×2 (4) | yes, one path |
| `flb_coro_destroy` (both, header `static inline`) | `coro` nonnull | `callee` (`if != NULL`, `co_delete`): `#released` | `instance->coro`, `input_coro->coro` (4) | yes, one path |
| `flb_input_chunk_release_route` (system) | none released | none | `task->ic` (2) | yes, `resource_borrowed` both |
| `flb_output_instance_destroy` (system) | `ins` nonnull | 20 pointer members and the TLS strings of `flb_output_free_properties` (046accf6 adds `tls_crl_file`; the member does not exist at 66910c10): `#released`; `tls` only when `use_tls`, `singleplex_queue` only under `FLB_OUTPUT_SYNCHRONOUS`: `{#released, #kept}` | `ins` ×16 (`flb_output_init_all` 13, `_exit`, `_enable_multi_threading`, `_net_property_check`) (32) | yes, pinned at 046accf6 and 90978ac6 |
| `flb_input_instance_destroy` (slice; 66910c10 differs by `tls_crl_file`) | `ins` nonnull | the pointer members it destroys (`alias`, `host.*`, TLS strings, `tag`, `cmt`, `metrics`, config maps, `http_server_config`, three chunk hash tables, …): `#released`; `tls` only when `use_tls`: `{#released, #kept}` | `ins` ×9 (18) | yes, pinned at 046accf6 and 90978ac6 |
| `flb_input_coro_destroy` (slice, `static`) | `input_coro` nonnull | `coro`: `#released` | `input_coro` (2) | yes, one path |
| `flb_input_collector_destroy` (slice) | `coll` nonnull | none (`event` embedded) | `collector`, a list entry (2) | yes, plain |
| `flb_tail_file_remove` (slice; 90978ac6 adds `flb_tail_file_budget_release`) | `file` nonnull | `decompression_context`, both encoders, `mult_sbuf.data`, `dmode_buf`, `dmode_lastline`, `tag_buf`, `buf_data`, `name`, `orig_name`, `real_name`, `hash_key`: `#released` | `file` ×16, `f` (17) | yes, pinned at 90978ac6 |
| `flb_tail_config_destroy` (slice; 90978ac6 adds the budget, dormant and aged-out tables) | `config` nonnull | `file_budget`, `ml_ctx`, `tag_regex`, `db`, six hash tables: `#released`; the five SQLite statements only when `db` is set: `{#released, #kept}` | `ctx` ×39 (`flb_tail_config_create` 28 on a tracked `flb_calloc` result, `in_tail_init` 9, `in_tail_exit`, `flb_tail_fs_inotify_init`) (39) | yes, pinned at 90978ac6 |
| `flb_task_retry_clean`, `flb_sched_request_invalidate`, `flb_output_exit`, `flb_input_exit_all`, `flb_engine_destroy_tasks`, `flb_tail_file_remove_all` | — | each destroys **list entries** reachable from a borrowed formal (a retry, a request, every instance, every task, every file); v1 states no list custody, so `resource_borrowed` would hide a release of an object a caller may still track | 18 system, 4 slice | **no** (unfaithful as borrowed) |
| `flb_task_users_release` | — | may destroy its borrowed task (`users == 0 && no retries`) | 4 | **no** (`W45-FB-USERS-RELEASE-CHECK`) |

The 30 system `unresolved resource identity` leaves (`flb_input_chunk_destroy`
16, `flb_task_retry_destroy` 14) are actuals the projection does not bind:
list entries and formals. Every **formal** actual of a release in these routes
already reads `unresolved resource identity` (`flb_task_queue_destroy`'s own
`flb_free(queue)`, `flb_task_retry_reschedule`'s `flb_task_retry_destroy(retry)`):
the reported run does not bind a caller's object. So a declaration turns a
`call to uncontracted function` leaf at a formal or list-entry actual into an
`unresolved resource identity` leaf, and decides nothing there.

## Prediction

1. **Mechanism (fixtures).** A direct call of a destroyer whose formal's
   object is tracked and whose member links a tracked object: `#released` /
   `#moved` release the linked object once and drop the link (no leak at the
   exit, a second release is the ordinary double-release candidate);
   `#kept` leaves it linked and live (P-B08-SHALLOW, as today); `#stale`
   takes it and keeps the link, and a later release of it refuses
   `member-path-double-release`; a set, an undecided guard's join and a path
   below a stated one refuse; a constant actual selects its guard's class.
2. **Corpus** (`flb_input_chunk.c` is the only Fluent Bit unit): no status
   moves. `flb_input_chunk_release_space` and `memrb_input_chunk_release_space`
   trade their `flb_task_destroy` leaf (a member-read actual, `x->task`) for
   `unresolved resource identity`; both stay Unknown (20 and 3 leaves).
   The contract corpus pins grow by the new module.
3. **System**: **0 Safe gains.** The three rows whose every leaf is a
   destroyer call (`flb_engine_destroy_tasks`, `flb_sched_request_invalidate`,
   `flb_sched_timer_cb_destroy`) keep one leaf each, now `unresolved resource
   identity` (a list entry, a list entry, a formal). The other rows trade
   leaves the same way; rows with a tracked owner (`flb_task_create`'s task,
   `flb_sched_*_create`'s timer) may lose or gain leak and link leaves near
   the destroy calls. No new Unsafe is predicted; any is read and named.
4. **Slice**: **0 Safe gains.** `flb_tail_file_remove_all` keeps two
   `unresolved resource identity` leaves (list entries).
   `flb_tail_config_create` (205 leaves) is the one row where rule 30's
   direct call reads linked members (its `ctx->…` stores of hash tables and
   the budget before 28 error-path destroys); it stays Unknown.
5. **Null property**: unchanged on all three routes (the declarations add no
   null-deref clause).
6. **Obligations** OB-10, OR-2…OR-6, OR-8…OR-11: leaf-level movement only in
   their functions (`flb_sched_request_create`, `flb_sched_event_handler`,
   `flb_sched_request_invalidate`, `flb_task_retry_destroy`,
   `flb_output_instance_destroy`'s callers, `flb_engine_start`'s callees); no
   obligation becomes decided.
