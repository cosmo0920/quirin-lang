# `TSAN-ROWS-2` — contention-forcing ThreadSanitizer schedules on the task route, and the CT46 decision (wave 39)

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Base: `main` at `e878b362`; gate at `ae73b5f9` (§7). Linux arm only (Ubuntu clang 21.1.8 / glibc 2.43 /
x86-64); no Windows or Apple claim is made anywhere in this card
(`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN` stays).

**Headline.** A second `-include` header parks every worker of a task-route
program at creation and releases the cohort at the creator's next join —
**together** (every thread live until every body has run) or **last-created
first, each to completion** — through the runtime's own
`QCORE_TASK_CREATE`/`QCORE_TASK_JOIN` seam, changing no emitted byte. Six
subjects run clean under both forced schedules with stdout byte-identical to
the reference execution and the realised schedule read back from the harness
and asserted; the subjects are the CT fixtures themselves, not twins. The
brief expected a mutation the unforced harness would miss and a forced one
would catch. **Measured, that direction is empty**: every emitted-program
mutation is caught 20 of 20 under all three schedules. The finding runs the
other way, twice: a forced schedule realised with synchronization TSan can see
**hides** the very race it exists to check (the first header did; the shipped
one is uninstrumented for that reason), and a forced harness whose release
trigger is the join **hides every race** when a mutation deletes the join.
CT46 is decided: serial by construction, forcing impossible.

## 1. What is true today (re-derived, not inherited)

* The task runtime creates and joins through `#ifndef QCORE_TASK_CREATE /
  #define QCORE_TASK_CREATE thrd_create` and the same for `QCORE_TASK_JOIN`
  (`crates/qcore-core/src/task_c_emit.rs:722-727`), used at
  `qcore_task_start` (`:885`) and `qcore_task_collect` (`:902`), both
  signature-compatible with `thrd_create`/`thrd_join`. CT07's own
  `refuse_second_spawn.h` already redefines `QCORE_TASK_CREATE`, so the seam
  is an established harness surface. Every worker's trace events go into its
  own private array and the parent replays them at collect (constraint 15a),
  which is why parking a worker moves no stdout byte.
* Wave 37's `tests/support/thread_sanitizer.rs` bound `thrd_create`/`thrd_join`
  to pthread through one `-include`, and its rows were one run each under
  whatever schedule the OS produced, with **no stdout read**
  (`STABILITY-REEVAL-3` §2.4). Its exercised subject was a literal duplicated
  from `write_lend_tasks_exercise.rs:45` and its CT46 row ran the unit file's
  hand-built `fork_join_trace()` (`TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`).
* Wave 36's `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not`
  matched any `SEGV`, had a `(Ok(_), Err(_)) => {}` arm, and an `(Ok, Ok)`
  arm whose assertion could not fail under `halt_on_error=1`
  (`WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`).
* The mutex-invariant backend calls `thrd_create` bare
  (`mutex_c_emit.rs:827`): no `QCORE_TASK_CREATE` seam. Its checked trace is a
  total order with the child's segment contiguous between spawn and join
  (`QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs`).
* CT46's fixture **does** render C through `compile_files`
  (`run_mutex_crossing` asserts it since `MUTEX-SHARE-CROSS-EMISSION`), and
  the CT harness compiles threshold-less programs inside its own stdout-sink
  wrappers — `diagnostic_sink::instrument` for CT07, `mutex_observation::instrument`
  for CT46 — because a program without a `log_level` prints nothing at its
  quiet default (`diagnostic_log_c_emit.rs:108`: "a harness may override both
  macros before this prelude"; the sink's *definition* needs the completed
  event struct, so it follows the text).

## 2. The hook and its byte-identity proof

### 2.1 The hook

