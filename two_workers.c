#define QCORE_TASK_ARGUMENT_CAPACITY 1
#define QCORE_BUFFER_EVENT_CAPACITY 17
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
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

/* qcore_io (card TRACE-STDIO): the one printing route in this program.
   `std.io`'s `print`, `std.logger`'s `log` and every checked trace line the
   compiler synthesizes expand to QCORE_IO_WRITE, so the only `fprintf`
   spelling in this file is the one below. A macro rather than a variadic
   wrapper, so each call site keeps its -Wformat / C4477 checking under
   `-Wall -Wextra -Werror` and `/W4 /WX`. QCORE_IO_FLUSH is sink handoff, not
   durability: no fsync is performed and no crash survival is claimed. */
#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))
#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))
#ifndef QCORE_TASK_CALLOC
#define QCORE_TASK_CALLOC calloc
#endif
#ifndef QCORE_TASK_FREE
#define QCORE_TASK_FREE free
#endif
#ifndef QCORE_TASK_CREATE
#define QCORE_TASK_CREATE thrd_create
#endif
#ifndef QCORE_TASK_JOIN
#define QCORE_TASK_JOIN thrd_join
#endif
#ifndef QCORE_BUFFER_MALLOC
#define QCORE_BUFFER_MALLOC malloc
#endif
#ifndef QCORE_BUFFER_MALLOC_FOR_TASK
#define QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt) QCORE_BUFFER_MALLOC(size)
#endif
#ifndef QCORE_BUFFER_FREE
#define QCORE_BUFFER_FREE free
#endif
/* Only checked move and read-share operations expose this private allocation to
 * workers. The generation never changes after successful allocation; the
 * payload changes only through a checked exclusive-authority write. */
struct qcore_buffer { uint64_t generation; int32_t payload; };
struct qcore_task_value { int32_t scalar; struct qcore_buffer *buffer; };
struct qcore_task_result { int kind; struct qcore_task_value value; int error; };
struct qcore_buffer_event { int kind; uint64_t generation; int32_t payload; };
struct qcore_task_context {
    uint64_t identity;
    uint64_t next_task;
    uint64_t next_buffer;
    uint64_t allocation_attempt;
    size_t event_count;
    struct qcore_buffer_event events[QCORE_BUFFER_EVENT_CAPACITY];
};
typedef struct qcore_task_result (*qcore_task_worker)(const struct qcore_task_value *, struct qcore_task_context *);
struct qcore_task_start_record {
    thrd_t thread;
    uint64_t identity;
    qcore_task_worker worker;
    struct qcore_task_value arguments[QCORE_TASK_ARGUMENT_CAPACITY];
    struct qcore_task_result result;
    struct qcore_task_context context;
};
struct qcore_task_slot {
    struct qcore_task_value value;
    struct qcore_task_result result;
    struct qcore_task_start_record *handle;
    int error;
};
static const char *qcore_task_error_name(int);
static void qcore_buffer_print_event(uint64_t task, struct qcore_buffer_event event) {
    switch (event.kind) {
    case 0: QCORE_IO_WRITE(stdout, "QCORE_BUFFER_ALLOC %" PRIu64 " %" PRIu64 " %" PRId32 "\n", task, event.generation, event.payload); break;
    case 1: QCORE_IO_WRITE(stdout, "QCORE_BUFFER_ALLOC_FAILED %" PRIu64 "\n", task); break;
    case 2: QCORE_IO_WRITE(stdout, "QCORE_BUFFER_READ %" PRIu64 " %" PRIu64 " %" PRId32 "\n", task, event.generation, event.payload); break;
    case 3: QCORE_IO_WRITE(stdout, "QCORE_BUFFER_DROP %" PRIu64 " %" PRIu64 "\n", task, event.generation); break;
    case 4: QCORE_IO_WRITE(stdout, "QCORE_BUFFER_WRITE %" PRIu64 " %" PRIu64 " %" PRId32 "\n", task, event.generation, event.payload); break;
    default: abort();
    }
}
static void qcore_buffer_event(struct qcore_task_context *context, int kind, uint64_t generation, int32_t payload) {
    struct qcore_buffer_event event = {kind, generation, payload};
    if (context->identity == 0) qcore_buffer_print_event(0, event);
    else {
        /* Each leaf worker owns its event storage. Only its parent reads this
         * array, after successful native join. No shared trace counter exists. */
        if (context->event_count >= QCORE_BUFFER_EVENT_CAPACITY) abort();
        context->events[context->event_count++] = event;
    }
}
static struct qcore_buffer *qcore_buffer_new(int32_t payload, struct qcore_task_context *context) {
    /* Finite leaf tasks have disjoint identity ranges and private counters.
     * Failed allocations consume an attempt, not a resource generation. */
    if (context->identity > UINT32_MAX || context->next_buffer == 0 ||
        context->next_buffer > UINT32_MAX || context->allocation_attempt == UINT64_MAX) abort();
    ++context->allocation_attempt;
    struct qcore_buffer *buffer = QCORE_BUFFER_MALLOC_FOR_TASK(
        sizeof(*buffer), context->identity, context->allocation_attempt);
    if (buffer == NULL) {
        qcore_buffer_event(context, 1, 0, 0);
        return NULL;
    }
    buffer->generation = context->identity * UINT64_C(4294967296) + context->next_buffer++;
    buffer->payload = payload;
    qcore_buffer_event(context, 0, buffer->generation, payload);
    return buffer;
}
static int32_t qcore_buffer_read(const struct qcore_buffer *buffer, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    int32_t payload = buffer->payload;
    qcore_buffer_event(context, 2, buffer->generation, payload);
    return payload;
}
/* Only a checked exclusive-authority holder reaches this: no read share may be
 * outstanding, so no other thread is reading this object while it is written. */
