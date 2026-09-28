# W47-FLB-LIB-LIFECYCLE: prediction (frozen before any code, contract or proof change)

Base: local `main` at `ec4c9d4a` (the worktree's HEAD; no reset needed).
BEFORE census at the base reproduces the archived census byte for byte
(`fixed.json` `5195d5f8…`, `previous.json` `fdd6dbe6…`). The arbiter BEFORE
runs are in flight when this is written.

The user's statement of the API (2026-09-29), to be **checked**, never
trusted: the library lifecycle is `flb_start` → `flb_stop` pairs; `flb_stop`
any number of times has no effect after the first; `flb_start` twice or more
without stopping is not permitted.

## The contract as it will be written

`fluent_bit/lib_engine_thread.qv` gains one clause per lifecycle function,
`requires resource_lifecycle(ctx, start|stop|destroy);` (a new obligation,
contract specification C04.11: a paired start/stop state machine over the
context, never applied on a per-unit route), and a new declaration
`flb_destroy` (custody held for review like the others). Six clauses, per
body per tree:

| Clause | Kind | Predicted at `90978ac6` export, `046accf6`, `66910c10` |
|---|---|---|
| L1 `start-requires-unstarted`: `flb_start` only from created or stopped | caller precondition | in force (a restriction: it can only refuse); a second start would spawn a second engine thread over `config->worker` (:969-973), so it is load-bearing |
| L2 `stop-ends-started-engine`: the first stop after a start that returned 0 joins the thread that start spawned, once | effect, body-checked | **satisfied** on every feasible path (`:1058` or `:1077`; the `!ctx->config` arm stays infeasible under P-DEREF) → in force; held instead if the stop's run meets a hop that may start or join a thread |
| L3 `stop-after-stop-no-effect` | effect, body-checked | **refuted**: every feasible path of `flb_stop` joins `ctx->config->worker`, which no path clears, so a second stop joins the joined thread again |
| L4 `stop-after-failed-start-no-effect` | effect, body-checked | **refuted**: after `do_start`'s failure returns that joined (:984, :999) the stop's join is a second join; after a failure before the spawn (`flb_fips_init`, the spawn) it joins an identifier that start never spawned |
| L5 `stop-unstarted-no-effect` (the contract's answer for a never-started context) | effect, body-checked | **refuted**: the stop joins `config->worker`, never written by a start |
| L6 `destroy-requires-unstarted`: `flb_destroy` only from created or stopped | caller precondition | in force; `flb_destroy`'s body joins no thread on any path, so it is load-bearing |

Bodies: `flb_start`, `do_start`, `flb_stop` identical at the three trees (the
stop-paths card's `bodies.py`); `flb_destroy` predicted identical too. So the
per-tree table is predicted identical. Refuted clauses (L3-L5) are withdrawn
from every caller (rule 10g): the census never applies them and reads the
body; each is a **candidate** only (`W47-ENGINE-THREAD-JOIN-CANDIDATES` 1 and
3), missing a native reproduction.

## The tests against the contract

* Fixed test (`90978ac6`, `flb_test_inotify_pause_collectors`): on every path
  created → one `flb_start` → `flb_stop` on both start outcomes → `flb_destroy`
  after the `exit` guard. **Conforms**: no double start, no destroy before stop.
* Old test (`90978ac6~1`): start once, stop through `test_tail_ctx_destroy`,
  destroy. **Conforms** (its C5 failure is the `unlink` before the stop).
* Whole census: **0** lifecycle violations in either revision's roots. A stop
  on a never-started context occurs (`flb_test_db`'s second context,
  `write_msg` failure before its start): permitted, governed by L5, refuted,
  so read from the body. Calls inside `flb_create` (its own error path on a
  context no create assigned) are reported unresolved, gating nothing.

## C5 under the in-force clauses

* Fixed test's five releases after `flb_stop` (`flb_destroy` :2588, `unlink`
  :2591, `pthread_cond_destroy` :2595, `pthread_mutex_destroy` :2596, scope end
  :2597) and C4 `cond_destroy` :2595: **not discharged**, reason unchanged
  (`flb_lib_worker` joined a second time, :1058): the failed-start path is
  governed by L4, which is refuted and withdrawn, so the body's repeated join
  stands. Even with L4 in force the status-branch first join would stay
  unchecked (busy, cycle), so no clause can discharge it.
* Old test: `unlink` :286 and `flb_destroy` :293 **not discharged** (unchanged).
* Census tallies unchanged at both revisions (fixed 97 discharged / 134 not
  discharged; previous 97 / 131); every existing release and site byte for
  byte; a new `lifecycle` section is added.
* Negative controls (fixtures): double start → refused; stop joining again
  after a failed start → L4 refuted, C5 not discharged; stop on a
  never-started context → L5 decided from the body; destroy before stop →
  refused.

## Corpus, system, slice

* Module counts: corpus modules 122 and selected 58 unchanged; v1
  declarations 251 → 252; pending review 9 → 10; the clause table's
  `flb_start`/`flb_stop` rows 4 → 5 and a new `flb_destroy` row.
* Per-unit routes: a call of a lifecycle function is refused there
  (`lifecycle__` mark under the reserved prefix), behind the review hold that
  already applies first. Predicted **0 status moves** on all three routes;
  leaves reword only at calls of `flb_destroy` (to `awaiting-human-review`),
  in the slice at least `flb_create` (Unknown already). No Safe caller of any
  of the three, so no Safe → non-Safe move and no correction. No leak or
  violation added or removed.
* Pins: `flb_start`, `flb_stop`, `flb_destroy` re-pinned at the three trees;
  `pin --check` 0 at `66910c10` and `046accf6`; at the export as before
  (pre-existing unrecorded entries) unless the base lock changed.

## Lean

`QCore.CondvarJoin`, new section `Lifecycle`: a clause is in force only if
every body path satisfies it; a refuted clause withdraws and never
discharges; an in-force idempotent stop adds no join; start twice and
destroy before stop are rejected; a lifecycle violation blocks a release; the
fixed test's failed-start path and the old test's shape stay not discharged.
