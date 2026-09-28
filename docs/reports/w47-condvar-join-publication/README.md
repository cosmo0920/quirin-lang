# W47-CONDVAR-JOIN-PUBLICATION: condvar publication and join before release (C4–C5)

Card W47-CONDVAR-JOIN-PUBLICATION (wave 47, in_tail track; card-high),
gateway rule **10q**, contract specification **§C04.10**, constraint 27 as
amended. The specification is
[W47-TAIL-PAUSE-BINDINGS](../w47-tail-pause-bindings/README.md) §6. Base: local
`main` at `51fb793c` (W47-LOOP-OWNER-CONTRACTS landed); this worktree's HEAD
was already there. Frozen prediction: [PREDICTION.md](PREDICTION.md) (commit
`41b2092f`, before any kernel, contract, loader, gateway or route change).

Nothing here is a verified race, a verified fix, an `Unsafe` claim or an
upstream defect claim. Every old-vs-fixed statement reads: *the obligation is
(not) discharged, or unresolved, at this path under these derived premises*.

## 1. Mechanism

Five parts, each fail-closed; new logic lives in new modules with thin hooks.

1. **Kernel** `crates/qcore-core/src/condvar_join.rs` (source-independent).
   `wait_verdict` admits a wait only as the body of a loop that rechecks a
   predicate under the mutex it releases, with that mutex held (a timed wait
   may `break` on a nonzero return); `signal_verdict` admits a signal only
   under the condvar's paired mutex; `publication_verdict` requires the
   results before the predicate in the signalling section; `read_verdict`
   licenses a guarded read only by the predicate observed under the mutex, or
   by a derived single publication; `quiescence_verdict` admits `init`/`destroy`
   before publication, after the last signal (derived single-signal premise)
   or after a covering join; `release_verdict` (C5) discharges only when every
   path of every live thread saw a join whose zero result was checked.
   `waiter_step`/`run_waiter` are the waiter's rule (NC8). Kept refusals are
   `KeptRefusal`. Verdicts compose `Discharged < Unresolved < NotDischarged <
   Refused`; only a discharge admits. `publication_trace` lowers the
   publish/await/join shape into a Cycle 14 `ConcurrentProgram` (a wait is the
   unlock and relock of its mutex): `check_concurrent`'s vector clocks accept
   it, report a data race when a read skips the mutex, and a non-quiescent
   destruction when the release precedes the join (the reuse of concurrent14).
2. **Vocabulary** (`contract_loader.rs`, §C04.10): `requires
   resource_thread_sync(cond, mutex, cond_wait|cond_timedwait)`,
   `(cond, cond_signal|cond_init|cond_destroy)` or `(thread, join)`; a condvar
   and a mutex are distinct `ptr<mut, u8>` parameters, a thread a `u64`/`usize`;
   any other spelling, a second clause, a wrong parameter kind or a review
   marker around it rejects the module.
3. **Contracts** `providers/quirin-provider-libc/contracts/libc/pthread_sync.qv`
   (flexible, rule 10j): `pthread_cond_{init,destroy,signal,wait,timedwait}`
   and `pthread_join`, custody neutral (every pointer borrowed, scalar
   result), `pthread_join` only at a null result slot
   (`resource_literal_argument(thread_return, 0)`). `pthread_create` stays
   undeclared (a retained `arg` and a callback, rule 10j); the census reads it
   as the spawn hop. Broadcast, `cnd_*` (their `threads.qv` custody
   declarations unchanged), rwlocks and try/timed locks are untouched. No
   Fluent Bit–side contract: `flb_start`/`flb_stop` have analysed bodies and
   the census reads them, so nothing needed pinning in
   `quirin-provider-fluent-bit`. The libc lock gains six flexible entries
   (`pin --verify-contracts` 0).
4. **Per-unit route hook** (`legacy_literal_premise.rs`): every call of a
   declared callee is rewritten under the literal premise's reserved prefix
   with a `thread_sync__` mark and refused as `call to X outside its declared
   thread-synchronization obligation (no derived thread, predicate loop or
   join on this route)`, category `unsupported`, as rule 10p's.