`tests/support/thread_sanitizer.rs::SCHEDULE_FORCING`, a second `-include`
after the pthread binding, so its own `thrd_create`/`thrd_join` calls are the
bound ones. `qcore_forced_create` records the worker in a static table
(ordinal = creation order, generation = the creator's join count) and spawns a
trampoline that **parks** until the worker's `released` flag is set;
`qcore_forced_join` **opens the gate** — every worker created before this join
becomes eligible — and then really joins. Two release policies, selected by
`-DQCORE_FORCED_REVERSE`:

| Policy | Release | What holds by construction |
|---|---|---|
| `ForcedTogether` | every eligible worker at once | all of a cohort's workers exist before any body runs; an **exit barrier** keeps every thread of the cohort live until every body of it has returned |
| `ForcedReverse` | the most recently created eligible worker, and the next only when no released body is still running | last-created first, each body to completion, serialized |

The trampoline writes `QCORE_FORCED start N` before the body and
`QCORE_FORCED leave N` after it (after the exit barrier under *together*;
before the next release under *reverse*), parsed into `Sanitized::schedule`,
so a row asserts the schedule it ran under (§3) rather than assuming it.

**Invisible to ThreadSanitizer by construction** (§4.2 is why): every
function is `__attribute__((no_sanitize("thread")))`, all bookkeeping sits
behind an uninstrumented spinlock, and a parked worker spins on a relaxed
flag with `sched_yield`. The only happens-before edges TSan observes in a
forced run are the program's own thread creation and thread join.

`QCORE_TSAN_DROP_JOIN` — the join-edge-removal mutation's stand-in for
`QCORE_TASK_JOIN` — is defined in the binding as a 50 ms wait reporting
success without joining, and **redefined by the forcing header to open the
gate first**. §4.2(b) is why.

### 2.2 Byte identity, checked four ways

1. `git diff --stat` over `crates/qcore-core/src/`, `compiler/src/` and
   `examples/` across the whole card is **empty** (`measure/emission-diff.txt`
   at the gate commit; §7 quotes it).
2. `the_binding_changes_no_emitted_byte`: for all six task-route subjects the
   emitted C is deterministic, mentions none of `qcore_tsan_`, `qcore_forced_`,
   `QCORE_TSAN_DROP_JOIN`, and still carries both
   `#define QCORE_TASK_CREATE thrd_create` and
   `#define QCORE_TASK_JOIN thrd_join`.
3. Both headers reach the compiler only through `-include` flags; a build
   without them cannot see them.
4. **New:** `run_scheduled` reads the translation unit back from disk after
   writing it and returns it as `Sanitized::compiled`; every forced row asserts
   `compiled == c` — the bytes the compiler was handed under a forced schedule
   are the emitted bytes. For a program with no `log_level` (CT07's case,
   CT46's case, wave 36's lease-free companion) `c` is the CT harness's own
   sink wrapper around the emission, and `prepared` asserts the emitted text
   sits **contiguous and unchanged** inside it (`text.contains(c)`).

## 3. Rows (subject × schedule × verdict)

Every row: `races == 0`, `clean` (exit 0, no `ThreadSanitizer:` line),
`compiled == c`, **stdout byte-identical to the reference execution's
projection** (`execute_with_source_lines` — composed sites for a module with
lines, the empty site list for one without), and the schedule trace of the
shape the row names. "Cohort" is the number of workers the creator spawns
before its first join, read off the program; a one-worker cohort realises the
same trace under both policies and the row says so instead of claiming two
schedules. Tests: `every_subject_dies_unbound_at_the_one_fault`,
`every_subject_runs_clean_under_thread_sanitizer_when_bound` (unforced),
`every_subject_runs_clean_under_each_forced_schedule` (forced),
`ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam`.

| Subject (by its own path) | Cohort | Unbound | Bound, unforced | Bound, **together** | Bound, **reverse** | This row means |
|---|---|---|---|---|---|---|
| write-lend flagship, `fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore` (the one file both tests read) | 2 | SEGV `0x18` | clean, stdout = reference | clean; trace `start{1,2}` then `leave{1,2}` | clean; trace `start 2, leave 2, start 1, leave 1` | two disjoint byte writers are not a race under either forced schedule |
| **CT07's own case**, `fixtures/ct/CT07/case.qcore`, `compile_files`, entry `null`, inside `diagnostic_sink::instrument` | 2 | SEGV `0x18` | clean, stdout = reference | clean, as above | clean, as above | the CT07 row's citation (§8, §9) |
| `examples/threads/two_workers` | 2 | SEGV `0x18` | clean, stdout = reference | clean | clean | two workers, no shared object |
| `examples/threads/shared_readers` | 2 | SEGV `0x18` | clean, stdout = reference | clean | clean | three readers under counted shares; the parent's post-join write ordered by the joins |
| `examples/threads/owned_buffer` | 1 | SEGV `0x18` | clean, stdout = reference | clean; `start 1, leave 1` | same trace (one worker) | exclusive transfer |
| `examples/threads/core_context` | 1 | SEGV `0x18` | clean, stdout = reference | clean; `start 1, leave 1` | same trace | owned handoff and join |
| **CT46's own case**, `fixtures/ct/CT46/case.qcore`, inside `mutex_observation::instrument` | — | SEGV `0x18` | clean, stdout = `interpret_mutex_program` | header present, **inert**: no worker released, no trace line, clean | same | serial by construction (§6); the forcing header attaches to nothing |

Each forced row's meaning is exactly "this schedule, forced" (constraint 23's
shape): it is not a claim that the two bodies' store instructions overlapped
in time — nothing outside an emitted body can force that
(`TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`) — and TSan's verdict does not depend
on it, because TSan reasons about happens-before. Under *together* what holds
by construction is that both workers were created and released before either
body ran and both threads stayed live until both bodies had returned; under
*reverse*, that the second-created body ran to completion before the first
started.

