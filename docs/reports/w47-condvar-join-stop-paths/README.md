# W47-CONDVAR-JOIN-STOP-PATHS: flb_stop's paths, unchecked joins and descendants (C5)

Card W47-CONDVAR-JOIN-STOP-PATHS (wave 47, in_tail track; card-high), gateway
rule **10q** amended, contract specification §C04.10 amended. Base: local
`main` at `fefb9199` (the typed-dispatch integration after the condvar/join
card); this worktree's HEAD was already there. Frozen prediction:
[PREDICTION.md](PREDICTION.md) (commit `72e7079f`, before any code, contract or
proof change). The integrator's added step (record the observations as
contracts) is §5.

Nothing here is a verified race, a verified bug, a verified fix or an `Unsafe`
claim. Every old-vs-fixed statement reads: *the obligation is (not)
discharged, or unresolved, at this path under these derived premises*. An
upstream path is named only as a **candidate**, with the evidence it lacks.

## 1. Mechanism

New logic lives in new code with thin hooks; `thread_entry_census.rs`,
`loop_owner_census.rs`, `legacy_c.rs` and `legacy_literal_premise.rs` are
untouched, so the loop-owner census (C1) and every per-unit route read
exactly as before.

1. **Reader** (`crates/qcore-c-gateway/src/condvar_census/stop_paths.rs`, a
   child of `condvar_census.rs`; the path reader gains four hooks):
   * *First and repeated joins.* A join of a thread the path started records
     the spawn wrapper that started it (`UncheckedFirst { wrapper }` when its
     result is ignored); a join of a thread the path already joined is
     `Repeated`, whatever its result.
   * *Null tests.* Every `!X->F` / `X->F == NULL` in a function the path reader
     inlines is read with: the earlier top-level statement on every path to
     it that dereferences `X->F` (not in a call's arguments, since a macro may
     drop them; not under `sizeof`; not beside `&&`, `||`, `?`), what the
     function does to `X` and `X->F`, `X`'s declared type (parameters and
     locals, then file scope; a local shadows; typedefs resolved), and every
     store, address and whole-object write of field `F` of that type in the
     file set, each store classified as *pre-publication* (a top-level
     statement of a function whose earlier statements on a path to it touch
     the base only as its table-allocator assignment, a bare null test or a
     declaration). The adapter's decider (the kernel) drops the null arm or not.
   * *Thread sites.* Every `pthread_join`/`pthread_detach` with the entries its
     identifier may name (spawn out-arguments, `memcpy`, fields keyed as the
     path reader keys them), every `pthread_self()` (kept unless passed
     straight to `pthread_equal`/`pthread_kill`), and every spawn wrapper's
     joinability (its body calls `pthread_create` once through its own
     out-parameter, with a null or `PTHREAD_CREATE_JOINABLE` attribute, and
     names no detach; `mk_utils_worker_spawn` derives, `mk_utils.c:316-318`).
   * *Entry runs.* Every spawned entry is run on its own thread to its exit
     (budget 200,000 statement-state steps; exhaustion is an unresolved
     reason), recording each thread it started and how it was joined.
2. **Kernel** (`crates/qcore-core/src/condvar_join.rs`):
   `null_branch_excluded` (all of: dominating dereference, no local defeat,
   read type, no unresolved write, every store pre-publication; premise
   **P-DEREF**); `join_succeeds` over `JoinExclusions` (joinable, not busy,
   not self, no cycle; premise **P-POSIX-JOIN**); `JoinEvidence::{UncheckedFirst,
   Repeated}`; `DescendantJoin`/`DescendantEvidence` in `ReleaseSite`;
   `release_verdict` ranks a repeated join after an absent one and before a
   failed one, admits an unchecked first join only when `join_succeeds`, then
   refuses a never-joined descendant and leaves an unresolved one open;
   `release_premises` names P-POSIX-JOIN when it was used.
3. **Adapter** (`crates/quirin-source-analysis/src/condvar_join_census.rs`):
   `ThreadFacts` (reached join/detach/self sites with the threads their
   functions run on, from the loop-owner contexts; a function no root reaches
   and no unresolved path may reach never runs) computes each unchecked first
   join's four exclusions for its joiner (`T` at a test root, the parent at an
   entry run); each live retaining thread's descendants are decided per
   direct parent from that parent's entry run. The census example prints the
   exclusions, descendants, premises, null tests and wrappers.

