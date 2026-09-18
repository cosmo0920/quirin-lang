# Wave 39 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`TSAN-ROWS-2` (2026-09-18).** Contention-forcing ThreadSanitizer schedules
on the checked task route, on the CT fixtures themselves, and the CT46
decision — `docs/reports/tsan-rows-2/README.md`. A second `-include` header
(`tests/support/thread_sanitizer.rs::SCHEDULE_FORCING`) rides the task
runtime's own `#ifndef QCORE_TASK_CREATE`/`QCORE_TASK_JOIN` seam — the seam
CT07's `refuse_second_spawn.h` already rides — parking every worker at
creation and releasing the cohort at the creator's next join, **together**
(every thread live until every body has run, an exit barrier) or
**last-created first, each body to completion**; emission is untouched
(`git diff --stat` over `crates/qcore-core/src/`, `compiler/src/` and
`examples/` empty; every subject's emitted C mentions neither harness and
still carries both hook defaults; the bytes the compiler was handed are read
back and asserted equal to the emitted C under every schedule). The subjects
are no longer twins: the write-lend flagship is read by both
`task_route_tsan.rs` and `write_lend_tasks_exercise.rs` from **one fixture
file** (`compiler/tests/fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore`),
CT07's own case is lowered through `compile_files` and pinned as that flagship
minus its logger, and CT46's own case is a subject by its own path — clean
bound, stdout byte-identical to `interpret_mutex_program`'s observation, and
**unforceable by construction**: the mutex backend calls `thrd_create` bare,
so the header attaches to nothing there (no worker released, no schedule
line, measured), and the checked order keeps the creator in `thrd_join` for
the whole of the child's run. Rows: six task-route subjects × {unforced,
together, reverse} all clean with zero races and stdout equal to the
reference execution (the wave-37 rows read no stdout; these do), the realised
schedule read back from the harness's stderr trace and asserted. **The
finding runs the other way from the brief's expectation.** Every
emitted-program mutation — the overlap on CT07's case and on the flagship,
the join edge removed on `shared_readers` and on CT07's case — is caught
**20 of 20 under all three schedules** (`artifacts/catch-rates.json`), so no
mutation the unforced harness misses was found (`TSAN-FORCED-CATCH-POWER-EQUAL`);
what forcing changes is the report set (16 races unforced, 6 forced, under
`halt_on_error=0`, correcting wave 37's "two races" sentence with numbers) and
what it adds is executed-schedule coverage. Two asymmetries were found in the
forcing direction instead: the **first** header, built on a pthread mutex and
condition variable, made the reverse schedule **miss** the overlap — worker 1
woke by acquiring what worker 2 released after its write, an edge TSan could
see (`artifacts/catch-point-reverse-mutex-edge.log`,
`schedule_forcing.v1-pthread.h`) — so the shipped header is
`no_sanitize("thread")` throughout with an uninstrumented spinlock, and the
reproduction is a shipped mutation (`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`);
and a join deleted **without** opening the gate hides every race (0 of 20
forced, 20 of 20 unforced), pinned by test, which is why the join-edge
stand-in opens the gate under forcing (`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`).
Wave 36's `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not`
now asserts both halves of its boundary: unbound, both programs die at the
`0x18` shape (not any SEGV), with every asymmetry and the "both run" case a
`panic!` naming `TASK-ROUTE-TSAN-PTHREAD-BINDING`'s discharge; bound, both
run race-free with stdout equal to the reference. **CT46 decision** (the
second discharge arm of `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`, taken
and recorded, with proposed `why` text for the integrator): this route's
native evidence stays serial by construction. Mutations: 17 arms, every one
`RED-AS-REQUIRED` on its intended assertion, two of them moving the header
itself, harness and logs shipped (`artifacts/mutate.sh`, `mutations.json`).
No semantic surface moved, so no Lean; constraint 14 not engaged (no word, no
token); no new `Q` code; CT07's row gains a **citation**, not a stage flip;
no Windows or Apple claim (`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN` stays).
Retired, each met word for word: `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`,
`WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`, `CT07-TWO-THREAD-REPLAY-ORDER` (its
second arm, the TSan row landed on CT07's own fixture, with the boundary
restated as `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`). Registered:
`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`, `TSAN-FORCED-CATCH-POWER-EQUAL`,
`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`, `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`,
`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`, `CT46-NOTES-PREDATE-TSAN-ROW`;
`named_followups` 228 → **231**. Gate figures in the report's §7 and
`gate.json`.
