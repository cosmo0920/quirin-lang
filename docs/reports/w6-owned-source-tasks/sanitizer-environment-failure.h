/* Pointer-identity and join-before-reclamation controls for the bounded Buffer profile. */
#include <threads.h>
#include <stdio.h>
#include <stdlib.h>
static void *ct_buffer;
static int ct_running;
static void *ct_alloc(size_t size) {
    if (ct_buffer) abort();
    ct_buffer = malloc(size);
    if (!ct_buffer) abort();
    fputs("BUFFER_ALLOC\n", stderr);
    return ct_buffer;
}
static void ct_buffer_free(void *pointer) {
    if (!pointer || pointer != ct_buffer || ct_running) abort();
    ct_buffer = NULL;
    fputs("BUFFER_FREE\n", stderr);
    free(pointer);
}
static int ct_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    if (QCORE_CT_FAIL_CREATE) {
        fputs("CREATE_FAILED\n", stderr);
        return thrd_error;
    }
    int status = thrd_create(thread, worker, argument);
    if (status != thrd_success) abort();
    ct_running = 1;
    fputs("CREATE_OK\n", stderr);
    return status;
}
static int ct_join(thrd_t thread, int *result) {
    int status = thrd_join(thread, result);
    if (status != thrd_success || !ct_running) abort();
    ct_running = 0;
    fputs("JOIN_OK\n", stderr);
    return status;
}
static void ct_env_free(void *pointer) {
    if (ct_running || pointer == ct_buffer) abort();
    fputs("ENV_FREE\n", stderr);
    free(pointer);
}
#define QCORE_BUFFER_MALLOC ct_alloc
#define QCORE_BUFFER_FREE ct_buffer_free
#define QCORE_TASK_CREATE ct_create
#define QCORE_TASK_JOIN ct_join
#define QCORE_TASK_FREE ct_env_free

static void *fail_environment(size_t n,size_t s) {(void)n;(void)s;return NULL;}
#define QCORE_TASK_CALLOC fail_environment
