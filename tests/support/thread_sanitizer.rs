//! Card `TASK-ROUTE-TSAN`: run a program the checked task route emitted under
//! ThreadSanitizer. Card `TSAN-ROWS-2` (wave 39) adds the contention-forcing
//! schedules — see [`SCHEDULE_FORCING`] and [`Schedule`] — through the same
//! `-include` mechanism, changing no emitted byte either.
//!
//! **Why this module exists at all.** The emitted task runtime creates threads
//! with C11 `thrd_create`, which is the right primitive for a route that must
//! also emit for MSVC. ThreadSanitizer's runtime ships an interceptor for
//! `pthread_create` and **none for `thrd_create`** — verified on the binary,
//! `nm` finding `__interceptor_pthread_create` and no `thrd_create`
//! counterpart. On glibc 2.34 and later `thrd_create` reaches `start_thread`
//! without passing through that interceptor, so the new thread is never
//! registered with TSan, `cur_thread()` is null in it, and the first
//! instrumented event in the worker faults reading offset `0x18` of that null
//! pointer — `__tsan_func_entry+19`, `mov 0x18(%rdi),%rax`. That is the whole
//! of `TASK-ROUTE-TSAN-SEGV`, and it is a property of the sanitizer runtime
//! rather than of anything QCore emits: a fifteen-line C file containing no
//! QCore code at all, which calls `thrd_create` on a body holding one
//! instrumented access, dies at the identical address and pc.
//!
//! **What this module does about it.** [`PTHREAD_BINDING`] replaces the two
//! C11 thread primitives the emitted programs call — `thrd_create` and
//! `thrd_join` — with pthread-backed equivalents, which TSan *does* intercept.
//! **No emitted byte changes**: the C the compiler produced is compiled
//! verbatim and the binding arrives through `-include`, the same mechanism the
//! existing allocation-tracking harnesses use for `QCORE_BUFFER_MALLOC`. One
//! binding covers both emitters, because they reach the same two names from
//! different directions — the task runtime through its own documented
//! `#ifndef QCORE_TASK_CREATE` hook, whose default *is* `thrd_create`, and the
//! mutex-invariant backend through a literal call.
//!
//! The replacement bodies are not invented here: they are the pthread-backed
//! `thrd_create`/`thrd_join` the shipped `c11_threads_compat.h` already
//! compiles on Apple hosts, so every task program in this repository is
//! already built this way on one supported platform.
//!
//! **What that costs, stated rather than elided.** The program TSan observes
//! differs from the published program in which symbol spawns the thread. It is
//! not a different concurrency structure: glibc implements `thrd_create` by
//! calling `pthread_create`'s implementation, so both reach the same
//! `start_thread` — visible in the faulting backtrace itself, where the
//! unbound `thrd_create` frame sits directly on `start_thread`. Every
//! happens-before edge TSan reasons about (thread creation, thread join) is
//! therefore the same edge in both builds. What TSan cannot speak for is the
//! `thrd_create` call itself and any Windows or Apple thread arm. Registered
//! as `TASK-ROUTE-TSAN-PTHREAD-BINDING`.
//!
//! **The forced schedules (card `TSAN-ROWS-2`).** Wave 37's rows were each one
//! run under whatever schedule the OS produced, and the schedule was not
//! recorded (`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`). [`SCHEDULE_FORCING`] is a
//! second `-include` header that redefines the task runtime's own
//! `QCORE_TASK_CREATE`/`QCORE_TASK_JOIN` hooks — the seam CT07's
//! `refuse_second_spawn.h` already rides — so that every worker **parks at
//! creation** and is released only when the creator reaches its next join.
//! Two release policies exist, selected at compile time:
//!
//! * [`Schedule::ForcedTogether`]: every worker created since the previous
//!   release is released **at once**, and every one of those threads stays
//!   live until **every** body of that cohort has returned (an exit barrier).
//!   So all of a cohort's workers exist before any body runs, and no thread of
//!   it leaves before all bodies have run.
//! * [`Schedule::ForcedReverse`]: the same cohort is released **one at a
//!   time, last-created first**, each body running to completion before the
//!   next is released — the reverse of creation order, serialized.
//!
//! What the forcing does **not** do: it does not make two bodies' store
//! instructions physically overlap (nothing outside an emitted body can force
//! that), and it does not change what ThreadSanitizer reasons about, which is
//! happens-before and not timing. The realised schedule is written to stderr
//! as `QCORE_FORCED start N` / `QCORE_FORCED leave N` lines, parsed into
//! [`Sanitized::schedule`], so a row can assert the schedule it ran under
//! rather than assume it. Worker trace events are buffered privately and
//! replayed by the parent at collect (constraint 15a), which is why parking a
//! worker moves no stdout byte and the strict stdout comparison stays strict.
//!
//! Linux only, and gated as such. `run` is `#[cfg(target_os = "linux")]`
//! because the diagnosis, the binding and every measurement above are
//! x86-64-Linux/glibc observations; a caller on another platform gets no row
//! rather than a vacuous pass, and must say so in its own test.