## 4. Non-vacuity in both directions

### 4.1 Direction one — a forced schedule reports a broken disjointness

The checker refuses the overlap (`overlapping_leases.qcore` → `Q600`), so the
mutation is on the emitted C, as in wave 37: both workers' byte writes rebased
onto absolute byte `0` (`with_overlapping_writers`). The join-edge mutation
replaces `QCORE_TASK_JOIN(record->thread, &status)` with the gate-opening
no-edge stand-in (`with_join_edge_removed`). Tests:
`overlapping_writers_are_reported_under_every_schedule`,
`removing_the_join_edge_is_reported_under_every_schedule`. The measurement
harness (`measure_catch_rates_for_the_report`, an ignored test; output
`artifacts/catch-rates.json`, run log `artifacts/catch-rates.run.log`) ran each
mutation 20 times per schedule under `halt_on_error=1`:

| Mutation | Unforced | Together | Reverse |
|---|---|---|---|
| overlap, CT07's case | **20/20** | **20/20** | **20/20** |
| overlap, flagship | 20/20 | 20/20 | 20/20 |
| join edge removed, `shared_readers` | 20/20 | 20/20 | 20/20 |
| join edge removed, CT07's case | 20/20 | 20/20 | 20/20 |
| join **deleted without opening the gate**, CT07's case | 20/20 | **0/20** | **0/20** |

Under a forced schedule every caught run also released its workers (the
`workers_released` column of the JSON is 20 wherever the gate opened).
Catch points: the overlap is named inside `qcore_buffer_byte_write` with both
worker stacks; the join-edge race is the parent's post-join read (CT07: byte
reads; `shared_readers`: the write) against the workers' accesses.

### 4.2 Direction two — forcing can hide a race, twice

**(a) A forced schedule realised with synchronization TSan can see hides
worker-versus-worker races.** The first version of the forcing header parked
and released workers on a pthread mutex and condition variable
(`artifacts/schedule_forcing.v1-pthread.h`, never committed, kept as the
record). Under the reverse policy the overlap mutation on CT07's case went
**unreported** — `artifacts/catch-point-reverse-mutex-edge.log`, the run in
which the unforced and together schedules reported it and reverse did not —
because worker 2's byte write preceded its unlock, worker 1 woke by acquiring
the same mutex, and TSan saw a release/acquire edge between the two writes.
Serializing the bodies through visible synchronization *ordered* them. The
shipped header is therefore uninstrumented throughout (§2.1), and with it the
reverse row reports the overlap 20 of 20. The reproduction is shipped in the
mutation harness: re-instrument the header **and** have a released worker take
the spinlock once before its body (`hook_visible_to_tsan`), and the overlap
arm goes red **by missing the race** in every sweep run (three of three):
under reverse deterministically, since the released worker's pre-body
acquire always follows the finished worker's post-write release; under
together whenever the second worker's pre-body acquire lands after the
first's post-write lock cycles. Which of the two the test reaches first
varies (`artifacts/hook_visible_to_tsan.log` is the archived run, a reverse
miss; of the two sweeps before it one missed under together and one under
reverse), and the test stops at the first schedule that misses. Re-instrumenting **alone** (`hook_instrumented_only`)
is timing-dependent and weak: it produced a reverse miss once, in the card's
very first sweep, whose log was not kept, and **0 of 5** in the archived
sweep (`artifacts/hook_instrumented_only.count`) — so no claim rests on it;
the finding rests on the archived first-header catch point and on
`hook_visible_to_tsan`.
Registered as `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`: a rule for any future
forcing harness, indexed by name.

