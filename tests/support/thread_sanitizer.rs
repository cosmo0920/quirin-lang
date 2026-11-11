// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
//! **The release trigger the join does not own (card `TSAN-FAILURE-ARMS`,
//! wave 41).** Wave 39's gate opened at the creator's `QCORE_TASK_JOIN` and
//! nowhere else, which had two consequences it registered as boundaries: a
//! mutation deleting the join deleted the release with it and hid every race
//! (`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`, measured 0/20 forced against 20/20
//! unforced), and a program whose **failure arm never reaches a join** could
//! not be forced at all (`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`).
//! `QCORE_FORCED_COHORT` is a second, independent trigger: the row supplies the
//! number of workers the program parks in one cohort, and the gate opens inside
//! `QCORE_TASK_CREATE` as soon as that many workers stand parked — before the
//! creator reaches any join, and whether or not it ever reaches one. The join
//! trigger is **retained** rather than replaced, so a cohort count that is
//! never reached (a refused spawn) still releases at the first join; a row
//! supplies the cohort its arm actually creates. `0` selects wave 39's
//! join-only behaviour and is kept reachable as a control.
//!
//! **The failure arms (card `TSAN-FAILURE-ARMS`).** [`FAILURE_INJECTION`] is a
//! third `-include` header, placed after the forcing header so that it wraps
//! whatever `QCORE_TASK_CREATE` then means. [`Failure::RefuseSecondCreate`]
//! refuses the second creation only, so the first worker really is created,
//! really runs and really is joined on the recovery arm;
//! [`Failure::NullBufferMalloc`] makes `QCORE_BUFFER_MALLOC` return `NULL`, so
//! the entry's `qcore_buffer_new(...)?` takes its error edge. Neither changes
//! an emitted byte: both arrive at the same `-include` seam as the binding.
//!
//! **Load, made deterministic (card `W45-TSAN-ROWS-LOAD-SENSITIVE`, wave
//! 45).** Two "the race must be reported" rows failed integration gates under
//! parallel load and passed alone. Neither was the detector: (1) the forcing
//! header's exit barrier released its spinlock on every poll, and clang
//! instruments atomics even in a `no_sanitize("thread")` function, so a worker
//! waiting for a descheduled partner wrote enough trace to evict its own body's
//! accesses from ThreadSanitizer's history, and TSan v3 drops a race whose
//! earlier stack it cannot restore; (2) the join-edge-removal stand-in slept
//! 50 ms and assumed the worker had finished, so a late worker met a freed
//! start record and TSan reported a `heap-use-after-free`, or nothing. Every
//! wait in these headers now polls a relaxed atomic inline and the stand-in
//! waits for completion over a relaxed flag; [`Row::hold`] reproduces the
//! load on demand. See `docs/reports/w45-tsan-rows-load-sensitive/`.
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
/// mutation's** replacement for `QCORE_TASK_JOIN`: it reports success without
/// joining, so the happens-before edge is gone and nothing else is. It is
/// defined here so the mutated text is one string under every schedule; the
/// forcing header redefines the name to a variant that also opens its gate,
/// because a join that is deleted outright would otherwise delete the release
/// with it and hide the race rather than expose it.
///
/// **It waits for the worker, not for a clock** (card
/// `W45-TSAN-ROWS-LOAD-SENSITIVE`). Wave 39's stand-in slept 50 ms and
/// assumed the worker had finished by then. Under parallel load that
/// assumption failed: a worker still parked or runnable after 50 ms touched
/// its start record *after* the parent had freed it, so ThreadSanitizer
/// reported a `heap-use-after-free` instead of the data race the row asserts,
/// or nothing at all if the process ended first. The stand-in now spins until
/// the thread's start function has returned, observed through a completion
/// flag written and read with **relaxed** atomics — which carry no
/// happens-before edge in C11 or in ThreadSanitizer, so the removed edge stays
/// removed, and the worker's accesses precede the parent's by construction
/// rather than by a 50 ms bet. The wait polls inline, without a call and
/// without a release, so ThreadSanitizer records nothing new per poll.
///
/// A worker the forcing header holds behind a gate that has not opened for it
/// never finishes, and only the waiting creator could open that gate, so the
/// stand-in asks `QCORE_TSAN_NEVER_RUNS(thread)` once before waiting; its
/// default here is `0`, and the forcing header redefines it. That is what keeps
/// `a_join_deleted_is_caught_by_the_cohort_trigger`'s gate-closed control a
/// program that ends rather than one that hangs.
///
/// `QCORE_TSAN_HOLD_ORDINAL` / `QCORE_TSAN_HOLD_MS` are the same card's
/// **deterministic load**: the `N`-th thread created sleeps that long before
/// its start function runs (`thrd_sleep`, which ThreadSanitizer does not
/// intercept, so the hold adds no event of its own). Default `0`: no hold.
pub const PTHREAD_BINDING: &str = r#"
#include <pthread.h>
#include <sched.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdlib.h>
#include <threads.h>
#include <time.h>

