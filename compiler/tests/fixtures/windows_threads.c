/* Test actual OS threads, with deterministic failures at the adapter boundary. */
#include <windows.h>
#include <process.h>
#include <stdint.h>
#include <stdlib.h>
#include <limits.h>
#include <assert.h>

static int fail_alloc, fail_create, fail_close, complete_before_return;
static DWORD failed_wait;
static int live_records;
static void *tracked_alloc(size_t size) {
    void *p;
    if (fail_alloc) return NULL;
    p = malloc(size);
    if (p) ++live_records;
    return p;
}
static void tracked_free(void *p) {
    assert(p && live_records > 0);
    --live_records;
    free(p);
}
static uintptr_t tracked_begin(void *security, unsigned size,
    unsigned (__stdcall *start)(void *), void *arg, unsigned flags, unsigned *id) {
    uintptr_t h;
    if (fail_create) return 0;
    h = _beginthreadex(security, size, start, arg, flags, id);
    if (h && complete_before_return)
        assert(WaitForSingleObject((HANDLE)h, INFINITE) == WAIT_OBJECT_0);
    return h;
}
static DWORD tracked_wait(HANDLE h, DWORD timeout) {
    return failed_wait ? failed_wait : WaitForSingleObject(h, timeout);
}
static BOOL tracked_close(HANDLE h) {
    return fail_close ? FALSE : CloseHandle(h);
}
#define malloc tracked_alloc
#define free tracked_free
#define _beginthreadex tracked_begin
#define WaitForSingleObject tracked_wait
#define CloseHandle tracked_close
#include "c11_threads_compat.h"
#undef malloc
#undef free
#undef _beginthreadex
#undef WaitForSingleObject
#undef CloseHandle

static int result_worker(void *arg) { return *(int *)arg; }
static mtx_t mutex;
static int counter;
static int increment(void *arg) {
    int i;
    (void)arg;
    for (i = 0; i < 2000; ++i) {
        assert(mtx_lock(&mutex) == thrd_success);
        ++counter;
        assert(mtx_unlock(&mutex) == thrd_success);
        thrd_yield();
    }
    return 0;
}

int main(void) {
    thrd_t t = NULL, threads[4];
    int values[] = { INT_MIN, -1, 0, 42, INT_MAX };
    int result = 123, value = -42, i, round;
    DWORD handles_before, handles_after;
    assert(GetProcessHandleCount(GetCurrentProcess(), &handles_before));
    fail_alloc = 1;
    assert(thrd_create(&t, result_worker, &value) == thrd_nomem);
    assert(t == NULL && live_records == 0);
    fail_alloc = 0;
    fail_create = 1;
    assert(thrd_create(&t, result_worker, &value) == thrd_error);
    assert(t == NULL && live_records == 0);
    fail_create = 0;

    assert(thrd_create(&t, result_worker, &value) == thrd_success);
    failed_wait = WAIT_FAILED;
    assert(thrd_join(t, &result) == thrd_error);
    assert(result == 123 && live_records == 1);
    failed_wait = WAIT_TIMEOUT;
    assert(thrd_join(t, &result) == thrd_error);
    assert(result == 123 && live_records == 1);
    failed_wait = 0;
    fail_close = 1;
    assert(thrd_join(t, &result) == thrd_error);
    assert(result == 123 && live_records == 1);
    fail_close = 0;
    assert(thrd_join(t, &result) == thrd_success);
    assert(result == value && live_records == 0);

    /* A completed child must remain joinable, with its full signed int result. */
    complete_before_return = 1;
    for (round = 0; round < 32; ++round) {
        for (i = 0; i < 5; ++i) {
            assert(thrd_create(&t, result_worker, &values[i]) == thrd_success);
            assert(thrd_equal(t, t) && live_records == 1);
            assert(thrd_join(t, &result) == thrd_success);
            assert(result == values[i] && live_records == 0);
        }
    }
    assert(thrd_create(&t, result_worker, &value) == thrd_success);
    assert(thrd_join(t, NULL) == thrd_success && live_records == 0);
    complete_before_return = 0;

    assert(mtx_init(&mutex, 99) == thrd_error);
    assert(mtx_init(&mutex, mtx_plain) == thrd_success);
    for (i = 0; i < 4; ++i)
        assert(thrd_create(&threads[i], increment, NULL) == thrd_success);
    assert(!thrd_equal(threads[0], threads[1]));
    for (i = 0; i < 4; ++i)
        assert(thrd_join(threads[i], NULL) == thrd_success);
    assert(counter == 8000 && live_records == 0);
    mtx_destroy(&mutex);
    assert(GetProcessHandleCount(GetCurrentProcess(), &handles_after));
    assert(handles_after == handles_before);
    return 0;
}
