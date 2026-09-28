# W47-TAIL-PAUSE-BINDINGS: in_tail pause/resume bindings, negative cases and slice baseline

Card W47-TAIL-PAUSE-BINDINGS (wave 47, round 3; effort medium). This is a
binding census plus a slice measurement. **No analyzer, contract, provider or
Lean code changes.** The input is a design sketch, `in_tail_pause_contracts.md`
revision 1 (SHA-256 `58add9efed43576dc7ae95de8e56c55b45fe4915fcb5493b1f37977a851d51e0`,
the user's untracked file). It proposes concurrency contracts C1–C6 for
Fluent Bit's in_tail pause/resume. The sketch is design-only: nothing in it is
treated as a fact. This card:

1. binds each actor, event and obligation to C source at `90978ac6`;
2. names the sketch's adverse traces as negative cases;
3. measures a new slice so later cards have a baseline;
4. sizes the two implementation cards the sketch implies.

Nothing here is a verified race, an `Unsafe` claim or an upstream defect claim.
Every reject below is a *candidate* for a future checker.

Base: local `main` at `3aed6150` ("W47-SUMMARY-VALIDATION-PROTECTED-FORMALS:
measurements, attribution and records"). That is later than the brief's
`2d7970a4`, which is its ancestor, and this worktree fast-forwarded to it.

## 1. Pins and the 18-commit delta

| Item | Value |
|---|---|
| Fluent Bit commit | `90978ac6a59e96d19621108f63e115eb4cd1c576` ("tests: run tail pause checks on the engine thread"), tree `8d1f3cd78a00fcfb4095a39d3d0b3e7eaac80be4` |
| Parent | `cb8d69ffc4bb7f01c5a75b6a5d73a2ab86e186ff`; parent diff SHA-256 `87800af96d47c9f4b4cea616ae85308a6449211136e0850ffaaee4c3296b4314` (matches the brief; only `tests/runtime/in_tail.c`, +112/−25) |
| Export | `git archive 90978ac6` → `private/w47-tail/attempt-1/export.tar` (SHA-256 `04a0fbeeb6a74b37d82d6a2c2c956974f033196325d3e7d81e73df61e563eab8`), extracted and committed as one synthetic commit `34155955e4962a56d94eb23c03a002880899a0d1` outside the repository; its `git write-tree` equals `8d1f3cd7…` |
| Audited revision | the slice report's `fluent_bit_revision` is the synthetic commit `34155955`, whose tree is `90978ac6`'s |
| Per-file SHA-256 | `bindings.json` → `source.file_sha256` (12 bound files at `90978ac6`) and `previous_test_sha256` (`90978ac6~1:tests/runtime/in_tail.c`, `008e32fc…`) |

The user's checkout was read only through `git rev-parse`, `log`, `show`, `diff`,
`grep` and `archive`, and no untracked file in it was opened. `quirin-audit
prepare` cloned the synthetic export, not the checkout.

**`046accf6..90978ac6` (18 commits, all on the in_tail branch), by bound file.**
Files the sketch binds that the delta does not touch have identical blobs at
`046accf6` and `90978ac6`: `src/flb_input.c`, `src/flb_input_thread.c`,
`src/flb_lib.c`, `src/flb_engine.c`, `plugins/in_tail/tail_fs_inotify.c` and
`lib/monkey/mk_core/mk_event_epoll.c`. `tests/runtime/in_tail.c` at `046accf6`
is also blob-identical to the parent `90978ac6~1` (`2f93a552`). So the previous
non-threaded test is the pinned base's test.

| Commit | Bound files touched | Semantic change on the sketch's paths |
|---|---|---|
| `dc5da02b1` share the open file budget across inputs | tail.c, tail_config.[ch], tail_file.c, new tail_file_budget.[ch], tail_scan_glob.c | adds a process-wide file budget shared by all tail inputs (`budget_lock` plus CFL atomics); tail.c skips the stale-file DB delete when files are deferred and adds the `max_open_files` map entry; no pause/resume path changes |
| `208517f05`, `9402000cb`, `2d6763d51`, `794a070be` | tail_file_budget.[ch], tail.c, tail_config.[ch], tail_file.[ch], tail_scan.c, tail_db.[ch] | budget pressure, reclamation of dormant files from the scan collector, DB deletion by file id; no collector pause/resume change |
| `ed7ffedeb` | win32/* | Windows only, not built |
| `90978ac6` | tests/runtime/in_tail.c | the non-threaded test now runs its pause/snapshot/resume inside a collector callback on the engine thread and publishes through a mutex and condvar, then joins through `flb_stop` before freeing; the threaded test is unchanged |

The other 11 commits touch no bound file (benchmarks and integration tests).

**`flb_input_pause`'s shutdown clause** (`src/flb_input.c:3077-3081`). At
`66910c10` an already buffer-paused instance returned −1. Commit `058c2c3c7`
("input: Handle shutdown notification correctly", between `66910c10` and
`046accf6`) adds a pass-through: while `is_shutting_down`, an instance that
carries `FLB_INPUT_SHUTDOWN_FLUSH` reaches its pause callback again. Only
`in_emitter` sets that flag (`emitter.c:433`), so for in_tail the clause reduces
to the `66910c10` behaviour. Binding B27 records it.

## 2. Binding census

`bindings.json` is written by `scripts/bindings.py`. The script checks that
every binding's `anchor` text still stands on its `file:line`, and refuses
otherwise. There are **60 bindings** and **9 negative cases** across 12 files at
`90978ac6`, plus the parent's test. Threads are derived from the call graph
back to each thread's entry:

* **T**: the test's main thread.
* **E**: `do_start` → `mk_utils_worker_spawn(flb_lib_worker)` →
  `pthread_create` → `flb_engine_start`. E owns `config->evl`
  (`src/flb_engine.c:1150`).
* **I**: `flb_input_thread_instance_init` → `flb_tp_thread_start` →
  `flb_worker_create` → `mk_utils_worker_spawn` → `input_thread`. I owns
  `thi->evl`.

Column abbreviations: NT means a non-threaded input, TH a threaded one.
"Carrier" names the existing kernel, Lean model or contract that could carry
the obligation, or "none".

| Id | C | Element | Location | Function | Thread(s) | Carrier |
|---|---|---|---|---|---|---|
| B01 | C1/C5 | thread E entry | `src/flb_lib.c:969` | `do_start` | T spawns E | task_execution |
| B02 | C5 | thread E entry (pthread_create) | `lib/monkey/mk_core/mk_utils.c:318` | `mk_utils_worker_spawn` | T (engine) or E (input workers via flb_worker_create) | none |
| B03 | C1 | loop ownership L(E): config->evl | `src/flb_engine.c:1150` | `flb_engine_start` | NT: E | event_reactor.rs |
| B04 | C1 | loop ownership L(I): thi->evl | `src/flb_input_thread.c:255` | `input_thread_instance_create` | TH: E creates it (flb_input_init_all on E); I runs it from src/flb_input_thread.c:421 | event_reactor.rs |
| B05 | C1 | collector attachment coll->evl (threaded) | `src/flb_input.c:2251` | `collector_create` | TH: I (for a threaded input, cb_init and cb_pre_run both run on I: src/flb_input_thread.c:369, :409) | none |
| B06 | C1 | collector attachment coll->evl (non-threaded) and the stated ownership comment | `src/flb_input.c:2254` | `collector_create` | NT: E (cb_init and cb_pre_run run on E) | none |
| B07 | C1 | collector register (time) | `src/flb_input.c:2273` | `flb_input_set_collector_time` | NT: E; TH: I | none |
| B08 | C1 | collector register (fd event) | `src/flb_input.c:2294` | `flb_input_set_collector_event` | NT: E; TH: I | none |
| B09 | C1 | collector start | `src/flb_input.c:2335` | `collector_start` | NT: E via flb_input_collectors_start (src/flb_engine.c:1392); TH: I via flb_input_thread_collectors_start (src/flb_input_thread.c:405) | none |
| B10 | C1 | collector running (read) | `src/flb_input.c:2964` | `flb_input_collector_running` | NT: E (revised test callback) or T (previous test); TH: T (threaded test poll) | none |
| B11 | C1/C3 | collector pause | `src/flb_input.c:3158` | `flb_input_collector_pause` | NT: E (revised test) / T (previous test); TH: I | none |
| B12 | C1 | collector delete | `src/flb_input.c:3197` | `flb_input_collector_delete` | NT: E; TH: I | contract libc pthread_mutex.qv |
| B13 | C1/C3 | collector resume | `src/flb_input.c:3218` | `flb_input_collector_resume` | NT: E (revised test) / T (previous test); TH: I | none |
| B14 | C3 | is_ingestion_active early return | `src/flb_input.c:3241` | `flb_input_collector_resume` | NT: E / T; TH: I | none |
| B15 | C3 | is_ingestion_active writers | `src/flb_engine.c:1742` | `flb_engine_stop_ingestion` | E (engine stop path, src/flb_engine.c:1477) | none |
| B16 | C1/C5 | collector destroy | `src/flb_input.c:3140` | `flb_input_collector_destroy` | NT: E (flb_input_instance_destroy from flb_engine_shutdown); TH: E after flb_input_thread_instance_exit joined I | none |
| B17 | C1 | engine dispatch of a collector | `src/flb_input.c:3276` | `flb_input_collector_fd` | NT: E (flb_engine_handle_event, src/flb_engine.c:1002) | event_reactor.rs |
| B18 | C1 | worker dispatch of a collector | `src/flb_input_thread.c:133` | `input_collector_fd` | TH: I | event_reactor.rs |
| B19 | C1 | mk_event_add | `lib/monkey/mk_core/mk_event.c:99` | `mk_event_add` | caller's thread | contract monkey wrapper_paths.qv declares _mk_event_add/_del/_timeout_* for the Unix/Libevent backend only |
| B20 | C1 | mk_event_add (epoll): priority-queue link removal | `lib/monkey/mk_core/mk_event_epoll.c:149` | `_mk_event_add` | caller's thread | none |
| B21 | C1 | mk_event_del | `lib/monkey/mk_core/mk_event.c:132` | `mk_event_del` | caller's thread | contract monkey wrapper_paths.qv declares _mk_event_add/_del/_timeout_* for the Unix/Libevent backend only |
| B22 | C1 | mk_event_del (epoll): priority-queue link removal | `lib/monkey/mk_core/mk_event_epoll.c:180` | `_mk_event_del` | caller's thread | none |
| B23 | C1 | timeout create | `lib/monkey/mk_core/mk_event.c:156` | `mk_event_timeout_create` | caller's thread | contract monkey wrapper_paths.qv declares _mk_event_add/_del/_timeout_* for the Unix/Libevent backend only |
| B24 | C1 | timeout destroy | `lib/monkey/mk_core/mk_event_epoll.c:341` | `_mk_event_timeout_destroy` | caller's thread | contract monkey wrapper_paths.qv declares _mk_event_add/_del/_timeout_* for the Unix/Libevent backend only |
| B25 | C1 | engine loop iteration over the bucket queue | `include/fluent-bit/flb_event_loop.h:74` | `flb_event_priority_live_foreach (macro)` | NT: E (src/flb_engine.c:1454); TH: I (src/flb_input_thread.c:422) | event_reactor.rs |
| B26 | C2 | flb_input_pause | `src/flb_input.c:3074` | `flb_input_pause` | NT: E (revised test callback; flb_engine_shutdown/stop_ingestion via flb_input_pause_all) or T (previous test); TH: T (threaded test) or E (flb_input_pause_all at shutdown) | none |
| B27 | C2 | flb_input_pause shutdown clause | `src/flb_input.c:3078` | `flb_input_pause` | as B26 | none |
| B28 | C2 | flb_input_resume | `src/flb_input.c:3100` | `flb_input_resume` | NT: E or T; TH: T | none |
| B29 | C2 | synchronous plugin pause (non-threaded) | `src/flb_input.c:3034` | `flb_input_plugin_pause` | NT: the flb_input_pause caller's thread; TH: I (handle_input_event) | none |
| B30 | C2 | synchronous plugin resume (non-threaded) | `src/flb_input.c:3054` | `flb_input_plugin_resume` | NT: the flb_input_resume caller's thread; TH: I | none |
| B31 | C2/C6 | threaded pause request: pipe write | `src/flb_input_thread.c:542` | `flb_input_thread_instance_pause` | TH: T or E | none |
| B32 | C2/C6 | threaded resume request: pipe write | `src/flb_input_thread.c:571` | `flb_input_thread_instance_resume` | TH: T | none |
| B33 | C2 | threaded request receive | `src/flb_input_thread.c:53` | `handle_input_event` | TH: I (src/flb_input_thread.c:475) | none |
| B34 | C2/C3 | in_tail pause callback | `plugins/in_tail/tail.c:518` | `in_tail_pause` | NT: as B29; TH: I | none |
| B35 | C2/C3 | in_tail resume callback | `plugins/in_tail/tail.c:550` | `in_tail_resume` | NT: as B30; TH: I | none |
| B36 | C3 | backend dispatch | `plugins/in_tail/tail_fs.h:45` | `flb_tail_fs_pause` | as B34 | none |
| B37 | C3 | inotify backend pause | `plugins/in_tail/tail_fs_inotify.c:457` | `flb_tail_fs_inotify_pause` | as B34 | none |
| B38 | C3 | inotify backend resume | `plugins/in_tail/tail_fs_inotify.c:463` | `flb_tail_fs_inotify_resume` | as B35 | none |
| B39 | C3 | Cfs registration | `plugins/in_tail/tail_fs_inotify.c:432` | `flb_tail_fs_inotify_init` | NT: E; TH: I | none |
| B40 | C3 | Cprogress registration | `plugins/in_tail/tail_fs_inotify.c:444` | `flb_tail_fs_inotify_init` | NT: E; TH: I | none |
| B41 | C1/C2/C3/C4 | test collector callback: disables itself, pause, snapshot, resume, publish | `tests/runtime/in_tail.c:2475` | `cb_tail_pause_test` | NT: E (dispatched by flb_input_collector_fd) | concurrency.rs |
| B42 | C4 | publication under the mutex | `tests/runtime/in_tail.c:2496` | `cb_tail_pause_test` | NT: E | QCore.MutexInvariant |
| B43 | C1 | test collector registration on the owner | `tests/runtime/in_tail.c:2515` | `cb_tail_pause_test_pre_run` | NT: E (flb_input_pre_run_all at src/flb_engine.c:1314, before flb_input_collectors_start at :1392) | none |
| B44 | C5 | plugin copy and test-owned data | `tests/runtime/in_tail.c:2556` | `flb_test_inotify_pause_collectors` | NT: T, before flb_start publishes the instance to E | none |
| B45 | C4 | await with deadline | `tests/runtime/in_tail.c:2565` | `flb_test_inotify_pause_collectors` | NT: T | none |
| B46 | C5 | stop and join | `tests/runtime/in_tail.c:2583` | `flb_test_inotify_pause_collectors` | NT: T | task_execution |
| B47 | C5 | free after join | `tests/runtime/in_tail.c:2588` | `flb_test_inotify_pause_collectors` | NT: T | none |
| B48 | C5 | flb_start | `src/flb_lib.c:1012` | `flb_start` | T | none |
| B49 | C5 | flb_stop | `src/flb_lib.c:1040` | `flb_stop` | T | none |
| B50 | C5 | engine join | `src/flb_lib.c:1077` | `flb_stop` | T joins E | task_execution |
| B51 | C5 | input worker join | `src/flb_input_thread.c:607` | `flb_input_thread_instance_exit` | TH: E joins I (flb_engine_shutdown -> flb_input_exit_all) | task_execution |
| B52 | C4 | worker status publish | `src/flb_input_thread.c:683` | `input_thread_instance_set_status` | TH: I | QCore.MutexInvariant |
| B53 | C4 | worker status await | `src/flb_input_thread.c:699` | `input_thread_instance_get_status` | TH: E (flb_input_thread_instance_init and flb_input_thread_instance_pre_run) | none |
| B54 | C4 | condvar re-init after unlock | `src/flb_input_thread.c:715` | `input_thread_instance_get_status` | TH: E | none |
| B55 | C1 | threaded test: unsynchronized running poll | `tests/runtime/in_tail.c:2440` | `wait_tail_collectors_state` | TH: T reads coll->running written by I | none |
| B56 | C2 | threaded test: pause request from T | `tests/runtime/in_tail.c:2634` | `flb_test_inotify_threaded_pause_collectors` | TH: T | none |
| B57 | C1 | threaded test: context read after start | `tests/runtime/in_tail.c:2629` | `flb_test_inotify_threaded_pause_collectors` | TH: T reads ins->context written by I in cb_init | none |
| B58 | C5 | fixture teardown helper: unlink before stop, stop result ignored | `tests/runtime/in_tail.c:286` | `test_tail_ctx_destroy` | T | none |
| B59 | C1/C2 | previous non-threaded test: pause from the test thread | `tests/runtime/in_tail.c:2497` | `flb_test_inotify_pause_collectors` | NT: T, while E dispatches config->evl | none |
| B60 | C1 | previous non-threaded test: running read from the test thread | `tests/runtime/in_tail.c:2499` | `flb_test_inotify_pause_collectors` | NT: T | none |

Carrier summary: 43 of 60 bindings have **no** carrier. Those that have one
carry only part of the obligation:

* `event_reactor.rs` (5): cross-thread use and synchronization are supplied
  premises.
* the task/spawn-join ledger (4): no C binding.
* `QCore.MutexInvariant` (2): the QCore task route only.
* monkey `wrapper_paths.qv` (4): the Libevent backend only, syntax-preserved.
* `pthread_mutex.qv` (1): custody only.
* Cycle 14 `concurrency.rs` (1): supplied traces.

## 3. Negative cases

`bindings.json` → `negative_cases` lists each case's bound path as binding ids.
"Reachable" means that the pinned test source contains the trace. Each such
trace is a candidate, not an observed failure.

| Case | Trace | Expected | Reachable in the pinned tests |
|---|---|---|---|
| NC1 | old non-threaded test: T:start; E:dispatch ‖ T:pause/resume | reject C1/C2 (T lacks Own(L); `in_tail_pause` → `flb_input_collector_pause` → `mk_event_del` unlinks E's `_priority_head` links on T) | **yes (candidate)**: `90978ac6~1` and `046accf6`, `tests/runtime/in_tail.c:2497` |
| NC2 | revised test: E:callback, pause, snapshot, resume, publish; T:await, stop/join, free | candidate-satisfying under the stated premises | yes: the intended trace at `90978ac6`; no model checks it |
| NC3 | T:enqueue_pause; T:read Cfs.running | reject C1/C2 (enqueue is not completion; unsynchronized read) | **yes (candidate)**: threaded test `tests/runtime/in_tail.c:2634-2636`, `wait_tail_collectors_state` polls `coll->running` 50 × 100 ms |
| NC4 | E:publish done; T:free S; E:return from callback | reject C5 (publication is not join) | no: frees only after `flb_stop` returned 0 |
| NC5 | T:wait timeout; T:free S; E:late callback | reject C5 (timeout is not cancellation) | no: a timeout takes the same `flb_stop` path, and a failed stop exits without freeing (`:2586`); residual R2 |
| NC6 | E:resume while ingestion disabled; T:assume running | reject C3 (missing ingestion premise; `src/flb_input.c:3241` returns 0 without resuming) | no: neither test resumes after stop_ingestion |
| NC7 | one backend collector fails to resume | no conclusion (partial state; both results ignored at `tail_fs_inotify.c:465-466`) | not driven (needs an `mk_event_add` or timerfd failure) |
| NC8 | CV wakes before publication | wait again; no result claim | allowed by POSIX; the revised loop rechecks `test.done` |
| NC9 | fixture removal before join (`test_tail_ctx_destroy`) | reject C5's test-specific fixture rule (unlink at `:286` before `flb_stop` at `:292`, whose result is ignored) | **yes (candidate)**: the threaded test and the previous non-threaded test tear down through it. Added by this census; not in the sketch |

## 4. Slice cut and measurement

**Selection** `contract/profiles/fluent-bit/tail-pause-selection.json` covers
13 files:

* `src/flb_input.c`, `src/flb_input_thread.c` and `src/flb_lib.c`, each in
  static and shared variants;
* the 10 compiled `plugins/in_tail/*.c`.

It excludes, explicitly and with reasons in the file:

* `tail_scan_glob.c` and `tail_scan_win32.c`, which `tail_scan.c` includes
  textually;
* `win32/*.c`, which is MSVC only;
* `tests/runtime/in_tail.c`. The recorded prepare route configures FLB_DEV,
  which enables FLB_TESTS_INTERNAL but not FLB_TESTS_RUNTIME, so the file has
  no compile record. A control run that includes it gives one Unknown unit,
  `environment-unavailable` (`artifacts/control-test-unit.json`).

The selection does not change the profile's 185-unit `audit.json`.

**Route.** The recorded `quirin-audit` route (w45-fb-backlog-custody §4):

1. `scripts/prepare-attempt.sh attempt-1 BINARY SYNTHETIC_REPO 34155955…`
   (exit 0);
2. `scripts/run-current.sh attempt-1 slice-1 BINARY --selection
   contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8` (exit 0).

Both use Clang 21.1.8 and a release `quirin-audit` built at `3aed6150` (SHA-256
`4bb4bb03…`). The archive is `artifacts/slice/`: gzipped files with a
filename-keyed `manifest.json` of raw SHA-256s. The tally is
`artifacts/slice-tally.json` (`scripts/tally.py`).

| Measure | Figure |
|---|---|
| Units | 16 (13 files): **0 Safe / 0 Pending / 1 provisional Unsafe / 15 Unknown** |
| Distinct functions | 271: **24 Safe / 1 provisional Unsafe / 245 Unknown / 1 not projected** (`flb_socket_init_win32`) |
| Function rows (variants) | 404: 34 Safe / 1 Unsafe / 369 Unknown |
| Leaves | unsupported **2,857**, unknown_alias **1,009**, counted_user **1** |
| Contract revision | 127 pinned (127 in force, 0 not in force), 105 libc under the standing assumption; 0 Pending, 0 refuted-by-body |

### 4.1 What is and is not Safe

**Every function this census binds that lies in a selected file is Unknown.**
There are 30 such functions, from `flb_input_pause`/`resume`, the collector
operations and `handle_input_event` to `in_tail_pause`/`resume`,
`flb_tail_fs_inotify_pause`/`resume`, `input_thread_instance_{set,get}_status`,
`flb_start` and `flb_stop` (`slice-tally.json` → `bound_functions_in_slice`).
For example, `flb_input_collector_running` is Unknown on one `unsupported`
leaf and `flb_tail_fs_inotify_pause` on two uncontracted calls.

The 24 Safe functions are getters and pure helpers:

* `collector_id`, `instance_id`, `flb_input_check`, `flb_input_log_check`,
  `flb_input_coro_id_get`, `flb_input_get_cmt_instance`,
  `flb_input_handle_notification` and `flb_input_rate_gate_effective_limit`
  (flb_input.c);
* `in_instance_get`, `out_instance_get`, `filter_instance_get`,
  `flb_lib_free` and `flb_loop` (flb_lib.c);
* `consume_byte`, `consume_bytes`, `drop_bytes`, `stat_get_st_dev`,
  `tail_file_stat`, `utf8_safe_truncate_pos`, `flb_tail_fs_stat_exit`,
  `tail_fs_stat_path`, `flb_tail_mult_destroy`, `tail_mult_append` and
  `flb_tail_scan_register_ignored_file_size` (in_tail).

A Safe verdict here is a **resource-custody** statement under the published
scope and nothing more. `flb_loop` polls `ctx->status`, which E writes with no
synchronization, and it is Safe because it performs no resource operation.
That verdict carries no concurrency meaning. `collector_id` walks
`ins->collectors` without a lock and is Safe on the same custody-only ground.

### 4.2 The provisional Unsafe, classified by reading: artifact

`plugins/in_tail/tail_file_budget.c`, `flb_tail_file_budget_destroy`
(`:123-136`, new in `dc5da02b1`), has three leaves:

* a counted_user leaf, "a user of `budget` dropped without a release check",
  at `:124`;
* an unsupported leaf at `:130`;
* an unknown_alias leaf at `:133`.

The body decrements `budget->users` and, when it reaches zero, clears
`shared_budget` and frees the budget, all under `budget_lock`. Rule 10e's pass
(`legacy_counted_users.rs::counter_write`) matches any `B->users` by **member
spelling** against the counter that `task_users.qv` declares for `flb_task`.
An inline zero test followed by `flb_free` is not a declared `release_check`.
So the C3 shape fires on a correct release, which makes this an **artifact**,
not a real leak and not a premise gap. It sits in a file outside every
concurrency path. It is not an upstream defect claim. Follow-up
`W47-COUNTED-USER-MEMBER-NAME-MATCH` records it. This card adds no analyzer
change, so it adds no verdict. The leaf is part of the new slice's first
measurement.

## 5. Survey verification and gap analysis

**The integrator's survey** (`bindings.json` → `survey` holds the evidence for
each claim):

| Claim | Result |
|---|---|
| no thread/owner/happens-before clause in resource-v1 | held |
| condvars refused under constraint 27 | held (libc `threads.qv` still declares `cnd_wait`/`cnd_timedwait` custody-only) |
| no `pthread_create`/`join`/`cond` contracts | held |
| event_reactor dispatcher-per-base, L04/L05; `sameBaseSingleDispatcher` a Bool premise | held (`EventDriven.lean:131`, assumed at `:149`, `:195`; L05's `synchronized` flag is supplied) |
| none of the sketch's files is in any current selection | **partly held**: `src/flb_engine.c` (engine dispatch, start/shutdown, the `is_ingestion_active` writers) is in `system-selection.json`, and `mk_event.c` is in the 185-unit corpus through the forced-Libevent Monkey build (the libevent backend, not epoll). The rest are in none |
| `flb_input.c:2231-2240` states loop ownership | **held at `66910c10` only** (`2233-2240`); at `046accf6`/`90978ac6` it is `2250-2258` |
| the non-threaded test at `90978ac6~1` pauses from the test thread | held (`:2497`) |
| the threaded test polls `running` unsynchronized, 50 × 100 ms | held (`:2440-2459`) |
| `flb_tail_fs_inotify_pause` ignores both return values | held (`:459-460`; resume `:465-466`) |
| `flb_input_collector_resume` returns 0 without resuming when ingestion is inactive | held (`:3241-3243`) |
| `input_thread_instance_set/get_status` re-inits its condvar after unlock | held (unlock `:712`, destroy/init `:715-716`, unlocked read `:719`). Ordered after the only signal by the mutex hand-off, so it is candidate-satisfying under that premise |

**Gaps per contract.** Each row lists the vocabulary, source-binding,
semantics, Lean and checker gaps.

* **C1, collector access follows the loop owner.**
  * Vocabulary: no thread identity, owner or `Own(L)` capability.
  * Binding: thread sets must be derived from the spawn sites (B01, B02, B04),
    and the loop attachment is `collector_create` (B05, B06).
  * Semantics: the priority-queue links (B20, B22) are in `mk_event_epoll.c`,
    which no selection covers.
  * Lean: `EventDriven`'s single dispatcher is assumed, not derived.
  * Checker: none. The resource pass reads `coll->running` as a scalar.
* **C2, two completion meanings.**
  * Vocabulary: no "enqueued, not completed" distinction.
  * Binding: B31–B33. The live-worker premise has no witness, because a NULL
    `thi` returns 0 (R4).
  * Semantics: no acknowledgement exists to bind.
  * Lean: none. `CallbackTaskHandoff` models deferred-work lifetime, not
    acknowledgement.
  * Checker: none.
* **C3, paired backend collectors.**
  * Vocabulary: a conditional ensures over two collectors' states.
  * Binding: B37–B40, plus the ingestion premise B14.
  * Semantics: both results are discarded inside `void` functions. The
    postcondition is therefore unobservable at their interface, and any
    checker must reason inside the body. `in_tail_pause` leaves three
    collectors running (R3).
  * Lean and checker: none.
* **C4, condvar publication.**
  * Vocabulary: condvar wait/signal is refused (constraint 27). No
    `pthread_cond_*` contract exists.
  * Binding: B41, B42, B45 and B52–B54. The test file is outside the route.
  * Semantics: the mutex edge exists in `QCore.Concurrency`/`concurrency.rs`
    over supplied traces, but wait/recheck/spurious-wakeup semantics are
    absent.
  * Lean: needs a condvar edge.
  * Checker: none.
* **C5, join before release.**
  * Vocabulary: join exists only on the QCore task route. No `pthread_join`
    contract exists.
  * Binding: B46–B51. `flb_stop` ignores its join in one branch (R2), and
    `test_tail_ctx_destroy` removes fixtures before the join (NC9).
  * Lean: a join-release theorem over C threads is missing.
  * Checker: none.
* **C6, request control API.** No API exists at `90978ac6`: the worker pipe
  carries an untagged 64-bit command and no reply. There is nothing to bind,
  so this is registered as a follow-up only.

## 6. Proposed cards

### W47-LOOP-OWNER-CONTRACTS (C1–C3; card-high)

* **Rule.** A collector or loop operation (`flb_input_collector_*`,
  `mk_event_add`/`del`/`timeout_*` on `coll->evl`) discharges its owner
  obligation only on a call path whose derived thread set is exactly
  `{owner(L)}`. Any other thread, or an unresolved one, is Unknown. A threaded
  pause/resume discharges "request enqueued", never "operation applied". A
  resume discharges "running" only under `is_ingestion_active`.
* **Soundness premises, derived and never declared.**
  1. A closed thread-entry inventory: every `pthread_create` reached through
     `mk_utils_worker_spawn`, and a function pointer that cannot be resolved
     is Unknown.
  2. Loop dispatch runs a callback on the loop's owner. This replaces
     `sameBaseSingleDispatcher : Bool` with a theorem over the derived owner.
* **Must reject** NC1, NC3 and NC6. **Must give no conclusion** on NC7.
  **Must accept, as a control,** the NC2 path's collector operations on E.
* **Size.** A new thread-entry census module and a loop-owner vocabulary in
  `quirin-provider-fluent-bit`, which needs a contract-spec amendment. The
  Lean model is an ownership theorem plus a reduction to the reactor's L05. On
  the tail-pause slice it touches 25 bound functions.
* **Frozen-prediction sketch.**
  * Tail-pause slice: no Safe gained, because the new obligation can only
    refuse.
  * 185-unit corpus: it moves only callers of `flb_input_pause`/`resume` in
    `src/flb_input_chunk.c`, which has 3 call sites at `90978ac6` (`:3058`,
    `:3090`, `:3125`; re-locate them at `66910c10`). The card must name those
    callers' current status before it implements anything.
  * System slice: `flb_engine.c`'s shutdown callers
    (`flb_input_pause_all`).

### W47-CONDVAR-JOIN-PUBLICATION (C4–C5; card-high)

* **Rule.**
  1. A wait releases and reacquires its mutex. A read of guarded results is
     licensed only by the predicate observed under the mutex, and each wakeup
     rechecks it.
  2. Storage a callback may retain is released only after a successful join
     of every thread that can reach it. A timeout or a failed join grants
     nothing.
* **Soundness premise.** The joined set covers every thread that reaches the
  storage, taken from the loop-owner card's inventory. So this card follows
  W47-LOOP-OWNER-CONTRACTS.
* **Needs:**
  * a deliberate amendment of constraint 27;
  * `pthread_cond_*` and `pthread_join` contracts in `quirin-provider-libc`,
    pinned;
  * a condvar edge in `QCore.Concurrency` and a join-release theorem.
* **Must reject** NC4, NC5 and NC9. **Must make no result claim on** NC8.
  **Must accept, as a control,** NC2's publish/await/join path and the worker
  status handshake (B52–B54, under the mutex premise).
* **Size.** 7 bound functions in the slice. The test file needs
  W47-TAIL-PAUSE-TEST-UNIT's route, or the card's own fixtures.
* **Frozen-prediction sketch.** No corpus or system move is expected, because
  none of their units calls `pthread_cond_*` or `pthread_join` on a verdict
  path. Measure that before implementing rather than assume it.

### W47-TAIL-REQUEST-CONTROL-API (C6)

Registered as a follow-up only. It needs an upstream API first, proposed
separately and never made by editing the audited input.

## 7. Follow-ups

`followups.json` holds the entries, and `scripts/update_ledger.py`
(idempotent) registers them. The ledger goes from 537 to 543, with none
retired:

* `W47-LOOP-OWNER-CONTRACTS`;
* `W47-CONDVAR-JOIN-PUBLICATION`;
* `W47-TAIL-REQUEST-CONTROL-API`;
* `W47-TAIL-PAUSE-BINDING-RESIDUALS`, R1–R6:
  * R1: collector destroy through `config->evl`;
  * R2: `flb_stop`'s unchecked join and the plain `ctx->status`;
  * R3: three tail collectors stay unpaused;
  * R4: a NULL `thi` returns 0;
  * R5: the ignored `mk_event_timeout_destroy` result;
  * R6: the cross-input file budget, which no C1–C6 obligation covers;
* `W47-COUNTED-USER-MEMBER-NAME-MATCH`;
* `W47-TAIL-PAUSE-TEST-UNIT`.

## 8. Gate

See `gate.json` and `gate/`, and `scripts/gate.sh` for the commands. No
`providers/` path changes, so the provider gate is not run.

## Reproduce

```sh
git -C FB archive 90978ac6a59e96d19621108f63e115eb4cd1c576 > export.tar   # then extract, git init, add -A -f, commit; check write-tree == 8d1f3cd7…
docs/reports/w47-tail-pause-bindings/scripts/prepare-attempt.sh attempt-N BINARY SYNTHETIC_REPO SYNTHETIC_COMMIT
docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-N RUN BINARY --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8
python3 docs/reports/w47-summary-validation-protected-formals/scripts/archive.py OUT/attempt-N/RUN docs/reports/w47-tail-pause-bindings/artifacts/slice
python3 docs/reports/w47-tail-pause-bindings/scripts/bindings.py EXPORT_ROOT PREVIOUS_TEST docs/reports/w47-tail-pause-bindings/bindings.json
python3 docs/reports/w47-tail-pause-bindings/scripts/tally.py artifacts/slice/resource-diagnostics.json.gz bindings.json artifacts/slice-tally.json
python3 docs/reports/w47-tail-pause-bindings/scripts/update_ledger.py && python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w47-tail-pause-bindings/gate.json
docs/reports/w47-tail-pause-bindings/scripts/gate.sh LOGS rust && docs/reports/w47-tail-pause-bindings/scripts/gate.sh LOGS records
```

Here `OUT` is `/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail`.
