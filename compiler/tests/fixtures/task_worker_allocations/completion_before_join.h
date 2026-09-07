/* One real child: its body completes while custody remains unjoined. */
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <unistd.h>
static void *ct_buffer, *ct_record;
static int ct_created, ct_joined;
static thrd_t ct_thread;
static thrd_start_t ct_worker;
static atomic_int ct_body_done;
static void ct_finish(void) {
    if (ct_buffer || ct_record || ct_created || !ct_joined) _Exit(91);
    fputs("RUN_CLEAN\n", stderr);
}
static void *ct_allocate(size_t size) {
    if (ct_buffer) abort();
    ct_buffer=malloc(size);
    if (!ct_buffer) abort();
    fputs("BUFFER_ALLOC\n", stderr);
    return ct_buffer;
}
static void *ct_environment(size_t count,size_t size) {
    if (ct_record || atexit(ct_finish)!=0) abort();
    ct_record=calloc(count,size);
    if (!ct_record) abort();
    fputs("ENV_ALLOC\n", stderr);
    return ct_record;
}
static int ct_complete_body(void *argument) {
    int result=ct_worker(argument);
    fputs("WORKER_BODY_DONE\n", stderr);
    atomic_store_explicit(&ct_body_done,1,memory_order_release);
    return result;
}
static int ct_create(thrd_t *thread,thrd_start_t worker,void *argument) {
    if (ct_created || argument!=ct_record) abort();
    ct_worker=worker;
    if (thrd_create(thread,ct_complete_body,argument)!=thrd_success) abort();
    ct_thread=*thread;ct_created=1;
    alarm(5);
    while (!atomic_load_explicit(&ct_body_done,memory_order_acquire)) thrd_yield();
    fputs("CREATE_RETURNS_AFTER_BODY\n",stderr);
    return thrd_success;
}
static int ct_join(thrd_t thread,int *result) {
    if (!ct_created || ct_joined || !ct_record || !ct_buffer ||
        !thrd_equal(thread,ct_thread) || !atomic_load_explicit(&ct_body_done,memory_order_acquire)) abort();
    fputs("UNJOINED_BUFFER_LIVE\n",stderr);
    if (thrd_join(thread,result)!=thrd_success) abort();
    ct_created=0;ct_joined=1;
    fputs("JOIN_OK\n",stderr);
    return thrd_success;
}
static void ct_environment_free(void *pointer) {
    if (!pointer || pointer!=ct_record || ct_created || !ct_joined) abort();
    ct_record=NULL;fputs("ENV_FREE\n",stderr);free(pointer);
}
static void ct_buffer_free(void *pointer) {
    if (!pointer || pointer!=ct_buffer || !ct_joined || ct_created) abort();
    ct_buffer=NULL;fputs("BUFFER_FREE\n",stderr);free(pointer);
}
#define QCORE_TASK_CALLOC ct_environment
#define QCORE_TASK_FREE ct_environment_free
#define QCORE_TASK_CREATE ct_create
#define QCORE_TASK_JOIN ct_join
#define QCORE_BUFFER_MALLOC ct_allocate
#define QCORE_BUFFER_FREE ct_buffer_free