static void qcore_buffer_write(struct qcore_buffer *buffer, int32_t payload, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    buffer->payload = payload;
    qcore_buffer_event(context, 4, buffer->generation, payload);
}
static void qcore_buffer_drop(struct qcore_buffer *buffer, struct qcore_task_context *context) {
    if (buffer == NULL) abort();
    qcore_buffer_event(context, 3, buffer->generation, 0);
    QCORE_BUFFER_FREE(buffer);
}
static void qcore_task_print_result(uint64_t identity, struct qcore_task_result result) {
    if (result.error != 0 && result.value.buffer != NULL)
        QCORE_IO_WRITE(stdout, "QCORE_TASK_RESULT %" PRIu64 " ERROR_BUFFER %s %" PRIu64 "\n", identity,
               qcore_task_error_name(result.error), result.value.buffer->generation);
    else if (result.error != 0)
        QCORE_IO_WRITE(stdout, "QCORE_TASK_RESULT %" PRIu64 " ERROR %s\n", identity, qcore_task_error_name(result.error));
    else if (result.kind == 0)
        QCORE_IO_WRITE(stdout, "QCORE_TASK_RESULT %" PRIu64 " UNIT\n", identity);
    else if (result.kind == 1)
        QCORE_IO_WRITE(stdout, "QCORE_TASK_RESULT %" PRIu64 " I32 %" PRId32 "\n", identity, result.value.scalar);
    else if (result.kind == 2) {
        if (result.value.buffer == NULL) abort();
        QCORE_IO_WRITE(stdout, "QCORE_TASK_RESULT %" PRIu64 " BUFFER %" PRIu64 "\n", identity, result.value.buffer->generation);
    } else abort();
}
static int qcore_task_trampoline(void *pointer) {
    struct qcore_task_start_record *record = pointer;
    record->result = record->worker(record->arguments, &record->context);
    return 0;
}
static struct qcore_task_start_record *qcore_task_start(qcore_task_worker worker,
        const struct qcore_task_value *arguments, size_t count, struct qcore_task_context *context, size_t worker_id) {
    if (context->next_task == 0 || context->next_task > UINT32_MAX) abort();
    struct qcore_task_start_record *record = QCORE_TASK_CALLOC(1, sizeof(*record));
    if (record == NULL) {
        QCORE_IO_WRITE(stdout, "QCORE_TASK_CREATE_FAILED %zu\n", worker_id);
        return NULL;
    }
    record->worker = worker;
    record->identity = context->next_task;
    record->context.identity = record->identity;
    record->context.next_buffer = 1;
    for (size_t i = 0; i < count; ++i) record->arguments[i] = arguments[i];
    if (QCORE_TASK_CREATE(&record->thread, qcore_task_trampoline, record) != thrd_success) {
        /* Creation failure never transfers the buffer; discard only the
         * environment's non-owning copies. The parent still owns each slot. */
        QCORE_TASK_FREE(record);
        QCORE_IO_WRITE(stdout, "QCORE_TASK_CREATE_FAILED %zu\n", worker_id);
        return NULL;
    }
    ++context->next_task;
    QCORE_IO_WRITE(stdout, "QCORE_TASK_SPAWN %" PRIu64 " %zu\n", record->identity, worker_id);
    return record;
}
static struct qcore_task_result qcore_task_collect(struct qcore_task_start_record *record) {
    int status;
    /* An unsuccessful join gives no permission to read completion data or
     * reclaim either the environment or an owned completion payload. */
    if (QCORE_TASK_JOIN(record->thread, &status) != thrd_success) abort();
    if (status != 0) abort();
    struct qcore_task_result result = record->result;
    for (size_t i = 0; i < record->context.event_count; ++i)
        qcore_buffer_print_event(record->identity, record->context.events[i]);
    qcore_task_print_result(record->identity, result);
    QCORE_IO_WRITE(stdout, "QCORE_TASK_JOIN %" PRIu64 "\n", record->identity);
    QCORE_TASK_FREE(record);
    return result;
}
static const char *qcore_task_error_name(int tag) {
    switch (tag) {
    case 1: return "\101\154\154\157\143\105\162\162\157\162";
    case 2: return "\123\160\141\167\156\105\162\162\157\162";
    default: abort();
    }
}
static struct qcore_task_result qcore_task_fn_0(const struct qcore_task_value *, struct qcore_task_context *);
static struct qcore_task_result qcore_task_fn_1(const struct qcore_task_value *, struct qcore_task_context *);
static struct qcore_task_result qcore_task_fn_0(const struct qcore_task_value *args, struct qcore_task_context *context) {
    (void)args; (void)context;
    struct qcore_task_slot s_2 = {0}; (void)&s_2;
    s_2.value = args[0];
    goto qcore_block_0;
qcore_block_0: ;
    { struct qcore_task_result returned = {1, (struct qcore_task_value){s_2.value.scalar, NULL}, 0};
    return returned; }
}
static struct qcore_task_result qcore_task_fn_1(const struct qcore_task_value *args, struct qcore_task_context *context) {
    (void)args; (void)context;
    struct qcore_task_slot s_3 = {0}; (void)&s_3;
    struct qcore_task_slot s_4 = {0}; (void)&s_4;
    struct qcore_task_slot s_6 = {0}; (void)&s_6;
    struct qcore_task_slot s_7 = {0}; (void)&s_7;
    struct qcore_task_slot s_8 = {0}; (void)&s_8;
    struct qcore_task_slot s_9 = {0}; (void)&s_9;
    struct qcore_task_slot s_11 = {0}; (void)&s_11;
    struct qcore_task_slot s_12 = {0}; (void)&s_12;
    struct qcore_task_slot s_13 = {0}; (void)&s_13;
    struct qcore_task_slot s_14 = {0}; (void)&s_14;
    goto qcore_block_0;
qcore_block_0: ;
    s_3.value = (struct qcore_task_value){INT32_C(42), NULL};
    s_4.value = (struct qcore_task_value){INT32_C(7), NULL};
    /* checked scope 5 begins */
    {
        struct qcore_task_value captures[QCORE_TASK_ARGUMENT_CAPACITY] = {{0}};
        captures[0] = (struct qcore_task_value){s_3.value.scalar, NULL};
        s_6.handle = qcore_task_start(qcore_task_fn_0, captures, 1, context, 0);
        if (s_6.handle == NULL) goto qcore_block_2;
        goto qcore_block_1;
    }
qcore_block_1: ;
    {
        struct qcore_task_value captures[QCORE_TASK_ARGUMENT_CAPACITY] = {{0}};
        captures[0] = (struct qcore_task_value){s_4.value.scalar, NULL};
        s_7.handle = qcore_task_start(qcore_task_fn_0, captures, 1, context, 0);
        if (s_7.handle == NULL) goto qcore_block_4;
        goto qcore_block_3;
    }
qcore_block_2: ;
    /* checked scope 5 exits without tasks */
    return (struct qcore_task_result){0, {0, NULL}, 2};
qcore_block_3: ;
    {
        s_12.result = qcore_task_collect(s_7.handle);
        s_7.handle = NULL;
    }
    s_8.value = s_12.result.value; s_12.result.value.buffer = NULL;
    {
        s_13.result = qcore_task_collect(s_6.handle);
        s_6.handle = NULL;
    }
    s_9.value = s_13.result.value; s_13.result.value.buffer = NULL;
    /* checked scope 5 exits without tasks */
    { struct qcore_task_result returned = {1, (struct qcore_task_value){s_8.value.scalar, NULL}, 0};
    return returned; }
qcore_block_4: ;
    {
        s_14.result = qcore_task_collect(s_6.handle);
        s_6.handle = NULL;
    }
    s_11.value = s_14.result.value; s_14.result.value.buffer = NULL;
    /* checked scope 5 exits without tasks */
    { struct qcore_task_result returned = {1, (struct qcore_task_value){s_11.value.scalar, NULL}, 0};
    return returned; }
}
int main(void) {
    (void)&qcore_task_start; (void)&qcore_task_collect; (void)&qcore_buffer_new; (void)&qcore_buffer_read; (void)&qcore_buffer_write; (void)&qcore_buffer_drop;
    (void)&qcore_task_fn_0;
    (void)&qcore_task_fn_1;
    struct qcore_task_context context = {0}; context.next_task = 1; context.next_buffer = 1;
    struct qcore_task_result result = qcore_task_fn_1(NULL, &context);
    qcore_task_print_result(0, result);
    return 0;
}