/* The two casts below are the whole of the binding's representation
 * assumption, and it is checked rather than assumed. */
_Static_assert(sizeof(thrd_t) == sizeof(pthread_t),
    "the pthread binding requires thrd_t and pthread_t to be one representation");
_Static_assert(sizeof(thrd_t) == sizeof(unsigned long),
    "the completion table stores a thrd_t as an unsigned long");

#ifndef QCORE_TSAN_HOLD_ORDINAL
#define QCORE_TSAN_HOLD_ORDINAL 0
#endif
#ifndef QCORE_TSAN_HOLD_MS
#define QCORE_TSAN_HOLD_MS 0
#endif
#define QCORE_TSAN_THREAD_CAPACITY 64

/* Card W45-TSAN-ROWS-LOAD-SENSITIVE: one completion flag per created thread.
 * Every access is a relaxed atomic, so the table synchronizes nothing: it lets
 * the join-edge-removal stand-in wait for a worker without ordering anything
 * after it. */
struct qcore_tsan_thread { atomic_ulong thread; atomic_int done; };
static struct qcore_tsan_thread qcore_tsan_threads[QCORE_TSAN_THREAD_CAPACITY];
static atomic_size_t qcore_tsan_thread_count;

struct qcore_tsan_start {
    int (*function)(void *);
    void *argument;
    struct qcore_tsan_thread *slot;
    size_t ordinal;     /* 1-based creation order */
};

static void *qcore_tsan_trampoline(void *opaque) {
    struct qcore_tsan_start start = *(struct qcore_tsan_start *)opaque;
    int result;
    free(opaque);
    if (QCORE_TSAN_HOLD_ORDINAL != 0 && start.ordinal == (size_t)QCORE_TSAN_HOLD_ORDINAL) {
        struct timespec hold = {QCORE_TSAN_HOLD_MS / 1000, (QCORE_TSAN_HOLD_MS % 1000) * 1000000L};
        thrd_sleep(&hold, NULL);
    }
    result = start.function(start.argument);
    atomic_store_explicit(&start.slot->done, 1, memory_order_relaxed);
    return (void *)(intptr_t)result;
}

