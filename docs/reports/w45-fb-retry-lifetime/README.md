# W45-FB-RETRY-LIFETIME: counted users and container publication, decided on the singleplex queue

Card W45-FB-RETRY-LIFETIME, 2026-09-26, base `74579af6` ("Record the
w45-fb-retry-fault-injection integration gate"). It is the first semantic
slice of P1.3 (FB-RETRY-LIFETIME) of the
[Fluent Bit system plan](../../plans/fluent-bit-system-coverage/README.md),
narrowed as W45-FB-SYSTEM-SELECTION §5 recommends to the five singleplex-queue
functions of `src/flb_output.c`. Two user directions amended the brief while it
ran: the obligations are **checked Fluent Bit provider contracts** decided by
`quirin-audit`, and the Fluent Bit source is **re-pinned to `046accf6`**
(upstream `master`); contracts carry no revision identity.

## Headline

* **Two generic mechanisms**, each with loader, gateway and audited Lean:
  * **Container publication** (contract spec §C04.8, gateway constraint 10d):
    `resource_published(entry, container)` / `resource_unpublished(entry)`.
    `mk_list_add` now publishes a body object into a caller's list; a release
    while linked is a violation candidate; `mk_list_del` restores custody only
    of an object the path published. `MK-LIST-ADD-BORROWED-PUBLICATION`'s false
    accusation of `flb_output_task_queue_enqueue` is gone.
  * **Counted users** (§C04.7, constraint 10e): `resource_user_added/dropped/
    dropped_checked/release_check(p, COUNTER)`, `resource_user_neutral()` and the
    exit clause `resource_user_exit(p, RETURNED, added|restored|dropped)`.
    A per-path pass checks every clause against the body it is declared for and
    applies a declaration of a defined body only where its body validates.
* **The obligations as provider contracts**
  ([`task_users.qv`](../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/task_users.qv),
  [`output_singleplex.qv`](../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/output_singleplex.qv),
  [`monkey/intrusive_list.qv`](../../../providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qv)).
  `quirin-audit` publishes a verdict per clause (`user_count_clauses`):
  * **OR-1 refuted** at `src/flb_output.c:376` (the C2 shape), a provisional
    `Unsafe` candidate under the synchronous-proxy premise;
  * **OR-7 refuted** at `src/flb_output.c:541` (the N2 shape, threaded arm), a
    provisional `Unsafe` candidate reachable by any threaded output;
  * **OR-6 not stated** (Unknown; blocker registered); **OR-2 not reached**.
  None is an upstream defect claim.
* **Corpus (185 units):** functions 390 / 12 / 4,432 → **389 / 12 / 4,433**,
  units unchanged, leak **37** unchanged; 11 leaves removed and 18 added, every
  one attributed. One correction: `flb_input_chunk_create` Safe → Unknown.
* **System slice at `046accf6`:** units 0 / 3 / 12 → **0 / 2 / 13**; functions
  25 / 2 / 207 / 3 → **24 / 2 / 208 / 3**: both publication leaks withdrawn, the
  two OR functions provisional `Unsafe`, one correction (`flb_input_chunk_create`).

## 1. Bases and the re-pin

* **Arbiter BEFORE** (`artifacts/arbiter-before/`): the base binary's run
  reproduces `docs/reports/w45-parenthesised-callee/integration/artifacts/` —
  diagnostics, chunk contracts and Monkey source byte-identical, contracts
  identical after the root alias, `verdict_bearing_identical: true`
  (`source-effects` differs in checkout-path digests only, as in every worktree
  run).
* **Slice BEFORE at `864c3d97`** (`artifacts/slice-before-864c3d97/`, attempt-1):
  reproduces the published W45-FB-SYSTEM-SELECTION report function for function
  and diagnostic for diagnostic (0 / 3 / 12; 25 / 2 / 206 / 3).
* **Re-pin.** The user moved the checkout to `046accf6` (182 commits of
  `master`); attempt-2 is a fresh `prepare --revision 046accf6… --output …`
  (`scripts/prepare-attempt.sh`, exit 0; the clone's `HEAD` is `046accf6`,
  [`binding.json`](binding.json)). Slice BEFORE at `046accf6`
  (`artifacts/slice-before-046accf6/`, base binary): **0 / 3 / 12 units;
  25 / 2 / 207 / 3 functions**; leak 3, `unknown_alias` 661, `unsupported`
  4,331. The difference from `864c3d97` is **source change, not analyzer
  change** ([`artifacts/slice-864c3d97-vs-046accf6.json`](artifacts/slice-864c3d97-vs-046accf6.json),
  [`artifacts/source-diff-864c3d97-046accf6.json`](artifacts/source-diff-864c3d97-046accf6.json)):
  one new function (`flb_output_task_flush_error`, Unknown), `flb_output_task_flush`
  edited, `flb_engine.c` (+36 lines), `flb_engine_dispatch.c` (+30),
  `flb_input_chunk.c` (+62/−5) and `flb_output.h` (+29) edited; every other moved
  row is an origin-line shift. `flb_scheduler.c`, `flb_task.c`, `flb_task.h`,
  `sb.c` and `mk_list.h` are byte-identical.

**What the re-pin changes for the predictions.** The four singleplex helpers,
`flb_task_users_*`, `flb_task_retry_destroy`, `flb_output_thread_pool_flush`
and `mk_list_add/del` have identical bodies at both revisions, so C1, C2 and
OR-1 apply unchanged. `flb_output_task_flush` changed (upstream `09ebf1318` and
the flush-in-flight commits): when no flush can be created it now adds a user,
reports `FLB_ERROR` through `flb_output_task_flush_error` and returns 0, and
returns -1 only if that report's pipe write fails (after rolling back the
user). So:

* **OR-7** is re-derived from the new source: every non-threaded `-1` return
  rolls its user back and every non-threaded `0` return keeps one; the
  **threaded arm is unchanged** — a failed pool dispatch rolls the user back and
  falls through to `return 0` (N2 stands).
* **C3's native run is bound to `864c3d97` and describes code that no longer
  exists as it was:** it forced `flb_output_task_flush` to return -1 at entry,
  and at `864c3d97` the realistic -1 arms were flush creation and the pipe
  write. At `046accf6` flush creation no longer returns -1; only the two pipe
  writes do. The C3 shape (an unchecked drop in `flb_output_task_queue_flush_one`
  then a failed flush) is still in the new source on those arms, but it is not
  decided here (§4) and C3 is used only as the shape of a fixture.
* `flush_in_flight` is incremented exactly on the `0` returns that keep a user;
  it is recorded here, not modelled (a second counter would change no verdict
  on the threaded arm, which increments neither).

The frozen [PREDICTION.md](PREDICTION.md) was written after the re-pin.

## 2. Mechanisms

### 2.1 Container publication (§C04.8, constraint 10d)

`crates/qcore-c-gateway/src/legacy_container_publication.rs`, one thin hook in
`legacy_c.rs`'s `ContractCall` transfer and one in `apply_resource_action`. The
declaration keeps `resource_borrowed` on both positions (ABI projection,
member-address admission) and adds the effect; the schema carries it as
`ResourceFunctionContract::container_effects`, omitted when empty, so every
existing contract serializes byte for byte (qcore-schemas stays 15.x: the new
fields are optional with defaults).