## 2. Negative controls first

`crates/qcore-core/tests/unit/condvar_join.rs` (+4 tests) and
`crates/quirin-source-analysis/tests/condvar_join_stop_paths.rs` (8 C
fixtures of `flb_create`/`do_start`/`flb_stop`/`mk_utils_worker_spawn` in
miniature, run through the census):

| Case | Outcome |
|---|---|
| a stop whose config is tested before any dereference | the arm stays: **not discharged** (`returned from stop_nullable … without joining`) |
| the fixed shape, but a published engine's config stored elsewhere (`e->config = NULL`) | not a pre-publication store: the arm stays, **not discharged** |
| a detached thread joined (`PTHREAD_CREATE_DETACHED` in the wrapper) | the status branch's first join: **not discharged** (`not joinable`) |
| a thread joined twice (failed start joins, stop joins again) | **not discharged** (`joined a second time`) |
| a join from the thread itself | `not_self` open: `join_succeeds` refuses |
| a descendant never joined (`flb_destroy` after the engine's detached child) | **not discharged** (`never joined`) |
| each POSIX case left open, one at a time (kernel) | refused, naming the case |
| the fixed shape (dereference before the test, one pre-publication store, joins only on the test thread, no `pthread_self`) | the arm dropped, the first join admitted: **discharged under P-POSIX-JOIN** |
| a descendant joined with a checked join | **discharged** |

The 13 kernel and 14 census tests of W47-CONDVAR-JOIN-PUBLICATION pass
unchanged (the R2 fixture still refuses: its wrapper has no body, so
*joinable* is open).

## 3. The fixed test against the old one (census over the test binary)

`docs/reports/w47-condvar-join-publication/scripts/census.sh` over the
loop-owner census's 1,685 files of the `90978ac6` export and the same tree
with the previous test (`90978ac6~1`). BEFORE reproduces the archived census
byte for byte (`fixed.json` `7a4f66d5…`, `previous.json` `251f7d9e…`).
Archive: [artifacts/census/](artifacts/census/); the comparison is
[`scripts/compare_census.py`](scripts/compare_census.py)
([artifacts/compare-fixed.json](artifacts/compare-fixed.json),
[artifacts/compare-previous.json](artifacts/compare-previous.json)).

**Step 2, path feasibility of `flb_stop` on the test's path** (`src/flb_lib.c`):

* `!ctx->config` (:1062-1064): **infeasible, derived**. `tid =
  ctx->config->worker` (:1047) dereferences it on every path to the test;
  `flb_stop` never assigns `ctx` or `ctx->config`; `ctx` is `flb_ctx_t *`
  (`struct flb_lib_ctx`); the only store of `config` into that type in the
  file set is `flb_create`'s (:174), into the object it allocated at :163
  after only its null test and a `flb_free` on a returning branch; no address
  of the field, no whole-object write. Under P-DEREF the arm is dropped. It is
  the only one of **450** null tests the census reads that is excluded. The
  `Absent … returned from flb_stop at src/flb_lib.c:1063` evidence leaves all
  117 releases that carried it (113 at `90978ac6~1`).
* The status branch (:1049-1060): **stays feasible, not derived away**. The
  test calls `flb_stop` whatever `flb_start` returned (`if (TEST_CHECK(ret ==
  0)) { … }` guards only the wait, `tests/runtime/in_tail.c:2560`); after
  `do_start`'s failure returns (:980-986, :994-1001) `ctx->status` is
  `FLB_LIB_ERROR`; `do_start` can return 0 without `FLB_LIB_OK` (:1008), and
  the engine thread itself writes `FLB_LIB_NONE` (:932) unsynchronised with
  the read (:1049). The census does not track `ctx->status`, and nothing here
  would exclude either value.

**Step 3, the unchecked join at :1058.** On paths where it is the thread's
first join it is `UncheckedFirst { mk_utils_worker_spawn }`. Its exclusions:

| POSIX case | Derived? |
|---|---|
| not joinable (`EINVAL`/`ESRCH`) | **yes**: `mk_utils_worker_spawn` creates it joinable (`mk_utils.c:318`), no reached detach names it (the two `pthread_detach` sites name `_timeout_worker`) |
| a self join (`EDEADLK`) | **yes**: the test thread's path started it |
| already joined or awaited by another joiner (`EINVAL`) | **no**: 7 reached join sites may name `flb_lib_worker` off the test thread — `flb_stop`'s and `do_start`'s own joins (:984, :999, :1058, :1077), reached on 6 other threads through the chunk-trace pipeline's `flb_stop`/`flb_start` (the census names a thread by its entry, so it cannot tell that pipeline's engine from the test's), and 3 identities it does not read (`mk_clock.c:133`, `throttle_size.c:756`, a vendored libmaxminddb test) |
| a join cycle through the joiner (`EDEADLK`) | **no**: 6 reached `pthread_self()` sites on unresolved paths may keep an identifier (`evthread_pthread.c:103`; five in `winpthreads.c`, a Windows-only file the unconfigured census reads) |

So the ignored result stays **not checked**: the join is not counted as
successful. The main path's join (:1077) is checked by `flb_stop` and its
result by the test (`exit` on nonzero, :2586), as before.

**Repeated joins.** Every path through `do_start`'s failing joins (:984,
:999) and then `flb_stop` (:1058 on the status branch, or :1077) is a second
join of a thread no longer joinable: 108 releases at `90978ac6` carry one.
This is now the fixed test's first-ranked blocker.

