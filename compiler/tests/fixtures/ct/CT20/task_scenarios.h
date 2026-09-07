/* Test-only failures and exact allocation/thread identity checks. */
#include <threads.h>
#include <stdio.h>
#include <stdlib.h>
static void *ct20_buffer;
static void *ct20_records[2];
static thrd_t ct20_threads[2];
static int ct20_created[2];
static int ct20_allocations, ct20_creates;
static void ct20_finish(void) {
    if (ct20_buffer || ct20_records[0] || ct20_records[1] ||
        ct20_created[0] || ct20_created[1]) _Exit(91);
    fputs("RUN_CLEAN\n", stderr);
}
static void *ct20_buffer_alloc(size_t size) {
    if (atexit(ct20_finish) != 0 || ct20_buffer) abort();
    if (QCORE_CT_FAIL_ALLOC == 1) {
        fputs("BUFFER_FAILED\n", stderr);
        return NULL;
    }
    ct20_buffer = malloc(size);
    if (!ct20_buffer) abort();
    fputs("BUFFER_ALLOC\n", stderr);
    return ct20_buffer;
}
static void ct20_buffer_free(void *pointer) {
    if (!pointer || pointer != ct20_buffer || ct20_created[0] || ct20_created[1]) abort();
    ct20_buffer = NULL;
    fputs("BUFFER_FREE\n", stderr);
    free(pointer);
}
static void *ct20_env_alloc(size_t count, size_t size) {
    if (ct20_allocations == 2) abort();
    void *pointer = calloc(count, size);
    if (!pointer) abort();
    ct20_records[ct20_allocations++] = pointer;
    fprintf(stderr, "ENV_ALLOC:%d\n", ct20_allocations);
    return pointer;
}
static int ct20_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    int index = ct20_creates++;
    if (index >= 2 || argument != ct20_records[index]) abort();
    if (QCORE_CT_FAIL_CREATE == index + 1) {
        fprintf(stderr, "CREATE_FAILED:%d\n", index + 1);
        return thrd_error;
    }
    int status = thrd_create(thread, worker, argument);
    if (status != thrd_success) abort();
    ct20_threads[index] = *thread;
    ct20_created[index] = 1;
    fprintf(stderr, "CREATE_OK:%d\n", index + 1);
    return status;
}
static int ct20_join(thrd_t thread, int *result) {
    int index = -1;
    for (int i = 0; i < 2; ++i)
        if (ct20_created[i] && thrd_equal(thread, ct20_threads[i])) index = i;
    if (index < 0 || !ct20_records[index]) abort();
    int status = thrd_join(thread, result);
    if (status != thrd_success) abort();
    ct20_created[index] = 0;
    fprintf(stderr, "JOIN_OK:%d\n", index + 1);
    return status;
}
static void ct20_env_free(void *pointer) {
    int index = -1;
    for (int i = 0; i < 2; ++i)
        if (pointer && pointer == ct20_records[i]) index = i;
    if (index < 0 || ct20_created[index] || pointer == ct20_buffer) abort();
    ct20_records[index] = NULL;
    fprintf(stderr, "ENV_FREE:%d\n", index + 1);
    free(pointer);
}
#define QCORE_BUFFER_MALLOC ct20_buffer_alloc
#define QCORE_BUFFER_FREE ct20_buffer_free
#define QCORE_TASK_CALLOC ct20_env_alloc
#define QCORE_TASK_FREE ct20_env_free
#define QCORE_TASK_CREATE ct20_create
#define QCORE_TASK_JOIN ct20_join