/* Signature-compatible with thrd_create, so no emitted call site changes. */
__attribute__((unused))
static int qcore_tsan_thread_create(thrd_t *thread, int (*function)(void *), void *argument) {
    struct qcore_tsan_start *start = malloc(sizeof(*start));
    struct qcore_tsan_thread *slot;
    size_t index;
    pthread_t created;
    if (start == NULL) return thrd_error;
    index = atomic_fetch_add_explicit(&qcore_tsan_thread_count, 1, memory_order_relaxed);
    if (index >= QCORE_TSAN_THREAD_CAPACITY) abort();
    slot = &qcore_tsan_threads[index];
    start->function = function;
    start->argument = argument;
    start->slot = slot;
    start->ordinal = index + 1;
    if (pthread_create(&created, NULL, qcore_tsan_trampoline, start) != 0) {
        free(start);
        return thrd_error;
    }
    atomic_store_explicit(&slot->thread, (unsigned long)created, memory_order_relaxed);
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
 * TSAN-ROWS-2; the wait is card W45-TSAN-ROWS-LOAD-SENSITIVE's): wait until
 * the thread's start function has returned, then "joined", with no
 * happens-before edge. The flag is read relaxed and inline — no call, no
 * release — so however long the wait, ThreadSanitizer records nothing new. */
__attribute__((unused))
static int qcore_tsan_unordered_join(thrd_t thread, int *status, int never_runs) {
    size_t count = atomic_load_explicit(&qcore_tsan_thread_count, memory_order_relaxed);
    struct qcore_tsan_thread *slot = NULL;
    for (size_t i = 0; i < count && i < QCORE_TSAN_THREAD_CAPACITY; ++i)
        if (atomic_load_explicit(&qcore_tsan_threads[i].thread, memory_order_relaxed) == (unsigned long)thread)
            slot = &qcore_tsan_threads[i];
    if (slot == NULL) abort();
    if (!never_runs)
        while (!atomic_load_explicit(&slot->done, memory_order_relaxed)) sched_yield();
    if (status != NULL) *status = 0;
    return thrd_success;
}
/* The forcing header redefines this: a worker parked behind a gate that has
 * not opened for it never finishes while its creator waits. */
#define QCORE_TSAN_NEVER_RUNS(thread) 0
/* A macro, not a function, so that QCORE_TSAN_NEVER_RUNS is expanded at the
 * call site, after the forcing header (when present) has redefined it. The
 * join-deleted mutation calls this name directly and so still bypasses the
 * forcing header's gate-opening QCORE_TSAN_DROP_JOIN. */
#define qcore_tsan_drop_join(thread, status) \
    qcore_tsan_unordered_join((thread), (status), QCORE_TSAN_NEVER_RUNS(thread))
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
/// state change happens under one uninstrumented spinlock.
///
/// `QCORE_FORCED_COHORT` (card `TSAN-FAILURE-ARMS`) is the **second release
/// trigger**: the row's own count of the workers this program parks in one
/// cohort. The gate opens inside the create hook as soon as that many workers
/// stand parked, so the creator's join no longer owns the release. `0` is wave
/// 39's join-only behaviour.
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
 * memory-access instrumentation (no_sanitize), all bookkeeping sits behind a
 * spinlock, and a parked worker spins on its own flag with sched_yield.
 *
 * What no_sanitize does NOT remove (card W45-TSAN-ROWS-LOAD-SENSITIVE): clang
 * still instruments ATOMIC operations, and function entry/exit around them, in
 * a no_sanitize("thread") function. So TSan does see the spinlock — but no
 * worker acquires anything between its release and its body (parking reads
 * its flag relaxed), so no edge TSan sees orders one body after another.
 *
 * And every wait here must be QUIET: it polls a relaxed atomic inline, with
 * no call and no release per iteration, taking the lock only when a counter
 * says something changed. An exit barrier that took and released the lock on
 * every poll gave TSan a new epoch and ~6 trace events per iteration; ~20,000
 * polls (a partner delayed ~5 ms under load) recycled the history holding the
 * waiting worker's own body accesses, and TSan v3 then silently drops a race
 * whose earlier stack it cannot restore. A quiet wait writes a trace bounded
 * by the cohort, however long it waits. */
#include <sched.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <time.h>

#ifndef QCORE_FORCED_REVERSE
#define QCORE_FORCED_REVERSE 0
#endif
/* Card TSAN-FAILURE-ARMS: the row's own cohort count. 0 keeps wave 39's
 * join-only trigger; N > 0 adds a trigger the creator's join does not own. */
#ifndef QCORE_FORCED_COHORT
#define QCORE_FORCED_COHORT 0
#endif
#define QCORE_FORCED_CAPACITY 64
#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize("thread")))

struct qcore_forced_worker {
    int (*function)(void *);
    void *argument;
    thrd_t thread;      /* written by the creator after a successful create */
    size_t ordinal;     /* 1-based creation order */
    size_t generation;  /* the gate count when this worker was created */
    size_t cohort;      /* the gate opening (1-based) that released it; 0 while parked */
    atomic_int released;
    atomic_int finished;
};
static atomic_flag qcore_forced_spin = ATOMIC_FLAG_INIT;
static struct qcore_forced_worker qcore_forced_workers[QCORE_FORCED_CAPACITY];
static size_t qcore_forced_created = 0;
static size_t qcore_forced_gate = 0;
static size_t qcore_forced_cohort_size[QCORE_FORCED_CAPACITY + 1];
static size_t qcore_forced_cohort_finished[QCORE_FORCED_CAPACITY + 1];
/* Card W45-TSAN-ROWS-LOAD-SENSITIVE: bumped, under the lock, on every change
 * to the two cohort tables above, so the exit barrier can poll it relaxed and
 * take the lock only when there is something new to read. */
static atomic_size_t qcore_forced_changes;

QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_lock(void) {
    while (atomic_flag_test_and_set_explicit(&qcore_forced_spin, memory_order_acquire)) sched_yield();
}
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_unlock(void) {
    atomic_flag_clear_explicit(&qcore_forced_spin, memory_order_release);
}

/* Under the lock: a cohort table changed; wake the exit barrier's poll. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_changed(void) {
    atomic_fetch_add_explicit(&qcore_forced_changes, 1, memory_order_relaxed);
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

/* Under the lock: how many created workers stand parked? Card
 * TSAN-FAILURE-ARMS reads this in the create hook to decide whether the row's
 * own cohort has assembled, which is a release trigger no join owns. */
QCORE_FORCED_UNINSTRUMENTED static size_t qcore_forced_parked(void) {
    size_t parked = 0;
    for (size_t i = 0; i < qcore_forced_created; ++i)
        if (!atomic_load_explicit(&qcore_forced_workers[i].released, memory_order_relaxed)) ++parked;
    return parked;
}

/* Under the lock, reverse policy: release the most recently created worker
 * that is eligible (created before the last gate opening) and not yet
 * released, and only when no released body is still running. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_release_next_reverse(void) {
    if (qcore_forced_body_running()) return;
    for (size_t i = qcore_forced_created; i > 0; --i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i - 1];
        if (worker->generation < qcore_forced_gate &&
            !atomic_load_explicit(&worker->released, memory_order_relaxed)) {
            worker->cohort = qcore_forced_gate;
            qcore_forced_cohort_size[worker->cohort] += 1;
            qcore_forced_changed();
            atomic_store_explicit(&worker->released, 1, memory_order_relaxed);
            return;
        }
    }
}

/* Under the lock: a trigger fired — the creator reached a join, or (card
 * TSAN-FAILURE-ARMS) the row's cohort assembled — so every worker parked
 * before it becomes eligible. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_open_gate(void) {
    if (qcore_forced_gate >= QCORE_FORCED_CAPACITY) abort();
    ++qcore_forced_gate;
    if (QCORE_FORCED_REVERSE) {
        qcore_forced_release_next_reverse();
        return;
    }
    for (size_t i = 0; i < qcore_forced_created; ++i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i];
        if (worker->generation < qcore_forced_gate &&
            !atomic_load_explicit(&worker->released, memory_order_relaxed)) {
            worker->cohort = qcore_forced_gate;
            qcore_forced_cohort_size[worker->cohort] += 1;
            qcore_forced_changed();
            atomic_store_explicit(&worker->released, 1, memory_order_relaxed);
        }
    }
}

QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_park(struct qcore_forced_worker *worker) {
    while (!atomic_load_explicit(&worker->released, memory_order_relaxed)) sched_yield();
}

/* Together: the exit barrier keeps this thread live until every body of its
 * cohort has returned, so "all created before any runs, all live until all
 * have run" holds by construction rather than by luck.
 *
 * QUIET (card W45-TSAN-ROWS-LOAD-SENSITIVE): the tables are read under the
 * lock only when qcore_forced_changes has moved since the last read, so the
 * number of lock round trips is bounded by the cohort's own changes, not by
 * how long a descheduled partner takes. Between them the poll is one relaxed
 * load at an unchanged epoch, which ThreadSanitizer deduplicates. */
QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_exit_barrier(struct qcore_forced_worker *worker) {
    size_t seen = 0;
    int checked = 0;
    for (;;) {
        size_t now = atomic_load_explicit(&qcore_forced_changes, memory_order_relaxed);
        if (!checked || now != seen) {
            int complete;
            checked = 1;
            seen = now;
            qcore_forced_lock();
            complete = qcore_forced_cohort_finished[worker->cohort] >= qcore_forced_cohort_size[worker->cohort];
            qcore_forced_unlock();
            if (complete) return;
        }
        sched_yield();
    }
}

QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_finish(struct qcore_forced_worker *worker) {
    qcore_forced_lock();
    atomic_store_explicit(&worker->finished, 1, memory_order_relaxed);
    qcore_forced_cohort_finished[worker->cohort] += 1;
    qcore_forced_changed();
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
    worker->generation = qcore_forced_gate;
    worker->cohort = 0;
    atomic_store_explicit(&worker->released, 0, memory_order_relaxed);
    atomic_store_explicit(&worker->finished, 0, memory_order_relaxed);
    ++qcore_forced_created;
    /* Card TSAN-FAILURE-ARMS: the trigger the creator's join does not own.
     * Once the row's cohort stands parked the gate opens here, inside the
     * create hook, before the creator has reached any join at all. */
    if (QCORE_FORCED_COHORT > 0 && qcore_forced_parked() >= (size_t)QCORE_FORCED_COHORT)
        qcore_forced_open_gate();
    qcore_forced_unlock();
    outcome = thrd_create(thread, qcore_forced_trampoline, worker);
    if (outcome != thrd_success) {
        /* A worker that was never created must not be waited for — and, if the
         * cohort trigger already released it, must not be waited for at an exit
         * barrier either, so its cohort seat is given back. */
        qcore_forced_lock();
        if (atomic_load_explicit(&worker->released, memory_order_relaxed)) {
            qcore_forced_cohort_size[worker->cohort] -= 1;
            qcore_forced_changed();
            atomic_store_explicit(&worker->released, 0, memory_order_relaxed);
            worker->cohort = 0;
        }
        --qcore_forced_created;
        qcore_forced_unlock();
    } else {
        worker->thread = *thread;
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

/* Card W45-TSAN-ROWS-LOAD-SENSITIVE: will this worker stay parked while its
 * creator waits? Only the creator opens the gate (the create, join and
 * drop-join hooks all run on it) and the creator is the caller, so a worker
 * not yet released whose generation no opened gate covers stays parked. Read
 * without the lock — `gate`, `generation`, `thread` and the table length are
 * the creator's own writes and `released` is a relaxed atomic — so the query
 * acquires nothing and orders no worker's body before the caller. */
__attribute__((unused)) QCORE_FORCED_UNINSTRUMENTED
static int qcore_forced_never_runs(thrd_t thread) {
    for (size_t i = 0; i < qcore_forced_created; ++i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i];
        if (thrd_equal(worker->thread, thread))
            return !atomic_load_explicit(&worker->released, memory_order_relaxed) &&
                   worker->generation >= qcore_forced_gate;
    }
    return 0;
}
#undef QCORE_TSAN_NEVER_RUNS
#define QCORE_TSAN_NEVER_RUNS(thread) qcore_forced_never_runs(thread)

#ifdef QCORE_TSAN_DROP_JOIN
/* The join-edge-removal mutation under forcing: open the gate as the real
 * join would, then wait — without joining and without an edge — until the
 * worker's start function has returned, and report success. */
__attribute__((unused)) QCORE_FORCED_UNINSTRUMENTED
static int qcore_forced_drop_join(thrd_t thread, int *status) {
    qcore_forced_lock();
    qcore_forced_open_gate();
    qcore_forced_unlock();
    return qcore_tsan_unordered_join(thread, status, qcore_forced_never_runs(thread));
}
#undef QCORE_TSAN_DROP_JOIN
#define QCORE_TSAN_DROP_JOIN qcore_forced_drop_join
#endif

#define QCORE_TASK_CREATE qcore_forced_create
#define QCORE_TASK_JOIN qcore_forced_join
"#;

/// Card `TSAN-FAILURE-ARMS`: the **failure arms** of a task program, injected
/// at the same `-include` seam and placed *after* [`SCHEDULE_FORCING`] so that
/// whatever `QCORE_TASK_CREATE` then means is what a permitted creation really
/// travels through. `QCORE_TSAN_FAILURE` selects the arm (`0` none, `1` the
/// second creation refuses, `2` `QCORE_BUFFER_MALLOC` returns `NULL`).
///
/// Neither injection is invented here. The refusing create is CT07's own
/// `refuse_second_spawn.h` rule — only the *second* creation refuses, so the
/// first worker is really created, really runs its body and is really joined on
/// the recovery arm — and the null allocator is the shape CT20's
/// `task_scenarios.h` already uses for `QCORE_CT_FAIL_ALLOC`. What is new is
/// that they compose with the pthread binding and the forced schedules, so a
/// failure arm can be run under ThreadSanitizer at all.
pub const FAILURE_INJECTION: &str = r#"
#include <stdlib.h>
#include <threads.h>

#ifndef QCORE_TSAN_FAILURE
#define QCORE_TSAN_FAILURE 0
#endif

#if QCORE_TSAN_FAILURE == 1
#ifndef QCORE_TASK_CREATE
#define QCORE_TASK_CREATE thrd_create
#endif
/* Whatever QCORE_TASK_CREATE means at this point — the binding's
 * pthread-backed thrd_create, or the forcing header's qcore_forced_create —
 * expands here, once, so the permitted creation is the row's own schedule. */
__attribute__((unused))
static int qcore_tsan_permitted_create(thrd_t *thread, int (*function)(void *), void *argument) {
    return QCORE_TASK_CREATE(thread, function, argument);
}
static int qcore_tsan_create_attempts = 0;
__attribute__((unused))
static int qcore_tsan_refuse_second_create(thrd_t *thread, int (*function)(void *), void *argument) {
    if (++qcore_tsan_create_attempts >= 2) return thrd_error;
    return qcore_tsan_permitted_create(thread, function, argument);
}
#undef QCORE_TASK_CREATE
#define QCORE_TASK_CREATE qcore_tsan_refuse_second_create
#endif

#if QCORE_TSAN_FAILURE == 2
/* The entry's own qcore_buffer_new goes through QCORE_BUFFER_MALLOC_FOR_TASK,
 * whose default is QCORE_BUFFER_MALLOC(size); defining the latter before the
 * emitted prelude leaves the prelude's own #ifndef untaken. */
__attribute__((unused))
static void *qcore_tsan_null_malloc(size_t size) { (void)size; return NULL; }
#define QCORE_BUFFER_MALLOC qcore_tsan_null_malloc
#endif
"#;

/// Which failure the injection header forces (card `TSAN-FAILURE-ARMS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// No injection: every creation and every allocation succeeds.
    None,
    /// Only the **second** `QCORE_TASK_CREATE` refuses, so the first worker
    /// runs and the program takes its spawn-refusal recovery arm.
    RefuseSecondCreate,
    /// `QCORE_BUFFER_MALLOC` returns `NULL`, so the entry's
    /// `qcore_buffer_new(...)?` takes its allocation-failure error edge.
    NullBufferMalloc,
}

impl Failure {
    pub fn tag(self) -> &'static str {
        match self {
            Failure::None => "no-failure",
            Failure::RefuseSecondCreate => "refuse-second-create",
            Failure::NullBufferMalloc => "null-buffer-malloc",
        }
    }

    fn selector(self) -> i32 {
        match self {
            Failure::None => 0,
            Failure::RefuseSecondCreate => 1,
            Failure::NullBufferMalloc => 2,
        }
    }
}

