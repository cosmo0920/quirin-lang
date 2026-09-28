# W47-FLB-LIB-LIFECYCLE: the library lifecycle as a checked contract (C5)

Card W47-FLB-LIB-LIFECYCLE (wave 47, in_tail track; card-high), gateway rule
**10q** amended, contract specification **§C04.11** added. Base: local `main`
at `ec4c9d4a` (this worktree's HEAD; no reset). Frozen prediction:
[PREDICTION.md](PREDICTION.md) (commit `6a5a1a59`, before any code, contract
or proof change).

The user's decision (2026-09-29) states the API: `flb_start` → `flb_stop`
pairs, a stop after the first has no effect, and a start without a stop in
between is not permitted. It is written down as a contract and **checked
against the bodies**, never trusted. Nothing here is a verified race, a
verified bug, a verified fix or an `Unsafe` claim; every old-vs-fixed
statement reads *discharged / not discharged / unresolved at this path under
these derived premises*, and an upstream path is named only as a
**candidate** with the evidence it lacks.

## 1. Mechanism

New logic lives in new modules with thin hooks.

1. **The clause** (`crates/qcore-contract/src/contract_loader.rs`, spec
   §C04.11): `requires resource_lifecycle(ctx, start|stop|destroy)` on a
   `ptr<mut, u8>` context, at most once, rederived from the retained texts
   (`lifecycle_obligations`, `declared_lifecycle_obligations`); any other
   spelling rejects the whole module. The per-unit route refuses every call
   of a declared function (`legacy_literal_premise.rs`, mark `lifecycle__`
   under the reserved prefix, wording `outside its declared lifecycle
   obligation`), behind the review hold that applies first; rule 10i's
   holds-lifted second analysis therefore finds it standing.
2. **The kernel** (`crates/qcore-core/src/paired_lifecycle.rs`): the state
   machine `step` (created, started with its start's outcome, stopped,
   destroyed), the six clauses, the body decisions `no_effect_status` and
   `ends_started_status` (in force only where every followed path satisfies
   the clause with nothing unresolved, refuted only where every followed path
   contradicts it, held otherwise), `governed_joins` (the withdrawal: a
   caller's stop is read with the clause's joins only where a no-effect
   clause is in force, with the body's otherwise) and `lifecycle_gate`.
3. **The reading** (`crates/qcore-c-gateway/src/condvar_census/lifecycle.rs`,
   a child of `condvar_census.rs`; the path reader gains four hooks and
   `read_program_lifecycle`, and with no roles reads exactly as before):
   * *Bodies.* Each declared stop is run by the path reader, join trace on,
     from each clause's state: never started (L5), after the declared start
     returned 0 (L2), after a stop that followed it (L3), after the start
     returned nonzero (L4); each declared destroy after a successful start.
   * *Callers.* On each test root's paths every declared call records its
     context's history before it (key: `TYPE.FIELD` for a declared struct
     base, or `FUNCTION::NAME`; a creator's result assigned to the key starts
     it). Every release records every context's history on each path.
4. **The adapter** (`crates/quirin-source-analysis/src/lifecycle_census.rs`,
   `census_with_lifecycle` in `condvar_join_census.rs`): decides each clause,
   each call (state before it per path, the precondition it violates, the
   clauses governing its effect, and why a path's state is undetermined),
   and joins `lifecycle_gate` before every release verdict and every C4
   `destroy` decided after a join. A history with no creation, or after its
   key's instance was destroyed, derives no state (P-LIFECYCLE-INSTANCE):
   it is undetermined and gates nothing. The census example takes
   `--lifecycle-module=MODULE`; without it the output is byte-identical to
   the base's.
5. **The contract** (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/lib_engine_thread.qv`):
   `flb_start` and `flb_stop` gain their lifecycle clause; `flb_destroy` is
   declared with `resource_lifecycle(ctx, destroy)`, its custody
   (`resource_released(ctx, nullable)`) held for review like the module's
   other declarations. The creator (`flb_create`) is a binding table entry
   (`FLUENT_BIT_LIFECYCLE_CREATORS`): its declaration takes no parameter a
   clause could name (`W47-LIFECYCLE-CREATOR-CONTRACT`).

## 2. Negative controls first

`crates/quirin-source-analysis/tests/condvar_join_lifecycle.rs` (9 tests,
the stop-path card's miniature library; `crates/qcore-core/tests/unit/paired_lifecycle.rs`,
13 kernel tests):

| Case | Outcome |
|---|---|
| a double start, then one checked stop | **refused**: `the paired lifecycle is violated on a path: … start of a context in state started without a stop (L1)`. Without the lifecycle the same fixture **discharges**: the reader keys the engine thread by its entry, so the second thread collapses into the first and one join looks like it ended both (`W47-CONDVAR-JOIN-RESPAWN-COLLAPSE`) |
| a stop that joins again after a failed start | L4 **refuted by body**; the stop is read from its body, the release stays **not discharged** (`joined a second time`) |
| a stop of a never-started context | permitted (created stays created), governed by L5; the body joins an identifier no start began, so L5 is **refuted** |
| a destroy before the stop | **refused** (`destroy of a context in state started before its stop (L6)`) |
| a stop that joins on some followed paths only (`stop_once`) | L3 **held**, neither in force nor refuted |
| the fixed shape (create, start, stop, check, release) | conforms; L2 **in force**, L3 refuted; the release verdict is exactly the census's without the lifecycle |
| malformed clauses (unknown operation, missing operand, a scalar or unknown parameter, a duplicate) | the whole module rejects (both the lifecycle reader and resource-v1) |
| a per-unit call of a declared function | refused `outside its declared lifecycle obligation`, never `Safe` |

## 3. The clauses against the bodies, per tree

The census with the module over the loop-owner census's file set at the
`90978ac6` export (fixed test), the same tree with the previous test
(`90978ac6~1`), the `046accf6` clone and the `66910c10` corpus source
([`scripts/census.sh`](scripts/census.sh)). `flb_start`, `do_start`,
`flb_stop` and `flb_destroy` are byte-identical at the three trees
([`scripts/bodies.py`](scripts/bodies.py)); the four runs decide identically:

| Clause | Status | Followed paths (`src/flb_lib.c`) |
|---|---|---|
| L1 start only from created or stopped | **in force** (precondition) | a second start would spawn another engine thread and overwrite `config->worker` (:969-973) |
| L2 the first stop after a start that returned 0 joins that start's thread once | **in force** | 2 paths: the first join at :1058 (status branch) or :1077; the `!ctx->config` arm stays excluded (P-DEREF) |
| L3 a stop after a stop has no effect | **refuted by body** | 2 paths, a second join of the joined thread at :1058 / :1077 |
| L4 a stop after a failed start has no effect | **refuted by body** | 6 paths, each joining: a second join after `do_start`'s own (:984/:999), a join of an identifier no start began (failure before the spawn), or a first join where the reader cannot tell the spawn failed |
| L5 a stop of a never-started context has no effect | **refuted by body** | 2 paths, a join of an identifier no start began at :1058 / :1077 |
| L6 destroy only from created or stopped | **in force** (precondition) | `flb_destroy`'s own body never joins `ctx->config->worker`; of its 482 followed paths 144 reach a join the entry-keyed reader names `flb_lib_worker`, through the chunk-trace pipeline's `flb_stop` of *its own* context (`src/flb_chunk_trace.c:106`, `:270`; P-JOIN-IDENTITY cannot tell the two) |

The refutations are one fact: every feasible path of `flb_stop` joins
`ctx->config->worker`, and nothing clears it. They are withdrawn from every
caller and registered as **candidates** in
`W47-ENGINE-THREAD-JOIN-CANDIDATES` ((i) L4 after a joined failure, (iii) L5
and L4 before the spawn, (v) new: L3, a second `flb_stop`), each missing a
native reproduction.

## 4. The tests against the contract

280 declared-call readings (root × site, 63 distinct sites) at `90978ac6`
and at `90978ac6~1`; **0 violations** at either revision (and at `046accf6`
and `66910c10`). 119 readings are determined on every path; 161 carry an
undetermined path, all from the reader's path insensitivity: `flb_create`'s
own error-path `flb_stop`/`flb_destroy` on its local context (no creator
assigned it), and paths the reader follows past `if (!TEST_CHECK(ctx !=
NULL)) return;` after `test_tail_ctx_create` returned `NULL` (the key has no
creation, or its instance was destroyed at `in_tail.c:245`/`:262`). Every one
of the 46 roots conforms where derived; `W47-LIFECYCLE-PATH-FEASIBILITY`
tracks the rest. A first reading that counted those histories as violations
moved 90 releases (discarded, `private/ll/census-a1`); the undetermined
reading moves none.

* **Fixed test** (`flb_test_inotify_pause_collectors`, `90978ac6`): on every
  derived path `flb_start` (:2559) from *created*, once; `flb_stop` (:2583)
  from *started* (L2, in force) or *failed start* (L4, **refuted: read from
  the body**); `flb_destroy` (:2588) from *stopped*. It never starts twice
  without a stop and calls the stop on the failed-start path too.
* **Old test** (`90978ac6~1`, negative control): `flb_start` (:2486) from
  created, `flb_stop` (:292, in `test_tail_ctx_destroy`) from started / failed
  start, `flb_destroy` (:293) from stopped: it conforms too. Its C5 failure is
  its `unlink` before the stop, which the lifecycle does not touch.

## 5. C5 under the in-force clauses

Every release and every C4 site is byte-identical to the base census once
the added `lifecycle`, `lifecycle_violations` and `lifecycle_stops` keys are
removed, at both revisions ([`scripts/compare_census.py`](scripts/compare_census.py),
[artifacts/compare-fixed.json](artifacts/compare-fixed.json),
[artifacts/compare-previous.json](artifacts/compare-previous.json)): fixed
97 discharged / 134 not discharged, previous 97 / 131, every C4 tally
unchanged.

| Site | `90978ac6~1` | `90978ac6` |
|---|---|---|
| the fixed test's 5 releases after `flb_stop` (`flb_destroy` :2588, `unlink` :2591, `pthread_cond_destroy` :2595, `pthread_mutex_destroy` :2596, scope end :2597) and C4 `cond_destroy` :2595 | absent | **not discharged**: `flb_lib_worker is joined a second time on a path (src/flb_lib.c:1058)`; the stop at :2583 is governed by L2 (in force) after a successful start and by L4 (refuted, withdrawn: read from the body) after a failed one |
| negative control, `unlink` :286 before `flb_stop` | **not discharged** (no join on the path) | absent from the non-threaded test |
| negative control, `flb_destroy` :293 after an unchecked `flb_stop` | **not discharged** (repeated join; a failed join at :1077 on a path) | as at left in the threaded test |

**Final C5 status of the fixed test: not discharged at this path under these
derived premises.** Derived: the `!ctx->config` arm infeasible (P-DEREF);
the engine thread joinable and not self-joined; L2 in force and L3-L5 refuted
by `flb_stop`'s body; the test's lifecycle conforming on every derived path.
Stated, not proved: P-JOIN-IDENTITY, P-LIFECYCLE-INSTANCE, P-RETAIN, P-CHECK,
the lexical reading and binding tables (the creator table included),
acutest's per-test fork. Not derived: the status branch's busy and
join-cycle POSIX cases (so P-POSIX-JOIN does not admit that join), and the 4
(9 at `flb_destroy`) retaining descendants. Even with L4 in force the
failed-start path's remaining join (do_start's unchecked one) would leave
the release not discharged. The old test's C5 stays not discharged, and no
C1 path moves (the loop-owner census is untouched).

## 6. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| L1, L6 | in force (preconditions) | **held** |
| L2 | satisfied (or held if a hop opens the stop's run) | **in force**: nothing unresolved on the stop's run |
| L3, L4, L5 | refuted | **held** at all three trees |
| `flb_destroy`'s body joins no thread | no join on any path | **not held**: its own body never joins `ctx->config->worker`, but 144 of 482 followed paths reach the chunk-trace pipeline's `flb_stop`, whose join the entry-keyed reader names `flb_lib_worker`; L6 stays a precondition either way |
| fixed and old tests conform | yes | **held**, on every derived path; every root has undetermined paths (§4) |
| lifecycle violations in the census | 0 | **held** (0 at all four runs), after the destroyed-key correction of §4 |
| census releases and sites | unchanged, byte for byte | **held** at both revisions |
| negative controls | double start refused, L4 refuted, L5 from the body, destroy before stop refused | **held**; the double start also shows the entry collapse the gate closes |
| module counts | v1 251 → 252, pending review 9 → 10, clause rows 4 → 5 (+ `flb_destroy`) | **held** |
| corpus / system / slice | 0 status moves; leaves reword at calls of `flb_destroy` | **held**: 0 status moves; 1 slice leaf (`flb_create`'s call of `flb_destroy`), none on system or corpus |

## 7. Corpus, system and slice

[`scripts/measure.sh`](scripts/measure.sh) `PHASE TREE` runs the three
arbiters (`--jobs 8`) into fresh directories: BEFORE with this worktree at its
base `ec4c9d4a` (release binary `77bc3a13…`, built before any change), AFTER
with the change (`artifacts/after/binary.sha256`). **BEFORE reproduces all
three baselines** (`docs/reports/w47-loop-owner-instance-flow/integration/artifacts/`):
the corpus `resource-diagnostics.json` is byte-identical, and system and
slice have 0 moved functions, 0 moved units and equal diagnostic counts
([artifacts/identity-before.json](artifacts/identity-before.json); the two
differ from the archive only in run identity). AFTER against the baseline:
[artifacts/identity-after.json](artifacts/identity-after.json). The moved
routes are archived as the new baseline in
[artifacts/after/](artifacts/after/) (system, slice).

| Route | Units / functions | Moves |
|---|---|---|
| corpus (185 units, `66910c10`) | 526 / 10 / 4,298 | **none**: `resource-diagnostics.json` byte-identical to BEFORE |
| system (`046accf6`) | 31 / 2 / 201 | **0 status, 0 leaf**; only the contract environment's identity (`resource_contract_sources` and the per-unit `inventory_sha256`) changes |
| tail-pause slice (`90978ac6`) | 24 Safe / 1 provisional Unsafe / 244 Unknown / 1 Pending (`flb_input`) | **0 status**; 1 leaf rewords, plus the environment identity |

**Attribution.** The one moving leaf is in `flb_create` (`src/flb_lib.c`,
`Unknown` before and after): `call to uncontracted function flb_destroy` →
``call to `flb_destroy` whose declared contract awaits human review
(awaiting-human-review)``, category `unsupported` unchanged, from the new
held `flb_destroy` declaration. No `Safe` function calls `flb_start`,
`flb_stop` or `flb_destroy`, so there is **no Safe → non-Safe move and no
correction**; the `Pending` row (`flb_input`) calls none of them and stays.
The lifecycle clause itself moves no leaf: every declared call is held for
review first, and no `Pending` row carries one, so rule 10i's second
analysis (where the `lifecycle__` refusal would stand) is not reached for
them. **Leaks and violations:** none added or removed on any route; no
provisional `Unsafe` is added.

## 8. Lean

`proof/QCore/CondvarJoin.lean`, section `Lifecycle`, audited in
`proof/AuditCondvarJoin.lean` (**69** audited theorems, 51 → 69; no
placeholders; axioms only `propext`, `Quot.sound`, `Classical.choice`; the
audit count is unchanged, the file is extended):

* the state machine: `starting_twice_is_rejected`,
  `destroying_before_the_stop_is_rejected`, `nothing_follows_a_destroy`,
  `an_accepted_start_began_unstarted`, `an_accepted_destroy_was_unstarted`,
  **`the_paired_sequence_is_accepted`** (the fixed test's sequence on every
  start outcome; a restart after a stop),
  **`a_history_with_a_double_start_is_rejected`**;
* clauses against bodies:
  **`a_no_effect_clause_is_in_force_only_if_every_path_joins_nothing`**,
  `a_no_effect_clause_is_refuted_only_if_every_path_joins`,
  **`the_first_stop_clause_is_in_force_only_if_every_path_joins_the_started_thread_once`**,
  `no_followed_path_holds_the_clause`;
* withdrawal: **`an_in_force_idempotent_stop_adds_no_join`**,
  `an_in_force_no_effect_clause_agrees_with_the_body`,
  `a_withdrawn_clause_reads_the_body`, **`a_refuted_clause_never_discharges`**;
* the gate and the tests: **`a_lifecycle_violation_blocks_the_release`**,
  `without_a_violation_the_release_is_unchanged`,
  **`the_old_and_fixed_test_shapes_stay_not_discharged`**.

## 9. Follow-ups

`followups.json`, applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py). Nothing is
measurably discharged, so nothing is retired. **Amended by measurement:**
`W47-ENGINE-THREAD-JOIN-CANDIDATES` (the lifecycle refutations; candidate
(v), a second `flb_stop`), `W47-LIB-ENGINE-THREAD-REVIEW` (six declarations;
the lifecycle part is now checked, the custody part stays for a reviewer),
`W47-CONDVAR-JOIN-PREMISES` (+P-LIFECYCLE-INSTANCE) and R2 of
`W47-TAIL-PAUSE-BINDING-RESIDUALS`. **Registered:**
`W47-LIFECYCLE-PATH-FEASIBILITY` (161 of 280 readings with an undetermined
path, 46 roots conforming only where derived), `W47-CONDVAR-JOIN-RESPAWN-COLLAPSE`
(a second spawn of a live entry collapses; closed here only for the declared
start), `W47-LIFECYCLE-CREATOR-CONTRACT` (the creator table).

## 10. Gate

[`scripts/gate.sh`](scripts/gate.sh) `rust` then `records`; exits in
[gate/](gate/), figures in [gate.json](gate.json), logs under
`private/ll/gate-1/` and `private/ll/gate-records/` of this worktree. The
Rust, Lean and provider steps ran on the tree of `fbac2c12` (the records
commit after it changes only documents): submodule (`--recursive`) 0; Rust
suite **353 units / 366 summaries, 6,309 passed, 0 failed, 11 ignored**
(exit 0); fmt and clippy 0; both probes 0; **224 Lean audits** passed
(`AuditCondvarJoin.lean` 51 → 69 theorems); provider gate: contracts check,
package build and verify (`qcore.libevent`) 0, **24** provider Python tests
OK, **129** provider tests passed (0 failed, 1 ignored), provider fmt and
clippy 0; `pin --verify-contracts` 0. `pin --check` of the Fluent Bit crate:
0 at `66910c10` and `046accf6` (65 recorded each); **1 at the `90978ac6`
export**: 14 recorded (the base's 13 plus `flb_destroy`), 0 not in force, 0
stale, and 51 in force only through their `046accf6`/`66910c10` identity,
exactly the base's pre-existing condition (`W47-FB-PIN-EXPORT-IDENTITIES`);
this card pins only its own three declarations. The records phase ran the
Python suite and the docs-index check after the matrix and index
regeneration (see gate.json).

**Composition with W47-REVIEW-HOLD-CONCRETE-REASON (landed on `main` at
`c6ea2482` after this base).** In `lib_engine_thread.qv` this card only
*adds* lines: the lifecycle comment block before `flb_start` (base line 50,
+36 lines), one `requires resource_lifecycle(ctx, start);` line after
`flb_start`'s `requires resource_contract_v1;` (base :53), one `requires
resource_lifecycle(ctx, stop);` after `flb_stop`'s (base :61), and the new
`flb_destroy` declaration before `flb_input_thread_instance_exit` (base :66,
+9 lines). No existing marker or clause is replaced: every
`resource_review_pending` marker, including the new
`rewrite_before_acceptance` word on `flb_start`, should survive unchanged
beside the lifecycle clause, and `flb_destroy`'s own marker may take whatever
word C06.1 now requires. In `contracts.lock` it adds the `flb_destroy` entry
(base after :103, +36 lines) and changes the `contract_sha256` of `flb_start`
(base :1296) and `flb_stop` (base :1332) with their re-recorded identities;
after composing, `flb_start`, `flb_stop` and `flb_destroy` must be re-pinned
at the three trees, since the composed contract texts differ from both
sides'.

**Files likely to conflict with a parallel card:** `docs/agents/gateway.md`
(an amendment appended to 10q), `docs/agents/README.md` (the 10q row),
`docs/QCORE_CONTRACT_SPEC_r0_3.md` (§C04.11 added), `docs/STATUS.md`,
`docs/development/increments/wave-47.md`, the acceptance matrix,
`docs/reports/INDEX.md`, `crates/qcore-contract/src/contract_loader.rs`,
`crates/qcore-c-gateway/src/legacy_literal_premise.rs`,
`crates/qcore-c-gateway/src/condvar_census.rs` (hooks),
`crates/quirin-source-analysis/src/condvar_join_census.rs` (hooks) and its
example, `crates/qcore-core/src/lib.rs`, `crates/quirin-source-analysis/src/lib.rs`,
the Fluent Bit crate's `lib_engine_thread.qv`, `contracts.lock` and README,
`proof/QCore/CondvarJoin.lean` and `proof/AuditCondvarJoin.lean`, and the
count pins in `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`
(v1 declarations 251 → 252, pending review 9 → 10, clause rows). Module
counts (corpus 122, selected 58, the resource views) are unchanged.

## Reproduce

```sh
bash docs/reports/w47-flb-lib-lifecycle/scripts/census.sh OUT
python3 docs/reports/w47-flb-lib-lifecycle/scripts/compare_census.py BEFORE.json AFTER.json
python3 docs/reports/w47-flb-lib-lifecycle/scripts/bodies.py ROOT_A ROOT_B
bash docs/reports/w47-flb-lib-lifecycle/scripts/measure.sh PHASE TREE
python3 docs/reports/w47-flb-lib-lifecycle/scripts/identity.py PHASE_DIR SLICE_DIR BASELINE_DIR
target/release/quirin-audit pin --source-root ROOT --crate quirin-provider-fluent-bit \
  --function flb_start --function flb_stop --function flb_destroy
python3 docs/reports/w47-flb-lib-lifecycle/scripts/update_ledger.py
bash docs/reports/w47-flb-lib-lifecycle/scripts/gate.sh LOGS rust
bash docs/reports/w47-flb-lib-lifecycle/scripts/gate.sh LOGS records
```
