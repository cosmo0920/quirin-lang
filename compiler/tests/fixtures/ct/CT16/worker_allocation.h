/* Fixed one-worker allocation failures and exact cleanup identities. */
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
static void *ct16_buffers[2], *ct16_record;
static thrd_t ct16_thread;
static int ct16_created;
static atomic_int ct16_ready;
static void ct16_finish(void) {
    if (ct16_buffers[0] || ct16_buffers[1] || ct16_record || ct16_created) _Exit(91);
    fputs("RUN_CLEAN\n", stderr);
}
static void *ct16_environment(size_t count, size_t size) {
    if (ct16_record || atexit(ct16_finish) != 0) abort();
    ct16_record=calloc(count,size);
    if (!ct16_record) abort();
    fputs("ENV_ALLOC\n", stderr);
    return ct16_record;
}
static void *ct16_allocate(size_t size, uint64_t task, uint64_t attempt) {
    if (task != 1 || attempt < 1 || attempt > 2) abort();
    while (!atomic_load_explicit(&ct16_ready,memory_order_acquire)) thrd_yield();
    if (task == QCORE_CT_ALLOC_TASK && attempt == QCORE_CT_FAIL_ALLOC) {
        fprintf(stderr,"BUFFER_FAILED:%llu\n",(unsigned long long)attempt);
        return NULL;
    }
    void *pointer=malloc(size);
    if (!pointer) abort();
    ct16_buffers[attempt-1]=pointer;
    fprintf(stderr,"BUFFER_ALLOC:%llu\n",(unsigned long long)attempt);
    return pointer;
}
static void ct16_buffer_free(void *pointer) {
    int index=-1;
    for (int i=0;i<2;i++) if(pointer && pointer==ct16_buffers[i]) index=i;
    if(index<0) abort();
    ct16_buffers[index]=NULL;
    fprintf(stderr,"BUFFER_FREE:%d\n",index+1);
    free(pointer);
}
static int ct16_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    if(argument!=ct16_record || ct16_created) abort();
    if(QCORE_CT_FAIL_CREATE==1) {fputs("CREATE_FAILED\n",stderr);return thrd_error;}
    if(thrd_create(thread,worker,argument)!=thrd_success) abort();
    ct16_thread=*thread;ct16_created=1;
    fputs("CREATE_OK\n",stderr);
    atomic_store_explicit(&ct16_ready,1,memory_order_release);
    return thrd_success;
}
static int ct16_join(thrd_t thread,int *result) {
    if(!ct16_created || !thrd_equal(thread,ct16_thread)) abort();
    if(thrd_join(thread,result)!=thrd_success) abort();
    ct16_created=0;
    fputs("JOIN_OK\n",stderr);
    return thrd_success;
}
static void ct16_environment_free(void *pointer) {
    if(!pointer || pointer!=ct16_record || ct16_created) abort();
    ct16_record=NULL;fputs("ENV_FREE\n",stderr);free(pointer);
}
#define QCORE_TASK_CALLOC ct16_environment
#define QCORE_TASK_FREE ct16_environment_free
#define QCORE_TASK_CREATE ct16_create
#define QCORE_TASK_JOIN ct16_join
#define QCORE_BUFFER_MALLOC_FOR_TASK ct16_allocate
#define QCORE_BUFFER_FREE ct16_buffer_free