5. **Census**: the lexical reader `crates/qcore-c-gateway/src/condvar_census.rs`
   and its adapter `crates/quirin-source-analysis/src/condvar_join_census.rs`
   (the gateway depends on no kernel crate), run by
   `examples/condvar_join_census.rs` and [`scripts/census.sh`](scripts/census.sh)
   over the loop-owner census's file set. It reuses the loop-owner census's
   reading and `qcore_core::loop_owner::derive` for thread contexts, spawns
   and descendants rather than duplicating them. Bodies become statement
   trees (`goto`, `switch`, a label or a conditional group splitting a
   statement leaves the body unread); a must/may lockset dataflow with
   per-mutex predicate observations decides C4 per site; a path interpreter
   runs every test-table root on T, inlining every function whose direct
   calls reach a spawn, a join or a release, tracks each started thread's
   join state per path (checked success, unchecked, failed, none) and the
   identifiers stored in fields, and decides C5 at every release (`unlink`,
   `flb_destroy`, the destroy or scope end of a local whose address the root
   stored into a field). A joined thread's derived children are unresolved
   unless joined; a call that may start a thread through a hop the path
   reader does not follow is unresolved.

**Decision on B54** (`input_thread_instance_get_status` destroys and
re-initialises its condvar after the unlock, and reads `init_status` after
it): admitted only under a **derived** single-signal premise, never a
declared one. The census derives it: every signal of
`flb_input_thread_instance.init_condition` is in `input_thread_instance_set_status`,
whose only caller is the thread entry `input_thread` (a spawn target the
census never calls directly), which calls it three times with the first two
immediately followed by `return` and none in a loop. When that derivation
fails the re-init and the unlocked read are refused (fixture
`a_second_signal_on_a_path_withholds_the_reinit`).

## 2. Negative controls (fixtures first)

`crates/qcore-core/tests/unit/condvar_join.rs` (13 tests) and
`crates/quirin-source-analysis/tests/condvar_join_census.rs` (14 tests, C
fixtures of the bound shapes run through the census, the production module and
the per-unit route):

| Case | Outcome |
|---|---|
| NC2 (publish/await/join with a checked join) | **discharged**: signal, wait, init, reads, both destroys and every release |
| NC4 (publication is not join) | **rejected**: `publication is not a join of engine_worker`; the condvar's own `destroy` is admitted after the derived last signal, which grants nothing to the storage |
| NC5 (timeout is not cancellation) | **rejected**: `a timeout is not cancellation`; the timed wait itself is admitted |
| NC9 (fixture removed before the join) | **rejected**: `released before any join` |
| R2 shape (a stop that returns 0 after an unchecked join) | **rejected**: `the join ... is not checked` |
| NC8 (wait without its predicate loop) | the wait **refused** (`wait-outside-predicate-loop`), the result read **not licensed**: no result claim |
| kept refusals | broadcast, a signal without the mutex, one condvar with two mutexes, a static initialiser, `cnd_wait`, `pthread_rwlock_rdlock`, `pthread_mutex_trylock`: **refused** |
| worker handshake (B52–B54 shape) | **admitted** under the derived single signal; a second signal on a path withholds the re-init and the unlocked read |
| results after the predicate; an unreadable body (`goto`) | not a publication; **unresolved** |
| module | six obligations load; eight malformed or duplicate clauses reject the module; the per-unit route refuses `pthread_cond_signal` |

## 3. The previous test against the fix (census over the test binary)