**Step 4, descendants.** At the fixed test's non-engine releases the 4
retaining descendants (`downstream_worker_thread`, `input_thread`,
`output_thread`, `update_pod_service_map`), under their direct parents
(`flb_lib_worker`, `input_thread`, `downstream_worker_thread`), are all
**unresolved**; at `flb_destroy` all 9 (adding `do_reload`, `etw_worker`,
`log_worker_collector`, and the wrapper-internal entries
`mk_utils_worker_entry`, `step_callback`, which are decided under their
callers' entries, never as never-joined). The reasons, from the entry runs:
`flb_lib_worker`'s path to its exit carries 8 (three bodies past the nesting
limit — `flb_stop`, `flb_chunk_trace_destroy`, `trace_chunk_context_destroy`;
four calls that may start a thread through a hop the path reader does not
follow — `flb_custom_init_all`, `flb_input_chunk_set_limits`,
`flb_processor_destroy`, `trace_pipeline_shutdown`; and the step budget);
`input_thread`'s 4 such hops; `downstream_worker_thread` and
`trace_chunk_pipeline_thread` are not read (`goto`,
`flb_downstream_worker.c:244`, `flb_chunk_trace.c:179`). The shutdown joins
themselves, read and not derived, all ignore their result
(`flb_input_thread.c:607`, `flb_output_thread.c:646`,
`flb_downstream_worker.c:520`, `kubernetes.c:916`).

| Site / case | `90978ac6~1` | `90978ac6` |
|---|---|---|
| C5 releases after `flb_stop` (`flb_destroy` :2588, `unlink` :2591, `pthread_cond_destroy` :2595, `pthread_mutex_destroy` :2596, scope end of `test` :2597) | absent | **not discharged**: `flb_lib_worker is joined a second time on a path (src/flb_lib.c:1058)`; also on paths: the status branch's first join not counted (busy, cycle); 4 (9 at `flb_destroy`) descendants unresolved |
| C4 `cond_destroy` :2595 | absent | **not discharged**, same reason (its quiescence rests on the release reading) |
| other C4 sites of the test (init :2550, wait :2565, reads :2570-:2576) | absent | discharged (unchanged) |
| negative control, `unlink` :286 before `flb_stop` | **not discharged** (`released before any join … (no join on this path)`) | absent from the non-threaded test; not discharged in the threaded one |
| negative control, `flb_destroy` :293 after an unchecked `flb_stop` | **not discharged** (repeated join; a failed join at :1077 on a path) | as at left in the threaded test |
| releases before `flb_start` (`flb_create` :216, `test_tail_ctx_create` :245/:253/:262) | discharged | discharged |

