#ifndef QCORE_C11_THREADS_COMPAT_H
#define QCORE_C11_THREADS_COMPAT_H

/* The generated runtime uses only joinable threads and plain mutexes.
 * Windows and Apple Clang need this subset of C11 <threads.h>; this is not
 * a replacement for the full standard header (no detach, TLS or conditions).
 * Keep it embedded so emitted C needs no QCore include paths or libraries. */
#if defined(_WIN32)
/* Adapted from the joinable-thread/mutex portion of Fluent Bit's Monkey
 * mk_core/external/winpthreads.c and include/monkey/mk_core/external/winpthreads.h.
 * Upstream revision: f78472443d32691c319c424c75d9ea2343dbc8f7.
 * Local changes: C11 names and int results, no cancellation/TLS/detach,
 * checked create/wait/close failures, no worker access to the published handle,
 * and SRW locks for the supported non-recursive mtx_plain mode.
 *
 * (C) 2010 Lockless Inc.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *  * Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *  * Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *  * Neither the name of Lockless Inc. nor the names of its contributors may be
 *    used to endorse or promote products derived from this software without
 *    specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AN
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE
 * OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED
 * OF THE POSSIBILITY OF SUCH DAMAGE.
 */
#include <windows.h>
#include <process.h>
#include <stdint.h>
#include <stdlib.h>

typedef int (*thrd_start_t)(void *);
struct qcore_winpthread {
    HANDLE handle;
    thrd_start_t function;
    void *argument;
    int result;
};
typedef struct qcore_winpthread *thrd_t;
typedef SRWLOCK mtx_t;
enum { thrd_success = 0, thrd_error = 1, thrd_nomem = 2, mtx_plain = 0 };
#if defined(__GNUC__) || defined(__clang__)
#define QCORE_THREADS_UNUSED __attribute__((unused))
#else
#define QCORE_THREADS_UNUSED
#endif

static unsigned __stdcall qcore_winpthread_trampoline(void *opaque) {
    struct qcore_winpthread *thread = (struct qcore_winpthread *)opaque;
    thread->result = thread->function(thread->argument);
    /* Only the joiner owns the handle and record, even after completion. */
    return 0;
}

static QCORE_THREADS_UNUSED inline int thrd_create(thrd_t *thread, thrd_start_t function, void *argument) {
    struct qcore_winpthread *start = malloc(sizeof(*start));
    uintptr_t handle;
    if (start == NULL) return thrd_nomem;
    start->function = function;
    start->argument = argument;
    handle = _beginthreadex(NULL, 0, qcore_winpthread_trampoline, start, 0, NULL);
    if (handle == 0) {
        free(start);
        return thrd_error;
    }
    start->handle = (HANDLE)handle;
    *thread = start;
    return thrd_success;
}

static QCORE_THREADS_UNUSED inline int thrd_join(thrd_t thread, int *result) {
    if (WaitForSingleObject(thread->handle, INFINITE) != WAIT_OBJECT_0)
        return thrd_error;
    if (!CloseHandle(thread->handle)) return thrd_error;
    if (result != NULL) *result = thread->result;
    free(thread);
    return thrd_success;
}

static QCORE_THREADS_UNUSED inline int thrd_equal(thrd_t left, thrd_t right) {
    return left == right;
}

static QCORE_THREADS_UNUSED inline void thrd_yield(void) {
    (void)SwitchToThread();
}

static QCORE_THREADS_UNUSED inline int mtx_init(mtx_t *mutex, int kind) {
    if (kind != mtx_plain) return thrd_error;
    InitializeSRWLock(mutex);
    return thrd_success;
}

static QCORE_THREADS_UNUSED inline int mtx_lock(mtx_t *mutex) {
    AcquireSRWLockExclusive(mutex);
    return thrd_success;
}

static QCORE_THREADS_UNUSED inline int mtx_unlock(mtx_t *mutex) {
    ReleaseSRWLockExclusive(mutex);
    return thrd_success;
}

static QCORE_THREADS_UNUSED inline void mtx_destroy(mtx_t *mutex) {
    /* An unlocked SRW lock owns no separately allocated OS resource. */
    (void)mutex;
}
#elif defined(__APPLE__)
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