#![allow(dead_code)]

/// Replace `thrd_create`/`thrd_join` with pthread-backed equivalents.
/// Prepended to the emitted C through `-include`, so the emitted translation
/// unit itself is unmodified; `<threads.h>` is pulled in *before* the macros
/// are defined, so the real declarations are never rewritten and only call
/// sites are affected.
///
/// `QCORE_TSAN_DROP_JOIN` (card `TSAN-ROWS-2`) is the **join-edge-removal
/// mutation's** replacement for `QCORE_TASK_JOIN`: it waits 50 ms and reports
/// success without joining, so the happens-before edge is gone and nothing
/// else is. It is defined here so the mutated text is one string under every
/// schedule; the forcing header redefines the name to a variant that also
/// opens its gate, because a join that is deleted outright would otherwise
/// delete the release with it and hide the race rather than expose it.
pub const PTHREAD_BINDING: &str = r#"
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>
#include <threads.h>
#include <time.h>

/* The two casts below are the whole of the binding's representation
 * assumption, and it is checked rather than assumed. */
_Static_assert(sizeof(thrd_t) == sizeof(pthread_t),
    "the pthread binding requires thrd_t and pthread_t to be one representation");

struct qcore_tsan_start { int (*function)(void *); void *argument; };

static void *qcore_tsan_trampoline(void *opaque) {
    struct qcore_tsan_start start = *(struct qcore_tsan_start *)opaque;
    free(opaque);
    return (void *)(intptr_t)start.function(start.argument);
}

/* Signature-compatible with thrd_create, so no emitted call site changes. */
__attribute__((unused))
static int qcore_tsan_thread_create(thrd_t *thread, int (*function)(void *), void *argument) {
    struct qcore_tsan_start *start = malloc(sizeof(*start));
    pthread_t created;
    if (start == NULL) return thrd_error;
    start->function = function;
    start->argument = argument;
    if (pthread_create(&created, NULL, qcore_tsan_trampoline, start) != 0) {
        free(start);
        return thrd_error;
    }
    *thread = (thrd_t)created;
    return thrd_success;
}

__attribute__((unused))
static int qcore_tsan_thread_join(thrd_t thread, int *result) {
    void *value = NULL;
    if (pthread_join((pthread_t)thread, &value) != 0) return thrd_error;
    if (result != NULL) *result = (int)(intptr_t)value;
    return thrd_success;
}

/* The join-edge-removal mutation's stand-in for QCORE_TASK_JOIN (card
 * TSAN-ROWS-2): 50 ms, then "joined", with no happens-before edge. */
__attribute__((unused))
static int qcore_tsan_drop_join(thrd_t thread, int *status) {
    struct timespec pause = {0, 50000000};
    (void)thread;
    thrd_sleep(&pause, NULL);
    if (status != NULL) *status = 0;
    return thrd_success;
}
#define QCORE_TSAN_DROP_JOIN qcore_tsan_drop_join

/* Call sites only: the task runtime reaches these through its own
 * QCORE_TASK_CREATE/QCORE_TASK_JOIN hooks, whose defaults are these names, and
 * the mutex-invariant backend reaches them directly. */