* **Publish** discharges a live, settled, non-null body object into a container
  reached from an unrebound formal (`&P->F…`, `P`) or from a local whose one
  write is its declaration's initialiser from one (`struct flb_sched *sched =
  config->sched;`). The object becomes `ESCAPED` and joins a published set that
  joins by **intersection**.
* **Refusals:** an unnamed entry; any other container; an unknown, ambiguous,
  derived or nullable binding; a non-live identity; a name-only (borrowed) formal
  site. An **unbound formal** entry reports its unresolved identity — W4's signal
  to bind it and derive `Consumed` — so the borrowed launder of constraint 10
  stays closed (`p_link` in the fixtures). An object that owns linked members is
  published with them withdrawn and the call refused.
* **After publication:** a release is `release_after_escape`; a `return` refuses
  (the caller would get a pointer into a container, not custody); an unlink
  restores `LIVE` only for a definitely published site, and is the one use of a
  published object's member address the O04-B09B10 liveness premise admits.
* Every identity-admission route that reads "all modes `Borrowed`" refuses a
  container effect (`has_container_effect`: nested calls, desugared conditions,
  neutral positions, sub-object store values); W4's conflict check compares a
  declaration's custody projection only.

### 2.2 Counted users (§C04.7, constraint 10e)

`crates/qcore-c-gateway/src/legacy_counted_users.rs`, one call at the end of
`analyze_defined_function`. It is the identity for an environment without
counted-user clauses (measured: the corpus arbiter's movement is all
publication).

* Operations are attributed only to an **unrebound formal**; a drop below the
  entry count owes a release check; a rollback of the path's own addition does
  not; after a release check the object may be gone.
* Paths are enumerated over the body's statements; a branch is pruned only when
  its condition compares a stored call result, or a formal a literal premise
  fixes, with a literal. A loop, `switch`, `goto` or label leaves the function's
  clauses undecided.
* Call effects: declared operations and exit clauses (a declaration of a body
  defined in the unit is applied **only if its own clauses hold there**, so
  `flb_task_users_inc`/`dec` are checked in every unit that includes
  `flb_task.h`); `resource_user_neutral()`; a body in the unit derived neutral
  (no counter write, only neutral calls); a declaration with no clause and no body
  here (**UC-FRAME**, stated, not proved); anything else opaque.
* Verdicts per clause: refuted (a known contradicting count on an exit that
  definitely returns the value), undecided (unknown count or value), held. An
  uncovered exit must restore the count; a pending unchecked drop at an exit is
  the C3 shape. Refutations are `counted_user` violation candidates; undecided
  clauses are `unsupported` leaves; every clause's record is published as the
  function row's `user_count_clauses`.

### 2.3 Lean

* `QCore.ContainerPublication` ([`AuditContainerPublication.lean`](../../../proof/AuditContainerPublication.lean),
  18 printed theorems): publication discharges; the borrowed reading accused the
  same body (non-vacuity); a release while published is a violation whatever
  follows and is never `Safe`; unlink restores, and unlink without release
  leaks; other containers, a return of a published identity, a second
  publication and an unlink of an unpublished identity refuse; nothing
  certifies after a refusal or violation; the intersection join.
* `QCore.CountedUsers` ([`AuditCountedUsers.lean`](../../../proof/AuditCountedUsers.lean),
  17 printed theorems): counted-reference preservation (`delta_append`,
  `delta_counts`, opacity is absorbing); a failure arm with a positive residual
  refutes a `restored` clause and is never held; `held` means every returning
  path agrees and an opaque call is never held; the release-check facts (C3
  pending, rollback owes nothing, re-add or check discharges); C2 and N2
  refuted, a balanced body held.

Both are models of the decision, not extractions of the Rust (trust boundary
28); the Rust correspondence is pinned by the fixtures.

## 3. Controls (`crates/qcore-c-gateway/tests/retry_lifetime.rs`, 19 tests)

Two held-out families, `p_*` (list) and `u_*` (counter), unrelated to Fluent Bit:

| Control | Expected | Result |
|---|---|---|
| Publication into a caller's list (`p_enqueue`) | Safe | Safe |
| Same body under the borrowed declaration | leak | provisional Unsafe (leak) |
| Publish, unlink, free (`p_cycle`) | Safe | Safe |
| Release while linked | violation | provisional Unsafe (`release_after_escape`) |
| Publication into a local list | refused | Unknown |
| Helper publishing a borrowed formal, caller frees | not Safe | not Safe |
| Unlink of an object not held | refused | Unknown |
| Return of a published object | refused | Unknown |
| Container through `T *s = formal->f` (`p_wait`) | caller consumed | free after: not Safe; keep: Safe |
| Publication of an object owning a member | refused | not Safe |
| Balanced add/drop with release check, with and without exit clauses | held | Safe, 3 clauses held |
| C2 shape (declared and undeclared) | refuted | provisional Unsafe |
| C3 shape; with the release check on the failure arm | pending drop / no C3 | C3 diagnostic; gone |
| N2 shape (rolled-back addition returned as success) | refuted | provisional Unsafe, no pending check |
| Unattributed operation; opaque call | undecided | Unknown |
| Declared add/drop checked against their bodies (drop under its premise); neutral body | held | held |
| A body contradicting its declared add | refuted | provisional Unsafe |
| Environment without counted-user clauses | untouched | no records |
| Loader: misplaced, non-canonical, duplicate clauses | refused | refused |

## 4. The OR table (system slice, `046accf6`)

Produced by `quirin-audit` (§6); both routes give the same verdicts. The slice
path is reachable in production only through a proxy plugin that sets
`FLB_OUTPUT_SYNCHRONOUS` (no in-tree plugin does; C2 and C3 were
simulations); that premise is part of every verdict on
`flb_output_task_singleplex_enqueue` and on the `SYNCHRONOUS` sub-arms of
`flb_output_task_flush`.

| Obligation | BEFORE | AFTER | Evidence |
|---|---|---|---|
| **OR-1** `flb_output_task_singleplex_enqueue` restores `task->users` on every `-1` return — `resource_user_exit(task, -1, restored)` | Unknown: not stated (function Unknown; `flb_task_users_*` uncontracted) | **decided-refuted**, provisional Unsafe candidate at `src/flb_output.c:376`: `flb_task_users_inc(task)` (held against its header body), `flb_output_task_queue_enqueue` (derived neutral; its `-1` arm calls `flb_task_retry_destroy`, declared neutral), `ret == -1`, `return -1` with +1 user. Matches C2 (native, simulated synchronous output, `864c3d97`; the function bodies are identical at `046accf6`). Premise: synchronous proxy output. | `user_count_clauses`, `counted_user` leaf |
| OR-1 companion — `resource_user_exit(task, 0, added)` | not stated | undecided: holds on `return 0`, undecided on `return flb_output_task_queue_flush_one(…)` (its counter operations act on `queued_task->task`) | `W45-FB-OR1-FLUSH-ONE-UNATTRIBUTED` |
| **OR-7** `flb_output_task_flush` returns -1 exactly when it added no user — `resource_user_exit(task, 0, added)` | Unknown: not stated | **decided-refuted**, provisional Unsafe candidate at `src/flb_output.c:541`: threaded arm, `flb_output_thread_pool_flush` returns -1, the user is rolled back, the output is not synchronous, `return 0` with 0 users added. Matches N2 (source reading; not natively observed). No synchronous premise needed; the pool-flush neutrality is a cross-unit premise. | `user_count_clauses` |
| OR-7 — `resource_user_exit(task, -1, restored)` | not stated | undecided: every non-threaded path passes `flb_output_flush_create`, whose derivation reaches the uncontracted `flb_output_thread_instance_get` | `W45-FB-OR7-FLUSH-CREATE-OPAQUE` |
| **OR-6** `queued_task` not used after the flush fails | Unknown | **Unknown, not stated**: needs the identity of a `mk_list_entry_first` element and a summary that `flb_output_task_singleplex_flush_next` frees it (N3) | `W45-FB-OR6-ELEMENT-IDENTITY` |
| **OR-2** every scheduler timer linked or destroyed | Unknown | not reached (unchanged) | umbrella entry |
| `MK-LIST-ADD-BORROWED-PUBLICATION` in the slice | `flb_output_task_queue_enqueue`, `sb_append_chunk_to_segregated_backlog` provisional Unsafe (publication leak) | both **Unknown**: leak withdrawn, other refusals kept | §5 |

The counter primitives' own clauses are **held** in the output unit
(`resource_user_added(task, users)`, `resource_user_dropped(task, users)` under
the `release_check == 0` premise), and are listed per caller as `callee …`
records. `flb_task_retry_destroy`'s neutral clause is **undecided** where its
body is analysed (`src/flb_task.c`: `flb_sched_request_invalidate` is
uncontracted); in `src/flb_output.c` it is applied as the cross-unit premise
every declaration is (`W45-FB-CROSS-UNIT-USER-PREMISES`). Revision sensitivity:
OR-7's verdict depends on the threaded arm's fall-through to `return 0` and
OR-1's on the enqueue's early `return -1`; both would move if upstream changes
those arms, and the declared-ABI check refuses a changed signature.

## 5. Measurements

### 5.1 The 185-unit arbiter

`run_audit.py … --jobs 6` on `e1259190` (the final code), archived as the new
baseline [`artifacts/arbiter-after/`](artifacts/arbiter-after/) with
[`delta.txt`](artifacts/arbiter-after/delta.txt). Against the BEFORE run:

| | BEFORE | AFTER |
|---|---:|---:|
| Units Safe / Unsafe / Unknown / data-only | 4 / 10 / 167 / 4 | 4 / 10 / 167 / 4 |
| Functions Safe / Unsafe / Unknown | 390 / 12 / 4,432 | **389 / 12 / 4,433** |
| leak / unknown_alias / unsupported / summary_conflict | 37 / 3,567 / 42,699 / 1 | 37 / **3,571** / **42,702** / 1 |

11 leaves removed, 18 added, all attributed:

* **7 unlink refusals** replace the `indirect memory access` refusal of the same
  `mk_list_del(&x->_head)` statement (the object was never published by that
  body): `chunk_state_sync`, `cio_chunk_close` ×2, `cio_stream_destroy`,
  `header_lookup`, `mk_http_thread_destroy`, `mk_http_thread_purge`.
* **`chunk_state_sync`** +2 `unknown_alias ch`: it re-links the formal `ch`; the
  publication of an unbound formal reports it (constraint 10d).
* **`cio_chunk_open`**: the publication of `ch`, which owns linked members, is
  refused (+1), and the later access through the published `ch` reports its
  identity (the `indirect memory access` at the same line is exchanged).
* **`cio_stream_create`** +1: a publication of an object that is not one live
  identity is refused.
* **`flb_input_chunk_create` Safe → Unknown — a correction.** It links `ic` into
  `in->chunks` and returns it; its `Safe` rested on the borrowed reading, under
  which the caller owned an object the input's list also held. Now the
  publication of `ic` (which owns its chunk and routes mask) is refused and the
  return of a published object refuses (+2).
* **`input_chunk_get`**: it lost `flb_input_chunk_create`'s derived summary, so 2
  `nullable resource ic used by a non-null contract` refusals are exchanged for 1
  uncontracted call and 2 unresolved identities.
* **`mk_sched_add_connection`**: the header helper `mk_sched_conn_timeout_add`
  links its formal `conn` into `sched->timeout_queue` on one branch only, so W4
  derives no summary where it derived a borrowed (laundering) one; `unknown_alias
  conn` is exchanged for the uncontracted call.

**Leak pin 37, unchanged.** Every leak leaf is identical: the four corpus leaves
registered under `MK-LIST-ADD-BORROWED-PUBLICATION` are not reached — in each the
linking statement is refused upstream or the container lies inside an object the
body acquired (`W45-CONTAINER-ROOT-REACH`, 4 leaves). No leak or violation leaf
was added, so none needed classification.

### 5.2 The system slice at `046accf6`

Profile route (`scripts/run-current.sh attempt-2 slice-after BINARY --selection
contract/profiles/fluent-bit/system-selection.json --jobs 4`), archived in
[`artifacts/slice-after-046accf6/`](artifacts/slice-after-046accf6/) with
[`summary-vs-before.json`](artifacts/slice-after-046accf6/summary-vs-before.json):

| | BEFORE | AFTER |
|---|---:|---:|
| Units Safe / Unsafe / Unknown | 0 / 3 / 12 | **0 / 2 / 13** |
| Functions Safe / Unsafe / Unknown / not projected | 25 / 2 / 207 / 3 | **24 / 2 / 208 / 3** |
| leak / counted_user / unknown_alias / unsupported | 3 / 0 / 661 / 4,331 | **0 / 4 / 685 / 4,325** |

20 distinct functions move (static and shared rows agree):

* `sb_append_chunk_to_segregated_backlog`, `flb_output_task_queue_enqueue`:
  provisional Unsafe → Unknown (publication leaks withdrawn). The first is the
  concurrent FB-BACKLOG-CUSTODY card's function; it is recorded, not claimed.
* `flb_output_task_singleplex_enqueue`, `flb_output_task_flush`: Unknown →
  provisional Unsafe (OR-1, OR-7).
* `flb_input_chunk_create`: Safe → Unknown (the correction of §5.1).
* 15 Unknown functions exchange refusals: `call to uncontracted function
  flb_task_retry_destroy` → `unknown_alias retry` (the release of a formal the
  contract-free run leaves unbound) in 6; `call to uncontracted function
  flb_task_users_dec` → `outside its declared literal-argument premise` for the
  `FLB_TRUE` sites; unlink refusals for `mk_list_del` of objects not published
  (`flb_output_flush_prepare_destroy`, `flb_output_instance_destroy`,
  `flb_sched_request_destroy`, `flb_task_retry_destroy`); unattributed counter
  operations on locals and container elements (`handle_output_event`,
  `flb_engine_dispatch_retry`, `tasks_start`, `flb_output_task_queue_flush_one`,
  `task_alloc`'s `task->users = 0`); `schedule_request_now` reports the formal
  it links; `retry->attempts++` in `flb_task_retry_create` and
  `flb_engine_dispatch_retry` is now claimed by the formal-mutation gate because
  `retry` reaches a declared release; `input_chunk_get` as in §5.1.

`schedule_request_wait` stays `Safe`, now relative to a derived `Consumed`
summary (it links the caller's `request` into `config->sched`'s list) instead of
the borrowed one.

## 6. Commands (the obligation table's source)

From the repository root, with `OUT=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-retry-lifetime`:

```sh
cargo build --release -p qcore-audit
docs/reports/w45-fb-retry-lifetime/scripts/prepare-attempt.sh attempt-2 target/release/quirin-audit
# the profile manifest route (cycle19-resource-contracts.json, 49 modules)
docs/reports/w45-fb-retry-lifetime/scripts/run-current.sh attempt-2 slice-after target/release/quirin-audit \
  --selection contract/profiles/fluent-bit/system-selection.json --jobs 4
