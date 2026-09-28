# W47-CONDVAR-JOIN-PUBLICATION: frozen prediction

Frozen before any kernel, contract, loader, gateway or route change. Base:
local `main` at `51fb793c` (W47-LOOP-OWNER-CONTRACTS landed); this worktree's
HEAD was already there. The caller census below
([`scripts/callers.py`](scripts/callers.py) over the loop-owner card's AFTER
archives, `private/cj/callers-before/`) is the call-graph evidence the
predictions rest on.

Nothing here is a verified race or a verified fix. Every old-vs-fixed row says
whether an obligation is discharged, not discharged or unresolved **at this
path under these derived premises**.

## Mechanism being predicted

1. **Contracts** in `quirin-provider-libc` (all flexible, rule 10j),
   module `libc/pthread_sync.qv`: `pthread_cond_wait`, `pthread_cond_timedwait`,
   `pthread_cond_signal`, `pthread_cond_init`, `pthread_cond_destroy` and
   `pthread_join`, each with a new call-site precondition
   `requires resource_thread_sync(...)` naming its role (a wait names its
   condvar and mutex; the others their condvar or thread). Custody is exact
   and neutral (every pointer borrowed, scalar result); `pthread_join` is
   declared only where its result slot is the null pointer constant
   (`resource_literal_argument(retval, 0)`). **`pthread_create` stays
   undeclared** (rule 10j: it retains `arg` and runs a callback, neither of
   which v1 states); the census reads it as the spawn hop it already is.
   `pthread_cond_broadcast`, C11 `cnd_*` (their custody declarations are
   unchanged), rwlocks and try/timed locks are not touched. No Fluent Bit–side
   contract is added: `flb_start`/`flb_stop` have analysed bodies and are
   read by the census, never declared.