#define thrd_create qcore_tsan_thread_create
#define thrd_join qcore_tsan_thread_join
"#;

/// Card `TSAN-ROWS-2`: the contention-forcing schedule, as a second `-include`
/// header placed **after** [`PTHREAD_BINDING`] so that its own `thrd_create`
/// and `thrd_join` calls are the bound ones.
///
/// It attaches only to the task runtime's `#ifndef QCORE_TASK_CREATE` /
/// `#ifndef QCORE_TASK_JOIN` seam. An emission with no such seam — the
/// mutex-invariant backend calls `thrd_create` bare — is compiled with this
/// header present and **unchanged in behaviour**: no worker is parked, no
/// schedule line is written, which a row can assert.
///
/// `QCORE_FORCED_REVERSE` selects the release policy (`0`: together with an
/// exit barrier; `1`: last-created first, serialized). Bookkeeping is a static
/// table of at most `QCORE_FORCED_CAPACITY` workers per process, and every
/// state change happens under one mutex with one condition variable.
pub const SCHEDULE_FORCING: &str = r#"
/* Card TSAN-ROWS-2: park every worker at creation; release at the creator's
 * next join, together (with an exit barrier) or last-created first.
 *
 * INVISIBLE TO THREADSANITIZER BY CONSTRUCTION. A first version of this header
 * parked and released workers on a pthread mutex and condition variable, and
 * under the reverse policy the overlap mutation went UNREPORTED: worker 2's
 * byte write preceded its unlock, worker 1 woke by acquiring the same mutex,
 * and TSan saw a release/acquire edge between the two writes. A schedule
 * realised with synchronization TSan can see hides exactly the races the
 * rows exist to detect. So every function here is compiled without TSan
 * instrumentation (no_sanitize), all bookkeeping sits behind an uninstrumented
 * spinlock, and a parked worker spins on its own flag with sched_yield.
 * The only happens-before edges TSan observes are the program's own:
 * thread creation and thread join. */
#include <sched.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <time.h>

#ifndef QCORE_FORCED_REVERSE
#define QCORE_FORCED_REVERSE 0
#endif
#define QCORE_FORCED_CAPACITY 64
#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize("thread")))

struct qcore_forced_worker {
    int (*function)(void *);
    void *argument;
    size_t ordinal;     /* 1-based creation order */
    size_t generation;  /* the creator's join count when this worker was created */
    size_t cohort;      /* the join (1-based) that released it; 0 while parked */
    atomic_int released;
    atomic_int finished;
};
static atomic_flag qcore_forced_spin = ATOMIC_FLAG_INIT;
static struct qcore_forced_worker qcore_forced_workers[QCORE_FORCED_CAPACITY];
static size_t qcore_forced_created = 0;
static size_t qcore_forced_joins = 0;
static size_t qcore_forced_cohort_size[QCORE_FORCED_CAPACITY + 1];
static size_t qcore_forced_cohort_finished[QCORE_FORCED_CAPACITY + 1];

QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_lock(void) {
    while (atomic_flag_test_and_set_explicit(&qcore_forced_spin, memory_order_acquire)) sched_yield();
}
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_unlock(void) {
    atomic_flag_clear_explicit(&qcore_forced_spin, memory_order_release);
}

/* Under the lock: is a released worker still running its body? */
QCORE_FORCED_UNINSTRUMENTED static int qcore_forced_body_running(void) {
    for (size_t i = 0; i < qcore_forced_created; ++i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i];
        if (atomic_load_explicit(&worker->released, memory_order_relaxed) &&
            !atomic_load_explicit(&worker->finished, memory_order_relaxed)) return 1;
    }
    return 0;
}

