# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`TASK-ROUTE-TSAN` (2026-09-18).** The checked concurrency routes had **no
ThreadSanitizer evidence at all** — wave 36's `WRITE-LEND-TASKS-EXERCISE`
measured TSan killing its lease exercise, wave 31's held-only program and the
published `examples/threads/two_workers` at the identical address `0x18` and
pc, concluded the fault was the route's, and registered
`TASK-ROUTE-TSAN-SEGV`. **It is not the route's.** ThreadSanitizer's runtime
ships an interceptor for `pthread_create` and **none for `thrd_create`**, so on
glibc 2.34+ a thread the emitted runtime creates reaches `start_thread`
unregistered, `cur_thread()` is null in it, and the first instrumented event
faults at `__tsan_func_entry+19` — `mov 0x18(%rdi),%rax`, symbolized under
`gdb`. The measurement that settles the attribution is a fifteen-line C file
containing **no QCore code**, which dies identically; its `pthread_create` twin
runs clean; and `nm` finds three `pthread_create` interceptor symbols and no
`thrd_create` counterpart beside a `thrd_create@GLIBC_2.34` import. Wave 36's
fourth row — a minimal `thrd_create` program running clean — was a **false
negative**: its thread body held no memory access and no call, so clang
instrumented it not at all and there was no `__tsan_func_entry` to fault. The
charter's own first hypothesis, a fixed-address mapping colliding with TSan's
reserved ranges, was **refuted** rather than assumed away: `0x18` is on the
zero page, and no `mmap` or `MAP_FIXED_NOREPLACE` appears in anything this
route emits. **Nothing in QCore changed and no emitted byte moved** — emitting
C11 `thrd_create` is deliberate, the route must also emit for MSVC — so the
workaround is a harness one: `tests/support/thread_sanitizer.rs` replaces
`thrd_create`/`thrd_join` at their **call sites** through `-include`, with the
pthread-backed bodies the shipped `c11_threads_compat.h` already compiles on
Apple hosts, one binding covering both emitters because the task runtime
reaches those names through its own documented `QCORE_TASK_CREATE` hook and the
mutex backend reaches them directly. The rows: all five task-route subjects
(the lease exercise and all four published `examples/threads` programs) die
unbound at the one fault and run **clean bound**, with zero races; CT46's
emitted crossing joins them in `mutex_c_emit.rs`. The clean row is **not
vacuous** — forcing the two workers onto one byte makes TSan name the race
inside `qcore_buffer_byte_write`, with both worker stacks and the heap block
`qcore_buffer_new` allocated — so wave 36's C11 3.14 claim that two threads
writing distinct bytes of one `int32_t` are not in a data race is now backed by
a race detector rather than by argument. All nine new arms were shown non-vacuous by
mutation, restored with mtime advanced — including a second, route-level one: 
replacing `shared_readers`' emitted `QCORE_TASK_JOIN` with a sleep, keeping
every other statement, makes TSan report two races, so the clean rows depend on
the join edge rather than on the detector seeing nothing. That mutation also
shows the instrument's character, since the sleep means the accesses never
physically overlap and TSan reports them anyway. What the rows cannot say
is stated rather than elided: they run a pthread-bound build
(`TASK-ROUTE-TSAN-PTHREAD-BINDING`), a clean detector run is evidence about the
schedules that executed and the failure arms are never taken
(`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`), CT46's checked trace still serialises
its child so `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION` is restated rather than
improved on, and every row is `cfg`-gated to Linux and contributes **no rows**
elsewhere rather than passing vacuously
(`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN`). One process finding is published rather
than smoothed over: `docs/reports/remaining-t04/README.md` §5.1 had stated the
**correct** diagnosis, and the remedy, waves earlier; it reached no tracker,
and wave 36 then contradicted it — `TSAN-DIAGNOSIS-WAS-LOST-THEN-CONTRADICTED`.
`named_followups` **171 → 174** (+4 −1; `TASK-ROUTE-TSAN-SEGV` retired against
its own discharge condition, met word for word). No Lean: no semantic surface
moved — no checker rule, no model, no emitted byte.