# the provider route
docs/reports/w45-fb-retry-lifetime/scripts/run-current.sh attempt-2 slice-after-providers target/release/quirin-audit \
  --selection contract/profiles/fluent-bit/system-selection.json --load-path providers/contract-load-path.json \
  --provider quirin-provider-fluent-bit --provider quirin-provider-libc \
  --provider qcore-provider-libevent:monkey/intrusive_list.qcore --jobs 4
python3 docs/reports/w45-fb-retry-lifetime/scripts/slice_summary.py $OUT/attempt-2/slice-after/resource-diagnostics.json \
  --before $OUT/attempt-2/slice-before/resource-diagnostics.json
```

`run-current.sh` expands to `quirin-audit current --checkout
$OUT/attempt-2/fluent-bit-clang-source --profile-directory
contract/profiles/fluent-bit --compile-database
$OUT/attempt-2/fluent-bit-clang-production/compile_commands.json --report
$OUT/attempt-2/RUN/resource-diagnostics.json …`. Both runs exit 0. The provider
route loads only the three selections (20 / 2 / 212 functions over the slice) and
publishes the **same** `user_count_clauses` for the three declared functions
([`artifacts/slice-after-providers-046accf6/summary.json`](artifacts/slice-after-providers-046accf6/summary.json)):
`--provider quirin-provider-fluent-bit` alone would leave `malloc` and
`mk_list_add` uncontracted, so the `flb_malloc` body would be opaque. The
no-build route (`quirin-audit contracts`) is not used: a compile database
exists for every selected unit.

## 7. Gate

Measured on `74d30db3` (`scripts/gate.sh`, `CARGO_INCREMENTAL=0`, every exit
to its own file); logs under the worktree-private
`private/w45-fb-retry-lifetime/gate-74d30db3/`; [`gate.json`](gate.json). The
arbiter and slice binaries are `e1259190`'s; `74d30db3` differs in code only
by a clippy needless-borrow fix.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 |
| Rust suite (`scripts/run_test_binaries.py`) | exit 0: 298 units, 308 summaries; **5,815 passed**, 0 failed, 11 ignored (`private/test-binaries/20260926T063018Z-3446288/`) |
| `cargo clean --profile dev` | exit 0 |
| `cargo fmt --all -- --check` / `cargo clippy --all-targets -- -D warnings` | exit 0 / exit 0 |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| Lean audits | exit 0, **197** passed |
| Python suite / `build_docs_index.py --check` | exit 0, **358** OK / exit 0 |
| Earlier attempt (before the pins) | exit 101: 5,801 passed, 14 failed — 13 contract-count pins this card moves, 1 the load-sensitive `standalone_host` row, which passed alone (3/3; constraint 16, not weakened) |
| Provider gate: `cargo test` / `fmt --check` / `clippy -D warnings` (providers workspace) | exit 0 (124 passed) / 0 / 0 |
| `providers/package.py build`, `verify` ×5 | exit 0; 0 ×5 |
| `providers/contracts.py sync`, then `check` | exit 0, 0 |

Pins moved deliberately: the Cycle 19 manifest 47 → 49 modules
(`contract_loader.rs`, `neighbours.rs`, `prod_contracts.rs`,
`provider_selection.rs`), the contract corpus 111 → 113 modules and its clause
table (`qcore_contract_corpus.rs`), the v1 declaration census 155 → 162, the
resource-mode parity table (`mk_list_add`'s publication and the six additions,
listed explicitly), and the multi-module relation registry 12 → 13
(`mk_list_del` in `chunkio/list_unlink.qcore` and `monkey/intrusive_list.qcore`,
registered as `LifecycleProfiles`).

## 8. Ledger

* **Retired:** `MK-LIST-ADD-BORROWED-PUBLICATION` — its discharge condition (a
  vocabulary stating the publication, gateway and Lean parity, an arbiter
  attributing each withdrawn leaf) is met; its 4 corpus leaves, which this card
  does not withdraw, are rehomed with their size in `W45-CONTAINER-ROOT-REACH`.
* **Amended:** `W45-FB-SYSTEM-SLICE-RESIDUALS` — part (4) discharged for the
  slice, part (5) advanced (OR-1, OR-7 decided; OR-6, OR-2 open).
* **Registered** ([`followups.json`](followups.json), `named_followups` 421 → 428):
  `W45-FB-OR6-ELEMENT-IDENTITY` (1 obligation), `W45-FB-OR7-FLUSH-CREATE-OPAQUE`
  (1 clause), `W45-FB-OR1-FLUSH-ONE-UNATTRIBUTED` (1 clause),
  `W45-FB-USERS-RELEASE-CHECK` (4 FLB_TRUE sites), `W45-FB-CROSS-UNIT-USER-PREMISES`
  (2 declarations), `W45-FB-SINGLEPLEX-CUSTODY-DECLARED` (2 declarations),
  `W45-CONTAINER-ROOT-REACH` (4 corpus leaves), `W45-COUNTED-USERS-LOOPS`.
* The corpus row is rewritten from the AFTER artifacts (previous row in
  [`previous-corpus-row.json`](previous-corpus-row.json)).

## 9. Boundaries

* **Evidence kinds.** The verdicts are Quirin's, under the declarations named
  above and the premises UC-FRAME and trust boundary 28; the Lean modules model
  the decisions; C1–C3 are native observations at `864c3d97` used only as the
  shapes of fixtures, and moved no verdict.
* **Provisional.** OR-1 and OR-7 refutations are violation candidates: branch
  feasibility beyond the decided conditions is not proved (for OR-7, that the
  pool dispatch can fail for a non-synchronous threaded output; for OR-1, that
  the enqueue allocation can fail and that a synchronous proxy exists). They are
  not upstream defect claims and nothing was reported upstream.
* **Unknowns stay in the denominator.** No function entered `Safe` from this
  card; the one `Safe` that stayed (`schedule_request_wait`) now rests on a
  validated `Consumed` summary.

Scripts: [`scripts/`](scripts/) (`prepare-attempt.sh`, `run-current.sh`,
`identity.sh`, `slice_summary.py`, `source_diff.py`, `archive_slice.py`,
`archive_arbiter.py`, `delta.py`, `bind.py`, `verify_packages.sh`,
`update_ledger.py`).