/* Under the lock, reverse policy: release the most recently created worker
 * that is eligible (created before the last join) and not yet released, and
 * only when no released body is still running. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_release_next_reverse(void) {
    if (qcore_forced_body_running()) return;
    for (size_t i = qcore_forced_created; i > 0; --i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i - 1];
        if (worker->generation < qcore_forced_joins &&
            !atomic_load_explicit(&worker->released, memory_order_relaxed)) {
            worker->cohort = qcore_forced_joins;
            qcore_forced_cohort_size[worker->cohort] += 1;
            atomic_store_explicit(&worker->released, 1, memory_order_relaxed);
            return;
        }
    }
}

/* Under the lock: the creator reached a join, so every worker parked before
 * it becomes eligible. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_open_gate(void) {
    if (qcore_forced_joins >= QCORE_FORCED_CAPACITY) abort();
    ++qcore_forced_joins;
    if (QCORE_FORCED_REVERSE) {
        qcore_forced_release_next_reverse();
        return;
    }
    for (size_t i = 0; i < qcore_forced_created; ++i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i];
        if (worker->generation < qcore_forced_joins &&
            !atomic_load_explicit(&worker->released, memory_order_relaxed)) {
            worker->cohort = qcore_forced_joins;
            qcore_forced_cohort_size[worker->cohort] += 1;
            atomic_store_explicit(&worker->released, 1, memory_order_relaxed);
        }
    }
}

QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_park(struct qcore_forced_worker *worker) {
    while (!atomic_load_explicit(&worker->released, memory_order_relaxed)) sched_yield();
}

/* Together: the exit barrier keeps this thread live until every body of its
 * cohort has returned, so "all created before any runs, all live until all
 * have run" holds by construction rather than by luck. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_exit_barrier(struct qcore_forced_worker *worker) {
    for (;;) {
        int complete;
        qcore_forced_lock();
        complete = qcore_forced_cohort_finished[worker->cohort] >= qcore_forced_cohort_size[worker->cohort];
        qcore_forced_unlock();
        if (complete) return;
        sched_yield();
    }
}

QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_finish(struct qcore_forced_worker *worker) {
    qcore_forced_lock();
    atomic_store_explicit(&worker->finished, 1, memory_order_relaxed);
    qcore_forced_cohort_finished[worker->cohort] += 1;
    if (QCORE_FORCED_REVERSE) {
        /* Serialized: this body is done, and the trace says so before the
         * next worker can be released and write its own start line. */
        fprintf(stderr, "QCORE_FORCED leave %zu\n", worker->ordinal);
        qcore_forced_release_next_reverse();
        qcore_forced_unlock();
        return;
    }
    qcore_forced_unlock();
    qcore_forced_exit_barrier(worker);
    fprintf(stderr, "QCORE_FORCED leave %zu\n", worker->ordinal);
}

QCORE_FORCED_UNINSTRUMENTED static int qcore_forced_trampoline(void *opaque) {
    struct qcore_forced_worker *worker = opaque;
    int result;
    qcore_forced_park(worker);
    fprintf(stderr, "QCORE_FORCED start %zu\n", worker->ordinal);
    result = worker->function(worker->argument);
    qcore_forced_finish(worker);
    return result;
}

/* Signature-compatible with thrd_create: the runtime's QCORE_TASK_CREATE. */
__attribute__((unused)) QCORE_FORCED_UNINSTRUMENTED
static int qcore_forced_create(thrd_t *thread, int (*function)(void *), void *argument) {
    struct qcore_forced_worker *worker;
    int outcome;
    qcore_forced_lock();
    if (qcore_forced_created >= QCORE_FORCED_CAPACITY) abort();
    worker = &qcore_forced_workers[qcore_forced_created];
    worker->function = function;
    worker->argument = argument;
    worker->ordinal = qcore_forced_created + 1;
    worker->generation = qcore_forced_joins;
    worker->cohort = 0;
    atomic_store_explicit(&worker->released, 0, memory_order_relaxed);
    atomic_store_explicit(&worker->finished, 0, memory_order_relaxed);
    ++qcore_forced_created;
    qcore_forced_unlock();
    outcome = thrd_create(thread, qcore_forced_trampoline, worker);
    if (outcome != thrd_success) {
        /* A worker that was never created must not be waited for. */
        qcore_forced_lock();
        --qcore_forced_created;
        qcore_forced_unlock();
    }
    return outcome;
}

