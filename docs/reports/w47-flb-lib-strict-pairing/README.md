# W47-FLB-LIB-STRICT-PAIRING: the library lifecycle as a strict pair

Card W47-FLB-LIB-STRICT-PAIRING (wave 47, in_tail track; card-high), gateway
rule **10q** amended, contract specification **§C04.11** amended. Base: local
`main` at `fe603d49` (this worktree's HEAD; no reset). Frozen prediction:
[PREDICTION.md](PREDICTION.md) (commit `aa0fff44`, before any AFTER
measurement; the kernel was being drafted in the working tree, uncommitted and
unmeasured, when it was frozen). It cites `:1068-1070` for `flb_stop`'s
`cf_main` release; the lines are `:1066-1069` (corrected here, not there).

The user's decision (2026-09-29) supersedes the earlier "a stop after the
first has no effect": `flb_stop` is permitted **only after a successful
`flb_start`, exactly once**; after a failed start the caller calls only
`flb_destroy`; a second stop is a caller violation; a start twice without a
stop stays forbidden. It is written as a contract and **checked**, never
trusted. Nothing here is a verified race, bug or fix, or an `Unsafe` claim;
every old-vs-fixed statement reads *discharged / not discharged / unresolved
at this path under these derived premises*, and every path named against a
caller or the library is a **candidate** with the evidence it lacks.

## 1. Mechanism

1. **The kernel** (`crates/qcore-core/src/paired_lifecycle.rs`): `step` is
   the strict pair and returns a `Refusal` (a **violation**, or a
   precondition **not established** where the start's value is undecided).
   The clauses:

   | Clause | Kind |
   |---|---|
   | L1 a start only from created (a restart is not derived from any body, so it is refused) | caller precondition |
   | L2 the first stop after a start that returned 0 joins that start's thread, exactly once | body (`flb_stop`) |
   | L3 no stop after a stop | caller precondition |
   | L4 no stop after a start that returned nonzero | caller precondition |
   | L5 no stop of a never-started context | caller precondition |
   | L6 a destroy only from created, start-failed or stopped | caller precondition |
   | L7 a start that returns nonzero leaves no thread it began unjoined (the body fact behind L6's start-failed admission) | body (`flb_start`) |

   `no_effect_status` and `governed_joins` (the withdrawal of refuted
   no-effect clauses) are removed with the clauses they served;
   `failed_start_status` decides L7.
2. **The reading** (`crates/qcore-c-gateway/src/condvar_census/lifecycle.rs`,
   one hook in `condvar_census.rs`): each declared start's own nonzero
   returns are run with the threads each leaves unjoined
   (`LifecycleScenario::FailedStartReturn`); each release records, per path
   state, its histories with that path's own join evidence
   (`ReleaseReading::paths`, lifecycle mode only).
3. **The adapter** (`crates/quirin-source-analysis/src/lifecycle_census.rs`):
   L2 and L7 decided from the bodies; the stop run from the three forbidden
   states is kept as an **observation outside the contract**
   (`Observation`), never a clause; each call's refusals split into
   violations and not-established; `release_split` divides a release's paths
   into **conforming** (every history derived, none refused), undetermined
   (no refusal, some history not derived) and refused, and the census
   (`condvar_join_census.rs`) decides the release over the conforming paths
   apart (`LifecycleSplitVerdict`). The release's own verdict is unchanged
   in kind: the lifecycle gate joined with the reading over **every** path.
4. **The contract** (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/lib_engine_thread.qv`):
   the clause lines are unchanged (`resource_lifecycle(ctx, start|stop|destroy)`
   keeps its spelling; §C04.11 changes its meaning); the module comment and
   the three review reasons are rewritten for the strict pair (`flb_start`:
   restart not derived; `flb_stop`: the violating stops and their native
   reproduction; `flb_destroy`: L6 with start-failed, resting on L7). Every
   `resource_review_pending` marker stays; `flb_start` keeps
   `rewrite_before_acceptance` (its reason, `ctx` retained against
   `resource_borrowed`, is not made obsolete). Re-pinned at the three trees
   ([`scripts/pin.sh`](scripts/pin.sh)): each `pin` 0, each three-function
   `pin --check` 0 (3 recorded, 0 not in force, 0 stale); only the three
   `contract_sha256` values change in `contracts.lock`, every body identity
   is unchanged.

## 2. Negative controls first

`crates/quirin-source-analysis/tests/condvar_join_lifecycle.rs` (14 tests,
the miniature library) and `crates/qcore-core/tests/unit/paired_lifecycle.rs`
(17 kernel tests):

| Case | Outcome |
|---|---|
| stop after a failed start | **violation (L4)**; the release it reaches is refused (`the strictly paired lifecycle is not kept on a path`); the stop's body from that state is an observation (every path joins) |
| double stop | **violation (L3)** (`a second stop`, state `stopped`); release refused |
| start, stop, destroy (guarded) | **conforms**; L2 in force; the release equals the census's without the lifecycle, and its conforming-path verdict equals it |
| failed start, destroy | **conforms** (L6, with L7 named as its body support) |
| start, start | **refused (L1)**; without the lifecycle the fixture discharges (the entry-keyed collapse) |
| stop regardless of the start (the fixed test's shape) | refused on the failed-start paths; its conforming paths decide exactly as a shape reaching the release only after a successful start |
| stop of a never-started context | **violation (L5)**; the body joins an identifier no start began (observation) |
| destroy before the stop, restart after a stop | refused (L6, L1) |
| undecided start, then stop or destroy | **not established**, refused |

## 3. The clauses against the bodies, per tree

The census with the module over the rule 10q file set at the `90978ac6`
export (fixed test), the same tree with the previous test (`90978ac6~1`),
the `046accf6` clone and the `66910c10` corpus source
([`scripts/census.sh`](scripts/census.sh); archive
[artifacts/census/](artifacts/census/), figures
[artifacts/census-summary.json](artifacts/census-summary.json)). The four
runs decide identically:

| Clause | Status | Followed paths (`src/flb_lib.c`) |
|---|---|---|
| L1, L3, L4, L5, L6 | in force (preconditions) | — |
| L2 | **in force** | 2 paths, each one first join of the started thread: :1058 (status branch) or :1077 |
| L7 | **held** | 4 nonzero returns of `flb_start`: 3 leave nothing unjoined (fips failure, the joins at :984 / :999); 1 leaves `flb_lib_worker` unjoined, the spawn-failure return the reader cannot tell from a spawn |
| outside the contract | observed, every path joins | from never started: an identifier no start began (:1058/:1077); after a stop: a second join; after a failed start: a second join, an unstarted identifier, or a first join where the spawn's failure is not told |

Restart: no body reading derives that `flb_start` can run after `flb_stop`
(the engine thread's run is not read; `flb_stop` destroys
`ctx->config->cf_main` at :1066-1069), so L1 refuses it.

## 4. The callers against the preconditions

**in_tail.c, the census's test binary** (every tree; 280 declared-call
readings): **50 violating readings over all 46 roots** (0 before), at 14
`flb_stop` sites at `90978ac6` and 13 at `90978ac6~1`, `046accf6` and
`66910c10`: every one a stop reachable after `flb_start` returned nonzero
(L4), because the root continues past `TEST_CHECK(ret == 0)`; 5 also stop a
never-started context (L5: `test_tail_ctx_destroy` at :292 on setup-failure
paths). 0 not established; 188 readings carry an undetermined path (161
before; +27 destroys after a refused stop). No double stop and no restart in
the roots. Every `flb_start` (50) is from created; every `flb_destroy` is
from created or stopped where derived.

* **Fixed test** (`flb_test_inotify_pause_collectors`, `90978ac6`):
  `flb_start` :2559 from created; `flb_stop` :2583 from started-ok (L2,
  conforming) **and from start-failed: a caller violation (L4), a candidate
  against the test**: the stop runs whatever the start returned (the brief's
  `:2560` is the `if (TEST_CHECK(ret == 0))` line). The fix shape is a guard
  of the stop by the start's success, `flb_destroy` alone on the failure
  path. `flb_destroy` :2588 from stopped.
* **Old test** (`90978ac6~1`, negative control): `flb_stop` :292 in
  `test_tail_ctx_destroy` from started-ok and from start-failed (L4), the
  same candidate.

**Every runtime test** ([`scripts/runtime_tests.sh`](scripts/runtime_tests.sh),
one census per `tests/runtime/*.c` at the `90978ac6` export;
[artifacts/runtime/summary.json](artifacts/runtime/summary.json)): 113 of 116
files read (`in_dummy.c` and `in_random.c` are symlinks whose target text is
an `#include` line, unreadable at the export; `in_ebpf_exec_handler.c` was
stopped after 20 minutes of path reading); 103 with a declared call, **89
with a refusal: 848 of 984 roots, 875 readings over 542 sites**; by clause
L4 851, L5 10 (in_tail.c 5, `processor_cumulative_to_delta.c:605` 5: its
`rt_ctx_destroy` stops a context a setup failure left unstarted), L6 19 (a
`flb_destroy` of a context whose start unexpectedly succeeded, in tests that
expect the start to fail and continue past the failed check: e.g.
`in_tcp.c:916`, `out_syslog.c:307`). One L4 reading is a reader artifact
(`core_internal_logger.c:177`: its start is followed by `TEST_ASSERT`,
which acutest aborts on and the reader's binding table does not name). No
library caller (`src/`, `plugins/`) is refused on any test path.

**In-tree callers, by reading** (not derived by the census):
`flb_create`'s error path calls `flb_stop` on the context it is building
(:215), never started: **an L5 violation inside the library**, whose
status branch joins identifier 0 (the config is calloc'd), the behaviour the
native observation saw as SIGSEGV for (iii); a candidate, needing a failed
`mk_event_channel_create` (`W47-FLB-CREATE-STOP-UNSTARTED`). The chunk-trace
pipeline stops only after a successful start (`flb_chunk_trace.c:227` →
:270; :106 reached after the pipeline signalled its start) and destroys
alone on its start failure; `flb_reload.c:651` stops the running context and
destroys a failed new one alone (:656-664); `fluent-bit.c` (outside the
census file set) destroys alone on a failed start (:1473-1476) and stops
after a successful one (:1594). None is a violation by reading.

## 5. C5 under the strict pair

Release and C4 tallies are unchanged at every tree (fixed 97 discharged /
134 not discharged, the other three 97 / 131); every release a refused path
reaches was already not discharged (99 at the fixed export, 95 elsewhere),
and only its reason moves to the lifecycle refusal; one C4 site (`cond_destroy`
:2595, fixed) rewords likewise. **No Safe → non-Safe move, no correction.**

The fixed test's releases after its stop (`flb_destroy` :2588, `unlink`
:2591, `pthread_cond_destroy` :2595, `pthread_mutex_destroy` :2596, scope end
:2597), decided per path class:

| Paths | Count | Result |
|---|---|---|
| refused (start-failed, then the stop) | 10 | **not discharged**: the L4 violation; a violating path never discharges |
| conforming (start 0, stop, …) | 2 | **not discharged at this path under these derived premises**: exactly one engine join per path (L2), the checked one at :1077 or the status branch's unchecked first join at :1058, which does not count (`not busy`: 7 join sites may join `flb_lib_worker` off the joining thread; `no cycle`: 6 `pthread_self` sites); 12 parent-child descendant readings (26 at `flb_destroy`), every one unresolved |
| undetermined (the infeasible continuations past `test_tail_ctx_create`'s null return) | 37 | decide nothing; they still carry the repeated joins the release's own reading joins |

**Final C5 of the fixed test: not discharged on both paths.** On the
failed-start path by the caller violation (a candidate against the test); on
the success path by the status branch's join and the descendants, even with
only one engine join. Derived: P-DEREF's `!ctx->config` arm; the engine
thread joinable and not self-joined; L2 in force; the test's lifecycle on
every derived path (conforming on the success path, L4 on the other).
Stated, not proved: P-JOIN-IDENTITY, P-LIFECYCLE-INSTANCE, P-RETAIN,
P-CHECK, the lexical reading and binding tables (the creator table
included), acutest's per-test fork. Not derived: the status branch's busy
and join-cycle POSIX cases (P-POSIX-JOIN does not admit that join), the
retaining descendants, L7.

**Old test (`90978ac6~1`):** `unlink` :286 before the stop, 5 conforming
paths (30 undetermined, 0 refused): **not discharged** (no join on the path;
the stop comes later, so nothing is refused there); `flb_destroy` :293, 12
refused paths (L4) and 3 conforming: **not discharged** (a failed join at
:1077 the test does not guard). Its C1 path is untouched (no loop-owner code
changed) and stays not discharged.

## 6. Reclassification

`W47-ENGINE-THREAD-JOIN-CANDIDATES` (i) (a stop after a start whose engine
failed after the spawn), (iii) (a stop after a start that failed before it)
and (v) (a second stop) are, under the strict pair, **reproductions of
caller-contract violations** (L4, L4, L3): the native evidence of
W47-FB-DOUBLE-STOP-OBSERVATION stands as the observation of the library's
behaviour outside its contract, not a library defect. (ii) and (iv) are
unchanged and keep the entry open. The candidates move to the callers
(`W47-STRICT-PAIR-CALLER-CANDIDATES`, `W47-FLB-CREATE-STOP-UNSTARTED`).

## 7. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| L1, L3–L6 | in force (preconditions) | **held** |
| L2 | in force, 2 paths | **held** |
| L7 | held (spawn failure untold) | **held**, 1 of 4 nonzero returns |
| stop outside the contract | every path joins, from the three states | **held** |
| violating readings, roots | 50 (L4; 5 also L5), 46, at each tree | **held** |
| fixed / old test | stop :2583 / :292 violates L4 on the failed-start path | **held** |
| release tallies | unchanged; 99 / 95 reasons move | **held**; also 1 C4 reason (not predicted) |
| fixed test conforming paths | not discharged: :1058 not counted, descendants unresolved, no second join | **held** (2 conforming paths) |
| old test | `unlink` :286 not discharged, unrefused; `flb_destroy` :293 refused / not discharged | **held** |
| corpus / system / slice | 0 status, 0 leaf; environment identity only | **held**; also 4 review-hold reason strings in `flb_create`'s slice rows (verbatim reasons, not leaves) |
| undetermined readings | (not predicted numerically) | 161 → 188 |

## 8. Corpus, system and slice

[`scripts/measure.sh`](scripts/measure.sh) `PHASE TREE` (`--jobs 8`):
BEFORE at the base with the release binary built before any change
(`artifacts/before-binary.sha256`), AFTER with the change
(`artifacts/after/binary.sha256`). **BEFORE reproduces the baseline**
(`docs/reports/w47-flb-lib-lifecycle/integration/artifacts/`): corpus
`resource-diagnostics.json` byte-identical, system and slice 0 moved
functions, 0 moved units, equal diagnostic counts
([identity-before.json](artifacts/identity-before.json)). The brief's
`baseline_identity.py` does not read this archive's filename-keyed manifest
(exit 1, `KeyError: 'diagnostics'`); `scripts/identity.py` (the lifecycle
card's) was used instead. AFTER: [identity-after.json](artifacts/identity-after.json).

| Route | Functions | Moves |
|---|---|---|
| corpus (185 units, `66910c10`) | 526 / 10 / 4,298 | **none**: byte-identical to BEFORE |
| system (`046accf6`) | 31 / 2 / 201 | 0 status, 0 leaf; only `resource_contract_sources` and `inventory_sha256` of the 15 units ([`scripts/route_diff.py`](scripts/route_diff.py)) |
| slice (`90978ac6`) | 24 / 1 / 244 / 1 Pending | 0 status, 0 leaf; the 16 units' environment identity, and 4 review-hold reason strings in `flb_create`'s rows (Unknown before and after), which quote `flb_stop`'s and `flb_destroy`'s rewritten reasons verbatim ([`scripts/holds_diff.py`](scripts/holds_diff.py)) |

System and slice are archived as the new baseline in
[artifacts/after/](artifacts/after/). **Leaks and violations:** none added or
removed on any route; no provisional `Unsafe` added.

## 9. Lean

`proof/QCore/CondvarJoin.lean`, section `Lifecycle` rewritten, audited in
`proof/AuditCondvarJoin.lean` (**79** audited theorems, 69 → 79: 18 of the
lifecycle's retired or restated, 28 now; no placeholders; axioms only
`propext`, `Quot.sound`, `Classical.choice`):

* the strict pair: **`a_stop_is_admitted_only_from_started_ok`**,
  **`a_stop_after_a_failed_start_is_a_violation`**,
  **`a_second_stop_is_a_violation`**,
  `a_stop_of_a_never_started_context_is_a_violation`,
  `a_restart_after_a_stop_is_rejected`, `starting_twice_is_rejected`,
  `destroying_a_running_context_is_rejected`,
  **`a_destroy_after_a_failed_start_is_admitted`**,
  `an_accepted_start_began_created`, `an_accepted_destroy_was_not_running`,
  `nothing_follows_a_destroy`, `the_strict_sequences_are_accepted`,
  `a_history_with_a_double_start_is_rejected`, `lrun_append`,
  `a_refused_step_rejects_the_history`;
* one engine join: `an_accepted_step_spends_its_stop`,
  `an_accepted_history_stays_within_its_stop_budget`,
  `a_conforming_history_stops_at_most_once`,
  **`a_conforming_path_has_exactly_one_engine_join`** (L2 in force ⇒ the
  one stop's path is the thread's first join);
* body clauses:
  `the_first_stop_clause_is_in_force_only_if_every_path_joins_the_started_thread_once`,
  `the_failed_start_clause_is_in_force_only_if_no_return_leaves_a_thread`,
  `no_followed_path_holds_the_clause`;
* the gate: `a_lifecycle_violation_blocks_the_release`,
  `without_a_violation_the_release_is_unchanged`,
  **`a_refused_history_never_discharges`**,
  **`a_stop_after_a_failed_start_or_a_second_stop_never_discharges`**;
* the tests: **`the_fixed_test_under_the_strict_pair`** (failed-start path
  refused; conforming path accepted, no repeated join, not discharged; even
  with every POSIX case excluded the unresolved descendants block),
  **`the_old_and_fixed_test_shapes_stay_not_discharged`**.

Retired with their Rust counterparts: the no-effect clause and withdrawal
theorems (`noEffectStatus`, `governedJoins` and the six theorems on them),
`an_accepted_start_began_unstarted`, `an_accepted_destroy_was_unstarted`,
`the_paired_sequence_is_accepted`.

## 10. Follow-ups

[followups.json](followups.json), applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py) (568 → 571).
Nothing is measurably discharged, so nothing is retired. **Amended:**
`W47-ENGINE-THREAD-JOIN-CANDIDATES` (§6), `W47-LIB-ENGINE-THREAD-REVIEW`
(the strict pair; markers kept), `W47-LIFECYCLE-PATH-FEASIBILITY` (161 →
188; the undetermined class of `release_split`). **Registered:**
`W47-STRICT-PAIR-CALLER-CANDIDATES` (§4, sized), `W47-FLB-CREATE-STOP-UNSTARTED`
(the library's own L5 by reading), `W47-STRICT-PAIR-BODY-RESIDUALS` (L7 held;
restart not derived).

## 11. Gate

See [gate.json](gate.json) and [gate/](gate/) ([`scripts/gate.sh`](scripts/gate.sh)).

## Reproduce

```sh
bash docs/reports/w47-flb-lib-strict-pairing/scripts/census.sh OUT [BINARY]
python3 docs/reports/w47-flb-lib-strict-pairing/scripts/summarize.py OUT BEFORE
bash docs/reports/w47-flb-lib-strict-pairing/scripts/runtime_tests.sh OUT BINARY
python3 docs/reports/w47-flb-lib-strict-pairing/scripts/runtime_summary.py OUT
bash docs/reports/w47-flb-lib-strict-pairing/scripts/measure.sh PHASE TREE
bash docs/reports/w47-flb-lib-strict-pairing/scripts/pin.sh LOGS
bash docs/reports/w47-flb-lib-strict-pairing/scripts/finalize.sh
python3 docs/reports/w47-flb-lib-strict-pairing/scripts/update_ledger.py
bash docs/reports/w47-flb-lib-strict-pairing/scripts/gate.sh LOGS rust
bash docs/reports/w47-flb-lib-strict-pairing/scripts/gate.sh LOGS records
```