/// Card `TSAN-FAILURE-ARMS`: the `hook_visible_to_tsan` mutation, as a function
/// over the shipped [`SCHEDULE_FORCING`] text rather than as a patch file.
///
/// It puts back exactly what wave 39 measured and then removed: the forcing
/// header's bookkeeping lock becomes a **pthread mutex**, every function of the
/// header becomes instrumented, and a parked worker learns it was released *by
/// acquiring that mutex*. ThreadSanitizer then sees a release/acquire edge
/// between the releasing thread and the released one, and under
/// [`Schedule::ForcedReverse`] — where the releaser is the previous worker's
/// own `finish` — that edge orders the two workers' byte writes and the overlap
/// race goes **unreported**.
///
/// Every substitution asserts its anchor, so this function is also the
/// enforcement of `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`: a header edit that
/// moves the lock, the park loop or the `no_sanitize` attribute out of the
/// shapes below makes the mutation panic rather than silently become a no-op.
pub fn with_visible_synchronization(header: &str) -> String {
    const ANCHORS: [(&str, &str); 3] = [
        (
            "#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize(\"thread\")))",
            "#define QCORE_FORCED_UNINSTRUMENTED /* mutation: instrumented */",
        ),
        (
            "static atomic_flag qcore_forced_spin = ATOMIC_FLAG_INIT;",
            "#include <pthread.h>\nstatic pthread_mutex_t qcore_forced_visible = PTHREAD_MUTEX_INITIALIZER;",
        ),
        (
            concat!(
                "QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_lock(void) {\n",
                "    while (atomic_flag_test_and_set_explicit(&qcore_forced_spin, memory_order_acquire)) sched_yield();\n",
                "}\n",
                "QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_unlock(void) {\n",
                "    atomic_flag_clear_explicit(&qcore_forced_spin, memory_order_release);\n",
                "}",
            ),
            concat!(
                "static void qcore_forced_lock(void) { pthread_mutex_lock(&qcore_forced_visible); }\n",
                "static void qcore_forced_unlock(void) { pthread_mutex_unlock(&qcore_forced_visible); }",
            ),
        ),
    ];
    const PARK: (&str, &str) = (
        concat!(
            "QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_park(struct qcore_forced_worker *worker) {\n",
            "    while (!atomic_load_explicit(&worker->released, memory_order_relaxed)) sched_yield();\n",
            "}",
        ),
        concat!(
            "static void qcore_forced_park(struct qcore_forced_worker *worker) {\n",
            "    for (;;) {\n",
            "        int released;\n",
            "        qcore_forced_lock();\n",
            "        released = atomic_load_explicit(&worker->released, memory_order_relaxed);\n",
            "        qcore_forced_unlock();\n",
            "        if (released) return;\n",
            "        sched_yield();\n",
            "    }\n",
            "}",
        ),
    );
    let mut text = header.to_owned();
    for (anchor, replacement) in ANCHORS.iter().chain(std::iter::once(&PARK)) {
        assert_eq!(
            text.matches(anchor).count(),
            1,
            "the invisibility mutation's anchor must appear exactly once in the shipped \
             forcing header; it does not, so the header has moved out of the shape \
             TSAN-FORCED-HARNESS-SYNC-INVISIBILITY describes:\n{anchor}"
        );
        text = text.replace(anchor, replacement);
    }
    text
}

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
    Row {
        tag,
        source: c_source,
        bind_threads,
        schedule,
        tsan_options,
        ..Row::default()
    }
    .execute()
}