**(b) A release trigger that is the join loses the race with the join.**
Replace the emitted join with the *unforced* 50 ms stand-in, named by its
function so the forcing header's macro does not reach it, and under either
forced schedule no worker is ever released: 0 of 20 reports, the schedule
trace empty, the process exiting with both workers parked. The same text is
the join-edge mutation unforced, reported 20 of 20. Pinned by
`a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing`; it is
why `QCORE_TSAN_DROP_JOIN` opens the gate under forcing, and it is registered
as `TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`.

### 4.3 The direction the brief expected, and what was measured instead

No emitted-program mutation was found that the unforced harness misses:
20 of 20 in every cell of §4.1's first four rows. The reason is the one wave
37's §5.2 stated — ThreadSanitizer detects a missing happens-before edge, not
an observed interleaving, so two unordered accesses that both execute are
reported whatever order the forcing puts them in (the reverse row reports the
overlap although the second writer finished before the first started). What
the schedules measurably change is the **report set**: under `halt_on_error=0`
the `shared_readers` join-edge mutation yields **16** race reports unforced
and **6** under either forced schedule (`race_count_halt_on_error_0` in the
JSON). That also corrects wave 37's "two races" sentence with numbers and
discharges the last item of `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`. What the
forced schedules add is executed-schedule coverage — the discharge
`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES` asks for on its second half ("say which
schedules were covered"). Recorded as `TSAN-FORCED-CATCH-POWER-EQUAL` with the
one candidate class where a difference could exist (TSan's four-cell shadow
history per eight-byte word, which none of this route's programs can exhaust
on the payload word).

### 4.4 The mutation sweep (`artifacts/mutate.sh`, `artifacts/mutations.json`)

Seventeen cases: fifteen arms this card added or changed plus two rule-level
reproductions, each applied, the one named test run, the file restored with
its mtime advanced. **Sixteen are `RED-AS-REQUIRED`**, every verdict read
from the log's own panic line and checked to be the intended assertion rather
than a build failure (a first sweep had two reds for the wrong reason — an
unescaped `&` in a sed replacement and an unused-static `-Werror` — and both
cases were rewritten until red on the assertion); the seventeenth,
`hook_instrumented_only`, is a reproduction attempt that stayed green (0 of
5) and is published as such. The sweep ran three times in the card
(`measure/mutations/` is worktree-private; `artifacts/mutations.json`,
`artifacts/hook_visible_to_tsan.log` and the two `.count` files are the
archived final run, taken on the formatted tree that ships). Two cases are
timing-dependent once mutated and are run five times with the red count
published.

| Case | What is mutated | Named test | Verdict |
|---|---|---|---|
| `ct07_relation` | keep the `io` effect in the normalization | `ct07s_case_is_the_flagship_without_its_logger` | RED |
| `rows_reverse_expectation` | expect the reverse trace in creation order | `every_subject_runs_clean_under_each_forced_schedule` | RED |
| `rows_unbound` | forced rows without the binding | same | RED (the fault) |
| `hook_release_order` **(the rule)** | header releases in creation order | same | RED |
| `hook_exit_barrier` **(the rule)** | exit barrier neutralised | same, ×5 | RED, count in `mutations.json` |
| `overlap_not_applied` | no overlap | `overlapping_writers_are_reported_under_every_schedule` | RED |
| `hook_visible_to_tsan` **(the rule)** | header instrumented + pre-body lock | same | RED — **by missing the race** |
| `hook_instrumented_only` **(the rule)** | header instrumented only, ×5 | same | GREEN, 0 of 5 (a reproduction attempt, not an arm; §4.2(a)) |
| `join_not_removed` | join kept | `removing_the_join_edge_is_reported_under_every_schedule` | RED |
| `gate_opened_after_all` | the deleted join opens the gate | `a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing` | RED |
| `ct46_seam_expected` | expect a seam the backend lacks | `ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam` | RED |
| `ct46_unbound` | CT46 rows unbound | same | RED (the fault) |
| `bound_stdout` | compare stdout against nothing | `every_subject_runs_clean_under_thread_sanitizer_when_bound` | RED |
| `no_byte_change_join` | expect a join default the route does not emit | `the_binding_changes_no_emitted_byte` | RED |
| `wlt_unbound_shape` | expect a different fault address | `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not` | RED |
| `wlt_bound_unbound` | bound arm without the binding | same | RED |
| `wlt_bound_stdout` | bound stdout against nothing | same | RED |

Two arms of the wave-36 repair cannot be driven by mutation on this toolchain:
the `(Ok, Ok)` and asymmetric arms fire only when a toolchain runs the route
unbound. They are `panic!`s naming `TASK-ROUTE-TSAN-PTHREAD-BINDING`'s
discharge, and a `panic!` is not a no-op; that is the whole of their
non-vacuity today and it is stated as such.

## 5. Wave 36's test, repaired

`compiler/tests/write_lend_tasks_exercise.rs::thread_sanitizer_cannot_run_this_routes_programs_lease_or_not`
now asserts each boundary it is for, without deleting the assertion that
unbound TSan cannot run the route:

* **Unbound** (the published `thrd_create`): both the flagship and the
  lease-free companion must die at `SEGV on unknown address 0x000000000018` —
  the `0x18` shape wave 37 pins — not at any `SEGV`. `(Ok, Ok)` panics naming
  the day the interceptor gap closes and the binding's discharge is due;
  `(Err, Ok)` and `(Ok, Err)` each panic naming the asymmetry they would be
  about. No arm is silent.
* **Bound** (`run_native_threads(…, true, …)`, the pthread binding through
  `-include`, no emitted byte changed): both programs must run — under
  `halt_on_error=1` a race is a non-zero exit, returned as `Err`, so the `Ok`
  **is** the race-free claim and the stderr check beside it is labelled the
  redundancy it is — with stdout byte-identical to the reference rendering.
  The lease-free companion, which declares no `log_level`, is compiled inside
  `diagnostic_sink::instrument` with the emitted bytes asserted unchanged
  inside it; its reference takes the default script, since it spawns once and
  allocates nothing.

The flagship literal is gone from both files: `DISJOINT_EXERCISE` is
`include_str!` of the one fixture in both, and `ct07s_case_is_the_flagship_without_its_logger`
pins CT07's case as that program minus `import std.logger;`,
`log_level trace;` and the `io` effect. `run_native` is unchanged for every
other caller (it delegates to `run_native_threads` with `false`).

## 6. The CT46 decision

**Decision (this card takes the second discharge arm of
`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`): the mutex-invariant route's
native evidence stays serial by construction.** The serial schedule is a
property of the checked order, not of the harness: the checked trace is a
total order whose child segment is contiguous between the spawn and the join,
proved as `the_creator_performs_no_event_while_its_child_runs`, so the
faithful rendering has the creator inside `thrd_join` for the whole of the
child's run. Forcing is impossible by construction on two levels, both
measured here: the backend has no `QCORE_TASK_CREATE`/`QCORE_TASK_JOIN` seam
(bare `thrd_create`; asserted on CT46's own emission), so the header attaches
to nothing (no worker released, no schedule line under either policy); and a
hook that did let the creator contend would realise an order the checker never
saw, which would make the row evidence about a different program.

**Whether that retires the entry.** The entry's discharge is a disjunction and
its own text says the second arm "is a legitimate outcome and must be recorded
rather than left implicit". Recording it here meets that arm word for word,
so the entry **may be retired**. This card does not move it, because the brief
asks for the `why` text as a proposal; the first arm (a construct letting two
tasks' trace events genuinely interleave *under the model*) is not left open
by the decision but named as the only thing that could reopen it.

**Proposed `why` text** (in this card's words, for the integrator to apply as
an amendment if the entry is kept rather than retired):

> Card MUTEX-SHARE-CROSS-EMISSION (wave 36) renders a checked fork/join mutex
> trace with real thrd_t children, so CT46's payload is reached by the child's
> own thread. DECIDED by card TSAN-ROWS-2 (wave 39), on this entry's second
> arm: this route's native evidence stays SERIAL BY CONSTRUCTION. The checked
> trace is a total order whose child segment is contiguous between the spawn
> and the join (QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs),
> so the creator is inside thrd_join for the whole of the child's run; a
> rendering that let it contend would realise an order the checker never saw.
> Forcing is impossible by construction and measured so: the backend calls
> thrd_create bare, with no QCORE_TASK_CREATE seam, and the task route's
> forcing header compiled against CT46's own emission releases no worker and
> writes no schedule line (ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam).
> TSan runs on CT46's own fixture lowering, bound, clean, stdout byte-identical
> to interpret_mutex_program's observation, over that serial schedule and no
> other. What would reopen this: a construct that lets two tasks' trace events
> genuinely interleave under the model — a model change with its own card —
> together with a race detector for the emitted C; until then the sanitized
> rows witness acquisition/release pairing across a real thread boundary and
> nothing about contention, and say so.

## 7. Gate

The standing per-change gate from `AGENTS.md`, run **detached** from
`scripts/gate.sh` (`setsid -f`, no `trap '' TERM`; every step's exit in its
own file, no pipe) at **`ae73b5f9`** — the commit carrying every code change
of this card — into worktree-private `private/tsan-rows-2/gate/`, the exit
files and recordings copied to `gate/` beside `gate.json`, which
`scripts/gatejson.py` derives from those files (no figure typed). `df -h`
before launch: 208 GB free. 07:51:25Z → 08:22:39Z (31 min under four sibling
worktrees' builds).

| Step | Exit | Figure |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | 0 | **4,645 passed / 0 failed / 10 ignored** over 234 result summaries; `test result: FAILED` 0 times, so no constraint-16 row needed isolated re-verification and none is claimed |
| `cargo fmt --all -- --check` | 0 (post-fix re-check 0) | clean |
| `cargo clippy --all-targets -- -D warnings` | **101**, then **0** | **deliberately reported**: one `clippy::type_complexity` lint on this card's own repaired test (`write_lend_tasks_exercise.rs`), fixed by a local `type` alias moving no behaviour — the file's 16 tests re-run green (`gate/wlt-post.exit`) — and the whole workspace clippy re-run 0 (`gate/clippy-post.exit`), fmt re-checked 0 (`gate/fmt-post.exit`). The `cargo test` figure is the detached run's; it compiled and ran the same test file before the alias existed |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **137** Lean audits, under the required `lean_audits` key, checked by the generator against `proof/Audit*.lean` |
| Python suite | **1**, then **0** | the detached run's one failure was `CommittedMatrixIsSelfConsistent::test_each_derived_dimension_matches_the_section_it_counts` (`228 != 231`) — the tracker edited and not yet regenerated, `D02-DIMENSIONS-FOLLOWUP-COUNT`'s tripwire doing its job, the identical shape the wave-36/37/38 closes recorded. Regenerated with `--gate docs/reports/tsan-rows-2/gate.json` (`gate/regen.exit`, `gate/regen-final.exit` 0), then **303 OK** (`gate/python-post.exit`, `gate/python-final.exit` 0) |
| `clang --version` | 0 | `present: Ubuntu clang version 21.1.8 (6ubuntu1)`, recorded in `gate/clang-version.txt` and checked by the generator |

Provider **122 carried, not re-measured**: `git diff --name-only e878b362 HEAD -- providers/ crates/qcore-schemas/` is empty (the builder refuses to write otherwise) and `qcore-schemas` is still 14.0.0. Deltas against the wave-38 close's 4,639 / 9 ignored / 303 / 137: **+6 passed** (the six new tests of §4.4's table), **+1 ignored** (the measurement harness), Python +0, Lean +0. Emission diff (`git diff --stat e878b362 -- crates/qcore-core/src compiler/src examples`): **empty**. Nothing is promoted.

## 8. Residuals and retirements

`named_followups` **228 → 231** (+6 −3), applied by
`scripts/register.py` (idempotent; `--show` prints each retired entry's full
text first), `recorded_by: TSAN-ROWS-2` on every registration.

### Retired — each verified registered, then met word for word

* **`TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`**. Discharge, quoted: *"add
  CT07/case.qcore to the TSan subjects and a test asserting
  fork_join_trace().events equals CT46's lowered events (or reword both
  reports beside them to 'a CT46-shaped program' / 'the logger twin'); one
  shared fixture for DISJOINT_EXERCISE; and the 'two races' sentence corrected
  beside the report or a halt_on_error=0 run archived."* CT07's case is a
  subject (§3); the second item is met on its parenthetical arm — the note
  appended beside `docs/reports/task-route-tsan/README.md` rewords both
  phrases, and CT46's **own** case is now a subject by its own path, which is
  stronger than equating the twin; one shared fixture (§5); the count
  archived under `halt_on_error=0` (§4.3, 16 / 6 / 6).
* **`WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`**. Discharge, quoted: *"retire the
  wave-36 row in favour of every_subject_dies_unbound_at_the_one_fault, or
  tighten it to the 0x18 shape and make the (Ok, Err) arm a panic and the
  (Ok, Ok) arm honest about halt_on_error — either with the ground recorded."*
  Tightened (§5), the ground recorded in the test's doc comment and here.
* **`CT07-TWO-THREAD-REPLAY-ORDER`**. Discharge, quoted: *"either accept this
  as a stated boundary of the trace mechanism (the honest default, since the
  buffering exists to make traces comparable at all), or land the TSan row
  TASK-ROUTE-TSAN-SEGV blocks, which is the one instrument that would observe
  the two writers as concurrent rather than as replayed."* The second arm is
  met: the TSan row is landed on CT07's own fixture, unforced and under both
  forced schedules. One incidental claim of the entry is corrected rather
  than propagated, on the wave-37 precedent: TSan does not observe
  concurrency in time (it reasons about happens-before); what this card
  observes is the harness's own schedule trace — both workers created and
  released before either body, both threads live until both bodies returned —
  and the physical overlap of the two stores is still not observed, restated
  as `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`. The integrator may decline this
  retirement on that ground; §11 lists it.

### Registered (six, `recorded_by: TSAN-ROWS-2`)

1. **`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`** — the failure arms (a failed
   `qcore_buffer_new`, a failed spawn) are executed by no TSan row; schedule
   forcing cannot reach them (they are selected by an outcome), and this card
   did not compose the binding with the outcome hooks that can
   (`refuse_second_spawn.h`, a NULL `QCORE_BUFFER_MALLOC`). Discharge: such
   rows, or a permanent statement.
2. **`TSAN-FORCED-CATCH-POWER-EQUAL`** — §4.3.
3. **`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`** — §4.2(b).
4. **`TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`** — §3.
5. **`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`** — §4.2(a).
6. **`CT46-NOTES-PREDATE-TSAN-ROW`** — CT46's `expected.json` notes and
   manifest profile still say TSan is run nowhere in the repository; not
   edited by this card, which owns CT07's citation only. Discharge: a citation
   on the CT07 precedent (`scripts/cite_ct07.py`).

### Left registered, with proposed text for the integrator (§11)

* **`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`** — second arm taken, §6.
* **`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`** — its discharge is *"either state
  it permanently as a measured boundary (the current disposition), or drive
  the rows over the failure arms and over repeated runs, and say which
  schedules were covered."* Repeated runs (20 per cell) and named schedules
  are met; the failure arms are **not** (they are `TSAN-FORCED-SCHEDULE-FAILURE-ARMS`
  now); its CT46 half is decided (§6). Not met word for word, so not retired.
* **`TASK-ROUTE-TSAN-PTHREAD-BINDING`**, **`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN`**
  — unchanged; every forced row compounds the binding with a second
  substitution in the same header, as `STABILITY-REEVAL-3` §4 predicted.

## 9. What this card does not engage

* **No semantic surface moved → no Lean.** No checker, model, emitter or
  diagnostic changed; the Lean audits are the gate's count only.
* **Constraint 14 not engaged**: no word, no token, no lexer or parser change;
  stated rather than assumed (+0 / +0).
* **No new `Q` code**, no condition widened: nothing refuses that did not.
* **CT07's ledger row gains a citation, not a stage flip**
  (`scripts/cite_ct07.py`): one sentence appended to the manifest's
  `semantic_profile` and the fixture's `notes`, the wave-37 sentence kept as
  the state it describes; no `stages` entry moves; `git diff --stat` shows one
  line in each file.
* **Corpus, CT counts, unit/function statuses**: unmoved. Nothing is
  promoted.

## 10. Files and ownership

Owned by this card per the wave brief: `tests/support/thread_sanitizer.rs`,
`compiler/tests/task_route_tsan.rs`. Also touched, none owned by another
wave-39 card: `compiler/tests/write_lend_tasks_exercise.rs` (task 3 of the
brief), the new fixture `compiler/tests/fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore`,
`compiler/tests/fixtures/ct/CT07/expected.json` and
`docs/reevaluation/ct_manifest.json` (the citation), the append beside
`docs/reports/task-route-tsan/README.md`, `docs/reviews/r0_3/acceptance_matrix.json`
(registrations, regenerated with `--gate`), `AGENTS.md` (the count line),
`docs/development/increments/{README,wave-39}.md`. Not touched:
`compiler/tests/ct_corpus.rs` (its `PROOF_GATED_CASES` is
`CT07-TWO-THREAD-PROOF-LEG`'s), `crates/qcore-core/tests/unit/mutex_c_emit.rs`
(its twin row stands as the backend's own), every emitter and every checker.
The two CT support overlays (`diagnostic_sink.rs`, `mutex_observation.rs`) are
read through `#[path]` and unchanged.

## 11. For the integrator

1. **Headline numbers moved in this tree:** `named_followups` 228 → **231**
   (AGENTS.md ledger paragraph edited; the wave-39 record created with the
   index pointer and table row). No CT count, corpus count, leak count or
   status moves.
2. **Matrix**: regenerated with `--gate docs/reports/tsan-rows-2/gate.json`;
   `dimensions.named_followups` derived to 231 (§7).
3. **`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`**: this card takes its second
   arm (§6). Decision for you: retire it (the arm is met word for word and the
   decision is recorded here and in the wave-39 record), or keep it as a
   recorded decision with the proposed `why` text in §6 applied verbatim.
4. **`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`**: proposed amendment to its `why`,
   in these words, if you prefer amendment to leaving it as is: *"AMENDED by
   TSAN-ROWS-2 (wave 39): the second half of the discharge is met — every
   task-route subject runs 20 times per mutation under three named schedules
   (unforced; together, all created and released before any body runs and all
   threads live until all bodies have run; reverse, last-created first to
   completion), the realised schedule asserted from the harness trace — and
   the CT46 half is decided (serial by construction). What remains is the
   failure arms, carried as TSAN-FORCED-SCHEDULE-FAILURE-ARMS."*
5. **`CT07-TWO-THREAD-REPLAY-ORDER`** retirement (§8): decline if you read
   the entry's parenthetical ("observe the two writers as concurrent") as part
   of the condition; the restated boundary `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`
   carries it either way.
6. **`CT46-NOTES-PREDATE-TSAN-ROW`**: a two-text citation on the CT07
   precedent, yours if you take it at the close.
7. **Standing constraints**: none landed, none amended. If a standing rule
   is wanted for "a forced-schedule harness must be invisible to the race
   detector it serves", `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`'s `why` is the
   text; this card did not add one to AGENTS.md.
8. **The finding the brief did not expect** (§4.3): no unforced miss exists on
   these mutations; the value of forcing is executed-schedule coverage and the
   two hazards it surfaces. If the wave's ledger paragraph summarises this
   card, it should say that rather than "forced schedules catch what unforced
   misses".
9. **Wave-37 report**: a labelled note appended beside it (constraint 21),
   nothing above the rule edited.
