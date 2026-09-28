# W47-CONDVAR-JOIN-STOP-PATHS: frozen prediction

Frozen before any kernel, census, contract, loader, gateway or proof change.
Base: local `main` at `fefb9199` (W47-LOOP-OWNER-TYPED-DISPATCH integrated
after the condvar/join card); this worktree's HEAD was already there.

**BEFORE reproduces the census baseline byte for byte**:
`docs/reports/w47-condvar-join-publication/scripts/census.sh` into
`measure/census-before/` (build, fixed and previous runs exit 0) gives
`fixed.json` SHA-256 `7a4f66d5…` and `previous.json` `251f7d9e…`, the
manifest values of `docs/reports/w47-condvar-join-publication/artifacts/census/`,
and `tests.sha256` identical to the archived one.

Nothing here is a verified race, a verified bug or a verified fix. Every
old-vs-fixed row says whether an obligation is discharged, not discharged or
unresolved **at this path under these derived premises**.

## What was read before predicting

`src/flb_lib.c` at the `90978ac6` export (`flb_create` :145-228,
`flb_lib_worker` :917-933, `do_start` :944-1008, `flb_stop` :1040-1084),
`mk_utils_worker_spawn` (`lib/monkey/mk_core/mk_utils.c:297-328`), the
fixed test (`tests/runtime/in_tail.c:2524-2598`) and every `pthread_join`,
`pthread_detach`, `PTHREAD_CREATE_DETACHED` and `pthread_self` of the census
file set, and the census's derived contexts of the functions that join the
engine thread (a scratch probe, not committed).

Facts the prediction rests on:

* `flb_stop` reads `tid = ctx->config->worker` (:1047) **before** its
  `!ctx->config` test (:1062): the field is dereferenced on every path to
  the test. `ctx->config` is stored once in the program (`flb_create` :174,
  into the object `flb_create` allocated at :163 and has not yet handed to any
  call on a path to the store).
* The fixed test calls `flb_stop` **whether or not `flb_start` succeeded**:
  `if (TEST_CHECK(ret == 0)) { … }` guards only the wait. On `do_start`'s two
  failure returns after the spawn (:980-986, :994-1001) the engine thread is
  joined (result ignored) and `ctx->status = FLB_LIB_ERROR`; `flb_stop` then
  takes its status branch and joins the same identifier again (:1058).
* `ctx->status` is also written by the engine thread itself (`flb_lib_worker`
  :932, `FLB_LIB_NONE`) without synchronization with `flb_stop`'s read
  (:1049), and `do_start` can return 0 without setting `FLB_LIB_OK` (the
  notification loop falls through, :1005); so the status branch is reachable
  on a path where `flb_start` returned 0, and there its join (:1058) is the
  first join of the thread on the path.
* `mk_utils_worker_spawn` sets `PTHREAD_CREATE_JOINABLE` and calls
  `pthread_create` only when that succeeded (:316-318); no
  `pthread_detach`/`PTHREAD_CREATE_DETACHED` in the file set names the engine
  thread's identifier.
* `flb_stop` (hence the join of the `flb_config.worker` identifier) is reached
  on six threads other than the test's (`flb_lib_worker`, `input_thread`,
  `output_thread`, `downstream_worker_thread`, `trace_chunk_pipeline_thread`,
  `mk_sched_launch_worker_loop`: the chunk-trace pipeline's own `flb_stop`).
  The census names a thread by its entry, so it cannot tell that those calls
  join a different engine instance.

## Mechanism being predicted

1. **Branch feasibility (rule 10q amendment).** A null test `!X->F` /
   `X->F == NULL` in a function the path reader inlines is excluded only when
   (a) an earlier statement on every path to it in the same function
   dereferences `X->F` (`X->F->…`, `X->F[…]`, `*X->F`), `X` and `X->F` are
   never assigned and `X` never has its address taken in that function, and
   (b) every store of field `F` into an object of `X`'s declared type in the
   whole file set is a **pre-publication store** (into a local that holds an
   allocator's result and has reached no call, store or return on a path to
   the store), with no address of that field taken and no whole-object write
   of that type; a store whose base type the reader cannot resolve fails the
   derivation. Named premise: undefined behaviour (a null dereference) ends
   the analysed execution. Otherwise the branch stays.
2. **Joins (POSIX).** The path interpreter records, per path, whether a join
   is the **first** join of a thread the path started (and through which
   spawn wrapper) or a **repeated** join of a thread already joined on the
   path. A repeated join is never success (undefined behaviour). An unchecked
   first join counts as successful only when each POSIX error case is
   derived impossible under the named premise P-POSIX-JOIN (`pthread_join`
   returns 0 unless the target is not joinable, is already joined or awaited
   by another joiner, or the join deadlocks: the caller itself or a join cycle
   through it): **joinable** (the wrapper's body creates it joinable and no
   detach in the file set may name it), **not busy** (no join site that may
   name it runs on any thread other than the test's), **not self** (the path
   started it) and **no cycle** (no function running on the test's thread
   publishes `pthread_self()`). Anything underived keeps "not checked".