/// One row's complete run description (card `TSAN-FAILURE-ARMS`). The wave-39
/// entry points above are this with their own fields set and everything else
/// left at its wave-39 default, so no existing row's meaning moves.
#[derive(Clone, Copy, Debug)]
pub struct Row<'a> {
    pub tag: &'a str,
    pub source: &'a str,
    pub bind_threads: bool,
    pub schedule: Schedule,
    /// The row's own cohort count — the number of workers this program parks
    /// before the trigger should fire. `0` is wave 39's join-only gate.
    pub cohort: usize,
    /// Which failure arm the program is driven onto.
    pub failure: Failure,
    pub tsan_options: &'a str,
    /// A replacement for [`SCHEDULE_FORCING`] — used only by the
    /// `hook_visible_to_tsan` mutation, which must compile a *different*
    /// forcing header while changing nothing else about the run.
    pub forcing: Option<&'a str>,
    /// `Some((ordinal, ms))`: the `ordinal`-th thread created sleeps `ms`
    /// before its start function runs — card `W45-TSAN-ROWS-LOAD-SENSITIVE`'s
    /// deterministic stand-in for a partner descheduled under load, through
    /// the binding's `QCORE_TSAN_HOLD_ORDINAL`/`QCORE_TSAN_HOLD_MS`.
    pub hold: Option<(usize, u32)>,
}

