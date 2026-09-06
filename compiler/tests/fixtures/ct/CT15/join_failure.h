/* Test-only native adapter: source contains no success/failure oracle. */
#include <threads.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#if QCORE_CT_FAIL_JOIN
static void ct15_observed_abort(int signal_number) {
    (void)signal_number;
    _Exit(77);
}
#endif
static int ct15_join(thrd_t thread, int *result) {
#if QCORE_CT_FAIL_JOIN
    (void)thread;
    (void)result;
    /* Flush publication so the harness can detect a result incorrectly printed
       before native quiescence. Do not call native join or initialize result. */
    fflush(stdout);
    fputs("JOIN_FAILED\n", stderr);
    fflush(stderr);
    if (signal(SIGABRT, ct15_observed_abort) == SIG_ERR) _Exit(92);
    return thrd_error;
#else
    int status = thrd_join(thread, result);
    if (status == thrd_success) fputs("JOIN_OK\n", stderr);
    return status;
#endif
}
static void ct15_free(void *pointer) {
    fputs("FREE\n", stderr);
    fflush(stderr);
#if QCORE_CT_FAIL_JOIN
    (void)pointer;
    _Exit(91);
#else
    free(pointer);
#endif
}
#define QCORE_TASK_JOIN ct15_join
#define QCORE_TASK_FREE ct15_free
