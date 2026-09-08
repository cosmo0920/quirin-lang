#ifndef QCORE_C11_THREADS_COMPAT_H
#define QCORE_C11_THREADS_COMPAT_H

/* Apple Clang does not ship C11 <threads.h>. Keep the generated runtime's
 * small C11-thread subset source-compatible by implementing it with pthreads.
 * Other targets continue to use their native C11 threads implementation. */
#if defined(__APPLE__)
#include <pthread.h>
#include <sched.h>
#include <stdint.h>
#include <stdlib.h>

typedef pthread_t thrd_t;
typedef int (*thrd_start_t)(void *);
typedef pthread_mutex_t mtx_t;

enum { thrd_success = 0, thrd_error = 1, mtx_plain = 0 };
#define QCORE_THREADS_UNUSED __attribute__((unused))

struct qcore_pthread_start {
    thrd_start_t function;
    void *argument;
};

static void *qcore_pthread_trampoline(void *opaque) {
    struct qcore_pthread_start start = *(struct qcore_pthread_start *)opaque;
    free(opaque);
    return (void *)(intptr_t)start.function(start.argument);
}

static QCORE_THREADS_UNUSED int thrd_create(
    thrd_t *thread, thrd_start_t function, void *argument) {
    struct qcore_pthread_start *start = malloc(sizeof(*start));
    if (start == NULL) return thrd_error;
    start->function = function;
    start->argument = argument;
    if (pthread_create(thread, NULL, qcore_pthread_trampoline, start) != 0) {
        free(start);
        return thrd_error;
    }
    return thrd_success;
}

static QCORE_THREADS_UNUSED int thrd_join(thrd_t thread, int *result) {
    void *value = NULL;
    if (pthread_join(thread, &value) != 0) return thrd_error;
    if (result != NULL) *result = (int)(intptr_t)value;
    return thrd_success;
}

static QCORE_THREADS_UNUSED int thrd_equal(thrd_t left, thrd_t right) {
    return pthread_equal(left, right) != 0;
}

static QCORE_THREADS_UNUSED void thrd_yield(void) {
    (void)sched_yield();
}

static QCORE_THREADS_UNUSED int mtx_init(mtx_t *mutex, int kind) {
    (void)kind;
    return pthread_mutex_init(mutex, NULL) == 0 ? thrd_success : thrd_error;
}

static QCORE_THREADS_UNUSED int mtx_lock(mtx_t *mutex) {
    return pthread_mutex_lock(mutex) == 0 ? thrd_success : thrd_error;
}

static QCORE_THREADS_UNUSED int mtx_unlock(mtx_t *mutex) {
    return pthread_mutex_unlock(mutex) == 0 ? thrd_success : thrd_error;
}

static QCORE_THREADS_UNUSED void mtx_destroy(mtx_t *mutex) {
    if (pthread_mutex_destroy(mutex) != 0) abort();
}
#else
#include <threads.h>
#if defined(__STDC_NO_THREADS__)
#error "The QCore native concurrency profiles require C11 threads"
#endif
#endif

#endif