impl Default for Row<'_> {
    fn default() -> Self {
        Row {
            tag: "row",
            source: "",
            bind_threads: true,
            schedule: Schedule::Unforced,
            cohort: 0,
            failure: Failure::None,
            tsan_options: "halt_on_error=1",
            forcing: None,
            hold: None,
        }
    }
}

#[cfg(target_os = "linux")]
impl Row<'_> {
    pub fn execute(self) -> Result<Sanitized, String> {
        run_row(self)
    }
}

#[cfg(target_os = "linux")]
fn run_row(row: Row<'_>) -> Result<Sanitized, String> {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let Row {
        tag,
        source: c_source,
        bind_threads,
        schedule,
        cohort,
        failure,
        tsan_options,
        forcing: forcing_override,
        hold,
    } = row;

    // Run-scoped by tag, schedule, failure arm and pid, so two runs cannot
    // truncate each other.
    let directory = std::env::temp_dir().join(format!(
        "qcore-task-route-tsan-{tag}-{}-{}-{}",
        schedule.tag(),
        failure.tag(),
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let binding = directory.join("bind.h");
    let forcing = directory.join("force.h");
    let failing = directory.join("fail.h");
    let executable = directory.join("module");
    std::fs::write(&c, c_source).unwrap();
    std::fs::write(&binding, PTHREAD_BINDING).unwrap();
    std::fs::write(&forcing, forcing_override.unwrap_or(SCHEDULE_FORCING)).unwrap();
    std::fs::write(&failing, FAILURE_INJECTION).unwrap();
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
            compiler.arg(format!("-DQCORE_FORCED_COHORT={cohort}"));
            compiler.arg("-include").arg(&forcing);
        }
        Schedule::ForcedReverse => {
            compiler.arg("-DQCORE_FORCED_REVERSE=1");
            compiler.arg(format!("-DQCORE_FORCED_COHORT={cohort}"));
            compiler.arg("-include").arg(&forcing);
        }
    }
    if let Some((ordinal, ms)) = hold {
        assert!(bind_threads, "a hold rides the pthread binding");
        compiler.arg(format!("-DQCORE_TSAN_HOLD_ORDINAL={ordinal}"));
        compiler.arg(format!("-DQCORE_TSAN_HOLD_MS={ms}"));
    }
    if failure != Failure::None {
        compiler.arg(format!("-DQCORE_TSAN_FAILURE={}", failure.selector()));
        compiler.arg("-include").arg(&failing);
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