**Whole census:** no verdict moves at either revision (fixed: releases 97
discharged / 134 not discharged; previous: 97 / 131; every C4 tally
unchanged). At `90978ac6` 116 release reasons and one C4 reason reword (92
`:1063` → repeated join, 13 whose last note was `flb_log_destroy` → repeated
join, 11 → `failed on a path (:1077)` in tests that ignore `flb_stop`'s
result), and 18 releases whose `no join on this path` reason stands change
only their evidence; at `90978ac6~1` 112 and 19. **Final C5 status of the
fixed test:** not discharged at this path under these derived premises, the
old test's negative control holds, and no C1 path moves (the loop-owner
census is untouched).

## 4. Upstream candidates (not verified)

Registered as `W47-ENGINE-THREAD-JOIN-CANDIDATES`; each needs a native
reproduction to be more than a candidate, and none makes any verdict `Unsafe`:

1. `flb_stop` after a `flb_start` whose engine failed: `do_start` joins the
   engine thread (:984/:999) and sets `FLB_LIB_ERROR`; `flb_stop`'s status
   branch joins it again (:1058). Missing: a run forcing `FLB_ENGINE_FAILED`
   and observing the second join.
2. `ctx->status`: the engine thread's write (:932) and `flb_stop`'s read
   (:1049) are not synchronised. Missing: a TSan run of `flb_stop` against
   engine exit.
3. `flb_stop` after a `flb_start` that failed before spawning (`flb_fips_init`
   or the spawn) joins `ctx->config->worker`, which `do_start` never wrote
   (`flb_config_init` zero-fills it, `flb_config.c:288`). Missing: a run
   forcing the early failure.
4. `flb_output_thread_pool_destroy` skips the join of a worker whose stop
   command cannot be written and frees its parameters
   (`flb_output_thread.c:640-644`). Missing: a failed-write reproduction.

## 5. The observations as contracts (the integrator's step)

The derivation does not discharge C5, so the observations are written down
as Fluent Bit contracts: `providers/quirin-provider-fluent-bit/contracts/fluent_bit/lib_engine_thread.qv`
declares `flb_start`, `flb_stop`, `flb_input_thread_instance_exit`,
`flb_output_thread_pool_destroy` and `flb_downstream_worker_runtime_stop`.
What a function does to a thread is not a v1 custody fact and v1 has no
clause for it, so **every declaration carries a contract-level
`resource_review_pending` marker** (§C06.1): loaded, withheld from the function
table, discharging and accusing nothing, every call that would apply it held
under `awaiting-human-review` (never `Safe`). The reason strings carry the
observations and name the candidates as candidates.

* **Checked against the body by the census at `90978ac6`:** `flb_stop`'s
  branches (the infeasible `!ctx->config` return, the status branch's
  unchecked join, the checked join returned), `do_start`'s unchecked joins on
  its failure returns, the repeated join, `mk_utils_worker_spawn`'s joinable
  creation.
* **Read, not derived:** the three shutdown joiners' behaviour.
* **Pending review (every clause):** the custody clauses (`ctx`/`ins`
  borrowed, `runtime` released nullable, scalar results) and the thread
  behaviour; `flb_start` hands `ctx` to the engine thread, which
  `resource_borrowed` does not say.
* **Body identity:** pinned with `quirin-audit pin --function …` at the
  `90978ac6` export (synthetic commit `34155955`), `046accf6` and `66910c10`;
  all five bodies are identical at the three revisions
  ([`scripts/bodies.py`](scripts/bodies.py); `mk_utils_worker_spawn`, read by
  the census and not declared, differs at `66910c10`, where it also sets
  `PTHREAD_CREATE_JOINABLE`). `flb_start`'s pin covers its own body, not
  `do_start`'s, which the census reads.

The resource-view rule makes every loadable provider module part of the
corpus selection, so `cycle19-resource-contracts.json` selects it and the
three routes load it. Module-count pins: selected **56 → 57**, corpus
**120 → 121** (quirin-provider **47 → 48**), v1 declarations **242 → 247**
(the function table keeps 242; the 5 are its pending-review entries),
fluent-bit resource view 19 → 20, the per-declaration clause table +5 rows.
No libc contract changes: P-POSIX-JOIN is a named premise of the rule, not a
declaration.

