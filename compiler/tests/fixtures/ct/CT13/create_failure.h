/* Test-only create outcomes. Source handles a real discriminated result. */
#include <threads.h>
#include <stdio.h>
#include <stdlib.h>
static int ct13_create_count;
static thrd_t ct13_threads[2];
static int ct13_created[2];
static void *ct13_records[2];
static int ct13_create(thrd_t *thread, thrd_start_t function, void *argument) {
    ++ct13_create_count;
    if (ct13_create_count > 2) abort();
    if (ct13_create_count == QCORE_CT_FAIL_CREATE) {
        fprintf(stderr, "CREATE_FAILED:%d\n", ct13_create_count);
        return thrd_error;
    }
    int status = thrd_create(thread, function, argument);
    if (status == thrd_success) {
        ct13_threads[ct13_create_count - 1] = *thread;
        ct13_created[ct13_create_count - 1] = 1;
        ct13_records[ct13_create_count - 1] = argument;
        fprintf(stderr, "CREATE_OK:%d\n", ct13_create_count);
    }
    return status;
}
static int ct13_join(thrd_t thread, int *result) {
    int identity = 0;
    for (int i = 0; i < 2; ++i)
        if (ct13_created[i] && thrd_equal(thread, ct13_threads[i])) identity = i + 1;
    if (identity == 0) _Exit(93);
    int status = thrd_join(thread, result);
    if (status == thrd_success) {
        ct13_created[identity - 1] = 0;
        fprintf(stderr, "JOIN_OK:%d\n", identity);
    }
    return status;
}
static void ct13_free(void *pointer) {
    for (int i = 0; i < 2; ++i)
        if (ct13_created[i] && pointer == ct13_records[i]) _Exit(94);
    fputs("FREE\n", stderr);
    free(pointer);
}
#define QCORE_TASK_CREATE ct13_create
#define QCORE_TASK_JOIN ct13_join
#define QCORE_TASK_FREE ct13_free