2. **Per-unit route** (corpus, system, slice): no route derives a thread, a
   predicate loop or a join, so the obligation is never discharged there.
   Every call of a declared callee is rewritten under the literal premise's
   reserved prefix with a `thread_sync__` mark and refused (`call to X outside
   its declared thread-synchronization obligation`), exactly as rule 10p's
   loop-owner refusal. The refusal is category `unsupported`, as both refusals
   it replaces are (`call to uncontracted function pthread_join`, and the
   typed-access gate's `indirect memory access is outside the resource
   projection` at `&x->cond` actuals).
3. **Census route** (the only consumer that can discharge): a lexical
   statement reader and path interpreter over one program's files (the
   loop-owner census's inventory, reused for thread contexts and spawns)
   decides, per site: C4 (a wait only in a predicate loop under its mutex; a
   signal only under that mutex; results written before the predicate in one
   critical section; a guarded read only where the predicate was observed
   under the mutex; `init`/`destroy` only in a quiescent phase) and C5 (a
   release of storage another thread may retain only after a checked,
   successful join of every live thread; publication, a timeout and an
   unchecked or failed join grant nothing). Kept refusals refuse.

## Predictions

**Corpus (185 units, `66910c10`).** No status or count moves. The 6 call
sites of declared callees are all in Unknown functions (Monkey:
`mk_clock_exit` `pthread_join` `mk_clock.c:133`, `mk_stop` `mk_lib.c:290`,
`mk_sched_launch_worker_loop` `pthread_cond_signal` `mk_scheduler.c:407`,
`mk_sched_launch_thread` `pthread_cond_wait` `:456`, `mk_sched_init`
`pthread_cond_init` `:491`, `mk_sched_workers_join` `pthread_join` `:917`);
where the statement is reached, its leaf rewords to the new refusal.
Units 6 / 8 / 167 / 4, functions 526 / 10 / 4,298, leak 36, unknown_alias
7,936, unsupported 35,737 unchanged. No Safe → non-Safe move (no Safe
function calls a declared callee), no leak or violation added.

**System slice (`046accf6`).** No analysed function calls a declared callee
(the census finds only two `flb_start` calls, not declared). Byte-identical
verdict-bearing output; unique statuses 31 / 2 / 201.

**Tail-pause slice (`90978ac6` export).** No unit or function status moves
(units 0 / 1 / 15; functions 24 / 1 / 245 / 1 not projected). The 7 bound C4/C5
functions in the slice (`do_start`, `flb_start`, `flb_stop`,
`flb_input_collector_destroy`, `flb_input_thread_instance_exit`,
`input_thread_instance_set_status`, `input_thread_instance_get_status`) stay
Unknown. Leaf rewordings at the declared-callee sites that are reached:
`pthread_join` in `do_start` (×2), `flb_stop` (×2) and
`flb_input_thread_instance_exit`; the condvar calls in
`input_thread_instance_{create,set_status,get_status}` and
`flb_input_ingress_primitives_{init,destroy}` / `flb_input_instance_destroy`,
each in both variants where compiled twice. unknown_alias 1,009, unsupported
2,857, counted_user 1 unchanged.

**Census route, `90978ac6~1` (previous test) against `90978ac6` (fixed).**

| Site / case | `90978ac6~1` | `90978ac6` |
|---|---|---|
| C4 publisher `cb_tail_pause_test` (B41/B42) | absent | **discharged**: signal under `tail_pause_test.mutex`; every result written before `done` in the same critical section |
| C4 awaiter wait (B45) | absent | **discharged**: timed wait in a loop rechecking `done` under the mutex; a nonzero return breaks |
| C4 result reads (B45) | absent | **discharged**: each read is inside `if (TEST_CHECK(test.done))` under the mutex (premise P-CHECK: acutest's check macro is its condition's truth, read from `acutest.h`) |
| C4 test `cond_init` `:2550` | absent | discharged: before publication (`ins->data = &test`) and before any spawn |
| C4 test `cond_destroy` `:2595` | absent | **not discharged**: its quiescence rests on the join, which R2 leaves unchecked on `flb_stop`'s status branch |
| C5 releases after `flb_stop` (B47: `flb_destroy`, the fixture `unlink`, `test`'s destroys and scope end) | absent | **not discharged**: `flb_stop` returns 0 on its status branch after an unchecked join and on its `!ctx->config` branch with none (R2); in addition the engine's own worker joins are not derived (B51 is unchecked), which alone would leave it unresolved |
| NC4 (publication is not join) | not applicable | absent; a fixture releasing after `done` and before any join is **rejected** |
| NC5 (timeout is not cancellation) | not applicable | absent (the timeout takes the stop/join path); a fixture releasing on the timeout path is **rejected** |
| NC8 (wake before publication) | not applicable | handled: the loop rechecks, no result claim before `done` under the mutex; a fixture with a single wait and an unlicensed read makes **no result claim** |
| NC9 (fixture removed before join) | **present, not discharged** in both pause tests' teardown (`test_tail_ctx_destroy`: `unlink` `:286` before `flb_stop` `:292`, result ignored) | absent from the non-threaded test; **present, not discharged** through the threaded test's teardown |
| Worker handshake `input_thread_instance_set_status` (B52) | discharged | discharged (identical blob) |
| Worker handshake wait `input_thread_instance_get_status` (B53) | discharged | discharged |
| B54 re-init after unlock and the unlocked read `:719` | discharged only under the **derived** single-signal premise (every signal of `init_condition` is `set_status`'s, called only from the thread entry `input_thread`, at most once per path); refused otherwise | same |
| Kept refusals (broadcast, a wait outside a predicate loop, a signal without the mutex, one condvar with two mutexes, a static initialiser, `cnd_*`, rwlock, try/timed lock) | refused wherever the census reads one | same |

The fixture controls are expected to give: NC2's publish/await/join shape
with a checked join on every path **discharged**; NC4, NC5 and NC9
**rejected**; NC8 **no result claim**; broadcast, a wait without a loop and a
signal without the mutex **refused**.