3. **Descendant joins.** For each live retaining thread at a release, each
   thread it starts (transitively) is decided by running the parent's entry
   through the path reader to its exit: joined (checked, or derived under 2)
   on every path with nothing unresolved, never joined (no join site in the
   file set may name it and every join identity is derived), or unresolved
   with its reason. Only a joined descendant lets a release discharge.
4. **Lean** (`QCore.CondvarJoin`, audited in `AuditCondvarJoin.lean`): the
   excluded branch needs its derivation, an unchecked join counts only with
   every POSIX case excluded, a repeated join never counts, a descendant
   blocks release until its derived join, and the old test's shape stays not
   discharged.
5. **Contracts** (the integrator's added step): the observed `flb_stop` /
   `flb_start` / shutdown-join facts are recorded as Fluent Bit contracts in
   `quirin-provider-fluent-bit`. The per-branch join behaviour is not a v1
   custody fact, so it is recorded under the deferred-review mechanism
   (withheld, `awaiting-human-review`, never Safe); the body identity is
   pinned at the revisions where the body was read. No function they declare
   is defined or called in a corpus unit, and no system or slice selection
   gains the module.

## Predictions

**The `!ctx->config` branch (`flb_stop` :1062-1064):** derived infeasible on
every path (both conditions of 1 hold). The `Absent … returned from
flb_stop at src/flb_lib.c:1063 without joining` evidence disappears from every
release that had it, at both revisions.

**The status-branch join (:1058), first join on the path:** **not derived
successful**: joinable, not-self and no-cycle are derived; **not busy is
not** (the six other threads above reach `flb_stop`'s join of the same
entry-named identifier). It stays "not checked".

**Repeated joins:** every path through `do_start`'s failing joins (:984,
:999) followed by `flb_stop` (:1058 or :1077) is a repeated join. This is the
fixed test's new first-ranked blocker.

**Fixed test (`90978ac6`), `flb_test_inotify_pause_collectors`:** its five C5
releases (`flb_destroy` :2588, `unlink` :2591, `pthread_cond_destroy` :2595,
`pthread_mutex_destroy` :2596, scope end of `test` :2597) stay **not
discharged**; the reason moves from `released before any join … (returned
from flb_stop at src/flb_lib.c:1063 without joining)` to the repeated join of
`flb_lib_worker` at `src/flb_lib.c:1058`. The C4 `cond_destroy` site :2595
stays **not discharged** with the same new reason. The other C4 sites of the
test (init :2550, wait :2565, the six reads) are unchanged (discharged).

**Old test (`90978ac6~1`), negative control:** `unlink` :286 stays **not
discharged** (`released before any join … (no join on this path)`) and
`flb_destroy` :293 stays **not discharged** (a failed join at :1077 whose
nonzero `flb_stop` result the old test does not check, and the repeated
join). Nothing of the old test becomes discharged or unresolved.

**Descendants:** `flb_lib_worker` keeps 4 retaining descendants at the
non-engine releases and 9 at `flb_destroy`; each stays **unresolved**, now
with a per-descendant reason (predicted: the engine thread's path from its
entry to its exit is not read, because the engine loop's body is not a
statement tree the path reader reads). None is derived joined.

**Whole census tallies:** unchanged at both revisions (fixed: releases 97
discharged / 134 not discharged; previous: 97 / 131; every C4 tally
unchanged). Release reasons reword where the `:1063` path or a `do_start`
join was on a path.

**Candidates (upstream, not verified):** (i) `flb_stop` after a `flb_start`
whose engine thread already failed and was joined in `do_start` joins that
thread a second time (`src/flb_lib.c:984`/`:999`, then `:1058`); (ii) the
unsynchronized `ctx->status` write by the engine thread (:932) against
`flb_stop`'s read (:1049); (iii) `flb_stop` after a `flb_start` that failed
before spawning joins an identifier `do_start` never wrote (`:1047`, `:1058`).
Each needs a native reproduction to be more than a candidate.

**Corpus (185 units, `66910c10`), system (`046accf6`) and tail-pause slice
(`90978ac6`):** no census change reaches a per-unit route; **0 status and 0
leaf moves** on all three (corpus 526 / 10 / 4,298, system 31 / 2 / 201,
slice units 0 / 1 / 15 and functions 24 / 1 / 245 / 1). The corpus contract
sidecar lists one more module if the resource-view union puts the new
module in `cycle19-resource-contracts.json`.