## 6. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| `!ctx->config` branch | derived infeasible | **held**: excluded, the only one of 450 |
| status-branch first join | not derived successful: joinable, not-self, no-cycle derived; not-busy not | **partly held**: joinable and not-self derived; **not-busy and no-cycle not** (6 `pthread_self` sites on unresolved paths, which the prediction missed) |
| repeated joins | the fixed test's first-ranked blocker | **held** (108 releases) |
| fixed test's 5 releases and `cond_destroy` | not discharged, reason → repeated join at :1058 | **held** |
| old test (negative control) | :286 and :293 stay not discharged | **held** |
| descendants | 4 / 9 unresolved, reason: the engine's path to its exit not read | **held**; reasons as listed in §3 (the loop itself is read; nesting, hops, budget and `goto` bodies stop it) |
| census tallies | unchanged at both revisions | **held** |
| corpus / system / slice | 0 status and 0 leaf moves; the corpus sidecar lists one more module | **corpus held** (byte-identical rows); **system and slice not held on leaves**: 0 status moves, but 1 + 2 functions reword a leaf to `awaiting-human-review`, because the pending module is selected on every route (§7). The prediction assumed no system or slice selection gains the module; the resource-view rule selects every loadable provider module |

## 7. Corpus, system and slice

[`scripts/measure.sh`](scripts/measure.sh) `PHASE TREE` runs the three
arbiters into fresh directories; BEFORE uses the base tree (`fefb9199`
exported with `git archive` into the session scratchpad, outside `qcore-c/`,
binary `ef4bdae9…`), AFTER this worktree (binary `3c293007…`). **BEFORE
reproduces all three baselines**: the corpus `resource-diagnostics.json` is
byte-identical to the integration archive (`ae17152b…`), system and slice
compose with 0 mismatches (`compose3.py`). Comparisons:
[`compare-{corpus,system,slice}.json`](artifacts/) (the previous card's
`compare.py`); the moved routes are archived as the new baseline in
[artifacts/after/](artifacts/after/) ([`scripts/finalize.sh`](scripts/finalize.sh)).

| Route | Units / functions | Moves |
|---|---|---|
| corpus (185 units, `66910c10`) | 6 / 8 / 167 / 4; 526 / 10 / 4,298 | **none**: `resource-diagnostics.json` byte-identical; leak 36, unknown_alias 7,936, unsupported 35,737 |
| system (`046accf6`) | 0 / 1 / 7 units; 31 / 2 / 201 rows | **0 status**, 1 function's leaf rewords (`flb_output_exit`, `src/flb_output.c`) |
| tail-pause slice (`90978ac6`) | 0 / 1 / 12 units; 24 / 1 / 245 | **0 status**, 2 functions' leaves reword (`flb_input_instance_exit`, `flb_create`) |