/* The runtime's QCORE_TASK_JOIN: open the gate, then really join. */
__attribute__((unused)) QCORE_FORCED_UNINSTRUMENTED
static int qcore_forced_join(thrd_t thread, int *result) {
    qcore_forced_lock();
    qcore_forced_open_gate();
    qcore_forced_unlock();
    return thrd_join(thread, result);
}

/* The join-edge-removal mutation under forcing: open the gate as the real
 * join would, then wait 50 ms and report success without joining. */
__attribute__((unused)) QCORE_FORCED_UNINSTRUMENTED
static int qcore_forced_drop_join(thrd_t thread, int *status) {
    struct timespec pause = {0, 50000000};
    (void)thread;
    qcore_forced_lock();
    qcore_forced_open_gate();
    qcore_forced_unlock();
    thrd_sleep(&pause, NULL);
    if (status != NULL) *status = 0;
    return thrd_success;
}
#undef QCORE_TSAN_DROP_JOIN
#define QCORE_TSAN_DROP_JOIN qcore_forced_drop_join

#define QCORE_TASK_CREATE qcore_forced_create
#define QCORE_TASK_JOIN qcore_forced_join
"#;

/// Which schedule a run is forced onto (card `TSAN-ROWS-2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// Wave 37's rows: whatever the OS produced, unrecorded.
    Unforced,
    /// Every worker parked at creation; the cohort released together at the
    /// creator's next join, every thread of it live until every body has run.
    ForcedTogether,
    /// The same cohort released one at a time, last-created first, each body
    /// to completion before the next starts.
    ForcedReverse,
}

impl Schedule {
    pub const ALL: [Schedule; 3] = [
        Schedule::Unforced,
        Schedule::ForcedTogether,
        Schedule::ForcedReverse,
    ];
    pub const FORCED: [Schedule; 2] = [Schedule::ForcedTogether, Schedule::ForcedReverse];

    pub fn tag(self) -> &'static str {
        match self {
            Schedule::Unforced => "unforced",
            Schedule::ForcedTogether => "forced-together",
            Schedule::ForcedReverse => "forced-reverse",
        }
    }

    pub fn is_forced(self) -> bool {
        !matches!(self, Schedule::Unforced)
    }
}

/// One `QCORE_FORCED …` line of the forcing header's stderr trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleEvent {
    /// Worker `N` was released and is about to run its body.
    Start(usize),
    /// Worker `N`'s thread is leaving — after its body and, under the
    /// together policy, after every body of its cohort.
    Leave(usize),
}

/// One ThreadSanitizer run of one emitted program.
#[derive(Debug)]
pub struct Sanitized {
    pub stdout: String,
    pub stderr: String,
    /// `true` when the process exited 0 with no diagnostic on stderr.
    pub clean: bool,
    /// Number of `data race` reports ThreadSanitizer printed.
    pub races: usize,
    /// `Some(signal)` when the process died on a signal.
    pub signal: Option<i32>,
    /// The realised schedule, parsed from the forcing header's stderr lines.
    /// Empty under [`Schedule::Unforced`], and empty under a forced schedule
    /// whenever no worker was ever released.
    pub schedule: Vec<ScheduleEvent>,
    /// The exact bytes the C compiler was handed as the translation unit —
    /// read back from disk after writing — so a row can assert that a forced
    /// run compiled the very same emitted C as an unforced one.
    pub compiled: String,
}

impl Sanitized {
    /// The `TASK-ROUTE-TSAN-SEGV` shape: the null-`ThreadState` fault.
    pub fn is_unregistered_thread_fault(&self) -> bool {
        self.stderr
            .contains("SEGV on unknown address 0x000000000018")
    }

    /// Stderr with the schedule trace removed, for comparisons against a
    /// reference stderr projection.
    pub fn stderr_without_schedule(&self) -> String {
        self.stderr
            .lines()
            .filter(|line| !line.starts_with("QCORE_FORCED "))
            .map(|line| format!("{line}\n"))
            .collect()
    }
}

fn parse_schedule(stderr: &str) -> Vec<ScheduleEvent> {
    stderr
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("QCORE_FORCED ")?;
            let (kind, ordinal) = rest.split_once(' ')?;
            let ordinal = ordinal.trim().parse().ok()?;
            match kind {
                "start" => Some(ScheduleEvent::Start(ordinal)),
                "leave" => Some(ScheduleEvent::Leave(ordinal)),
                _ => None,
            }
        })
        .collect()
}

