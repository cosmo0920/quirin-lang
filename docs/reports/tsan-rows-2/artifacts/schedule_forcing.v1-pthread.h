/* Card TSAN-ROWS-2 (wave 39): the FIRST version of the forcing header, kept
 * as an artifact because it is the card's catch point and was never committed.
 *
 * It parks and releases workers on a pthread mutex and condition variable.
 * Under QCORE_FORCED_REVERSE=1 the overlap mutation (both workers' byte writes
 * rebased onto byte 0 of one int32_t) went UNREPORTED by ThreadSanitizer —
 * measure/run-tsan-1.log, archived beside this file as
 * catch-point-reverse-mutex-edge.log — while the unforced and the together
 * schedules reported it. The reason: worker 2's byte write precedes its
 * pthread_mutex_unlock in qcore_forced_trampoline, and worker 1 wakes from
 * pthread_cond_wait by acquiring the same mutex, so TSan sees a
 * release/acquire edge between the two writes. A schedule realised with
 * synchronization TSan can see hides exactly the races the rows exist to
 * detect. The shipped header (tests/support/thread_sanitizer.rs,
 * SCHEDULE_FORCING) is uninstrumented for that reason. Do not use this one. */
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <time.h>

#ifndef QCORE_FORCED_REVERSE
#define QCORE_FORCED_REVERSE 0
#endif
#define QCORE_FORCED_CAPACITY 64

struct qcore_forced_worker {
    int (*function)(void *);
    void *argument;
    size_t ordinal;     /* 1-based creation order */
    size_t generation;  /* the creator's join count when this worker was created */
    size_t cohort;      /* the join (1-based) that released it; 0 while parked */
    int released;
    int finished;
};
static pthread_mutex_t qcore_forced_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t qcore_forced_changed = PTHREAD_COND_INITIALIZER;
static struct qcore_forced_worker qcore_forced_workers[QCORE_FORCED_CAPACITY];
static size_t qcore_forced_created = 0;
static size_t qcore_forced_joins = 0;
static size_t qcore_forced_cohort_size[QCORE_FORCED_CAPACITY + 1];
static size_t qcore_forced_cohort_finished[QCORE_FORCED_CAPACITY + 1];

/* Under the lock: is a released worker still running its body? */
static int qcore_forced_body_running(void) {
    for (size_t i = 0; i < qcore_forced_created; ++i)
        if (qcore_forced_workers[i].released && !qcore_forced_workers[i].finished) return 1;
    return 0;
}

/* Under the lock, reverse policy: release the most recently created worker
 * that is eligible (created before the last join) and not yet released, and
 * only when no released body is still running. */
static void qcore_forced_release_next_reverse(void) {
    if (qcore_forced_body_running()) return;
    for (size_t i = qcore_forced_created; i > 0; --i) {
        struct qcore_forced_worker *worker = &qcore_forced_workers[i - 1];
        if (worker->generation < qcore_forced_joins && !worker->released) {
            worker->released = 1;
            worker->cohort = qcore_forced_joins;
            qcore_forced_cohort_size[worker->cohort] += 1;
            return;
        }
    }
}

/* Under the lock: the creator reached a join, so every worker parked before
 * it becomes eligible. */
static void qcore_forced_open_gate(void) {
    if (qcore_forced_joins >= QCORE_FORCED_CAPACITY) abort();
    ++qcore_forced_joins;
    if (QCORE_FORCED_REVERSE) {
        qcore_forced_release_next_reverse();
    } else {
        for (size_t i = 0; i < qcore_forced_created; ++i) {
            struct qcore_forced_worker *worker = &qcore_forced_workers[i];
            if (worker->generation < qcore_forced_joins && !worker->released) {
                worker->released = 1;
                worker->cohort = qcore_forced_joins;
                qcore_forced_cohort_size[worker->cohort] += 1;
            }
        }
    }
    pthread_cond_broadcast(&qcore_forced_changed);
}

static int qcore_forced_trampoline(void *opaque) {
    struct qcore_forced_worker *worker = opaque;
    int result;
    pthread_mutex_lock(&qcore_forced_lock);
    while (!worker->released) pthread_cond_wait(&qcore_forced_changed, &qcore_forced_lock);
    fprintf(stderr, "QCORE_FORCED start %zu\n", worker->ordinal);
    pthread_mutex_unlock(&qcore_forced_lock);

    result = worker->function(worker->argument);

    pthread_mutex_lock(&qcore_forced_lock);
    worker->finished = 1;
    qcore_forced_cohort_finished[worker->cohort] += 1;
    if (QCORE_FORCED_REVERSE) qcore_forced_release_next_reverse();
    pthread_cond_broadcast(&qcore_forced_changed);
    /* Together: the exit barrier keeps this thread live until every body of
     * its cohort has returned. */
    while (!QCORE_FORCED_REVERSE &&
           qcore_forced_cohort_finished[worker->cohort] < qcore_forced_cohort_size[worker->cohort])
        pthread_cond_wait(&qcore_forced_changed, &qcore_forced_lock);
    fprintf(stderr, "QCORE_FORCED leave %zu\n", worker->ordinal);
    pthread_mutex_unlock(&qcore_forced_lock);
    return result;
}

/* Signature-compatible with thrd_create: the runtime's QCORE_TASK_CREATE. */
__attribute__((unused))
static int qcore_forced_create(thrd_t *thread, int (*function)(void *), void *argument) {
    struct qcore_forced_worker *worker;
    int outcome;
    pthread_mutex_lock(&qcore_forced_lock);
    if (qcore_forced_created >= QCORE_FORCED_CAPACITY) abort();
    worker = &qcore_forced_workers[qcore_forced_created];
    worker->function = function;
    worker->argument = argument;
    worker->ordinal = qcore_forced_created + 1;
    worker->generation = qcore_forced_joins;
    worker->cohort = 0;
    worker->released = 0;
    worker->finished = 0;
    ++qcore_forced_created;
    pthread_mutex_unlock(&qcore_forced_lock);
    outcome = thrd_create(thread, qcore_forced_trampoline, worker);
    if (outcome != thrd_success) {
        pthread_mutex_lock(&qcore_forced_lock);
        --qcore_forced_created;
        pthread_mutex_unlock(&qcore_forced_lock);
    }
    return outcome;
}

/* The runtime's QCORE_TASK_JOIN: open the gate, then really join. */
__attribute__((unused))
static int qcore_forced_join(thrd_t thread, int *result) {
    pthread_mutex_lock(&qcore_forced_lock);
    qcore_forced_open_gate();
    pthread_mutex_unlock(&qcore_forced_lock);
    return thrd_join(thread, result);
}

__attribute__((unused))
static int qcore_forced_drop_join(thrd_t thread, int *status) {
    struct timespec pause = {0, 50000000};
    (void)thread;
    pthread_mutex_lock(&qcore_forced_lock);
    qcore_forced_open_gate();
    pthread_mutex_unlock(&qcore_forced_lock);
    thrd_sleep(&pause, NULL);
    if (status != NULL) *status = 0;
    return thrd_success;
}
#undef QCORE_TSAN_DROP_JOIN
#define QCORE_TSAN_DROP_JOIN qcore_forced_drop_join

#define QCORE_TASK_CREATE qcore_forced_create
#define QCORE_TASK_JOIN qcore_forced_join
