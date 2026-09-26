# W45-FB-SYSTEM-SELECTION: the Fluent Bit engine, backlog and retry files in a current audit

Card W45-FB-SYSTEM-SELECTION, 2026-09-26, base `6bd9b383` ("Record the
fb-system-audit-cards integration gate"). It completes P0 (FB-SYSTEM-CENSUS)
of the [system coverage plan](../../plans/fluent-bit-system-coverage/README.md)
as **measurement and selection**. It adds no semantics, no contract and no
verdict promotion. The only code change is an explicit unit selection for
`quirin-audit current`.

## Headline

* **Build.** A fresh Clang build is prepared from a private clone of Fluent
  Bit `864c3d97f3589515285a19396ed8dfc85cda08ae`, clean at that revision. The
  unchanged `cmetrics-static` preparation already yields compile records for
  all eight files: 15 records, and all 15 preprocess. No preparation change
  was needed.
* **Selection.** `quirin-audit current --selection FILE` (new) runs
  `contract/profiles/fluent-bit/system-selection.json`. It audits the eight
  files as a separate slice over the unchanged `fluent-bit` profile, and both
  sidecars name the selection. The pinned 185-unit corpus arbiter still
  reproduces the baseline's diagnostics byte for byte. A base-commit control
  run in the same worktree gives the same five artifacts, byte for byte.
* **Denominator.** 8 files, 15 units (static and shared variants kept):
  **0 Safe / 3 provisional Unsafe / 12 Unknown / 0 data-only**, 0 gateway
  failures, 0 absent or unprepared units. Over the 236 distinct functions:
  **25 Safe / 2 provisional Unsafe / 206 Unknown / 3 not projected**, identical
  in every static/shared pair.
* **`flb_input_chunk.c`.** 64 of 66 functions analysed: 9 Safe, 55 Unknown. It
  matches the current corpus baseline in every function's status and
  diagnostic multiset. It differs from the older census archive in diagnostics
  only, in 7 functions, with no status change. The two analyzer merges between
  those archives account for all 7.
* **Lifetime inventory.** [`lifetime-inventory.json`](lifetime-inventory.json)
  has 11 identities, 125 sites and 33 edges (8 borrow, 12 transfer, 8 retire,
  5 unresolved), with line citations at the pin.
* **P1 slices.** One bounded slice per card: 14 functions for
  FB-BACKLOG-CUSTODY and 15 for FB-RETRY-LIFETIME, with 10 and 11 predicted
  obligations. The two slices share no function. In neither slice does any
  single refusal class clear more than one function by itself.

The two provisional Unsafe functions are provisional. They are not validated
defects and not upstream findings (see §3). Every source-reading candidate
below is a hypothesis from source reading. None is a reproduced behaviour.

## 1. Build and evidence binding

`scripts/prepare-attempt.sh attempt-1 BINARY` runs `quirin-audit prepare`
with `--output /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-system-selection/attempt-1`,
`--revision 864c3d97…`, `--clang /usr/lib/llvm-21/bin/clang` and
`--clangxx /usr/lib/llvm-21/bin/clang++`. It ran with the base-commit release
binary and exited 0 (`artifacts/prepare-attempt-1.{log,exit}`). The user's
checkout was only read. The clone uses `--no-hardlinks` and has no object
alternates. `build/cycle19` was not touched.

[`binding.json`](binding.json) (`scripts/bind.py`) records:

| Item | Value |
|---|---|
| Clone `HEAD` / tracked status | `864c3d97f3589515285a19396ed8dfc85cda08ae` / empty |
| `prepare.json` | `7c6eadc22976699b8160793c8423e292676d1e531f3f7e9d0f4b6c31956236a5` |
| `build-provenance.json` | `b384dc814b6e61dd0f7279365f9b3badda1499b925b5271c58ed1353c96b179b` |
| `compile_commands.json` (1,415 entries) | `e8774afbe44c1d09bd7143ec1a1f3221a48fbcc5ef23fd8c4fcaf182006ded59` |
| Compiler / CMake | Ubuntu clang 21.1.8 (`/usr/lib/llvm-21/bin/clang`, C++ the same driver) / CMake 4.2.3 |
| Configure | `-DCMAKE_C_STANDARD=17 -DCMAKE_C_FLAGS:STRING=-std=gnu17 -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DFLB_DEV=ON -DFLB_EXAMPLES=ON`, target `cmetrics-static -j2` |
| Audit binary (commit `475bb547`) | `3720229dd5310e03ac40cb9087df5147cfb6ff90247a6994d377abc6208af82b` |

`binding.json` also keeps the 15 compile records of the eight files,
unmodified, with their digests. CMake exports records for every configured
target, so `sb.c` has 1 record (the `flb-plugin-in_storage_backlog` object).
The seven core files have 2 each (`fluent-bit-static` and `fluent-bit-shared`).
All 15 preprocess with the records' own commands. The generated headers they
need exist after the `cmetrics-static` build, so no `qcore-audit` preparation
change was needed.

No Fluent Bit binary was built for execution, and nothing was run natively.
This card has no native observation.

## 2. The explicit selection (`qcore-audit` change)

`--selection FILE`, or `"selection"` in a current configuration (both forms
must agree), replaces the profile's `audit.json` groups with exact files.
`crates/qcore-audit/src/unit_selection.rs` validates it; the route and the
CLI read it. The rules:

* The profile itself is unchanged: its pins, library entries and manifest.
  The selection is not a dependency of any unit's identity, so a unit keeps
  its `compile_configuration` under either route. Only the report label
  differs.
* Every distinct configuration of a selected file is its own unit. The
  static/shared variants stay separate, and the denominator reports them per
  unit. They are not deduplicated.
* A selected file with no compile record, or missing from the checkout, stays
  in the denominator as one `Unknown` unit with `environment-unavailable`. It is
  never dropped and never refuses the run. (No such unit arose here.)
* Refusals: a malformed file, a non-plain or duplicate path, a library the
  profile does not declare, or a conflict between the two forms. Each refuses
  before any report is written.
* `.invocation.json` (`selection`) and `.contracts.json` (`unit_selection`)
  record the selection's path, SHA-256 and contents. A run without a selection
  writes neither key, so its records are unchanged.

Tests: three unit tests in `unit_selection.rs`, plus two CLI tests in
`tests/current_cli.rs`. The first CLI test covers order, both absent kinds,
the sidecars, and identity equality with the profile route. The second covers
agreement between the forms, conflicts, a foreign library and path escape.

**Reproduction from the command line** (`scripts/run-current.sh`, from the
repository root, with `QCORE_CLANG=/usr/lib/llvm-21/bin/clang`):

```sh
quirin-audit current --checkout OUT/fluent-bit-clang-source \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database OUT/fluent-bit-clang-production/compile_commands.json \
  --selection contract/profiles/fluent-bit/system-selection.json \
  --report OUT/system-slice/resource-diagnostics.json --jobs 8
```

The command exited 0 with `fluent-bit-system 864c3d97… (current checkout)` and
`8 source file(s), 15 configuration(s): 0 Safe / 3 provisional Unsafe /
12 Unknown / 0 data-only / 0 gateway failures`. `--jobs 1` gives the same
report and `.contracts.json`, byte for byte (`artifacts/system-slice/jobs8-vs-jobs1.txt`).
The archived outputs are `artifacts/system-slice/`: the gzipped report and
contracts sidecar, the invocation record and the console.

**Corpus unchanged.** Three checks:

* The configured main-profile route (185 files, 186 configurations, with
  Monkey) was run over the same attempt with the base binary and with this
  card's binary. Every output except the invocation record is byte-identical
  (`artifacts/main-profile/base-vs-card.txt`). The invocation record differs
  only in its argv and paths. The run gives 4 Safe / 10 provisional Unsafe /
  168 Unknown / 4 data-only.
* The pinned arbiter (`run_audit.py`, `--pinned-evidence`, default job count)
  exited 0. `baseline_identity.py` against
  `docs/reports/w45-sub-object-stores/integration/artifacts` exits 0:
  `resource-diagnostics.json`, `chunk-contracts` and `monkey-source.json` are
  byte-identical, and `verdict_bearing_identical` is true. With the baseline
  worktree's root aliased, `contracts.json` is identical too. `source-effects`
  differs in 53 leaves, all digests of path-bearing configuration.
* A control run of the arbiter example built from the base commit `6bd9b383`,
  in this worktree, produces all five artifacts byte-identical to this card's
  run (`artifacts/arbiter/control-vs-card.txt`). So the `source-effects`
  difference is the worktree path, not this change.

## 3. Denominator

[`measurement/denominator.json`](measurement/denominator.json)
(`scripts/measure.py`) lists every unit and every function, with its status
and, for Unknown and Unsafe, its blocker classes and first blocker. A blocker
class is a diagnostic category plus its description with the names
abstracted.

| File | Units | Functions | Safe | Prov. Unsafe | Unknown | Not projected |
|---|---:|---:|---:|---:|---:|---:|
| `plugins/in_storage_backlog/sb.c` | 1 | 20 | 2 | 1 | 17 | 0 |
| `src/flb_input_chunk.c` | 2 | 66 | 9 | 0 | 55 | 2 |
| `src/flb_storage.c` | 2 | 21 | 2 | 0 | 19 | 0 |
| `src/flb_task.c` | 2 | 19 | 0 | 0 | 19 | 0 |
| `src/flb_engine.c` | 2 | 33 | 4 | 0 | 28 | 1 |
| `src/flb_engine_dispatch.c` | 2 | 5 | 0 | 0 | 5 | 0 |
| `src/flb_scheduler.c` | 2 | 34 | 5 | 0 | 29 | 0 |
| `src/flb_output.c` | 2 | 38 | 3 | 1 | 34 | 0 |
| **Total** | **15** | **236** | **25** | **2** | **206** | **3** |

Each unit Unknown carries `unsupported-syntax`, and each function row carries
the refusals behind it. Over the Unknown and Unsafe functions, the most common
classes are:

| Blocker class | Functions |
|---|---:|
| indirect memory access outside the projection | 164 |
| call to an uncontracted function | 116 |
| call, aggregate or indirect access in a branch condition | 106 |
| unresolved resource identity (alias) | 98 |
| braced initializer, expression or declaration | 74 |
| `for` condition (mostly `mk_list_foreach`) | 57 |

The 3 not-projected definitions are the `#else` stubs of
`sb_segregate_chunks` (`flb_engine.c`), `sb_get_releasable_output_queue_space`
and `sb_release_output_queue_space` (`flb_input_chunk.c`). This build defines
`FLB_HAVE_IN_STORAGE_BACKLOG`, so those stubs are compiled out. They are
configuration exclusions: they stay in the denominator, and their live
definitions are the `sb.c` rows.

**The two provisional Unsafe functions** follow one shape.
`sb_append_chunk_to_segregated_backlog` (leak: acquired at `sb.c:269`,
reported at `:289`) and `flb_output_task_queue_enqueue` (acquired at
`flb_output.c:288`, reported at `:302`) each link a fresh node with
`mk_list_add` into a list reachable from a parameter, then return. The
checked-in `monkey/intrusive_list.qcore` declares `mk_list_add` borrowed on
both positions, so the publication is unmodeled and the object is still
accused at the exit. This is the registered class
`MK-LIST-ADD-BORROWED-PUBLICATION`, now measured at 2 further leaves outside
the 185-unit corpus. Neither is a defect claim. Each function also carries
other refusals, so modelling the publication would make it Unknown, not Safe.

**`flb_input_chunk.c`** ([`measurement/input-chunk-comparison.json`](measurement/input-chunk-comparison.json)).
Each function is compared on its status and on the multiset of
`(category, description, original line)` of its diagnostics. Preprocessed
line numbers and file-name spelling are excluded: the pinned route spells
the file by basename and preprocesses other headers. The sources compared:

* the current pinned baseline (`w45-sub-object-stores`, 66910c10);
* the census archive (`w45-return-expression-families`, 66910c10);
* the configured main-profile run at 864 (static and shared);
* this slice at 864 (static and shared).

All six columns have 66 source functions, 64 analysed, the same two not
projected, 9 Safe / 55 Unknown, and unit Unknown `unsupported-syntax`. The
file's bytes are the same at both revisions (census report). Against the
current baseline, the four 864 columns differ in **no** function. Against the
census archive, 7 functions differ in diagnostics only, and none in status:

* `append_to_ring_buffer` (a branch-condition refusal at :3601 became an alias);
* `destroy_chunk_raw`, `flb_input_chunk_prefers_physical_delete` and
  `flb_input_chunk_ring_buffer_cleanup` (branch-condition and indirect-access
  refusals removed);
* `flb_input_chunk_get_direct_routes` (10 indirect-access refusals removed);
* `input_chunk_metadata_view` (4 removed);
* `flb_input_chunk_map` (one alias added at :1512).

Both archives are pinned-route runs of the same source. The differences lie
between two analyzer merges, W45-CALLEE-SUMMARY-CASCADE (`7546ff88`) and
W45-SUB-OBJECT-STORES (`7de5edf6`), and the 864 runs agree with the later
archive. The census's "64 functions: 9 Safe / 55 Unknown" still holds.

## 4. Lifetime inventory

[`lifetime-inventory.json`](lifetime-inventory.json) is **source reading at
the pin**. Each cited file is bound by SHA-256; the eight system files'
digests equal the earlier reports'. It is not an audit verdict, a model or a
native observation. It holds:

* **Identities (11):** chunk (`struct cio_chunk`), input-chunk wrapper,
  backlog wrapper (`struct sb_out_chunk`, with its `sb_out_queue`), task,
  task route, retry, scheduler request, scheduler timer, output flush,
  singleplex queue entry, and task-map slot. Each has an identity key, an
  owner, and acquisition, publication, removal and destruction sites
  (125 site entries).
* **Edges (33):** the engine event loop, the output-worker handoff (pipe),
  scheduler timers, the backlog collector, storage and shutdown. Each edge is
  classified as borrow (8), transfer (12) or retire (8) where the source
  decides it, with the deciding lines. The other 5 are unresolved, each with
  its missing premise:
  * E-BL-05: after a NULL `flb_input_chunk_map` return, whether the chunk is
    still owned depends on which arm failed;
  * E-BL-09: shutdown re-segregation;
  * E-BL-11: backlog teardown after output instances are freed;
  * E-RT-04: the completion key has no generation;
  * E-SH-01: worker exit when the stop write fails.
* **`unresolved_summary`** also names three shared-state premises the source
  does not establish:
  * engine-to-worker writes to shared task state;
  * `task->users` as a plain `int` read on the chunk-append thread;
  * eviction running on the engine thread.

[`measurement/inventory-audit-join.json`](measurement/inventory-audit-join.json)
joins the 69 functions that sites and edges name with this audit:

| Audit status | Functions |
|---|---:|
| Unknown | 56 |
| Safe (`sb_allocate_chunk`, `schedule_request_wait`) | 2 |
| Provisional Unsafe (the two above) | 2 |
| Named in two of the eight files (`sb_segregate_chunks`, `sb_release_output_queue_space`: live in `sb.c`, compiled-out stubs elsewhere) | 2 |
| Outside the eight files (header inlines or other units, for example `flb_task_users_release` and `flb_output_flush_create`) | 7 |

So no inventory edge is backed by an audit Safe beyond those two leaf helpers.

**Source-reading candidates** (in `failure_outcomes`). These are hypotheses,
not reproduced and not upstream findings. The inventory re-checks the earlier
candidates C1–C4 and A1–A4 at the pin, and adds new ones. This card re-read
the six below at the pin:

* **BC-1.** When the `routes_mask` allocation fails, `flb_input_chunk_map`
  closes the chunk (`flb_input_chunk.c:1512-1516`). `cb_queue_chunks` then
  downs and closes the same chunk (`sb.c:691-703`).
* **BX-1.** `sb_destroy_backlogs` unlinks a queue (`sb.c:188`) before
  `sb_destroy_backlog` looks the chunk up across the remaining queues, so the
  unlinked queue's own wrappers are not freed.
* **BX-2.** At shutdown, `flb_output_exit` (`flb_engine.c:1648`) frees the
  output instances before `flb_input_exit_all` (`:1650`) reaches the backlog
  teardown. That teardown writes `backlog->ins->fs_backlog_chunks_size`
  (`sb.c:228-237`) when a chunk is still queued for two or more outputs.
* **N1.** A failed multiplex retry flush (`flb_engine_dispatch.c:169-174`)
  destroys the retry without releasing the task. The comment at `:125-126` of
  the same function names that state as one nothing reaps.
* **N2.** A failed threaded thread-pool flush rolls back the user count, but
  `flb_output_task_flush` then returns 0 (`flb_output.c:422-435, 463`).
* **N3.** When the pipe write fails on a synchronous, non-threaded output,
  `flb_output_task_flush` calls `singleplex_flush_next`, which frees the
  in-progress entry (`flb_output.c:396-400`). `flb_output_task_queue_flush_one`
  then reads `queued_task->retry` (`:336`) and advances the queue a second
  time.

**FB-SYSTEM-SYNCHRONIZATION-INVENTORY** is closed for the selected slices:

* production callback and worker-handoff edges enumerated, bound to source
  digests and to this build's audited functions;
* unresolved premises distinguished.

What remains is registered in the ledger entry: the 5 unresolved edges and 3
shared-state premises, the worker loop body in `flb_output_thread.c`
(outside the eight files, read only in the cited ranges), and the
chunk-append callers in `flb_input_log.c`.

## 5. Slices for P1

[`measurement/slice-mechanisms.json`](measurement/slice-mechanisms.json) takes
each slice's functions from the inventory. It ranks blocker classes by the
number of slice functions each one blocks. It also gives a greedy cover: after
each class is added, how many functions would carry no refusal. That count is
an upper bound on reach, never a prediction of Safe. It also lists the
uncontracted callees. The two slices share **no function**. Their shared
mechanisms are list publication and indirect access, so P1.3 can run as a
separate lane, as the plan allows once overlap is measured.

### FB-BACKLOG-CUSTODY: startup segregation and `cb_queue_chunks` in `sb.c`

**Functions (14).** 12 in `sb.c`:

* `sb_segregate_chunks`, `sb_allocate_backlogs`;
* `sb_append_chunk_to_segregated_backlog{,s}`, `sb_allocate_chunk`,
  `sb_destroy_chunk`;
* `sb_remove_chunk_from_segregated_backlog{,s}`;
* `cb_queue_chunks`, `sb_release_output_queue_space`;
* `sb_destroy_backlog{,s}`.

Two callee summaries in `flb_input_chunk.c`: `flb_input_chunk_map` and
`flb_input_chunk_destroy`.

Audit: 2 Safe, 1 provisional Unsafe, 11 Unknown.

**Blocker ranking** (functions blocked out of 14):

| Blocker class | Functions |
|---|---:|
| indirect access | 12 |
| declarations or initializers | 11 |
| `for` condition (`mk_list_foreach`) | 10 |
| uncontracted calls (mostly `sb.c`'s own helpers and `cio_*`) | 9 |
| aliases | 7 |
| branch conditions | 7 |

No class is the sole refusal of any function. The greedy cover clears 1, 1,
2, 4 and 5 functions after the first five classes. So one mechanism alone
has near-zero reach here.

**Obligations a semantic card could decide** (inventory OB-1…OB-10):

* **Decidable with list publication and allocation tracking:**
  * OB-1: each wrapper is linked into exactly one queue, or never created.
  * OB-3: at most one wrapper per chunk in each queue.
  * OB-7: eviction retires every wrapper before the chunk is closed. This
    also needs `for`-loop iteration.
  * OB-8: the per-output byte counter equals the sum over its linked
    wrappers. It is shared with FB-BACKLOG-ACCOUNTING.
* **Decidable only with a per-arm callee summary** that tells the close and
  no-close failure arms of `flb_input_chunk_map` apart:
  * OB-5: the chunk is closed exactly once on a NULL map return. The source
    predicts a refutation (BC-1).
  * OB-6: no wrapper is used after a callee frees it.
* **Decidable with loop-carried list membership:** OB-2, partial-insertion
  failure. It is predicted refuted on the `-3` arm (A2).
* **Not decidable from this slice:**
  * OB-9: needs ChunkIO return-code summaries;
  * OB-10: teardown needs the shutdown ordering premise across files (BX-1,
    BX-2).

### FB-RETRY-LIFETIME: the retry route

**Functions (15).**

* `flb_engine.c`: `handle_output_event`.
* `flb_task.c`: `flb_task_retry_{create,destroy,reschedule,clean}`.
* `flb_scheduler.c`: `flb_sched_request_{create,destroy,invalidate}` and
  `flb_sched_event_handler`.
* `flb_engine_dispatch.c`: `flb_engine_dispatch_retry`.
* `flb_output.c`: `flb_output_task_singleplex_enqueue`,
  `flb_output_task_queue_enqueue`, `flb_output_task_queue_flush_one`,
  `flb_output_task_singleplex_flush_next` and `flb_output_task_flush`.

Audit: 1 provisional Unsafe, 14 Unknown.

**Blocker ranking** (functions blocked out of 15):

| Blocker class | Functions |
|---|---:|
| uncontracted calls | 14 |
| indirect access | 14 |
| aliases | 8 |
| branch conditions | 7 |
| declarations or initializers | 6 |

The top uncontracted callees are the selection's own lifetime operations:
`flb_task_users_dec` (7), `flb_task_retry_destroy` (7),
`flb_task_retry_clean` (4) and `flb_sched_request_destroy` (3). Several of
them are themselves Unknown, so the callee-summary route alone clears no
function. The greedy cover goes 0, 2, 6, 7, 9 after the first five classes.

**Obligations a semantic card could decide** (inventory OR-1…OR-11):

* **Counter arithmetic plus a callee failure summary:**
  * OR-1: `task->users` is restored on every failure return of
    `flb_output_task_singleplex_enqueue`. The source predicts a refutation
    (C2).
  * OR-7: `flb_output_task_flush` returns -1 exactly when it added no user.
    Predicted refuted on the threaded arm (N2).
* **Allocation tracking plus scheduler-list membership:** OR-2, every timer
  is linked or destroyed. Predicted refuted (C1).
* **Free tracking inside a callee:** OR-6, `queued_task` is not used after
  the flush fails. Predicted refuted (N3).
* **Interprocedural path enumeration:** OR-3, OR-4, OR-5, OR-8 and OR-11.
* **Needs an event-loop ordering premise:** OR-9.
* **Not decidable from source:** OR-10 needs a generation or a pinned-slot
  invariant (C4).

### Recommended next semantic card

Start with **FB-RETRY-LIFETIME, narrowed to the singleplex queue**: the five
`flb_output.c` functions above, with header summaries for
`flb_task_users_inc`, `flb_task_users_dec` and `flb_task_users_release`, and a
summary for `flb_task_retry_destroy`.

**Why this slice first.**

* It has the smallest blocker sets in either slice: 2 to 4 refusal classes per
  function.
* It holds three obligations decidable in one file (OR-1, OR-6, OR-7).
* It shares the list-publication mechanism with the backlog slice's
  provisional Unsafe (`MK-LIST-ADD-BORROWED-PUBLICATION`).
* W45-FB-RETRY-FAULT-INJECTION is producing native controls for C2 and C3 on
  the same functions.

**Prerequisite mechanisms, by reach in that sub-slice.**

1. Uncontracted calls plus indirect access together clear 2 of its 5
   functions (`flb_output_task_singleplex_enqueue` and
   `flb_output_task_flush`).
2. Intrusive-list publication would withdraw `flb_output_task_queue_enqueue`'s
   provisional accusation.

**Why FB-BACKLOG-CUSTODY comes second.** Its reach needs list iteration
(`mk_list_foreach` conditions, 10 of 14 functions) and declarations
(11 of 14) before its obligations become decidable.

Zero measured reach defers one proposal: a callee-summary-only card for the
retry slice. It clears no function.

## 6. Ledger

* **Closed:**
  * `FB-SYSTEM-CURRENT-BUILD-SELECTION`. A current-revision build was
    prepared and bound. The full file and function denominator and the audit
    results are published. There were no preparation refusals: 15 of 15
    records preprocessed, 0 absent.
  * `FB-SYSTEM-SYNCHRONIZATION-INVENTORY`, for the selected slices, with its
    remainder registered.
  * Neither was registered in the acceptance matrix. The Codex report's
    `followups.json` is frozen, so this report's `followups.json` records
    both closures.
* **Registered:** one acceptance-matrix entry,
  `W45-FB-SYSTEM-SLICE-RESIDUALS`. It names each residual with its discharge
  condition; `followups.json` gives them one by one:
  * the unresolved synchronization premises;
  * the six new source-reading candidates;
  * the list-publication reach measured here;
  * running the P1 slices.
* **Plan:** the queue table gains a Status column. P0 is marked done by this
  card. The plan's text is unchanged.

## 7. Gate

Measured on `475bb547`, the final code. Later commits add only this report,
its evidence and the four shared records. Steps are in `scripts/gate.sh`,
with `CARGO_INCREMENTAL=0`. Every exit went to its own file. The logs are in
the worktree-private `private/w45-fb-system-selection/gate-475bb547/`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 |
| Rust suite (`scripts/run_test_binaries.py`) | exit 0: 292 units, 302 summaries; **5,725 passed**, 0 failed, 11 ignored (`private/test-binaries/20260925T173528Z-299941/`) |
| `cargo clean --profile dev` | exit 0 |
| `cargo fmt --all -- --check` / `cargo clippy --all-targets -- -D warnings` | exit 0 / exit 0 |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| Lean audits | exit 0, **193** passed |
| Root Python tests | exit 0, **351** OK |
| `scripts/build_docs_index.py --check` | exit 0 |

The base integration gate had 5,720 passed. The 5 new tests are the 3 unit
tests and 2 CLI tests above. No provider path changed, so the provider gate
does not apply. After the records were edited, the Python suite and the docs
index were rerun; see `validation-after-records.json`. `clang --version`
is in `clang-version.txt`.

## 8. Boundaries

* **Evidence kinds.** Source reading (§4), the Quirin audit verdicts (§3) and
  the recorded provenance (§1) are separate. There is no model, Lean proof or
  native observation here.
* **Unknowns.** Every Unknown stays in the denominator. No verdict moved. The
  corpus figures are unchanged.
* **The selection** is an audit-scope choice. It is not a completeness claim
  for Fluent Bit and not a whole-library verdict.
* **P1 predictions** are predictions: which obligations a card could decide
  and which refusals block them. They are not results.

## Reproduce

From the repository root. Use a fresh attempt name every time.

```sh
cargo build --release -p qcore-audit
docs/reports/w45-fb-system-selection/scripts/prepare-attempt.sh attempt-N target/release/quirin-audit
docs/reports/w45-fb-system-selection/scripts/run-current.sh attempt-N system-slice target/release/quirin-audit \
  --selection contract/profiles/fluent-bit/system-selection.json --jobs 8
python3 docs/reports/w45-fb-system-selection/scripts/bind.py OUT/attempt-N system-slice --out binding.json
python3 docs/reports/w45-fb-system-selection/scripts/measure.py --slice-report OUT/attempt-N/system-slice/resource-diagnostics.json \
  --main-report OUT/attempt-N/MAIN/resource-diagnostics.json \
  --census-archive docs/reports/w45-return-expression-families/integration/artifacts/diagnostics.json.gz \
  --baseline-archive docs/reports/w45-sub-object-stores/integration/artifacts/diagnostics.json.gz \
  --inventory docs/reports/w45-fb-system-selection/lifetime-inventory.json --out FRESH_DIR
```

Here `OUT` is `/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-system-selection`.
The arbiter uses `docs/reports/w43-return-expression/scripts/run_audit.py`
and `baseline_identity.py`. `scripts/run_audit_binary.py` runs the
base-commit control, and `scripts/identity-alias.sh` runs the aliased
comparison.