**Attribution.** Every moving leaf is a call of a declaration of
`fluent_bit/lib_engine_thread.qv`, which the census change cannot reach (the
per-unit routes never run it): `call to uncontracted function X` →
``call to `X` whose declared contract awaits human review
(awaiting-human-review)``, category `unsupported` unchanged, at the call of
`flb_output_thread_pool_destroy` in `flb_output_exit` (system: 2 held call
sites, the static and shared variants), of `flb_input_thread_instance_exit`
in `flb_input_instance_exit` and of `flb_stop` in `flb_create` (slice: 4 held
call sites). Each function has other unresolved leaves, so it stays `Unknown`
(no `Pending` row appears: a function is `Pending` only when held calls are
its only unresolved construct). No `Safe` function calls a declared name, so
there is **no Safe → non-Safe move and no correction**. **Leaks and
violations:** none added or removed. The corpus route loads the module (the
resource-view rule) and reports its 5 declarations with 0 held call sites;
only its contract sidecar and the source-effects evidence digests (the
contract environment's identity) change.

## 8. Lean

`proof/QCore/CondvarJoin.lean`, section `StopPaths`, audited in
`proof/AuditCondvarJoin.lean` (**51** theorems, 30 → 51; no placeholders;
axioms only `propext`, `Quot.sound`, `Classical.choice`):

* the null arm: `a_pre_publication_store_leaves_a_reachable_field`,
  `pre_publication_stores_leave_a_reachable_field`,
  **`the_null_arm_is_infeasible_after_a_dereference`** (P-DEREF),
  `a_published_store_can_null_the_field`,
  **`a_dropped_null_arm_had_its_derivation`**,
  `without_a_dominating_dereference_the_null_arm_stays`,
  `a_store_after_publication_keeps_the_null_arm`;
* joins under P-POSIX-JOIN (`posixSucceeds`, the premise as a model):
  **`an_unchecked_first_join_with_every_case_excluded_succeeds`**,
  `a_detached_thread_join_never_succeeds`, `a_repeated_join_never_succeeds`,
  `a_join_from_the_thread_itself_never_succeeds`,
  `an_underived_case_does_not_count`;
* the release decision: **`a_discharged_release_joined_every_descendant`**,
  `a_repeated_join_refuses`,
  `an_unchecked_first_join_counts_only_with_every_case_excluded`,
  `an_unresolved_descendant_blocks_the_discharge`,
  `a_never_joined_descendant_refuses`, and operationally
  **`a_live_descendant_blocks_the_release`**;
* the two tests' shapes: `the_fixed_test_release_is_not_discharged`,
  `the_old_test_stays_not_discharged`,
  `the_repeated_join_alone_keeps_the_fixed_test_not_discharged`.

## 9. Premises stated, not proved

Trust boundary 28, added here: **P-DEREF**, **P-POSIX-JOIN**, the
declared-type reader's scope (a local declaration shadows a file-scope one;
a name declared by a declaration the reader cannot split is unread) and C's
effective-type rule; each decision names the premises it used. Unchanged
from rule 10q: the lexical reading and binding tables, P-SYNC-INSTANCE,
P-JOIN-IDENTITY, P-RETAIN, P-CHECK, acutest's per-test fork. The census
reads the file set unconfigured (a Windows-only file is read), which only
widens what stays open.

## 10. Follow-ups

`followups.json`, applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): **551 → 555**, none
retired (nothing is measurably discharged). **Amended by measurement:**
`W47-CONDVAR-JOIN-DESCENDANT-JOINS` (4 / 9 descendants, their reasons, the
shutdown joins read), `W47-CONDVAR-JOIN-PREMISES` (+P-DEREF, P-POSIX-JOIN),
and R2 of `W47-TAIL-PAUSE-BINDING-RESIDUALS` (its outcome here; R1, R3–R6
untouched). **Registered:** `W47-CONDVAR-JOIN-INSTANCE-IDENTITY` (the busy
case: 7 join sites), `W47-CONDVAR-JOIN-SELF-IDENTIFIER` (the cycle case: 6
`pthread_self` sites), `W47-ENGINE-THREAD-JOIN-CANDIDATES` (§4, 4
candidates), `W47-LIB-ENGINE-THREAD-REVIEW` (the 5 withheld declarations).

## 11. Gate

GATE-SECTION

## Reproduce

```sh
bash docs/reports/w47-condvar-join-publication/scripts/census.sh OUT
python3 docs/reports/w47-condvar-join-stop-paths/scripts/compare_census.py BEFORE.json AFTER.json OUT.json
python3 docs/reports/w47-condvar-join-stop-paths/scripts/bodies.py ROOT_A ROOT_B
bash docs/reports/w47-condvar-join-stop-paths/scripts/measure.sh PHASE TREE
python3 docs/reports/w47-condvar-join-publication/scripts/compare.py BEFORE AFTER OUT.json
target/release/quirin-audit pin --source-root ROOT --crate quirin-provider-fluent-bit \
  --function flb_start --function flb_stop --function flb_input_thread_instance_exit \
  --function flb_output_thread_pool_destroy --function flb_downstream_worker_runtime_stop
python3 docs/reports/w47-condvar-join-stop-paths/scripts/update_ledger.py
bash docs/reports/w47-condvar-join-stop-paths/scripts/gate.sh LOGS rust
bash docs/reports/w47-condvar-join-stop-paths/scripts/gate.sh LOGS records
```