[`scripts/census.sh`](scripts/census.sh) reads the loop-owner census's 1,685
files of the `90978ac6` export (synthetic commit `34155955`) and the same tree
with the previous test (`90978ac6~1`, SHA-256 `008e32fc…`, blob-identical to
`046accf6`'s). Archive: [artifacts/census/](artifacts/census/); the table
below is [artifacts/census-summary.json](artifacts/census-summary.json)
([`scripts/summary.py`](scripts/summary.py)).

| Site / case | `90978ac6~1` | `90978ac6` |
|---|---|---|
| C4 publisher `cb_tail_pause_test` `:2497` (B41/B42) | absent | **discharged**: signal under `tail_pause_test.mutex`; results `pause_result`, `resume_result`, `fs_running`, `progress_running` written before `done` in that section |
| C4 await `:2565` (B45) | absent | **discharged**: timed wait in `while (!test.done)` under the mutex, `break` on a nonzero return |
| C4 reads `:2570`–`:2576` (B45) | absent | **discharged** (6 reads): inside `if (TEST_CHECK(test.done))` under the mutex (P-CHECK) |
| C4 `cond_init` `:2550` | absent | **discharged**: before publication (`ins->data = &test` follows) |
| C4 `cond_destroy` `:2595` | absent | **not discharged**: no single-signal derivation (the callback is a periodic collector, not a spawn entry), so its quiescence rests on the join, which R2 leaves open |
| C5 releases after `flb_stop` (`flb_destroy` `:2588`, `unlink` `:2591`, the two destroys, the scope end of `test` `:2597`; B47) | absent | **not discharged**: `released before any join of flb_lib_worker (returned from flb_stop at src/flb_lib.c:1063 without joining)`, the `!ctx->config` branch; other paths carry `Unchecked` joins at `:1058` (the status branch) and in `do_start` (`:984`, `:999`). Also unresolved: `flb_lib_worker` starts 4 retaining threads whose joins before it ends are not derived |
| NC4, NC5 | not applicable | absent: every release follows `flb_stop` and its result check |
| NC8 | not applicable | handled: the wait loop rechecks; reads only under the observed predicate |
| NC9 (`test_tail_ctx_destroy` `:286` `unlink` before `flb_stop` `:292`) | **not discharged** in both pause tests (`released before any join of flb_lib_worker (no join on this path)`) | absent from the non-threaded test; **not discharged** through the threaded test |
| Worker handshake (B52 `:693`, B53 `:710`, B54 `:715`/`:716`, the read `:719`, the create `:248`) | **discharged** (identical blob) | **discharged** |
| Releases before `flb_start` (`test_tail_ctx_create` `:245`, `:253`, `:262`; `flb_create` `:216`) | discharged (no retaining thread live) | discharged |

So: the fix removes the NC9 path from the non-threaded test and introduces
the NC2 publish/await path, whose C4 obligations all discharge at this path
under the derived premises. Its C5 obligation stays **not discharged**: R2
(`flb_stop` returns 0 on its status branch after an unchecked join, and on its
`!ctx->config` branch with no join) is the named blocker, and the engine's own
children (B51's unchecked worker join among them) would leave it unresolved
even without R2. NC9 remains in the threaded test at both revisions.

**Whole program** (`90978ac6`; the bound files above excepted): 4 waits, 3
signals and 2 reads discharge elsewhere (Monkey's scheduler start, the
Kubernetes filter's service map, the ETW input's startup, the log worker);
kept refusals: 4 broadcasts, 5 rwlock, 8 try/timed lock and 2 recursive-mutex
calls and one lone wait outside a predicate loop (`src/flb_chunk_trace.c:312`);
**not discharged by reading gaps, not findings** (`W47-CONDVAR-JOIN-CENSUS-READING-GAPS`):
four signals whose guarded field is written in a pre-spawn initialisation of
a caller-supplied object or through a base the reader does not key
(`flb_log.c:531`, `mk_scheduler.c:407`, `flb_chunk_trace.c:110`,
`flb_downstream_worker.c:203`) and one wait whose mutex the enclosing loop's
join loses (`flb_downstream_worker.c:593`); 45 sites are unresolved. Over
every test root the census records 231 releases at `90978ac6` (97 discharged,
134 not discharged) and 228 at `90978ac6~1` (97 / 131), nearly all of the
latter through `test_tail_ctx_destroy`'s NC9 shape shared by the other in_tail
tests. None is a defect claim.

## 4. Prediction against outcome

Every predicted figure held ([PREDICTION.md](PREDICTION.md)).

| Measure | Predicted | Measured |
|---|---|---|
| corpus (185 units, `66910c10`) | no status or count move; the 6 Monkey sites reword where reached | **0 status moves**; exactly the 6 sites (`mk_clock_exit`, `mk_stop`, `mk_sched_init`, `mk_sched_launch_thread`, `mk_sched_launch_worker_loop`, `mk_sched_workers_join`) reword `indirect memory access is outside the resource projection` → `outside its declared thread-synchronization obligation`; 6 / 8 / 167 / 4 units, 526 / 10 / 4,298 rows, leak 36, unknown_alias 7,936, unsupported 35,737 unchanged |
| system (`046accf6`) | byte-identical verdict-bearing output | **0 status and 0 leaf moves** (31 / 2 / 201; unsupported 2,888, unknown_alias 1,020) |
| tail-pause slice (`90978ac6`) | no status move; the listed sites reword | **0 unit and 0 function status moves** (0 / 1 / 15; 24 / 1 / 245 / 1); 9 functions reword 13 sites (26 leaves over the static and shared variants) (`do_start` ×2, `flb_stop` ×2, `flb_input_thread_instance_exit`, `input_thread_instance_{create,set_status}` ×1, `_get_status` ×3, `flb_input_ingress_primitives_{init,destroy}`, `flb_input_instance_destroy`), `call to uncontracted function pthread_join` or `indirect memory access …` → the new wording; counts unchanged |
| census, old vs fixed | §3 of the prediction | as predicted (§3) |

**Attribution.** Every moving leaf is a declared-callee call site whose
statement was refused before (uncontracted `pthread_join`, or the typed-access
gate at an `&x->cond` actual) and is now refused first by rule 10q's premise
refusal; the category stays `unsupported`. No function moves; no Safe →
non-Safe correction occurs, because no Safe function calls a declared callee
([`scripts/callers.py`](scripts/callers.py): corpus 6 sites, system 0, slice
all in Unknown functions). **Leaks and violations:** none added or removed.

Artifacts: [artifacts/](artifacts/) — `after/{corpus,system,slice}` (gzipped,
filename-keyed manifest; the new baseline for the next card),
`compare-{corpus,system,slice}.json` ([`scripts/compare.py`](scripts/compare.py)),
`callers-before/`, `census/` and `census-summary.json`. BEFORE reproduces all
three baselines (corpus `resource-diagnostics.json` byte-identical; system and
slice 0 mismatches under `compose3.py`). Binary SHA-256s: BEFORE `5a1b8adc…`,
final AFTER in `artifacts/after/binary.sha256`.

## 5. Lean

`proof/QCore/CondvarJoin.lean`, audited in `proof/AuditCondvarJoin.lean` (30
theorems; no placeholders; axioms only `propext`, `Quot.sound`,
`Classical.choice`):

* publication through the condvar edge: `no_claim_before_publication`,
  `after_publication_the_snapshot_holds`,
  **`observed_done_reads_the_published_snapshot`** (a mutex-serialised
  schedule `pre ++ W :: post`, `W` the only writer of the predicate and the
  results: every prefix that observes the predicate reads the snapshot), and
  on the Cycle 14 lifetime model (`QCore.Concurrency.Lifetime`, extended here
  with the condvar edge: a wait is the waiter's unlock and relock around the
  publisher's section) `the_condvar_edge_is_an_accepted_lifetime_trace` and
  `a_release_before_the_join_is_refused_by_the_lifetime_model`;
* the waiter (NC8): `a_claim_needs_the_predicate`,
  `a_wakeup_without_the_predicate_claims_nothing`, `a_spurious_wakeup_waits_again`,
  `a_timeout_that_leaves_claims_nothing`, `a_run_claims_only_at_an_observed_predicate`;
* join before release: **`join_before_release_leaves_no_access_after_it`**,
  `an_ended_thread_never_acts`, `ended_stays_ended`,
  `an_accepted_release_joined_every_retainer`, `a_timeout_is_not_a_join`,
  `publication_is_not_a_join`, `an_unchecked_or_failed_join_grants_nothing`;
* kept refusals refuse: `every_kept_refusal_refuses`,
  `a_wait_outside_a_predicate_loop_refuses`, `one_condvar_with_two_mutexes_refuses_a_wait`,
  `a_broadcast_refuses`, `a_signal_without_the_mutex_refuses`,
  `a_discharged_wait_rechecks_under_its_mutex`, `a_discharged_signal_holds_the_paired_mutex`,
  `the_per_unit_route_refuses_every_synchronization_call`;
* decisions: `a_discharged_release_saw_a_checked_join_on_every_path`,
  `publication_timeout_or_an_unchecked_join_refuses`, `only_a_discharge_admits`,
  `a_join_is_no_less_severe`, `a_joined_discharge_needs_both`.

## 6. Premises stated, not proved

Trust boundary 28: the lexical reading, the statement reader and the binding
tables (`FLUENT_BIT_SYNC`: spawn wrappers and their identifier out-arguments,
releases, allocators, exits); P-SYNC-INSTANCE; P-JOIN-IDENTITY; P-RETAIN (the
retaining threads of the test's storage and the fixture are those whose
loop-owner-census contexts reach `plugins/in_tail/` or the test file; the
engine context's are every live thread); P-CHECK (acutest's `TEST_CHECK(c)`
is `c`'s truth, `tests/lib/acutest/acutest.h:85`, `:662-722`); acutest's
per-test fork. Each is registered in `W47-CONDVAR-JOIN-PREMISES`. The
coroutine hop of `W47-LOOP-OWNER-COROUTINE-HOP` does not enter this card's
derivation: the callback's contexts are used only to name retaining threads.

## 7. Follow-ups

`followups.json`, applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 547 → 550,
**retiring `W47-CONDVAR-JOIN-PUBLICATION`** (its DISCHARGE clause: the Rust
rule, audited Lean extending `QCore.Concurrency` with a condvar edge and a
join-release theorem, the amendment of constraint 27, the libc contracts with
their lock entries, and the controls) and registering
`W47-CONDVAR-JOIN-DESCENDANT-JOINS` (4 retaining children of the engine
thread; B51), `W47-CONDVAR-JOIN-AUDIT-ROUTE` (6 corpus + the slice call sites),
`W47-CONDVAR-JOIN-CENSUS-READING-GAPS` (5 not-discharged and 45 unresolved
whole-program sites; 251 unread bodies) and `W47-CONDVAR-JOIN-PREMISES`.
`W47-TAIL-PAUSE-BINDING-RESIDUALS` stays open: R2 is bound here with a stated
outcome (the fixed test's C5 blocker), R1 and R3–R6 are not this card's.
`W47-TAIL-PAUSE-TEST-UNIT` stays open: the census reads the test file
directly, the per-unit route still has no compile record for it.

## 8. Gate

[`scripts/gate.sh`](scripts/gate.sh) `rust` then `records`; exits in `gate/`,
figures in [gate.json](gate.json).

## Reproduce

```sh
bash docs/reports/w47-condvar-join-publication/scripts/callers.sh RUNS OUT
bash docs/reports/w47-condvar-join-publication/scripts/measure.sh PHASE
python3 docs/reports/w47-condvar-join-publication/scripts/identity.py ARCHIVE RUN
python3 docs/reports/w47-condvar-join-publication/scripts/compare.py BEFORE AFTER OUT.json
bash docs/reports/w47-condvar-join-publication/scripts/census.sh OUT
python3 docs/reports/w47-condvar-join-publication/scripts/summary.py OUT/previous.json OUT/fixed.json SUMMARY.json
target/release/quirin-audit pin --source-root ROOT --crate quirin-provider-libc \
  --function pthread_cond_init --function pthread_cond_destroy --function pthread_cond_signal \
  --function pthread_cond_wait --function pthread_cond_timedwait --function pthread_join
python3 docs/reports/w47-condvar-join-publication/scripts/update_ledger.py
bash docs/reports/w47-condvar-join-publication/scripts/gate.sh LOGS rust
bash docs/reports/w47-condvar-join-publication/scripts/gate.sh LOGS records
```
