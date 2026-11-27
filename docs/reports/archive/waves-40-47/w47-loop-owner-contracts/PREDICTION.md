# W47-LOOP-OWNER-CONTRACTS: frozen prediction

Frozen before any contract, loader, gateway or route change. Only the
source-independent kernel (`crates/qcore-core/src/loop_owner.rs`, no caller)
existed when this was written; it cannot move any measurement. Base: local
`main` at `8eba46d1`.

BEFORE reproduces all three baselines (0 mismatches; `private/lo/before/`):
corpus 6 / 8 / 167 / 4 units, leak 36, unknown_alias 7,936, unsupported
35,737; system unknown_alias 1,020, unsupported 2,888; tail-pause slice units
0 / 1 / 15, unknown_alias 1,009, unsupported 2,857, counted_user 1.

## Mechanism being predicted

1. A resource-v1 precondition `requires resource_loop_owner(p, KIND);` on the
   four collector operations `flb_input_collector_pause`, `_resume`,
   `_running` and `_delete`, in a new `fluent_bit/loop_owner.qv`. Their
   bodies are byte-identical at `66910c10`, `046accf6` and `90978ac6`
   (`scripts/bodies.py`). The custody part is exact: `coll_id` and `in` are
   borrowed, no user count changes, the result is a scalar.
2. On every per-unit resource route (corpus, system, slice) no derived thread
   set exists, so the obligation is **never discharged there**: every call to
   a declared operation is refused, exactly as an uncontracted call is today.
3. Registration (`flb_input_set_collector_*`) and the Monkey `mk_event_*`
   operations are **not** declared: registration retains a callback and
   `mk_event_add`/`timeout_create` retain the caller's event (rule 10j keeps
   both undeclared). `flb_input_pause`/`resume` dispatch a plugin callback
   (hop V) and stay undeclared for the same reason; C2 lives in the kernel's
   completion vocabulary, not in a v1 contract.
4. A lexical thread-entry census over the in_tail runtime test binary's
   sources derives each function's contexts and decides the obligation per
   operation site. That is the only route that can discharge it.

## Predictions

**Tail-pause slice.** No function or unit changes status: units stay
0 / 1 / 15, functions 24 Safe / 1 provisional Unsafe / 245 Unknown / 1 not
projected. No Safe function calls a declared operation (the census
`scripts/callers.py` finds 42 call sites of loop operations or the public API
in the slice, all in Unknown functions), so the refusal can only reword a
leaf, never lose a Safe. Expected leaf rewording at the declared-operation
call sites in `tail.c` (`in_tail_pause` ×4, `in_tail_resume` ×4),
`tail_fs_inotify.c` (×4), `tail_fs_stat.c` (×4) and `flb_input.c`
(`flb_input_collector_delete` → pause), where the call is reached before an
earlier refusal. The four defining functions in `flb_input.c` gain an own
contract and stay Unknown (their bodies call the undeclared `mk_event_*`).
Every bound function in the slice stays Unknown.

**185-unit corpus.** The three `flb_input_chunk.c` sites at `66910c10` are
`flb_input_chunk_set_limits` `:3018` (`flb_input_resume`) and
`flb_input_chunk_protect` `:3050` and `:3085` (`flb_input_pause`). All three
functions are Unknown today on `call to uncontracted function
flb_input_resume/pause`. They do not move: the public API stays undeclared.
No corpus unit defines or calls a declared collector operation, and the 27
corpus call sites of loop operations are in Unknown (25) or provisional-Unsafe
(2, `mk_net_conn_create`) functions calling undeclared `mk_event_*`. So the
verdict-bearing corpus output is **byte-identical**; only the contracts
sidecar changes (one module more).

**System slice at `046accf6`.** `flb_engine_shutdown` `:1671` and
`flb_engine_stop_ingestion` `:1746` call the undeclared `flb_input_pause_all`
and stay Unknown. `in_storage_backlog/sb.c` calls declared operations
(`cb_sb_pause` `:784`, `cb_sb_resume` `:790`, `cb_sb_exit` `:797`); all three
are Unknown and stay so, with at most a reworded leaf. Unique statuses stay
31 / 2 / 201.

**Negative cases, `90978ac6~1` against `90978ac6`** (census route):

| Case | `90978ac6~1` | `90978ac6` |
|---|---|---|
| NC1 | not discharged: the previous test's T path through `flb_input_pause` (non-threaded fork) → `in_tail_pause` → collector pause | path absent; the collector pause in `in_tail_pause` is still not discharged, through the threaded test's T call of `flb_input_pause` whose threaded flag the census cannot fix (F without configuration flow) |
| NC2 | absent | the E path (engine dispatch of `cb_collect`, V* followed as a field registration of `cb_pre_run`) discharges per path; the site as a whole may stay unresolved if another `cb_collect` dispatcher (the coroutine collector in `flb_input.h`) is an unresolved hop |
| NC3 | not discharged (threaded test's T poll of `running`) | not discharged (unchanged test) |
| NC6 | not reachable in the tests; the kernel refuses the claim on a fixture | same |
| NC7 | no conclusion (both results discarded) | same |

Nothing here is a verified race or a verified fix: each row says whether the
owner obligation is discharged at that path under the derived premises.