/// Compile `c_source` under ThreadSanitizer and run it. `bind_threads` selects
/// whether [`PTHREAD_BINDING`] is prepended; `false` reproduces the unbound
/// failure, which is what keeps the diagnosis pinned rather than remembered.
///
/// Returns `Err` only when the C compiler rejected the program — a program
/// that builds and then dies is a [`Sanitized`] with `clean == false`, because
/// dying is a measurement here and not a harness failure.
#[cfg(target_os = "linux")]
pub fn run(tag: &str, c_source: &str, bind_threads: bool) -> Result<Sanitized, String> {
    run_scheduled(tag, c_source, bind_threads, Schedule::Unforced)
}

/// [`run`], forced onto `schedule` (card `TSAN-ROWS-2`). A forced schedule
/// adds [`SCHEDULE_FORCING`] as a second `-include` after the binding, with
/// `QCORE_FORCED_REVERSE` set for [`Schedule::ForcedReverse`]. The emitted
/// translation unit is written and compiled unchanged under every schedule,
/// and the bytes handed to the compiler come back in [`Sanitized::compiled`].
#[cfg(target_os = "linux")]
pub fn run_scheduled(
    tag: &str,
    c_source: &str,
    bind_threads: bool,
    schedule: Schedule,
) -> Result<Sanitized, String> {
    run_with_options(tag, c_source, bind_threads, schedule, "halt_on_error=1")
}

/// [`run_scheduled`] with the `TSAN_OPTIONS` string chosen by the caller —
/// `halt_on_error=0` lets ThreadSanitizer report every race rather than the
/// first, which is how a race *count* is measured (card `TSAN-ROWS-2`).
#[cfg(target_os = "linux")]
pub fn run_with_options(
    tag: &str,
    c_source: &str,
    bind_threads: bool,
    schedule: Schedule,
    tsan_options: &str,
) -> Result<Sanitized, String> {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    // Run-scoped by tag, schedule and pid, so two runs cannot truncate each
    // other.
    let directory = std::env::temp_dir().join(format!(
        "qcore-task-route-tsan-{tag}-{}-{}",
        schedule.tag(),
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let binding = directory.join("bind.h");
    let forcing = directory.join("force.h");
    let executable = directory.join("module");
    std::fs::write(&c, c_source).unwrap();
    std::fs::write(&binding, PTHREAD_BINDING).unwrap();
    std::fs::write(&forcing, SCHEDULE_FORCING).unwrap();
    let compiled = std::fs::read_to_string(&c).unwrap();

    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args([
        "-std=c11",
        "-pthread",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-g",
        "-O0",
        "-fno-omit-frame-pointer",
        "-fsanitize=thread",
    ]);
    if bind_threads {
        compiler.arg("-include").arg(&binding);
    }
    match schedule {
        Schedule::Unforced => {}
        Schedule::ForcedTogether => {
            compiler.arg("-DQCORE_FORCED_REVERSE=0");
            compiler.arg("-include").arg(&forcing);
        }
        Schedule::ForcedReverse => {
            compiler.arg("-DQCORE_FORCED_REVERSE=1");
            compiler.arg("-include").arg(&forcing);
        }
    }
    let built = compiler
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .map_err(|error| format!("the C compiler could not be run: {error}"))?;
    if !built.status.success() {
        return Err(format!(
            "the C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&built.stderr)
        ));
    }

    let output = Command::new(&executable)
        .env("TSAN_OPTIONS", tsan_options)
        .output()
        .map_err(|error| format!("the built program could not be run: {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let races = stderr.matches("ThreadSanitizer: data race").count();
    let sanitized = Sanitized {
        clean: output.status.success() && !stderr.contains("ThreadSanitizer:"),
        races,
        signal: output.status.signal(),
        schedule: parse_schedule(&stderr),
        compiled,
        stdout,
        stderr,
    };
    std::fs::remove_dir_all(&directory).ok();
    Ok(sanitized)
}
